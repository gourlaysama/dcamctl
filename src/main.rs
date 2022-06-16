use std::path::Path;

use anyhow::{anyhow, bail, Result};
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
    let options_matches = ProgramOptions::clap().get_matches();
    let options = ProgramOptions::from_clap(&options_matches);

    if options.version {
        // HACK to disambiguate short/long invocations for the same cli option;
        // there has to be a better way of doing this...
        let i = options_matches
            .index_of("version")
            .ok_or_else(|| anyhow!("should never happen: version set yet no version flag"))?;
        if std::env::args().nth(i).unwrap_or_default() == "-V" {
            print_version(false);
        } else {
            print_version(true);
        }
        return Ok(());
    }

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
        AudioSupport::new(!conf.no_echo_cancel)?
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
    let mut conf = config::Config::builder();
    // merge default values as fallback
    conf = conf.add_source(config::File::from_str(
        DEFAULT_CONFIG,
        config::FileFormat::Yaml,
    ));

    if let Some(path) = &options.config {
        debug!("looking for config file '{}'", path.display());
        conf = conf.add_source(config::File::from(path.as_ref()));
        info!("using config from '{}'", path.canonicalize()?.display());
    } else if let Some(p) = directories() {
        let f = p.config_dir().join("config.yml");
        debug!("looking for config file '{}'", f.display());

        if f.exists() {
            info!("using config from '{}'", f.canonicalize()?.display());
            conf = conf.add_source(config::File::from(f));
        } else {
            empty = true;
        }
    };
    if empty {
        info!("no config file found, using default values");
    };

    fn set_conf_from_options(
        conf: config::ConfigBuilder<config::builder::DefaultState>,
        option: &Option<String>,
        key: &str,
    ) -> Result<config::ConfigBuilder<config::builder::DefaultState>> {
        let c = if let Some(value) = option {
            conf.set_override(key, Some(value.as_str()))?
        } else {
            conf
        };

        Ok(c)
    }

    conf = set_conf_from_options(conf, &options.port.map(|p| p.to_string()), "port")?;
    conf = set_conf_from_options(conf, &options.device, "device")?;
    conf = set_conf_from_options(conf, &options.resolution, "resolution")?;
    conf = set_conf_from_options(conf, &options.flip, "flip")?;
    if options.no_audio {
        conf = conf.set_override("no_audio", Some(true))?;
    }
    if options.no_echo_cancel {
        conf = conf.set_override("no_echo_cancel", Some(true))?;
    }

    let conf: ProgramConfig = conf.build()?.try_deserialize()?;
    trace!("full config: {:#?}", conf);

    Ok(conf)
}

fn print_version(long: bool) {
    if long {
        println!(
            "{} {} ({})",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
            option_env!("BUILD_ID").unwrap_or("unknown")
        );
        println!("rustc {} ({})", env!("BUILD_RUSTC"), env!("BUILD_INFO"));
        if let Some(p) = directories() {
            println!(
                "\nconfig location: {}",
                p.config_dir().join("config.yml").display()
            );
        }
    } else {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    }
}
