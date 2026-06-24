use std::ffi::CString;
use std::{env, io::Write};

use crate::open_jtalk::model::MecabModel;
use crate::utils::ptr_to_str_unchecked;
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

unsafe fn run_push_word_state_test(
    inputs: &[(&str, &str, &str, &str, i32, i32)],
    expected_phonemes: &[&str],
    expected_short_pause_flag: i32,
) {
    unsafe {
        let label =
            libc::calloc(1, std::mem::size_of::<ffi::JPCommonLabel>()) as *mut ffi::JPCommonLabel;
        ffi::JPCommonLabel_initialize(label);

        for (pron, pos, ctype, cform, acc, chain) in inputs {
            let pron_c = CString::new(*pron).unwrap();
            let pos_c = CString::new(*pos).unwrap();
            let ctype_c = CString::new(*ctype).unwrap();
            let cform_c = CString::new(*cform).unwrap();

            let _ = super::jpcommon_push_word::JPCommonLabel_push_word(
                label,
                pron_c.as_ptr(),
                pos_c.as_ptr(),
                ctype_c.as_ptr(),
                cform_c.as_ptr(),
                *acc,
                *chain,
            );
        }

        assert_eq!(
            (*label).short_pause_flag,
            expected_short_pause_flag,
            "short_pause_flag mismatch"
        );

        let mut actual_phonemes = Vec::new();
        let mut p = (*label).phoneme_head;
        let mut p_idx = 0;
        while !p.is_null() {
            assert!(p_idx < 10000, "Infinite loop detected in phoneme list");
            if (*p).phoneme.is_null() {
                actual_phonemes.push("".to_string());
            } else {
                actual_phonemes.push(CStr::from_ptr((*p).phoneme).to_string_lossy().into_owned());
            }
            p = (*p).next;
            p_idx += 1;
        }

        assert_eq!(
            actual_phonemes, expected_phonemes,
            "Phoneme sequence mismatch"
        );

        ffi::JPCommonLabel_clear(label);
        libc::free(label as *mut _);
    }
}

#[test]
fn test_jpcommon_label_push_word_basic() {
    unsafe {
        run_push_word_state_test(
            &[("コンニチワ", "名詞", "*", "*", 0, 0)],
            &["k", "o", "N", "n", "i", "ch", "i", "w", "a"],
            0,
        );
    }
}

#[test]
fn test_jpcommon_label_push_word_pause_and_chain() {
    unsafe {
        run_push_word_state_test(
            &[
                ("オハヨー", "感動詞", "*", "*", 1, 0),
                ("、", "記号", "*", "*", 0, 0),
                ("ゴザイマス", "助動詞", "*", "*", 0, 1),
            ],
            &[
                "o", "h", "a", "y", "o", "o", "pau", "g", "o", "z", "a", "i", "m", "a", "s", "u",
            ],
            0,
        );
    }
}

#[test]
fn test_jpcommon_label_push_word_unvoice_and_marks() {
    unsafe {
        run_push_word_state_test(
            &[
                ("キ’", "名詞", "*", "*", 1, 0),
                ("！", "記号", "*", "*", 0, 0),
                ("？", "記号", "*", "*", 0, 0),
            ],
            &["k", "I"],
            1,
        );
    }
}

#[test]
fn test_jpcommon_label_push_word_unknown_mora() {
    unsafe {
        run_push_word_state_test(&[("マ*ホウ", "名詞", "*", "*", 0, 0)], &["m", "a"], 0);
    }
}

