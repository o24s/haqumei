use crate::{
    accent_rule::AccentRuleParseError, ctype::CTypeParseError, pos::POSParseError,
    pronunciation::PronunciationParseError,
};

#[derive(Debug, thiserror::Error)]
pub enum JPreprocessError {
    #[error("Io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to fetch word from dictionary: {0}")]
    DictionaryError(#[from] DictionaryError),
    #[error("Failed to parse pronunciation: {0}")]
    PronunciationParseError(#[from] PronunciationParseError),
    #[error("Failed to parse part of speech (POS): {0}")]
    PartOfSpeechParseError(#[from] POSParseError),
    #[error("Failed to parse conjugation type (CType): {0}")]
    CTypeParseError(#[from] CTypeParseError),
    #[error("Failed to parse conjugation form (CForm)")]
    CFormParseError,
    #[error("Failed to parse accent rule: {0}")]
    AccentRuleParseError(#[from] AccentRuleParseError),
    #[error("Provided mora size {0} is different from that of calculated from pronunciation {1}")]
    MoraSizeMismatch(usize, usize),
}

#[derive(Debug, thiserror::Error)]
pub enum DictionaryError {
    #[error("Unsupported dictionary format: {0}; rebuild the dictionary from source")]
    UnsupportedFormat(String),
    #[error("Word with id {0} not found")]
    IdNotFound(u32),
    #[error("Failed to decode: {0}")]
    FailDecode(#[from] rkyv::rancor::Error),
    #[error("The word is flagged as UserDictionary, but UserDictionary is empty")]
    UserDictionaryNotProvided,
    #[error("The word is flagged as UserDictionary, but UserDictionary mode is not set")]
    UserDictionaryModeNotSet,
}
