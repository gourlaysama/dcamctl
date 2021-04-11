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
}

impl Pipeline {
    pub fn new(device: &Path, resolution: Resolution, port: u16) -> Result<Pipeline> {
        let device = format!("device={}", device.to_string_lossy());
        let caps = format!(
            "video/x-raw,format=YUY2,width={},height={}",
            resolution.width, resolution.height
        );
        let pipeline = gstreamer::parse_launch(
            &format!("souphttpsrc location=http://127.0.0.1:{}/videofeed do-timestamp=true is-live=true ! queue ! multipartdemux ! decodebin ! videoconvert ! videoscale ! {} ! v4l2sink {} sync=true", port, caps, device))?;

        Ok(Pipeline { pipeline })
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
