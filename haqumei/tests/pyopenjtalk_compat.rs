#[cfg(test)]
mod tests {
    use haqumei::{
        Haqumei, HaqumeiOptions, OpenJTalk, WordPhonemeDetail, WordPhonemeMap, WordPhonemePair,
    };

    #[test]
    fn test_njd_features() {
        let mut haqumei = Haqumei::new().unwrap();
        let features = haqumei.run_frontend("こんにちは").unwrap();

        assert_eq!(features.len(), 1);
        let f = &features[0];

        assert_eq!(f.string, "こんにちは");
        assert_eq!(f.pos, "感動詞");
        assert_eq!(f.pos_group1, "*");
        assert_eq!(f.pos_group2, "*");
        assert_eq!(f.pos_group3, "*");
        assert_eq!(f.ctype, "*");
        assert_eq!(f.cform, "*");
        assert_eq!(f.orig, "こんにちは");
        assert_eq!(f.read, "コンニチハ");
        assert_eq!(f.pron, "コンニチワ");
        assert_eq!(f.acc, 0);
        assert_eq!(f.mora_size, 5);
        assert_eq!(f.chain_rule, "-1");
        assert_eq!(f.chain_flag, -1);
    }

    #[test]
    fn test_jtalk_surface_reconstruction() {
        let mut haqumei = Haqumei::new().unwrap();
        let texts = vec![
            "今日も良い天気ですね",
            "こんにちは。",
            "どんまい！",
            "パソコンのとりあえず知っておきたい使い方",
        ];

        for text in texts {
            let features = haqumei.run_frontend(text).unwrap();
            let surface: String = features.iter().map(|f| f.string.as_str()).collect();
            assert_eq!(surface, text, "Surface reconstruction failed for: {}", text);
        }
    }

    #[test]
    fn test_g2p_kana() {
        let mut haqumei = Haqumei::new().unwrap();
        let cases = vec![
            ("", ""),
            ("今日もこんにちは", "キョーモコンニチワ"),
            ("いやあん", "イヤーン"),
            (
                "パソコンのとりあえず知っておきたい使い方",
                "パソコンノトリアエズシッテオキタイツカイカタ",
            ),
        ];

        for (text, expected) in cases {
            let p = haqumei.g2p_kana(text).unwrap();
            assert_eq!(p, expected, "Failed for text: {}", text);
        }
    }

    #[test]
    fn test_g2p_phone() {
        let mut haqumei = Haqumei::new().unwrap();
        let cases = vec![
            ("", ""),
            ("こんにちは", "k o N n i ch i w a"),
            ("ななみんです", "n a n a m i N d e s U"),
            ("ハローユーチューブ", "h a r o o y u u ch u u b u"),
        ];

        for (text, expected) in cases {
            let p = haqumei.g2p(text).unwrap().join(" ");
            assert_eq!(p, expected, "Failed for text: {}", text);
        }
    }

    #[test]
    fn test_g2p_nani_model() {
        let mut haqumei = Haqumei::with_options(HaqumeiOptions {
            predict_nani: true,
            ..Default::default()
        })
        .unwrap();

        let cases = vec![
            (
                "何か問題があれば何でも言ってください、どんな些細なことでも何とかします。",
                "ナニカモンダイガアレバナンデモイッテクダサイ、ドンナササイナコトデモナントカシマス。",
            ),
            (
                "何か特別なことをしたわけではありませんが、何故か周りの人々が何かと気にかけてくれます。何と言えばいいのか分かりません。",
                "ナニカトクベツナコトヲシタワケデワアリマセンガ、ナゼカマワリノヒトビトガナニカトキニカケテクレマス。ナントイエバイイノカワカリマセン。",
            ),
            (
                "私も何とかしたいですが、何でも行くリソースはありません。",
                "ワタシモナントカシタイデスガ、ナンデモイクリソースワアリマセン。",
            ),
            (
                "何を言っても何の問題もありません。",
                "ナニヲイッテモナンノモンダイモアリマセン。",
            ),
            (
                "これは何ですか？何の情報？",
                "コレワナンデスカ？ナンノジョーホー？",
            ),
            (
                "何だろう、何でも嘘つくのやめてもらっていいですか？",
                "ナンダロー、ナンデモウソツクノヤメテモラッテイイデスカ？",
            ),
            ("質問は何のことかな？", "シツモンワナンノコトカナ？"),
        ];

        for (text, expected) in cases {
            let p = haqumei.g2p_kana(text).unwrap();
            assert_eq!(p, expected, "Nani model check failed for: {}", text);
        }
    }

