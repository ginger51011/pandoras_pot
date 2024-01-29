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
    #[serde(default = "default_http_port")]
    pub port: String,
    /// Routes to be handled. Is overriden by `http.catch_all`.
    #[serde(default = "default_http_routes")]
    pub routes: Vec<String>,
    /// If all routes are to be served.
    #[serde(default = "default_http_catch_all")]
    pub catch_all: bool,
    /// How many connections that can be made over `http.rate_limit_period` seconds. Will
    /// not set any limit if set to 0.
    #[serde(default = "default_http_rate_limit")]
    pub rate_limit: u64,
    /// Amount of seconds that `http.rate_limit` checks on. Does nothing if rate limit is set
    /// to 0.
    #[serde(default = "default_http_rate_limit_period")]
    pub rate_limit_period: u64,
    /// Enables `http.health_port` to be used for health checks (to see if `pandoras_pot`).
    /// Useful if you want to use your chad gaming PC that might not always be up and running
    /// to back up an instance running on your RPi 3 web server.
    #[serde(default = "default_http_health_port_enabled")]
    pub health_port_enabled: bool,
    /// Port to be used for health checks. Should probably not be accessible from the
    /// outside. Has no effect if `http.health_port_enabled` is `false`.
    #[serde(default = "default_http_health_port")]
    pub health_port: String,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            port: default_http_port(),
            routes: default_http_routes(),
            catch_all: default_http_catch_all(),
            rate_limit: default_http_rate_limit(),
            rate_limit_period: default_http_rate_limit(),
            health_port_enabled: default_http_health_port_enabled(),
            health_port: default_http_health_port(),
        }
    }
}

// Note naming convention for these

fn default_http_port() -> String {
    "8080".to_string()
}

fn default_http_routes() -> Vec<String> {
    vec!["/".to_string()]
}

fn default_http_catch_all() -> bool {
    true
}

fn default_http_rate_limit() -> u64 {
    0
}

fn default_http_rate_limit_period() -> u64 {
    // 5 minutes
    5 * 60
}

fn default_http_health_port_enabled() -> bool {
    false
}

fn default_http_health_port() -> String {
    "8081".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GeneratorConfig {
    /// The size of each generated chunk in bytes. Has a big impact on performance, so
    /// play around a bit!
    #[serde(default = "default_generator_chunk_size")]
    pub chunk_size: usize,

    /// The type of generator to be used
    #[serde(default = "default_generator_generator_type")]
    #[serde(rename = "type")]
    pub generator_type: GeneratorType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "name", content = "data")]
#[serde(rename_all = "snake_case")]
pub(crate) enum GeneratorType {
    Random,
    /// Markov chain that also contains a path to the text to be used for generation
    MarkovChain(PathBuf),
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            chunk_size: default_generator_chunk_size(),
            generator_type: default_generator_generator_type(),
        }
    }
}

// Note naming convention for these

fn default_generator_chunk_size() -> usize {
    1024 * 16
}

fn default_generator_generator_type() -> GeneratorType {
    GeneratorType::Random
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct LoggingConfig {
    /// Output file for logs.
    #[serde(default = "default_logging_output_path")]
    pub output_path: String,

    /// If pretty logs should be written to standard output.
    #[serde(default = "default_logging_print_pretty_logs")]
    pub print_pretty_logs: bool,

    /// If no logs at all should be printed to stdout. Overrides other stdout logging
    /// settings.
    #[serde(default = "default_logging_no_stdout")]
    pub no_stdout: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            output_path: default_logging_output_path(),
            print_pretty_logs: default_logging_print_pretty_logs(),
            no_stdout: default_logging_no_stdout(),
        }
    }
}

// Note naming convention for these

fn default_logging_output_path() -> String {
    "pandoras.log".to_string()
}

fn default_logging_print_pretty_logs() -> bool {
    true
}

fn default_logging_no_stdout() -> bool {
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

    #[test]
    fn deserialize_markov_chain_generator_config() {
        let toml_str = r#"
            [generator]
            type = { name = "markov_chain", data = "/some/random/path" }
        "#;
        toml::from_str::<Config>(toml_str).unwrap();
    }

    #[test]
    fn deserialize_random_generator_config() {
        let toml_str = r#"
            [generator]
            type = { name = "random" }
        "#;
        toml::from_str::<Config>(toml_str).unwrap();
    }

    #[test]
    fn deserialize_config_1() {
        let toml_str = r#"
            [http]
            port = "7796"
            routes = ["/wp-login.php"]
            catch_all = false

            [generator]
            min_chunk_size = 400
            max_chunk_size = 500
            type = { name = "markov_chain", data = "/home/whatever/kladd/markovseed.txt" }
        "#;
        toml::from_str::<Config>(toml_str).unwrap();
    }

    #[test]
    fn deserialize_config_2() {
        let toml_str = r#"
            [http]
            # Make sure this matches your Dockerfile's "EXPOSE" if using Docker
            port = "8080"
            # Routes to send misery to. Is overridden by `http.catch_all`
            routes = ["/wp-login.php", "/.env"]
            # If all routes are to be served.
            catch_all = true
            # How many connections that can be made over `http.rate_limit_period` seconds. Will
            # not set any limit if set to 0.
            rate_limit = 0
            # Amount of seconds that `http.rate_limit` checks on. Does nothing if rate limit is set
            # to 0.
            rate_limit_period = 300 # 5 minutes

            [generator]
            chunk_size = 1024
            # The type of generator to be used
            type = { name = "random" }

            # For generator.type it is also possible to set a markov chain generator, using
            # a text file as a source of data. Then you can use this (but uncommented, duh):
            # type = { name = "markov_chain", data = "/rootvalue.txt" }

            [logging]
            # Output file for logs.
            output_path = "pandoras.log"

            # If pretty logs should be written to standard output.
            print_pretty_logs = true

            # If no logs at all should be printed to stdout. Overrides other stdout logging
            # settings.
            no_stdout = false
        "#;
        toml::from_str::<Config>(toml_str).unwrap();
    }
}
