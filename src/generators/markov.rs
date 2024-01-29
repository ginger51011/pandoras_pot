use std::{fs, process::exit};

use markov::Chain;

use crate::config::{GeneratorConfig, GeneratorType};

use super::{Generator, P_TAG_SIZE};

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
                    exit(555);
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
        let s: String = self
            .chain
            .str_iter_for(self.chunk_size - P_TAG_SIZE)
            .collect();
        Some(format!("<p>\n{}\n</p>\n", s))
    }
}
