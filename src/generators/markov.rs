use std::{fs, process::exit};

use markov::Chain;
use rand::{thread_rng, Rng};

use crate::{config::GeneratorConfig, generators::GeneratorType};

use super::{web_stream_from_iterator, Generator};

pub(crate) struct MarkovChainGenerator {
    /// Chain used to generate responses. Used to hold ownership.,
    /// use `chain_iter`.
    chain: Chain<String>,
    // The range of length for each generated string segment (not
    // counting HTML) in bytes.
    chunk_size_range: std::ops::Range<usize>,
}

impl Clone for MarkovChainGenerator {
    fn clone(&self) -> Self {
        // Create a new chain, since it doesn't implement clone by itself...
        let mut new_chain = Chain::new();
        new_chain.feed(self.chain.generate());
        Self {
            chain: new_chain,
            chunk_size_range: self.chunk_size_range.clone(),
        }
    }
}

impl Generator for MarkovChainGenerator {
    fn from_config(config: GeneratorConfig) -> Self {
        match config.generator_type {
            GeneratorType::MarkovChain(pb) => {
                let content = fs::read_to_string(&pb).unwrap_or_else(|e| {
                    println!(
                        "Could not create Markov chain generator due to error:\n\t{}",
                        e
                    );
                    exit(555);
                });
                let mut chain: Chain<String> = Chain::new();
                chain.feed_str(&content);
                Self {
                    chain,
                    chunk_size_range: config.min_chunk_size..config.max_chunk_size,
                }
            }
            _ => panic!("wrong generator type in config"),
        }
    }

    fn to_stream(self) -> impl futures::prelude::stream::Stream<Item = String> + Send {
        web_stream_from_iterator(self)
    }
}

impl Iterator for MarkovChainGenerator {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let mut rng = thread_rng();
        let size = rng.gen_range(self.chunk_size_range.to_owned());
        let mut current_chunk_size: usize = 0;
        let response = self
            .chain
            .str_iter() // Not `.str_iter_for()`, it goes by token
            .take_while(move |s| {
                // String::len() is the amount of bytes
                current_chunk_size += s.len();
                current_chunk_size < size
            })
            .collect::<Vec<String>>()
            .join(" ");
        Some(format!("<p>{}</p>", response))
    }
}
