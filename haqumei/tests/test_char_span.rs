use haqumei::{Haqumei, HaqumeiOptions, njd_char_spans};

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