fn run_extract_fullcontext_comparison(njd_features: &[NjdFeature]) {
    let mut jt = OpenJTalk::new().unwrap();

    let rust_labels = jt.extract_fullcontext_labels(njd_features).unwrap();
    let rust_strings: Vec<String> = rust_labels.into_iter().map(|l| l.to_string()).collect();

    unsafe {
        crate::OpenJTalk::features_to_njd(njd_features, &mut jt.njd).unwrap();
        let jp = jt.jp_common.inner.as_mut();
        let njd = jt.njd.inner.as_mut();

        ffi::njd2jpcommon(jp, njd);
        ffi::JPCommon_make_label(jp);

        let label = jp.label;
        let size = ffi::JPCommonLabel_get_size(label);
        let features_ptr = ffi::JPCommonLabel_get_feature(label);

        let mut c_strings = Vec::with_capacity(size as usize);
        if !features_ptr.is_null() {
            for i in 0..size {
                let p = *features_ptr.add(i as usize);
                if !p.is_null() {
                    c_strings.push(CStr::from_ptr(p).to_string_lossy().into_owned());
                }
            }
        }

        assert_eq!(
            rust_strings.len(),
            c_strings.len(),
            "Extracted labels count mismatch"
        );
        for (i, (r_str, c_str)) in rust_strings.iter().zip(c_strings.iter()).enumerate() {
            assert_eq!(
                r_str, c_str,
                "Label mismatch at index {i}\nRust: {r_str}\n   C: {c_str}"
            );
        }

        ffi::JPCommon_refresh(jp);
        ffi::NJD_refresh(njd);
    }
}

#[test]
fn test_extract_fullcontext_basic() {
    let njd_features = vec![NjdFeature {
        string: "こんにちは".to_string(),
        pos: "感動詞".to_string(),
        pos_group1: "*".to_string(),
        pos_group2: "*".to_string(),
        pos_group3: "*".to_string(),
        ctype: "*".to_string(),
        cform: "*".to_string(),
        orig: "こんにちは".to_string(),
        read: "コンニチワ".to_string(),
        pron: "コンニチワ".to_string(),
        acc: 0,
        mora_size: 5,
        chain_rule: "*".to_string(),
        chain_flag: -1,
    }];
    run_extract_fullcontext_comparison(&njd_features);
}

