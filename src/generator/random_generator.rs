use std::sync::Arc;

use crate::config::GeneratorConfig;
use bytes::Bytes;
use rand::{
    distributions::{Alphanumeric, DistString},
    rngs::SmallRng,
    SeedableRng,
};
use tokio::sync::Semaphore;

use super::{Generator, P_TAG_SIZE};

#[derive(Clone, Debug)]
pub(crate) struct RandomGenerator {
    config: GeneratorConfig,
    semaphore: Arc<Semaphore>,
}

impl Generator for RandomGenerator {
    fn from_config(config: GeneratorConfig) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent()));
        Self { config, semaphore }
    }

    fn permits(&self) -> Arc<Semaphore> {
        self.semaphore.clone()
    }

    fn config(&self) -> &GeneratorConfig {
        &self.config
    }
}

impl Default for RandomGenerator {
    fn default() -> Self {
        Self::from_config(GeneratorConfig::default())
    }
}

impl Iterator for RandomGenerator {
    type Item = Bytes;

    fn next(&mut self) -> Option<Self::Item> {
        // No need to be secure, we are smacking bots
        let mut smol_rng = SmallRng::from_entropy();
        let s = Alphanumeric.sample_string(&mut smol_rng, self.config().chunk_size - P_TAG_SIZE);
        Some(Bytes::from(format!("<p>\n{s}\n</p>\n")))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config::{GeneratorConfig, GeneratorType},
        generator::{
            random_generator::RandomGenerator, tests::test_generator_is_limited, Generator,
        },
    };

    #[tokio::test(flavor = "multi_thread")]
    async fn random_generator_limits() {
        for limit in 1..100 {
            let gen_config = GeneratorConfig::new(20, GeneratorType::Random, limit, 0, 0);
            let gen = RandomGenerator::from_config(gen_config);
            assert!(
                test_generator_is_limited(gen, limit),
                "last generator could produce output while blocked"
            );
        }
    }
}
