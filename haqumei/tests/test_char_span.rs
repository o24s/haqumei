use haqumei::{Haqumei, HaqumeiOptions, njd_char_spans};

#[test]
fn spaced_numeric_groups_keep_expected_word_origins() {
    let mut engine = Haqumei::new().unwrap();
    for (text, expected) in [
        (
            "01 10円",
            vec![
                ("０", 0..1),
                ("一", 1..2),
                ("　", 2..3),
                ("十", 3..5),
                ("円", 5..6),
            ],
        ),
        (
            "1200 10円",
            vec![
                ("千", 0..1),
                ("二", 1..3),
                ("百", 3..4),
                ("　", 4..5),
                ("十", 5..7),
                ("円", 7..8),
            ],
        ),
        (
            "0 〇二日",
            vec![("０", 0..1), ("　", 1..2), ("〇", 2..3), ("二日", 3..5)],
        ),
    ] {
        let (features, morphs) = engine.run_frontend_detailed(text).unwrap();
        assert_eq!(
            features
                .iter()
                .zip(njd_char_spans(&features, &morphs))
                .map(|(feature, span)| (feature.string.as_str(), span))
                .collect::<Vec<_>>(),
            expected
                .iter()
                .filter(|(word, _)| *word != "　")
                .cloned()
                .collect::<Vec<_>>(),
            "{text}"
        );
        let basic = engine.g2p_mapping(text).unwrap();
        let detailed = engine.g2p_mapping_detailed(text).unwrap();
        let prosody = engine.g2p_mapping_prosody(text).unwrap();
        for actual in [
            basic
                .iter()
                .map(|word| (word.word.as_str(), word.char_span.clone()))
                .collect::<Vec<_>>(),
            detailed
                .iter()
                .map(|word| (word.word.as_str(), word.char_span.clone()))
                .collect::<Vec<_>>(),
            prosody
                .iter()
                .map(|word| (word.word.as_str(), word.char_span.clone()))
                .collect::<Vec<_>>(),
        ] {
            assert_eq!(actual, expected, "{text}");
        }
    }
}

#[test]
fn repeated_digits_do_not_cross_numeric_separators() {
    let mut engine = Haqumei::new().unwrap();
    for separator in [" ", "　", "\n", "\t", "\r\n", "  "] {
        for (first, second) in [
            ("1", "10"),
            ("1", "11"),
            ("10", "100"),
            ("12", "120"),
            ("1", "1000"),
            ("01", "010"),
            ("01", "10"),
            ("0", "〇二日"),
            ("0", "0 2日"),
        ] {
            let suffix = if second.ends_with('日') { "" } else { "円" };
            let text = format!("{first}{separator}{second}{suffix}");
            let (features, morphs) = engine.run_frontend_detailed(&text).unwrap();
            let njd_spans = njd_char_spans(&features, &morphs);
            let mut end = 0;
            for span in njd_spans.iter().filter(|span| !span.is_empty()) {
                assert!(
                    span.start >= end,
                    "{text:?}: {features:?}, {morphs:?}, {njd_spans:?}"
                );
                end = span.end;
            }
            assert!(
                njd_spans[0].end <= first.chars().count(),
                "{text:?}: {features:?}, {morphs:?}, {njd_spans:?}"
            );

            let basic = engine.g2p_mapping(&text).unwrap();
            let detailed = engine.g2p_mapping_detailed(&text).unwrap();
            let prosody = engine.g2p_mapping_prosody(&text).unwrap();
            let spans = basic
                .iter()
                .map(|word| word.char_span.clone())
                .collect::<Vec<_>>();
            let mut end = 0;
            for span in spans.iter().filter(|span| !span.is_empty()) {
                assert_eq!(span.start, end, "{text:?}: {basic:?}");
                end = span.end;
            }
            assert_eq!(end, text.chars().count(), "{text:?}: {basic:?}");
            assert_eq!(
                spans,
                detailed
                    .iter()
                    .map(|word| word.char_span.clone())
                    .collect::<Vec<_>>(),
                "{text:?}"
            );
            assert_eq!(
                spans,
                prosody
                    .iter()
                    .map(|word| word.char_span.clone())
                    .collect::<Vec<_>>(),
                "{text:?}"
            );
        }
    }
}

