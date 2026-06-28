use haqumei::{Haqumei, Phoneme::*, ProsodyFormat};

#[test]
fn test_default_no_allophones() {
    let mut h = Haqumei::new().unwrap();

    // デフォルトでは元の音素 (Nn, Cl) が維持される
    assert_eq!(h.g2p("サンマ").unwrap(), [S, A, Nn, M, A]);
    assert_eq!(h.g2p("切符").unwrap(), [K, I, Cl, P, U]);
    assert_eq!(h.g2p("あっ").unwrap(), [A, Cl]);
}

#[test]
fn test_n_allophones() {
    let mut h = Haqumei::new().unwrap();
    h.options.split_n_allophones = true;

    // 両唇音 (p, b, m) の前 -> Nm
    assert_eq!(h.g2p("サンマ").unwrap(), [S, A, Nm, M, A]);

    // 軟口蓋音 (k, g) の前 -> Ng
    assert_eq!(h.g2p("参加").unwrap(), [S, A, Ng, K, A]);

    // 歯茎音 (t, d, n, etc) の前 -> Nd
    assert_eq!(h.g2p("サンタ").unwrap(), [S, A, Nd, T, A]);

    // 語末・ポーズの前 -> Nq
    assert_eq!(h.g2p("本").unwrap(), [H, O, Nq]);

    // 母音などの前 -> Nn (鼻声化母音は解決しない)
    assert_eq!(h.g2p("恋愛").unwrap(), [R, E, Nn, A, I]);
}

#[test]
fn test_n_allophones_options() {
    let mut h = Haqumei::new().unwrap();
    h.options.split_n_allophones = true;

    // デフォルト（オプション無効）では ch/j, r の前は Nd に統合される
    assert_eq!(h.g2p("漢字").unwrap(), [K, A, Nd, J, I]);
    assert_eq!(h.g2p("新緑").unwrap(), [Sh, I, Nd, Ry, O, K, U]);

    // オプションを個別に有効化
    h.options.split_n_before_palatal_affricate = true;
    h.options.split_n_before_r = true;

    // 有効化時は専用ラベルに分岐する
    assert_eq!(h.g2p("漢字").unwrap(), [K, A, Npl, J, I]);
    assert_eq!(h.g2p("新緑").unwrap(), [Sh, I, Nr, Ry, O, K, U]);
}

#[test]
fn test_q_allophones() {
    let mut h = Haqumei::new().unwrap();
    h.options.split_q_allophones = true;

    // p の前 -> ClP
    assert_eq!(h.g2p("切符").unwrap(), [K, I, ClP, P, U]);

    // t, ts, ch の前 -> ClT
    assert_eq!(h.g2p("切手").unwrap(), [K, I, ClT, T, E]);

    // k の前 -> ClK
    assert_eq!(h.g2p("学校").unwrap(), [G, A, ClK, K, O, O]);

    // s, sh, f, h, v の前 -> ClS
    assert_eq!(h.g2p("雑誌").unwrap(), [Z, A, ClS, Sh, I]);

    // b, d, g, z, j の前 -> ClV
    assert_eq!(h.g2p("バッグ").unwrap(), [B, A, ClV, G, U]);

    // 語末は対象外 (Cl のまま)
    assert_eq!(h.g2p("あっ").unwrap(), [A, Cl]);
}

#[test]
fn test_final_glottal_stop() {
    let mut h = Haqumei::new().unwrap();
    h.options.enable_final_glottal_stop = true;

    // 語末 -> ClQ
    assert_eq!(h.g2p("あっ").unwrap(), [A, ClQ]);

    // 語末以外 -> Cl のまま
    assert_eq!(h.g2p("切符").unwrap(), [K, I, Cl, P, U]);
}

#[test]
fn test_use_allophones_master_switch() {
    let mut h = Haqumei::new().unwrap();
    h.options.use_allophones = true;

    // 個別フラグが false でも、`use_allophones` が優先され有効化される
    h.options.split_n_allophones = false;
    h.options.split_q_allophones = false;
    h.options.enable_final_glottal_stop = false;

    h.options.split_n_before_r = true;
    h.options.split_n_before_palatal_affricate = true;

    assert_eq!(h.g2p("サンマ").unwrap(), [S, A, Nm, M, A]);
    assert_eq!(h.g2p("切符").unwrap(), [K, I, ClP, P, U]);
    assert_eq!(h.g2p("あっ").unwrap(), [A, ClQ]);
    assert_eq!(h.g2p("漢字").unwrap(), [K, A, Nd, J, I]);
    assert_eq!(h.g2p("新緑").unwrap(), [Sh, I, Nd, Ry, O, K, U]);
}

#[test]
fn test_cross_word_boundary_lookahead() {
    let mut h = Haqumei::new().unwrap();
    h.options.use_allophones = true;

    // 撥音
    assert_eq!(
        h.g2p_per_word("本も").unwrap(),
        [vec![H, O, Nm], vec![M, O]]
    );

    // 促音
    assert_eq!(
        h.g2p_per_word("買って").unwrap(),
        [vec![K, A, ClT], vec![T, E]]
    );

    // "あっ" の後ろに読点があるため、発話境界として ClQ になる
    assert_eq!(
        h.g2p_per_word("あっ、そう").unwrap(),
        [vec![A, ClQ], vec![Pau], vec![S, O, O]]
    );
}

#[test]
fn test_mapping_cross_word_lookahead() {
    let mut h = Haqumei::new().unwrap();
    h.options.use_allophones = true;

    let mapping = h.g2p_mapping("本も").unwrap();
    assert_eq!(mapping[0].phonemes, [H, O, Nm]);

    let mapping_detailed = h.g2p_mapping_detailed("買って").unwrap();
    assert_eq!(mapping_detailed[0].phonemes, [K, A, ClT]);
}

#[test]
fn test_prosody_skip_boundaries_and_pauses() {
    let mut h = Haqumei::new().unwrap();
    h.options.use_allophones = true;

    // アクセント句境界 # をスキップして m を先読みできるか
    let prosody = h
        .g2p_prosody_with_options("本も", ProsodyFormat::Default)
        .unwrap();
    assert_eq!(prosody, ["^", "h", "o", "]", "Nm", "m", "o", "$"]);

    // プロソディ上のポーズ記号を跨いで、正しく発話境界 (ClQ / Nq) として解決されるか
    let prosody = h
        .g2p_prosody_with_options("あっ、そう", ProsodyFormat::Default)
        .unwrap();
    assert_eq!(
        prosody,
        ["^", "a", "]", "clq", "_", "s", "o", "[", "o", "$"]
    );

    let prosody = h
        .g2p_prosody_with_options("本、買った", ProsodyFormat::Default)
        .unwrap();
    assert_eq!(
        prosody,
        [
            "^", "h", "o", "]", "Nq", "_", "k", "a", "[", "clt", "t", "a", "$"
        ]
    );
}
