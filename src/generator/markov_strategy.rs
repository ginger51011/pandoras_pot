use std::{fs, path::Path, process::exit, sync::Arc};

use bytes::Bytes;
use markovish::Chain;
use rand::{rngs::SmallRng, SeedableRng};
use tokio::sync::mpsc::{self};
use tracing::instrument;

use crate::error_code;

use super::{GeneratorStrategy, P_TAG_SIZE};

/// A generator using Markov chains to generate text. Due to the nature of
/// markov chains, each new generated piece of string may not exactly be
/// `chunk_size`, and might be a bit larger.
#[derive(Clone, Debug)]
pub(crate) struct MarkovChain {
    /// Chain used to generate responses
    chain: Arc<Chain>,
    chunk_size: usize,
}

impl MarkovChain {
    pub fn new(chunk_size: usize, input: &Path) -> Self {
        let content = fs::read_to_string(input).unwrap_or_else(|e| {
            println!(
                "Could not create Markov chain generator due to error:\n\t{}",
                e
            );
            exit(error_code::CANNOT_READ_GENERATOR_DATA_FILE);
        });

        let chain: Chain =
            Chain::from_text(&content).expect("could not create markov chain from file");

        Self {
            chain: Arc::new(chain),
            chunk_size,
        }
    }
}

impl GeneratorStrategy for MarkovChain {
    #[instrument(name = "spawn_markov_chain", skip(self))]
    fn spawn(self, buffer_size: usize) -> mpsc::Receiver<Bytes> {
        let (tx, rx) = mpsc::channel(buffer_size);
        let span = tracing::Span::current();
        tokio::task::spawn_blocking(move || {
            let _entered = span.enter();
            let desired_size = self.chunk_size - P_TAG_SIZE;

            loop {
                let mut result = String::with_capacity(desired_size + 100);
                let mut smol_rng = SmallRng::from_entropy();
                'outer: while result.as_bytes().len() < desired_size {
                    // We don't want to check result size every time, but we cannot know
                    // how large a token is. But most of them are (probably English) words,
                    // most words are 5 chars long and each English UTF-8 char
                    // is 1 byte. So we take a guess and see later.
                    let size_left = desired_size - result.as_bytes().len();
                    let likely_token_n = size_left / 5;

                    if likely_token_n == 0 {
                        break;
                    }

                    let generated = &self.chain.generate_str(&mut smol_rng, likely_token_n);
                    let generated_strs = match generated {
                        Some(s) => s,
                        None => {
                            tracing::error!("failed to generate string from chain");
                            continue;
                        }
                    };

                    // Cut off if we took too many
                    let mut current_size = 0;
                    for s in generated_strs {
                        result.push_str(s);
                        current_size += s.as_bytes().len();
                        if current_size > size_left {
                            break 'outer;
                        }
                    }
                }

                if tx.blocking_send(result.into()).is_err() {
                    break;
                }
            }
        });
        rx
    }
}