    #[test]
    fn test_odoriji() {
        let mut haqumei = Haqumei::new().unwrap();

        // --- 一の字点（ゝ、ゞ、ヽ、ヾ）の処理テスト ---

        // 濁点なしの一の字点: "なゝ樹" -> "な" "な" "き"
        let f = haqumei.run_frontend("なゝ樹").unwrap();
        assert_eq!(f[0].read, "ナ");
        assert_eq!(f[0].pron, "ナ");
        assert_eq!(f[0].mora_size, 1);
        assert_eq!(f[1].read, "ナ");
        assert_eq!(f[1].pron, "ナ");
        assert_eq!(f[1].mora_size, 1);
        assert_eq!(f[2].read, "キ");
        assert_eq!(f[2].pron, "キ");
        assert_eq!(f[2].mora_size, 1);

        // 濁点ありの一の字点: "金子みすゞ" -> ... "ミス" "ズ"
        let f = haqumei.run_frontend("金子みすゞ").unwrap();
        assert_eq!(f[0].read, "カネコ");
        assert_eq!(f[0].pron, "カネコ");
        assert_eq!(f[0].mora_size, 3);
        assert_eq!(f[1].read, "ミス");
        assert_eq!(f[1].pron, "ミス");
        assert_eq!(f[1].mora_size, 2);
        assert_eq!(f[2].read, "ズ");
        assert_eq!(f[2].pron, "ズ");
        assert_eq!(f[2].mora_size, 1);

        // 濁点なしの一の字点（づゝ） -> "ヅ" "ツ"
        // ※「づ」の清音は「つ」
        let f = haqumei.run_frontend("づゝ").unwrap();
        assert_eq!(f[0].read, "ヅ");
        assert_eq!(f[0].pron, "ヅ");
        assert_eq!(f[0].mora_size, 1);
        assert_eq!(f[1].read, "ツ");
        assert_eq!(f[1].pron, "ツ");
        assert_eq!(f[1].mora_size, 1);

        // 濁点ありの一の字点（ぶゞ漬け） -> "ブ" "ブ"
        let f = haqumei.run_frontend("ぶゞ漬け").unwrap();
        assert_eq!(f[0].read, "ブ");
        assert_eq!(f[0].pron, "ブ");
        assert_eq!(f[0].mora_size, 1);
        assert_eq!(f[1].read, "ブ");
        assert_eq!(f[1].pron, "ブ");
        assert_eq!(f[1].mora_size, 1);
        assert_eq!(f[2].read, "ヅケ");
        assert_eq!(f[2].pron, "ヅケ");
        assert_eq!(f[2].mora_size, 2);

        // 片仮名の一の字点（バナヽ）
        let f = haqumei.run_frontend("バナヽ").unwrap();
        assert_eq!(f[0].read, "バナ");
        assert_eq!(f[0].pron, "バナ");
        assert_eq!(f[0].mora_size, 2);
        assert_eq!(f[1].read, "ナ");
        assert_eq!(f[1].pron, "ナ");
        assert_eq!(f[1].mora_size, 1);

        // --- 踊り字（々）の処理テスト ---

        // 単一の踊り字（辞書登録外）: "愛々" -> "アイ" "アイ"
        let f = haqumei.run_frontend("愛々").unwrap();
        assert_eq!(f[0].read, "アイ");
        assert_eq!(f[0].pron, "アイ");
        assert_eq!(f[0].mora_size, 2);
        assert_eq!(f[1].read, "アイ");
        assert_eq!(f[1].pron, "アイ");
        assert_eq!(f[1].mora_size, 2);

        let f = haqumei.run_frontend("咲々").unwrap();
        assert_eq!(f[0].read, "サキ");
        assert_eq!(f[0].pron, "サキ");
        assert_eq!(f[0].mora_size, 2);
        assert_eq!(f[1].read, "サキ");
        assert_eq!(f[1].pron, "サキ");
        assert_eq!(f[1].mora_size, 2);

        // 再解析が必要なケース
        // 結婚式々場 -> ケッコンシキ + シキジョウ
        let f = haqumei.run_frontend("結婚式々場").unwrap();
        assert_eq!(f[0].read, "ケッコンシキ");
        assert_eq!(f[0].pron, "ケッコンシ’キ");
        assert_eq!(f[0].mora_size, 6);
        assert_eq!(f[1].read, "シキジョウ");
        assert_eq!(f[1].pron, "シ’キジョー");
        assert_eq!(f[1].mora_size, 4);

        // 学生々活 -> ガクセイ + セイカツ
        let f = haqumei.run_frontend("学生々活").unwrap();
        assert_eq!(f[0].read, "ガクセイ");
        assert_eq!(f[0].pron, "ガク’セー");
        assert_eq!(f[0].mora_size, 4);
        assert_eq!(f[1].read, "セイカツ");
        assert_eq!(f[1].pron, "セーカツ");
        assert_eq!(f[1].mora_size, 4);

        // 民主々義 -> ミンシュ + シュギ
        let f = haqumei.run_frontend("民主々義").unwrap();
        assert_eq!(f[0].read, "ミンシュ");
        assert_eq!(f[0].pron, "ミンシュ");
        assert_eq!(f[0].mora_size, 3);
        assert_eq!(f[1].read, "シュギ");
        assert_eq!(f[1].pron, "シュギ");
        assert_eq!(f[1].mora_size, 2);

        // 連続する踊り字
        // 叙々々苑
        let f = haqumei.run_frontend("叙々々苑").unwrap();
        assert_eq!(f[0].read, "ジョ");
        assert_eq!(f[0].pron, "ジョ");
        assert_eq!(f[0].mora_size, 1);
        assert_eq!(f[1].read, "ジョジョ");
        assert_eq!(f[1].pron, "ジョジョ");
        assert_eq!(f[1].mora_size, 2);

        // 叙々々々苑
        let f = haqumei.run_frontend("叙々々々苑").unwrap();
        assert_eq!(f[0].read, "ジョ");
        assert_eq!(f[0].pron, "ジョ");
        assert_eq!(f[0].mora_size, 1);
        assert_eq!(f[1].read, "ジョジョ");
        assert_eq!(f[1].pron, "ジョジョ");
        assert_eq!(f[1].mora_size, 2);
        assert_eq!(f[2].read, "ジョ");
        assert_eq!(f[2].pron, "ジョ");
        assert_eq!(f[2].mora_size, 1);

        // 叙々々々々苑
        let f = haqumei.run_frontend("叙々々々々苑").unwrap();
        assert_eq!(f[0].read, "ジョ");
        assert_eq!(f[0].pron, "ジョ");
        assert_eq!(f[0].mora_size, 1);
        assert_eq!(f[1].read, "ジョジョ");
        assert_eq!(f[1].pron, "ジョジョ");
        assert_eq!(f[1].mora_size, 2);
        assert_eq!(f[2].read, "ジョジョ");
        assert_eq!(f[2].pron, "ジョジョ");
        assert_eq!(f[2].mora_size, 2);

        // 叙々々々々々苑 -> ジョ (ジョジョジョジョジョ)
        let f = haqumei.run_frontend("叙々々々々々苑").unwrap();
        assert_eq!(f[0].read, "ジョ");
        assert_eq!(f[0].pron, "ジョ");
        assert_eq!(f[0].mora_size, 1);
        assert_eq!(f[1].read, "ジョジョジョジョジョ");
        assert_eq!(f[1].pron, "ジョジョジョジョジョ");
        assert_eq!(f[1].mora_size, 5);

        // 複々々線
        let f = haqumei.run_frontend("複々々線").unwrap();
        assert_eq!(f[0].read, "フク");
        assert_eq!(f[0].pron, "フ’ク");
        assert_eq!(f[0].mora_size, 2);
        assert_eq!(f[1].read, "フクフク");
        assert_eq!(f[1].pron, "フ’クフ’ク");
        assert_eq!(f[1].mora_size, 4);

        // 複々々々線
        let f = haqumei.run_frontend("複々々々線").unwrap();
        assert_eq!(f[0].read, "フク");
        assert_eq!(f[0].pron, "フ’ク");
        assert_eq!(f[0].mora_size, 2);
        assert_eq!(f[1].read, "フクフク");
        assert_eq!(f[1].pron, "フ’クフ’ク");
        assert_eq!(f[1].mora_size, 4);
        assert_eq!(f[2].read, "フク");
        assert_eq!(f[2].pron, "フ’ク");
        assert_eq!(f[2].mora_size, 2);

        // 今日も前進々々 (複数漢字トークンの繰り返し)
        let f = haqumei.run_frontend("今日も前進々々").unwrap();
        assert_eq!(f[0].read, "キョウ");
        assert_eq!(f[0].pron, "キョー");
        assert_eq!(f[0].mora_size, 2);
        assert_eq!(f[1].read, "モ");
        assert_eq!(f[1].pron, "モ");
        assert_eq!(f[1].mora_size, 1);
        assert_eq!(f[2].read, "ゼンシン");
        assert_eq!(f[2].pron, "ゼンシン");
        assert_eq!(f[2].mora_size, 4);
        assert_eq!(f[3].read, "ゼンシン");
        assert_eq!(f[3].pron, "ゼンシン");
        assert_eq!(f[3].mora_size, 4);

        // 部分々々
        let f = haqumei.run_frontend("部分々々").unwrap();
        assert_eq!(f[0].read, "ブブン");
        assert_eq!(f[0].pron, "ブブン");
        assert_eq!(f[0].mora_size, 3);
        assert_eq!(f[1].read, "ブブン");
        assert_eq!(f[1].pron, "ブブン");
        assert_eq!(f[1].mora_size, 3);

        // 後手々々
        let f = haqumei.run_frontend("後手々々").unwrap();
        assert_eq!(f[0].read, "ゴテ");
        assert_eq!(f[0].pron, "ゴテ");
        assert_eq!(f[0].mora_size, 2);
        assert_eq!(f[1].read, "ゴテ");
        assert_eq!(f[1].pron, "ゴテ");
        assert_eq!(f[1].mora_size, 2);

        // 其他々々
        let f = haqumei.run_frontend("其他々々").unwrap();
        assert_eq!(f[0].read, "ソノ");
        assert_eq!(f[0].pron, "ソノ");
        assert_eq!(f[0].mora_size, 2);
        assert_eq!(f[1].read, "ホカ");
        assert_eq!(f[1].pron, "ホカ");
        assert_eq!(f[1].mora_size, 2);
        assert_eq!(f[2].read, "ソノホカ");
        assert_eq!(f[2].pron, "ソノホカ");
        assert_eq!(f[2].mora_size, 4);

        // 踊り字の前に漢字がない場合 (絵文字含む)
        let f = haqumei
            .run_frontend("やっほー！元気かな？ヾ(≧▽≦)ﾉ")
            .unwrap();
        assert_eq!(f[0].read, "ヤッホー");
        assert_eq!(f[0].pron, "ヤッホー");
        assert_eq!(f[0].mora_size, 4);
        assert_eq!(f[1].read, "！");
        assert_eq!(f[1].pron, "！");
        assert_eq!(f[1].mora_size, 0);
        assert_eq!(f[2].read, "ゲンキ");
        assert_eq!(f[2].pron, "ゲンキ’");
        assert_eq!(f[2].mora_size, 3);
        assert_eq!(f[3].read, "カ");
        assert_eq!(f[3].pron, "カ");
        assert_eq!(f[3].mora_size, 1);
        assert_eq!(f[4].read, "ナ");
        assert_eq!(f[4].pron, "ナ");
        assert_eq!(f[4].mora_size, 1);
        assert_eq!(f[5].read, "？");
        assert_eq!(f[5].pron, "？");
        assert_eq!(f[5].mora_size, 0);
        assert_eq!(f[6].read, "、");
        assert_eq!(f[6].pron, "、");
        assert_eq!(f[6].mora_size, 0);
        assert_eq!(f[7].read, "、");
        assert_eq!(f[7].pron, "、");
        assert_eq!(f[7].mora_size, 0);
        assert_eq!(f[8].read, "ノ");
        assert_eq!(f[8].pron, "ノ");
        assert_eq!(f[8].mora_size, 1);
    }

