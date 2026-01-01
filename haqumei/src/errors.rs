use std::{ffi::NulError, io, path::{PathBuf, StripPrefixError}};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum HaqumeiError {
    #[error("Failed to allocate internal structures: {0}")]
    AllocationError(&'static str),

    #[error("The provided dictionary path is invalid: {0}")]
    InvalidDictionaryPath(String),

    #[error("Failed to load MeCab dictionary.")]
    MecabLoadError,

    #[error("Input data for FFI contains an interior NUL byte at position {pos}: `{}`", String::from_utf8_lossy(bytes))]
    InteriorNulError {
        bytes: Vec<u8>,
        pos: usize,
    },

    #[error("Cache directory I/O error at path '{path}'")]
    CacheIo {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("text2mecab conversion failed: {0}")]
    Text2MecabError(String),

    #[error("Embedded dictionary verification failed at '{path}': checksum mismatch.\n  Expected: {expected}\n  Actual:   {actual}")]
    DictionaryVerification {
        path: PathBuf,
        expected: String,
        actual: String,
    },

    #[error("Could not determine a valid data directory for this system")]
    DataDirectoryNotFound,

    #[error("Could not determine a valid cache directory for this system")]
    CacheDirectoryNotFound,

    #[error("Global dictionary is not initialized yet")]
    GlobalDictionaryNotInitialized,

    #[error(transparent)]
    StripPrefixError(#[from] StripPrefixError),

    #[error(transparent)]
    IoError(#[from] io::Error),

    #[error(transparent)]
    DirError(#[from] walkdir::Error),

    #[error(transparent)]
    VibratoError(#[from] vibrato_rkyv::errors::VibratoError),

    #[error(transparent)]
    OrtError(#[from] ort::Error),
}

impl From<NulError> for HaqumeiError {
    fn from(value: NulError) -> Self {
        let pos = value.nul_position();
        HaqumeiError::InteriorNulError { bytes: value.into_vec(), pos }
    }
}
