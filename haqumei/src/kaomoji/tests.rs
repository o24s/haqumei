use crate::{Haqumei, HaqumeiOptions, UnicodeNormalization};

#[test]
fn user_dictionary_entries_are_kept_even_without_reading_protection() {
    use crate::{MecabDictIndexCompiler, open_jtalk::Dictionary};
    let dictionary = Dictionary::from_embedded().unwrap();
    let temp = tempfile::tempdir().unwrap();
    for (i, (surface, text, reading)) in [
        ("ω", "前(-'ω')後", "オメガ"),
        ("（＾＿＾）", "前(^_^)後", "カオ"),
        ("前（＾＿＾）後", "前(^_^)後", "ゼンタイ"),
        ("ノ", "前(^_^)ノ後", "ウデ"),
        ("人", "(^ω^)人(^ω^)", "ヒト"),
        ("人", "(^o^)人(^o^)", "ヒト"),
        ("ｏ", "(^o^)人(^o^)", "オー"),
        ("っ", "(^ω^)っ)ω^)", "テ"),
        ("σ", "(^ω^)σ\")ω^)", "シグマ"),
        ("☆", "(^_^)つー☆", "ホシ"),
    ]
    .into_iter()
    .enumerate()
    {
        let csv = temp.path().join(format!("user-{i}.csv"));
        let dic = temp.path().join(format!("user-{i}.dic"));
        std::fs::write(&csv,format!("{surface},1345,1345,-20000,名詞,一般,*,*,*,*,{surface},{reading},{reading},0/3,C3\n")).unwrap();
        MecabDictIndexCompiler::new()
            .dict_dir(&dictionary.dict_dir)
            .userdict_out_path(&dic)
            .add_input_file(&csv)
            .run()
            .unwrap();
        for mode in [
            UnicodeNormalization::None,
            UnicodeNormalization::Nfc,
            UnicodeNormalization::Nfkc,
        ] {
            for protect in [false, true] {
                let mut engine = Haqumei::from_path_with_userdict(
                    &dictionary.dict_dir,
                    &dic,
                    HaqumeiOptions {
                        normalize_unicode: mode,
                        protect_user_dict_readings: protect,
                        ..Default::default()
                    },
                )
                .unwrap();
                let original = engine.run_mecab_detailed(text).unwrap();
                let registered: Vec<_> = original
                    .iter()
                    .filter(|m| m.is_from_user_dictionary())
                    .collect();
                assert!(!registered.is_empty(), "{surface} {text}");
                let result = engine.g2p_mapping_detailed(text).unwrap();
                for face in result.iter().filter(|w| w.pos_group1 == "顔文字") {
                    assert!(
                        registered
                            .iter()
                            .all(|m| m.char_span.end <= face.char_span.start
                                || face.char_span.end <= m.char_span.start)
                    );
                }
                assert!(
                    result.iter().any(|w| w.read == reading),
                    "{text} {result:?}"
                );
                assert_eq!(
                    result,
                    engine.g2p_candidates_detailed(text).unwrap().candidates[0].words
                );
            }
        }
    }
}
