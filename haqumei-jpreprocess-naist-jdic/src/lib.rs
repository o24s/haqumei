//! 同梱の NAIST 辞書を Haqumei の形態素解析器に読み込みます。

#[cfg(feature = "naist-jdic")]
use haqumei_jpreprocess_dictionary::mecab::Model;

/// 同梱のシステム辞書を読み込みます。
#[cfg(feature = "naist-jdic")]
pub fn load() -> std::io::Result<Model> {
    load_with_user_dictionaries(&[])
}

/// 同梱の辞書とコンパイル済みのユーザー辞書を読み込みます。
#[cfg(feature = "naist-jdic")]
pub fn load_with_user_dictionaries(users: &[std::path::PathBuf]) -> std::io::Result<Model> {
    let directory = tempfile::tempdir()?;
    for (name, bytes) in [
        (
            "sys.dic",
            include_bytes!(concat!(env!("JPREPROCESS_WORKDIR"), "/sys.dic")).as_slice(),
        ),
        (
            "unk.dic",
            include_bytes!(concat!(env!("JPREPROCESS_WORKDIR"), "/unk.dic")).as_slice(),
        ),
        (
            "char.bin",
            include_bytes!(concat!(env!("JPREPROCESS_WORKDIR"), "/char.bin")).as_slice(),
        ),
        (
            "matrix.bin",
            include_bytes!(concat!(env!("JPREPROCESS_WORKDIR"), "/matrix.bin")).as_slice(),
        ),
    ] {
        std::fs::write(directory.path().join(name), bytes)?;
    }
    Model::open(directory.path(), users)
}
