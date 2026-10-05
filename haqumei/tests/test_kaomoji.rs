use haqumei::{Haqumei, HaqumeiOptions, OpenJTalk, Phoneme, UnicodeNormalization};
use unicode_normalization::UnicodeNormalization as _;

fn normalized(text: &str, mode: UnicodeNormalization) -> String {
    let unicode = match mode {
        UnicodeNormalization::None => text.to_owned(),
        UnicodeNormalization::Nfc => text.nfc().collect(),
        UnicodeNormalization::Nfkc => text.nfkc().collect(),
    };
    haqumei_jpreprocess::normalize_text_for_open_jtalk(&unicode)
}

#[test]
fn faces_keep_their_surface_with_empty_reading_and_phonemes() {
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
        for face in [
            "(^_^)",
            "(-'ω')",
            "(-ノ_ノ)",
            "(T_T)",
            "(0_0)",
            "(⊃-⊂)",
            "(ﾉ)ω(ヾ)",
            "(*>ω<)ω<*)",
            "(^ω^)人(^ω^)",
            "(^o^)人(^o^)",
            "(T_T)人(T_T)",
            "(0_0)人(0_0)",
            "(^ω^人^ω^)",
            "(^ω^)σ)ω^)",
            "(^ω^)っ)ω^)",
            "(^ω^)σ\")ω^)",
            "(^ω^)σ”)ω^)",
            "(ﾉｼ｀･ω･)",
            "(^ω^)ω^)ω^)",
            "(๑′ฅฅ‵๑)",
            "(´∩′•ω•∩)",
            "(^_^)つー○○○",
            "(^_^)つ☕️",
        ] {
            let expected = normalized(face, mode);
            let (features, morphs) = engine.run_frontend_detailed(face).unwrap();
            assert_eq!(features.len(), 1, "{face} {mode:?}: {features:?}");
            assert_eq!(features[0].string, expected, "{face} {mode:?}");
            assert_eq!(features[0].pos_group1, "顔文字", "{face} {mode:?}");
            assert!(
                features[0].read.is_empty() && features[0].pron.is_empty(),
                "{face}: {features:?}"
            );
            assert_eq!((features[0].mora_size, features[0].acc), (0, 0));
            assert_eq!(morphs.len(), 1);
            assert!(!morphs[0].is_unknown);
            assert_eq!(engine.g2k(face).unwrap(), "");
            assert_eq!(engine.g2k_per_word(face).unwrap(), [""]);
            assert!(engine.g2p(face).unwrap().is_empty());
            assert!(engine.extract_fullcontext(face).unwrap().is_empty());
            let map = engine.g2p_mapping_detailed(face).unwrap();
            assert_eq!(map.len(), 1);
            assert_eq!(map[0].word, expected);
            assert_eq!(map[0].char_span, 0..expected.chars().count());
            assert!(map[0].phonemes.is_empty() && map[0].read.is_empty() && map[0].pron.is_empty());
            assert!(map[0].is_ignored && !map[0].is_unknown);
            assert_eq!(
                map,
                engine.g2p_candidates_detailed(face).unwrap().candidates[0].words
            );
        }
    }
}

#[test]
fn halfwidth_shi_before_a_face_belongs_to_the_preceding_word() {
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
        for (prefix, face) in [
            ("仲良ｼ", "(^_^)"),
            ("仲良ｼ", "(^o^)人(^o^)"),
            ("仲良ｼ", "(^ω^)っ)ω^)"),
        ] {
            let map = engine
                .g2p_mapping_detailed(&format!("{prefix}{face}"))
                .unwrap();
            let faces: Vec<_> = map.iter().filter(|w| w.pos_group1 == "顔文字").collect();
            assert_eq!(faces.len(), 1, "{map:?}");
            assert_eq!(faces[0].word, normalized(face, mode));
            assert_eq!(
                faces[0].char_span.start,
                normalized(prefix, mode).chars().count()
            );
            assert!(map.iter().any(|w| w.pron.contains('シ')));
        }
    }
}

fn audible(phones: impl IntoIterator<Item = Phoneme>) -> Vec<Phoneme> {
    phones
        .into_iter()
        .filter(|p| !matches!(p, Phoneme::Sp | Phoneme::Pau))
        .collect()
}

