//! Oxide CLI — scaffold projects, generate code, run tooling.

use anyhow::Result;
use clap::{Parser, Subcommand};

mod bench_cmd;
mod generate;
mod migrate_cmd;
mod new_project;
mod run_cmd;
mod test_cmd;

#[derive(Parser)]
#[command(name = "oxide")]
#[command(about = "Oxide web framework CLI", version, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new Oxide application in `<name>/`
    New {
        /// Project directory name (created under the current working directory)
        name: String,
        /// Overwrite existing directory (use with care)
        #[arg(long)]
        force: bool,
        /// `oxide_framework_core` dependency: `path=../oxide-framework-core` or `version=0.1`
        #[arg(long, default_value = "path=../oxide-framework-core")]
        oxide: String,
    },
    /// Generate controllers, routes, or middleware stubs
    Generate {
        #[command(subcommand)]
        command: GenerateCmd,
    },
    /// Run the current crate (`cargo run`) with optional env vars
    Run {
        /// Extra arguments passed to `cargo run` (use `oxide run -- --release`)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        cargo_args: Vec<String>,
    },
    /// Run tests (`cargo test` in app dir, or Oxide workspace tests from repo root)
    Test {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        cargo_args: Vec<String>,
    },
    /// Run Criterion + load-test benchmarks (Oxide repo) or `cargo bench` in app
    Bench {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        cargo_args: Vec<String>,
    },
    /// Database migrations (reserved — not implemented yet)
    Migrate {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Subcommand)]
enum GenerateCmd {
    /// Generate a controller skeleton (`#[controller]` or functional `OxideRouter`)
    Controller {
        /// Rust struct name, e.g. `User` or `UserController`
        name: String,
        /// URL prefix for routes, e.g. `/api/users`
        #[arg(long, default_value = "/api")]
        prefix: String,
        /// `macro` = `#[controller]` impl block; `functional` = `OxideRouter` module
        #[arg(long, default_value = "macro")]
        style: String,
        /// Overwrite existing file
        #[arg(long)]
        force: bool,
    },
    /// Append a route method to an existing controller file
    Route {
        /// Controller struct name, e.g. `UserController`
        controller: String,
        /// HTTP method, e.g. `GET`, `POST`
        method: String,
        /// Route path, e.g. `/` or `/items/{id}`
        path: String,
        /// Skip duplicate detection (same path + method)
        #[arg(long)]
        force: bool,
    },
    /// Stub for custom middleware (Tower layer) — creates `src/middleware/<name>.rs`
    Middleware {
        /// Module name in snake_case, e.g. `request_id`
        name: String,
        #[arg(long)]
        force: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::New { name, force, oxide } => new_project::run(&name, force, &oxide),
        Commands::Generate { command: cmd } => match cmd {
            GenerateCmd::Controller {
                name,
                prefix,
                style,
                force,
            } => generate::controller(&name, &prefix, &style, force),
            GenerateCmd::Route {
                controller,
                method,
                path,
                force,
            } => generate::route(&controller, &method, &path, force),
            GenerateCmd::Middleware { name, force } => generate::middleware(&name, force),
        },
        Commands::Run { cargo_args } => run_cmd::run(&cargo_args),
        Commands::Test { cargo_args } => test_cmd::run_tests(&cargo_args),
        Commands::Bench { cargo_args } => bench_cmd::run_bench(&cargo_args),
        Commands::Migrate { args } => migrate_cmd::run(&args),
    }
}