    const PHONEME_MAPPING_CORPUS: &[&str] = &[
        "こんにちは",
        "おはようございます",
        "東京は日本の首都です",
        "東京都知事が記者会見を行った。",
        "大阪",
        "外国人参政権",
        "学生生活",
        "学生々活は楽しい",
        "部分々々",
        "東京、大阪",
        "東京　大阪",
        "（テスト・ケース）",
        "今日は2112年9月3日です",
        "電話番号は090-1234-5678です",
        "明日は雨が降るでしょう",
        "ご遠慮ください",
        "お入りください",
        "食べよう",
        "見よう",
        "読もう",
        "書こう",
        "遊ぼう",
        "起きよう",
        "考えよう",
        "見せよう",
        "行こう",
        "入ろう",
        "来よう",
        "しよう",
        "食べている",
        "読んでいる",
        "書いている",
        "走っている",
        "見ている",
        "起きている",
        "つまみ出されようとした",
    ];

    const LONG_VOWEL_MERGE_CASES: &[(&str, &str, &str)] = &[
        ("食べよう", "食べよう", "食べる"),
        ("見よう", "見よう", "見る"),
        ("読もう", "読もう", "読む"),
        ("書こう", "書こう", "書く"),
        ("遊ぼう", "遊ぼう", "遊ぶ"),
        ("起きよう", "起きよう", "起きる"),
        ("考えよう", "考えよう", "考える"),
        ("見せよう", "見せよう", "見せる"),
        ("行こう", "行こう", "行く"),
        ("入ろう", "入ろう", "入る"),
        ("来よう", "来よう", "来る"),
        ("つまみ出されようとした", "れよう", "れる"),
        (
            "あーーーーーーーーあ",
            "あーーーーーーーー",
            "あーーーーーーーー",
        ),
    ];

