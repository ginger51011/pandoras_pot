//! This module contains structures to create a generator used for data creation.

pub(crate) mod markov;
pub(crate) mod random;

use crate::config::GeneratorConfig;
use futures::Stream;

const GENERATOR_BUFFER_SIZE: usize = 1024 * 16;
const GENERATOR_CHANNEL_BUFFER: usize = 2;

/// Trait that describes a generator that can be converted to a stream,
/// outputting (probably infinite) amounts of very useful strings.
pub trait Generator
where
    Self: Sync + Iterator<Item = String> + Clone + Send + 'static,
{
    /// Creates the generator from a config.
    fn from_config(config: GeneratorConfig) -> Self;

    /// Fills the buffer with data from the generator.
    ///
    /// Will panic if the iterator fails to generate a new value.
    async fn read(&mut self, buff: &mut [u8; GENERATOR_BUFFER_SIZE]) {
        let s = self.next().unwrap();
        let bytes = s.as_bytes();
        buff.copy_from_slice(&bytes[..GENERATOR_BUFFER_SIZE])
    }

    /// Returns an infinite stream using this generator.
    fn stream(&self) -> impl Stream<Item = [u8; GENERATOR_BUFFER_SIZE]> {
        let (tx, rx) = tokio::sync::mpsc::channel(GENERATOR_CHANNEL_BUFFER);

        // This is kind of expensive, but not really. First response time is not an issue.
        let mut gen = self.clone();
        tokio::spawn(async move {
            let mut buff = [0_u8; GENERATOR_BUFFER_SIZE];
            let mut bytes_written = 0_usize;
            loop {
                gen.read(&mut buff);
                match tx.send(buff).await {
                    Ok(_) => bytes_written += buff.len(),
                    Err(_) => {
                        // TODO: Add metadata
                        tracing::info!(
                            "Stream broken, wrote {} MB, or {} GB",
                            (bytes_written as f64) * 1e-6,
                            (bytes_written as f64) * 1e-6
                        );
                        break;
                    }
                }
            }
        });

        tokio_stream::wrappers::ReceiverStream::new(rx)
    }
}
