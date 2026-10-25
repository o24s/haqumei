use haqumei::open_jtalk::Dictionary;
use haqumei::{Haqumei, HaqumeiOptions, MecabDictIndexCompiler};

fn engine(csv: &str) -> (tempfile::TempDir, Haqumei) {
    let _dictionary = Dictionary::from_embedded().unwrap();
    let dict_dir = dirs::cache_dir().unwrap().join("haqumei/decompressed");
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("user.csv");
    let output = temp.path().join("user.dic");
    std::fs::write(&source, csv).unwrap();
    MecabDictIndexCompiler::new()
        .dict_dir(&dict_dir)
        .userdict_out_path(&output)
        .add_input_file(&source)
        .run()
        .unwrap();
    let engine = Haqumei::from_path_with_userdict(
        &dict_dir,
        &output,
        HaqumeiOptions {
            protect_user_dict_readings: true,
            ..Default::default()
        },
    )
    .unwrap();
    (temp, engine)
}

#[test]
fn protected_iteration_marks_keep_their_registered_readings() {
    let (_temp, mut engine) = engine(concat!(
        "々,1345,1345,-20000,名詞,一般,*,*,*,*,々,ノマ,ノマ,0/2,C1\n",
        "ゝ,1345,1345,-20000,名詞,一般,*,*,*,*,ゝ,オドリジ,オドリジ,0/4,C1\n",
    ));
    for (text, surface, expected) in [
        ("山々", "々", "ノマ"),
        ("山々々", "々", "ノマ"),
        ("やまゝ", "ゝ", "オドリジ"),
        ("やまゝゝ", "ゝ", "オドリジ"),
    ] {
        let features = engine.run_frontend(text).unwrap();
        let registered: Vec<_> = features.iter().filter(|f| f.string == surface).collect();
        assert_eq!(registered.len(), text.matches(surface).count());
        for feature in registered {
            assert_eq!(feature.read, expected, "{text}");
            assert_eq!(feature.pron, expected, "{text}");
        }
    }
    let (_temp, mut following) =
        self::engine("活,1345,1345,-20000,名詞,一般,*,*,*,*,活,サンプル,サンプル,1/4,C1\n");
    let features = following.run_frontend("学生々活").unwrap();
    assert_eq!(
        features.iter().find(|f| f.string == "活").unwrap().pron,
        "サンプル"
    );
}

#[test]
fn protected_loanwords_do_not_move_the_phrase_nucleus() {
    let (_temp, mut engine) =
        engine("イェロー,1345,1345,-20000,名詞,一般,*,*,*,*,イェロー,イエロー,イエロー,2/4,C1\n");
    let restored = engine.run_frontend("累積イェロー").unwrap();
    engine.options.restore_loanword_kana = false;
    let unchanged = engine.run_frontend("累積イェロー").unwrap();
    assert_eq!(restored, unchanged);
    assert_eq!(restored[0].acc, 6);
    let word = restored.iter().find(|f| f.string == "イェロー").unwrap();
    assert_eq!(word.pron, "イエロー");
    assert_eq!(word.mora_size, 4);
}

#[test]
fn protected_unit_letter_keeps_the_registered_symbol_reading() {
    let (_temp, mut engine) =
        engine("ｇ,4,4,-10000,記号,アルファベット,*,*,*,*,ｇ,ジー,ジー,0/2,*\n");
    let features = engine.run_frontend("5ｇ").unwrap();
    let letter = features.iter().find(|f| f.string == "ｇ").unwrap();
    assert_eq!(letter.read, "ジー");
    assert_eq!(letter.pos, "記号");
    assert_eq!(letter.acc, 0);
}

#[test]
fn numeral_reanalysis_keeps_registered_words_and_splits_native_counts() {
    let (_temp, mut engine) = engine(concat!(
        "三三−〇一,1348,1348,-10000,名詞,固有名詞,一般,*,*,*,三三−〇一,サンプル,サンプル,1/4,C1\n",
        "正一,1351,1351,-10000,名詞,固有名詞,人名,名,*,*,正一,ショウイチ,ショーイチ,1/4,C1\n",
        "二人,1345,1345,-10000,名詞,一般,*,*,*,*,二人,フタリ,フタリ,0/3,C1\n",
        "一駅,1345,1345,-10000,名詞,一般,*,*,*,*,一駅,ヒトエキ,ヒトエキ,2/4,C1\n",
        "十日夜,1375,1375,-10000,名詞,副詞可能,*,*,*,*,十日夜,トオカヤ,トーカヤ,0/4,C1\n",
        "三重塔,1345,1345,4615,名詞,一般,*,*,*,*,三重塔,サンジュウノトウ,サンジューノトー,5/7,C1\n",
        "十回転,1345,1345,2137,名詞,一般,*,*,*,*,十回転,ジュッカイテン,ジュッカイテン,3/6,C1\n",
    ));
    for protected in [false, true] {
        engine.options.protect_user_dict_readings = protected;
        for (text, expected) in [
            ("三三−〇一です", "サンプルデス"),
            ("四十二人", "ヨンジューニニン"),
            ("二人", "フタリ"),
            ("十一駅", "ジューイチエキ"),
            ("二十日夜", "ハツカヨル"),
            ("十三重塔", "ジューサンジューノトー"),
            ("七十回転", "ナナジュッカイテン"),
        ] {
            assert_eq!(
                engine.g2k(text).unwrap(),
                expected,
                "{text}, protected={protected}"
            );
        }
        let morphs = engine.run_mecab_detailed("正一一万円を払う").unwrap();
        assert_eq!(morphs[0].surface, "正一");
        assert!(morphs[0].is_from_user_dictionary());
        for text in ["十三重塔", "七十回転"] {
            let morphs = engine.run_mecab_detailed(text).unwrap();
            assert!(morphs.iter().any(|m| m.is_from_user_dictionary()
                && m.surface == text.chars().skip(1).collect::<String>()));
            assert!(
                engine
                    .analyze_lattice(text)
                    .unwrap()
                    .iter()
                    .any(|m| m.dictionary_index == 1
                        && m.is_best
                        && m.surface == text.chars().skip(1).collect::<String>())
            );
        }
    }
}
