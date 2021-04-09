use anyhow::*;
use dcam::cli::ProgramOptions;
use log::*;
use env_logger::{Builder, Env};
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

fn run(_options: ProgramOptions)-> Result<ReturnCode> {
    info!("Hello, world!");

    Ok(0)
}
