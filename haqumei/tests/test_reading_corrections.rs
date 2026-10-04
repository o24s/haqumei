use haqumei::{Haqumei, HaqumeiOptions, OpenJTalk};

#[test]
fn numeric_boundaries_and_limited_identifiers() {
    let mut engine = Haqumei::new().unwrap();
    for (text, kana) in [
        ("119に電話する", "イチイチキューニデンワスル"),
        ("110に電話した", "イチイチゼロニデンワシタ"),
        ("119にかける", "イチイチキューニカケル"),
        ("119に掛けた", "イチイチキューニカケタ"),
        ("100に電話料金を足す", "ヒャクニデンワリョーキンヲタス"),
        ("電話は100ある", "デンワワヒャクアル"),
        (
            "119に電話連絡する",
            "ヒャクジューキューニデンワレンラクスル",
        ),
        ("〒1048011", "〒イチゼロヨンハチゼロイチイチ"),
        ("〒104・8011", "〒イチゼロヨン・ハチゼロイチイチ"),
        ("〒〇七〇-〇〇〇〇", "〒ゼロナナゼロ−ゼロゼロゼロゼロ"),
        (
            "070-〇〇〇〇-〇〇〇〇",
            "ゼロナナゼロ−ゼロゼロゼロゼロ−ゼロゼロゼロゼロ",
        ),
        ("1 0", "イチゼロ"),
        ("1　　0", "イチゼロ"),
        ("1 234円", "イチニヒャクサンジューヨエン"),
        (
            "〇七〇-〇〇二四-五六七九",
            "ゼロナナゼロ−ゼロゼロニーヨン−ゴーロクナナキュー",
        ),
        ("〇〇町", "マルマルマチ"),
        ("〇〇人", "マルマルニン"),
        ("〇〇年", "マルマルネン"),
        ("〇〇円", "マルマルエン"),
        ("〇〇", "マルマル"),
        ("〇円", "レーエン"),
        ("二〇〇〇年", "ニセンネン"),
    ] {
        assert_eq!(engine.g2k(text).unwrap(), kana, "{text}");
        assert_eq!(
            engine.run_frontend(text).unwrap(),
            engine.run_frontend_detailed(text).unwrap().0,
            "{text}"
        );
        assert_eq!(
            engine.g2p_mapping(text).unwrap(),
            engine.g2p_candidates(text).unwrap().candidates[0].words,
            "{text}"
        );
    }
    let separated = engine.run_frontend("EF65 1032号機").unwrap();
    assert!(!separated.iter().any(|node| node.string == "万"));
    assert!(
        !engine
            .g2p("65 1032")
            .unwrap()
            .iter()
            .any(|p| p.as_str() == "pau")
    );
    let mut raw = OpenJTalk::new().unwrap();
    assert_eq!(
        raw.run_frontend("65 1032").unwrap(),
        raw.run_frontend_detailed("65 1032").unwrap().0
    );
    let mecab = raw.run_mecab("65 1032").unwrap();
    let njd = raw.run_njd_from_mecab(&mecab).unwrap();
    assert_eq!(raw.run_frontend("65 1032").unwrap(), njd);
    assert_eq!(
        raw.g2p("65 1032").unwrap(),
        raw.extract_phonemes(&njd).unwrap()
    );
    assert_eq!(
        raw.g2p("65 1032").unwrap(),
        raw.g2p_mapping("65 1032")
            .unwrap()
            .into_iter()
            .filter(|word| !word.is_ignored)
            .flat_map(|word| word.phonemes)
            .collect::<Vec<_>>()
    );
    let expanded = raw
        .run_njd_from_mecab(["〇８０−００００,名詞,数,*,*,*,*,*,*,*,*,*,*"])
        .unwrap();
    assert!(expanded.iter().all(|f| f.read != "*"));
    assert_eq!(expanded.iter().filter(|f| f.read == "ゼロ").count(), 6);
    assert!(!expanded.iter().any(|f| f.pron.contains("キュー")));

    for text in ["〒10480112", "1048011", "1119に電話する"] {
        assert!(
            raw.run_frontend(text)
                .unwrap()
                .iter()
                .any(|f| { matches!(f.string.as_str(), "十" | "百" | "千" | "万") }),
            "{text}"
        );
    }
    assert!(
        raw.run_frontend("3.119に電話する")
            .unwrap()
            .iter()
            .any(|f| f.read == "テン")
    );

    for text in [
        "〒〇七〇-〇〇〇〇",
        "070-〇〇〇〇-〇〇〇〇",
        "〇七〇-〇〇二四-五六七九",
    ] {
        let words = engine.g2p_mapping(text).unwrap();
        assert_eq!(words.len(), text.chars().count(), "{text}");
        for (index, word) in words.iter().enumerate() {
            assert_eq!(word.char_span, index..index + 1, "{text}: {}", word.word);
        }
    }
}

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

