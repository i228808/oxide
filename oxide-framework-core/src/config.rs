use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub app_name: String,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    3000
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            app_name: String::from("oxide-app"),
        }
    }
}

impl AppConfig {
    /// Load config from a YAML file, then overlay any matching environment
    /// variables. Missing file is not an error — defaults are used instead.
    pub fn load(path: Option<&str>) -> Self {
        let mut cfg = match path {
            Some(p) if Path::new(p).exists() => {
                let contents = std::fs::read_to_string(p)
                    .unwrap_or_else(|e| panic!("failed to read config file {p}: {e}"));
                serde_yaml::from_str(&contents)
                    .unwrap_or_else(|e| panic!("failed to parse config file {p}: {e}"))
            }
            _ => Self::default(),
        };

        // Environment variable overrides (OXIDE_ prefix)
        if let Ok(v) = std::env::var("OXIDE_HOST") {
            cfg.host = v;
        }
        if let Ok(v) = std::env::var("OXIDE_PORT")
            && let Ok(port) = v.parse::<u16>()
        {
            cfg.port = port;
        }
        if let Ok(v) = std::env::var("OXIDE_APP_NAME") {
            cfg.app_name = v;
        }

        cfg
    }
}