#[test]
fn test_extract_fullcontext_complex() {
    let njd_features = vec![
        NjdFeature {
            string: "今日".to_string(),
            pos: "名詞".to_string(),
            pos_group1: "副詞可能".to_string(),
            pos_group2: "*".to_string(),
            pos_group3: "*".to_string(),
            ctype: "*".to_string(),
            cform: "*".to_string(),
            orig: "今日".to_string(),
            read: "キョウ".to_string(),
            pron: "キョー".to_string(),
            acc: 1,
            mora_size: 2,
            chain_rule: "C1".to_string(),
            chain_flag: -1,
        },
        NjdFeature {
            string: "は".to_string(),
            pos: "助詞".to_string(),
            pos_group1: "係助詞".to_string(),
            pos_group2: "*".to_string(),
            pos_group3: "*".to_string(),
            ctype: "*".to_string(),
            cform: "*".to_string(),
            orig: "は".to_string(),
            read: "ハ".to_string(),
            pron: "ワ".to_string(),
            acc: 0,
            mora_size: 1,
            chain_rule: "*".to_string(),
            chain_flag: 1,
        },
        NjdFeature {
            string: "です".to_string(),
            pos: "助動詞".to_string(),
            pos_group1: "*".to_string(),
            pos_group2: "*".to_string(),
            pos_group3: "*".to_string(),
            ctype: "特殊・デス".to_string(),
            cform: "基本形".to_string(),
            orig: "です".to_string(),
            read: "デス".to_string(),
            pron: "デス".to_string(),
            acc: 0,
            mora_size: 2,
            chain_rule: "*".to_string(),
            chain_flag: 1,
        },
        NjdFeature {
            string: "ツォー’".to_string(),
            pos: "名詞".to_string(),
            pos_group1: "*".to_string(),
            pos_group2: "*".to_string(),
            pos_group3: "*".to_string(),
            ctype: "*".to_string(),
            cform: "*".to_string(),
            orig: "ツォー’".to_string(),
            read: "ツォー’".to_string(),
            pron: "ツォー’".to_string(),
            acc: 1,
            mora_size: 2,
            chain_rule: "*".to_string(),
            chain_flag: -1,
        },
        NjdFeature {
            string: "、".to_string(),
            pos: "記号".to_string(),
            pos_group1: "読点".to_string(),
            pos_group2: "*".to_string(),
            pos_group3: "*".to_string(),
            ctype: "*".to_string(),
            cform: "*".to_string(),
            orig: "、".to_string(),
            read: "、".to_string(),
            pron: "、".to_string(),
            acc: 0,
            mora_size: 0,
            chain_rule: "*".to_string(),
            chain_flag: -1,
        },
        NjdFeature {
            string: "ヴャ！".to_string(),
            pos: "感動詞".to_string(),
            pos_group1: "*".to_string(),
            pos_group2: "*".to_string(),
            pos_group3: "*".to_string(),
            ctype: "*".to_string(),
            cform: "*".to_string(),
            orig: "ヴャ！".to_string(),
            read: "ヴャ！".to_string(),
            pron: "ヴャ！".to_string(),
            acc: 0,
            mora_size: 1,
            chain_rule: "*".to_string(),
            chain_flag: -1,
        },
        NjdFeature {
            string: "？".to_string(),
            pos: "記号".to_string(),
            pos_group1: "*".to_string(),
            pos_group2: "*".to_string(),
            pos_group3: "*".to_string(),
            ctype: "*".to_string(),
            cform: "*".to_string(),
            orig: "？".to_string(),
            read: "？".to_string(),
            pron: "？".to_string(),
            acc: 0,
            mora_size: 0,
            chain_rule: "*".to_string(),
            chain_flag: -1,
        },
        NjdFeature {
            string: "ミョン".to_string(),
            pos: "名詞".to_string(),
            pos_group1: "*".to_string(),
            pos_group2: "*".to_string(),
            pos_group3: "*".to_string(),
            ctype: "*".to_string(),
            cform: "*".to_string(),
            orig: "ミョン".to_string(),
            read: "ミョン".to_string(),
            pron: "ミョン".to_string(),
            acc: 1,
            mora_size: 2,
            chain_rule: "*".to_string(),
            chain_flag: -1,
        },
    ];
    run_extract_fullcontext_comparison(&njd_features);
}

macro_rules! assert_cstr_eq {
    ($c_ptr:expr, $r_ptr:expr, $msg:expr) => {
        let c_str = ptr_to_str_unchecked($c_ptr);
        let r_str = ptr_to_str_unchecked($r_ptr);
        assert_eq!(c_str, r_str, "{}: C='{}', Rust='{}'", $msg, c_str, r_str);
    };
}

macro_rules! assert_ptr_presence_eq {
    ($c_ptr:expr, $r_ptr:expr, $msg:expr) => {
        assert_eq!(
            $c_ptr.is_null(),
            $r_ptr.is_null(),
            "{}: C is_null={}, Rust is_null={}",
            $msg,
            $c_ptr.is_null(),
            $r_ptr.is_null()
        );
    };
}

