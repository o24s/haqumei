use crate::open_jtalk::model::MecabModel;
use std::{env, io::Write};
use tempfile::NamedTempFile;

use super::*;

#[test]
#[cfg(feature = "embed-dictionary")]
fn test_global_dictionary() {
    let model = &GLOBAL_MECAB_DICTIONARY.load_full().model;
    assert!(model.is_initialized());
}

/// ```no_run
/// BOOL Mecab_initialize(Mecab *m)
/// {
///    m->feature = NULL;
///    m->size = 0;
///    m->model = NULL;
///    m->tagger = NULL;
///    m->lattice = NULL;
///    return TRUE;
/// }
/// ```
#[test]
fn test_mecab_new() {
    {
        let mecab = Mecab::new().unwrap();
        let mecab_raw = unsafe { mecab.inner.as_ref() };

        assert!(mecab_raw.feature.is_null());
        assert!(mecab_raw.size == 0);
        assert!(mecab_raw.model.is_null());
        assert!(mecab_raw.tagger.is_null());
        assert!(mecab_raw.lattice.is_null());
    }

    #[cfg(feature = "embed-dictionary")]
    {
        let model = Dictionary::from_embedded().unwrap().model;
        let mecab = Mecab::from_model(&model).unwrap();
        let mecab_raw = unsafe { mecab.inner.as_ref() };

        assert!(mecab_raw.feature.is_null());
        assert!(mecab_raw.size == 0);

        // これらは `from_model` を通してその引数の `MecabModel` からポインタがコピーされる
        assert!(!mecab_raw.model.is_null());
        assert!(!mecab_raw.tagger.is_null());
        assert!(!mecab_raw.lattice.is_null());

        drop(mecab);

        // `from_model` を通して作成された model は解放されない
        assert!(!model.as_ref().ptr.is_null());
    }
}

#[test]
fn test_model_new() {
    let model = MecabModel::new_uninitialized();
    assert!(model.ptr.is_null());

    #[cfg(feature = "embed-dictionary")]
    {
        let model = Dictionary::from_embedded().unwrap().model;

        assert!(model.is_initialized())
    }
}

/// ```no_run
/// void JPCommon_initialize(JPCommon * jpcommon)
/// {
///    jpcommon->head = NULL;
///    jpcommon->tail = NULL;
///    jpcommon->label = NULL;
/// }
/// ```
#[test]
fn test_jpcommon() {
    let jpcommon = JpCommon::new().unwrap();
    let jpcommon_raw = unsafe { jpcommon.inner.as_ref() };

    assert!(jpcommon_raw.head.is_null());
    assert!(jpcommon_raw.tail.is_null());
    assert!(jpcommon_raw.label.is_null());
}

/// ```no_run
/// void NJD_initialize(NJD * njd)
/// {
///    njd->head = NULL;
///    njd->tail = NULL;
/// }
/// ```
#[test]
fn test_njd() {
    let njd = Njd::new().unwrap();

    let njd_raw = unsafe { njd.inner.as_ref() };

    assert!(njd_raw.head.is_null());
    assert!(njd_raw.tail.is_null());
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
        OpenJTalk::from_path_with_userdict(&dict_dir, user_out_path).unwrap();

    for (text, expected) in &tests {
        let p = ojt_with_userdic.g2p(text).unwrap().join(" ");
        assert_eq!(&p, expected);
    }
}
