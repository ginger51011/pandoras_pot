//! This module contains structures to create a generator used for data creation.
use futures::stream;

use rand::distributions::{Alphanumeric, DistString};
use rand::{thread_rng, Rng};

use crate::config::GeneratorConfig;

#[derive(Clone, Debug)]
pub(crate) struct PandorasGenerator {
    // The range of length for each generated string segment (not
    // counting HTML) in bytes.
    chunk_size_range: std::ops::Range<usize>,
}

impl PandorasGenerator {
    pub fn new(config: GeneratorConfig) -> Self {
        Self {
            chunk_size_range: config.min_chunk_size..config.max_chunk_size,
        }
    }

    pub fn to_stream(&self) -> impl stream::Stream<Item = String> {
        stream::iter(self.clone())
    }
}

impl Default for PandorasGenerator {
    fn default() -> Self {
        Self::new(GeneratorConfig::default())
    }
}

impl Iterator for PandorasGenerator {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let mut rng = thread_rng();
        let size = (&mut rng).gen_range(self.chunk_size_range.to_owned());
        Some(Alphanumeric.sample_string(&mut rng, size))
    }
}