unsafe fn assert_label_eq(c_label: *mut ffi::JPCommonLabel, r_label: *mut ffi::JPCommonLabel) {
    unsafe {
        assert_eq!(
            (*c_label).is_valid,
            (*r_label).is_valid,
            "is_valid mismatch"
        );
        assert_eq!(
            (*c_label).short_pause_flag,
            (*r_label).short_pause_flag,
            "short_pause_flag mismatch"
        );

        let mut c_p = (*c_label).phoneme_head;
        let mut r_p = (*r_label).phoneme_head;
        let mut p_idx = 0;
        while !c_p.is_null() && !r_p.is_null() {
            assert!(p_idx < 10000, "Infinite loop detected in phoneme list");
            let msg = format!("Phoneme[{}]", p_idx);
            assert_cstr_eq!((*c_p).phoneme, (*r_p).phoneme, format!("{}.phoneme", msg));
            assert_ptr_presence_eq!((*c_p).prev, (*r_p).prev, format!("{}.prev", msg));
            assert_ptr_presence_eq!((*c_p).next, (*r_p).next, format!("{}.next", msg));
            assert_ptr_presence_eq!((*c_p).up, (*r_p).up, format!("{}.up", msg));

            if !(*c_p).up.is_null() {
                assert_cstr_eq!(
                    (*(*c_p).up).mora,
                    (*(*r_p).up).mora,
                    format!("{}.up.mora", msg)
                );
            }
            c_p = (*c_p).next;
            r_p = (*r_p).next;
            p_idx += 1;
        }
        assert!(
            c_p.is_null() && r_p.is_null(),
            "Phoneme list length mismatch"
        );

        let mut c_m = (*c_label).mora_head;
        let mut r_m = (*r_label).mora_head;
        let mut m_idx = 0;
        while !c_m.is_null() && !r_m.is_null() {
            assert!(m_idx < 10000, "Infinite loop detected in mora list");
            let msg = format!("Mora[{}]", m_idx);
            assert_cstr_eq!((*c_m).mora, (*r_m).mora, format!("{}.mora", msg));
            assert_ptr_presence_eq!((*c_m).head, (*r_m).head, format!("{}.head", msg));
            assert_ptr_presence_eq!((*c_m).tail, (*r_m).tail, format!("{}.tail", msg));
            assert_ptr_presence_eq!((*c_m).prev, (*r_m).prev, format!("{}.prev", msg));
            assert_ptr_presence_eq!((*c_m).next, (*r_m).next, format!("{}.next", msg));
            assert_ptr_presence_eq!((*c_m).up, (*r_m).up, format!("{}.up", msg));

            if !(*c_m).head.is_null() {
                assert_cstr_eq!(
                    (*(*c_m).head).phoneme,
                    (*(*r_m).head).phoneme,
                    format!("{}.head.phoneme", msg)
                );
            }
            if !(*c_m).up.is_null() {
                assert_cstr_eq!(
                    (*(*c_m).up).pron,
                    (*(*r_m).up).pron,
                    format!("{}.up.pron", msg)
                );
            }
            c_m = (*c_m).next;
            r_m = (*r_m).next;
            m_idx += 1;
        }
        assert!(c_m.is_null() && r_m.is_null(), "Mora list length mismatch");

        let mut c_w = (*c_label).word_head;
        let mut r_w = (*r_label).word_head;
        let mut w_idx = 0;
        while !c_w.is_null() && !r_w.is_null() {
            assert!(w_idx < 10000, "Infinite loop detected in word list");
            let msg = format!("Word[{}]", w_idx);
            assert_cstr_eq!((*c_w).pron, (*r_w).pron, format!("{}.pron", msg));
            assert_cstr_eq!((*c_w).pos, (*r_w).pos, format!("{}.pos", msg));
            assert_cstr_eq!((*c_w).ctype, (*r_w).ctype, format!("{}.ctype", msg));
            assert_cstr_eq!((*c_w).cform, (*r_w).cform, format!("{}.cform", msg));

            assert_ptr_presence_eq!((*c_w).head, (*r_w).head, format!("{}.head", msg));
            assert_ptr_presence_eq!((*c_w).tail, (*r_w).tail, format!("{}.tail", msg));
            assert_ptr_presence_eq!((*c_w).prev, (*r_w).prev, format!("{}.prev", msg));
            assert_ptr_presence_eq!((*c_w).next, (*r_w).next, format!("{}.next", msg));
            assert_ptr_presence_eq!((*c_w).up, (*r_w).up, format!("{}.up", msg));

            if !(*c_w).head.is_null() {
                assert_cstr_eq!(
                    (*(*c_w).head).mora,
                    (*(*r_w).head).mora,
                    format!("{}.head.mora", msg)
                );
            }
            c_w = (*c_w).next;
            r_w = (*r_w).next;
            w_idx += 1;
        }
        assert!(c_w.is_null() && r_w.is_null(), "Word list length mismatch");

        let mut c_a = (*c_label).accent_head;
        let mut r_a = (*r_label).accent_head;
        let mut a_idx = 0;
        while !c_a.is_null() && !r_a.is_null() {
            assert!(
                a_idx < 10000,
                "Infinite loop detected in accent phrase list"
            );
            let msg = format!("AccentPhrase[{}]", a_idx);
            assert_eq!((*c_a).accent, (*r_a).accent, "{}.accent mismatch", msg);
            assert_cstr_eq!((*c_a).emotion, (*r_a).emotion, format!("{}.emotion", msg));
            assert_cstr_eq!((*c_a).excl, (*r_a).excl, format!("{}.excl", msg));

            assert_ptr_presence_eq!((*c_a).head, (*r_a).head, format!("{}.head", msg));
            assert_ptr_presence_eq!((*c_a).tail, (*r_a).tail, format!("{}.tail", msg));
            assert_ptr_presence_eq!((*c_a).prev, (*r_a).prev, format!("{}.prev", msg));
            assert_ptr_presence_eq!((*c_a).next, (*r_a).next, format!("{}.next", msg));
            assert_ptr_presence_eq!((*c_a).up, (*r_a).up, format!("{}.up", msg));

            if !(*c_a).head.is_null() {
                assert_cstr_eq!(
                    (*(*c_a).head).pron,
                    (*(*r_a).head).pron,
                    format!("{}.head.pron", msg)
                );
            }
            c_a = (*c_a).next;
            r_a = (*r_a).next;
            a_idx += 1;
        }
        assert!(
            c_a.is_null() && r_a.is_null(),
            "AccentPhrase list length mismatch"
        );

        let mut c_b = (*c_label).breath_head;
        let mut r_b = (*r_label).breath_head;
        let mut b_idx = 0;
        while !c_b.is_null() && !r_b.is_null() {
            assert!(b_idx < 10000, "Infinite loop detected in breath group list");
            let msg = format!("BreathGroup[{}]", b_idx);
            assert_ptr_presence_eq!((*c_b).head, (*r_b).head, format!("{}.head", msg));
            assert_ptr_presence_eq!((*c_b).tail, (*r_b).tail, format!("{}.tail", msg));
            assert_ptr_presence_eq!((*c_b).prev, (*r_b).prev, format!("{}.prev", msg));
            assert_ptr_presence_eq!((*c_b).next, (*r_b).next, format!("{}.next", msg));

            if !(*c_b).head.is_null() {
                assert_eq!(
                    (*(*c_b).head).accent,
                    (*(*r_b).head).accent,
                    "{}.head.accent",
                    msg
                );
            }
            c_b = (*c_b).next;
            r_b = (*r_b).next;
            b_idx += 1;
        }
        assert!(
            c_b.is_null() && r_b.is_null(),
            "BreathGroup list length mismatch"
        );
    }
}

