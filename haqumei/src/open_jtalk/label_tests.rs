use super::njd::{extract_fullcontext_labels, extract_phonemes};
use crate::NjdFeature;

fn make_label(features: &[NjdFeature]) -> Vec<String> {
    extract_fullcontext_labels(features)
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect()
}

// 移行前の Open JTalk と一致していたラベルを固定し、音素・アクセント・句境界を比較する。
fn assert_labels(features: &[NjdFeature], expected: &str) {
    let labels = extract_fullcontext_labels(features).unwrap();
    let actual: Vec<_> = labels.iter().map(ToString::to_string).collect();
    let expected: Vec<_> = expected.lines().collect();
    assert_eq!(actual, expected);
}

fn assert_word_labels(inputs: &[(&str, &str, &str, &str, i32, i32)], expected: &str) {
    assert_labels(&push_word_features(inputs), expected);
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
    assert_labels(
        &njd_features,
        include_str!("label_tests/test_extract_fullcontext_basic.lab"),
    );
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
    assert_labels(
        &njd_features,
        include_str!("label_tests/test_extract_fullcontext_complex.lab"),
    );
}

#[test]
fn test_push_word_basic() {
    assert_word_labels(
        &[("コンニチワ", "名詞", "*", "*", 0, 0)],
        include_str!("label_tests/test_push_word_basic.lab"),
    );
}

#[test]
fn test_push_word_chain_and_accents() {
    assert_word_labels(
        &[
            ("オハヨー", "感動詞", "*", "*", 1, 0),
            ("ゴザイマス", "助動詞", "*", "*", 0, 1),
            ("キョーワ", "名詞", "*", "*", 1, 0),
            ("イイ", "形容詞", "*", "*", 1, 0),
            ("テンキデス", "名詞", "*", "*", 1, 1),
        ],
        include_str!("label_tests/test_push_word_chain_and_accents.lab"),
    );
}

#[test]
fn test_push_word_unvoice_and_long_vowel() {
    assert_word_labels(
        &[
            ("キ’", "名詞", "*", "*", 1, 0),
            ("アー", "感動詞", "*", "*", 0, 0),
            ("スッ", "感動詞", "*", "*", 1, 0),
        ],
        include_str!("label_tests/test_push_word_unvoice_and_long_vowel.lab"),
    );
}

#[test]
fn test_push_word_marks_pause_question_exclamation() {
    assert_word_labels(
        &[
            ("ナンデ", "名詞", "*", "*", 1, 0),
            ("？", "記号", "*", "*", 0, 0),
            ("スゴイ", "形容詞", "*", "*", 2, 0),
            ("！", "記号", "*", "*", 0, 0),
            ("ソレデ", "接続詞", "*", "*", 0, 0),
            ("、", "記号", "*", "*", 0, 0),
            ("アア", "感動詞", "*", "*", 0, 0),
        ],
        include_str!("label_tests/test_push_word_marks_pause_question_exclamation.lab"),
    );
}

#[test]
fn test_push_word_unknown_mora_fail_soft() {
    assert_word_labels(
        &[
            ("マ*ホウ", "名詞", "*", "*", 0, 0),
            ("デス", "助動詞", "*", "*", 0, 1),
        ],
        include_str!("label_tests/test_push_word_unknown_mora_fail_soft.lab"),
    );
}

#[test]
fn test_push_word_invalid_start_mora() {
    assert_word_labels(
        &[
            ("ー", "記号", "*", "*", 0, 0),
            ("’", "記号", "*", "*", 0, 0),
            ("？", "記号", "*", "*", 0, 0),
            ("、", "記号", "*", "*", 0, 0),
            ("テスト", "名詞", "*", "*", 1, 0),
            ("、", "記号", "*", "*", 0, 0),
            ("、", "記号", "*", "*", 0, 0),
        ],
        include_str!("label_tests/test_push_word_invalid_start_mora.lab"),
    );
}

#[test]
fn test_push_word_complex_case() {
    assert_word_labels(
        &[
            ("ツォ", "名詞", "*", "*", 1, 0),
            ("ー", "記号", "*", "*", 0, 1),
            ("’", "記号", "*", "*", 0, 1),
            ("、", "記号", "*", "*", 0, 0),
            ("ヴャ", "感動詞", "*", "*", 0, 0),
            ("！", "記号", "*", "*", 0, 0),
            ("？", "記号", "*", "*", 0, 0),
            ("ミョ", "名詞", "*", "*", 1, 0),
            ("ン", "名詞", "*", "*", 0, 1),
        ],
        include_str!("label_tests/test_push_word_complex_case.lab"),
    );
}
fn push_word_features(inputs: &[(&str, &str, &str, &str, i32, i32)]) -> Vec<NjdFeature> {
    inputs
        .iter()
        .map(|&(pron, pos, ctype, cform, acc, chain_flag)| NjdFeature {
            string: pron.to_owned(),
            pos: pos.to_owned(),
            pos_group1: match pos {
                "名詞" => "一般",
                "形容詞" => "自立",
                _ => "*",
            }
            .to_owned(),
            pos_group2: "*".to_owned(),
            pos_group3: "*".to_owned(),
            ctype: ctype.to_owned(),
            cform: cform.to_owned(),
            orig: pron.to_owned(),
            read: pron.to_owned(),
            pron: pron.to_owned(),
            acc,
            mora_size: 0,
            chain_rule: "*".to_owned(),
            chain_flag,
        })
        .collect()
}

