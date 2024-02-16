//! This module contains structures to create a generator used for data creation.

pub(crate) mod markov_generator;
pub(crate) mod random_generator;
pub(crate) mod static_generator;

use std::{
    fmt::Debug,
    sync::Arc,
    time::{self, Duration},
};

use crate::config::GeneratorConfig;
use bytes::{Bytes, BytesMut};
use futures::Stream;
use tokio::sync::{mpsc::Receiver, Semaphore};

use self::{
    markov_generator::MarkovChainGenerator, random_generator::RandomGenerator,
    static_generator::StaticGenerator,
};

/// Size of wrapping a string in a "<p>\n{<yourstring>}\n</p>\n"
const P_TAG_SIZE: usize = 0xA;

/// Container for generators
#[derive(Clone, Debug)]
pub(crate) enum GeneratorContainer {
    Random(RandomGenerator),
    MarkovChain(MarkovChainGenerator),
    Static(StaticGenerator),
}

/// Trait that describes a generator that can be converted to a stream,
/// outputting infinite amounts of very useful strings.
pub trait Generator
where
    Self: Sync + Iterator<Item = Bytes> + Clone + Send + 'static + Debug,
{
    /// Creates the generator from a config.
    fn from_config(config: GeneratorConfig) -> Self;

    /// Retrieve the config used for this generator.
    fn config(&self) -> &GeneratorConfig;

    /// Retrieves a semaphore used as a permit to start generating values.
    fn permits(&self) -> Arc<Semaphore>;

    /// Returns an infinite stream using this generator, prepending `<html><body>\n` to the
    /// first chunk.
    fn into_receiver(mut self) -> Receiver<Bytes> {
        // To provide accurate stats, the buffer must be 1
        let (tx, rx) = tokio::sync::mpsc::channel::<Bytes>(1);

        tokio::spawn(async move {
            let _permit = self.permits().acquire_owned().await.unwrap();

            tracing::debug!(
                "Acquired permit to generate, {} permits left",
                self.permits().available_permits()
            );

            // Prepend so it kind of looks like a valid website
            let mut bytes_written = 0_usize;

            // For the first value we want to prepend something to make it look like HTML.
            // We don't want to just chain it, because then the first chunk of the body always
            // looks the same.
            let mut first_msg = BytesMut::from("<html><body>");
            first_msg.extend(self.next().expect("next returned None"));
            let first_msg_size = first_msg.len();
            let start_time = time::SystemTime::now();
            match tx.send(first_msg.freeze()).await {
                Ok(_) => bytes_written += first_msg_size,
                Err(_) => {
                    tracing::info!("Stream broken before first message could be sent");
                    return;
                }
            }

            // Don't want to call `self.config()` over and over
            let time_limit = self.config().time_limit;
            let time_limit_duration = Duration::from_secs(time_limit);
            let size_limit = self.config().size_limit;
            loop {
                // `0` means no limit

                // If system time is messed up, assume no time has passed
                if time_limit != 0
                    && (start_time.elapsed().unwrap_or(Duration::from_secs(0))
                        > time_limit_duration)
                {
                    tracing::info!("Time limit was reached ({} s), breaking stream", time_limit,);
                    return;
                }

                if size_limit != 0 && bytes_written >= size_limit {
                    tracing::info!(
                        "Size limit was reached ({} MB, {} GB)",
                        (bytes_written as f64) * 1e-6,
                        (bytes_written as f64) * 1e-9
                    );
                    return;
                }

                // Limits were find, produce some data
                let s = self.next().expect("next returned None");

                // The size may be dynamic if the generator does not have a strict
                // chunk size
                let s_size = s.len();
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

    fn into_stream(self) -> impl Stream<Item = Bytes> {
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