#[test]
fn numeric_alignment_keeps_group_boundaries_in_the_long_input_fallback() {
    let mut engine = Haqumei::new().unwrap();
    for (first, second) in [
        ("1".repeat(129), "10".into()),
        ("10".into(), "1".repeat(129)),
    ] {
        let text = format!("{first} {second}円");
        let separator = first.chars().count();
        let mapping = engine.g2p_mapping_detailed(&text).unwrap();
        let mut end = 0;
        for word in mapping.iter().filter(|word| !word.char_span.is_empty()) {
            assert_eq!(word.char_span.start, end, "{text:?}: {mapping:?}");
            if !word.is_ignored {
                assert!(
                    word.char_span.end <= separator || word.char_span.start > separator,
                    "{text:?}: {mapping:?}"
                );
            }
            end = word.char_span.end;
        }
        assert_eq!(end, text.chars().count(), "{text:?}: {mapping:?}");
    }
}

#[test]
fn a_contracted_number_owns_its_internal_formatting_spaces() {
    let mut engine = Haqumei::new().unwrap();
    for text in ["1, 000円", "1 ,000円", "1, 000, 000円"] {
        assert_eq!(
            engine.g2k(text).unwrap(),
            engine.g2k(&text.replace(' ', "")).unwrap()
        );
        let (features, morphs) = engine.run_frontend_detailed(text).unwrap();
        let spans = njd_char_spans(&features, &morphs);
        let basic = engine.g2p_mapping(text).unwrap();
        let detailed = engine.g2p_mapping_detailed(text).unwrap();
        let prosody = engine.g2p_mapping_prosody(text).unwrap();
        let mut end = 0;
        for word in detailed.iter().filter(|word| !word.char_span.is_empty()) {
            assert_eq!(word.char_span.start, end, "{text:?}: {detailed:?}");
            end = word.char_span.end;
        }
        assert_eq!(end, text.chars().count(), "{text:?}: {detailed:?}");
        assert_eq!(
            spans,
            detailed
                .iter()
                .map(|word| word.char_span.clone())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            spans,
            basic
                .iter()
                .map(|word| word.char_span.clone())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            spans,
            prosody
                .iter()
                .map(|word| word.char_span.clone())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn line_breaks_and_tabs_keep_numbers_separate_in_all_mappings() {
    let mut engine = Haqumei::new().unwrap();
    for separator in ["\n", "\r", "\r\n", "\t"] {
        for (before, after) in [("10", "20円"), ("1", "2人"), ("今日は晴れ。", "明日は雨。")]
        {
            let input = format!("{before}{separator}{after}");
            let reference = format!("{before}{}{after}", " ".repeat(separator.len()));
            assert_eq!(engine.g2p(&input).unwrap(), engine.g2p(&reference).unwrap());
            assert_eq!(engine.g2k(&input).unwrap(), engine.g2k(&reference).unwrap());
            assert_eq!(
                engine.g2p_prosody(&input).unwrap(),
                engine.g2p_prosody(&reference).unwrap()
            );
            let basic = engine.g2p_mapping(&input).unwrap();
            let detailed = engine.g2p_mapping_detailed(&input).unwrap();
            let prosody = engine.g2p_mapping_prosody(&input).unwrap();
            let spans: Vec<_> = basic.iter().map(|word| word.char_span.clone()).collect();
            let mut end = 0;
            for span in &spans {
                if !span.is_empty() {
                    assert_eq!(span.start, end, "{input:?}: {basic:?}");
                    end = span.end;
                }
            }
            assert_eq!(end, input.chars().count(), "{input:?}");
            assert_eq!(
                spans,
                detailed
                    .iter()
                    .map(|word| word.char_span.clone())
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                spans,
                prosody
                    .iter()
                    .map(|word| word.char_span.clone())
                    .collect::<Vec<_>>()
            );
        }
    }
}

#[test]
fn explicit_person_name_keeps_the_honorific_span() {
    let mut engine = Haqumei::new().unwrap();
    for (text, suffix) in [
        ("六三四さん", 3..5),
        ("六三四くん", 3..5),
        ("六三四君", 3..4),
    ] {
        let (features, morphs) = engine.run_frontend_detailed(text).unwrap();
        let spans = njd_char_spans(&features, &morphs);
        assert_eq!(spans, vec![0..3, suffix], "{text}: {features:?}");
        let mapped = engine.g2p_mapping(text).unwrap();
        assert_eq!(
            mapped
                .iter()
                .map(|word| word.char_span.clone())
                .collect::<Vec<_>>(),
            spans
        );
    }
}

#[test]
fn number_and_symbol_boundaries_preserve_all_mapping_variants() {
    use haqumei::Phoneme;
    let mut engine = Haqumei::new().unwrap();
    for input in [
        "三〇九,三〇八",
        "一,〇〇〇",
        "　―「一人で",
        "㏄　＋　レモン",
        "４　四つの重点課題",
        "３三振",
        "１一般",
        "千九百二十一　十一月",
        "予定1 10日まで",
        "1 20日",
        "1 24日",
        "1　10日",
        "1  10日",
        "1 一日",
        "1 十日",
        "1 二十日",
        "1 四日",
        "十 十三",
        "十 十三駅",
        "1 〇二日",
        "1 〇・五日",
        "住所は13ー3ー5です。",
        "13ーー3ーー5",
    ] {
        let basic = engine.g2p_mapping(input).unwrap();
        let detailed = engine.g2p_mapping_detailed(input).unwrap();
        let prosody = engine.g2p_mapping_prosody(input).unwrap();
        let mut end = 0;
        for word in &basic {
            if !word.char_span.is_empty() {
                assert_eq!(word.char_span.start, end, "{input}: {basic:?}");
                end = word.char_span.end;
            }
        }
        assert_eq!(end, input.chars().count(), "{input}: {basic:?}");
        let spans: Vec<_> = basic.iter().map(|w| &w.char_span).collect();
        assert_eq!(
            spans,
            detailed.iter().map(|w| &w.char_span).collect::<Vec<_>>(),
            "{input}"
        );
        assert_eq!(
            spans,
            prosody.iter().map(|w| &w.char_span).collect::<Vec<_>>(),
            "{input}"
        );
        let mapped: Vec<_> = basic
            .iter()
            .flat_map(|w| w.phonemes.iter().copied())
            .filter(|p| !matches!(p, Phoneme::Sp | Phoneme::Pau | Phoneme::Unk))
            .collect();
        let flat: Vec<_> = detailed
            .iter()
            .flat_map(|w| w.phonemes.iter().copied())
            .filter(|p| !matches!(p, Phoneme::Sp | Phoneme::Pau | Phoneme::Unk))
            .collect();
        assert_eq!(mapped, flat, "{input}");
    }
}

#[test]
fn spaced_faces_keep_the_following_sentence_and_phone_origins() {
    let mut engine = Haqumei::new().unwrap();
    for faces in ["(^^) (^^)", "(^^)　(^^)", "(^^)  (^^)"] {
        for prefix in ["おはよう", "1 24日"] {
            let text = format!("{prefix}{faces}今日は晴れです。");
            assert_eq!(
                engine.g2p(&text).unwrap(),
                engine.g2p(&format!("{prefix}今日は晴れです。")).unwrap()
            );
            let map = engine.g2p_mapping_detailed(&text).unwrap();
            assert_eq!(map.iter().filter(|w| w.pos_group1 == "顔文字").count(), 2);
            assert!(map.iter().any(|w| w.word == "今日"));
            assert!(!engine.g2p_mapping_prosody(&text).unwrap().is_empty());
        }
    }
}

#[test]
fn numeric_contractions_keep_commas_zeros_and_the_day_counter() {
    let mut engine = Haqumei::new().unwrap();
    for input in ["1,000ー", "二〇日", "11日", "1日2日", "1,2"] {
        let detailed = engine.g2p_mapping_detailed(input).unwrap();
        let mut end = 0;
        for word in &detailed {
            if !word.char_span.is_empty() {
                assert_eq!(word.char_span.start, end, "{input}: {detailed:?}");
                end = word.char_span.end;
            }
        }
        assert_eq!(end, input.chars().count(), "{input}: {detailed:?}");
        let basic = engine.g2p_mapping(input).unwrap();
        let prosody = engine.g2p_mapping_prosody(input).unwrap();
        assert_eq!(
            basic.iter().map(|w| &w.char_span).collect::<Vec<_>>(),
            detailed.iter().map(|w| &w.char_span).collect::<Vec<_>>()
        );
        assert_eq!(
            prosody.iter().map(|w| &w.char_span).collect::<Vec<_>>(),
            detailed.iter().map(|w| &w.char_span).collect::<Vec<_>>()
        );
    }
}

/// 表層形がそのまま対応する場合、NJD の位置は MeCab の位置と一致する。
#[test]
fn spans_line_up_when_the_segmentation_matches() {
    let mut haqumei = Haqumei::with_options(HaqumeiOptions::default()).unwrap();
    let text = "今日は良い天気";
    let (features, morphs) = haqumei.run_frontend_detailed(text).unwrap();
    let spans = njd_char_spans(&features, &morphs);

    assert_eq!(spans.len(), features.len());
    // 位置から入力を切り出すと表層形に戻る
    let chars: Vec<char> = text.chars().collect();
    for (f, span) in features.iter().zip(&spans) {
        let cut: String = chars[span.clone()].iter().collect();
        assert_eq!(cut, f.string, "{} の位置が合っていない", f.string);
    }
}

/// NJD は数字を正規化するので、形態素は 1 対 1 にならない。
///
/// `１４７３年` の MeCab の形態素は `１ ４ ７ ３ 年` だが、NJD は
/// `千 四 百 七 十 三 年` にする。数字から作られた形態素の位置は元の 4 文字の
/// 内側に収まり、`年` は自分の位置に戻る。
#[test]
fn contracted_digits_keep_pointing_at_the_original_text() {
    let mut haqumei = Haqumei::with_options(HaqumeiOptions::default()).unwrap();
    let text = "１４７３年";
    let (features, morphs) = haqumei.run_frontend_detailed(text).unwrap();
    let spans = njd_char_spans(&features, &morphs);

    assert_eq!(spans.len(), features.len());

    let year = features.iter().position(|f| f.string == "年").unwrap();
    assert_eq!(spans[year], 4..5);

    // 数字から作られた形態素は、いずれも元の 4 文字の内側を指す
    for span in &spans[..year] {
        assert!(span.start < 4, "数字の位置が元の範囲を超えている: {span:?}");
        assert!(span.end <= 4, "数字の位置が元の範囲を超えている: {span:?}");
    }
}

/// 位置は後戻りしない。
///
/// 数字の縮約で形態素の対応が崩れても、前の形態素より手前を指すことは無い。
#[test]
fn spans_are_monotonic() {
    let mut haqumei = Haqumei::with_options(HaqumeiOptions::default()).unwrap();
    let (features, morphs) = haqumei
        .run_frontend_detailed("２０億円を、慈善事業に寄付した。")
        .unwrap();
    let spans = njd_char_spans(&features, &morphs);

    let mut previous = 0;
    for span in &spans {
        assert!(span.start >= previous, "位置が戻っている: {span:?}");
        previous = span.start;
    }
}

/// 算用数字と漢数字が混ざっても位置がずれない。
///
/// `njd_set_digit` は位取りの文字を差し込む一方 (`２０` -> `二 十`)、`５千` の
/// ように元から漢数字がある並びでは入力を吸収する。1 つずつ順に対応させると
/// そこでずれて、以降の形態素が 1 つ手前を指すようになる。
#[test]
fn digit_blocks_do_not_shift_the_following_spans() {
    let mut haqumei = Haqumei::with_options(HaqumeiOptions::default()).unwrap();

    for (text, expected) in [
        // ５ が 五 に、千 はそのまま。円 は自分の位置に残る
        ("５千円", vec![("五", 0..1), ("千", 1..2), ("円", 2..3)]),
        // ２０ が 二 十 になる。億 と 円 は自分の位置に残る
        (
            "２０億円",
            vec![("二", 0..1), ("十", 1..2), ("億", 2..3), ("円", 3..4)],
        ),
    ] {
        let (features, morphs) = haqumei.run_frontend_detailed(text).unwrap();
        let spans = njd_char_spans(&features, &morphs);
        let got: Vec<(&str, std::ops::Range<usize>)> = features
            .iter()
            .zip(&spans)
            .map(|(f, s)| (f.string.as_str(), s.clone()))
            .collect();
        assert_eq!(got, expected, "{text} の位置がずれている");
    }
}

/// 位取りとして差し込まれた形態素には、元の文字が無い。
///
/// 元の文字が無いので、区間は空になる。空の区間は必ず形態素の境界に来るため開始
/// 位置が隣と並んでしまい、これは避けられないので、位置で形態素を引く側が空の区間を
/// 除く必要がある (`apply_postprocessing` の読みの書き戻し)。
///
/// 中身のある区間どうしは開始位置が重ならない。開始位置を鍵にしたハッシュマップで
/// 形態素を引ける根拠である。
#[test]
fn inserted_place_value_morphemes_get_an_empty_span() {
    let mut haqumei = Haqumei::with_options(HaqumeiOptions::default()).unwrap();
    let (features, morphs) = haqumei.run_frontend_detailed("１４７３年").unwrap();
    let spans = njd_char_spans(&features, &morphs);

    // 百 と 十 は元の文字を持たない
    for (f, span) in features.iter().zip(&spans) {
        if f.string == "百" || f.string == "十" {
            assert_eq!(span.start, span.end, "{} は空の区間のはず", f.string);
        }
    }

    let mut starts: Vec<usize> = spans
        .iter()
        .filter(|s| s.start < s.end)
        .map(|s| s.start)
        .collect();
    let before = starts.len();
    starts.sort_unstable();
    starts.dedup();
    assert_eq!(
        before,
        starts.len(),
        "中身のある区間の開始位置が重なっている"
    );
}
