use haqumei::{CandidateOptions, Haqumei, HaqumeiOptions};

fn phoneme_string(words: &[haqumei::WordPhonemeMap]) -> String {
    words
        .iter()
        .flat_map(|w| w.phonemes.iter())
        .map(|p| p.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

/// 先頭は候補ではなく 1-best そのものである。
#[test]
fn the_first_candidate_is_the_one_best() {
    let mut haqumei = Haqumei::new().unwrap();
    for text in [
        "今日は良い天気だ。",
        "傘の柄が折れた。",
        "１４７３年に何かが起きた。",
        "𰻞𰻞麺 お冷を頼んだ",
    ] {
        let expected = haqumei.g2p_mapping(text).unwrap();
        let got = haqumei.g2p_candidates(text).unwrap();
        assert_eq!(got.candidates[0].words, expected, "{text}");
        assert_eq!(got.candidates[0].delta, 0, "{text}");
        assert!(
            got.candidates[0].choices.iter().all(|&c| c == 0),
            "{text} の先頭が 1-best を選んでいない"
        );

        let expected = haqumei.g2p_mapping_detailed(text).unwrap();
        let got = haqumei.g2p_candidates_detailed(text).unwrap();
        assert_eq!(got.candidates[0].words, expected, "{text} (detailed)");

        let expected = haqumei.g2p_mapping_prosody(text).unwrap();
        let got = haqumei.g2p_candidates_prosody(text).unwrap();
        assert_eq!(got.candidates[0].words, expected, "{text} (prosody)");
    }
}

/// 同形異音語は 2 件目の候補が出る。
#[test]
fn a_heteronym_gives_a_second_reading() {
    let mut haqumei = Haqumei::new().unwrap();
    let got = haqumei.g2p_candidates("今日は良い天気だ。").unwrap();

    let branch = got
        .branches
        .iter()
        .find(|b| b.surface == "今日")
        .expect("今日 が分岐点になっていない");
    assert_eq!(branch.char_span, 0..2);
    assert_eq!(branch.alternatives[0].pron(), "キョー");
    assert!(
        branch.alternatives.iter().any(|a| a.pron() == "コンニチ"),
        "コンニチ が代替に無い"
    );

    let phonemes: Vec<String> = got
        .candidates
        .iter()
        .map(|c| phoneme_string(&c.words))
        .collect();
    assert!(
        phonemes.iter().any(|p| p.starts_with("ky o o")),
        "{phonemes:?}"
    );
    assert!(
        phonemes.iter().any(|p| p.starts_with("k o N n i ch i")),
        "{phonemes:?}"
    );
}

/// 経路はコスト差の昇順に並び、先頭は必ず 1-best である。
#[test]
fn alternatives_are_ordered_by_cost() {
    let mut haqumei = Haqumei::new().unwrap();
    let got = haqumei
        .g2p_candidates("誰が今日の方角を決めたのか、何も聞いていない。")
        .unwrap();

    for branch in &got.branches {
        assert_eq!(branch.alternatives[0].delta, 0, "{}", branch.surface);
        assert!(
            branch
                .alternatives
                .windows(2)
                .all(|w| w[0].delta <= w[1].delta),
            "{} の代替が昇順でない",
            branch.surface
        );
    }
    assert!(
        got.candidates.windows(2).all(|w| w[0].delta <= w[1].delta),
        "候補がコスト差の昇順でない"
    );
}

/// 音素列が同じ候補は残さない。
#[test]
fn candidates_are_unique_by_phonemes() {
    let mut haqumei = Haqumei::new().unwrap();
    for text in [
        "傘の柄が折れた。",
        "何か白いものが見えた。",
        "１０００語を超えてはならない。",
        "誰が今日の方角を決めたのか。",
    ] {
        let got = haqumei.g2p_candidates(text).unwrap();
        let mut seen = std::collections::HashSet::new();
        for c in &got.candidates {
            assert!(seen.insert(phoneme_string(&c.words)), "{text} に重複がある");
        }
    }
}

/// 読みを決める補正が書き込む箇所は、ラティスが分かれていても候補が 1 つになる。
///
/// `predict_nani` が「何」の読みを無条件に書き込むので、`ナニ` と `ナン` の
/// 両方がラティスに立っていても音素は変わらない。オプションを `false` にすると
/// 2 件になる。
#[test]
fn a_correction_that_decides_the_reading_collapses_the_branch() {
    let mut haqumei = Haqumei::new().unwrap();
    let text = "何か白いものが見えた。";
    let got = haqumei.g2p_candidates(text).unwrap();
    assert!(
        got.branches.iter().any(|b| b.surface == "何"),
        "何 が分岐点になっていない"
    );
    assert_eq!(got.candidates.len(), 1, "predict_nani が畳んでいない");

    let mut haqumei = Haqumei::with_options(HaqumeiOptions {
        predict_nani: false,
        ..Default::default()
    })
    .unwrap();
    let got = haqumei.g2p_candidates(text).unwrap();
    assert_eq!(got.candidates.len(), 2);
}

/// `max_candidates` が減らすのは候補だけで、分岐点は残る。
#[test]
fn max_candidates_caps_the_candidates_but_not_the_branches() {
    let mut haqumei = Haqumei::new().unwrap();
    let text = "誰が今日の方角を決めたのか、何も聞いていない。";
    let branches = haqumei.g2p_candidates(text).unwrap().branches;
    assert!(branches.len() >= 2, "分岐点が足りない: {branches:?}");

    // 0 は 1 として扱う。候補は空にならない
    for limit in [0usize, 1, 2, 3] {
        let got = haqumei
            .g2p_candidates_with_options(
                text,
                CandidateOptions {
                    max_candidates: limit,
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(got.candidates.len() <= limit.max(1), "{limit}");
        assert_eq!(got.candidates[0].delta, 0, "{limit}");
        assert_eq!(got.branches, branches, "{limit} で分岐点が減っている");
    }
}

/// `max_delta` を 0 にすると、単語コストまで同じエントリしか残らない。
#[test]
fn max_delta_limits_the_alternatives() {
    let mut haqumei = Haqumei::new().unwrap();
    let text = "今日は良い天気だ。";
    let got = haqumei
        .g2p_candidates_with_options(
            text,
            CandidateOptions {
                max_delta: 0,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(got.branches.is_empty(), "{:?}", got.branches);
    assert_eq!(got.candidates.len(), 1);
}

/// 語の区間を並べると、入力に隙間ができない。
///
/// 並べるのは `is_ignored` を含めた全部で、空白を除くと隙間ができる。数字の縮約が
/// 起きる入力では区間が重なるので、位置の順に並べ直してから見る。
#[test]
fn the_spans_of_all_words_cover_the_text() {
    let mut haqumei = Haqumei::new().unwrap();
    for text in [
        "今日は良い天気だ。",
        "これは テスト です。",
        "𰻞𰻞麺 お冷を頼んだ",
        "１４７３年",
        "２００万円",
        "1 0 個",
    ] {
        let got = haqumei.g2p_candidates(text).unwrap();
        let len = got.text.chars().count();

        let mut spans: Vec<_> = got.candidates[0]
            .words
            .iter()
            .map(|w| w.char_span.clone())
            .filter(|s| s.start < s.end)
            .collect();
        spans.sort_by_key(|s| (s.start, s.end));

        assert_eq!(spans.first().map(|s| s.start), Some(0), "{text}");
        let mut at = 0usize;
        for span in &spans {
            assert!(span.start <= at, "{text}: {span:?} の前に隙間がある");
            assert!(span.end <= len, "{text}: {span:?} が末尾を越えている");
            at = at.max(span.end);
        }
        assert_eq!(at, len, "{text}: 末尾に達していない");
    }
}

/// 語の区間は `njd_char_spans` が返すものと同じである。
///
/// 同じ問いに 2 つの答えがあると、どちらを読めばよいかを利用者が決められない。
#[test]
fn the_spans_agree_with_njd_char_spans() {
    let mut haqumei = Haqumei::new().unwrap();
    for text in [
        "今日は良い天気だ。",
        "１４７３年",
        "２００万円",
        "１０パーセント",
    ] {
        let (features, morphs) = haqumei.run_frontend_detailed(text).unwrap();
        let expected = haqumei::njd_char_spans(&features, &morphs);
        let got: Vec<_> = haqumei
            .g2p_mapping_detailed(text)
            .unwrap()
            .into_iter()
            .filter(|w| !w.is_ignored)
            .map(|w| w.char_span)
            .collect();
        assert_eq!(got, expected, "{text}");
    }
}

/// 区間から切り出すと表層形に戻る。数字の縮約だけは戻らない。
///
/// `text2mecab` は半角カナと濁点の並びを 1 文字にまとめ、ASCII を全角にするので、
/// `Candidates::text` が入力のままだと切り出しが表層形に戻らない。
#[test]
fn a_span_cuts_out_the_surface() {
    let mut haqumei = Haqumei::new().unwrap();
    for text in ["今日は良い天気だ。", "ｶﾞムを買う", "1個ください"] {
        let got = haqumei.g2p_candidates(text).unwrap();
        let chars: Vec<char> = got.text.chars().collect();
        for w in &got.candidates[0].words {
            // 数字は NJD が漢数字に置き換えるので、切り出しても戻らない
            if w.word
                .chars()
                .any(|c| "一二三四五六七八九十百千万".contains(c))
            {
                continue;
            }
            let cut: String = chars[w.char_span.clone()].iter().collect();
            assert_eq!(cut, w.word, "{text}: {} の位置が合っていない", w.word);
        }
    }
}

/// 2 文字以上の記号の未知語は 1 文字ずつの形態素に割られる。
///
/// 割ったあとの区間に対応するラティスのノードが無いので、その区間では 1-best の
/// 経路が見つからない。分岐点にならないだけで、候補は返る。
#[test]
fn a_split_symbol_run_does_not_break_the_generation() {
    let mut haqumei = Haqumei::new().unwrap();
    let got = haqumei.g2p_candidates("♨♨に行く。").unwrap();
    assert_eq!(got.candidates.len(), 1);
    assert_eq!(
        got.candidates[0].words,
        haqumei.g2p_mapping("♨♨に行く。").unwrap()
    );
}

/// 空の入力では何も返さない。
#[test]
fn empty_text_gives_nothing() {
    let mut haqumei = Haqumei::new().unwrap();
    let got = haqumei.g2p_candidates("").unwrap();
    assert!(got.text.is_empty());
    assert!(got.branches.is_empty());
    assert!(got.candidates.is_empty());
}

/// `choices` は分岐点と長さが揃い、`alternatives` の添字として使える。
#[test]
fn choices_index_into_the_branches() {
    let mut haqumei = Haqumei::new().unwrap();
    let got = haqumei
        .g2p_candidates("誰が今日の方角を決めたのか。")
        .unwrap();
    for c in &got.candidates {
        assert_eq!(c.choices.len(), got.branches.len());
        let delta: i64 = c
            .choices
            .iter()
            .zip(&got.branches)
            .map(|(&i, b)| b.alternatives[i].delta)
            .sum();
        assert_eq!(delta, c.delta);
    }
}

/// バッチは 1 文ずつ呼んだ結果と同じものを返す。
#[test]
fn the_batch_matches_one_by_one() {
    let mut haqumei = Haqumei::new().unwrap();
    let texts = [
        "今日は良い天気だ。".to_string(),
        "傘の柄が折れた。".to_string(),
    ];
    let batched = haqumei.g2p_candidates_batch(&texts).unwrap();
    for (text, got) in texts.iter().zip(batched) {
        assert_eq!(haqumei.g2p_candidates(text).unwrap(), got);
    }
}

/// プロソディの候補は、音素だけでは同じになるものを残す。
///
/// アクセント核の位置と句の切れ目が候補ごとに変わるので、
/// `g2p_candidates_prosody` は `g2p_candidates` より候補が多くなることがある。
#[test]
fn prosody_keeps_candidates_that_phonemes_alone_would_merge() {
    let mut haqumei = Haqumei::new().unwrap();
    let text = "六時半に起きた。";
    let by_phoneme = haqumei.g2p_candidates(text).unwrap();
    let by_prosody = haqumei.g2p_candidates_prosody(text).unwrap();
    assert_eq!(by_prosody.branches, by_phoneme.branches);
    assert!(by_prosody.candidates.len() >= by_phoneme.candidates.len());
}

/// 分割の違いも候補になる。
///
/// 「彼の」は `彼` + `の` (カレノ) と 連体詞 `彼の` (アノ) の 2 通りで、形態素の数が
/// 変わる。区間が 1-best の形態素と一致するノードだけを見ていると出てこない。
#[test]
fn a_different_segmentation_is_also_a_candidate() {
    let mut haqumei = Haqumei::new().unwrap();
    let got = haqumei.g2p_candidates("彼の話を聞いた。").unwrap();

    let branch = got
        .branches
        .iter()
        .find(|b| b.surface == "彼の")
        .expect("彼の が分岐点になっていない");
    assert_eq!(branch.char_span, 0..2);

    // 1-best は 彼 + の の 2 形態素、代替は 連体詞 1 形態素
    assert_eq!(branch.alternatives[0].pron(), "カレノ");
    assert_eq!(branch.alternatives[0].nodes.len(), 2);
    let ano = branch
        .alternatives
        .iter()
        .find(|a| a.pron() == "アノ")
        .expect("アノ が代替に無い");
    assert_eq!(ano.nodes.len(), 1);
    assert_eq!(ano.nodes[0].surface, "彼の");
    assert_eq!(ano.nodes[0].char_span, 0..2);

    let phonemes: Vec<String> = got
        .candidates
        .iter()
        .map(|c| phoneme_string(&c.words))
        .collect();
    assert!(
        phonemes.iter().any(|p| p.starts_with("k a r e n o")),
        "{phonemes:?}"
    );
    assert!(
        phonemes.iter().any(|p| p.starts_with("a n o")),
        "{phonemes:?}"
    );
}

/// 差し替えた語の `features` は、その候補が使ったエントリのものになる。
///
/// 1-best の形態素列を使い回すと、`pron` は NJD が入れ直すので正しく、`features`
/// だけが 1-best のエントリのまま残る。
#[test]
fn the_features_belong_to_the_chosen_entry() {
    let mut haqumei = Haqumei::new().unwrap();
    let got = haqumei
        .g2p_candidates_detailed("今日は良い天気だ。")
        .unwrap();
    assert!(got.candidates.len() >= 2);
    for c in &got.candidates {
        let w = &c.words[0];
        assert_eq!(w.word, "今日");
        // 発音の列 (表層形が先頭に付くので 9 列目)
        assert_eq!(
            w.features[9], w.pron,
            "features が候補と食い違う: {:?}",
            w.features
        );
    }
}

/// 未知語のノードは既定では経路に含めない。
#[test]
fn unknown_nodes_are_not_readings_by_default() {
    let mut haqumei = Haqumei::new().unwrap();
    let text = "ボッティチェリの作品に期待してます。";
    let got = haqumei.g2p_candidates(text).unwrap();
    assert!(
        !got.branches
            .iter()
            .any(|b| b.surface.contains("ボッティチェリ")),
        "{:?}",
        got.branches
    );

    let got = haqumei
        .g2p_candidates_with_options(
            text,
            CandidateOptions {
                branch_on_unknown_words: true,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(
        got.branches
            .iter()
            .any(|b| b.surface.contains("ボッティチェリ")),
        "{:?}",
        got.branches
    );
}

/// 極端な上限を渡しても落ちない。
///
/// `usize::MAX` を掛け算に通すと桁が溢れ、そのまま `Vec::with_capacity` に渡すと
/// 確保に失敗する。
#[test]
fn extreme_limits_do_not_panic() {
    let mut haqumei = Haqumei::new().unwrap();
    let text = "誰が今日の方角を決めたのか。";
    let got = haqumei
        .g2p_candidates_with_options(
            text,
            CandidateOptions {
                max_delta: i64::MAX,
                max_alternatives_per_branch: usize::MAX,
                max_candidates: usize::MAX,
                branch_on_unknown_words: true,
            },
        )
        .unwrap();
    assert!(!got.candidates.is_empty());
    assert_eq!(got.candidates[0].delta, 0);
}