    trait PhonemesExtractor {
        fn get_phonemes(&self) -> &[String];
    }
    impl PhonemesExtractor for WordPhonemePair {
        fn get_phonemes(&self) -> &[String] {
            &self.phonemes
        }
    }
    impl PhonemesExtractor for WordPhonemeMap {
        fn get_phonemes(&self) -> &[String] {
            &self.phonemes
        }
    }
    impl PhonemesExtractor for WordPhonemeDetail {
        fn get_phonemes(&self) -> &[String] {
            &self.phonemes
        }
    }

    fn flatten_mapping_phonemes<T: PhonemesExtractor>(
        mapping: &[T],
        keep_pause: bool,
    ) -> Vec<String> {
        let mut phonemes = Vec::new();
        for entry in mapping {
            let p = entry.get_phonemes();
            if !keep_pause && (p == ["pau"] || p == ["sp"]) {
                continue;
            }
            if p == ["unk"] {
                continue;
            }
            phonemes.extend(p.iter().cloned());
        }
        phonemes
    }

    fn extract_label_phonemes(labels: &[String], keep_pause: bool) -> Vec<String> {
        if labels.len() <= 2 {
            return vec![];
        }
        let mut phonemes = Vec::new();
        for label in &labels[1..labels.len() - 1] {
            let p = label.split('-').nth(1).unwrap().split('+').next().unwrap();
            if !keep_pause && p == "pau" {
                continue;
            }
            phonemes.push(p.to_string());
        }
        phonemes
    }

