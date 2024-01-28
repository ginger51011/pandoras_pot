use crate::config::GeneratorConfig;
use rand::{
    distributions::{Alphanumeric, DistString},
    thread_rng,
};

use super::{Generator, GENERATOR_BUFFER_SIZE};

#[derive(Clone, Debug)]
pub(crate) struct RandomGenerator;

impl Generator for RandomGenerator {
    fn from_config(_: GeneratorConfig) -> Self {
        Self {}
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
        let mut rng = thread_rng();
        let s = Alphanumeric.sample_string(&mut rng, GENERATOR_BUFFER_SIZE);
        Some(s)
    }
}
