use haqumei::{Haqumei, HaqumeiOptions, UnicodeNormalization};
use unicode_normalization::UnicodeNormalization as _;

fn render(mut value: u16) -> String {
    let mut result = String::new();
    for (n, s) in [
        (1000, "Ⅿ"),
        (900, "ⅭⅯ"),
        (500, "Ⅾ"),
        (400, "ⅭⅮ"),
        (100, "Ⅽ"),
        (90, "ⅩⅭ"),
        (50, "Ⅼ"),
        (40, "ⅩⅬ"),
        (10, "Ⅹ"),
        (9, "ⅠⅩ"),
        (5, "Ⅴ"),
        (4, "ⅠⅤ"),
        (1, "Ⅰ"),
    ] {
        while value >= n {
            result.push_str(s);
            value -= n;
        }
    }
    result
}

fn normalized(s: &str, mode: UnicodeNormalization) -> String {
    let s = match mode {
        UnicodeNormalization::None => s.to_owned(),
        UnicodeNormalization::Nfc => s.nfc().collect(),
        UnicodeNormalization::Nfkc => s.nfkc().collect(),
    };
    haqumei_jpreprocess::normalize_text_for_open_jtalk(&s)
}

#[test]
fn roman_values_use_cardinal_readings_and_counter_rules() {
    let mut engine = Haqumei::new().unwrap();
    for n in 1..=120 {
        for suffix in [
            "", "個", "本", "人", "章", "部", "巻", "回", "月", "時", "日", "年", "分", "組",
        ] {
            if (suffix == "月" && n > 12) || (suffix == "時" && n > 24) {
                continue;
            }
            let input = format!("{}{suffix}", render(n));
            let arabic = format!("{n}{suffix}");
            assert_eq!(
                engine.g2p(&input).unwrap(),
                engine.g2p(&arabic).unwrap(),
                "{input} / {arabic}"
            );
            assert_eq!(
                engine.g2p_prosody(&input).unwrap(),
                engine.g2p_prosody(&arabic).unwrap(),
                "accent {input} / {arabic}"
            );
        }
    }
    for n in [
        121, 199, 200, 399, 400, 499, 500, 600, 800, 900, 999, 1000, 1009, 1984, 2026, 3000, 3999,
    ] {
        assert_eq!(
            engine.g2p(&render(n)).unwrap(),
            engine.g2p(&n.to_string()).unwrap(),
            "{n}"
        );
    }
}

#[test]
fn original_spans_and_all_output_paths_survive_expansion() {
    for mode in [
        UnicodeNormalization::None,
        UnicodeNormalization::Nfc,
        UnicodeNormalization::Nfkc,
    ] {
        let mut engine = Haqumei::with_options(HaqumeiOptions {
            normalize_unicode: mode,
            ..Default::default()
        })
        .unwrap();
        for input in [
            "Ⅻ",
            "番号Ⅻ",
            "第XIV章",
            "第ｘｌ巻",
            "(Ⅻ)",
            "ｶﾞⅩⅣ日後",
            "ⅩⅣ日間",
            "ⅩⅩⅣ日",
            "Ⅻ冊を読む",
            "ⅩⅣ (^_^) Ⅻ",
            "(^_^)ⅩⅣ",
            "Ⅻ・ⅩⅣ",
            "Ⅻ.ⅩⅣ",
            "Ⅻ ⅩⅣ",
            "Ⅻ-ⅩⅣ",
            "Ⅻ、第Ⅳ章",
        ] {
            let text = normalized(input, mode);
            let (features, morphs) = engine.run_frontend_detailed(input).unwrap();
            assert_eq!(features, engine.run_frontend(input).unwrap(), "{input}");
            let map = engine.g2p_mapping_detailed(input).unwrap();
            let mut end = 0;
            for w in &map {
                assert!(w.char_span.start >= end, "{input}: {map:?}");
                assert_eq!(
                    w.word,
                    text.chars()
                        .skip(w.char_span.start)
                        .take(w.char_span.len())
                        .collect::<String>(),
                    "{input}: {w:?} / {features:?} / {morphs:?}"
                );
                end = w.char_span.end;
            }
            assert_eq!(
                map.iter().map(|w| w.word.as_str()).collect::<String>(),
                text,
                "{input}: {map:?}"
            );
            assert_eq!(
                map,
                engine.g2p_candidates_detailed(input).unwrap().candidates[0].words,
                "{input}"
            );
            assert_eq!(
                engine.g2p_mapping_prosody(input).unwrap(),
                engine.g2p_candidates_prosody(input).unwrap().candidates[0].words,
                "{input}"
            );
        }
        assert_eq!(engine.g2k("番号Ⅻ").unwrap(), "バンゴージューニ");
        assert_eq!(engine.g2k("Ⅰ・Ⅱ").unwrap(), "イチ・ニ");
    }
}