#[test]
fn separated_counters_match_contiguous_readings_and_accents() {
    let mut engine = Haqumei::new().unwrap();
    for (spaced, joined) in [
        ("2024 年", "2024年"),
        ("3 人", "3人"),
        ("2 人", "2人"),
        ("3 本", "3本"),
        ("10 時", "10時"),
        ("3 分", "3分"),
        ("6 匹", "6匹"),
        ("1.5 日", "1.5日"),
        ("0.2 人", "0.2人"),
        ("1 年 1 組", "1年1組"),
    ] {
        assert_eq!(
            engine.g2k(spaced).unwrap(),
            engine.g2k(joined).unwrap(),
            "{spaced}"
        );
        assert_eq!(
            engine.extract_fullcontext(spaced).unwrap(),
            engine.extract_fullcontext(joined).unwrap(),
            "{spaced}"
        );
    }
    for (text, word, pron) in [
        ("3 分の1", "分", "ブン"),
        ("場面6 人前で話す", "人", "ヒト"),
        ("２ 人づくりの基盤", "人", "ヒト"),
        ("1 月ほど待った", "月", "ツキ"),
        ("一 月が過ぎた", "月", "ツキ"),
    ] {
        let features = engine.run_frontend(text).unwrap();
        assert_eq!(
            features
                .iter()
                .find(|f| f.string == word)
                .unwrap()
                .pron
                .replace('’', ""),
            pron,
            "{text}"
        );
    }
}

#[test]
fn separated_counter_preserves_protected_noun_reading() {
    let mut engine = Haqumei::new().unwrap();
    engine.set_morph_filter(|_, _, morphs| {
        for morph in morphs.iter_mut().filter(|m| m.surface == "人") {
            morph.dictionary_index = 1;
        }
    });
    engine.options.protect_user_dict_readings = true;
    for numeral in [true, false] {
        engine.options.modify_numeral_reading = numeral;
        assert_eq!(engine.g2k("2 人").unwrap(), "ニヒト");
    }
    engine.options.protect_user_dict_readings = false;
    assert_eq!(engine.g2k("2 人").unwrap(), "フタリ");
}

#[test]
fn comma_grouped_numbers_match_plain_numbers() {
    let mut engine = Haqumei::new().unwrap();
    let texts = [
        "1,050円",
        "12,005人",
        "1,234,567",
        "1,050.5円",
        "1,050円と12,005円",
    ];
    for text in texts {
        let plain = text.replace(',', "");
        assert_eq!(
            engine.g2k(text).unwrap(),
            engine.g2k(&plain).unwrap(),
            "{text}"
        );
        assert_eq!(
            engine.extract_fullcontext(text).unwrap(),
            engine.extract_fullcontext(&plain).unwrap(),
            "{text}"
        );
        assert_eq!(
            engine.g2p_mapping(text).unwrap(),
            engine.g2p_candidates(text).unwrap().candidates[0].words,
            "{text}"
        );
        assert_eq!(
            engine.run_frontend(text).unwrap(),
            engine.run_frontend_detailed(text).unwrap().0,
            "{text}"
        );
    }
    let expected: Vec<_> = texts.iter().map(|t| engine.g2k(t).unwrap()).collect();
    assert_eq!(engine.g2k_batch(&texts).unwrap(), expected);
    for text in [
        "1,2",
        "01,234円",
        "1,2345円",
        "1,,234",
        "1,234,56",
        "1,000，円",
    ] {
        assert!(engine.g2k(text).unwrap().contains('，'), "{text}");
    }
    let huge = format!("1{}円と2,345円", ",000".repeat(25));
    assert!(
        engine
            .g2k(&huge)
            .unwrap()
            .ends_with("ニセンサンビャクヨンジューゴエン")
    );
}
