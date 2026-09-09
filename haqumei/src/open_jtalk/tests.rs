use std::{env, io::Write};

use crate::open_jtalk::model::MecabModel;
use tempfile::NamedTempFile;

use super::*;

#[test]
#[cfg(feature = "embed-dictionary")]
fn test_global_dictionary() {
    let model = &GLOBAL_MECAB_DICTIONARY.load_full().model;
    assert!(model.is_initialized());
}

#[test]
#[cfg(feature = "embed-dictionary")]
fn test_mecab_model_is_shared() {
    let model = Dictionary::from_embedded().unwrap().model;
    let mut first = Mecab::from_model(&model).unwrap();
    let mut second = Mecab::from_model(&model).unwrap();
    let expected = first.analyze("こんにちは").unwrap();
    drop(first);
    drop(model);
    let actual = second.analyze("こんにちは").unwrap();
    assert_eq!(actual.nodes, expected.nodes);
    assert_eq!(actual.best_path, expected.best_path);
}

#[test]
fn test_model_new() {
    let model = MecabModel::new_uninitialized();
    assert!(!model.is_initialized());
    assert!(Mecab::from_model(&model).is_err());

    #[cfg(feature = "embed-dictionary")]
    {
        let model = Dictionary::from_embedded().unwrap().model;
        assert!(model.is_initialized());
    }
}

#[test]
fn test_njd() {
    let njd = super::njd::features_to_njd(&[]).unwrap();
    assert!(njd.nodes.is_empty());
}

#[test]
fn test_userdict() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("Failed to get MANIFEST_DIR");
    let manifest_dir = Path::new(&manifest_dir);

    let mut ojt = OpenJTalk::new().unwrap();

    let tests = vec![("nnmn", "n a n a m i N"), ("GNU", "g u n u u")];

    for (text, expected) in &tests {
        let p = ojt.g2p(text).unwrap().join(" ");
        assert_ne!(&p, expected);
    }

    let mut user_csv = NamedTempFile::new().unwrap();
    writeln!(
        user_csv.as_file_mut(),
        "ｎｎｍｎ,,,1,名詞,一般,*,*,*,*,ｎｎｍｎ,ナナミン,ナナミン,1/4,*"
    )
    .unwrap();
    writeln!(
        user_csv.as_file_mut(),
        "ＧＮＵ,,,1,名詞,一般,*,*,*,*,ＧＮＵ,グヌー,グヌー,2/3,*"
    )
    .unwrap();
    let user_csv_path = user_csv.into_temp_path();

    let user_out_path = NamedTempFile::new().unwrap().into_temp_path();

    let dict_dir = GLOBAL_MECAB_DICTIONARY.load().dict_dir.clone();
    MecabDictIndexCompiler::new()
        .dict_dir(manifest_dir.join("dictionary"))
        .add_input_file(&user_csv_path)
        .userdict_out_path(&user_out_path)
        .run()
        .unwrap();

    let mut ojt_with_userdic =
        OpenJTalk::from_path_with_userdict(&dict_dir, &user_out_path).unwrap();

    for (text, expected) in &tests {
        let p = ojt_with_userdic.g2p(text).unwrap().join(" ");
        assert_eq!(&p, expected);
    }

    const XYZ: &str = "e cl k U s u w a i z e cl t o";
    assert_ne!(ojt_with_userdic.g2p("XYZ").unwrap().join(" "), XYZ);

    let mut second_csv = NamedTempFile::new().unwrap();
    writeln!(
        second_csv.as_file_mut(),
        "ＸＹＺ,,,1,名詞,一般,*,*,*,*,ＸＹＺ,エックスワイゼット,エックスワイゼット,5/9,*"
    )
    .unwrap();
    let second_csv_path = second_csv.into_temp_path();
    let second_out_path = tempfile::Builder::new()
        .prefix("haqumei,user-dict-")
        .tempfile()
        .unwrap()
        .into_temp_path();
    MecabDictIndexCompiler::new()
        .dict_dir(manifest_dir.join("dictionary"))
        .add_input_file(&second_csv_path)
        .userdict_out_path(&second_out_path)
        .run()
        .unwrap();

    let mut ojt_two = OpenJTalk::from_paths(
        &dict_dir,
        &[user_out_path.to_path_buf(), second_out_path.to_path_buf()],
    )
    .unwrap();
    for (text, expected) in &tests {
        assert_eq!(&ojt_two.g2p(text).unwrap().join(" "), expected);
    }
    assert_eq!(ojt_two.g2p("XYZ").unwrap().join(" "), XYZ);
}

/// 辞書のパスが誤っているときに、Mecab の「読み込み失敗」ではなく原因を返すこと。
#[test]
fn test_dictionary_path_errors() {
    use crate::open_jtalk::Dictionary;

    let dict_dir = GLOBAL_MECAB_DICTIONARY.load().dict_dir.clone();
    let missing = dict_dir.join("この名前のファイルは無い.dic");

    assert!(matches!(
        Dictionary::from_paths(&missing, &[] as &[&Path]),
        Err(HaqumeiError::DictionaryNotFound { .. })
    ));
    // システム辞書にファイルを渡した
    assert!(matches!(
        Dictionary::from_paths(&dict_dir.join("system.bin"), &[] as &[&Path]),
        Err(HaqumeiError::InvalidDictionaryPath(_))
    ));
    // ユーザー辞書にディレクトリを渡した
    assert!(matches!(
        Dictionary::from_paths(&dict_dir, &[&dict_dir]),
        Err(HaqumeiError::InvalidDictionaryPath(_))
    ));
}
