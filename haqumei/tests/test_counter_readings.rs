use haqumei::Haqumei;

#[test]
fn counters_distinguish_quantities_suffixes_and_ordinals() {
    let mut engine = Haqumei::new().unwrap();
    for (text, expected) in [
        ("一試合", "イッシアイ"),
        ("第一試合", "ダイイチシアイ"),
        ("一人", "ヒトリ"),
        ("一人前", "イチニンマエ"),
        ("七分袖", "シチブソデ"),
        ("七分", "ナナフン"),
        ("三階級", "サンカイキュー"),
        ("三本", "サンボン"),
        ("八課", "ハチカ"),
        ("六課", "ロッカ"),
        ("一箱", "ヒトハコ"),
        ("六箱", "ロッパコ"),
        ("三柱", "ミハシラ"),
        ("七夜", "シチヤ"),
        ("七年生", "ナナネンセー"),
        ("七アンダー", "セブンアンダー"),
        ("十七アンダー", "ジューナナアンダー"),
        ("3石", "サンゼキ"),
        ("8石", "ハッセキ"),
    ] {
        assert_eq!(engine.g2k(text).unwrap(), expected, "{text}");
    }
}

#[test]
fn teens_join_within_the_verified_range() {
    let mut engine = Haqumei::new().unwrap();
    for text in ["十一人", "十二個", "十五階"] {
        let nodes = engine.run_frontend(text).unwrap();
        assert_eq!(nodes[0].string, "十", "{text}");
        assert_eq!(nodes[1].chain_flag, 1, "{text}");
    }
    for text in ["十四時", "十五分", "十九時", "十五万", "十五世"] {
        let nodes = engine.run_frontend(text).unwrap();
        assert_eq!(nodes[0].string, "十", "{text}");
        assert_eq!(nodes[0].acc, 1, "{text}");
        assert_eq!(nodes[1].chain_flag, 0, "{text}");
    }
    let date = engine.run_frontend("十四日").unwrap();
    assert_eq!((&*date[0].string, date[0].acc), ("十", 1));
    assert_eq!((&*date[1].string, date[1].chain_flag), ("四日", 0));
    let ordinal = engine.run_frontend("十四日目").unwrap();
    assert_eq!((&*ordinal[0].string, ordinal[0].acc), ("十四日", 6));
    assert_eq!((&*ordinal[1].string, ordinal[1].chain_flag), ("目", 1));
}
