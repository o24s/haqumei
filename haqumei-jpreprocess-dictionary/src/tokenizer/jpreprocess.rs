use std::borrow::Cow;

use haqumei_jpreprocess_core::{
    error::DictionaryError,
    token::{Token, Tokenizer},
    word_entry::WordEntry,
    JPreprocessResult,
};

use crate::{
    dictionary::word_encoding::{DictionaryWordEncoding, JPreprocessDictionaryWordEncoding},
    word_data::get_word_data,
};

pub struct JPreprocessTokenizer {
    tokenizer: lindera::segmenter::Segmenter,
}

impl JPreprocessTokenizer {
    pub fn new(tokenizer: lindera::segmenter::Segmenter) -> Self {
        Self { tokenizer }
    }

    fn get_word(
        &self,
        word_id: lindera_dictionary::viterbi::WordId,
    ) -> Result<WordEntry, DictionaryError> {
        if word_id.is_unknown() {
            Ok(WordEntry::default())
        } else if word_id.is_system() {
            Self::get_word_from_data(
                &self.tokenizer.dictionary.prefix_dictionary.words_idx_data,
                &self.tokenizer.dictionary.prefix_dictionary.words_data,
                word_id,
            )
        } else {
            let user = &self.tokenizer.user_dictionary;
            user.as_ref()
                .map_or(Err(DictionaryError::UserDictionaryNotProvided), |user| {
                    Self::get_word_from_data(
                        &user.dict.words_idx_data,
                        &user.dict.words_data,
                        word_id,
                    )
                })
        }
    }

    /// PANIC: It must be ensured that the prefix_dict is the correct dictionary for the word_id.
    pub(super) fn get_word_from_data(
        idx: &[u8],
        words: &[u8],
        word_id: lindera_dictionary::viterbi::WordId,
    ) -> Result<WordEntry, DictionaryError> {
        let header = get_word_data(idx, words, None).unwrap_or_default();
        if header != JPreprocessDictionaryWordEncoding::identifier().as_bytes() {
            return Err(DictionaryError::UnsupportedFormat(
                String::from_utf8_lossy(header).into_owned(),
            ));
        }
        if word_id.is_unknown() {
            Ok(WordEntry::default())
        } else {
            let data = get_word_data(idx, words, Some(word_id.id() as usize))
                .ok_or(DictionaryError::IdNotFound(word_id.id()))?;
            Ok(JPreprocessDictionaryWordEncoding::deserialize(data)?)
        }
    }
}

impl Tokenizer for JPreprocessTokenizer {
    fn tokenize<'a>(&'a self, text: &'a str) -> JPreprocessResult<Vec<impl 'a + Token>> {
        let words = self.tokenizer.segment(text.into())?;
        words
            .into_iter()
            .map(|token| {
                Ok(JPreprocessToken::new(
                    token.surface,
                    self.get_word(token.word_id)?,
                ))
            })
            .collect::<Result<_, _>>()
    }
}

pub struct JPreprocessToken<'a> {
    text: Cow<'a, str>,
    entry: WordEntry,
}

impl<'a> JPreprocessToken<'a> {
    pub(crate) fn new(text: Cow<'a, str>, entry: WordEntry) -> Self {
        Self { text, entry }
    }
}

impl Token for JPreprocessToken<'_> {
    fn fetch(&mut self) -> Result<(&str, WordEntry), haqumei_jpreprocess_core::JPreprocessError> {
        Ok((&self.text, self.entry.clone()))
    }
}
