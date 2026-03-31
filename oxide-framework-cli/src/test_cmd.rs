//! `oxide test` — workspace tests when in Oxide repo, else `cargo test`.

use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};

pub fn run_tests(cargo_args: &[String]) -> Result<()> {
    let in_oxide_repo = Path::new("oxide_framework_core/Cargo.toml").exists();

    let mut cmd = Command::new("cargo");
    if in_oxide_repo {
        cmd.arg("test").arg("--workspace");
    } else {
        cmd.arg("test");
    }
    if !cargo_args.is_empty() {
        cmd.args(cargo_args);
    }
    cmd.stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let status = cmd
        .status()
        .with_context(|| "failed to spawn `cargo test`")?;
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}