#[test]
fn english_invalid_numerals_and_overlines_are_untouched() {
    for mode in [
        UnicodeNormalization::None,
        UnicodeNormalization::Nfc,
        UnicodeNormalization::Nfkc,
    ] {
        let mut engine = Haqumei::with_options(HaqumeiOptions {
            normalize_unicode: mode,
            ..Default::default()
        })
        .unwrap();
        for input in [
            "CD",
            "MIX",
            "cm",
            "ビタミンC",
            "Mサイズ",
            "C型肝炎",
            "MC黄猿",
            "X100V",
            "CIVIL",
            "LIVE",
            "第MIXER章",
            "第IIX章",
            "第ⅩIV章",
            "Ⅳ4",
            "4Ⅳ",
            "Ⅳ\u{0305}",
            "Ⅹ_Ⅳ",
            "fooⅣbar",
            "一Ⅻ",
            "Ⅻ三",
            "ↁ",
            "ⅠⅠⅠⅠ",
            "ⅯⅯⅯⅯ",
        ] {
            engine.options.resolve_roman_numerals = true;
            let actual = engine.run_frontend_detailed(input).unwrap();
            engine.options.resolve_roman_numerals = false;
            assert_eq!(
                actual,
                engine.run_frontend_detailed(input).unwrap(),
                "{input}"
            );
        }
    }
}

#[test]
fn filters_override_roman_reading_and_batch_matches_single() {
    let mut engine = Haqumei::new().unwrap();
    engine.set_morph_filter(|_, _, morphs| {
        for m in morphs {
            if m.surface == "Ⅻ" {
                m.feature = "Ⅻ,名詞,一般,*,*,*,*,Ⅻ,テスト,テスト,1/3,C1".into();
            }
        }
    });
    assert_eq!(engine.g2k("Ⅻ").unwrap(), "テスト");
    assert_eq!(
        engine.g2p_mapping("Ⅻ").unwrap(),
        engine.g2p_candidates("Ⅻ").unwrap().candidates[0].words
    );
    engine.set_morph_filter(|_, _, _| {});
    assert_eq!(engine.g2k("(^_^)Ⅻ").unwrap(), "ジューニ");
    engine.clear_morph_filter();
    let texts = ["Ⅻ", "第XIV章", "ⅩⅣ日間", "MIX"];
    let expected: Vec<_> = texts.iter().map(|s| engine.g2k(s).unwrap()).collect();
    assert_eq!(engine.g2k_batch(&texts).unwrap(), expected);
    engine.options.resolve_roman_numerals = false;
    let expected: Vec<_> = texts.iter().map(|s| engine.g2k(s).unwrap()).collect();
    assert_eq!(engine.g2k_batch(&texts).unwrap(), expected);
}

#[test]
fn explicitly_edited_counters_are_not_restored() {
    let mut engine = Haqumei::new().unwrap();
    engine.set_morph_filter(|_, _, morphs| {
        for m in morphs {
            if matches!(
                m.surface.as_str(),
                "月" | "巻" | "章" | "世" | "日" | "日間" | "人"
            ) {
                m.feature = format!(
                    "{},名詞,一般,*,*,*,*,{},テスト,テスト,1/3,C1",
                    m.surface, m.surface
                );
            }
        }
    });
    for text in [
        "Ⅻ巻",
        "Ⅻ 月",
        "Ⅹ章",
        "第IV章",
        "Ⅹ世",
        "Ⅳ日",
        "Ⅰ人",
        "ⅩⅣ日間",
        "ⅩⅩⅣ日",
    ] {
        assert!(engine.g2k(text).unwrap().contains("テスト"), "{text}");
        assert_eq!(
            engine.g2p_mapping_detailed(text).unwrap(),
            engine.g2p_candidates_detailed(text).unwrap().candidates[0].words
        );
    }
}

#[test]
fn unicode_normalization_does_not_change_numeral_separators() {
    let mut engine = Haqumei::new().unwrap();
    for input in ["Ⅰ-Ⅱ", "Ⅰ-Ⅹ", "Ⅴ‐Ⅲ", "Ⅰ・Ⅱ", "Ⅴ.Ⅲ", "Ⅰ X", "Ⅻ (^_^) Ⅱ"]
    {
        engine.options.normalize_unicode = UnicodeNormalization::None;
        let before = engine.g2p_prosody(input).unwrap();
        engine.options.normalize_unicode = UnicodeNormalization::Nfkc;
        assert_eq!(before, engine.g2p_prosody(input).unwrap(), "{input}");
    }
}
