#[cfg(test)]
mod tests {
    use haqumei_kanalizer::{
        ConvertOptions, Kanalizer, KanalizerError, MaxLength, Strategy, StrategyTopK, StrategyTopP,
    };

    #[test]
    fn test_initialization() {
        let kanalizer = Kanalizer::new();
        assert!(kanalizer.is_ok(), "Failed to initialize Kanalizer");
    }

    #[test]
    fn test_inference_greedy() {
        let mut kanalizer = Kanalizer::new().unwrap();

        let cases = vec![
            ("hi", "ハイ"),
            ("pc", "プク"),
            ("hello", "ヘロ"),
            ("hallo", "ハロー"),
            ("international", "インターナショナル"),
            ("kanalizer", "カナライザー"),
            ("level", "レベル"),
            ("easily", "イージリー"),
            ("typo", "タイポ"),
            ("google", "グーグル"),
            ("deepmind", "ディープマインド"),
            ("pneumono", "ニューモノ"),
            ("ultra", "ウルトラ"),
            ("micro", "マイクロ"),
            ("scopic", "スコピック"),
            ("silico", "シリコ"),
            ("volcano", "ヴォルカノ"),
            ("coniosis", "コニオシス"),
            ("humilated", "ヒューミレイテッド"),
            ("taught", "トート"),
            ("thought", "ソート"),
            ("warning", "ワーニング"),
            ("freunde", "フロインデ"),
            ("haben", "ハーベン"),
            ("neun", "ノイン"),
            ("informationswissenschaft", "インフォメイションズ"),
            ("haqumei", "ハクメイ"),
        ];

        for (input, expected) in cases {
            let result = kanalizer.convert(input).expect("Inference failed");
            assert_eq!(result, expected, "Failed on input: {}", input);
        }
    }

    #[test]
    fn test_empty_input() {
        let mut kanalizer = Kanalizer::new().unwrap();

        let result = kanalizer.convert("");
        assert!(matches!(result, Err(KanalizerError::EmptyInput)));
    }

    #[test]
    fn test_invalid_characters() {
        let mut kanalizer = Kanalizer::new().unwrap();

        let result = kanalizer.convert("hello123");
        assert!(matches!(result, Err(KanalizerError::InvalidCharacter('1'))));

        let options = ConvertOptions {
            error_on_invalid_input: false,
            ..Default::default()
        };
        let result_ignored = kanalizer
            .convert_with_options("hello123", &options)
            .unwrap();
        assert_eq!(result_ignored, "ヘロ");
    }

    #[test]
    fn test_inference_strategies() {
        let mut kanalizer = Kanalizer::new().unwrap();
        let input = "random";

        let options_topk = ConvertOptions {
            strategy: Strategy::TopK(StrategyTopK { k: 5 }),
            error_on_incomplete: false,
            ..Default::default()
        };
        let res_topk = kanalizer.convert_with_options(input, &options_topk);
        assert!(res_topk.is_ok(), "TopK inference failed: {:?}", res_topk);
        assert!(!res_topk.unwrap().is_empty());

        let options_topp = ConvertOptions {
            strategy: Strategy::TopP(StrategyTopP {
                top_p: 0.8,
                temperature: 1.2,
            }),
            error_on_incomplete: false,
            ..Default::default()
        };
        let res_topp = kanalizer.convert_with_options(input, &options_topp);
        assert!(res_topp.is_ok(), "TopP inference failed: {:?}", res_topp);
        assert!(!res_topp.unwrap().is_empty());
    }

    #[test]
    fn test_incomplete_conversion() {
        let mut kanalizer = Kanalizer::new().unwrap();

        let options = ConvertOptions {
            max_length: MaxLength::Fixed(std::num::NonZeroUsize::new(3).unwrap()),
            error_on_incomplete: true,
            ..Default::default()
        };

        let result = kanalizer.convert_with_options("internationalization", &options);
        assert!(matches!(result, Err(KanalizerError::IncompleteConversion)));

        let options_no_err = ConvertOptions {
            max_length: MaxLength::Fixed(std::num::NonZeroUsize::new(3).unwrap()),
            error_on_incomplete: false,
            ..Default::default()
        };
        let result_no_err = kanalizer.convert_with_options("internationalization", &options_no_err);
        assert!(result_no_err.is_ok());
    }
}
