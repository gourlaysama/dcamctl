use std::{io::Stdout, path::Path};

use crate::config::Resolution;
use anyhow::*;
use futures::{FutureExt, StreamExt};
use gstreamer::prelude::*;
use log::*;
use regex::Regex;
use termion::raw::{IntoRawMode, RawTerminal};

mod cam_info;
pub mod cli;
pub mod config;
mod control;
#[macro_use]
mod macros;

pub struct AdbServer {
    port: u16,
}

impl AdbServer {
    pub fn init() -> Result<()> {
        run_cmd!("adb", "start-server" => "could not start adb server");

        Ok(())
    }

    pub fn connect(port: u16) -> Result<AdbServer> {
        let port_str = format!("tcp:{}", port);
        run_cmd!("adb", "forward", &port_str, &port_str => "could not enable adb tcp forwarding");
        debug!("forwarding adb port {} to 127.0.0.1:{}", port, port);

        Ok(AdbServer { port })
    }
}

impl Drop for AdbServer {
    fn drop(&mut self) {
        run_cmd!("adb", "forward", "--remove", &format!("tcp:{}", self.port) => "could not remove adb tcp forwarding", |s| {
            warn!("could not remove adb tcp forwarding (got {})", s)
        });
    }
}

pub struct Dcam {
    port: u16,
    pipeline: gstreamer::Pipeline,
    _audio: Option<AudioSupport>,
    _stdout: RawTerminal<Stdout>,
}

