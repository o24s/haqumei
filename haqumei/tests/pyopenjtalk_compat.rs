#[cfg(test)]
mod tests {
    use haqumei::{Haqumei, HaqumeiOptions};

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
            modify_kanji_yomi: true,
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
}
