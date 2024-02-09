//! This module contains structures to create a generator used for data creation.

pub(crate) mod markov_generator;
pub(crate) mod random_generator;
pub(crate) mod static_generator;

use std::sync::Arc;

use crate::config::GeneratorConfig;
use futures::Stream;
use tokio::sync::{mpsc::Receiver, Semaphore};

use self::{
    markov_generator::MarkovChainGenerator, random_generator::RandomGenerator,
    static_generator::StaticGenerator,
};

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
/// outputting infinite amounts of very useful strings.
pub trait Generator
where
    Self: Sync + Iterator<Item = String> + Clone + Send + 'static,
{
    /// Creates the generator from a config.
    fn from_config(config: GeneratorConfig) -> Self;

    /// Retrieves a semaphore used as a permit to start generating values.
    fn permits(&self) -> Arc<Semaphore>;

    /// Returns an infinite stream using this generator, prepending `<html><body>\n` to the
    /// first chunk.
    fn into_receiver(self) -> Receiver<String> {
        // To provide accurate stats, the buffer must be 1
        let (tx, rx) = tokio::sync::mpsc::channel(1);

        tokio::spawn(async move {
            let _permit = self.permits().acquire_owned().await.unwrap();

            tracing::debug!(
                "Acquired permit to generate, {} permits left",
                self.permits().available_permits()
            );

            // Prepend so it kind of looks like a valid website
            let mut value_iter = ["<html><body>\n".to_string()].into_iter().chain(self);
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

#[cfg(test)]
mod tests {
    use core::time::Duration;
    use std::thread;

    use tokio::sync::mpsc::error::TryRecvError;

    use super::Generator;

    /// The duration the sender to a [`Generator::into_receiver()`] is absolutely
    /// guaranteed to have acquired a permit and sent its first message.
    const SENDER_WARMUP_DURATION: Duration = Duration::from_millis(5);

    /// Verifies that a generator is limited to a specified amount of concurrent generators.
    ///
    /// Tests calling this _must_ have the `#[tokio::test(flavor = "multi_threaded")]` annotation,
    /// otherwise no thread will ever make senders produce output.
    pub(crate) fn test_generator_is_limited(gen: impl Generator, limit: usize) -> bool {
        let mut receivers = Vec::with_capacity(limit);
        for _ in 0..limit {
            let g = gen.clone();
            let r = g.into_receiver();
            receivers.push(r);
        }

        thread::sleep(SENDER_WARMUP_DURATION);

        // Ensure all receivers have sent their first message
        for r in &mut receivers {
            let _ = r
                .try_recv()
                .expect(format!("Receiver within limit have not sent message").as_str());
        }

        // If we now attempt to use the original generator, it
        // should be blocked (since we are still holding on to active
        // receivers)
        let mut r = gen.into_receiver();

        // This should be instant, but we give it some time
        thread::sleep(SENDER_WARMUP_DURATION);

        // We want this to be blocked
        let res = match r.try_recv() {
            Ok(_) => false,
            Err(TryRecvError::Disconnected) => false,
            Err(TryRecvError::Empty) => true,
        };

        // So we can be completely sure that no generators
        // were dropped until now
        std::mem::drop(receivers);

        res
    }
}
