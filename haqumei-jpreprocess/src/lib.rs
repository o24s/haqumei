#![cfg_attr(docsrs, feature(doc_cfg))]

mod normalize_open_jtalk;
pub use normalize_open_jtalk::normalize_text_for_open_jtalk;

#[doc(hidden)]
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

mod normalize_text;
pub use normalize_text::normalize_text_for_naist_jdic;

pub use haqumei_jpreprocess_core::error;
use haqumei_jpreprocess_core::{token::Tokenizer, *};
pub use haqumei_jpreprocess_njd::NJD;

pub struct JPreprocess<T: Tokenizer> {
    tokenizer: T,
}

impl<T: Tokenizer> JPreprocess<T> {
    /// Creates JPreprocess from provided tokenizer.
    pub fn from_tokenizer(tokenizer: T) -> Self {
        Self { tokenizer }
    }

    /// テキストを形態素解析して NJD の単語列を返します。
    pub fn text_to_njd(&self, text: &str) -> JPreprocessResult<NJD> {
        let normalized_input_text = normalize_text_for_naist_jdic(text);
        let tokens = self.tokenizer.tokenize(normalized_input_text.as_str())?;

        NJD::from_tokens(tokens)
    }

    /// テキストを形態素解析し、前処理済みの NJD 特徴量を返します。
    pub fn run_frontend(&self, text: &str) -> JPreprocessResult<Vec<String>> {
        let mut njd = Self::text_to_njd(self, text)?;
        njd.preprocess();
        Ok(njd.into())
    }

    /// Generate jpcommon features from NJD features(returned by [`run_frontend`]).
    ///
    /// [`run_frontend`]: #method.run_frontend
    pub fn make_label(&self, njd_features: Vec<String>) -> Vec<haqumei_jlabel::Label> {
        let njd = NJD::from_strings(njd_features);
        haqumei_jpreprocess_jpcommon::njdnodes_to_features(&njd.nodes)
    }

    /// Generate jpcommon features from a text.
    ///
    /// This is not guaranteed to be same as calling [`run_frontend`] and [`make_label`].
    ///
    /// [`run_frontend`]: #method.run_frontend
    /// [`make_label`]: #method.make_label
    pub fn extract_fullcontext(&self, text: &str) -> JPreprocessResult<Vec<haqumei_jlabel::Label>> {
        let mut njd = Self::text_to_njd(self, text)?;
        njd.preprocess();
        Ok(haqumei_jpreprocess_jpcommon::njdnodes_to_features(
            &njd.nodes,
        ))
    }
}

#[cfg(feature = "tokenizer")]
mod dictionary;
#[cfg(feature = "tokenizer")]
pub use dictionary::*;