impl Dcam {
    pub async fn setup(
        audio: Option<AudioSupport>,
        device: &Path,
        resolution: Option<Resolution>,
        port: u16,
        flip: Option<String>,
    ) -> Result<Dcam> {
        let mut _stdout = std::io::stdout().into_raw_mode()?;

        let resolution = match resolution {
            Some(r) => r,
            None => match control::get_cam_info(port, false).await {
                Ok(cam_info) => {
                    debug!(
                        "autodetecting default resolution of {}",
                        cam_info.curvals.video_size
                    );
                    cam_info.curvals.video_size
                }
                Err(e) => {
                    debug!("{}", e);
                    warn!("failed to autodetect device resolution; using 640x480");
                    Resolution {
                        height: 480,
                        width: 640,
                    }
                }
            },
        };

        let device_str = device.to_string_lossy();
        let caps = format!(
            "video/x-raw,format=YUY2,width={},height={}",
            resolution.width, resolution.height
        );
        let method = match flip.as_deref() {
            Some("horizontal") => "horizontal-flip",
            Some("vertical") => "vertical-flip",
            Some("none") | None => "none",
            Some(other) => {
                debug!("unknown flip method '{}', ignoring", other);
                "none"
            }
        };

        let mut pipeline_desc = String::new();
        if audio.is_some() {
            pipeline_desc.push_str(&format!("souphttpsrc location=http://127.0.0.1:{}/audio.wav do-timestamp=true is-live=true ! audio/x-raw,format=S16LE,layout=interleaved,rate=44100,channels=1 ! queue ! pulsesink device=dcamctl_webcam sync=true ", port));
        }
        pipeline_desc.push_str(&format!("souphttpsrc location=http://127.0.0.1:{}/videofeed do-timestamp=true is-live=true ! queue ! multipartdemux ! decodebin ! videoflip name=flip_elem method=\"{}\" ! videoconvert ! videoscale ! {} ! v4l2sink device={} sync=true", port, method,  caps, device_str));

        let pipeline = gstreamer::parse_launch(&pipeline_desc)?
            .downcast()
            .map_err(|_| anyhow!("broken pipeline"))?;

        info!(
            "set up video input '{}' with resolution {}",
            device_str, resolution
        );
        show!(Warn, "\r  Video     : {}\r", device_str);

        Ok(Dcam {
            port,
            pipeline,
            _audio: audio,
            _stdout,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        self.pipeline.set_state(gstreamer::State::Playing)?;
        debug!("running pipeline");

        let bus = match self.pipeline.bus() {
            Some(b) => b,
            None => bail!("No bus for gstreamer pipeline"),
        };

        let flip = self
            .pipeline
            .by_name("flip_elem")
            .ok_or_else(|| anyhow!("missing videoflip"))?;

        let stop_signals = crate::control::stop_signals().boxed_local();
        let quit_command = crate::control::process_commands(self.port, flip).boxed_local();
        let stop_run = futures::future::select(stop_signals, quit_command);
        let mut stream = bus.stream().take_until(stop_run);

        while let Some(msg) = stream.next().await {
            use gstreamer::MessageView;

            match msg.view() {
                MessageView::Eos(..) => {
                    warn!("received end-of-stream, quitting");
                    break;
                }
                MessageView::Error(err) => {
                    error!(
                        "Error from {:?}: {} ({:?})",
                        err.src().map(|s| s.path_string()),
                        err.error(),
                        err.debug()
                    );
                    break;
                }
                _ => {}
            }
        }

        self.pipeline.set_state(gstreamer::State::Paused)?;

        show!("Disconnected.\r");

        Ok(())
    }
}

impl Drop for Dcam {
    fn drop(&mut self) {
        // Shutdown pipeline
        if let Err(e) = self.pipeline.set_state(gstreamer::State::Null) {
            error!("{}", e);
        }
    }
}

#[derive(Debug)]
pub struct AudioSupport {
    default_source: String,
    default_sink: String,
    sink_id: u32,
    echo_cancel: EchoCancel,
}

#[derive(Debug)]
enum EchoCancel {
    Pulseaudio { cancel_sink_id: u32 },
    Disabled,
    // TODO: PipeWireNative
}

impl AudioSupport {
    pub fn new(echo_cancel: bool) -> Result<Option<AudioSupport>> {
        run_cmd!("pactl", "--version" => "unable to find 'pactl' command");

        let output = get_cmd!("pactl", "info" => "failed to get pulseaudio info");
        let out = String::from_utf8_lossy(&output.stdout);

        let re = Regex::new(r"PipeWire ([^[[:space:]]\)]+)?").unwrap();
        let mut default_sink = String::new();
        let mut default_source = String::new();
        let mut found_echo_cancel = None;
        for l in out.lines() {
            let mut l = l.split(": ");
            match l.next() {
                Some("Server Name") => {
                    if let Some(name) = l.next() {
                        if let Some(c) = re.captures(name) {
                            if let Ok(v) = lenient_semver::parse(&c[1]) {
                                debug!("using pipewire backend");
                                let acancel_version = lenient_semver::parse("0.3.30")?;
                                if v < acancel_version {
                                    debug!(
                                        "pirewire {} < {}: echo cancellation unsupported",
                                        v, acancel_version
                                    );
                                    found_echo_cancel = Some(false);
                                } else {
                                    debug!(
                                        "pirewire {} >= {}: echo cancellation supported",
                                        v, acancel_version
                                    );
                                    found_echo_cancel = Some(true);
                                }
                                continue;
                            }
                        }

                        debug!("using pulseaudio backend");
                        found_echo_cancel = Some(true);
                    }
                }
                Some("Default Sink") => {
                    if let Some(sink) = l.next() {
                        default_sink.push_str(sink);
                    }
                }
                Some("Default Source") => {
                    if let Some(source) = l.next() {
                        default_source.push_str(source);
                    }
                }
                _ => {}
            }
        }

        trace!("default_sink = {}", default_sink);
        trace!("default_source = {}", default_source);
        trace!("echo_cancel = {:?}", found_echo_cancel);

        let echo_cancel_backend = if let Some(found) = found_echo_cancel {
            if echo_cancel && found {
                EchoCancel::Pulseaudio { cancel_sink_id: 0 }
            } else {
                EchoCancel::Disabled
            }
        } else {
            debug!("could not find supported audio backend");
            return Ok(None);
        };

        let mut audio_support = AudioSupport {
            default_sink,
            default_source,
            sink_id: 0,
            echo_cancel: echo_cancel_backend,
        };

        audio_support.setup()?;

        Ok(Some(audio_support))
    }

    fn setup(&mut self) -> Result<()> {
        let output = get_cmd!(
            "pactl",
            "load-module",
            "module-null-sink",
            "sink_name=dcamctl_webcam",
            "format=S16LE rate=44100 channels=1",
            "sink_properties=\"device.description='dcamctl (raw)'\""
             => "failed to load dcamctl audio module");

        self.sink_id = String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse()
            .context("failed to parse sink_id")?;
        trace!("sink_id = {}", self.sink_id);

        self.echo_cancel.setup(&self.default_sink)?;

        match self.echo_cancel {
            EchoCancel::Pulseaudio { .. } => {
                run_cmd!("pactl", "set-default-source", "dcamctl_webcam_ec_src" => "failed to set dcamctl as default source");
                run_cmd!("pactl", "set-default-sink", "dcamctl_webcam_ec_aout" => "failed to set dcamctl as default sink");

                info!("set up default audio input 'Webcam Virtual Microphone (EC-cancelled)'");
                info!("set up default audio output 'Default Audio Out (EC-cancelled with Webcam Virtual Microphone)'");

                show!(Warn, "\rSetting temporary defaults:");
                show!(
                    Warn,
                    "  Microphone: Webcam Virtual Microphone (EC-cancelled)\r"
                );
                show!(
                    Warn,
                    "  Speaker   : Default Audio Out (EC-cancelled with Webcam Virtual Microphone)\r"
                );
            }
            EchoCancel::Disabled => {
                run_cmd!("pactl", "set-default-source", "dcamctl_webcam.monitor" => "failed to set dcamctl as default source");

                info!("set up default audio input 'Webcam Virtual Microphone'");

                show!(Warn, "Setting temporary defaults:\r");
                show!(Warn, "  Microphone: Webcam Virtual Microphone\r");
            }
        }

        Ok(())
    }
}

impl Drop for AudioSupport {
    fn drop(&mut self) {
        run_cmd!("pactl", "set-default-source", &self.default_source =>
            "failed to reset default source",
            |s| warn!(
                "error trying to set default source back to {} (returned {})",
                self.default_source, s
            )
        );

        run_cmd!("pactl", "set-default-sink", &self.default_sink =>
            "failed to reset default sink",
            |s| warn!(
                "error trying to set default sink back to {} (returned {})",
                self.default_sink, s
            )
        );

        run_cmd!("pactl", "unload-module", &self.sink_id.to_string() =>
            "failed to unload dcamctl audio module",
            |s| warn!(
                "error trying to unload webcam audio module, id={} (returned {})",
                self.sink_id, s
            )
        );
    }
}

impl EchoCancel {
    fn setup(&mut self, default_sink: &str) -> Result<()> {
        match self {
            EchoCancel::Pulseaudio { cancel_sink_id } => {
                let output = get_cmd!(
                    "pactl",
                    "load-module",
                    "module-echo-cancel",
                    "source_master=dcamctl_webcam.monitor",
                    "source_name=dcamctl_webcam_ec_src",
                    "source_properties=\"device.description='Webcam Virtual Microphone (EC-cancelled)'\"",
                    &format!("sink_master={}", default_sink),
                    "sink_name=dcamctl_webcam_ec_aout",
                    "sink_properties=\"device.description='Default Audio Out (EC-cancelled with Webcam Virtual Microphone)'\"",
                    "format=S16LE rate=44100 channels=1",
                    "aec_method=\"webrtc\"",
                    "save_aec=true",
                    "use_volume_sharing=true"
                    => "failed to load echo cancellation module"
                );

                let new_cancel_sink_id: u32 = String::from_utf8_lossy(&output.stdout)
                    .trim()
                    .parse()
                    .context("failed to parse cancel_sink_id")?;
                trace!("cancel_sink_id={}", new_cancel_sink_id);
                *cancel_sink_id = new_cancel_sink_id;
            }
            EchoCancel::Disabled => {}
        };

        Ok(())
    }
}

impl Drop for EchoCancel {
    fn drop(&mut self) {
        match self {
            EchoCancel::Pulseaudio { cancel_sink_id } => {
                run_cmd!("pactl", "unload-module", &cancel_sink_id.to_string() =>
                    "failed to unload echo cancellation module",
                    |s| warn!(
                        "error trying to unload echo cancelation module, id={} (returned {})",
                        cancel_sink_id, s
                    )
                );
            }
            EchoCancel::Disabled => {}
        }
    }
}