#[test]
fn output_paths_agree_across_faces_and_long_vowels() {
    let mut engine = Haqumei::new().unwrap();
    let mut raw = OpenJTalk::new().unwrap();
    for text in [
        "コ(^_^)ーヒー",
        "コー(^_^)ーヒー",
        "前(^_^)ー後",
        "前(^_^)ーー後",
        "(^_^)こんにちは",
        "こんにちは(^_^)世界",
        "1(^_^)2人",
        "です(^_^)！？",
        "前(^_^)。後",
        "前(^_^) (T_T)後",
    ] {
        let frontend = engine.run_frontend(text).unwrap();
        assert_eq!(
            frontend,
            engine.run_frontend_detailed(text).unwrap().0,
            "{text}"
        );
        let phones = audible(engine.g2p(text).unwrap());
        assert_eq!(
            phones,
            audible(raw.extract_phonemes(&frontend).unwrap()),
            "frontend {text}"
        );
        let map = engine.g2p_mapping(text).unwrap();
        assert_eq!(
            phones,
            audible(map.iter().flat_map(|w| w.phonemes.clone())),
            "mapping {text}: {map:?}"
        );
        let detail = engine.g2p_mapping_detailed(text).unwrap();
        assert_eq!(
            phones,
            audible(detail.iter().flat_map(|w| w.phonemes.clone())),
            "detail {text}"
        );
        let labels = engine.extract_fullcontext(text).unwrap();
        let labelled = labels
            .iter()
            .filter_map(|l| l.phoneme.c.as_deref())
            .filter(|&p| p != "sil")
            .map(|p| p.parse().unwrap());
        assert_eq!(phones, audible(labelled), "labels {text}");
        assert_eq!(
            map,
            engine.g2p_candidates(text).unwrap().candidates[0].words,
            "candidate {text}"
        );
        let prosody = engine.g2p_mapping_prosody(text).unwrap();
        assert_eq!(
            prosody,
            engine.g2p_candidates_prosody(text).unwrap().candidates[0].words,
            "prosody {text}"
        );
        for w in &detail {
            if w.pos_group1 == "顔文字" {
                assert!(
                    w.phonemes.is_empty() && w.read.is_empty() && w.pron.is_empty(),
                    "{w:?}"
                );
            }
        }
    }
    assert_eq!(
        engine
            .g2p("コ(^_^)ーヒー")
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["k", "o", "o", "h", "i", "i"]
    );
    assert_eq!(
        engine
            .g2p("コー(^_^)ーヒー")
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["k", "o", "o", "o", "h", "i", "i"]
    );
}

#[test]
fn non_faces_and_words_next_to_arms_are_preserved() {
    let mut engine = Haqumei::new().unwrap();
    for text in [
        "(p-q)",
        "(a+b)",
        "(2026年)",
        "(ﾉ)日本語(ヾ)",
        "(^ω^)3人(^ω^)",
        "(pH)",
        "(NaCl)",
    ] {
        let result = engine.run_frontend_detailed(text).unwrap();
        if !text.contains("^ω^") {
            engine.options.ignore_kaomoji = false;
            assert_eq!(
                result,
                engine.run_frontend_detailed(text).unwrap(),
                "{text}"
            );
            engine.options.ignore_kaomoji = true;
        } else {
            assert!(engine.g2k(text).unwrap().contains("サンニン"));
        }
    }
    for text in [
        "pc(^_^)です",
        "(^_^)ノート",
        "(^_^)つー○○○円",
        "(^_^)つー☆5個",
    ] {
        let (features, _) = engine.run_frontend_detailed(text).unwrap();
        assert!(
            features.iter().any(|f| f.pos_group1 == "顔文字"),
            "{text}: {features:?}"
        );
        let face = features.iter().find(|f| f.pos_group1 == "顔文字").unwrap();
        assert!(
            !face.string.contains("ｐｃ")
                && !face.string.contains("ノート")
                && !face.string.contains('円')
                && !face.string.contains('５')
        );
    }
}

#[test]
fn normalization_preserves_positions_and_grapheme_boundaries() {
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
        for text in [
            "か\u{3099}(^_^)後",
            "㈱(^_^)後",
            "ｶﾞ(^_^)後",
            "\t前(^_^)\n後",
            "前(^_^)\u{0301}後",
            "「_(┐「ε:)_」",
        ] {
            let normalized = normalized(text, mode);
            let chars: Vec<_> = normalized.chars().collect();
            let map = engine.g2p_mapping_detailed(text).unwrap();
            assert_eq!(
                map.iter().map(|w| w.word.as_str()).collect::<String>(),
                normalized,
                "{text} {mode:?}"
            );
            for word in &map {
                assert_eq!(
                    chars[word.char_span.clone()].iter().collect::<String>(),
                    word.word,
                    "{text} {word:?}"
                );
            }
            assert_eq!(
                map,
                engine.g2p_candidates_detailed(text).unwrap().candidates[0].words
            );
        }
    }
}