fn run_comparison_test(inputs: &[(&str, &str, &str, &str, i32, i32)]) {
    unsafe {
        let c_label =
            libc::calloc(1, std::mem::size_of::<ffi::JPCommonLabel>()) as *mut ffi::JPCommonLabel;
        let r_label =
            libc::calloc(1, std::mem::size_of::<ffi::JPCommonLabel>()) as *mut ffi::JPCommonLabel;

        ffi::JPCommonLabel_initialize(c_label);
        ffi::JPCommonLabel_initialize(r_label);

        for (pron, pos, ctype, cform, acc, chain) in inputs {
            let pron_c = CString::new(*pron).unwrap();
            let pos_c = CString::new(*pos).unwrap();
            let ctype_c = CString::new(*ctype).unwrap();
            let cform_c = CString::new(*cform).unwrap();

            ffi::JPCommonLabel_push_word(
                c_label,
                pron_c.as_ptr(),
                pos_c.as_ptr(),
                ctype_c.as_ptr(),
                cform_c.as_ptr(),
                *acc,
                *chain,
            );

            let res = super::jpcommon_push_word::JPCommonLabel_push_word(
                r_label,
                pron_c.as_ptr(),
                pos_c.as_ptr(),
                ctype_c.as_ptr(),
                cform_c.as_ptr(),
                *acc,
                *chain,
            );

            if (*c_label).is_valid == 0 {
                assert!(
                    res.is_err(),
                    "Rust should return Err when C sets is_valid=0"
                );
            }

            assert_label_eq(c_label, r_label);
        }

        ffi::JPCommonLabel_clear(c_label);
        ffi::JPCommonLabel_clear(r_label);
        libc::free(c_label as *mut _);
        libc::free(r_label as *mut _);
    }
}

