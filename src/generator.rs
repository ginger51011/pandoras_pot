//! This module contains structures to create a generator used for data creation.

pub(crate) mod markov_generator;
pub(crate) mod random_generator;
pub(crate) mod static_generator;

use crate::config::GeneratorConfig;
use futures::Stream;
use tokio::sync::{mpsc::Receiver, Semaphore};

use self::{
    markov_generator::MarkovChainGenerator, random_generator::RandomGenerator,
    static_generator::StaticGenerator,
};

// TODO: Make configurable
///.Max amounts of generators. Currently hardcoded to avoid abuse.
static GENERATOR_PERMITS: Semaphore = Semaphore::const_new(100);

/// Size of wrapping a string in a "<p>\n{<yourstring>}\n</p>\n"
const P_TAG_SIZE: usize = 0xA;

/// Container for generators
#[derive(Clone)]
pub(crate) enum GeneratorContainer {
    Random(RandomGenerator),
    MarkovChain(MarkovChainGenerator),
    Static(StaticGenerator),
}

/// Trait that describes a generator that can be converted to a stream,
/// outputting (probably infinite) amounts of very useful strings.
pub trait Generator
where
    Self: Sync + Iterator<Item = String> + Clone + Send + 'static,
{
    /// Creates the generator from a config.
    fn from_config(config: GeneratorConfig) -> Self;

    /// Returns an infinite stream using this generator, prepending `<html><body>` to the
    /// first chunk.
    fn into_receiver(self) -> Receiver<String> {
        // To provide accurate stats, the buffer must be 1
        let (tx, rx) = tokio::sync::mpsc::channel(1);

        tokio::spawn(async move {
            let _permit = GENERATOR_PERMITS.acquire().await.unwrap();

            // Prepend so it kind of looks like a valid website
            let mut value_iter = ["<html><body>".to_string()].into_iter().chain(self);
            let mut bytes_written = 0_usize;
            loop {
                let s = value_iter.next().expect("next returned None");

                // The size may be dynamic if the generator does not have a strict
                // chunk size
                let s_size = s.as_bytes().len();
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