use log::LevelFilter;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(about = "Use android device as webcam with v4l2loopback")]
pub struct ProgramOptions {
    #[structopt(long, short, global = true, parse(from_occurrences))]
    verbose: i8,

    /// Pass many times for less log output
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