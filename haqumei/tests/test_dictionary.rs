//! 同梱辞書が巻き戻っていないことを見る回帰テスト。
//!
//! `haqumei/dictionary/` は上流から同期していたので、今後の同期のたびに既定の読みや
//! 分割が巻き戻る可能性がある。
//! 網羅的な検証は別に持っているが、最小限をこのファイルに置く。
//!
//! 各項目に負の対照を置く。単語コストを動かす修正は、狙った語だけに効いて
//! いる保証が他に無いためで、対照が無いと隣の語を黙って壊しても気付けない。
//!
//! `Haqumei::new()` は埋め込み辞書を使うので、ここが落ちたときは
//! `build.rs` の `DICTIONARY_URL` が指すリリース資産が古い可能性もある。

#[test]
fn corrected_entries_record_the_pronounced_mora_count() {
    use haqumei_jpreprocess_core::pronunciation::Pronunciation;

    let dictionary = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("dictionary");
    let targets = [
        "登坂車線",
        "英文法",
        "ウォーミング",
        "ウィ〜ン",
        "ウェストモーランド",
        "ウォ〜ン",
        "ウフィッツィ",
        "エトキシフェニル",
        "チオホスフェイト",
        "テトラエチルピロホスフェイト",
        "ネブカドネツァル",
        "ヒンドゥ",
        "プレグナジェン",
        "体じゅう",
        "七五三参り",
        "茸狩り",
        "ｉＰｈｏｎｅ",
    ];
    let mut seen = std::collections::HashSet::new();
    for name in [
        "rare_syllables.csv",
        "naist-jdic.csv",
        "unidic-csj.csv",
        "heteronyms.csv",
    ] {
        let csv = std::fs::read_to_string(dictionary.join(name)).unwrap();
        for line in csv.lines() {
            let fields: Vec<_> = line.split(',').collect();
            if name != "rare_syllables.csv" && !targets.contains(&fields[0]) {
                continue;
            }
            let (accent, count) = fields[13].split_once('/').unwrap();
            let pronunciation = Pronunciation::parse(fields[12], 0).unwrap();
            assert_eq!(
                pronunciation.moras().len(),
                count.parse::<usize>().unwrap(),
                "{name}: {line}"
            );
            assert!(
                accent.parse::<usize>().unwrap() <= pronunciation.moras().len(),
                "{name}: {line}"
            );
            seen.insert(fields[0].to_owned());
        }
    }
    assert!(targets.iter().all(|word| seen.contains(*word)));
}

#[cfg(test)]
mod tests {
    use haqumei::Haqumei;

    /// エントリが欠けていて分割に負けていた語。
    #[test]
    fn test_missing_entries() {
        let mut haqumei = Haqumei::new().unwrap();
        for (text, expected) in [
            // 誤 に名詞の ゴ が無く、動詞の アヤマ しか無かった
            ("誤作動", "ゴサドー"),
            ("誤検知", "ゴケンチ"),
            ("誤り", "アヤマリ"),
            ("誤解", "ゴカイ"),
            ("正誤", "セーゴ"),
            // 長寿 という語のエントリに分割を奪われて 長寿 + 命 になっていた
            ("長寿命", "チョージュミョー"),
            ("短寿命", "タンジュミョー"),
            ("寿命", "ジュミョー"),
            // 上の が地名として 1 形態素になり、方 が接尾辞の カタ になっていた
            ("上の方", "ウエノホー"),
            ("下の方", "シタノホー"),
            ("上の橋", "ウエノバシ"),
            ("上野", "ウエノ"),
            // 問 の名詞は トイ しか無いので、分割が起きた時点で読めない
            ("過去問", "カコモン"),
            ("過去問題集", "カコモンダイシュー"),
            // 和英 は人名の カズヒデ しか無かった。英和 と対にしてある
            ("和英辞典", "ワエージテン"),
            ("英和辞典", "エーワジテン"),
            // 文法 のエントリはあるが 英文 + 法 に割れて負ける
            ("英文法", "エーブンポー"),
            ("英文", "エーブン"),
            ("文法", "ブンポー"),
            // 短 + 答 に割れて タンコタエ になっていた
            ("短答", "タントー"),
            ("答案", "トーアン"),
        ] {
            assert_eq!(haqumei.g2k(text).unwrap(), expected, "input: {}", text);
        }
    }

