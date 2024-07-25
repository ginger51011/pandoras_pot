use std::{fs, path::Path, process::exit};

use tokio::sync::mpsc;

use bytes::Bytes;
use tracing::instrument;

use crate::error_code;

use super::GeneratorStrategy;

/// A generator strategy that always returns the same string.
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
    #[instrument(name = "spawn_static", skip_all)]
    fn start(self, tx: mpsc::Sender<Bytes>) {
        let span = tracing::Span::current();

        // Cloning a `Bytes` is very cheap, so this does not need to be blocking
        tokio::task::spawn(async move {
            let _entered = span.enter();
            loop {
                if tx.send(self.data.clone()).await.is_err() {
                    break;
                }
            }
        });
    }
}