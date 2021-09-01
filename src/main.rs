use std::path::Path;

use anyhow::*;
use dcamctl::{cli::ProgramOptions, config::*};
use dcamctl::{show, AdbServer, AudioSupport, Dcam};
use directories_next::ProjectDirs;
use env_logger::{Builder, Env};
use log::*;
use structopt::StructOpt;
use tokio::runtime;

type ReturnCode = i32;

static DEFAULT_CONFIG: &str = include_str!("../config.yml");

fn main() -> Result<()> {
    let options = ProgramOptions::from_args();

    let mut b = Builder::default();
    b.format_timestamp(None);
    b.format_suffix("\r\n");
    b.filter_level(LevelFilter::Warn); // default filter lever
    b.parse_env(Env::from("DCAMCTL_LOG")); // override with env
                                           // override with CLI option
    if let Some(level) = options.log_level_with_default(2) {
        b.filter_level(level);
    };
    b.try_init()?;

    let rt = runtime::Builder::new_multi_thread().enable_all().build()?;

    std::process::exit(match rt.block_on(run(options)) {
        Ok(i) => i,
        Err(e) => {
            show!("Error: {}", e);
            for cause in e.chain().skip(1) {
                info!("cause: {}", cause);
            }
            1
        }
    })
}

async fn run(options: ProgramOptions) -> Result<ReturnCode> {
    let conf = make_config(options)?;

    check_kernel_module()?;

    AdbServer::init()?;
    let _server = AdbServer::connect(conf.port)?;

    gstreamer::init()?;

    let audio = if conf.no_audio {
        None
    } else {
        AudioSupport::new()?
    };
    let mut pipeline =
        Dcam::setup(audio, &conf.device, conf.resolution, conf.port, conf.flip).await?;

    pipeline.run().await?;

    Ok(0)
}

fn check_kernel_module() -> Result<()> {
    let path = Path::new("/sys/module/v4l2loopback");
    if !path.exists() {
        bail!("Kernel module v4l2looback isn't loaded");
    }
    Ok(())
}

fn directories() -> Option<ProjectDirs> {
    ProjectDirs::from("rs", "", "Dcamctl")
}

fn make_config(options: ProgramOptions) -> Result<ProgramConfig> {
    let mut empty = false;
    let mut conf = config::Config::default();
    // merge default values as fallback
    conf.merge(config::File::from_str(
        DEFAULT_CONFIG,
        config::FileFormat::Yaml,
    ))?;

    if let Some(path) = &options.config {
        debug!("looking for config file '{}'", path.display());
        conf.merge(config::File::from(path.as_ref()))?;
        info!("using config from '{}'", path.canonicalize()?.display());
    } else if let Some(p) = directories() {
        let f = p.config_dir().join("config.yml");
        debug!("looking for config file '{}'", f.display());

        if f.exists() {
            info!("using config from '{}'", f.canonicalize()?.display());
            conf.merge(config::File::from(f))?;
        } else {
            empty = true;
        }
    };
    if empty {
        info!("no config file found, using default values");
    };

    fn set_conf_from_options(
        conf: &mut config::Config,
        option: &Option<String>,
        key: &str,
    ) -> Result<()> {
        if let Some(value) = option {
            conf.set(key, Some(value.as_str()))?;
        }

        Ok(())
    }

    set_conf_from_options(&mut conf, &options.port.map(|p| p.to_string()), "port")?;
    set_conf_from_options(&mut conf, &options.device, "device")?;
    set_conf_from_options(&mut conf, &options.resolution, "resolution")?;
    set_conf_from_options(&mut conf, &options.flip, "flip")?;
    if options.no_audio {
        conf.set("no_audio", Some(true))?;
    }

    let conf: ProgramConfig = conf.try_into()?;
    trace!("full config: {:#?}", conf);

    Ok(conf)
}
