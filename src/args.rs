//! Functions for handling function arguments.

use std::{io::Write, path::PathBuf};

use crate::{config::Config, error_code};

const VERSION: &str = concat!(env!("CARGO_CRATE_NAME"), " ", env!("CARGO_PKG_VERSION"));
const HELP: &str = r#"pandoras_pot
https://github.com/ginger51011/pandoras_pot

High performance HTTP honeypot to punish unruly web crawlers with support for several different
generator types for the infinite HTTP response. To use another generator type, you can use the
following configuration for generator.type:

type = { name = "markov_chain", data = "<path to some text file>" }
or
type = { name = "static", data = "<path to some file>" }

More configuration options are listed in the project README.

USAGE:
  pandoras_pot [FLAGS] [CONFIG]

ARGS:
  [CONFIG]
    Configuration to use. If not provided, the default configuration path will be checked. If no
    configuration is found, the default configuration will be used instead. All configuration
    values are optional, and will fall back to a default value.

FLAGS:
  -h, --help                        Print help information and exit
  -V, --version                     Print version information and exit
      --print-default-config        Print default configuration and exit

AUTHOR:
  Written by Emil Eriksson (github.com/ginger51011)"#;

/// Parses arguments, and an optional provided [`Config`], or an exit code that should be used.
/// Writes all output to the provided writer.
///
/// Will print helpful information, so the caller should preferably exit using the provided code
/// immediately if possible.
///
/// # Examples
///
/// ```
//# use crate::{args::parse_args, config::Config};
/// // Note: Please check the result of parse_args
/// let pargs = pico_args::Arguments::from_env();
/// let config: Config = parse_args(pargs, &mut std::io::stdout()).unwrap();
/// ```
pub(crate) fn parse_args<W: Write>(
    mut pargs: pico_args::Arguments,
    output_writer: &mut W,
) -> Result<Option<Config>, i32> {
    if pargs.contains(["-h", "--help"]) {
        writeln!(output_writer, "{HELP}").map_err(|_| error_code::UNKNOWN_ERROR)?;
        return Err(0);
    } else if pargs.contains(["-V", "--version"]) {
        writeln!(output_writer, "{VERSION}",).map_err(|_| error_code::UNKNOWN_ERROR)?;
        return Err(0);
    } else if pargs.contains("--print-default-config") {
        let toml = toml::to_string_pretty(&Config::default())
            .expect("should be able to serialize default config");
        write!(output_writer, "{toml}").map_err(|_| error_code::UNKNOWN_ERROR)?;
        return Err(0);
    }

    let remaining = pargs.finish();

    if remaining.is_empty() {
        Ok(None)
    } else if remaining.len() == 1 {
        let possible_path = &remaining[0];
        let pb = PathBuf::from(possible_path);
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
        writeln!(output_writer, "{HELP}").map_err(|_| error_code::UNKNOWN_ERROR)?;
        Err(error_code::ARGUMENT_ERROR)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::NamedTempFile;

    use crate::{config::Config, error_code};

    use super::{parse_args, HELP, VERSION};

    #[test]
    fn no_args_ok() {
        let pargs = pico_args::Arguments::from_vec(vec![]);
        let mut buf: Vec<u8> = vec![];
        let res = parse_args(pargs, &mut buf);
        assert!(buf.is_empty());
        match res {
            Ok(None) => {
                // Ok
            }
            Ok(Some(_)) => panic!("got a config"),
            Err(_) => panic!("got exit code"),
        }
    }

    #[test]
    fn two_bad_arg_prints_help() {
        let pargs =
            pico_args::Arguments::from_vec(vec!["--bad-word".into(), "--another-one".into()]);
        let mut buf: Vec<u8> = vec![];
        let res = parse_args(pargs, &mut buf);
        assert_eq!(String::from_utf8(buf).unwrap(), format!("{HELP}\n"));
        match res {
            Err(error_code::ARGUMENT_ERROR) => {
                // Ok
            }
            Err(_) => panic!("wrong exit code"),
            Ok(_) => panic!("did not get exit code"),
        }
    }

    #[test]
    fn help_prints_help() {
        for flag in &["-h", "--help"] {
            let pargs = pico_args::Arguments::from_vec(vec![flag.into()]);
            let mut buf: Vec<u8> = vec![];
            let res = parse_args(pargs, &mut buf);
            assert_eq!(String::from_utf8(buf).unwrap(), format!("{HELP}\n"));
            match res {
                Err(0) => {
                    // Ok
                }
                Err(_) => panic!("wrong exit code"),
                Ok(_) => panic!("did not get exit code"),
            }
        }
    }

    #[test]
    fn version_prints_version() {
        for flag in &["-V", "--version"] {
            let pargs = pico_args::Arguments::from_vec(vec![flag.into()]);
            let mut buf: Vec<u8> = vec![];
            let res = parse_args(pargs, &mut buf);
            assert_eq!(String::from_utf8(buf).unwrap(), format!("{VERSION}\n"));
            match res {
                Err(0) => {
                    // Ok
                }
                Err(_) => panic!("wrong exit code"),
                Ok(_) => panic!("did not get exit code"),
            }
        }
    }

    #[test]
    fn help_has_priority() {
        let pargs = pico_args::Arguments::from_vec(vec!["-V".into(), "-h".into(), "--help".into()]);
        let mut buf: Vec<u8> = vec![];
        let res = parse_args(pargs, &mut buf);
        assert_eq!(String::from_utf8(buf).unwrap(), format!("{HELP}\n"));
        match res {
            Err(0) => {
                // Ok
            }
            Err(_) => panic!("wrong exit code"),
            Ok(_) => panic!("did not get exit code"),
        }
    }

    #[test]
    fn version_has_priority() {
        let pargs =
            pico_args::Arguments::from_vec(vec!["--print-default-config".into(), "-V".into()]);
        let mut buf: Vec<u8> = vec![];
        let res = parse_args(pargs, &mut buf);
        assert_eq!(String::from_utf8(buf).unwrap(), format!("{VERSION}\n"));
        match res {
            Err(0) => {
                // Ok
            }
            Err(_) => panic!("wrong exit code"),
            Ok(_) => panic!("did not get exit code"),
        }
    }

    #[test]
    fn print_default_prints_default() {
        let pargs = pico_args::Arguments::from_vec(vec!["--print-default-config".into()]);
        let mut buf: Vec<u8> = vec![];
        let res = parse_args(pargs, &mut buf);
        let toml =
            toml::to_string_pretty(&Config::default()).expect("failed to serialize default config");
        assert_eq!(String::from_utf8(buf).unwrap(), toml);
        match res {
            Err(0) => {
                // Ok
            }
            Err(_) => panic!("wrong exit code"),
            Ok(_) => panic!("did not get exit code"),
        }
    }

    #[test]
    fn config_argument_is_parsed() {
        // First write a custom config to a file
        let mut tmpfile: NamedTempFile = NamedTempFile::new().unwrap();
        let mut written_config = Config::default();
        written_config.http.health_port = "1".to_string();
        assert_ne!(
            written_config,
            Config::default(),
            "this test is wrong if this fails"
        );
        let toml = toml::to_string_pretty(&written_config).unwrap();
        tmpfile.write_all(toml.as_bytes()).unwrap();

        let pargs = pico_args::Arguments::from_vec(vec![tmpfile.path().into()]);
        let mut buf: Vec<u8> = vec![];
        let res = parse_args(pargs, &mut buf);

        match res {
            Ok(Some(parsed_config)) => {
                assert_eq!(
                    parsed_config, written_config,
                    "written and parsed config do not match!"
                );
            }
            Ok(None) => panic!("did not parse config!"),
            Err(_) => panic!("got exit code!"),
        }
    }
}
