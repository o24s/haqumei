use haqumei::Haqumei;

#[test]
fn kanji_numbers_use_the_same_phrases_as_arabic_numbers() {
    let mut engine = Haqumei::new().unwrap();
    for (written, arabic) in [
        ("二十三円", "23円"),
        ("二十八時間", "28時間"),
        ("二十四時間", "24時間"),
        ("十二人ほど集まった。", "12人ほど集まった。"),
        ("二十二人で輪になった。", "22人で輪になった。"),
        ("百十二人で輪になった。", "112人で輪になった。"),
    ] {
        assert_eq!(
            engine.g2k(written).unwrap(),
            engine.g2k(arabic).unwrap(),
            "{written}"
        );
        assert_eq!(
            engine.g2p_prosody(written).unwrap(),
            engine.g2p_prosody(arabic).unwrap(),
            "{written}"
        );
    }
    for (text, expected) in [
        ("十三駅", "ジューソーエキ"),
        ("二十歳", "ハタチ"),
        ("八百屋", "ヤオヤ"),
        ("五十嵐さん", "イガラシサン"),
        ("九九を覚える", "ククヲオボエル"),
        ("唯一一人の生存者です", "ユイイツヒトリノセーゾンシャデス"),
        ("〇三−九九です", "ゼロサン−キューキューデス"),
    ] {
        assert_eq!(engine.g2k(text).unwrap(), expected, "{text}");
    }
}

#[test]
fn numeral_reanalysis_preserves_mapping_and_lattice_spans() {
    let mut engine = Haqumei::new().unwrap();
    for text in ["二十三円", "二十八時間", "唯一一人", "〇三−九九−〇〇"] {
        let mapping = engine.g2p_mapping(text).unwrap();
        let mut end = 0;
        for word in &mapping {
            if !word.char_span.is_empty() {
                assert_eq!(word.char_span.start, end, "{text}: {mapping:?}");
                end = word.char_span.end;
            }
        }
        assert_eq!(end, text.chars().count());
        let detailed = engine.g2p_mapping_detailed(text).unwrap();
        let prosody = engine.g2p_mapping_prosody(text).unwrap();
        assert_eq!(
            mapping.iter().map(|w| &w.char_span).collect::<Vec<_>>(),
            detailed.iter().map(|w| &w.char_span).collect::<Vec<_>>()
        );
        assert_eq!(
            mapping.iter().map(|w| &w.char_span).collect::<Vec<_>>(),
            prosody.iter().map(|w| &w.char_span).collect::<Vec<_>>()
        );
        let lattice = engine.analyze_lattice(text).unwrap();
        assert!(!lattice.is_empty());
        assert!(
            lattice
                .iter()
                .all(|n| n.char_span.end <= text.chars().count())
        );
    }
}

#[test]
fn a_leading_number_does_not_split_a_place_name() {
    let mut engine = Haqumei::new().unwrap();
    for text in ["2九十九里", "2九十九里浜"] {
        let morphs = engine.run_mecab_detailed(text).unwrap();
        assert!(
            morphs
                .iter()
                .any(|m| m.surface.starts_with("九十九里") && m.feature.contains("クジュークリ")),
            "{morphs:?}"
        );
    }
}

#[test]
fn old_place_characters_keep_the_number_reading() {
    let mut engine = Haqumei::new().unwrap();
    for (written, usual) in [
        ("拾万円", "十万円"),
        ("拾萬円", "十万円"),
        ("一萬円", "一万円"),
    ] {
        assert_eq!(engine.g2k(written).unwrap(), engine.g2k(usual).unwrap());
        assert_eq!(
            engine.g2p_prosody(written).unwrap(),
            engine.g2p_prosody(usual).unwrap()
        );
        let mapping = engine.g2p_mapping_detailed(written).unwrap();
        let mut end = 0;
        for word in mapping {
            if !word.char_span.is_empty() {
                assert_eq!(word.char_span.start, end, "{written}");
                end = word.char_span.end;
            }
        }
        assert_eq!(end, written.chars().count(), "{written}");
    }
}
