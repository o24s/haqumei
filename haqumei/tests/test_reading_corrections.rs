use haqumei::{Haqumei, HaqumeiOptions, OpenJTalk};

fn check_context_readings(cases: &[(&str, &str, &str, i32)], preserved: &[&str]) {
    let mut engine = Haqumei::new().unwrap();
    for &(text, word, reading, moras) in cases {
        let features = engine.run_frontend(text).unwrap();
        let feature = features.iter().find(|f| f.string == word).unwrap();
        assert_eq!(feature.read, reading, "{text}");
        assert_eq!(feature.mora_size, moras, "{text}");
    }
    for &text in preserved {
        let with_rules = engine.g2k(text).unwrap();
        engine.options.modify_context_reading = false;
        let without_rules = engine.g2k(text).unwrap();
        engine.options.modify_context_reading = true;
        assert_eq!(with_rules, without_rules, "{text}");
    }
}

#[test]
fn conventional_phrases_select_readings_before_chaining() {
    check_context_readings(
        &[
            ("神の代から続く", "代", "ヨ", 1),
            ("崖の際に立つ", "際", "キワ", 2),
            ("淵の際まで進む", "際", "キワ", 2),
            ("事の本を正す", "本", "モト", 2),
            ("汚れを祓う", "汚れ", "ケガレ", 3),
            ("盲・聾・養護学校", "盲", "モウ", 2),
            ("盲・聾・養護学校", "聾", "ロウ", 2),
        ],
        &[
            "神の代理を務める",
            "その際に読む",
            "洪水の際に逃げる",
            "黒白の写真を撮る",
            "黒白を争う",
            "本を読む",
            "紅色細菌を調べる",
            "紅色の着物を着る",
            "町の聾学校",
            "水の面では問題がある",
            "水の面に月が映る",
            "靴の汚れを清める",
        ],
    );
}

#[test]
fn religious_terms_need_direct_context() {
    check_context_readings(
        &[
            ("天台声明を唱える", "声明", "ショウミョウ", 4),
            ("真言声明が唱えられる", "声明", "ショウミョウ", 4),
            ("この社は神を祀る", "社", "ヤシロ", 3),
            ("在家信者が集まる", "在家", "ザイケ", 3),
        ],
        &[
            "法要の声明を読んだ",
            "声明が唱える理念",
            "声明を唱える",
            "この社は社員を募集する",
            "在家さんが来る",
        ],
    );
}

#[test]
fn cloth_readings_keep_the_place_name() {
    check_context_readings(
        &[
            ("生成りの麻布を買う", "麻布", "アサヌノ", 4),
            ("麻布を織る", "麻布", "アサヌノ", 4),
        ],
        &["麻布に染色の店がある", "麻布を染める"],
    );
}

#[test]
fn idiomatic_face_and_sound_need_local_causes() {
    check_context_readings(
        &[
            ("御前で面を上げよ", "面", "オモテ", 3),
            ("厳しい訓練に音を上げる", "音", "ネ", 1),
        ],
        &[
            "ラケットの面を上げる",
            "号令に従い、面を上げよ",
            "音楽の音を上げる",
        ],
    );
}

#[test]
fn explicit_telephone_context_keeps_quantities_and_decimals() {
    let mut engine = Haqumei::new().unwrap();
    for (text, kana) in [
        ("119に電話", "イチイチキューニデンワ"),
        ("番号案内は104です", "バンゴーアンナイワイチゼロヨンデス"),
        ("時報は117です", "ジホーワイチイチナナデス"),
        ("171で伝言", "イチナナイチデデンゴン"),
        (
            "電話は03の1234の5678",
            "デンワワゼロサンノイチニーサンヨンノゴーロクナナハチ",
        ),
        ("電話は100ある", "デンワワヒャクアル"),
        ("FAXは100届いた", "ファックスワヒャクトドイタ"),
        ("得点は117です", "トクテンワヒャクジューナナデス"),
        ("電話番号は1.5です", "デンワバンゴーワイッテンゴデス"),
    ] {
        assert_eq!(engine.g2k(text).unwrap(), kana, "{text}");
    }
}

