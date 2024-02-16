use std::{fs, process::exit, sync::Arc};

use markov::Chain;
use tokio::sync::Semaphore;

use crate::{
    config::{GeneratorConfig, GeneratorType},
    error_code,
};

use super::{Generator, P_TAG_SIZE};

/// A generator using Markov chains to generate text. Due to the nature of
/// markov chains, each new generated piece of string may not exactly be
/// `chunk_size`, and might be a bit larger.
#[derive(Debug)]
pub(crate) struct MarkovChainGenerator {
    config: GeneratorConfig,
    /// Chain used to generate responses. Used to hold ownership.,
    /// use `chain_iter`.
    chain: Chain<String>,
    semaphore: Arc<Semaphore>,
}

impl Clone for MarkovChainGenerator {
    fn clone(&self) -> Self {
        // Create a new chain, since it doesn't implement clone by itself...
        let mut new_chain = Chain::new();
        new_chain.feed(self.chain.generate());
        Self {
            config: self.config.clone(),
            chain: new_chain,
            semaphore: self.semaphore.clone(),
        }
    }
}

impl Generator for MarkovChainGenerator {
    fn from_config(config: GeneratorConfig) -> Self {
        match config.generator_type {
            GeneratorType::MarkovChain(ref pb) => {
                let content = fs::read_to_string(pb).unwrap_or_else(|e| {
                    println!(
                        "Could not create Markov chain generator due to error:\n\t{}",
                        e
                    );
                    exit(error_code::CANNOT_READ_GENERATOR_DATA_FILE);
                });
                let mut chain: Chain<String> = Chain::new();
                chain.feed_str(&content);
                let semaphore = Arc::new(Semaphore::new(config.max_concurrent()));
                Self {
                    config,
                    chain,
                    semaphore,
                }
            }
            _ => panic!("wrong generator type in config"),
        }
    }

    fn permits(&self) -> Arc<Semaphore> {
        self.semaphore.clone()
    }

    fn config(&self) -> &GeneratorConfig {
        &self.config
    }
}

impl Iterator for MarkovChainGenerator {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let desired_size = self.config().chunk_size - P_TAG_SIZE;

        // Add some more, we are going to get a bit too much I think.
        let mut result = String::with_capacity(desired_size + 1024);
        while result.as_bytes().len() < desired_size {
            // Must do it this way to get a new generated string
            // each time
            result.push_str(&self.chain.generate_str());
        }
        Some(format!("<p>\n{}\n</p>\n", result))
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::NamedTempFile;

    use crate::{
        config::{GeneratorConfig, GeneratorType},
        generator::{
            markov_generator::MarkovChainGenerator, tests::test_generator_is_limited, Generator,
        },
    };

    #[tokio::test(flavor = "multi_thread")]
    async fn markov_generator_limits() {
        let mut tmpfile: NamedTempFile = tempfile::NamedTempFile::new().unwrap();
        write!(tmpfile, "I am but a little chain. I do chain things.").unwrap();

        for limit in 1..100 {
            let gen_config = GeneratorConfig::new(
                20,
                GeneratorType::MarkovChain(tmpfile.path().to_path_buf()),
                limit,
                0,
                0,
            );
            let gen = MarkovChainGenerator::from_config(gen_config);
            assert!(
                test_generator_is_limited(gen, limit),
                "last generator could produce output while blocked"
            );
        }
    }
}
