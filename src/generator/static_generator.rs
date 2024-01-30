use std::{fs, process::exit};

use crate::config::{GeneratorConfig, GeneratorType};

use super::Generator;

/// A generator that always returns the same string.
#[derive(Clone, Debug)]
pub(crate) struct StaticGenerator {
    data: String,
}

impl Generator for StaticGenerator {
    fn from_config(config: GeneratorConfig) -> Self {
        match config.generator_type {
            GeneratorType::Static(pb) => {
                let data = fs::read_to_string(&pb).unwrap_or_else(|_| {
                    println!("Data for static generator must be a path to a readable file.");
                    exit(33);
                });
                Self { data }
            }
            _ => panic!("wrong generator type in config"),
        }
    }
}

impl Default for StaticGenerator {
    fn default() -> Self {
        Self::from_config(GeneratorConfig::default())
    }
}

impl Iterator for StaticGenerator {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.data.clone())
    }
}
