use haqumei::{Haqumei, HaqumeiOptions};

fn loan_engine(protect: bool) -> Haqumei {
    let mut engine = Haqumei::with_options(HaqumeiOptions {
        protect_user_dict_accents: protect,
        ..Default::default()
    })
    .unwrap();
    engine.set_morph_filter(|_, _, morphs| {
        for morph in morphs.iter_mut().filter(|m| m.surface == "ローン") {
            morph.feature = "ローン,名詞,一般,*,*,*,*,ローン,ローン,ローン,2/3,C1".into();
            morph.dictionary_index = 1;
        }
    });
    engine
}

#[test]
fn protects_registered_nucleus_inside_a_phrase() {
    let mut engine = loan_engine(true);
    for (text, protected, unprotected) in [("ローン", 2, 1), ("住宅ローン", 6, 5)] {
        for readings in [false, true] {
            engine.options.protect_user_dict_readings = readings;
            engine.options.protect_user_dict_accents = true;
            let features = engine.run_frontend(text).unwrap();
            assert_eq!(features[0].acc, protected, "{text}");
            assert_eq!(features, engine.run_frontend_detailed(text).unwrap().0);
            let labels = engine.extract_fullcontext(text).unwrap();
            assert_eq!(
                labels[1]
                    .accent_phrase_curr
                    .as_ref()
                    .unwrap()
                    .accent_position,
                protected as u8,
            );
            engine.options.protect_user_dict_accents = false;
            assert_eq!(engine.run_frontend(text).unwrap()[0].acc, unprotected);
        }
    }
}

#[test]
fn does_not_freeze_nucleus_moved_by_chaining() {
    let mut engine = loan_engine(true);
    let protected = engine.run_frontend("ローンメール").unwrap();
    assert_eq!(protected[0].acc, 2);
    engine.options.protect_user_dict_accents = false;
    assert_eq!(protected, engine.run_frontend("ローンメール").unwrap());
    engine.options.retreat_acc_nuc = false;
    assert_eq!(engine.run_frontend("ローンメール").unwrap()[0].acc, 3);
}

#[test]
fn same_spelling_from_system_dictionary_is_not_protected() {
    let mut engine = loan_engine(true);
    engine.set_morph_filter(|_, _, morphs| {
        let mut seen = false;
        for morph in morphs.iter_mut().filter(|m| m.surface == "ローン") {
            morph.feature = "ローン,名詞,一般,*,*,*,*,ローン,ローン,ローン,2/3,C1".into();
            morph.dictionary_index = if seen { 0 } else { 1 };
            seen = true;
        }
    });
    let features = engine.run_frontend("ローン、ローン").unwrap();
    let nuclei: Vec<_> = features
        .iter()
        .filter(|f| f.string == "ローン")
        .map(|f| f.acc)
        .collect();
    assert_eq!(nuclei, [2, 1]);
}

#[test]
fn changed_pronunciation_does_not_preserve_a_stale_registration() {
    let mut engine = loan_engine(true);
    engine.set_morph_filter(|_, _, morphs| {
        for morph in morphs.iter_mut().filter(|m| m.surface == "何") {
            morph.feature = "何,名詞,代名詞,一般,*,*,*,何,ナニ,ナニ,2/2,C1".into();
            morph.dictionary_index = 1;
        }
    });
    engine.options.predict_nani = false;
    let protected = engine.run_frontend("何でも").unwrap();
    assert_eq!(protected[0].pron, "ナン");
    assert_eq!(protected[0].acc, 1);
    engine.options.protect_user_dict_accents = false;
    assert_eq!(protected, engine.run_frontend("何でも").unwrap());
}

#[test]
fn enabling_accent_protection_preserves_unknown_symbol_segmentation() {
    let mut engine = Haqumei::new().unwrap();
    for text in ["笑ヾ(≧▽≦)", "テスト😎！？", "𰻞𰻞麺"] {
        engine.options.protect_user_dict_accents = false;
        let expected = engine.run_frontend(text).unwrap();
        engine.options.protect_user_dict_accents = true;
        assert_eq!(expected, engine.run_frontend(text).unwrap(), "{text}");
    }
}

#[test]
fn prefix_split_calculates_each_phrase_nucleus() {
    let mut engine = Haqumei::new().unwrap();
    engine.set_morph_filter(|_, _, morphs| {
        for morph in morphs.iter_mut().filter(|m| m.surface == "本") {
            morph.feature = "本,接頭詞,名詞接続,*,*,*,*,本,ホン,ホン,1/2,P1".into();
            morph.dictionary_index = 1;
        }
    });
    for protect in [false, true] {
        engine.options.protect_user_dict_accents = protect;
        engine.options.split_prefix_accent_phrase = true;
        let features = engine.run_frontend("本商品を購入する").unwrap();
        assert_eq!((features[0].acc, features[0].mora_size), (1, 2));
        assert_eq!((features[1].acc, features[1].chain_flag), (1, 0));
        assert_eq!(
            features,
            engine.run_frontend_detailed("本商品を購入する").unwrap().0
        );
        let labels = engine.extract_fullcontext("本商品を購入する").unwrap();
        let prefix = labels[1].accent_phrase_curr.as_ref().unwrap();
        assert_eq!((prefix.accent_position, prefix.mora_count), (1, 2));

        engine.options.split_prefix_accent_phrase = false;
        let features = engine.run_frontend("本商品を購入する").unwrap();
        assert_eq!((features[0].acc, features[1].chain_flag), (3, 1));
    }
}
