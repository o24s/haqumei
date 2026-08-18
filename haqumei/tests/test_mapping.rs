#[cfg(test)]
mod tests {
    use haqumei::{Haqumei, OpenJTalk, Phoneme};
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::LazyLock,
    };

    static MANIFEST_DIR: LazyLock<&Path> = LazyLock::new(|| Path::new(env!("CARGO_MANIFEST_DIR")));
    static WAGANEKO_PATH: LazyLock<PathBuf> =
        LazyLock::new(|| MANIFEST_DIR.join("../resources/waganeko.txt"));

    const NIGHTMARE_TEXT: &str = "\
つまみ出されようとしたが、「「八十５歳」」にもなる 長老ー ー に助けられた。\
そこで、𰻞𰻞麺。\
ーっ、 𰻞ー𰻞。あ、はい。あーーーーーーーーあ\
叙々々々々々々苑々々様々々要所々々々々々槇野々々々散々々\
２0１８ Oｐeｎ ＪTaｌｋ　１．１１\
！？！？！？！？ー！￥／？ー！？！？\
CPU it It IT ああ aaー allあ haqumei g2ｐ\
";

    #[test]
    fn test_mapping_nightmare_case() {
        let mut haqumei = Haqumei::new().unwrap();
        haqumei.options.use_allophones = true;

        let expected = vec![
            (
                "つまみ出さ",
                vec!["ts", "u", "m", "a", "m", "i", "d", "a", "s", "a"],
            ),
            ("れよう", vec!["r", "e", "y", "o", "o"]),
            ("と", vec!["t", "o"]),
            ("し", vec!["sh", "I"]),
            ("た", vec!["t", "a"]),
            ("が", vec!["g", "a"]),
            ("、", vec!["pau"]),
            ("「", vec![]),
            ("「", vec![]),
            ("八", vec!["h", "a", "ch", "i"]),
            ("十", vec!["j", "u", "u"]),
            ("五", vec!["g", "o"]),
            ("歳", vec!["s", "a", "i"]),
            ("」", vec![]),
            ("」", vec![]),
            ("に", vec!["n", "i"]),
            ("も", vec!["m", "o"]),
            ("なる", vec!["n", "a", "r", "u"]),
            ("\u{3000}", vec!["sp"]),
            ("長老ーー", vec!["ch", "o", "o", "r", "o", "o", "o", "o"]),
            ("\u{3000}", vec!["sp"]),
            ("\u{3000}", vec!["sp"]),
            ("に", vec!["n", "i"]),
            ("助け", vec!["t", "a", "s", "U", "k", "e"]),
            ("られ", vec!["r", "a", "r", "e"]),
            ("た", vec!["t", "a"]),
            ("。", vec!["pau"]),
            ("そこで", vec!["s", "o", "k", "o", "d", "e"]),
            ("、", vec!["pau"]),
            ("𰻞𰻞", vec!["unk"]),
            ("麺", vec!["m", "e", "Nq"]),
            ("。", vec!["pau"]),
            ("ー", vec!["unk"]),
            ("っ", vec!["clq"]),
            ("、", vec!["pau"]),
            ("\u{3000}", vec!["sp"]),
            ("𰻞", vec!["unk"]),
            ("ー", vec!["unk"]),
            ("𰻞", vec!["unk"]),
            ("。", vec!["pau"]),
            ("あ", vec!["a"]),
            ("、", vec!["pau"]),
            ("はい", vec!["h", "a", "i"]),
            ("。", vec!["pau"]),
            (
                "あーーーーーーーー",
                vec!["a", "a", "a", "a", "a", "a", "a", "a", "a"],
            ),
            ("あ", vec!["a"]),
            ("叙", vec!["j", "o"]),
            (
                "々々々々々々",
                vec!["j", "o", "j", "o", "j", "o", "j", "o", "j", "o", "j", "o"],
            ),
            ("苑", vec!["e", "N"]),
            ("々々", vec!["e", "N", "e", "N"]),
            ("様々", vec!["s", "a", "m", "a", "z", "a", "m", "a"]),
            ("々", vec!["z", "a", "m", "a"]),
            (
                "要所々々",
                vec!["y", "o", "o", "sh", "o", "y", "o", "o", "sh", "o"],
            ),
            (
                "々々",
                vec!["y", "o", "o", "sh", "o", "y", "o", "o", "sh", "o"],
            ),
            ("々", vec!["y", "o", "o", "sh", "o"]),
            ("槇野々", vec!["m", "a", "k", "i", "n", "o", "n", "o"]),
            ("々々", vec!["n", "o", "n", "o"]),
            ("散々", vec!["s", "a", "Nd", "z", "a", "Nd"]),
            ("々", vec!["z", "a", "Nd"]),
            ("二", vec!["n", "i"]),
            ("千", vec!["s", "e", "Nd"]),
            ("十", vec!["j", "u", "u"]),
            ("八", vec!["h", "a", "ch", "i"]),
            ("\u{3000}", vec!["sp"]),
            (
                "Ｏｐｅｎ　ＪＴａｌｋ",
                vec![
                    "o", "o", "p", "u", "Nd", "j", "e", "e", "t", "o", "o", "k", "u",
                ],
            ),
            ("\u{3000}", vec!["sp"]),
            ("一", vec!["i", "clt"]),
            ("．", vec!["t", "e", "N"]),
            ("一", vec!["i", "ch", "i"]),
            ("一", vec!["i", "ch", "i"]),
            ("！", vec!["pau"]),
            ("？", vec!["pau"]),
            ("！", vec!["pau"]),
            ("？", vec!["pau"]),
            ("！", vec!["pau"]),
            ("？", vec!["pau"]),
            ("！", vec!["pau"]),
            ("？", vec!["pau"]),
            ("ー", vec!["unk"]),
            ("！", vec!["pau"]),
            ("￥", vec!["e", "Nq"]),
            ("／", vec!["pau"]),
            ("？", vec!["pau"]),
            ("ー", vec!["unk"]),
            ("！", vec!["pau"]),
            ("？", vec!["pau"]),
            ("！", vec!["pau"]),
            ("？", vec!["pau"]),
            ("ＣＰＵ", vec!["sh", "i", "i", "p", "i", "i", "y", "u", "u"]),
            ("\u{3000}", vec!["sp"]),
            ("ｉｔ", vec!["i", "clt", "t", "o"]),
            ("\u{3000}", vec!["sp"]),
            ("Ｉｔ", vec!["i", "clt", "t", "o"]),
            ("\u{3000}", vec!["sp"]),
            ("ＩＴ", vec!["a", "i", "t", "i", "i"]),
            ("\u{3000}", vec!["sp"]),
            ("ああ", vec!["a", "a"]),
            ("\u{3000}", vec!["sp"]),
            ("ａａー", vec!["a", "a", "a"]),
            ("\u{3000}", vec!["sp"]),
            ("ａｌｌ", vec!["o", "o", "r", "u"]),
            ("あ", vec!["a"]),
            ("\u{3000}", vec!["sp"]),
            ("ｈａｑｕｍｅｉ", vec!["h", "a", "k", "u", "m", "e", "i"]),
            ("\u{3000}", vec!["sp"]),
            ("ｇ", vec!["j", "i", "i"]),
            ("２", vec!["ts", "u", "u"]),
            ("ｐ", vec!["p", "i", "i"]),
        ];

        let result = haqumei.g2p_mapping(NIGHTMARE_TEXT).unwrap();
        let result: Vec<(&str, Vec<&str>)> = result
            .iter()
            .map(|d| {
                (
                    d.word.as_str(),
                    d.phonemes.iter().map(|s| s.as_str()).collect(),
                )
            })
            .collect();

        assert_eq!(result, expected);
    }

    #[test]
    fn test_mapping_flags() {
        let waganeko = fs::read_to_string(WAGANEKO_PATH.as_path()).unwrap();
        let waganeko: Vec<&str> = waganeko.lines().collect();

        let mut haqumei = Haqumei::new().unwrap();
        let mut open_jtalk = OpenJTalk::new().unwrap();
        let result_hq = haqumei.g2p_mapping_batch(&waganeko).unwrap();
        let result_ojt = open_jtalk.g2p_mapping_batch(&waganeko).unwrap();

        for (details_hq, details_ojt) in result_hq.into_iter().zip(result_ojt) {
            for (detail_hq, detail_ojt) in details_hq.clone().into_iter().zip(details_ojt) {
                // `is_ignored` であるとき、空白として追加された sp か、NJDが割り当てなかった場合である
                if detail_hq.is_ignored {
                    assert!(detail_hq.phonemes == [Phoneme::Sp] || detail_hq.phonemes.is_empty());
                }
                if detail_hq.is_unknown {
                    // 未知語の場合：
                    // 「unk」であるか、あるいは Open JTalk が推論した音素が入っているはず。
                    // 少なくとも空配列や、フォールバックされただけの pau であってはならない。
                    //
                    // また、先頭一致のマッピング処理において `is_unknown` と異なって `is_ignored` は伝播しない。
                    // なので、`is_unknown` かつ `is_ignored` である際に、`unk` に置き換える処理が正当化される。
                    assert!(
                        detail_hq.phonemes == [Phoneme::Unk]
                            || (!detail_hq.phonemes.is_empty()
                                && detail_hq.phonemes != [Phoneme::Pau]),
                        "Unknown word {:?} has unexpected phonemes: {:?}",
                        detail_hq.word,
                        detail_hq.phonemes
                    );

                    // 少なくとも、`is_ignored` かつ 未知語であれば、
                    // `unk` でなければならない。
                    //
                    // (detailed API で音響モデルが未知語を pau として受け取ると精度にもよくないため)
                    if detail_hq.is_ignored {
                        assert_eq!(detail_hq.phonemes, [Phoneme::Unk]);
                    }
                }
                // is_ignored であるとき、空白として追加された sp か、NJDが割り当てなかった場合である
                if detail_ojt.is_ignored {
                    assert!(detail_ojt.phonemes == [Phoneme::Sp] || detail_ojt.phonemes.is_empty());
                }
                if detail_ojt.is_unknown {
                    // 未知語の場合：
                    // 「unk」であるか、あるいは OpenJTalk が推論した音素が入っているはず。
                    // 少なくとも空配列や、フォールバックされただけの pau であってはならない。

                    // また、先頭一致のマッピング処理において `is_unknown` と異なって `is_ignored` は伝播しない。
                    // なので、`is_unknown` かつ `is_ignored` である際に、`unk` に置き換える処理が正当化される。
                    assert!(
                        detail_ojt.phonemes == [Phoneme::Unk]
                            || (!detail_ojt.phonemes.is_empty()
                                && detail_ojt.phonemes != [Phoneme::Pau]),
                        "Unknown word {:?} has unexpected phonemes: {:?}",
                        detail_ojt.word,
                        detail_ojt.phonemes
                    );

                    // 少なくとも、`is_ignored` かつ 未知語であれば、
                    // `unk` でなければならない。
                    if detail_ojt.is_ignored {
                        assert_eq!(detail_ojt.phonemes, [Phoneme::Unk]);
                    }
                }
            }
        }
    }

    #[test]
    fn test_mapping_complex_punctuation() {
        let mut haqumei = Haqumei::new().unwrap();
        let text = "「東京」、大阪」…、…あ";
        let mapping = haqumei.g2p_mapping(text).unwrap();

        let kagi_close_1 = mapping.iter().find(|m| m.word == "」").unwrap();
        let touten_1 = mapping.iter().find(|m| m.word == "、").unwrap();

        // 括弧類はデフォルト(default_is_non_pause_symbol) では pau が割り当てられない
        assert_eq!(kagi_close_1.phonemes, &[] as &[String]);
        assert_eq!(touten_1.phonemes, vec![Phoneme::Pau]);

        let osaka_idx = mapping.iter().position(|m| m.word == "大阪").unwrap();

        assert_eq!(mapping[osaka_idx + 1].word, "」");
        assert_eq!(mapping[osaka_idx + 1].phonemes, &[] as &[String]);

        // `…` は pause のように機能する記号である気がするし、
        // これをフィルタリングするのは G2P の責務ではない
        assert_eq!(mapping[osaka_idx + 2].word, "…");
        assert_eq!(mapping[osaka_idx + 2].phonemes, &[Phoneme::Pau]);

        assert_eq!(mapping[osaka_idx + 3].word, "、");
        assert_eq!(mapping[osaka_idx + 3].phonemes, &[Phoneme::Pau]);

        assert_eq!(mapping[osaka_idx + 4].word, "…");
        assert_eq!(mapping[osaka_idx + 4].phonemes, &[Phoneme::Pau]);
    }

    #[test]
    fn test_mapping_integrity() {
        let mut haqumei = Haqumei::new().unwrap();
        let text = "吾輩は猫である。名前　はまだ無　い。𰻞𰻞麺を、　食べたい。";

        let result = haqumei.g2p_mapping(text).unwrap();

        let reconstructed: String = result.iter().map(|d| d.word.as_str()).collect();

        assert_eq!(text, reconstructed);
    }

    #[test]
    fn test_mapping_merged_internal_spaces() {
        let mut haqumei = Haqumei::new().unwrap();
        let text = "なる　長老ー　ー　に";

        let mapping = haqumei.g2p_mapping(text).unwrap();

        // 期待される順序:
        // 0: なる
        // 1: sp (sp1)
        // 2: 長老ーー (結合された単語)
        // 3: sp (sp2 - 単語の中にあった空白)
        // 4: sp (sp3 - 単語の後にあった空白)
        // 5: に

        assert_eq!(mapping[0].word, "なる");
        assert_eq!(mapping[1].word, "\u{3000}"); // sp1
        assert!(mapping[1].is_ignored);

        assert_eq!(mapping[2].word, "長老ーー"); // 結合語
        assert_eq!(mapping[2].phonemes.len(), 8); // ch o o r o o o o (長音2つ分)

        assert_eq!(mapping[3].word, "\u{3000}"); // sp2 (内部にあった空白が後に回る)
        assert!(mapping[3].is_ignored);

        assert_eq!(mapping[4].word, "\u{3000}"); // sp3
        assert!(mapping[4].is_ignored);

        assert_eq!(mapping[5].word, "に");
    }

    /// 3つ以上の結合 + 複数の内部空白
    #[test]
    fn test_mapping_triple_merge_with_multiple_spaces() {
        let mut haqumei = Haqumei::new().unwrap();
        let text = "あー　ー　ー";

        let mapping = haqumei.g2p_mapping(text).unwrap();

        // 期待される順序:
        // 0: あーーー (すべて結合)
        // 1: sp (sp1)
        // 2: sp (sp2)

        assert_eq!(mapping[0].word, "あーーー");
        assert_eq!(mapping[1].word, "\u{3000}");
        assert_eq!(mapping[2].word, "\u{3000}");
        assert_eq!(mapping.len(), 3);
    }

    /// 未知語 + 空白 + 長音 (未知語は音素がないため、長音を吸収せず分離される)
    #[test]
    fn test_mapping_unknown_merged_with_space() {
        let mut haqumei = Haqumei::new().unwrap();
        let text = "𰻞　ー";

        let mapping = haqumei.g2p_mapping(text).unwrap();

        // 期待される結果:
        // 0: 𰻞 (unk)
        // 1: sp (sp1)
        // 2: ー (unk)

        assert_eq!(mapping[0].word, "𰻞");
        assert_eq!(mapping[0].phonemes, vec!["unk".to_string()]);

        assert_eq!(mapping[1].word, "\u{3000}");

        assert_eq!(mapping[2].word, "ー");
        assert_eq!(mapping[2].phonemes, vec!["unk".to_string()]);

        assert_eq!(mapping.len(), 3);
    }

    /// 文頭・文末の空白がマージに巻き込まれないことの確認
    #[test]
    fn test_mapping_merged_word_boundary_spaces() {
        let mut haqumei = Haqumei::new().unwrap();
        let text = "　あーー　";

        let mapping = haqumei.g2p_mapping(text).unwrap();

        // 0: sp (sp1)
        // 1: あーー
        // 2: sp (sp2)

        assert_eq!(mapping[0].word, "\u{3000}");
        assert_eq!(mapping[1].word, "あーー");
        assert_eq!(mapping[2].word, "\u{3000}");
    }

    /// 小文字の "ｉｔ" は辞書にないため Mecab で "ｉ" と "ｔ" に分かれ、
    /// `predict_kana_english` によってマージされる。
    /// このとき、先頭一致 (starts_with) 処理が直前の空白 "\u{3000}" を
    /// 単語の後ろへ誤配置しないことを確認する。
    #[test]
    fn test_mapping_space_before_merged_alphabet() {
        let mut haqumei = Haqumei::new().unwrap();
        let text = "\u{3000}ｉｔ";

        let result = haqumei.g2p_mapping(text).unwrap();
        let result: Vec<(&str, Vec<&str>)> = result
            .iter()
            .map(|d| {
                (
                    d.word.as_str(),
                    d.phonemes.iter().map(|s| s.as_str()).collect(),
                )
            })
            .collect();

        let expected = vec![
            ("\u{3000}", vec!["sp"]),
            ("ｉｔ", vec!["i", "cl", "t", "o"]),
        ];

        assert_eq!(result, expected);
    }

    /// "２0" は数字展開によって "二" と "十" に増える。(要素数増)
    /// "ｉｔ" は `predict_kana_english` によってマージされ 1つに減る。(要素数減)
    #[test]
    fn test_mapping_digit_adjacent_to_merged_alphabet() {
        let mut haqumei = Haqumei::new().unwrap();
        let text = "２0ｉｔ";

        let result = haqumei.g2p_mapping(text).unwrap();
        let result: Vec<(&str, Vec<&str>)> = result
            .iter()
            .map(|d| {
                (
                    d.word.as_str(),
                    d.phonemes.iter().map(|s| s.as_str()).collect(),
                )
            })
            .collect();

        let expected = vec![
            ("二", vec!["n", "i"]),
            ("十", vec!["j", "u", "u"]),
            ("ｉｔ", vec!["i", "cl", "t", "o"]),
        ];

        assert_eq!(result, expected);
    }

    #[test]
    fn test_mapping_tricky_merges_mixed() {
        let mut haqumei = Haqumei::new().unwrap();
        let text = "２0\u{3000}ｉｔ\u{3000}日々";

        let result = haqumei.g2p_mapping(text).unwrap();
        let result: Vec<(&str, Vec<&str>)> = result
            .iter()
            .map(|d| {
                (
                    d.word.as_str(),
                    d.phonemes.iter().map(|s| s.as_str()).collect(),
                )
            })
            .collect();

        let expected = vec![
            ("二", vec!["n", "i"]),
            ("十", vec!["j", "u", "u"]),
            ("\u{3000}", vec!["sp"]),
            ("ｉｔ", vec!["i", "cl", "t", "o"]),
            ("\u{3000}", vec!["sp"]),
            ("日々", vec!["h", "i", "b", "i"]),
        ];

        assert_eq!(result, expected);
    }

    // thanks と g が結合しないことを確認する
    #[test]
    fn test_mapping_thanks_g2p() {
        let mut haqumei = Haqumei::new().unwrap();
        let text = "thanks g2p";

        let result = haqumei.g2p_mapping(text).unwrap();
        let result: Vec<(&str, Vec<&str>)> = result
            .iter()
            .map(|d| {
                (
                    d.word.as_str(),
                    d.phonemes.iter().map(|s| s.as_str()).collect(),
                )
            })
            .collect();

        let expected = vec![
            ("ｔｈａｎｋｓ", vec!["s", "a", "N", "k", "u", "s", "u"]),
            ("\u{3000}", vec!["sp"]),
            ("ｇ", vec!["j", "i", "i"]),
            ("２", vec!["ts", "u", "u"]),
            ("ｐ", vec!["p", "i", "i"]),
        ];

        assert_eq!(result, expected);
    }

    #[test]
    fn test_mapping_english_two_english() {
        let mut haqumei = Haqumei::new().unwrap();

        let text1 = "g2p";
        let result1 = haqumei.g2p_mapping(text1).unwrap();
        let mapped1: Vec<(&str, Vec<&str>)> = result1
            .iter()
            .map(|d| {
                (
                    d.word.as_str(),
                    d.phonemes.iter().map(|s| s.as_str()).collect(),
                )
            })
            .collect();

        assert_eq!(
            mapped1,
            vec![
                ("ｇ", vec!["j", "i", "i"]),
                ("２", vec!["ts", "u", "u"]),
                ("ｐ", vec!["p", "i", "i"]),
            ]
        );

        let text2 = "text2video image2 text";
        let result2 = haqumei.g2p_mapping(text2).unwrap();
        let mapped2: Vec<(&str, Vec<&str>)> = result2
            .iter()
            .map(|d| {
                (
                    d.word.as_str(),
                    d.phonemes.iter().map(|s| s.as_str()).collect(),
                )
            })
            .collect();

        assert_eq!(
            mapped2,
            vec![
                ("ｔｅｘｔ", vec!["t", "e", "k", "I", "s", "u", "t", "o"]),
                ("２", vec!["t", "u", "u"]),
                ("ｖｉｄｅｏ", vec!["b", "i", "d", "e", "o"]),
                ("\u{3000}", vec!["sp"]),
                ("ｉｍａｇｅ", vec!["i", "m", "a", "a", "j", "u"]),
                ("二", vec!["n", "i"]),
                ("\u{3000}", vec!["sp"]),
                ("ｔｅｘｔ", vec!["t", "e", "k", "I", "s", "u", "t", "o"]),
            ]
        );
    }

    /// 数字の縮約が空白をまたぐとき、その空白が mapping から消えないこと。
    ///
    /// 「1　0」の morph は １ / 　 / ０ で、NJD はこれを「十」1 つに縮約する。
    /// 数字ブロックの内側にある空白は、外側にあるとき (「２0　ｉｔ」) と違って
    /// ループ先頭の ignored 回収に拾われないため、取りこぼしていた。
    #[test]
    fn test_mapping_digit_contraction_keeps_inner_space() {
        let mut haqumei = Haqumei::new().unwrap();

        for (text, expected) in [
            ("1　0", vec![("十", vec!["j", "u", "u"]), ("\u{3000}", vec!["sp"])]),
            (
                "1　00",
                vec![("百", vec!["hy", "a", "k", "u"]), ("\u{3000}", vec!["sp"])],
            ),
            (
                "1　0円",
                vec![
                    ("十", vec!["j", "u", "u"]),
                    ("\u{3000}", vec!["sp"]),
                    ("円", vec!["e", "N"]),
                ],
            ),
            // 数字ブロックの外側にある空白は元から拾えていた (退行防止)
            (
                "２0　ｉｔ",
                vec![
                    ("二", vec!["n", "i"]),
                    ("十", vec!["j", "u", "u"]),
                    ("\u{3000}", vec!["sp"]),
                    ("ｉｔ", vec!["i", "cl", "t", "o"]),
                ],
            ),
        ] {
            let mapping = haqumei.g2p_mapping(text).unwrap();
            let actual: Vec<(&str, Vec<&str>)> = mapping
                .iter()
                .map(|d| {
                    (
                        d.word.as_str(),
                        d.phonemes.iter().map(|s| s.as_str()).collect(),
                    )
                })
                .collect();
            assert_eq!(actual, expected, "入力: {text}");
        }
    }

}
