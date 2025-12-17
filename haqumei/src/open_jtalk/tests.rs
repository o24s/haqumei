use crate::open_jtalk::model::MecabModel;

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
    let njd =  Njd::new().unwrap();

    let njd_raw = unsafe { njd.inner.as_ref() };

    assert!(njd_raw.head.is_null());
    assert!(njd_raw.tail.is_null());
}
