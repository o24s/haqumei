#[cfg(test)]
mod tests {
    use haqumei::{
        Haqumei, HaqumeiOptions, IuPronunciation, UnicodeNormalization, errors::HaqumeiError,
        phoneme::Phoneme, utils::default_is_non_pause_symbol,
    };
    use unicode_normalization::UnicodeNormalization as _;

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
            .filter(|m| m.phonemes.contains(&Phoneme::Pau))
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
    fn test_custom_pause_symbol_config() {
        let mut haqumei = Haqumei::new().unwrap();
        let text = "「あ」";

        // デフォルトでは「default_is_non_pause_symbol」が使用され、
        // 括弧類には pau が割り当てられません。
        let mapping_default = haqumei.g2p_mapping(text).unwrap();

        assert_eq!(mapping_default[0].word, "「");
        assert!(mapping_default[0].phonemes.is_empty(),);
        assert!(mapping_default[0].is_ignored);

        // 「開き括弧『「』だけはポーズとして扱いたい」という独自のルールを作成
        fn my_custom_pause_rule(s: &str) -> bool {
            if s == "「" {
                return false; // false を返すと pau が割り当てられる
            }
            // それ以外はデフォルトの挙動を継承
            default_is_non_pause_symbol(s)
        }

        haqumei.options.is_non_pause_symbol = my_custom_pause_rule;

        let mapping_custom = haqumei.g2p_mapping(text).unwrap();

        // 開き括弧 「
        assert_eq!(mapping_custom[0].word, "「");
        assert_eq!(
            mapping_custom[0].phonemes,
            vec!["pau".to_string()],
            "カスタム関数により開き括弧に pau が付与されていること"
        );
        assert!(
            !mapping_custom[0].is_ignored,
            "pau があるので ignored ではなくなる"
        );

        // 中身の 「あ」
        assert_eq!(mapping_custom[1].word, "あ");
        assert_eq!(mapping_custom[1].phonemes, vec!["a".to_string()]);

        // 閉じ括弧 」
        assert_eq!(mapping_custom[2].word, "」");
        assert!(
            mapping_custom[2].phonemes.is_empty(),
            "閉じ括弧はカスタム関数でも true (non-pause) を返すため、空のままであること"
        );
        assert!(mapping_custom[2].is_ignored);
    }

    #[test]
    fn test_all_pause_denied_config() {
        // すべての記号に対して true (non-pause) を返す極端な設定
        let mut haqumei = Haqumei::with_options(HaqumeiOptions {
            is_non_pause_symbol: |_| true,
            ..Default::default()
        })
        .unwrap();
        let text = "あ、い。";

        let mapping = haqumei.g2p_mapping(text).unwrap();

        assert_eq!(mapping[0].word, "あ");
        assert_eq!(mapping[0].phonemes, vec!["a".to_string()]);

        assert_eq!(mapping[1].word, "、");
        assert!(
            mapping[1].phonemes.is_empty(),
            "コンフィグにより pau が除去されていること"
        );
        assert!(mapping[1].is_ignored);

        assert_eq!(mapping[2].word, "い");
        assert_eq!(mapping[2].phonemes, vec!["i".to_string()]);

        assert_eq!(mapping[3].word, "。");
        assert!(
            mapping[3].phonemes.is_empty(),
            "コンフィグにより pau が除去されていること"
        );
        assert!(mapping[3].is_ignored);

        assert_eq!(mapping.len(), 4);
    }

    #[test]
    fn test_g2p_kana_revert_long_vowels() {
        let text = "人生は効果的。";

        let mut haqumei = Haqumei::new().unwrap();
        let kana_default = haqumei.g2p_kana(text).unwrap();
        assert!(kana_default.contains("セー"));
        assert!(kana_default.contains("コーカ"));
        assert!(kana_default.contains("ワ")); // 助詞は「ワ」

        haqumei.options.revert_long_vowels = true;
        let kana_revert = haqumei.g2p_kana(text).unwrap();

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

        haqumei.options.revert_yotsugana = true;
        let kana_revert = haqumei.g2p_kana(text).unwrap();

        assert!(kana_revert.contains("ハナヂ"));
        assert!(kana_revert.contains("キヅカズ"));
    }