    #[test]
    fn test_run_frontend_empty_string() {
        let mut ojt = OpenJTalk::new().unwrap();
        let features = ojt.run_frontend("").unwrap();
        assert!(features.is_empty());
    }

    #[test]
    fn test_run_frontend_very_long_text() {
        let mut ojt = OpenJTalk::new().unwrap();
        let long_text = "あ".repeat(10000);
        let err = ojt.run_frontend(&long_text);
        assert!(err.is_err());

        let features = ojt.run_frontend("こんにちは").unwrap();
        assert!(!features.is_empty());
    }

    #[test]
    fn test_run_frontend_special_characters_only() {
        let mut ojt = OpenJTalk::new().unwrap();
        let features = ojt.run_frontend("!@#$%^&*()").unwrap();
        assert!(!features.is_empty() || features.is_empty()); // 少なくともクラッシュしないこと
    }

    #[test]
    fn test_run_frontend_null_bytes_should_not_segfault() {
        let mut ojt = OpenJTalk::new().unwrap();
        let err = ojt.run_frontend("\x00\x01\x02");
        assert!(err.is_err());
    }

    #[test]
    fn test_run_frontend_mixed_japanese_ascii() {
        let mut ojt = OpenJTalk::new().unwrap();
        let features = ojt.run_frontend("Hello世界123").unwrap();
        assert!(!features.is_empty());
    }

    #[test]
    fn test_make_label_too_long_feature_should_not_crash() {
        let mut ojt = OpenJTalk::new().unwrap();
        let mut features = ojt.run_frontend("こんにちは").unwrap();
        features[0].pron = "ア".repeat(400);
        let labels = ojt.make_label(&features).unwrap();
        assert!(!labels.is_empty());
    }

