//! This module contains structures to create a generator used for data creation.

pub(crate) mod markov;
pub(crate) mod random;

use crate::config::GeneratorConfig;
use futures::stream;

/// Creates a "plausible" web stream from an iterator
fn web_stream_from_iterator<T: Iterator<Item = String>>(
    gen: T,
) -> stream::Iter<std::iter::Chain<std::array::IntoIter<String, 1>, T>> {
    // Add some initial tags
    let initial_tags = [String::from("<html>\n<body>\n")];
    // Chain them, so we always start with some valid initial tags
    let iter = initial_tags.into_iter().chain(gen);
    stream::iter(iter)
}

/// Trait that describes a generator that can be converted to a stream,
/// outputting (probably infinite) amounts of very useful strings.
pub trait Generator {
    /// Creates the generator from a config.
    fn from_config(config: GeneratorConfig) -> Self;

    /// Converts the generator to a stream of text.
    fn to_stream(self) -> impl stream::Stream<Item = String> + Send;
}