#[test]
fn test_push_word_filler() {
    assert_word_labels(
        &[("エート", "フィラー", "*", "*", 0, 0)],
        include_str!("label_tests/test_push_word_filler.lab"),
    );
}

#[test]
fn test_push_word_pause_and_chain() {
    assert_word_labels(
        &[
            ("オハヨー", "感動詞", "*", "*", 1, 0),
            ("、", "記号", "*", "*", 0, 0),
            ("ゴザイマス", "助動詞", "*", "*", 0, 1),
        ],
        include_str!("label_tests/test_push_word_pause_and_chain.lab"),
    );
}

#[test]
fn standalone_unvoicing_mark_does_not_insert_a_pause() {
    let features = push_word_features(&[
        ("ア", "感動詞", "*", "*", 0, 0),
        ("’", "記号", "*", "*", 0, 0),
        ("イ", "感動詞", "*", "*", 0, 0),
    ]);
    let labels = extract_fullcontext_labels(&features).unwrap();
    let phonemes: Vec<_> = labels
        .iter()
        .map(|label| label.phoneme.c.as_deref().unwrap())
        .collect();
    assert_eq!(phonemes, ["sil", "a", "i", "sil"]);
}

fn editable_feature() -> NjdFeature {
    NjdFeature {
        string: "ア".to_owned(),
        pos: "動詞".to_owned(),
        pos_group1: "自立".to_owned(),
        pos_group2: "*".to_owned(),
        pos_group3: "*".to_owned(),
        ctype: "一段".to_owned(),
        cform: "基本形".to_owned(),
        orig: "ア".to_owned(),
        read: "ア".to_owned(),
        pron: "ア".to_owned(),
        acc: 1,
        mora_size: 1,
        chain_rule: "*".to_owned(),
        chain_flag: 0,
    }
}

#[test]
fn unregistered_attributes_preserve_labels_and_phonemes() {
    let pos_labels = include_str!("label_tests/test_unknown_pos.lab");
    let ctype_labels = include_str!("label_tests/test_unknown_ctype.lab");
    let cform_labels = include_str!("label_tests/test_unknown_cform.lab");
    for (field, value, expected) in [
        ("pos", "独自品詞", pos_labels),
        ("pos_group1", "独自細分類", pos_labels),
        ("pos_group2", "独自細分類", pos_labels),
        ("pos_group3", "独自細分類", pos_labels),
        ("ctype", "独自活用", ctype_labels),
        ("ctype", "ラ変・独自", ctype_labels),
        ("cform", "独自活用形", cform_labels),
    ] {
        let mut feature = editable_feature();
        let target = match field {
            "pos" => &mut feature.pos,
            "pos_group1" => &mut feature.pos_group1,
            "pos_group2" => &mut feature.pos_group2,
            "pos_group3" => &mut feature.pos_group3,
            "ctype" => &mut feature.ctype,
            "cform" => &mut feature.cform,
            _ => unreachable!(),
        };
        *target = value.to_owned();
        let features = [feature];
        assert_eq!(
            make_label(&features),
            expected.lines().collect::<Vec<_>>(),
            "{field}={value}"
        );
        assert_eq!(
            extract_phonemes(&features).unwrap(),
            [crate::Phoneme::A],
            "{field}={value}"
        );
    }
}

#[test]
fn original_pos_details_distinguish_registered_and_unknown_categories() {
    for (fields, expected) in [
        (["助動詞", "独自細分類", "*", "*"], None),
        (["助動詞", "非自立", "助動詞語幹", "*"], Some(10)),
        (["名詞", "特殊", "*", "*"], None),
        (["名詞", "特殊", "助動詞語幹", "*"], Some(2)),
        (["助動詞", "", "", ""], Some(10)),
    ] {
        let feature = NjdFeature {
            pos: fields[0].to_owned(),
            pos_group1: fields[1].to_owned(),
            pos_group2: fields[2].to_owned(),
            pos_group3: fields[3].to_owned(),
            ..editable_feature()
        };
        let labels = make_label(&[feature]);
        let label: crate::Label = labels[1].parse().unwrap();
        assert_eq!(label.word_curr.unwrap().pos, expected, "{fields:?}");
    }
}