    #[test]
    fn test_make_label_empty_string_fields_should_not_crash() {
        let mut ojt = OpenJTalk::new().unwrap();
        let mut features = ojt.run_frontend("こんにちは").unwrap();
        dbg!(&features);
        features[0].pron = "".to_string();
        features[0].pos = "".to_string();
        features[0].ctype = "".to_string();
        features[0].cform = "".to_string();
        let labels = ojt.make_label(&features).unwrap();
        assert_eq!(labels, &[] as &[String])
    }

    #[test]
    fn test_make_label_null_character_should_not_break_next_call() {
        let mut ojt = OpenJTalk::new().unwrap();
        let mut features = ojt.run_frontend("こんにちは").unwrap();
        features[0].pron = "ア\x00イ".to_string();
        let err = ojt.make_label(&features);
        assert!(err.is_err());

        let features2 = ojt.run_frontend("こんにちは").unwrap();
        let labels = ojt.make_label(&features2).unwrap();
        assert!(!labels.is_empty());
    }

    #[test]
    fn test_g2p_large_digit_sequence_should_keep_place_reading() {
        let mut ojt = OpenJTalk::new().unwrap();
        let pron = ojt.g2p("10000").unwrap().join(" ");
        assert_eq!(pron, "i ch i m a N");
    }

    #[test]
    fn test_g2p_large_digit_sequence_with_oku_should_keep_place_reading() {
        let mut ojt = OpenJTalk::new().unwrap();
        let pron = ojt.g2p("100000000").unwrap().join(" ");
        assert_eq!(pron, "i ch i o k u");
    }

    #[test]
    fn test_run_mecab_runtime_error_should_not_break_next_call() {
        let mut ojt = OpenJTalk::new().unwrap();
        let long_text = "😎".repeat(5000);
        let err = ojt.run_mecab(&long_text);
        assert!(err.is_err());

        let morphs = ojt.run_mecab("こんにちは").unwrap();
        assert!(!morphs.is_empty());
    }

    #[test]
    fn test_run_mecab_detailed_known_word() {
        let mut ojt = OpenJTalk::new().unwrap();
        let morphs = ojt.run_mecab_detailed("こんにちは").unwrap();
        assert!(!morphs.is_empty());
        assert!(morphs.iter().any(|m| !m.is_unknown));
    }

    #[test]
    fn test_run_mecab_detailed_unknown_word() {
        let mut ojt = OpenJTalk::new().unwrap();
        let morphs = ojt.run_mecab_detailed("xtjq").unwrap();
        assert!(morphs.iter().any(|m| m.is_unknown));
    }

    #[test]
    fn test_run_mecab_detailed_includes_ignored() {
        let mut ojt = OpenJTalk::new().unwrap();
        let normal = ojt.run_mecab("東京　大阪").unwrap();
        let detailed = ojt.run_mecab_detailed("東京　大阪").unwrap();
        assert!(detailed.len() >= normal.len());
    }

    #[test]
    fn test_run_mecab_detailed_feature_format() {
        let mut ojt = OpenJTalk::new().unwrap();
        let morphs = ojt.run_mecab_detailed("こんにちは").unwrap();
        for morph in morphs {
            assert!(morph.feature.starts_with(&morph.surface));
        }
    }

    #[test]
    fn test_make_phoneme_mapping_basic() {
        let mut ojt = OpenJTalk::new().unwrap();
        let mapping = ojt.g2p_pairs("こんにちは").unwrap();
        assert!(!mapping.is_empty());
        for entry in mapping {
            assert!(!entry.phonemes.is_empty());
        }
    }

    #[test]
    fn test_make_phoneme_mapping_with_punctuation() {
        let mut ojt = OpenJTalk::new().unwrap();
        let mapping = ojt.g2p_pairs("東京、大阪").unwrap();
        assert!(mapping.iter().any(|e| e.phonemes == ["pau"]));
    }

    #[test]
    fn test_make_phoneme_mapping_boundary_punctuation_end() {
        let mut ojt = OpenJTalk::new().unwrap();
        let mapping = ojt.g2p_pairs("あ。").unwrap();
        assert_eq!(mapping[0].word, "あ");
        assert_eq!(mapping[0].phonemes, ["a"]);
        assert_eq!(mapping[1].word, "。");
        assert_eq!(mapping[1].phonemes, ["pau"]);
    }

