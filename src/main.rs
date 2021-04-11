use std::process::Command;
use std::{
    path::Path,
    sync::mpsc::{self, Receiver},
    thread,
};

use anyhow::*;
use dcam::cli::ProgramOptions;
use env_logger::{Builder, Env};
use gstreamer::prelude::*;
use log::*;
use mpsc::TryRecvError;
use structopt::StructOpt;

type ReturnCode = i32;

fn main() -> Result<()> {
    let options = ProgramOptions::from_args();

    let mut b = Builder::from_env(Env::from("DCAM_LOG"));
    b.format_timestamp(None);
    if let Some(level) = options.log_level_with_default(2) {
        b.filter_level(level);
    };
    b.try_init()?;

    std::process::exit(match run(options) {
        Ok(i) => i,
        Err(e) => {
            println!("Error: {}", e);
            for cause in e.chain().skip(1) {
                info!("cause: {}", cause);
            }
            1
        }
    })
}

fn run(options: ProgramOptions) -> Result<ReturnCode> {
    check_kernel_module()?;

    // start adb-server if not already started
    let res = Command::new("adb").arg("start-server").status()?;
    if !res.success() {
        bail!("Could not start adb server");
    }
    // add forwarding rule
    let port = format!("tcp:{}", options.port);
    let res = Command::new("adb")
        .arg("forward")
        .arg(&port)
        .arg(&port)
        .status()?;
    if !res.success() {
        bail!("Could not enable tcp forwarding");
    }

    let device = format!("device={}", options.device.to_string_lossy());
    let caps = format!(
        "video/x-raw,format=YUY2,width={},height={}",
        options.resolution.width, options.resolution.height
    );

    gstreamer::init()?;

    let pipeline = gstreamer::parse_launch(
        &format!("souphttpsrc location=http://127.0.0.1:{}/videofeed do-timestamp=true is-live=true ! queue ! multipartdemux ! decodebin ! videoconvert ! videoscale ! {} ! v4l2sink {} sync=true", options.port, caps, device))?;

    pipeline.set_state(gstreamer::State::Playing)?;

    println!("Press <Enter> to disconnect the webcam.");
    let stdin_channel = watch_stdin();

    let bus = match pipeline.get_bus() {
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

        match stdin_channel.try_recv() {
            Ok(_) => break,
            Err(TryRecvError::Empty) => {}
            Err(a) => return Err(anyhow!("Internal threading error").context(a)),
        }
    }

    // Shutdown pipeline
    pipeline.set_state(gstreamer::State::Null)?;

    // remove forwarding rule
    let res = Command::new("adb")
        .arg("forward")
        .arg("--remove")
        .arg(&port)
        .status()?;
    if !res.success() {
        bail!("Could not disable tcp forwarding");
    }

    Ok(0)
}

fn watch_stdin() -> Receiver<()> {
    let (tx, rx) = mpsc::channel::<()>();
    thread::spawn(move || loop {
        // ignore line content
        let mut buffer = String::new();
        std::io::stdin().read_line(&mut buffer).unwrap();
        tx.send(()).unwrap();
    });
    rx
}

fn check_kernel_module() -> Result<()> {
    let path = Path::new("/sys/module/v4l2loopback");
    if !path.exists() {
        bail!("Kernel module v4l2looback isn't loaded");
    }
    Ok(())
}
