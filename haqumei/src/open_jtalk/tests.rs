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
fn test_split_dictionary_entry_char_spans() {
    let mut ojt = OpenJTalk::new().unwrap();
    for (surface, orig, pron, accent, lengths) in [
        (
            "富士河口湖",
            "富士:河口湖",
            "フジ:カワグチコ",
            "1/2:4/5",
            [2, 3],
        ),
        (
            "山本五十六",
            "山本:五十六",
            "ヤマモト:イソロク",
            "0/4:0/4",
            [2, 3],
        ),
        ("𠮷田富士", "𠮷田:富士", "ヨシダ:フジ", "0/3:1/2", [2, 2]),
    ] {
        let end = 1 + surface.chars().count();
        let morphs: Vec<MecabMorph> = [
            ("前", "前", "マエ", "1/2", 0..1),
            (surface, orig, pron, accent, 1..end),
            ("町", "町", "マチ", "2/2", end..end + 1),
        ]
        .into_iter()
        .map(|(surface, orig, pron, accent, char_span)| MecabMorph {
            surface: surface.into(),
            feature: format!(
                "{surface},名詞,固有名詞,地域,一般,*,*,{orig},{pron},{pron},{accent},*"
            ),
            left_id: 1353,
            right_id: 1353,
            pos_id: 0,
            word_cost: 0,
            is_unknown: false,
            char_span,
            dictionary_index: 0,
            is_ignored: false,
        })
        .collect();
        let features = ojt
            .run_njd_from_mecab(morphs.iter().map(|m| m.feature.as_str()))
            .unwrap();
        let middle = 1 + lengths[0];
        assert_eq!(
            njd_char_spans(&features, &morphs),
            [0..1, 1..middle, middle..end, end..end + 1],
            "{surface}"
        );
    }
}

#[test]
fn test_userdict() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("Failed to get MANIFEST_DIR");
    let manifest_dir = Path::new(&manifest_dir);

    let mut ojt = OpenJTalk::new().unwrap();

    let tests = vec![
        ("nnmn", "n a n a m i N"),
        ("GNU", "g u n u u"),
        ("∩！？", "a i z u"),
    ];

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
    writeln!(
        user_csv.as_file_mut(),
        "∩！？,1345,1345,-5000,名詞,一般,*,*,*,*,∩！？,アイズ,アイズ,1/3,C1"
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

    let mut protected = crate::Haqumei::from_path_with_userdict(
        &dict_dir,
        &user_out_path,
        crate::HaqumeiOptions {
            protect_user_dict_readings: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(protected.g2k("GNU2").unwrap(), "グヌーニ");
    let expected = ["g", "u", "n", "u", "u", "n", "i"];
    assert_eq!(protected.g2p("GNU2").unwrap(), expected);
    let mapping = protected.g2p_mapping("GNU2").unwrap();
    assert_eq!(mapping.len(), 1);
    assert_eq!(mapping[0].phonemes, expected);
    assert_eq!(mapping[0].char_span, 0..4);
    let frontend = protected.run_frontend("GNU2").unwrap();
    assert_eq!(frontend[0].read, "グヌーニ");
    assert_eq!(frontend[0].mora_size, 4);
    assert_eq!(protected.g2p_batch(&["GNU2"]).unwrap()[0], expected);

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
