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
    fn test_g2p_mapping_detailed_full() {
        let waganeko = fs::read_to_string(WAGANEKO_PATH.as_path()).unwrap();
        let waganeko: Vec<&str> = waganeko.lines().collect();

        let mut ojt = OpenJTalk::new().unwrap();
        ojt.g2p_mapping_detailed_batch(&waganeko).unwrap();

        let mut haqumei = Haqumei::new().unwrap();
        haqumei.g2p_mapping_detailed_batch(&waganeko).unwrap();
    }

    fn setup() -> Haqumei {
        Haqumei::new().expect("Failed to initialize Haqumei")
    }

    #[test]
    fn test_g2p_basic() {
        let mut haqumei = setup();
        let text = "こんにちは";
        let phonemes = haqumei.g2p(text).unwrap();

        assert_eq!(phonemes, vec!["k", "o", "N", "n", "i", "ch", "i", "w", "a"]);
    }

    /// 空文字列を渡してもクラッシュせず、空の結果が返ることを確認
    #[test]
    fn test_empty_string() {
        let mut haqumei = setup();
        let text = "";
        let result = haqumei.g2p(text).unwrap();
        assert!(result.is_empty());

        let mapping = haqumei.g2p_mapping_detailed(text).unwrap();
        assert!(mapping.is_empty());
    }

    /// NULL文字が含まれる入力でエラーになり、クラッシュしないこと
    #[test]
    fn test_null_byte_injection() {
        let mut haqumei = setup();
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
        let mut haqumei = setup();
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
        let mut haqumei = setup();

        let _ = haqumei.g2p("悪い\0Input");

        let text = "復帰";
        let result = haqumei.g2p(text);
        assert!(result.is_ok());
    }

    #[test]
    fn test_unknown_word_mapping() {
        let mut haqumei = setup();
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
        let mut haqumei = setup();
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
        let mut haqumei = setup();
        let text = "#$%&'()\n\t";

        let result = haqumei.g2p(text);
        assert!(result.is_ok());
    }
}
