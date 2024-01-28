use crate::config::GeneratorConfig;
use rand::{
    distributions::{Alphanumeric, DistString},
    thread_rng, Rng,
};

use super::Generator;

#[derive(Clone, Debug)]
pub(crate) struct RandomGenerator {
    /// The range of length for each generated string segment (not
    /// counting HTML) in bytes.
    chunk_size_range: std::ops::Range<usize>,
}

impl Generator for RandomGenerator {
    fn from_config(config: GeneratorConfig) -> Self {
        Self {
            chunk_size_range: config.min_chunk_size..config.max_chunk_size,
        }
    }
}

impl Default for RandomGenerator {
    fn default() -> Self {
        Self::from_config(GeneratorConfig::default())
    }
}

impl Iterator for RandomGenerator {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        // TODO: Fix this, good if it is u8 to start with, and then
        // perhaps not to include <p><p> stuff.
        let mut rng = thread_rng();
        let size = rng.gen_range(self.chunk_size_range.to_owned());
        let s = Alphanumeric.sample_string(&mut rng, size);
        Some(format! {"<p>\n{s}\n</p>\n"})
    }
}
