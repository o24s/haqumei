use haqumei::{Haqumei, HaqumeiOptions};

fn assert_option_changes<T: std::fmt::Debug + PartialEq>(
    analyze: impl Fn(&mut Haqumei, &str) -> Result<T, haqumei::errors::HaqumeiError>,
) {
    let mut reference = Haqumei::new().unwrap();
    let enabled = analyze(&mut reference, "𠮷野家").unwrap();
    reference.options.resolve_kanji_variants = false;
    let disabled = analyze(&mut reference, "𠮷野家").unwrap();
    assert_ne!(enabled, disabled);

    for with_filter in [false, true] {
        let mut engine = Haqumei::new().unwrap();
        if with_filter {
            engine.set_morph_filter(|_, _, _| {});
        }
        for resolve in [false, true, false, true] {
            engine.options.resolve_kanji_variants = resolve;
            assert_eq!(
                &analyze(&mut engine, "𠮷野家").unwrap(),
                if resolve { &enabled } else { &disabled },
                "resolve={resolve}, filter={with_filter}",
            );
            analyze(&mut engine, "").unwrap();
        }
    }
}

#[test]
fn option_changes_apply_to_each_analysis_entry_point() {
    assert_option_changes(Haqumei::run_frontend);
    assert_option_changes(Haqumei::run_frontend_detailed);
    assert_option_changes(Haqumei::run_mecab_detailed);
    assert_option_changes(Haqumei::analyze_lattice);
    assert_option_changes(Haqumei::g2p_candidates);
    assert_option_changes(Haqumei::g2p_candidates_detailed);
    assert_option_changes(Haqumei::g2p_candidates_prosody);
    assert_option_changes(|engine, text| engine.run_frontend_batch(&[text]));
    assert_option_changes(|engine, text| engine.g2p_candidates_batch(&[text]));
}

#[test]
fn recovers_words_without_changing_the_input_spelling() {
    let mut engine = Haqumei::new().unwrap();
    for (text, kana) in [
        ("𠮷野家", "ヨシノヤ"),
        ("黒﨑", "クロサキ"),
        ("高﨑駅", "タカサキエキ"),
        ("濵田", "ハマダ"),
        ("𠮷野家と齒性", "ヨシノヤトシセー"),
    ] {
        assert_eq!(engine.g2k(text).unwrap(), kana, "{text}");
        let morphs = engine.run_mecab_detailed(text).unwrap();
        let nodes = engine.analyze_lattice(text).unwrap();
        let chars: Vec<_> = text.chars().collect();
        for morph in &morphs {
            assert_eq!(
                chars[morph.char_span.clone()].iter().collect::<String>(),
                morph.surface
            );
            assert!(nodes.iter().any(|node| node.is_best
                && node.char_span == morph.char_span
                && format!("{},{}", node.surface, node.feature) == morph.feature));
        }
        for node in &nodes {
            assert_eq!(
                chars[node.char_span.clone()].iter().collect::<String>(),
                node.surface
            );
            assert!(node.delta >= 0);
        }
        let mapping = engine.g2p_mapping(text).unwrap();
        assert_eq!(
            mapping
                .iter()
                .map(|word| word.word.as_str())
                .collect::<String>(),
            text
        );
        assert_eq!(
            mapping,
            engine.g2p_candidates(text).unwrap().candidates[0].words
        );
        assert_eq!(
            engine.run_frontend(text).unwrap(),
            engine.run_frontend_detailed(text).unwrap().0
        );
    }
}

#[test]
fn single_character_readings_and_known_words_are_preserved() {
    let mut engine = Haqumei::new().unwrap();
    for text in [
        "齒性",
        "殼長",
        "郭懷一",
        "卽離",
        "神奈川",
        "髙島屋",
        "土と士、大と太",
        "ｶﾞムと歯",
    ] {
        engine.options.resolve_kanji_variants = false;
        let expected = engine.run_frontend_detailed(text).unwrap();
        let lattice = engine.analyze_lattice(text).unwrap();
        engine.options.resolve_kanji_variants = true;
        assert_eq!(
            expected,
            engine.run_frontend_detailed(text).unwrap(),
            "{text}"
        );
        assert_eq!(lattice, engine.analyze_lattice(text).unwrap(), "{text}");
    }
}

#[test]
fn options_batch_mapping_and_candidates_agree() {
    let texts = [
        "𠮷野家に行く",
        "黒﨑と高﨑駅",
        "𠮷野家と齒性",
        "𠮷野家で𠮷野家",
    ];
    for enabled in [false, true] {
        let mut engine = Haqumei::with_options(HaqumeiOptions {
            resolve_kanji_variants: enabled,
            ..Default::default()
        })
        .unwrap();
        let sequential: Vec<_> = texts
            .iter()
            .map(|text| engine.run_frontend(text).unwrap())
            .collect();
        assert_eq!(sequential, engine.run_frontend_batch(&texts).unwrap());
        for text in texts {
            assert_eq!(
                engine.g2p_mapping_detailed(text).unwrap(),
                engine.g2p_candidates_detailed(text).unwrap().candidates[0].words
            );
            assert_eq!(
                engine.g2p_mapping_prosody(text).unwrap(),
                engine.g2p_candidates_prosody(text).unwrap().candidates[0].words
            );
        }
    }
}
