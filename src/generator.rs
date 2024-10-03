//! This module contains structures to create a generator used for data creation using different
//! strategies.

pub(crate) mod markov_strategy;
pub(crate) mod random_strategy;
pub(crate) mod static_strategy;

use std::{
    fmt::Debug,
    sync::Arc,
    time::{self, Duration},
};

use crate::config::GeneratorConfig;
use bytes::{Bytes, BytesMut};
use futures::Stream;
use tokio::sync::{mpsc, Semaphore};
use tracing::Instrument;

use self::{markov_strategy::MarkovChain, random_strategy::Random, static_strategy::Static};

/// Size of wrapping a string in a "<p>\n{yourstring}\n</p>\n".
/// `generator.chunk_size` must be larger than this.
pub(crate) const P_TAG_SIZE: usize = 10;

/// Container for generators
#[derive(Clone, Debug)]
pub(crate) enum GeneratorStrategyContainer {
    Random(Random),
    MarkovChain(MarkovChain),
    Static(Static),
}

/// A strategy for genering helpful data for web crawlers.
///
/// Implementors should be _very_ cheap to clone, since [`GeneratorStrategy::start`] must take
/// ownership. This is to allow a strategy to only spawn a limited number of messages, or specific
/// messages in order in their [`GeneratorStrategy::start`] implementation.
pub trait GeneratorStrategy {
    /// Start generating using this strategy, filling the provided sender.
    ///
    /// This would generally mean passing the `tx` that to a _blocking_ tokio task generating
    /// data.
    ///
    /// Implementors **must** stop generating once the handle (the receiver of the channel) is
    /// dropped to avoid leaking resources.
    ///
    /// Implementors can, but do not have to, think about HTML. Note that the first message will be
    /// prefixed with config.generator.prefix.
    fn start(self, tx: mpsc::Sender<Bytes>);
}

/// Trait that describes a generator that can be converted to a stream, outputting infinite amounts
/// of very useful strings using a provided strategy.
///
/// Cheap to clone, as internals are wrapped in [`Arc`]. Does _not_ need to be wrapped in another
/// one.
#[derive(Debug, Clone)]
pub struct Generator {
    permits: Arc<Semaphore>,
    config: Arc<GeneratorConfig>,
}
impl Generator {
    pub fn from_config(config: Arc<GeneratorConfig>) -> Self {
        let permits = Arc::new(Semaphore::new(config.max_concurrent()));
        Self { permits, config }
    }

    fn permits(&self) -> Arc<Semaphore> {
        self.permits.clone()
    }