    #[test]
    fn test_g2p_kana_use_read_as_pron() {
        let text = "こんにちは、人生。";

        let mut haqumei = Haqumei::new().unwrap();
        let kana_default = haqumei.g2p_kana(text).unwrap();
        assert!(kana_default.contains("コンニチワ")); // 助詞は「ワ」
        assert!(kana_default.contains("ジンセー")); // 長音化

        haqumei.options.use_read_as_pron = true;
        let kana_read = haqumei.g2p_kana(text).unwrap();

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

    #[test]
    fn test_g2p_iu_normalize_iu() {
        let mut haqumei = Haqumei::with_options(HaqumeiOptions {
            normalize_iu: Some(IuPronunciation::Iu),
            ..Default::default()
        })
        .unwrap();

        assert!(haqumei.g2p_kana("言う").unwrap().contains("イウ"));
        assert!(haqumei.g2p_kana("言って").unwrap().contains("イッテ"));
        assert!(haqumei.g2p_kana("言えば").unwrap().contains("イエバ"));
        assert!(haqumei.g2p_kana("言おう").unwrap().contains("イオー"));
        assert!(haqumei.g2p_kana("言わない").unwrap().contains("イワナイ"));

        assert!(haqumei.g2p_kana("こういう事").unwrap().contains("コーイウ"));
        assert!(
            haqumei
                .g2p_kana("あっという間に")
                .unwrap()
                .contains("アットイウ")
        );
        assert!(haqumei.g2p_kana("物言う株主").unwrap().contains("モノイウ"));
    }

    #[test]
    fn test_g2p_iu_normalize_yuu() {
        let mut haqumei = Haqumei::with_options(HaqumeiOptions {
            normalize_iu: Some(IuPronunciation::Yuu),
            ..Default::default()
        })
        .unwrap();

        assert!(haqumei.g2p_kana("言う").unwrap().contains("ユウ"));
        assert!(haqumei.g2p_kana("言って").unwrap().contains("ユッテ"));
        assert!(haqumei.g2p_kana("言えば").unwrap().contains("ユエバ"));
        assert!(haqumei.g2p_kana("言おう").unwrap().contains("ユオー"));
        assert!(haqumei.g2p_kana("言わない").unwrap().contains("ユワナイ"));

        assert!(haqumei.g2p_kana("そういう事").unwrap().contains("ソーユウ"));
        assert!(
            haqumei
                .g2p_kana("アッと言う間に")
                .unwrap()
                .contains("アットユウ")
        );
        assert!(haqumei.g2p_kana("物言う株主").unwrap().contains("モノユウ"));
    }

    #[test]
    fn test_g2p_iu_normalize_exclusion() {
        let mut haqumei = Haqumei::new().unwrap();

        haqumei.options.normalize_iu = Some(IuPronunciation::Iu);
        assert!(haqumei.g2p_kana("正当な理由").unwrap().contains("リユー"));
        assert!(haqumei.g2p_kana("髪を結う").unwrap().contains("ユウ"));

        haqumei.options.normalize_iu = Some(IuPronunciation::Yuu);
        assert!(haqumei.g2p_kana("正当な理由").unwrap().contains("リユー"));
        assert!(haqumei.g2p_kana("髪を結う").unwrap().contains("ユウ"));
        assert!(haqumei.g2p_kana("言い争う").unwrap().contains("イー"));

        haqumei.options.normalize_iu = None;
        let default_kana = haqumei.g2p_kana("というのも").unwrap();

        haqumei.options.normalize_iu = Some(IuPronunciation::Yuu);
        let yuu_kana = haqumei.g2p_kana("というのも").unwrap();

        assert_eq!(default_kana, yuu_kana);
    }

    #[test]
    fn test_g2p_iu_normalize() {
        let mut haqumei = Haqumei::with_options(HaqumeiOptions {
            normalize_iu: Some(IuPronunciation::Iu),
            ..Default::default()
        })
        .unwrap();

        assert!(haqumei.g2p_kana("こういう事").unwrap().contains("コーイウ"));
        assert!(haqumei.g2p_kana("ああいう事").unwrap().contains("アアイウ"));

        assert!(
            haqumei
                .g2p_kana("あっという間に")
                .unwrap()
                .contains("アットイウ")
        );
        assert!(
            haqumei
                .g2p_kana("アッと言う間に")
                .unwrap()
                .contains("アットイウ")
        );

        assert!(haqumei.g2p_kana("君ていう人は").unwrap().contains("テイウ"));
        assert!(haqumei.g2p_kana("彼という人は").unwrap().contains("トイウ"));
        assert!(
            haqumei
                .g2p_kana("というのも")
                .unwrap()
                .contains("トイウノモ")
        );

        assert!(
            haqumei
                .g2p_kana("誰っていうの")
                .unwrap()
                .contains("ッテイウ")
        );
        assert!(haqumei.g2p_kana("誰とかいう").unwrap().contains("トカイウ"));

        assert!(
            haqumei
                .g2p_kana("いうなれば")
                .unwrap()
                .contains("イウナレバ")
        );
        assert!(
            haqumei
                .g2p_kana("言うまでもない")
                .unwrap()
                .contains("イウマデモナイ")
        );

        assert!(haqumei.g2p_kana("物言う株主").unwrap().contains("モノイウ"));

        haqumei.options.normalize_iu = Some(IuPronunciation::Yuu);

        assert!(haqumei.g2p_kana("こういう事").unwrap().contains("コーユウ"));
        assert!(haqumei.g2p_kana("ああいう事").unwrap().contains("アアユウ"));
        assert!(
            haqumei
                .g2p_kana("あっという間に")
                .unwrap()
                .contains("アットユウ")
        );
        assert!(
            haqumei
                .g2p_kana("アッと言う間に")
                .unwrap()
                .contains("アットユウ")
        );
        dbg!(haqumei.g2p_mapping_detailed("君ていう人は").unwrap());
        assert!(haqumei.g2p_kana("君ていう人は").unwrap().contains("テユウ"));
        assert!(haqumei.g2p_kana("彼という人は").unwrap().contains("トユウ"));
        assert!(
            haqumei
                .g2p_kana("というのも")
                .unwrap()
                .contains("トユウノモ")
        );
        assert!(
            haqumei
                .g2p_kana("誰っていうの")
                .unwrap()
                .contains("ッテユウ")
        );
        assert!(haqumei.g2p_kana("誰とかいう").unwrap().contains("トカユウ"));
        assert!(
            haqumei
                .g2p_kana("いうなれば")
                .unwrap()
                .contains("ユウナレバ")
        );
        assert!(
            haqumei
                .g2p_kana("言うまでもない")
                .unwrap()
                .contains("ユウマデモナイ")
        );
        assert!(haqumei.g2p_kana("物言う株主").unwrap().contains("モノユウ"));
    }

    #[test]
    fn test_g2p_iu_normalize_kanji_only() {
        let mut haqumei = Haqumei::new().unwrap();

        haqumei.options.normalize_iu = None;
        let default_hiragana_iu = haqumei.g2p_kana("そういう事").unwrap();
        let default_hiragana_teiu = haqumei.g2p_kana("君ていう人は").unwrap();
        let default_hiragana_mono = haqumei.g2p_kana("ものいう株主").unwrap();

        haqumei.options.normalize_iu = Some(IuPronunciation::KanjiYuu);

        assert!(haqumei.g2p_kana("言う").unwrap().contains("ユウ"));
        assert!(haqumei.g2p_kana("言って").unwrap().contains("ユッテ"));
        assert!(
            haqumei
                .g2p_kana("アッと言う間に")
                .unwrap()
                .contains("アットユウ")
        );
        assert!(haqumei.g2p_kana("物言う株主").unwrap().contains("モノユウ"));

        assert_eq!(haqumei.g2p_kana("そういう事").unwrap(), default_hiragana_iu);
        assert_eq!(
            haqumei.g2p_kana("君ていう人は").unwrap(),
            default_hiragana_teiu
        );
        assert_eq!(
            haqumei.g2p_kana("ものいう株主").unwrap(),
            default_hiragana_mono
        );
        let default_hiragana_atto = {
            haqumei.options.normalize_iu = None;
            haqumei.g2p_kana("あっという間に").unwrap()
        };
        haqumei.options.normalize_iu = Some(IuPronunciation::KanjiYuu);
        assert_eq!(
            haqumei.g2p_kana("あっという間に").unwrap(),
            default_hiragana_atto
        );
    }
}
