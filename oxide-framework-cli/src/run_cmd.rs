//! `oxide run` — `cargo run` with forwarded args.

use anyhow::{Context, Result};
use std::process::{Command, Stdio};

pub fn run(cargo_args: &[String]) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.arg("run");
    if !cargo_args.is_empty() {
        cmd.args(cargo_args);
    }
    cmd.stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let status = cmd
        .status()
        .with_context(|| "failed to spawn `cargo` — is Rust installed and on PATH?")?;
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}
