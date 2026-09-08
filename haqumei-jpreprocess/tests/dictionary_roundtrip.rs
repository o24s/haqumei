#![cfg(feature = "tokenizer")]

use haqumei_jpreprocess::JPreprocess;
use haqumei_jpreprocess_dictionary::dictionary::to_dict::JPreprocessDictionaryBuilder;
use lindera::dictionary::{load_fs_dictionary, load_user_dictionary_from_bin};
use std::path::Path;

#[test]
fn rkyv_system_and_user_dictionaries_match_text_dictionaries() {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data");
    let temporary = tempfile::tempdir().unwrap();
    let system_path = temporary.path().join("system");
    let user_path = temporary.path().join("user.bin");
    let metadata = lindera_dictionary::dictionary::metadata::Metadata::load(
        &std::fs::read(data.join("min-dict-src/metadata.json")).unwrap(),
    )
    .unwrap();
    let builder = JPreprocessDictionaryBuilder::new(metadata);
    builder
        .build_dictionary(&data.join("min-dict-src"), &system_path)
        .unwrap();
    builder
        .build_user_dictionary(&data.join("user/src.csv"), &user_path)
        .unwrap();

    for with_user in [false, true] {
        let plain = JPreprocess::with_dictionaries(
            load_fs_dictionary(&data.join("min-dict")).unwrap(),
            with_user.then(|| load_user_dictionary_from_bin(&data.join("user/user.bin")).unwrap()),
        );
        let archived = JPreprocess::with_dictionaries(
            load_fs_dictionary(&system_path).unwrap(),
            with_user.then(|| load_user_dictionary_from_bin(&user_path).unwrap()),
        );
        for text in [
            "日本語文を解析し、音声合成エンジンに渡せる形式に変換します．",
            "日本",
            "キログラム",
            "生麦生米生卵",
        ] {
            assert_eq!(
                archived.text_to_njd(text).unwrap(),
                plain.text_to_njd(text).unwrap(),
                "{text}, user={with_user}"
            );
            assert_eq!(
                archived.extract_fullcontext(text).unwrap(),
                plain.extract_fullcontext(text).unwrap(),
                "{text}, user={with_user}"
            );
        }
    }
}

#[cfg(feature = "naist-jdic")]
#[test]
fn embedded_naist_dictionary_produces_labels() {
    let dictionary = haqumei_jpreprocess_naist_jdic::lindera::load().unwrap();
    dictionary.metadata.validate_format_version().unwrap();
    let engine = JPreprocess::with_dictionaries(dictionary, None);
    let labels = engine
        .extract_fullcontext("日本語の音声合成です。")
        .unwrap();
    assert!(labels.len() > 10);
    assert_eq!(labels[0].phoneme.c.as_deref(), Some("sil"));
}
