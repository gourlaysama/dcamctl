macro_rules! run_cmd {
    ($nme:expr, $($args:expr),* => $ctx:expr, $oncode:expr) => {
        if log_enabled!(log::Level::Trace) {
            trace!("running '{}' with arguments '{:?}'", $nme, &[$($args,)*]);
        }
        match std::process::Command::new($nme).args(&[$($args,)*]).stdout(std::process::Stdio::null()).status().context($ctx) {
            Err(e) => error!("{}", e),
            Ok(s) => {
                if !s.success() {
                    $oncode(s)
                }
            }
        }
    };
    ($nme:expr, $($args:expr),* => $ctx:expr) => {
        if log_enabled!(log::Level::Trace) {
            trace!("running '{}' with arguments '{:?}'", $nme, &[$($args,)*]);
        }
        let s = std::process::Command::new($nme).args(&[$($args,)*]).stdout(std::process::Stdio::null()).status().context($ctx)?;
        if !s.success() {
            bail!("{} (got {})", $ctx, s);
        }
    };
}

macro_rules! get_cmd {
    ($nme:expr, $($args:expr),* => $ctx:expr, $oncode:expr) => {{
        if log_enabled!(log::Level::Trace) {
            trace!("running '{}' with arguments '{:?}'", $nme, &[$($args,)*]);
        }
        let o = std::process::Command::new($nme).args(&[$($args,)*]).output().context($ctx)?;
        if !o.status.success() {
            $oncode(o.status);
        };
        o
    }};
    ($nme:expr, $($args:expr),* => $ctx:expr) => {{
        if log_enabled!(log::Level::Trace) {
            trace!("running '{}' with arguments '{:?}'", $nme, &[$($args,)*]);
        }
        let o = std::process::Command::new($nme).args(&[$($args,)*]).output().context($ctx)?;
        if !o.status.success() {
            bail!("{} (got {})", $ctx, o.status);
        };
        o
    }};
}

#[macro_export]
macro_rules! show {
    ($level:ident, $($a:tt),*) => {
        if log_enabled!(log::Level::$level) {
            println!($($a,)*);
        }
    };
    ($($a:tt),*) => {
        if log_enabled!(log::Level::Error) {
            println!($($a,)*);
        }
    }
}
