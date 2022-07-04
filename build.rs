use clap::CommandFactory;
use clap_complete::{generate_to, Shell};
use std::env;
use std::io::Error;
use std::process::Command;

include!("src/cli.rs");

fn main() -> Result<(), Error> {
    let outdir = match env::var_os("OUT_DIR") {
        None => return Err(Error::new(std::io::ErrorKind::Other, "no $OUT_DIR!")),
        Some(outdir) => outdir,
    };
    let mut app = ProgramOptions::command();

    generate_to(Shell::Bash, &mut app, "dcamctl", &outdir)?;

    generate_to(Shell::Zsh, &mut app, "dcamctl", &outdir)?;

    generate_to(Shell::Fish, &mut app, "dcamctl", outdir)?;

    if let Some(v) = version_check::Version::read() {
        println!("cargo:rustc-env=BUILD_RUSTC={}", v)
    }

    if let Some(hash) = get_commit_hash().or_else(|| env::var("BUILD_ID").ok()) {
        println!("cargo:rustc-env=BUILD_ID={}", hash);
    }

    println!(
        "cargo:rustc-env=BUILD_INFO={}-{}-{}-{}",
        env::var("CARGO_CFG_TARGET_ARCH").unwrap(),
        env::var("CARGO_CFG_TARGET_VENDOR").unwrap(),
        env::var("CARGO_CFG_TARGET_OS").unwrap(),
        env::var("CARGO_CFG_TARGET_ENV").unwrap(),
    );

    Ok(())
}

fn get_commit_hash() -> Option<String> {
    Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|r| {
            if r.status.success() {
                String::from_utf8(r.stdout).ok()
            } else {
                None
            }
        })
}
