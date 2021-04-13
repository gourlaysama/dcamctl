use std::{
    path::Path,
    str::FromStr,
    sync::mpsc::{Receiver, TryRecvError},
};

use anyhow::*;
use gstreamer::prelude::*;
use log::*;

pub mod cli;
#[macro_use]
mod macros;

#[derive(Debug)]
pub struct Resolution {
    pub height: u16,
    pub width: u16,
}

impl FromStr for Resolution {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split('x');
        let width: u16 = if let Some(left) = parts.next() {
            left.parse()?
        } else {
            bail!("No width found")
        };
        let height: u16 = if let Some(left) = parts.next() {
            left.parse()?
        } else {
            bail!("No height found")
        };

        Ok(Self { height, width })
    }
}

pub struct AdbServer {}

pub struct AdbServerGuard {
    port: u16,
}

impl AdbServer {
    pub fn init() -> Result<()> {
        run_cmd!("adb", "start-server" => "could not start adb server");

        Ok(())
    }

    pub fn forward_port(port: u16) -> Result<AdbServerGuard> {
        let port_str = format!("tcp:{}", port);
        run_cmd!("adb", "forward", &port_str, &port_str => "could not enable adb tcp forwarding");

        Ok(AdbServerGuard { port })
    }
}

impl Drop for AdbServerGuard {
    fn drop(&mut self) {
        run_cmd!("adb", "forward", "--remove", &format!("tcp:{}", self.port) => "could not remove adb tcp forwarding", |s| {
            warn!("could not remove adb tcp forwarding (got {})", s)
        });
    }
}

pub struct Pipeline {
    pipeline: gstreamer::Element,
    _audio: Option<AudioSupport>,
}

impl Pipeline {
    pub fn new(
        audio: Option<AudioSupport>,
        device: &Path,
        resolution: Resolution,
        port: u16,
    ) -> Result<Pipeline> {
        let device = format!("device={}", device.to_string_lossy());
        let caps = format!(
            "video/x-raw,format=YUY2,width={},height={}",
            resolution.width, resolution.height
        );
        let mut pipeline_desc = String::new();
        if audio.is_some() {
            pipeline_desc.push_str(&format!("souphttpsrc location=http://127.0.0.1:{}/audio.wav do-timestamp=true is-live=true ! audio/x-raw,format=S16LE,layout=interleaved,rate=44100,channels=1 ! queue ! pulsesink device=dcamctl_webcam sync=true ", port));
        }
        pipeline_desc.push_str(&format!("souphttpsrc location=http://127.0.0.1:{}/videofeed do-timestamp=true is-live=true ! queue ! multipartdemux ! decodebin ! videoconvert ! videoscale ! {} ! v4l2sink {} sync=true", port, caps, device));

        let pipeline = gstreamer::parse_launch(&pipeline_desc)?;

        Ok(Pipeline {
            pipeline,
            _audio: audio,
        })
    }

    pub fn run(&self, stop: Receiver<()>) -> Result<()> {
        self.pipeline.set_state(gstreamer::State::Playing)?;

        let bus = match self.pipeline.get_bus() {
            Some(b) => b,
            None => bail!("No bus for gstreamer pipeline"),
        };
        loop {
            let msg = bus.timed_pop(100 * gstreamer::MSECOND);
            if let Some(msg) = msg {
                use gstreamer::MessageView;

                match msg.view() {
                    MessageView::Eos(..) => break,
                    MessageView::Error(err) => {
                        error!(
                            "Error from {:?}: {} ({:?})",
                            err.get_src().map(|s| s.get_path_string()),
                            err.get_error(),
                            err.get_debug()
                        );
                        break;
                    }
                    _ => {}
                }
            }

            match stop.try_recv() {
                Ok(_) => break,
                Err(TryRecvError::Empty) => {}
                Err(a) => return Err(anyhow!("Internal threading error").context(a)),
            }
        }

        // Stop pipeline
        self.pipeline.set_state(gstreamer::State::Paused)?;

        Ok(())
    }
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        // Shutdown pipeline
        if let Err(e) = self.pipeline.set_state(gstreamer::State::Null) {
            error!("{}", e);
        }
    }
}

pub struct AudioSupport {
    default_source: String,
    sink_id: u32,
    cancel_sink_id: u32,
}

impl AudioSupport {
    pub fn from_pulseaudio() -> Result<AudioSupport> {
        run_cmd!("pacmd", "--version" => "unable to find 'pacmd' command");
        run_cmd!("pactl", "--version" => "unable to find 'pactl' command");

        let output = get_cmd!("pacmd", "dump" => "failed to get pulseaudio info");
        let out = String::from_utf8_lossy(&output.stdout);
        let mut default_sink = String::new();
        let mut default_source = String::new();
        for l in out.lines() {
            let mut l = l.split_ascii_whitespace();
            match l.next() {
                Some("set-default-sink") => {
                    if let Some(sink) = l.next() {
                        default_sink.push_str(sink);
                    }
                }
                Some("set-default-source") => {
                    if let Some(source) = l.next() {
                        default_source.push_str(source);
                    }
                }
                _ => {}
            }
        }
        debug!("default_sink={}", default_sink);
        debug!("default_source={}", default_source);

        let output = get_cmd!(
            "pactl",
            "load-module",
            "module-null-sink",
            "sink_name=dcamctl_webcam",
            "format=S16LE rate=44100 channels=1",
            "sink_properties=\"device.description='dcamctl Webcam Virtual Microphone'\""
             => "failed to load dcamctl audio module");

        let sink_id: u32 = String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse()
            .context("failed to parse sink_id")?;
        debug!("sink_id={}", sink_id);

        let output = get_cmd!(
            "pactl",
            "load-module",
            "module-echo-cancel",
            "sink_name=dcamctl_webcam_echo_cancel",
            "source_master=dcamctl_webcam.monitor",
            &format!("sink_master={}", default_sink),
            "format=S16LE rate=44100 channels=1",
            "aec_method=\"webrtc\"",
            "save_aec=true",
            "use_volume_sharing=true"
            => "failed to load echo cancellation module"
        );

        let cancel_sink_id: u32 = String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse()
            .context("failed to parse cancel_sink_id")?;
        debug!("cancel_sink_id={}", cancel_sink_id);

        run_cmd!("pactl", "set-default-source", "dcamctl_webcam.monitor" => "failed to set dcamctl as default source");

        Ok(AudioSupport {
            default_source,
            sink_id,
            cancel_sink_id,
        })
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

        run_cmd!("pactl", "unload-module", &self.cancel_sink_id.to_string() =>
            "failed to unload echo cancellation module",
            |s| warn!(
                "error trying to unload echo cancelation module, id={} (returned {})",
                self.cancel_sink_id, s
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
