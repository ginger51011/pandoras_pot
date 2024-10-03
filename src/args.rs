//! Functions for handligng function arguments.

use std::{ffi::OsStr, path::PathBuf};

use crate::{config::Config, error_code};

const HELP: &str = r#"pandoras_pot
https://github.com/ginger51011/pandoras_pot

High performance HTTP honeypot to punish unruly web crawlers with support for several different
generator types for the infinite HTTP response. To configure the generator type, you can use the
following configuration for [generator.type]:

type = { name = "markov_chain", data = "<path to some text file>" }
or
type = { name = "static", data = "<path to some file>" }

USAGE:
  pandoras_pot [FLAGS] [CONFIG]

ARGS:
  [CONFIG]
    Configuration to use. If not provided, the default
    configuration will be used instead.

FLAGS:
  -h, --help                        Print help information
  -V, --version                     Print help information
      --print-default-config        Print default configuration

AUTHOR:
  Written by Emil Jonathan Eriksson (github.com/ginger51011)

SUPPORT
  If you like this software, consider donating to an efficient charity. These websites provide
  excellent suggestions:
  https://givewell.org list
  https://www.founderspledge.com
  https://animalcharityevaluators.org"#;

pub(crate) fn parse_path(s: &OsStr) -> Result<PathBuf, &'static str> {
    Ok(s.into())
}

/// Parses arguments, and an optional provided [`Config`], or an exit code that should be used.
///
/// Will print helpful information, so the caller should preferably exit using the provided code
/// immediately if possible.
pub(crate) fn parse_args() -> Result<Option<Config>, i32> {
    let mut pargs = pico_args::Arguments::from_env();

    if pargs.contains(["-h", "--help"]) {
        println!("{HELP}");
        return Err(0);
    } else if pargs.contains(["-V", "--version"]) {
        println!("{} {}", env!("CARGO_CRATE_NAME"), env!("CARGO_PKG_VERSION"));
        return Err(0);
    } else if pargs.contains("--print-default-config") {
        let toml =
            toml::to_string_pretty(&Config::default()).expect("failed to serialize default config");
        println!("{toml}");
        return Err(0);
    }

    let remaining = pargs.finish();

    if remaining.is_empty() {
        Ok(None)
    } else if remaining.len() == 1 {
        let possible_path = &remaining[0];
        let pb = parse_path(possible_path).map_err(|e| {
            eprintln!(
                "Failed to parse path '{}' due to error: {e}",
                &possible_path.to_string_lossy()
            );
            error_code::ARGUMENT_ERROR
        })?;
        let c = Config::from_path(&pb);
        if let Some(actual) = c {
            Ok(Some(actual))
        } else {
            eprintln!(
                "File at '{}' could not be parsed as proper config",
                pb.to_string_lossy()
            );
            Err(error_code::UNPARSEABLE_CONFIG)
        }
    } else {
        println!("{HELP}");
        return Err(error_code::ARGUMENT_ERROR);
    }
}