#[test]
fn punctuation_keeps_written_digit_groups_separate() {
    let mut engine = Haqumei::new().unwrap();
    for (text, kana) in [
        ("一〇一,", "イチゼロイチ，"),
        ("四〇五,電話", "ヨンゼロゴー，デンワ"),
        ("三〇九,三〇八", "サンゼロキュー，サンゼロハチ"),
        ("三〇九,三八一", "サンゼロキュー，サンハチイチ"),
        ("一,〇〇〇", "セン"),
        (
            "1,234,567",
            "ヒャクニジューサンマンヨンセンゴヒャクロクジューナナ",
        ),
        ("1,050.5円", "センゴジュッテンゴエン"),
        ("二〇〇〇年", "ニセンネン"),
    ] {
        assert_eq!(engine.g2k(text).unwrap(), kana, "{text}");
    }
}

#[test]
fn explicit_first_day_keeps_duration_readings() {
    let mut engine = Haqumei::new().unwrap();
    for (text, kana) in [
        ("本日1日付で", "ホンジツツイタチズケデ"),
        ("明日1日午前", "アシタツイタチゴゼン"),
        ("1日午後", "ツイタチゴゴ"),
        ("丸1日午後まで休む", "マルイチニチゴゴマデヤスム"),
        ("今日も1日、", "キョーモイチニチ、"),
        ("1日に3回", "イチニチニサンカイ"),
    ] {
        assert_eq!(engine.g2k(text).unwrap(), kana, "{text}");
    }
}

#[test]
fn school_year_lists_keep_decimal_quantities() {
    let mut engine = Haqumei::new().unwrap();
    for (text, kana) in [
        ("1・2年生", "イチ・ニネンセー"),
        ("二十・五％", "ニジュッテンゴパーセント"),
        ("1.5年", "イッテンゴネン"),
    ] {
        assert_eq!(engine.g2k(text).unwrap(), kana, "{text}");
    }
}

#[test]
fn musashi_needs_an_explicit_name_context() {
    let mut engine = Haqumei::new().unwrap();
    for text in [
        "名前は六三四です",
        "六三四さん",
        "六三四くん",
        "六三四君",
        "六三四という名前",
    ] {
        let features = engine.run_frontend(text).unwrap();
        let name = features
            .iter()
            .find(|f| f.string == "六三四")
            .unwrap_or_else(|| panic!("{text}: {features:?}"));
        assert_eq!(name.read, "ムサシ", "{text}");
        assert_eq!(name.mora_size, 3, "{text}");
    }
    for text in ["六三四", "六三四円", "一六三四", "六三四を数える"] {
        let with_rules = engine.g2k(text).unwrap();
        engine.options.modify_context_reading = false;
        let without_rules = engine.g2k(text).unwrap();
        engine.options.modify_context_reading = true;
        assert_eq!(with_rules, without_rules, "{text}");
    }
}

#[test]
fn ichiban_suki_keeps_compound_zuki_elsewhere() {
    let mut h = Haqumei::new().unwrap();
    for (text, expected) in [
        ("この味が一番好きだ", "コノアジガイチバンスキダ"),
        ("一番好きな道を歩く", "イチバンスキナミチヲアルク"),
        ("旅好きな人だ", "タビズキナヒトダ"),
        ("音楽好きの友人だ", "オンガクズキノユージンダ"),
        ("物好きな客だ", "モノズキナキャクダ"),
        ("一番手を任せる", "イチバンテヲマカセル"),
    ] {
        assert_eq!(h.g2k(text).unwrap(), expected, "{text}");
    }
    h.options.modify_context_reading = false;
    assert_eq!(h.g2k("一番好きだ").unwrap(), "イチバンズキダ");
}

#[test]
fn dated_suffix_keeps_zuke_instead_of_attached_tsuki() {
    let mut h = Haqumei::new().unwrap();
    for (text, expected) in [
        (
            "六月十二日付の通知を送る。",
            "ロクガツジューニニチズケノツーチヲオクル。",
        ),
        (
            "九月十日付で就任する。",
            "クガツトーカズケデシューニンスル。",
        ),
        (
            "四月一日付の辞令を受け取る。",
            "シガツツイタチズケノジレーヲウケトル。",
        ),
        (
            "八月十四日付の公告を読む。",
            "ハチガツジューヨッカズケノコーコクヲヨム。",
        ),
        ("二十日付で異動する。", "ハツカズケデイドースル。"),
        (
            "三十日付で就任する。",
            "サンジューニチズケデシューニンスル。",
        ),
        ("条件付の契約を結ぶ。", "ジョーケンツキノケーヤクヲムスブ。"),
        ("朝食付の宿を探す。", "チョーショクツキノヤドヲサガス。"),
        ("保証付の商品を選ぶ。", "ホショーツキノショーヒンヲエラブ。"),
    ] {
        assert_eq!(h.g2k(text).unwrap(), expected, "{text}");
    }
}

