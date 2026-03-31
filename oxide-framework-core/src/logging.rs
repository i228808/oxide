use tracing_subscriber::{EnvFilter, fmt};

/// Initialize structured logging. Safe to call multiple times — only the
/// first call installs the subscriber; subsequent calls are no-ops.
pub fn init() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = fmt().with_env_filter(filter).with_target(true).try_init();
}