    /// 既定の読みが実態と合っていなかった語。
    #[test]
    fn test_default_readings() {
        let mut haqumei = Haqumei::new().unwrap();
        for (text, expected) in [
            // ビンラン と同コストで並んでいて、ビンラン が勝っていた
            ("便覧", "ベンラン"),
            ("郵便", "ユービン"),
            // 唯一のエントリが ナイブンピツ だった
            ("内分泌", "ナイブンピ"),
            ("分泌", "ブンピツ"),
            // トクホン は戦前の教科書の読み。現代の書名は ドクホン
            ("読本", "ドクホン"),
            // 時 に トキ が無く、角 の既定が ツノ だった
            ("時を戻す", "トキヲモドス"),
            ("時計", "トケー"),
            ("街角", "マチカド"),
            // 皇 に音読みが無く、居城 / 城内 が人名のエントリに負けていた
            ("皇室", "コーシツ"),
            ("天皇", "テンノー"),
            ("居城", "キョジョー"),
            ("城内", "ジョーナイ"),
        ] {
            assert_eq!(haqumei.g2k(text).unwrap(), expected, "input: {}", text);
        }
    }

    /// `char.def` の `KATAKANA` が `INVOKE=1` であること。
    ///
    /// `INVOKE=0` は「辞書に載る語があれば未知語候補を作らない」意味なので、
    /// 長音記号や小書き仮名が語頭に立つ形態素ができてしまう
    /// (クールフェーラック が ク + ールフェーラック に割れる)。
    #[test]
    fn test_katakana_is_not_oversplit() {
        let mut haqumei = Haqumei::new().unwrap();

        for text in ["クールフェーラック", "コロリョーフ", "オミクロン"] {
            let mapping = haqumei.g2p_mapping(text).unwrap();
            assert_eq!(mapping.len(), 1, "分割された: {} -> {:?}", text, mapping);
        }

        // カタカナの ヘ が助詞と解析されて エッテ と読まれていた
        assert_eq!(haqumei.g2k("ヘッテ写本").unwrap(), "ヘッテシャホン");
    }

    /// 辞書にエントリのあるカタカナ語が、未知語候補に負けていないこと。
    ///
    /// 未知語候補は辞書の 名詞-一般 と文脈 ID が同じなので、エントリのコストが
    /// 未知語候補より高いと、辞書にあるのに `is_unknown` が立つ。
    #[test]
    fn test_known_katakana_is_not_unknown() {
        let mut haqumei = Haqumei::new().unwrap();
        for text in ["コーチ", "ブログ", "サプリメント", "クオリティー"] {
            let mapping = haqumei.g2p_mapping_detailed(text).unwrap();
            assert!(
                mapping.iter().all(|m| !m.is_unknown),
                "未知語になった: {} -> {:?}",
                text,
                mapping
            );
        }

        // 逆に、エントリの無い語は未知語のままであること
        let mapping = haqumei.g2p_mapping_detailed("クールフェーラック").unwrap();
        assert!(mapping.iter().any(|m| m.is_unknown));
    }

    #[test]
    fn lake_names_keep_their_nucleus_after_shortening() {
        let mut engine = Haqumei::new().unwrap();
        for (text, expected, accent) in [
            ("宮沢湖", "ミヤザワコ", 4),
            ("琵琶湖", "ビワコ", 0),
            ("諏訪湖", "スワコ", 0),
            ("湖", "ミズウミ", 3),
        ] {
            assert_eq!(engine.g2k(text).unwrap(), expected);
            assert_eq!(engine.run_frontend(text).unwrap()[0].acc, accent, "{text}");
        }
    }

    #[test]
    fn split_name_retains_each_original_form() {
        let mut engine = Haqumei::new().unwrap();
        let nodes = engine.run_frontend("山本五十六").unwrap();
        let words: Vec<_> = nodes
            .iter()
            .map(|n| (&*n.string, &*n.orig, &*n.pron))
            .collect();
        assert_eq!(
            words,
            [
                ("山本", "山本", "ヤマモト"),
                ("五十六", "五十六", "イソロク")
            ]
        );
    }
}
