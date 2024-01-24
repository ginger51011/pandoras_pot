//! This module contains the types used for configuration.
use serde::{Deserialize, Serialize};

const DEFAULT_CONFIG_PATH: &str = concat!(env!("CARGO_HOME"), "/config/pandoras_pot/config.toml");

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct Config {
    pub http: HttpConfig,
    pub generator: GeneratorConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct HttpConfig {
    /// Routes to be handled. Is overriden by `catch_all`.
    pub routes: Vec<String>,
    /// If all routes are to be served.
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
    pub min_chunk_size: usize,

    // The maximum possible length of a generated string segment
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
