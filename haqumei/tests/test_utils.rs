#[cfg(test)]
mod tests {
    use haqumei::utils::{hira2kata, kata2hira};

    #[test]
    fn test_hira2kata_basic() {
        assert_eq!(hira2kata("あいうえお"), "アイウエオ");
        assert_eq!(hira2kata("ぱぴぷぺぽ"), "パピプペポ");
        assert_eq!(hira2kata("ちゃちゅちょ"), "チャチュチョ");
        assert_eq!(hira2kata("っ"), "ッ");
    }

    #[test]
    fn test_kata2hira_basic() {
        assert_eq!(kata2hira("アイウエオ"), "あいうえお");
        assert_eq!(kata2hira("パピプペポ"), "ぱぴぷぺぽ");
        assert_eq!(kata2hira("チャチュチョ"), "ちゃちゅちょ");
        assert_eq!(kata2hira("ッ"), "っ");
    }

    #[test]
    fn test_precomposed_dakuten() {
        assert_eq!(hira2kata("がぎぐげご"), "ガギグゲゴ");
        assert_eq!(kata2hira("ガギグゲゴ"), "がぎぐげご");
    }

    #[test]
    fn test_combining_dakuten() {
        let hira_combined = "か\u{3099}"; // か + 結合濁点
        let kata_combined = "カ\u{3099}"; // カ + 結合濁点

        assert_eq!(hira2kata(hira_combined), kata_combined);
        assert_eq!(kata2hira(kata_combined), hira_combined);
    }

    #[test]
    fn test_edge_cases() {
        // ひらがな: ぁ(3041) 〜 ゖ(3096)
        // カタカナ: ァ(30A1) 〜 ヶ(30F6)
        assert_eq!(hira2kata("ぁ"), "ァ");
        assert_eq!(hira2kata("ゖ"), "ヶ");
        assert_eq!(kata2hira("ァ"), "ぁ");
        assert_eq!(kata2hira("ヶ"), "ゖ");

        assert_eq!(hira2kata("ゔ"), "ヴ");
        assert_eq!(kata2hira("ヴ"), "ゔ");
    }

    #[test]
    fn test_non_target_characters() {
        let mixed = "あ漢123!ー A";
        assert_eq!(hira2kata(mixed), "ア漢123!ー A");
        assert_eq!(kata2hira("ア漢123!ー A"), mixed);

        assert_eq!(hira2kata("ｱｲｳ"), "ｱｲｳ");
    }

    #[test]
    fn test_choonpu() {
        assert_eq!(hira2kata("らーめん"), "ラーメン");
        assert_eq!(kata2hira("ラーメン"), "らーめん");
    }

    #[test]
    fn test_special_hira() {
        // 「ゐ」(U+3090) / 「ゑ」(U+3091)
        // 「ヰ」(U+30F0) / 「ヱ」(U+30F1)
        assert_eq!(hira2kata("ゐゑ"), "ヰヱ");
        assert_eq!(kata2hira("ヰヱ"), "ゐゑ");
    }
}
