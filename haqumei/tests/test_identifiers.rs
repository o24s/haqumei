use haqumei::{Haqumei, HaqumeiOptions, NumberReading, UnicodeNormalization};
use unicode_normalization::UnicodeNormalization as _;

#[test]
fn reading_policies_distinguish_rooms_routes_and_models() {
    let mut engine = Haqumei::new().unwrap();

    for (text, expected) in [
        ("802号室", "ハチマルニーゴーシツ"),
        ("01号室", "マルイチゴーシツ"),
        ("二〇一号室", "ニーマルイチゴーシツ"),
        ("〇〇一号室", "マルマルイチゴーシツ"),
        ("409号線", "ヨンヒャクキューゴーセン"),
        ("型番AB-1200", "カタバンエイビーセンニヒャク"),
        ("型番B007", "カタバンビーゼロゼロナナ"),
        ("A320型", "エイサンビャクニジューガタ"),
        ("E231系", "イーニヒャクサンジューイチケー"),
        ("E5000形", "イーゴセンガタ"),
    ] {
        assert_eq!(engine.g2k(text).unwrap(), expected, "{text}");
    }

    engine.options.room_number_reading = NumberReading::Cardinal;
    engine.options.route_number_reading = NumberReading::DigitsWithMaru;
    engine.options.model_number_reading = NumberReading::Digits;

    for (text, expected) in [
        ("802号室", "ハッピャクニゴーシツ"),
        ("409号線", "ヨンマルキューゴーセン"),
        ("型番AB-1200", "カタバンエイビーイチニーゼロゼロ"),
        ("3248号機", "サンニーヨンハチゴーキ"),
    ] {
        assert_eq!(engine.g2k(text).unwrap(), expected, "{text}");
    }

    engine.options.resolve_number_identifiers = false;
    assert_eq!(engine.g2k("802号室").unwrap(), "ハッピャクニゴーシツ");
    assert_eq!(engine.g2k("409号線").unwrap(), "ヨンヒャクキューゴーセン");
    assert_eq!(engine.g2k("A320型").unwrap(), "Ａ３２０ガタ");
}

#[test]
fn room_zeros_can_be_read_as_zero_or_skipped_at_the_start() {
    let mut engine = Haqumei::with_options(HaqumeiOptions {
        room_number_reading: NumberReading::Digits,
        ..Default::default()
    })
    .unwrap();

    for (text, expected) in [
        ("802号室", "ハチゼロニーゴーシツ"),
        ("01号室", "ゼロイチゴーシツ"),
        ("001号室", "ゼロゼロイチゴーシツ"),
        ("000号室", "ゼロゼロゼロゴーシツ"),
    ] {
        assert_eq!(engine.g2k(text).unwrap(), expected, "{text}");
    }

    engine.options.skip_room_number_leading_zeros = true;

    for (text, expected) in [
        ("01号室", "イチゴーシツ"),
        ("001号室", "イチゴーシツ"),
        ("００１号室", "イチゴーシツ"),
        ("〇〇一号室", "イチゴーシツ"),
        ("零零一号室", "イチゴーシツ"),
        ("00802号室", "ハチゼロニーゴーシツ"),
        ("00100号室", "イチゼロゼロゴーシツ"),
        ("0号室", "ゼロゴーシツ"),
        ("000号室", "ゼロゴーシツ"),
    ] {
        assert_eq!(engine.g2k(text).unwrap(), expected, "{text}");
    }

    engine.options.room_number_reading = NumberReading::DigitsWithMaru;
    assert_eq!(engine.g2k("00802号室").unwrap(), "ハチマルニーゴーシツ");
    assert_eq!(engine.g2k("000号室").unwrap(), "マルゴーシツ");

    engine.options.room_number_reading = NumberReading::Cardinal;
    assert_eq!(engine.g2k("00802号室").unwrap(), "ハッピャクニゴーシツ");
    assert_eq!(engine.g2k("000号室").unwrap(), "ゼロゴーシツ");
}

