//! This module contains the types used for configuration.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::generators::GeneratorType;

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
    #[serde(default)]
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

    /// The type of generator to be used
    #[serde(default = "default_generator_type")]
    pub generator_type: GeneratorType,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            min_chunk_size: default_min_chunk_size(),
            max_chunk_size: default_max_chunk_size(),
            generator_type: default_generator_type(),
        }
    }
}

fn default_min_chunk_size() -> usize {
    1024
}

fn default_max_chunk_size() -> usize {
    8000
}

fn default_generator_type() -> GeneratorType {
    GeneratorType::Random
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct LoggingConfig {
    /// Output file for logs.
    #[serde(default = "default_output_path")]
    pub output_path: String,

    /// If pretty logs should be written to standard output.
    #[serde(default = "default_print_pretty_logs")]
    pub print_pretty_logs: bool,

    /// If no logs at all should be printed to stdout. Overrides other stdout logging
    /// settings.
    #[serde(default = "default_no_stdout")]
    pub no_stdout: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            output_path: default_output_path(),
            print_pretty_logs: default_print_pretty_logs(),
            no_stdout: default_no_stdout(),
        }
    }
}

fn default_output_path() -> String {
    "pandoras.log".to_string()
}

fn default_print_pretty_logs() -> bool {
    true
}

fn default_no_stdout() -> bool {
    false
}

#[cfg(test)]
mod test {
    use super::Config;

    #[test]
    fn deserialize_incomplete_config() {
        let toml_str = r#"
            [http]
            port = "7796"
            routes = ["/wp-login.php"]
            catch_all = false

            [generator]
            min_chunk_size = 8000
            max_chunk_size = 16000
        "#;

        toml::from_str::<Config>(toml_str).unwrap();
    }

    #[test]
    fn deserialize_empty_config() {
        toml::from_str::<Config>("").unwrap();
    }
}
