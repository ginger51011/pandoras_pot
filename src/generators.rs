//! This module contains structures to create a generator used for data creation.

pub(crate) mod markov;
pub(crate) mod random;

use crate::config::GeneratorConfig;
use futures::Stream;
use tokio::sync::mpsc::Receiver;

use self::{markov::MarkovChainGenerator, random::RandomGenerator};

const GENERATOR_CHANNEL_BUFFER: usize = 2;

/// Size of wrapping a string in a "<p>\n{<yourstring>}\n</p>\n"
const P_TAG_SIZE: usize = 0xA;

/// Container for generators
#[derive(Clone)]
pub(crate) enum GeneratorContainer {
    Random(RandomGenerator),
    MarkovChain(MarkovChainGenerator),
}

/// Trait that describes a generator that can be converted to a stream,
/// outputting (probably infinite) amounts of very useful strings.
pub trait Generator
where
    Self: Sync + Iterator<Item = String> + Clone + Send + 'static,
{
    /// Creates the generator from a config.
    fn from_config(config: GeneratorConfig) -> Self;

    /// Returns an infinite stream using this generator.
    fn into_receiver(mut self) -> Receiver<String> {
        let (tx, rx) = tokio::sync::mpsc::channel(GENERATOR_CHANNEL_BUFFER);

        tokio::spawn(async move {
            let mut bytes_written = 0_usize;
            loop {
                let s = self.next().expect("next returned None");
                let s_size = std::mem::size_of_val(&s);
                match tx.send(s).await {
                    Ok(_) => bytes_written += s_size,
                    Err(_) => {
                        // TODO: Add metadata
                        tracing::info!(
                            "Stream broken, wrote {} MB, or {} GB",
                            (bytes_written as f64) * 1e-6,
                            (bytes_written as f64) * 1e-9
                        );
                        break;
                    }
                }
            }
        });

        rx
    }

    fn into_stream(self) -> impl Stream<Item = String> {
        tokio_stream::wrappers::ReceiverStream::new(self.into_receiver())
    }
}