#[test]
fn skipping_room_zeros_retains_the_remaining_numbers_prosody() {
    let mut engine = Haqumei::with_options(HaqumeiOptions {
        skip_room_number_leading_zeros: true,
        ..Default::default()
    })
    .unwrap();

    for reading in [
        NumberReading::Cardinal,
        NumberReading::Digits,
        NumberReading::DigitsWithMaru,
    ] {
        engine.options.room_number_reading = reading;

        for number in (0..=120).chain([201, 409, 802, 1000, 2139, 3248, 10000]) {
            let expected = engine
                .g2p_prosody(&format!("{number}号室へ向かう"))
                .unwrap();

            for zeros in ["0", "000"] {
                let text = format!("{zeros}{number}号室へ向かう");
                assert_eq!(
                    engine.g2p_prosody(&text).unwrap(),
                    expected,
                    "{reading:?}: {text}"
                );
            }
        }
    }
}

#[test]
fn skipping_leading_zeros_is_limited_to_enabled_room_numbers() {
    let mut engine = Haqumei::new().unwrap();

    for text in [
        "0",
        "01",
        "802号室",
        "001号線",
        "001号機",
        "型番AB001",
        "〒001-0001",
        "01人",
        "2008 年 05 月 05 日",
        "Ⅻ号室",
        "A001号室",
        "0.01号室",
    ] {
        engine.options.skip_room_number_leading_zeros = false;
        let expected = engine.g2p_prosody(text).unwrap();
        engine.options.skip_room_number_leading_zeros = true;
        assert_eq!(engine.g2p_prosody(text).unwrap(), expected, "{text}");
    }

    engine.options.resolve_number_identifiers = false;
    engine.options.skip_room_number_leading_zeros = false;
    let expected = engine.g2p_prosody("001号室").unwrap();
    engine.options.skip_room_number_leading_zeros = true;
    assert_eq!(engine.g2p_prosody("001号室").unwrap(), expected);
}

#[test]
fn cardinal_policy_retains_existing_number_and_counter_accents() {
    let mut enabled = Haqumei::with_options(HaqumeiOptions {
        room_number_reading: NumberReading::Cardinal,
        ..Default::default()
    })
    .unwrap();
    let mut disabled = Haqumei::with_options(HaqumeiOptions {
        resolve_number_identifiers: false,
        ..Default::default()
    })
    .unwrap();

    for number in (0..=120).chain([201, 409, 802, 1000, 2139, 3248, 10000]) {
        for suffix in ["号室", "号線", "号機"] {
            let text = format!("{number}{suffix}へ向かう");

            assert_eq!(
                enabled.g2p_prosody(&text).unwrap(),
                disabled.g2p_prosody(&text).unwrap(),
                "{text}"
            );
        }
    }
}

#[test]
fn quantities_fragments_and_lexical_names_are_not_identifiers() {
    let mut enabled = Haqumei::new().unwrap();
    let mut disabled = Haqumei::with_options(HaqumeiOptions {
        resolve_number_identifiers: false,
        ..Default::default()
    })
    .unwrap();

    for text in [
        "A320",
        "H2O",
        "H2O型",
        "COVID19型",
        "B12系統",
        "十二号室",
        "1.2号室",
        "1,200号室",
        "第12号室",
        "-12号室",
        "AB12号室",
        "型番123円",
        "型番AB12日",
        "型番AB123cd",
        "型番AB123.4",
        "型番AB123/45",
        "型番AB123_45",
        "型番AB--123",
        "型番AB123-",
        "番号123円",
        "〒1048011",
        "119に電話する",
        "12 34人",
        "2008 年 05 月 05 日",
        "Ⅻ号室",
        "ＣＤを12個買う",
        "二十万人",
        "1万201号室",
    ] {
        assert_eq!(
            enabled.g2p_prosody(text).unwrap(),
            disabled.g2p_prosody(text).unwrap(),
            "{text}"
        );
    }
}

