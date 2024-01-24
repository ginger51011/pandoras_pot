//! This module contains the types used for configuration.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use home;
use toml;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct Config {
    #[serde(default)]
    pub http: HttpConfig,
    #[serde(default)]
    pub generator: GeneratorConfig,
}

impl Config {
    pub fn default_path() -> Option<PathBuf> {
        let mut dir = home::home_dir()?;
        dir.push(".config/pandoras_pot/config.toml");
        Some(dir)
    }

    pub fn from_path(path: &Path) -> Option<Self> {
        let toml = std::fs::read_to_string(path).ok()?;
        let config = toml::from_str(&toml).ok()?;
        config
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
    // The minimum possible length of a generated string segment
    #[serde(default = "default_min_chunk_size")]
    pub min_chunk_size: usize,

    // The maximum possible length of a generated string segment
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
