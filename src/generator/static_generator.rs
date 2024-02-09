use std::{fs, process::exit, sync::Arc};

use tokio::sync::Semaphore;

use crate::{
    config::{GeneratorConfig, GeneratorType},
    error_code,
};

use super::Generator;

/// A generator that always returns the same string.
#[derive(Clone, Debug)]
pub(crate) struct StaticGenerator {
    data: String,
    semaphore: Arc<Semaphore>,
}

impl Generator for StaticGenerator {
    fn from_config(config: GeneratorConfig) -> Self {
        match config.generator_type {
            GeneratorType::Static(ref pb) => {
                let data = fs::read_to_string(pb).unwrap_or_else(|_| {
                    println!("Data for static generator must be a path to a readable file.");
                    exit(error_code::CANNOT_READ_GENERATOR_DATA_FILE);
                });
                Self {
                    data,
                    semaphore: Arc::new(Semaphore::new(config.max_concurrent())),
                }
            }
            _ => panic!("wrong generator type in config"),
        }
    }

    fn permits(&self) -> Arc<Semaphore> {
        self.semaphore.clone()
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

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::NamedTempFile;

    use crate::{
        config::{GeneratorConfig, GeneratorType},
        generator::{
            static_generator::StaticGenerator, tests::test_generator_is_limited, Generator,
        },
    };

    #[tokio::test(flavor = "multi_thread")]
    async fn static_generator_limits() {
        let mut tmpfile: NamedTempFile = tempfile::NamedTempFile::new().unwrap();
        write!(tmpfile, "I am but a little chain. I do chain things.").unwrap();

        for limit in 1..100 {
            let gen_config = GeneratorConfig::new(
                20,
                GeneratorType::Static(tmpfile.path().to_path_buf()),
                limit,
            );
            let gen = StaticGenerator::from_config(gen_config);
            assert!(
                test_generator_is_limited(gen, limit),
                "last generator could produce output while blocked"
            );
        }
    }
}
