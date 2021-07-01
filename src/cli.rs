use std::path::PathBuf;

use log::LevelFilter;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(about = "Use android device as webcam with v4l2loopback")]
pub struct ProgramOptions {
    /// Port to forward between the device and localhost.
    ///
    /// The port on on the device with this value will be forwarded to the same port on localhost.
    /// [default: 8080]
    #[structopt(long, short)]
    pub port: Option<u16>,

    /// v4l2loopback video device to use.
    ///
    /// This device must be one expose by the v4l2loopback kernel module. Check the devices under /dev/video* with
    /// `v4l2-ctl -d /dev/videoX -D` for the correct one.
    /// [default: /dev/video0]
    #[structopt(long, short)]
    pub device: Option<String>,

    /// Output resolution to use.
    ///
    /// The video feed will be resized to this value if needed.
    /// [default: auto]
    #[structopt(long, short)]
    pub resolution: Option<String>,

    /// Use the given configuration file instead of the default.
    ///
    /// By default, dcamctl looks for a configuration file in "$XDG_CONFIG_HOME/dcamctl/config.yml"
    /// or "$HOME/.config/dcamctl/config.yml".
    #[structopt(long)]
    pub config: Option<PathBuf>,

    /// Disable audio support.
    ///
    /// Do not setup audio forwarding or interact at all with the audio system.
    #[structopt(long, short)]
    pub no_audio: bool,

    /// Pass for more log output.
    #[structopt(long, short, global = true, parse(from_occurrences))]
    verbose: i8,

    /// Pass for less log output.
    #[structopt(
        long,
        short,
        global = true,
        parse(from_occurrences),
        conflicts_with = "verbose"
    )]
    quiet: i8,
}

impl ProgramOptions {
    pub fn log_level_with_default(&self, default: i8) -> Option<LevelFilter> {
        let level = default + self.verbose - self.quiet;
        let new_level = match level {
            i8::MIN..=0 => LevelFilter::Off,
            1 => LevelFilter::Error,
            2 => LevelFilter::Warn,
            3 => LevelFilter::Info,
            4 => LevelFilter::Debug,
            5..=i8::MAX => LevelFilter::Trace,
        };

        if level != default {
            Some(new_level)
        } else {
            None
        }
    }
}
