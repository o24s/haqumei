#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::LazyLock,
    };

    use haqumei::{Haqumei, OpenJTalk, errors::HaqumeiError};

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

        let mapping = haqumei.g2p_mapping_detailed(text).unwrap();
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

        let mapping = haqumei.g2p_mapping_detailed(text).unwrap();

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

        let mapping = haqumei.g2p_mapping_detailed(text).unwrap();

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

        let result = haqumei.g2p_mapping_detailed(text).unwrap();

        let reconstructed: String = result.iter().map(|d| d.word.as_str()).collect();

        assert_eq!(text, reconstructed);
    }

    #[test]
    fn test_mapping_flags() {
        let waganeko = fs::read_to_string(WAGANEKO_PATH.as_path()).unwrap();
        let waganeko: Vec<&str> = waganeko.lines().collect();

        let mut haqumei = Haqumei::new().unwrap();
        let mut open_jtalk = OpenJTalk::new().unwrap();
        let result_hq = haqumei.g2p_mapping_detailed_batch(&waganeko).unwrap();
        let result_ojt = open_jtalk.g2p_mapping_detailed_batch(&waganeko).unwrap();

        for (details_hq, details_ojt) in result_hq.into_iter().zip(result_ojt) {
            for (detail_hq, detail_ojt) in details_hq.clone().into_iter().zip(details_ojt) {
                if detail_hq.is_ignored {
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
                if detail_ojt.is_ignored {
                    assert_eq!(detail_ojt.phonemes, &["sp"]);
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
";

        let result = haqumei.g2p_mapping_detailed(text).unwrap();
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
        ];

        assert_eq!(result, expected);
    }
}
