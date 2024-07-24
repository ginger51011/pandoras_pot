use std::{fs, path::Path, process::exit};

use tokio::sync::mpsc;

use bytes::Bytes;
use tracing::instrument;

use crate::error_code;

use super::GeneratorStrategy;

/// A generator that always returns the same string.
#[derive(Clone, Debug)]
pub(crate) struct Static {
    data: Bytes,
}

impl Static {
    pub fn new(input: &Path) -> Self {
        let data = fs::read_to_string(input).unwrap_or_else(|_| {
            println!("Data for static generator must be a path to a readable file.");
            exit(error_code::CANNOT_READ_GENERATOR_DATA_FILE);
        });
        Self {
            data: Bytes::from(data),
        }
    }
}

impl GeneratorStrategy for Static {
    #[instrument(name = "spawn_static", skip(self))]
    fn spawn(self, buffer_size: usize) -> mpsc::Receiver<Bytes> {
        let (tx, rx) = mpsc::channel(buffer_size);
        let span = tracing::Span::current();
        tokio::task::spawn_blocking(move || loop {
            let _entered = span.enter();
            loop {
                if tx.blocking_send(self.data.clone()).is_err() {
                    break;
                }
            }
        });
        rx
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::NamedTempFile;

    use crate::{
        config::{GeneratorConfig, GeneratorType},
        generator::{static_strategy::Static, tests::test_generator_is_limited},
    };

    #[tokio::test(flavor = "multi_thread")]
    async fn static_generator_limits() {
        let mut tmpfile: NamedTempFile = tempfile::NamedTempFile::new().unwrap();
        write!(tmpfile, "I am but a little chain. I do chain things.").unwrap();

        for limit in 1..100 {
            let gen_config = GeneratorConfig::new(
                20,
                GeneratorType::Static(tmpfile.path().to_path_buf()),
                limit,
                0,
                0,
            );
            let gen = Static::new(0); // TODO
            assert!(
                test_generator_is_limited(gen, limit),
                "last generator could produce output while blocked"
            );
        }
    }
}