    #[test]
    fn test_make_phoneme_mapping_pause_like_symbols() {
        let mut ojt = OpenJTalk::new().unwrap();
        let mapping = ojt.g2p_pairs("（テスト・ケース）").unwrap();
        assert_eq!(mapping[0].word, "（");
        assert_eq!(mapping[0].phonemes, ["pau"]);
        assert_eq!(mapping[1].word, "テスト");
        assert_eq!(mapping[1].phonemes, ["t", "e", "s", "U", "t", "o"]);
        assert_eq!(mapping[2].word, "・");
        assert_eq!(mapping[2].phonemes, ["pau"]);
    }

    #[test]
    fn test_make_phoneme_mapping_digit() {
        let mut ojt = OpenJTalk::new().unwrap();
        let mapping = ojt.g2p_pairs("123").unwrap();
        assert!(!mapping.is_empty());
    }

    #[test]
    fn test_g2p_mapping_basic() {
        let mut ojt = OpenJTalk::new().unwrap();
        let detailed = ojt.g2p_mapping("こんにちは").unwrap();
        assert!(!detailed.is_empty());
        assert!(detailed.iter().any(|e| !e.is_unknown));
    }

    #[test]
    fn test_g2p_mapping_unknown_word() {
        let mut ojt = OpenJTalk::new().unwrap();
        let detailed = ojt.g2p_mapping("xtjqは最高").unwrap();
        assert!(detailed.iter().any(|e| e.is_unknown));
    }

    #[test]
    fn test_g2p_mapping_unknown_after_digit_normalization() {
        let mut ojt = OpenJTalk::new().unwrap();
        let detailed = ojt.g2p_mapping("7xyz").unwrap();
        assert!(detailed.iter().any(|e| e.word == "七"));
        let xyz_entry = detailed.iter().find(|e| e.word == "ｘｙｚ").unwrap();
        assert!(xyz_entry.is_unknown);
    }

    #[test]
    fn test_g2p_mapping_corpus_phoneme_consistency() {
        let mut ojt = OpenJTalk::new().unwrap();
        for text in PHONEME_MAPPING_CORPUS {
            let mapping = ojt.g2p_mapping(text).unwrap();
            let labels = ojt.extract_fullcontext(text).unwrap();

            assert_eq!(
                flatten_mapping_phonemes(&mapping, false),
                extract_label_phonemes(&labels, false)
            );
        }
    }

    #[test]
    fn test_g2p_mapping_detailed_long_vowel_metadata() {
        let mut ojt = OpenJTalk::new().unwrap();
        for &(text, merged_surface, expected_orig) in LONG_VOWEL_MERGE_CASES {
            let mapping = ojt.g2p_mapping_detailed(text).unwrap();
            let merged_entry = mapping.iter().find(|e| e.word == merged_surface).unwrap();
            assert!(merged_entry.features.is_empty());
            assert_eq!(merged_entry.orig, expected_orig);
        }
    }

    #[test]
    fn test_run_frontend_detailed_basic() {
        let mut ojt = OpenJTalk::new().unwrap();
        let text = "こんにちは";
        let (njd, morphs) = ojt.run_frontend_detailed(text).unwrap();
        let njd_normal = ojt.run_frontend(text).unwrap();
        assert_eq!(njd, njd_normal);
        assert!(!morphs.is_empty());
    }

    #[test]
    fn test_g2p_mapping_detailed_features_populated() {
        let mut ojt = OpenJTalk::new().unwrap();
        let mapping = ojt.g2p_mapping_detailed("東京は日本の首都です").unwrap();
        for entry in mapping {
            if !entry.features.is_empty() {
                assert_eq!(entry.features[0], entry.word);
                assert!(entry.features.len() >= 8);
            }
        }
    }

    #[test]
    fn test_g2p_mapping_detailed_features_unknown_word() {
        let mut ojt = OpenJTalk::new().unwrap();
        let mapping = ojt.g2p_mapping_detailed("xtjqは最高").unwrap();
        let xtjq = mapping.iter().find(|e| e.word == "ｘｔｊｑ").unwrap();
        assert!(xtjq.is_unknown);
        assert_eq!(xtjq.features.len(), 8);
        assert_eq!(xtjq.features[0], "ｘｔｊｑ");
    }

