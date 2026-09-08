use haqumei_jpreprocess_dictionary::mecab::Model;
use std::path::PathBuf;

/// 同梱するシステム辞書の種類です。
pub enum JPreprocessDictionaryKind {
    #[cfg(feature = "naist-jdic")]
    NaistJdic,
}

impl JPreprocessDictionaryKind {
    pub(crate) fn load(&self, _users: &[PathBuf]) -> std::io::Result<Model> {
        match *self {
            #[cfg(feature = "naist-jdic")]
            Self::NaistJdic => haqumei_jpreprocess_naist_jdic::load_with_user_dictionaries(_users),
        }
    }
}
