use std::{
    path::Path,
    sync::mpsc::{self, Receiver},
    thread,
};

use anyhow::*;
use dcam::cli::ProgramOptions;
use dcam::{AdbServer, Pipeline};
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
    AdbServer::init()?;

    // add forwarding rule
    let _guard = AdbServer::forward_port(options.port)?;

    gstreamer::init()?;

    let pipeline = Pipeline::new(&options.device, options.resolution, options.port)?;

    println!("Press <Enter> to disconnect the webcam.");
    pipeline.run(watch_stdin())?;

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