    #[test]
    fn test_g2p_mapping_odori_resync() {
        let mut haqumei = Haqumei::new().unwrap();
        let mapping = haqumei.g2p_mapping_detailed("学生々活は楽しい").unwrap();

        let words: Vec<_> = mapping.iter().map(|e| e.word.as_str()).collect();
        assert!(words.contains(&"学生"));
        assert!(words.contains(&"生活"));
        assert!(words.contains(&"は"));
        assert!(words.contains(&"楽しい"));

        let seikatsu = mapping.iter().find(|e| e.word == "生活").unwrap();
        assert_eq!(seikatsu.phonemes, ["s", "e", "e", "k", "a", "ts", "u"]);

        let tanoshii = mapping.iter().find(|e| e.word == "楽しい").unwrap();
        assert!(!tanoshii.phonemes.is_empty());
    }

    #[test]
    fn test_g2p_mapping_odori_digit_unknown_combined() {
        let mut ojt = OpenJTalk::new().unwrap();
        let mapping = ojt.g2p_mapping_detailed("学生々活7xyz大阪").unwrap();

        let xyz = mapping.iter().find(|e| e.word == "ｘｙｚ").unwrap();
        assert!(xyz.is_unknown);

        let osaka = mapping.iter().find(|e| e.word == "大阪").unwrap();
        assert!(!osaka.is_unknown);
        assert_eq!(osaka.phonemes, ["o", "o", "s", "a", "k", "a"]);
    }

    #[test]
    fn test_g2p_mapping_accent_phrase_boundary() {
        let mut ojt = OpenJTalk::new().unwrap();
        let mapping = ojt
            .g2p_mapping_detailed("東京都知事が記者会見を行った。")
            .unwrap();

        let tokyo = mapping.iter().find(|e| e.word == "東京").unwrap();
        let tochiji = mapping.iter().find(|e| e.word == "都知事").unwrap();
        let ga = mapping.iter().find(|e| e.word == "が").unwrap();
        let kisha = mapping.iter().find(|e| e.word == "記者").unwrap();

        assert!(tokyo.chain_flag == -1 || tokyo.chain_flag == 0);
        assert_eq!(tochiji.chain_flag, 1);
        assert_eq!(ga.chain_flag, 1);
        assert_eq!(kisha.chain_flag, 0);
    }

    #[test]
    fn test_odoriji_voiced_and_voiceless_conversion() {
        let mut haqumei = Haqumei::new().unwrap();
        assert_eq!(haqumei.g2p_kana("がゝ").unwrap(), "ガカ");
        assert_eq!(haqumei.g2p_kana("バヽ").unwrap(), "バハ");
        assert_eq!(haqumei.g2p_kana("かゞ").unwrap(), "カガ");
        assert_eq!(haqumei.g2p_kana("ハヾ").unwrap(), "ハバ");
    }

    #[test]
    fn test_g2p_mapping_integrity() {
        let mut ojt = OpenJTalk::new().unwrap();
        let text = "吾輩は猫である。名前　はまだ無　い。𰻞𰻞麺を、　食べたい。";
        let mapping = ojt.g2p_mapping(text).unwrap();
        let reconstructed: String = mapping.into_iter().map(|e| e.word).collect();
        assert_eq!(reconstructed, text);
    }

    #[test]
    fn test_g2p_mapping_unknown_word_rare_kanji_mix() {
        let mut ojt = OpenJTalk::new().unwrap();
        let mapping = ojt.g2p_mapping("𰻞𰻞麺").unwrap();
        assert_eq!(mapping[0].word, "𰻞𰻞");
        assert_eq!(mapping[0].phonemes, ["unk"]);
        assert!(mapping[0].is_unknown);

        assert_eq!(mapping[1].word, "麺");
        assert_eq!(mapping[1].phonemes, ["m", "e", "N"]);
        assert!(!mapping[1].is_unknown);
    }

    #[test]
    fn test_g2p_recovery_after_error() {
        let mut ojt = OpenJTalk::new().unwrap();
        let long_text = "あ".repeat(10000);
        let _ = ojt.g2p(&long_text); // Returns error

        let result = ojt.g2p("復帰").unwrap().join(" ");
        assert_eq!(result, "f u cl k i");
    }

    #[test]
    fn test_g2p_symbols_and_control_chars() {
        let mut ojt = OpenJTalk::new().unwrap();
        let result = ojt.g2p("#$%&'()\n\t").unwrap();
        // Should not crash and should return something (or empty)
        assert!(result.is_empty() || !result.is_empty());
    }
}
