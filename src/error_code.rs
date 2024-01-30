//! This module contains error codes used by `pandoras_pot`

/// Cannot deserialize config.
pub(crate) const UNPARSEABLE_CONFIG: i32 = 10;

/// A configuration has conflicting settings.
pub(crate) const BAD_CONFIG: i32 = 11;

/// The desired log file path could not be opened.
pub(crate) const CANNOT_OPEN_LOG_FILE: i32 = 20;

/// The configured generator data file path could not be read.
pub(crate) const CANNOT_READ_GENERATOR_DATA_FILE: i32 = 30;
