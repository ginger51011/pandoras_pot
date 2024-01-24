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
    /// Routes to be handled. Is overriden by `catch_all`.
    #[serde(default)]
    pub routes: Vec<String>,
    /// If all routes are to be served.
    #[serde(default)]
    pub catch_all: bool,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            routes: vec![String::from("/")],
            catch_all: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GeneratorConfig {
    // The minimum possible length of a generated string segment
    #[serde(default)]
    pub min_chunk_size: usize,

    // The maximum possible length of a generated string segment
    #[serde(default)]
    pub max_chunk_size: usize,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            min_chunk_size: 1024,
            max_chunk_size: 8000,
        }
    }
}