#[test]
fn restored_pronunciation_updates_the_public_mora_count() {
    for (options, expected) in [
        (
            HaqumeiOptions {
                use_read_as_pron: true,
                ..Default::default()
            },
            4,
        ),
        (
            HaqumeiOptions {
                revert_long_vowels: true,
                ..Default::default()
            },
            3,
        ),
    ] {
        let mut engine = Haqumei::with_options(options).unwrap();
        let words = engine.g2p_mapping_detailed("おめぇー").unwrap();
        assert_eq!(words.iter().map(|w| w.mora_count).sum::<i32>(), expected);
        let features = engine.run_frontend("おめぇー").unwrap();
        assert_eq!(features.iter().map(|f| f.mora_size).sum::<i32>(), expected);
    }
}

#[test]
fn karada_uses_its_dictionary_accent_before_chaining() {
    let mut engine = Haqumei::new().unwrap();
    for text in [
        "身体を鍛える",
        "身体の調子",
        "身体が重い",
        "お身体の調子",
        "身体 を 鍛える",
    ] {
        let expected = text.replace("身体", "体");
        assert_eq!(
            engine.g2p_prosody(text).unwrap(),
            engine.g2p_prosody(&expected).unwrap(),
            "{text}"
        );
    }
    for text in ["身体を拘束する", "身体の機能", "身体が不自由だ"] {
        let features = engine.run_frontend(text).unwrap();
        assert_eq!(
            features.iter().find(|f| f.string == "身体").unwrap().read,
            "シンタイ"
        );
    }
}

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
fn number_and_nannimo_corrections_respect_registered_readings() {
    let mut engine = Haqumei::new().unwrap();
    engine.set_morph_filter(|_, _, morphs| {
        for morph in morphs
            .iter_mut()
            .filter(|m| matches!(m.surface.as_str(), "何" | "〇"))
        {
            morph.dictionary_index = 1;
        }
    });
    engine.options.protect_user_dict_readings = true;
    assert!(
        engine
            .g2k("何にも知らない")
            .unwrap()
            .starts_with("ナニニモ")
    );
    assert!(
        engine
            .run_frontend("〇七〇")
            .unwrap()
            .iter()
            .filter(|f| f.string == "〇")
            .all(|f| f.read != "ゼロ")
    );
}

#[test]
fn negative_nannimo_keeps_case_particle_uses() {
    let mut engine = Haqumei::new().unwrap();
    for (text, reading) in [
        ("何にも知らない", "ナン"),
        ("何にも知りません", "ナン"),
        ("何にも知らず", "ナン"),
        ("何にも知らん", "ナン"),
        ("何にも知らぬ", "ナン"),
        ("何にも分からなかった", "ナン"),
        ("何にも知らなければ", "ナン"),
        ("何にも答えてもらっていない", "ナン"),
        ("何にも返事がなく", "ナン"),
        ("何にもありません", "ナン"),
        ("何にもすることができず", "ナン"),
        ("何にも面白くない", "ナン"),
        ("何にもしないで宣伝ばかり", "ナン"),
        ("何にも似ていない", "ナニ"),
        ("何にも全然似ていない", "ナニ"),
        ("何にも代えがたい", "ナニ"),
        ("何にも依存していない", "ナニ"),
        ("何にも属さない", "ナニ"),
        ("何にも頼らない", "ナニ"),
        ("何にも答えたが教えない", "ナニ"),
        ("何にも。知らない", "ナニ"),
        ("「何にも」という語を知らない", "ナニ"),
        ("何にも似ることができない", "ナニ"),
        ("何にもとづいて決めた", "ナニ"),
    ] {
        let features = engine.run_frontend(text).unwrap();
        assert_eq!(
            features.iter().find(|f| f.string == "何").unwrap().read,
            reading,
            "{text}"
        );
    }
    engine.options.modify_context_reading = false;
    assert!(
        engine
            .g2k("何にも知らない")
            .unwrap()
            .starts_with("ナニニモ")
    );
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
