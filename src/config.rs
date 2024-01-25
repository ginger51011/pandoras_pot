//! This module contains the types used for configuration.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Configuration for `pandoras_pot`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct Config {
    /// Configuration related to HTTP server.
    #[serde(default)]
    pub http: HttpConfig,

    /// Configuration related to generator creating HTML.
    #[serde(default)]
    pub generator: GeneratorConfig,

    /// Configuration related to logs.
    pub logging: LoggingConfig,
}

impl Config {
    pub fn default_path() -> Option<PathBuf> {
        let mut dir = home::home_dir()?;
        dir.push(".config/pandoras_pot/config.toml");
        Some(dir)
    }

    pub fn from_path(path: &Path) -> Option<Self> {
        let toml = std::fs::read_to_string(path).ok()?;
        toml::from_str(&toml).ok()
    }

    pub fn read_from_default_path() -> Option<Self> {
        if let Some(path) = Self::default_path() {
            Self::from_path(&path)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct HttpConfig {
    /// Port to listen on.
    #[serde(default = "default_port")]
    pub port: String,
    /// Routes to be handled. Is overriden by `catch_all`.
    #[serde(default = "default_routes")]
    pub routes: Vec<String>,
    /// If all routes are to be served.
    #[serde(default = "default_catch_all")]
    pub catch_all: bool,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            routes: default_routes(),
            catch_all: default_catch_all(),
        }
    }
}

fn default_port() -> String {
    "8080".to_string()
}

fn default_routes() -> Vec<String> {
    vec!["/".to_string()]
}

fn default_catch_all() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GeneratorConfig {
    /// The minimum possible length of a generated string segment
    #[serde(default = "default_min_chunk_size")]
    pub min_chunk_size: usize,

    /// The maximum possible length of a generated string segment
    #[serde(default = "default_max_chunk_size")]
    pub max_chunk_size: usize,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            min_chunk_size: default_min_chunk_size(),
            max_chunk_size: default_max_chunk_size(),
        }
    }
}

fn default_min_chunk_size() -> usize {
    1024
}

fn default_max_chunk_size() -> usize {
    8000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct LoggingConfig {
    /// Output file for logs. Will not write to logs if
    /// disabled.
    #[serde(default = "default_output_path")]
    pub output_path: Option<String>,

    /// If pretty logs should be written to standard output.
    #[serde(default = "default_print_pretty_logs")]
    pub print_pretty_logs: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            output_path: default_output_path(),
            print_pretty_logs: default_print_pretty_logs(),
        }
    }
}

fn default_output_path() -> Option<String> {
    None
}

fn default_print_pretty_logs() -> bool {
    true
}
