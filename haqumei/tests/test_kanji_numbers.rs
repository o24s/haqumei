use haqumei::Haqumei;

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