#[test]
fn test_push_word_basic() {
    run_comparison_test(&[("コンニチワ", "名詞", "*", "*", 0, 0)]);
}

#[test]
fn test_push_word_chain_and_accents() {
    run_comparison_test(&[
        ("オハヨー", "感動詞", "*", "*", 1, 0),
        ("ゴザイマス", "助動詞", "*", "*", 0, 1),
        ("キョーワ", "名詞", "*", "*", 1, 0),
        ("イイ", "形容詞", "*", "*", 1, 0),
        ("テンキデス", "名詞", "*", "*", 1, 1),
    ]);
}

#[test]
fn test_push_word_unvoice_and_long_vowel() {
    run_comparison_test(&[
        ("キ’", "名詞", "*", "*", 1, 0),
        ("アー", "感動詞", "*", "*", 0, 0),
        ("スッ", "感動詞", "*", "*", 1, 0),
    ]);
}

#[test]
fn test_push_word_marks_pause_question_exclamation() {
    run_comparison_test(&[
        ("ナンデ", "名詞", "*", "*", 1, 0),
        ("？", "記号", "*", "*", 0, 0),
        ("スゴイ", "形容詞", "*", "*", 2, 0),
        ("！", "記号", "*", "*", 0, 0),
        ("ソレデ", "接続詞", "*", "*", 0, 0),
        ("、", "記号", "*", "*", 0, 0),
        ("アア", "感動詞", "*", "*", 0, 0),
    ]);
}

#[test]
fn test_push_word_unknown_mora_fail_soft() {
    run_comparison_test(&[
        ("マ*ホウ", "名詞", "*", "*", 0, 0),
        ("デス", "助動詞", "*", "*", 0, 1),
    ]);
}

#[test]
fn test_push_word_invalid_start_mora() {
    run_comparison_test(&[
        ("ー", "記号", "*", "*", 0, 0),
        ("’", "記号", "*", "*", 0, 0),
        ("？", "記号", "*", "*", 0, 0),
        ("、", "記号", "*", "*", 0, 0),
        ("テスト", "名詞", "*", "*", 1, 0),
        ("、", "記号", "*", "*", 0, 0),
        ("、", "記号", "*", "*", 0, 0),
    ]);
}

#[test]
fn test_push_word_complex_case() {
    run_comparison_test(&[
        ("ツォ", "名詞", "*", "*", 1, 0),
        ("ー", "記号", "*", "*", 0, 1),
        ("’", "記号", "*", "*", 0, 1),
        ("、", "記号", "*", "*", 0, 0),
        ("ヴャ", "感動詞", "*", "*", 0, 0),
        ("！", "記号", "*", "*", 0, 0),
        ("？", "記号", "*", "*", 0, 0),
        ("ミョ", "名詞", "*", "*", 1, 0),
        ("ン", "名詞", "*", "*", 0, 1),
    ]);
}
