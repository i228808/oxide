//! `oxide bench` — Criterion + load test in Oxide repo, else `cargo bench`.

use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};

pub fn run_bench(cargo_args: &[String]) -> Result<()> {
    let in_oxide_repo = Path::new("oxide_core/benches/overhead.rs").exists();

    if in_oxide_repo {
        // Criterion
        let mut criterion = Command::new("cargo");
        criterion
            .arg("bench")
            .arg("-p")
            .arg("oxide_core")
            .arg("--bench")
            .arg("overhead");
        if !cargo_args.is_empty() {
            criterion.args(cargo_args);
        }
        criterion
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        let status = criterion
            .status()
            .with_context(|| "failed to run `cargo bench`")?;
        if !status.success() {
            std::process::exit(status.code().unwrap_or(1));
        }

        println!();
        println!("--- Load test (loadtest example) ---");

        let mut load = Command::new("cargo");
        load.arg("run")
            .arg("-p")
            .arg("oxide_core")
            .arg("--release")
            .arg("--example")
            .arg("loadtest");
        load.stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        let status = load
            .status()
            .with_context(|| "failed to run loadtest example")?;
        if !status.success() {
            std::process::exit(status.code().unwrap_or(1));
        }
        return Ok(());
    }

    let mut cmd = Command::new("cargo");
    cmd.arg("bench");
    if !cargo_args.is_empty() {
        cmd.args(cargo_args);
    }
    cmd.stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    let status = cmd.status().with_context(|| "failed to run `cargo bench`")?;
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}