#[test]
fn digit_reading_keeps_every_zero_and_number_boundary() {
    let mut engine = Haqumei::new().unwrap();
    let digits = [
        "マル",
        "イチ",
        "ニー",
        "サン",
        "ヨン",
        "ゴー",
        "ロク",
        "ナナ",
        "ハチ",
        "キュー",
    ];

    for number in 0..=999 {
        let text = format!("{number:03}号室");
        let expected = format!(
            "{}{}{}ゴーシツ",
            digits[number / 100],
            digits[number / 10 % 10],
            digits[number % 10]
        );

        assert_eq!(engine.g2k(&text).unwrap(), expected, "{text}");
    }

    for length in [64, 65, 1000] {
        let text = format!("型番A{}型", "0".repeat(length));
        let _ = engine.run_frontend(&text).unwrap();
    }
}

#[test]
fn identifier_spans_candidates_and_batch_agree() {
    for (mode, skip_zeros) in [
        UnicodeNormalization::None,
        UnicodeNormalization::Nfc,
        UnicodeNormalization::Nfkc,
    ]
    .into_iter()
    .flat_map(|mode| [false, true].map(|skip| (mode, skip)))
    {
        let mut engine = Haqumei::with_options(HaqumeiOptions {
            normalize_unicode: mode,
            skip_room_number_leading_zeros: skip_zeros,
            ..Default::default()
        })
        .unwrap();
        let inputs = [
            "802号室",
            "０１　号室",
            "001号室",
            "000号室",
            "零零一号室",
            "000802号室",
            "二〇一号室",
            "〇〇一号室",
            "409号線",
            "型番ＡＢ−１２００",
            "型番はB007です",
            "型番A-20B",
            "A320型",
            "E231系",
            "E5000形",
            "ｶﾞ E231系",
            "EF65 1032号機",
            "(^_^)802号室でⅫ冊読む",
            "2008 年 05 月 05 日に802号室へ行く",
            "802号室と409号線と型番AB12",
        ];

        for input in inputs {
            let text = match mode {
                UnicodeNormalization::None => input.to_owned(),
                UnicodeNormalization::Nfc => input.nfc().collect(),
                UnicodeNormalization::Nfkc => input.nfkc().collect(),
            };
            let text = haqumei_jpreprocess::normalize_text_for_open_jtalk(&text);

            assert_eq!(
                engine.run_frontend(input).unwrap(),
                engine.run_frontend_detailed(input).unwrap().0,
                "{input}"
            );

            let map = engine.g2p_mapping_detailed(input).unwrap();
            let mut end = 0;

            for word in &map {
                assert!(word.char_span.start >= end, "{input}: {map:?}");
                assert_eq!(
                    word.word,
                    text.chars()
                        .skip(word.char_span.start)
                        .take(word.char_span.len())
                        .collect::<String>(),
                    "{input}: {word:?}"
                );
                end = word.char_span.end;
            }

            assert_eq!(
                map.iter().map(|w| w.word.as_str()).collect::<String>(),
                text,
                "{input}"
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

        let single: Vec<_> = inputs
            .iter()
            .map(|text| engine.g2k(text).unwrap())
            .collect();
        assert_eq!(engine.g2k_batch(&inputs).unwrap(), single);
    }
}

#[test]
fn morph_filters_take_precedence_over_number_policies() {
    let mut engine = Haqumei::new().unwrap();
    engine.set_morph_filter(|_, _, morphs| {
        for m in morphs {
            if m.surface == "号室" {
                m.feature = "号室,名詞,一般,*,*,*,*,号室,テスト,テスト,1/3,C1".into();
            }
        }
    });

    assert_eq!(engine.g2k("802号室").unwrap(), "ハッピャクニテスト");
    let expected = engine.g2p_prosody("001号室").unwrap();
    engine.options.skip_room_number_leading_zeros = true;
    assert_eq!(engine.g2p_prosody("001号室").unwrap(), expected);
    assert_eq!(
        engine.g2p_mapping_detailed("802号室").unwrap(),
        engine
            .g2p_candidates_detailed("802号室")
            .unwrap()
            .candidates[0]
            .words
    );

    engine.set_morph_filter(|_, _, _| {});
    assert_eq!(engine.g2k("802号室").unwrap(), "ハチマルニーゴーシツ");
}
