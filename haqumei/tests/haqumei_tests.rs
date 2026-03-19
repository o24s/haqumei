#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::LazyLock,
    };

    use haqumei::{Haqumei, HaqumeiOptions, OpenJTalk, UnicodeNormalization, errors::HaqumeiError};
    use unicode_normalization::UnicodeNormalization as _;

    static MANIFEST_DIR: LazyLock<&Path> = LazyLock::new(|| Path::new(env!("CARGO_MANIFEST_DIR")));
    static WAGANEKO_PATH: LazyLock<PathBuf> =
        LazyLock::new(|| MANIFEST_DIR.join("../resources/waganeko.txt"));

    #[test]
    fn test_g2p_basic() {
        let mut haqumei = Haqumei::new().unwrap();
        let text = "こんにちは";
        let phonemes = haqumei.g2p(text).unwrap();

        assert_eq!(phonemes, vec!["k", "o", "N", "n", "i", "ch", "i", "w", "a"]);
    }

    /// 空文字列を渡してもクラッシュせず、空の結果が返ることを確認
    #[test]
    fn test_empty_string() {
        let mut haqumei = Haqumei::new().unwrap();
        let text = "";
        let result = haqumei.g2p(text).unwrap();
        assert!(result.is_empty());

        let mapping = haqumei.g2p_mapping(text).unwrap();
        assert!(mapping.is_empty());
    }

    /// NULL文字が含まれる入力でエラーになり、クラッシュしないこと
    #[test]
    fn test_null_byte_injection() {
        let mut haqumei = Haqumei::new().unwrap();
        let text = "こん\0にちは";

        let result = haqumei.g2p(text);
        assert!(result.is_err());

        match result.unwrap_err() {
            HaqumeiError::InteriorNulError { bytes, pos } => {
                assert_eq!(
                    bytes,
                    vec![
                        227, 129, 147, // こ
                        227, 130, 147, // ん
                        0,   // \0 (NUL)
                        227, 129, 171, // に
                        227, 129, 161, // ち
                        227, 129, 175, // は
                    ]
                );

                assert_eq!(pos, 6)
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_huge_input_range_error() {
        let mut haqumei = Haqumei::new().unwrap();
        // BUFFER_SIZE (16384) を超える入力を生成
        let huge_text = "あ".repeat(20000);

        let result = haqumei.g2p(&huge_text);

        assert!(result.is_err());
        match result.unwrap_err() {
            HaqumeiError::Text2MecabError(msg) => {
                assert!(msg.contains("too long"));
            }
            err => panic!("Unexpected error type: {:?}", err),
        }
    }

    #[test]
    fn test_recovery_from_error() {
        let mut haqumei = Haqumei::new().unwrap();

        let _ = haqumei.g2p("悪い\0Input");

        let text = "復帰";
        let result = haqumei.g2p(text);
        assert!(result.is_ok());
    }

    #[test]
    fn test_unknown_word_mapping() {
        let mut haqumei = Haqumei::new().unwrap();
        let text = "𰻞𰻞麺";

        let mapping = haqumei.g2p_mapping(text).unwrap();

        // "𰻞𰻞" -> unk, is_unknown: true
        // "麺"   -> m e N, is_unknown: false

        assert_eq!(mapping[0].word, "𰻞𰻞");
        assert_eq!(mapping[0].phonemes, vec!["unk".to_string()]);
        assert!(mapping[0].is_unknown);

        assert_eq!(mapping[1].word, "麺");
        assert_eq!(mapping[1].phonemes, vec!["m", "e", "N"]);
        assert!(!mapping[1].is_unknown);
    }

    #[test]
    fn test_punctuation_pause() {
        let mut haqumei = Haqumei::new().unwrap();
        let text = "あ、あ。";

        let mapping = haqumei.g2p_mapping(text).unwrap();

        let pauses: Vec<_> = mapping
            .iter()
            .filter(|m| m.phonemes.contains(&"pau".to_string()))
            .collect();

        assert!(!pauses.is_empty());
    }

    #[test]
    fn test_symbols_and_control_chars() {
        let mut haqumei = Haqumei::new().unwrap();
        let text = "#$%&'()\n\t";

        let result = haqumei.g2p(text);
        assert!(result.is_ok());
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
    fn test_unicode_normalization() {
        let mut haqumei = Haqumei::with_options(HaqumeiOptions {
            normalize_unicode: UnicodeNormalization::Nfc,
            ..Default::default()
        })
        .unwrap();
        let text = &[
            "\u{304B}\u{3099}",         // が
            "\u{306F}\u{309A}",         // ぱ
            "\u{30B3}\u{3099}",         // ゴ
            "\u{0065}\u{0301}",         // é
            "\u{1112}\u{1161}\u{11AB}", // 한
        ];

        let results = haqumei.g2p_mapping_batch(text).unwrap();

        let results: Vec<String> = results
            .iter()
            .map(|v| v.iter().map(|d| d.word.as_str()).collect::<String>())
            .collect();

        for (result, text) in results.iter().zip(text) {
            let expected: String = text.nfc().collect();
            assert_eq!(&expected, result);
            assert_eq!(result.nfc().collect::<String>(), *result);
        }
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
                // 先頭の長音記号などは、未知語かつ無視される対象であるが、
                // 未知語でない無視されるトークンはおそらく空白のみである
                if detail_hq.is_ignored && !detail_hq.is_unknown {
                    assert_eq!(detail_hq.phonemes, &["sp"]);
                }
                if detail_hq.is_unknown {
                    // 未知語の場合：
                    // 「unk」であるか、あるいは OpenJTalk が推論した音素が入っているはず。
                    // 少なくとも空配列や、フォールバックされただけの pau であってはならない。
                    assert!(
                        detail_hq.phonemes == ["unk"]
                            || (!detail_hq.phonemes.is_empty() && detail_hq.phonemes != ["pau"]),
                        "Unknown word {:?} has unexpected phonemes: {:?}",
                        detail_hq.word,
                        detail_hq.phonemes
                    );
                }
                // 先頭の長音記号などは、未知語かつ無視される対象であるが、
                // 未知語でない無視されるトークンはおそらく空白のみである
                if detail_ojt.is_ignored && !detail_ojt.is_unknown {
                    assert_eq!(detail_hq.phonemes, &["sp"]);
                }
                if detail_ojt.is_unknown {
                    // 未知語の場合：
                    // 「unk」であるか、あるいは OpenJTalk が推論した音素が入っているはず。
                    // 少なくとも空配列や、フォールバックされただけの pau であってはならない。
                    assert!(
                        detail_ojt.phonemes == ["unk"]
                            || (!detail_ojt.phonemes.is_empty() && detail_ojt.phonemes != ["pau"]),
                        "Unknown word {:?} has unexpected phonemes: {:?}",
                        detail_ojt.word,
                        detail_ojt.phonemes
                    );
                }
            }
        }
    }

    #[test]
    fn test_mapping_nightmare_case() {
        let mut haqumei = Haqumei::new().unwrap();
        let text = "\
つまみ出されようとしたが、「「八十五歳」」にもなる 長老 に助けられた。\
わーいです。そこで、𰻞𰻞麺とお冷を飲み食いしたです。\
ーっ、 𰻞ー𰻞。あ、はい。あーーーーーーーーあ\
叙々々々々々々苑々々様々々要所々々々々々槇野々々々\
";

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
            ("「", vec!["pau"]),
            ("「", vec!["pau"]),
            ("八", vec!["h", "a", "ch", "i"]),
            ("十", vec!["j", "u", "u"]),
            ("五", vec!["g", "o"]),
            ("歳", vec!["s", "a", "i"]),
            ("」", vec!["pau"]),
            ("」", vec!["pau"]),
            ("に", vec!["n", "i"]),
            ("も", vec!["m", "o"]),
            ("なる", vec!["n", "a", "r", "u"]),
            ("\u{3000}", vec!["sp"]),
            ("長老", vec!["ch", "o", "o", "r", "o", "o"]),
            ("\u{3000}", vec!["sp"]),
            ("に", vec!["n", "i"]),
            ("助け", vec!["t", "a", "s", "U", "k", "e"]),
            ("られ", vec!["r", "a", "r", "e"]),
            ("た", vec!["t", "a"]),
            ("。", vec!["pau"]),
            ("わーい", vec!["w", "a", "a", "i"]),
            ("です", vec!["d", "e", "s", "U"]),
            ("。", vec!["pau"]),
            ("そこで", vec!["s", "o", "k", "o", "d", "e"]),
            ("、", vec!["pau"]),
            ("𰻞𰻞", vec!["unk"]),
            ("麺", vec!["m", "e", "N"]),
            ("と", vec!["t", "o"]),
            ("お冷", vec!["o", "h", "i", "y", "a"]),
            ("を", vec!["o"]),
            ("飲み", vec!["n", "o", "m", "i"]),
            ("食い", vec!["g", "u", "i"]),
            ("し", vec!["sh", "I"]),
            ("た", vec!["t", "a"]),
            ("です", vec!["d", "e", "s", "U"]),
            ("。", vec!["pau"]),
            ("ー", vec!["unk"]),
            ("っ", vec!["cl"]),
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
        ];

        assert_eq!(result, expected);
    }

    #[test]
    fn test_g2p_kana_revert_long_vowels() {
        let text = "人生は効果的。";

        let mut haqumei = Haqumei::new().unwrap();
        let kana_default = haqumei.g2p_kana(text).unwrap();
        assert!(kana_default.contains("セー"));
        assert!(kana_default.contains("コーカ"));
        assert!(kana_default.contains("ワ")); // 助詞は「ワ」

        let mut haqumei_revert = Haqumei::with_options(HaqumeiOptions {
            revert_long_vowels: true,
            ..Default::default()
        })
        .unwrap();
        let kana_revert = haqumei_revert.g2p_kana(text).unwrap();

        assert!(kana_revert.contains("セイ"));
        assert!(kana_revert.contains("コウカ"));
        assert!(kana_revert.contains("ワ")); // 助詞の「ワ」は維持されていること
    }

    #[test]
    fn test_g2p_kana_revert_yotsugana() {
        let text = "鼻血に気づかず。";

        let mut haqumei = Haqumei::new().unwrap();
        let kana_default = haqumei.g2p_kana(text).unwrap();
        assert!(kana_default.contains("ハナジ"));
        assert!(kana_default.contains("キズカズ"));

        let mut haqumei_revert = Haqumei::with_options(HaqumeiOptions {
            revert_yotsugana: true,
            ..Default::default()
        })
        .unwrap();
        let kana_revert = haqumei_revert.g2p_kana(text).unwrap();

        assert!(kana_revert.contains("ハナヂ"));
        assert!(kana_revert.contains("キヅカズ"));
    }

    #[test]
    fn test_g2p_kana_use_read_as_pron() {
        let text = "こんにちは、人生。";

        let mut haqumei_default = Haqumei::new().unwrap();
        let kana_default = haqumei_default.g2p_kana(text).unwrap();
        assert!(kana_default.contains("コンニチワ")); // 助詞は「ワ」
        assert!(kana_default.contains("ジンセー")); // 長音化

        let mut haqumei_read = Haqumei::with_options(HaqumeiOptions {
            use_read_as_pron: true,
            ..Default::default()
        })
        .unwrap();
        let kana_read = haqumei_read.g2p_kana(text).unwrap();

        assert!(kana_read.contains("コンニチハ")); // 助詞が「ハ」のまま
        assert!(kana_read.contains("ジンセイ")); // 長音が「セイ」のまま
    }

    #[test]
    fn test_g2p_kana_combined_selective() {
        let text = "人生は、鼻血に気づかず。";

        // 全てを組み合わせて、助詞の「は」だけは「ワ」のままで、
        // 長音化や四つ仮名だけを直したいケース
        let mut haqumei = Haqumei::with_options(HaqumeiOptions {
            revert_long_vowels: true,
            revert_yotsugana: true,
            ..Default::default()
        })
        .unwrap();
        let kana = haqumei.g2p_kana(text).unwrap();

        assert!(kana.contains("ジンセイ")); // 復元
        assert!(kana.contains("ワ")); // 助詞維持
        assert!(kana.contains("ハナヂ")); // 復元
        assert!(kana.contains("キヅカズ")); // 復元
    }

    #[test]
    fn test_odoriji_basic_expansion() {
        let mut haqumei = Haqumei::new().unwrap();

        assert_eq!(haqumei.g2p_kana("さゝみ").unwrap(), "ササミ");

        assert_eq!(haqumei.g2p_kana("いすゞ").unwrap(), "イスズ");

        assert_eq!(haqumei.g2p_kana("カヽ").unwrap(), "カカ");

        assert_eq!(haqumei.g2p_kana("ガヾ").unwrap(), "ガガ");
    }

    #[test]
    fn test_odoriji_voiceless_conversion() {
        let mut haqumei = Haqumei::new().unwrap();

        // 濁音の後に清音の踊り字が来た場合、清音化されるべき
        // 「がゝ」 -> 「ガカ」
        assert_eq!(haqumei.g2p_kana("がゝ").unwrap(), "ガカ");
        assert_eq!(haqumei.g2p_kana("バヽ").unwrap(), "バハ");
    }

    #[test]
    fn test_odoriji_voiced_conversion() {
        let mut haqumei = Haqumei::new().unwrap();

        // 清音の後に濁音の踊り字が来た場合、濁音化されるべき
        // 「かゞ」 -> 「カガ」
        assert_eq!(haqumei.g2p_kana("かゞ").unwrap(), "カガ");
        assert_eq!(haqumei.g2p_kana("ハヾ").unwrap(), "ハバ");
    }

    #[test]
    fn test_odoriji_mora_handling_with_small_kana() {
        let mut haqumei = Haqumei::new().unwrap();

        // モーラを伴う繰り返し (本来拗音を含むモーラに一の字点がくることは望まれないが)
        // 「じょゝ」 -> 「ジョジョ」
        assert_eq!(haqumei.g2p_kana("じょゝ").unwrap(), "ジョジョ");

        // 「ちゅゞ」 -> 「チュヂュ」 (チ+濁点+ュ)
        let result = haqumei.g2p_kana("ちゅゞ").unwrap();
        assert_eq!(result, "チュヂュ");
    }

    #[test]
    fn test_odoriji_pos_change() {
        let mut haqumei = Haqumei::new().unwrap();

        let mut detailed = haqumei.g2p_detailed("いすゞ").unwrap();
        assert_eq!(detailed.pop().unwrap(), "u");
        assert_eq!(detailed.pop().unwrap(), "z");

        let mapping = haqumei.g2p_mapping("いすゞ").unwrap();
        let odoriji_word = mapping.iter().find(|m| m.word.contains("ゞ")).unwrap();

        assert_eq!(odoriji_word.phonemes, ["i", "s", "u", "z", "u"]);
    }

    #[test]
    fn test_odoriji_invalid_cases() {
        let mut haqumei = Haqumei::new().unwrap();

        let result = haqumei.g2p_kana("ゝ").unwrap();
        assert_eq!(result, "ゝ");

        // 半濁点がついた不正な踊り字（ゝ+゜）
        // 濁音とはみなされず、清音として処理されること
        assert_eq!(haqumei.g2p_kana("かゝ゜").unwrap(), "カカ゜");
    }

    #[test]
    fn test_dounojiten_expansion() {
        let mut haqumei = Haqumei::new().unwrap();

        let text = "叙々々々々々々苑々々様々々要所々々々々々槇野々々々";

        let result = haqumei.g2p_mapping(text).unwrap();

        let mapping: Vec<(&str, Vec<&str>)> = result
            .iter()
            .map(|d| {
                (
                    d.word.as_str(),
                    d.phonemes.iter().map(|s| s.as_str()).collect(),
                )
            })
            .collect();

        let expected = vec![
            ("叙", vec!["j", "o"]),
            (
                "々々々々々々",
                vec!["j", "o", "j", "o", "j", "o", "j", "o", "j", "o", "j", "o"],
            ),
            // 漢字を跨いだ後の展開: 「苑」を「々々」が繰り返す
            ("苑", vec!["e", "N"]),
            ("々々", vec!["e", "N", "e", "N"]),
            // 様々からの抽出: 「様々」の後半(ザマ)だけを「々」が繰り返す
            ("様々", vec!["s", "a", "m", "a", "z", "a", "m", "a"]),
            ("々", vec!["z", "a", "m", "a"]),
            // 複数文字熟語の連鎖: 「要所々々」の展開結果を、さらに「々々」「々」が引き継ぐ
            (
                "要所々々",
                vec!["y", "o", "o", "sh", "o", "y", "o", "o", "sh", "o"],
            ),
            (
                "々々",
                vec!["y", "o", "o", "sh", "o", "y", "o", "o", "sh", "o"],
            ),
            ("々", vec!["y", "o", "o", "sh", "o"]),
            // 固有名詞的な末尾からの抽出: 「槇野々」の末尾(ノノ)を「々々」が引き継ぐ
            ("槇野々", vec!["m", "a", "k", "i", "n", "o", "n", "o"]),
            ("々々", vec!["n", "o", "n", "o"]),
        ];

        assert_eq!(mapping, expected);
    }

    #[test]
    fn test_u_long_vowel_revert() {
        let mut haqumei = Haqumei::new().unwrap();

        let cases = vec![
            // イ段 + う (シナジー, イミジー化を防ぐ)
            ("しなじう", vec!["sh", "i", "n", "a", "j", "i", "u"]),
            ("いみじう", vec!["i", "m", "i", "j", "i", "u"]),
            // オ段 + う (正当な長音化: これは「ー」のままでなければならない)
            ("行こう", vec!["i", "k", "o", "o"]), // i k o:
            ("言おう", vec!["i", "o", "o"]),      // i o:
            // ア段 + う (古語的・方言的な「～わう」など: 「ワー」化を防ぐ)
            ("買わう", vec!["k", "a", "w", "a", "u"]),
            // エ段 + う (古語的な助動詞などの連結: 「エー」化を防ぐ)
            ("捨てう", vec!["s", "U", "t", "e", "u"]),
        ];

        for (text, expected_phonemes) in cases {
            let result = haqumei.g2p_mapping(text).unwrap();

            let actual_phonemes: Vec<&str> = result
                .iter()
                .flat_map(|d| d.phonemes.iter().map(|s| s.as_str()))
                .collect();

            assert_eq!(
                actual_phonemes, expected_phonemes,
                "Failed at text: {}",
                text
            );
        }
    }
}
