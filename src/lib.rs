use std::{
    path::Path,
    process::Command,
    str::FromStr,
    sync::mpsc::{Receiver, TryRecvError},
};

use anyhow::*;
use gstreamer::prelude::*;
use log::*;

pub mod cli;

macro_rules! run_cmd {
    ($nme:expr, $($args:expr),* => $ctx:expr, $oncode:expr) => {
        if log_enabled!(log::Level::Trace) {
            trace!("running '{}' with arguments '{:?}'", $nme, &[$($args,)*]);
        }
        match Command::new($nme).args(&[$($args,)*]).status().context($ctx) {
            Err(e) => error!("{}", e),
            Ok(s) => {
                if !s.success() {
                    $oncode(s)
                }
            }
        }
    };
    ($nme:expr, $($args:expr),* => $ctx:expr) => {
        if log_enabled!(log::Level::Trace) {
            trace!("running '{}' with arguments '{:?}'", $nme, &[$($args,)*]);
        }
        match Command::new($nme).args(&[$($args,)*]).status().context($ctx) {
            Err(e) => bail!("{}", e),
            Ok(s) => {
                if !s.success() {
                    bail!("{} (got {})", $ctx, s);
                }
            }
        }
    };
}

macro_rules! get_cmd {
    ($nme:expr, $($args:expr),* => $ctx:expr, $oncode:expr) => {{
        if log_enabled!(log::Level::Trace) {
            trace!("running '{}' with arguments '{:?}'", $nme, &[$($args,)*]);
        }
        let o = Command::new($nme).args(&[$($args,)*]).output().context($ctx)?;
        if !o.status.success() {
            $oncode(o.status);
        };
        o
    }};
    ($nme:expr, $($args:expr),* => $ctx:expr) => {{
        if log_enabled!(log::Level::Trace) {
            trace!("running '{}' with arguments '{:?}'", $nme, &[$($args,)*]);
        }
        let o = Command::new($nme).args(&[$($args,)*]).output().context($ctx)?;
        if !o.status.success() {
            bail!("{} (got {})", $ctx, o.status);
        };
        o
    }};
}

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
        let res = Command::new("adb").arg("start-server").status()?;
        if !res.success() {
            bail!("Could not start adb server");
        }

        Ok(())
    }

    pub fn forward_port(port: u16) -> Result<AdbServerGuard> {
        let port_str = format!("tcp:{}", port);
        let res = Command::new("adb")
            .arg("forward")
            .arg(&port_str)
            .arg(&port_str)
            .status()?;
        if !res.success() {
            bail!("Could not enable tcp forwarding");
        }

        Ok(AdbServerGuard { port })
    }
}

impl Drop for AdbServerGuard {
    fn drop(&mut self) {
        let res = Command::new("adb")
            .arg("forward")
            .arg("--remove")
            .arg(&format!("tcp:{}", self.port))
            .status();
        if let Ok(res) = res {
            if res.success() {
                return;
            }
        }

        error!("Could not disable tcp forwarding");
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
            pipeline_desc.push_str(&format!("souphttpsrc location=http://127.0.0.1:{}/audio.wav do-timestamp=true is-live=true ! audio/x-raw,format=S16LE,layout=interleaved,rate=44100,channels=1 ! queue ! pulsesink device=dcam_webcam sync=true ", port));
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
            "sink_name=dcam_webcam",
            "format=S16LE rate=44100 channels=1",
            "sink_properties=\"device.description='DCam Webcam Virtual Microphone'\""
             => "failed to load dcam audio module");

        let sink_id: u32 = String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse()
            .context("failed to parse sink_id")?;
        debug!("sink_id={}", sink_id);

        let output = get_cmd!(
            "pactl",
            "load-module",
            "module-echo-cancel",
            "sink_name=dcam_webcam_echo_cancel",
            "source_master=dcam_webcam.monitor",
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

        run_cmd!("pactl", "set-default-source", "dcam_webcam.monitor" => "failed to set dcam as default source");

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
            "failed to unload dcam audio module",
            |s| warn!(
                "error trying to unload webcam audio module, id={} (returned {})",
                self.sink_id, s
            )
        );
    }
}
