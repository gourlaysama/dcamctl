use std::path::Path;
use std::process::Command;

use anyhow::*;
use dcam::cli::ProgramOptions;
use env_logger::{Builder, Env};
use log::*;
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
    let mut child = Command::new("gst-launch-1.0")
        .args(&["-e", "-vt", "--gst-plugin-spew"])
        .args(&[
            "souphttpsrc",
            "location=http://127.0.0.1:8080/videofeed",
            "do-timestamp=true",
            "is-live=true",
            "!",
            "queue",
            "!",
            "multipartdemux",
            "!",
            "decodebin",
            "!",
            "videoconvert",
            "!",
            "videoscale",
            "!",
            &caps,
            "!",
            "v4l2sink",
            &device,
            "sync=true",
        ])
        .spawn()?;

    println!("Press <Enter> to disconnect the webcam.");
    let mut _o = String::new();
    std::io::stdin().read_line(&mut _o)?;

    child.kill()?;

    // add forwarding rule
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

fn check_kernel_module() -> Result<()> {
    let path = Path::new("/sys/module/v4l2loopback");
    if !path.exists() {
        bail!("Kernel module v4l2looback isn't loaded");
    }
    Ok(())
}