    /// Returns an infinite stream using this generator strategy, prepending generator.prefix to
    /// the first chunk.
    fn into_receiver<T>(self, strategy: T) -> mpsc::Receiver<Bytes>
    where
        T: GeneratorStrategy + Send + 'static,
    {
        // To provide accurate stats, the buffer must be 1
        let (tx, rx) = tokio::sync::mpsc::channel::<Bytes>(1);
        tokio::spawn(
            async move {
                let _permit = self.permits().acquire_owned().await.unwrap();
                tracing::debug!(
                    "Acquired permit to generate, {} permits left",
                    self.permits().available_permits()
                );

                let (gen_tx, mut gen) = mpsc::channel(self.config.chunk_buffer);
                strategy.start(gen_tx);

                // Prepend so it kind of looks like a valid website
                let mut bytes_written = 0_usize;

                // For the first value we want to prepend something to make it look like HTML.
                // We don't want to just chain it, because then the first chunk of the body always
                // looks the same.
                let mut first_msg = BytesMut::from(self.config.prefix.as_str());
                if let Some(first_gen) = gen.recv().await {
                    first_msg.extend(first_gen);
                } else {
                    return;
                }

                let first_msg_size = first_msg.len();
                let start_time = time::SystemTime::now();
                if tx.send(first_msg.freeze()).await.is_ok() {
                    bytes_written += first_msg_size;
                } else {
                    tracing::info!("Stream broken before first message could be sent");
                    return;
                };

                // Don't want to call `self.config()` over and over
                let time_limit = self.config.time_limit;
                let time_limit_duration = Duration::from_secs(time_limit);
                let size_limit = self.config.size_limit;
                loop {
                    // `0` means no limit

                    // If system time is messed up, assume no time has passed
                    if time_limit != 0
                        && (start_time.elapsed().unwrap_or(Duration::from_secs(0))
                            > time_limit_duration)
                    {
                        tracing::info!(
                            "Time limit was reached ({} s), breaking stream",
                            time_limit,
                        );
                        return;
                    }

                    if size_limit != 0 && bytes_written >= size_limit {
                        tracing::info!(
                            "Size limit was reached ({:.2} MB, {:.2} GB)",
                            (bytes_written as f64) * 1e-6,
                            (bytes_written as f64) * 1e-9
                        );
                        return;
                    }

                    // Limits were find, produce some data
                    let s = if let Some(s) = gen.recv().await {
                        s
                    } else {
                        return;
                    };

                    // The size may be dynamic if the generator does not have a strict
                    // chunk size
                    let s_size = s.len();
                    if tx.send(s).await.is_ok() {
                        bytes_written += s_size;
                    } else {
                        tracing::info!(
                            "Stream broken, wrote {:.2} MB, or {:.2} GB",
                            (bytes_written as f64) * 1e-6,
                            (bytes_written as f64) * 1e-9
                        );
                        break;
                    };
                }
            }
            .in_current_span(), // Ensure logging is made with request details
        );

        rx
    }

    pub fn into_stream<T>(self, strategy: T) -> impl Stream<Item = Bytes>
    where
        T: GeneratorStrategy + Send + 'static,
    {
        tokio_stream::wrappers::ReceiverStream::new(self.into_receiver(strategy))
    }
}

#[cfg(test)]
mod tests {
    use core::{panic, time::Duration};
    use std::sync::Arc;

    use tokio::sync::mpsc::error::TryRecvError;

    use crate::config::{GeneratorConfig, GeneratorType};

    use super::{random_strategy::Random, Generator};

    /// The duration the sender to a [`Generator::into_receiver()`] is absolutely
    /// guaranteed to have acquired a permit and sent its first message.
    const SENDER_WARMUP_DURATION: Duration = Duration::from_millis(100);

    /// Verifies that the generator is limited to a specified amount of concurrent streams.
    ///
    /// This _must_ have the `#[tokio::test(flavor = "multi_threaded")]` annotation,
    /// otherwise no thread will ever make senders produce output.
    #[tokio::test(flavor = "multi_thread")]
    async fn generator_is_limited() {
        for limit in 1..50 {
            let mut receivers = Vec::with_capacity(limit);
            let config = Arc::new(GeneratorConfig::new(
                0,
                GeneratorType::Random,
                limit,
                0, // No limit
                0, // No limit
                1,
                "<html>".to_string(),
            ));

            let g = Generator::from_config(config);
            for _ in 0..limit {
                let r = g.clone().into_receiver(Random::default());
                receivers.push(r);
            }

            tokio::time::sleep(SENDER_WARMUP_DURATION).await;

            // Ensure all receivers have sent their first message
            for r in &mut receivers {
                let _ = r
                    .try_recv()
                    .unwrap_or_else(|_| { panic!("{}", "Receiver within limit have not sent message".to_string()) });
            }

            // If we now attempt to use the original generator, it
            // should be blocked (since we are still holding on to active
            // receivers)
            let mut r = g.into_receiver(Random::default());

            // This should be instant, but we give it some time
            tokio::time::sleep(SENDER_WARMUP_DURATION).await;

            // We want this to be blocked
            match r.try_recv() {
                Ok(_) => panic!("should be blocked"),
                Err(TryRecvError::Disconnected) => panic!("disconnected!"),
                Err(TryRecvError::Empty) => {}
            };

            // So we can be completely sure that no generators
            // were dropped until now
            std::mem::drop(receivers);
        }
    }
}
