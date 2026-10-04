use haqumei::{Haqumei, HaqumeiOptions, OpenJTalk};

#[test]
fn unvoicing_uses_corrected_readings_in_each_api() {
    let texts = [
        "博士課程",
        "東高洲橋",
        "そうですね",
        "結婚式々場",
        "黒﨑と博士課程",
    ];
    let mut engine = Haqumei::new().unwrap();
    for text in texts {
        let features = engine.run_frontend(text).unwrap();
        assert_eq!(
            features,
            engine.run_frontend_detailed(text).unwrap().0,
            "{text}"
        );
        assert_eq!(
            engine.g2p_mapping(text).unwrap(),
            engine.g2p_candidates(text).unwrap().candidates[0].words,
            "{text}",
        );
    }
    let expected: Vec<_> = texts
        .iter()
        .map(|t| engine.run_frontend(t).unwrap())
        .collect();
    assert_eq!(engine.run_frontend_batch(&texts).unwrap(), expected);
    assert_eq!(expected[0][0].pron, "ハク’シ");
    assert_eq!(expected[2][0].pron, "ソーデス’ネ");

    engine.options.use_read_as_pron = true;
    assert!(
        engine
            .run_frontend("博士課程")
            .unwrap()
            .iter()
            .all(|f| !f.pron.contains('’'))
    );

    let mut raw = OpenJTalk::new().unwrap();
    assert!(raw.run_frontend("です！").unwrap()[0].pron.contains('’'));
}

#[test]
fn decimal_digits_do_not_take_integer_counter_readings() {
    let mut engine = Haqumei::new().unwrap();
    for (text, expected) in [
        ("1.1本", "イッテンイチホン"),
        ("1.3本", "イッテンサンホン"),
        ("1.6本", "イッテンロクホン"),
        ("1.8個", "イッテンハチコ"),
        ("1.1人", "イッテンイチニン"),
        ("1.4日", "イッテンヨンニチ"),
        ("2.11本", "ニーテンイチイチホン"),
        ("2.1万本", "ニーテンイチマンボン"),
        ("1本", "イッポン"),
        ("3本", "サンボン"),
        ("6本", "ロッポン"),
        ("8個", "ハッコ"),
        ("4日", "ヨッカ"),
        ("米・一貫目", "コメ・イッカンメ"),
        ("5分待ち・10分待ち", "ゴフンマチ・ジュップンマチ"),
    ] {
        assert_eq!(engine.g2k(text).unwrap(), expected, "{text}");
    }
    let enumeration = engine.run_frontend("四・六級").unwrap();
    let six = enumeration.iter().find(|f| f.string == "六").unwrap();
    assert_eq!(six.pron, "ロッ");
}

#[test]
fn room_counter_is_not_changed_back_to_beya() {
    let mut engine = Haqumei::new().unwrap();
    for (text, expected) in [
        ("一部屋", "ヒトヘヤ"),
        ("二部屋", "フタヘヤ"),
        ("三部屋", "サンヘヤ"),
        ("子供部屋", "コドモベヤ"),
        ("相撲部屋", "スモーベヤ"),
    ] {
        assert_eq!(engine.g2k(text).unwrap(), expected, "{text}");
    }
    let mut without_context = Haqumei::with_options(HaqumeiOptions {
        modify_context_reading: false,
        ..Default::default()
    })
    .unwrap();
    assert_eq!(
        engine.g2k("三部屋").unwrap(),
        without_context.g2k("三部屋").unwrap()
    );
}

#[test]
fn group_quantities_and_class_numbers() {
    let mut engine = Haqumei::new().unwrap();
    for (text, kana) in [
        ("一組", "ヒトクミ"),
        ("1組", "ヒトクミ"),
        ("二組", "フタクミ"),
        ("2組", "フタクミ"),
        ("一年一組", "イチネンイチクミ"),
        ("1年1組", "イチネンイチクミ"),
        ("三年二組", "サンネンニクミ"),
        ("３年２組", "サンネンニクミ"),
        ("第一組", "ダイイチクミ"),
        ("第1組", "ダイイチクミ"),
        ("第2組", "ダイニクミ"),
        ("二年に一組", "ニネンニヒトクミ"),
        ("11組", "ジューイチクミ"),
        ("12組", "ジューニクミ"),
        ("1.1組", "イッテンイチクミ"),
        ("1.2組", "イッテンニクミ"),
    ] {
        assert_eq!(engine.g2k(text).unwrap(), kana, "{text}");
        assert_eq!(
            engine.run_frontend(text).unwrap(),
            engine.run_frontend_detailed(text).unwrap().0
        );
    }
    for text in ["一組", "1組", "二組", "2組"] {
        let features = engine.run_frontend(text).unwrap();
        assert_eq!(features[0].acc, 2, "{text}");
        assert_eq!(
            features.iter().map(|f| f.mora_size).sum::<i32>(),
            4,
            "{text}"
        );
        assert_eq!(
            engine.g2p_mapping(text).unwrap(),
            engine.g2p_candidates(text).unwrap().candidates[0].words
        );
    }
    let texts = ["1組", "二組", "三年一組", "第2組"];
    let sequential: Vec<_> = texts
        .iter()
        .map(|s| engine.run_frontend(s).unwrap())
        .collect();
    assert_eq!(engine.run_frontend_batch(&texts).unwrap(), sequential);
    engine.options.modify_numeral_reading = false;
    assert_eq!(engine.g2k("一組").unwrap(), "イチクミ");
    assert_eq!(engine.g2k("2組").unwrap(), "ニクミ");
}

#[test]
fn registered_group_reading_is_preserved() {
    let mut engine = Haqumei::new().unwrap();
    engine.set_morph_filter(|_, _, morphs| {
        for morph in morphs.iter_mut().filter(|m| m.surface == "一組") {
            morph.dictionary_index = 1;
        }
    });
    engine.options.protect_user_dict_readings = true;
    assert_eq!(engine.g2k("一組").unwrap(), "イチクミ");
    engine.options.modify_context_reading = false;
    assert_eq!(engine.g2k("一組").unwrap(), "イチクミ");
    engine.options.protect_user_dict_readings = false;
    assert_eq!(engine.g2k("一組").unwrap(), "ヒトクミ");
}