#[test]
fn option_and_batch_use_the_same_processing() {
    let mut engine = Haqumei::new().unwrap();
    let text = "前(-ノ_ノ)後";
    let enabled = engine.run_frontend(text).unwrap();
    engine.options.ignore_kaomoji = false;
    let disabled = engine.run_frontend(text).unwrap();
    assert_ne!(enabled, disabled);
    for enabled_flag in [true, false, true] {
        engine.options.ignore_kaomoji = enabled_flag;
        assert_eq!(
            engine.run_frontend_batch(&[text]).unwrap(),
            [if enabled_flag {
                enabled.clone()
            } else {
                disabled.clone()
            }]
        );
        assert_eq!(
            engine.g2p_mapping_batch(&[text]).unwrap(),
            [engine.g2p_mapping(text).unwrap()]
        );
        assert_eq!(
            engine.g2p_candidates_batch(&[text]).unwrap()[0].candidates[0].words,
            engine.g2p_mapping(text).unwrap()
        );
    }
}

#[test]
fn unknown_morph_crossing_the_face_keeps_its_original_analysis() {
    let mut engine = Haqumei::new().unwrap();
    let text = "一(^_^)々";
    let before = engine.run_frontend_detailed(text).unwrap();
    assert!(
        before.0.iter().all(|f| f.pos_group1 != "顔文字"),
        "{before:?}"
    );
    engine.options.ignore_kaomoji = false;
    assert_eq!(before, engine.run_frontend_detailed(text).unwrap());
}

#[test]
fn explicit_morph_edits_are_not_silenced_or_replaced_by_candidates() {
    let mut engine = Haqumei::new().unwrap();
    engine.set_morph_filter(|_, _, morphs| {
        for m in morphs {
            if m.surface == "ω" {
                m.feature = format!(
                    "{},名詞,一般,*,*,*,*,{},ワン,ワン,0/2,C3",
                    m.surface, m.surface
                );
            }
        }
    });
    for text in ["前(・ω・)後", "ω"] {
        let result = engine.g2p_mapping_detailed(text).unwrap();
        assert!(result.iter().any(|w| w.read == "ワン"));
        assert_eq!(
            result,
            engine.g2p_candidates_detailed(text).unwrap().candidates[0].words
        );
        assert_eq!(
            engine.run_frontend(text).unwrap(),
            engine.run_frontend_detailed(text).unwrap().0
        );
    }
}

#[test]
fn alternatives_keep_faces_silent_and_outside_reading_branches() {
    let mut engine = Haqumei::new().unwrap();
    let text = "上手(^_^)今日は何も聞いていない。";
    let result = engine
        .g2p_candidates_detailed_with_options(
            text,
            haqumei::CandidateOptions {
                max_delta: 10000,
                max_candidates: 16,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(result.candidates.len() > 1);
    for candidate in &result.candidates {
        let faces: Vec<_> = candidate
            .words
            .iter()
            .filter(|w| w.pos_group1 == "顔文字")
            .collect();
        assert_eq!(faces.len(), 1);
        let face = faces[0];
        assert_eq!(face.word, "（＾＿＾）");
        assert!(face.phonemes.is_empty() && face.read.is_empty() && face.pron.is_empty());
        assert!(
            result
                .branches
                .iter()
                .all(|b| b.char_span.end <= face.char_span.start
                    || face.char_span.end <= b.char_span.start)
        );
    }
}

#[test]
fn allophones_and_reading_options_keep_mapping_consistent() {
    for options in [
        HaqumeiOptions {
            use_allophones: true,
            ..Default::default()
        },
        HaqumeiOptions {
            use_read_as_pron: true,
            ..Default::default()
        },
        HaqumeiOptions {
            revert_long_vowels: true,
            revert_yotsugana: true,
            ..Default::default()
        },
    ] {
        let mut engine = Haqumei::with_options(options).unwrap();
        for text in [
            "三(^_^)軒",
            "一冊(^_^)配った",
            "コ(^_^)ーヒー",
            "(^_^)発車します",
        ] {
            let phones = audible(engine.g2p(text).unwrap());
            let mapping = engine.g2p_mapping_detailed(text).unwrap();
            assert_eq!(
                phones,
                audible(mapping.iter().flat_map(|w| w.phonemes.clone())),
                "{text}"
            );
            assert_eq!(
                mapping,
                engine.g2p_candidates_detailed(text).unwrap().candidates[0].words
            );
            assert!(
                mapping
                    .iter()
                    .filter(|w| w.pos_group1 == "顔文字")
                    .all(|w| w.phonemes.is_empty() && w.read.is_empty() && w.pron.is_empty())
            );
        }
    }
}
