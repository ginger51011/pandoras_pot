use crate::config::GeneratorConfig;
use rand::{
    distributions::{Alphanumeric, DistString},
    rngs::SmallRng,
    SeedableRng,
};

use super::{Generator, P_TAG_SIZE};

#[derive(Clone, Debug)]
pub(crate) struct RandomGenerator {
    /// The range of length for each generated string segment (not
    /// counting HTML) in bytes.
    chunk_size: usize,
}

impl Generator for RandomGenerator {
    fn from_config(config: GeneratorConfig) -> Self {
        Self {
            chunk_size: config.chunk_size,
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
        // No need to be secure, we are smacking bots
        let mut smol_rng = SmallRng::from_entropy();
        let s = Alphanumeric.sample_string(&mut smol_rng, self.chunk_size - P_TAG_SIZE);
        Some(format!("<p>\n{}\n</p>\n", s))
    }
}
