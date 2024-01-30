use std::{fs, process::exit};

use markov::Chain;

use crate::{
    config::{GeneratorConfig, GeneratorType},
    error_code,
};

use super::{Generator, P_TAG_SIZE};

/// A generator using Markov chains to generate text. Due to the nature of
/// markov chains, each new generated piece of string may not exactly be
/// `chunk_size`, and might be a bit larger.
pub(crate) struct MarkovChainGenerator {
    chunk_size: usize,
    /// Chain used to generate responses. Used to hold ownership.,
    /// use `chain_iter`.
    chain: Chain<String>,
}

impl Clone for MarkovChainGenerator {
    fn clone(&self) -> Self {
        // Create a new chain, since it doesn't implement clone by itself...
        let mut new_chain = Chain::new();
        new_chain.feed(self.chain.generate());
        Self {
            chunk_size: self.chunk_size,
            chain: new_chain,
        }
    }
}

impl Generator for MarkovChainGenerator {
    fn from_config(config: GeneratorConfig) -> Self {
        match config.generator_type {
            GeneratorType::MarkovChain(pb) => {
                let content = fs::read_to_string(pb).unwrap_or_else(|e| {
                    println!(
                        "Could not create Markov chain generator due to error:\n\t{}",
                        e
                    );
                    exit(error_code::CANNOT_READ_GENERATOR_DATA_FILE);
                });
                let mut chain: Chain<String> = Chain::new();
                chain.feed_str(&content);
                Self {
                    chunk_size: config.chunk_size,
                    chain,
                }
            }
            _ => panic!("wrong generator type in config"),
        }
    }
}

impl Iterator for MarkovChainGenerator {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let desired_size = self.chunk_size - P_TAG_SIZE;

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
