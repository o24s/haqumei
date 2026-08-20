use haqumei::OpenJTalk;

/// `delta == 0` のノードを繋ぐと最良経路になる。
#[test]
fn best_nodes_form_the_one_best_path() {
    let mut oj = OpenJTalk::new().unwrap();
    let text = "聖明王";

    let mut best: Vec<_> = oj
        .analyze_lattice(text)
        .unwrap()
        .into_iter()
        .filter(|n| n.is_best)
        .collect();
    best.sort_by_key(|n| n.char_span.start);

    let morphs = oj.run_mecab_detailed(text).unwrap();
    let expected: Vec<_> = morphs
        .iter()
        .filter(|m| !m.is_ignored)
        .map(|m| m.surface.clone())
        .collect();
    let got: Vec<_> = best.iter().map(|n| n.surface.clone()).collect();

    assert_eq!(got, expected);
}

/// 最良経路に選ばれなかった候補も、負けている量とともに出る。
///
/// `聖明王` は 1 語のエントリ (生起コスト 10000) が勝っていて、`明王` の
/// 名詞-一般 (5746) はそこから離されている。`delta` はその差で、同じだけ
/// 単語コストを下げれば分割される。
#[test]
fn losing_candidates_carry_their_margin() {
    let mut oj = OpenJTalk::new().unwrap();
    let nodes = oj.analyze_lattice("聖明王").unwrap();

    let myouou = nodes
        .iter()
        .find(|n| n.surface == "明王" && n.feature.starts_with("名詞,一般"))
        .expect("明王 の 名詞-一般 が候補に出ていない");

    assert!(!myouou.is_best);
    assert!(myouou.delta > 0);
}

/// ラティスを覗いても、そのあとの解析の挙動が変わらない。
///
/// 辺を作らせるために要求種別を書き換えるので、戻し忘れると `node.next` が
/// ラティス全体を辿るようになり、以降の解析結果が壊れる。
#[test]
fn peeking_at_the_lattice_does_not_change_later_analysis() {
    let mut oj = OpenJTalk::new().unwrap();
    let text = "聖明王が祀られている";

    let before = oj.run_mecab_detailed(text).unwrap();
    oj.analyze_lattice(text).unwrap();
    let after = oj.run_mecab_detailed(text).unwrap();

    assert_eq!(before, after);
}
