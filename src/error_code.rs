//! This module contains error codes used by `pandoras_pot`

/// Cannot parse arguments
pub(crate) const ARGUMENT_ERROR: i32 = 1;
pub(crate) const UNKNOWN_ERROR: i32 = 2;

/// Cannot deserialize config.
pub(crate) const UNPARSEABLE_CONFIG: i32 = 10;

/// A configuration has conflicting settings.
pub(crate) const BAD_CONFIG: i32 = 11;
pub(crate) const BAD_CONTENT_TYPE: i32 = 12;

/// The desired log file path could not be opened.
pub(crate) const CANNOT_OPEN_LOG_FILE: i32 = 20;

/// The configured generator data file path could not be read.
pub(crate) const CANNOT_READ_GENERATOR_DATA_FILE: i32 = 30;
pub(crate) const GENERATOR_CHUNK_SIZE_TOO_SMALL: i32 = 31;
pub(crate) const GENERATOR_CHUNK_BUFFER_TOO_SMALL: i32 = 32;
