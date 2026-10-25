use haqumei::Haqumei;

#[test]
fn table_reading_is_selected_before_particle_accents() {
    let mut engine = Haqumei::new().unwrap();
    for (text, accent) in [
        ("統計の表だけを見る。", 0),
        ("統計の表などを見る。", 3),
        ("統計の表さえを見る。", 3),
        ("統計の表くらいを見る。", 3),
    ] {
        let features = engine.run_frontend(text).unwrap();
        let table = features.iter().find(|f| f.string == "表").unwrap();
        assert_eq!(table.read, "ヒョウ");
        assert_eq!(table.pron, "ヒョー");
        assert_eq!(table.mora_size, 2);
        assert_eq!(table.acc, accent, "{text}: {features:?}");
    }
    assert!(
        engine
            .g2k("表計算を使う。")
            .unwrap()
            .starts_with("ヒョーケーサン")
    );
    assert!(
        engine
            .g2k("カードの表に印を付けた。")
            .unwrap()
            .contains("オモテ")
    );
    engine.options.modify_context_reading = false;
    assert!(engine.g2k("統計の表から読む。").unwrap().contains("オモテ"));
}

#[test]
fn weekday_ranges_do_not_read_money_or_the_moon() {
    let mut engine = Haqumei::new().unwrap();
    for (text, expected) in [
        ("月〜金", "ゲツ〜キン"),
        ("月～金", "ゲツ～キン"),
        ("火〜金", "カ〜キン"),
        ("水〜金", "スイ〜キン"),
        ("金を使う", "カネヲツカウ"),
        ("月を見る", "ツキヲミル"),
    ] {
        assert_eq!(engine.g2k(text).unwrap(), expected, "{text}");
    }
    engine.options.modify_context_reading = false;
    assert_eq!(engine.g2k("月〜金").unwrap(), "ツキ〜カネ");
}

#[test]
fn parenthetical_kamei_requires_a_person_name() {
    let mut engine = Haqumei::new().unwrap();
    assert!(
        engine
            .g2k("里山トメ子（仮名）")
            .unwrap()
            .contains("（カメー）")
    );
    assert!(
        engine
            .g2k("自動車登録番号(仮名)")
            .unwrap()
            .contains("（カナ）")
    );
    assert_eq!(engine.g2k("仮名で書く").unwrap(), "カナデカク");
}

#[test]
fn leave_verb_keeps_particle_and_inflection_nuclei() {
    let mut engine = Haqumei::new().unwrap();
    for (text, accent) in [
        ("放っておく", 0),
        ("放っといた", 4),
        ("放っておきません", 8),
        ("放っておきなさい", 8),
    ] {
        assert_eq!(engine.run_frontend(text).unwrap()[0].acc, accent, "{text}");
    }
}
