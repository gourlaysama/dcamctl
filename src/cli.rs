use std::path::PathBuf;

use log::LevelFilter;

#[derive(clap::Parser, Debug)]
#[clap(
    about = "Use android device as webcam with v4l2loopback",
    setting = clap::AppSettings::NoAutoVersion,
    mut_arg("help", |h| h.help_heading("INFO")),
    mut_arg("version", |h| h.help_heading("INFO")),
)]
pub struct ProgramOptions {
    /// Port to forward between the device and localhost.
    ///
    /// The port on on the device with this value will be forwarded to the same port on localhost.
    /// [default: 8080]
    #[clap(long, short)]
    pub port: Option<u16>,

    /// v4l2loopback video device to use.
    ///
    /// This device must be one expose by the v4l2loopback kernel module. Check the devices under /dev/video* with
    /// `v4l2-ctl -d /dev/videoX -D` for the correct one.
    /// [default: /dev/video0]
    #[clap(long, short)]
    pub device: Option<String>,

    /// Output resolution to use.
    ///
    /// The video feed will be resized to this value if needed.
    /// [default: auto]
    #[clap(long, short)]
    pub resolution: Option<String>,

    /// Use the given configuration file instead of the default.
    ///
    /// By default, dcamctl looks for a configuration file in "$XDG_CONFIG_HOME/dcamctl/config.yml"
    /// or "$HOME/.config/dcamctl/config.yml".
    #[clap(long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Disable audio support.
    ///
    /// Do not setup audio forwarding or interact at all with the audio system.
    #[clap(long, short, help_heading = "FLAGS")]
    pub no_audio: bool,

    /// Disable echo canceling.
    #[clap(long, short = 'C', help_heading = "FLAGS")]
    pub no_echo_cancel: bool,

    /// Flip method used to mirror the video.
    ///
    /// Defaults to none.
    #[clap(long, short, possible_values(&["horizontal", "vertical", "none"]), value_name = "METHOD")]
    pub flip: Option<String>,

    /// Connect to android device with the given serial.
    #[clap(long, short, value_name = "ANDROID_SERIAL")]
    pub serial: Option<String>,

    /// Pass for more log output.
    #[clap(
        long,
        short,
        global = true,
        parse(from_occurrences),
        help_heading = "FLAGS"
    )]
    verbose: i8,

    /// Pass for less log output.
    #[clap(
        long,
        short,
        global = true,
        parse(from_occurrences),
        conflicts_with = "verbose",
        help_heading = "FLAGS"
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
