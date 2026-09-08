use haqumei_jpreprocess_core::{
    error::DictionaryError,
    token::{Token, Tokenizer},
    word_entry::WordEntry,
    JPreprocessResult,
};

use super::{
    identify_dictionary::DictionaryIdent,
    jpreprocess::{JPreprocessToken, JPreprocessTokenizer},
};

pub struct DefaultTokenizer {
    lindera_tokenizer: lindera::segmenter::Segmenter,
    system: TokenizerType,
    user: Option<TokenizerType>,
}

enum TokenizerType {
    JPreprocessTokenizer,
    LinderaTokenizer,
    Unsupported(String),
}

impl DefaultTokenizer {
    pub fn new(tokenizer: lindera::segmenter::Segmenter) -> Self {
        fn identify_tokenizer(idx: &[u8], words: &[u8]) -> TokenizerType {
            let ident = DictionaryIdent::from_idx_data(idx, words);
            match ident {
                DictionaryIdent::JPreprocess => TokenizerType::JPreprocessTokenizer,
                DictionaryIdent::Lindera => TokenizerType::LinderaTokenizer,
                DictionaryIdent::Unsupported(format) => TokenizerType::Unsupported(format),
            }
        }

        Self {
            system: identify_tokenizer(
                &tokenizer.dictionary.prefix_dictionary.words_idx_data,
                &tokenizer.dictionary.prefix_dictionary.words_data,
            ),
            user: tokenizer
                .user_dictionary
                .as_ref()
                .map(|d| identify_tokenizer(&d.dict.words_idx_data, &d.dict.words_data)),
            lindera_tokenizer: tokenizer,
        }
    }
}

impl Tokenizer for DefaultTokenizer {
    fn tokenize<'a>(&'a self, text: &'a str) -> JPreprocessResult<Vec<impl 'a + Token>> {
        for kind in std::iter::once(&self.system).chain(self.user.iter()) {
            if let TokenizerType::Unsupported(format) = kind {
                return Err(DictionaryError::UnsupportedFormat(format.clone()).into());
            }
        }
        let tokens = self.lindera_tokenizer.segment(text.into())?;

        tokens
            .into_iter()
            .map(|token| {
                if token.word_id.is_unknown() {
                    Ok(DefaultToken::Lindera(token))
                } else if token.word_id.is_system() {
                    match &self.system {
                        TokenizerType::JPreprocessTokenizer => {
                            Ok(DefaultToken::JPreprocess(JPreprocessToken::new(
                                token.surface,
                                JPreprocessTokenizer::get_word_from_data(
                                    &token.dictionary.prefix_dictionary.words_idx_data,
                                    &token.dictionary.prefix_dictionary.words_data,
                                    token.word_id,
                                )?,
                            )))
                        }
                        TokenizerType::LinderaTokenizer => Ok(DefaultToken::Lindera(token)),
                        TokenizerType::Unsupported(format) => {
                            Err(DictionaryError::UnsupportedFormat(format.clone()).into())
                        }
                    }
                } else {
                    match &self.user {
                        Some(TokenizerType::JPreprocessTokenizer) => {
                            Ok(DefaultToken::JPreprocess(JPreprocessToken::new(
                                token.surface,
                                JPreprocessTokenizer::get_word_from_data(
                                    &token.user_dictionary.as_ref().unwrap().dict.words_idx_data,
                                    &token.user_dictionary.as_ref().unwrap().dict.words_data,
                                    token.word_id,
                                )?,
                            )))
                        }
                        Some(TokenizerType::LinderaTokenizer) => Ok(DefaultToken::Lindera(token)),
                        None => Ok(DefaultToken::Lindera(token)),
                        Some(TokenizerType::Unsupported(format)) => {
                            Err(DictionaryError::UnsupportedFormat(format.clone()).into())
                        }
                    }
                }
            })
            .collect()
    }
}

enum DefaultToken<'a> {
    Lindera(lindera::token::Token<'a>),
    JPreprocess(JPreprocessToken<'a>),
}

impl Token for DefaultToken<'_> {
    fn fetch(&mut self) -> JPreprocessResult<(&str, WordEntry)> {
        match self {
            Self::Lindera(token) => token.fetch(),
            Self::JPreprocess(token) => token.fetch(),
        }
    }
}
