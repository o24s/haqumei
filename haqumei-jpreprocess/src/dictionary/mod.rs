use haqumei_jpreprocess_core::JPreprocessResult;
use haqumei_jpreprocess_dictionary::mecab::Model;
use std::path::PathBuf;

/// 形態素解析に使うシステム辞書を指定します。
pub enum SystemDictionaryConfig {
    /// UTF-8 の MeCab 互換辞書をディレクトリーから読み込みます。
    File(PathBuf),
}

impl SystemDictionaryConfig {
    /// システム辞書を読み込みます。
    pub fn load(self) -> JPreprocessResult<Model> {
        self.load_with_user_dictionaries(&[])
    }

    /// システム辞書とコンパイル済みのユーザー辞書を読み込みます。
    pub fn load_with_user_dictionaries(self, users: &[PathBuf]) -> JPreprocessResult<Model> {
        Ok(match self {
            Self::File(path) => Model::open(&path, users)?,
        })
    }
}
