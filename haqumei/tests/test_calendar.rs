use haqumei::{Haqumei, HaqumeiOptions, UnicodeNormalization};
use unicode_normalization::UnicodeNormalization as _;

fn prosody(engine: &mut Haqumei, text: &str) -> Vec<String> {
    engine
        .g2p_prosody(text)
        .unwrap()
        .into_iter()
        .filter(|s| s != "sp")
        .collect()
}

#[test]
fn spaced_calendar_dates_match_compact_dates() {
    let mut engine = Haqumei::new().unwrap();

    for year in [2008, 2023, 2024] {
        for month in 1..=12 {
            for day in [1, 2, 5, 9, 14, 20, 24, 28] {
                let compact = format!("{year}年{month}月{day}日");
                let expected = prosody(&mut engine, &compact);

                for space in [" ", "　"] {
                    for padded in [false, true] {
                        let (m, d) = if padded {
                            (format!("{month:02}"), format!("{day:02}"))
                        } else {
                            (month.to_string(), day.to_string())
                        };
                        let input =
                            format!("{year}{space}年{space}{m}{space}月{space}{d}{space}日");

                        assert_eq!(prosody(&mut engine, &input), expected, "{input}");
                    }
                }
            }
        }
    }

    for (era, year) in [
        ("令和", 6),
        ("平成", 10),
        ("昭和", 50),
        ("大正", 2),
        ("明治", 40),
    ] {
        for month in 1..=12 {
            let input = format!("{era} {year} 年 {month:02} 月 01 日");
            let compact = format!("{era}{year}年{month}月1日");

            assert_eq!(
                prosody(&mut engine, &input),
                prosody(&mut engine, &compact),
                "{input}"
            );
        }
    }

    for day in 1..=31 {
        let input = format!("2024 年 01 月 {day:02} 日");
        let compact = format!("2024年1月{day}日");

        assert_eq!(
            prosody(&mut engine, &input),
            prosody(&mut engine, &compact),
            "{input}"
        );
    }

    for (input, compact) in [
        ("2008 年 05 月", "2008年5月"),
        ("令和 元 年 05 月 01 日", "令和元年5月1日"),
        ("令和 06 年 04 月 01 日", "令和6年4月1日"),
        ("2024 年 02 月 29 日", "2024年2月29日"),
        ("2000 年 02 月 29 日", "2000年2月29日"),
        (
            "2024 年 04 月 01 日から出発する",
            "2024年4月1日から出発する",
        ),
        ("今日は 2024 年 04 月 01 日です", "今日は 2024年4月1日です"),
        ("2024 年 04 月 01 日。", "2024年4月1日。"),
        ("2024 年 04 月 01 日に会う", "2024年4月1日に会う"),
        ("2024 年 04 月 01 日まで待つ", "2024年4月1日まで待つ"),
        ("2024 年 04 月 01 日を予定する", "2024年4月1日を予定する"),
        ("2024 年 04 月 01 日の予定", "2024年4月1日の予定"),
    ] {
        assert_eq!(
            prosody(&mut engine, input),
            prosody(&mut engine, compact),
            "{input}"
        );
    }

    assert_eq!(
        engine.g2k("2008 年 05 月 05 日").unwrap(),
        "ニセンハチネンゴガツイツカ"
    );
}

#[test]
fn periods_invalid_dates_and_word_fragments_keep_their_morphemes() {
    let mut engine = Haqumei::with_options(HaqumeiOptions {
        resolve_number_identifiers: false,
        ..Default::default()
    })
    .unwrap();

    for input in [
        "1 月に一度会う",
        "1 月ほど待つ",
        "1 年 1 月が過ぎた",
        "1 年 1 月 1 日かかった",
        "20 月待つ",
        "2023 年 02 月 29 日",
        "1900 年 02 月 29 日",
        "2100 年 02 月 29 日",
        "2024 年 04 月 31 日",
        "2024 年 00 月 01 日",
        "2024 年 13 月 01 日",
        "2024 年 05 月 00 日",
        "2024 年 05 月 32 日",
        "2024 年 5 月次報告",
        "2024 年 5 月面調査",
        "2024 年 5 月額料金",
        "2024 年 5 月刊誌",
        "第2024 年 4 月 1 日",
        "12024 年 4 月 1 日",
        "A2024 年 4 月 1 日",
        "2024 年 4 月 1 日間かかった",
        "2024 年 4 月ほどかかった",
        "2024 年 4 月が経過した",
        "2024 年 4 月あまり過ぎた",
        "2024 年 04 月 01 日用品を買う",
        "2024 年 04 月 01 日誌を読む",
        "2024 年 04 月 01 日系企業",
        "2024 年 04 月 01 日本文化",
        "2024 年 04 月 01 日刊紙",
        "2024 年 04 月 01 日々の記録",
        "2024 年 04 月 01 日曜日",
        "2008 年 05 月 05 日付で発行",
        "2008年05月05日",
        "802号室",
        "409号線",
        "JAL15便",
        "EF65 1032号機",
        "4 月 1 日",
    ] {
        assert_eq!(
            engine.run_frontend_detailed(input).unwrap().1,
            engine.run_mecab_detailed(input).unwrap(),
            "{input}"
        );
    }
}

#[test]
fn calendar_mapping_preserves_spaces_zeros_and_candidate_consistency() {
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

        for input in [
            "2008 年 05 月 05 日",
            "2024 年 01 月 14 日",
            "2024 年 01 月 24 日",
            "2024 年 01 月 31 日",
            "２０２４　年　０４　月　０１　日",
            "ｶﾞ 2024 年 04 月 01 日から",
            "令和 元 年 05 月 01 日",
            "2024 年 01 月 20 日 (^_^) Ⅻ冊",
            "Ⅻ、2024 年 04 月 01 日",
            "2024 年 04 月 01 日と2025 年 05 月 02 日。",
        ] {
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
                "{input}: {map:?}"
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
    }
}

#[test]
fn morph_filters_override_dates_and_batch_matches_single() {
    let mut engine = Haqumei::new().unwrap();
    engine.set_morph_filter(|_, _, morphs| {
        for m in morphs {
            if m.surface == "月" {
                m.feature = "月,名詞,一般,*,*,*,*,月,テスト,テスト,1/3,C1".into();
            }
        }
    });

    let input = "2008 年 05 月 05 日";
    assert!(engine.g2k(input).unwrap().contains("テスト"));
    assert_eq!(
        engine.g2p_mapping_detailed(input).unwrap(),
        engine.g2p_candidates_detailed(input).unwrap().candidates[0].words
    );

    engine.set_morph_filter(|_, _, _| {});
    assert_eq!(engine.g2k(input).unwrap(), "ニセンハチネンゴガツイツカ");

    engine.clear_morph_filter();
    let texts = [input, "令和 6 年 04 月 01 日", "1 月に一度会う"];
    let expected: Vec<_> = texts.iter().map(|s| engine.g2k(s).unwrap()).collect();

    assert_eq!(engine.g2k_batch(&texts).unwrap(), expected);
}
