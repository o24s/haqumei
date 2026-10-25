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
