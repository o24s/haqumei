use crate::{
    dictionary::word_encoding::{DictionaryWordEncoding, JPreprocessDictionaryWordEncoding},
    word_data::get_word_data,
};

pub enum DictionaryIdent {
    Lindera,
    JPreprocess,
    Unsupported(String),
}

impl DictionaryIdent {
    pub fn from_idx_data(idx: &[u8], data: &[u8]) -> Self {
        match get_word_data(idx, data, None) {
            Some([]) => Self::Lindera,
            Some(bytes) if bytes == JPreprocessDictionaryWordEncoding::identifier().as_bytes() => {
                Self::JPreprocess
            }
            Some(bytes) => Self::Unsupported(String::from_utf8_lossy(bytes).into_owned()),
            None if idx.is_empty() && data.is_empty() => Self::Lindera,
            None => Self::Unsupported("invalid word index".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_and_unknown_formats_are_rejected() {
        for header in [
            "haqumei_jpreprocess 0.1.0",
            "haqumei_jpreprocess rkyv-0.8-v2",
            "jpreprocess 0.15.0",
        ] {
            assert!(matches!(
                DictionaryIdent::from_idx_data(
                    &(header.len() as u32).to_le_bytes(),
                    header.as_bytes()
                ),
                DictionaryIdent::Unsupported(_)
            ));
        }
        let header = JPreprocessDictionaryWordEncoding::identifier();
        assert!(matches!(
            DictionaryIdent::from_idx_data(&(header.len() as u32).to_le_bytes(), header.as_bytes()),
            DictionaryIdent::JPreprocess
        ));
        assert!(matches!(
            DictionaryIdent::from_idx_data(&0u32.to_le_bytes(), b"data"),
            DictionaryIdent::Lindera
        ));
    }
}
