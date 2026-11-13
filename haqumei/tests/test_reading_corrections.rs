use haqumei::{Haqumei, HaqumeiOptions, OpenJTalk};

#[test]
fn elongated_ha_before_breath_predicates_has_its_own_accent_phrase() {
    let mut engine = Haqumei::new().unwrap();
    for (text, span, breath) in [
        ("私はーって何度も言い直しながら自己紹介した", 1..3, false),
        ("私、はーって息を吹きかけたんです", 2..4, true),
        ("私はーって息を吹きかけたんです", 1..3, true),
        ("私はーっと息をふきかけたんです", 1..3, true),
        ("私ははーって息を吹きかけたんです", 2..4, true),
        ("私はーって言い直してから息を吹きかけたんです", 1..3, false),
        ("私はーって息を吐かずに言った", 1..3, false),
        ("私はーって息を吐く暇もなく名乗った。", 1..3, false),
        ("私はーって息を吐く時間もなく名乗った。", 1..3, false),
        ("私はーっと息を吹きかけたことはない。", 1..3, false),
        ("私はーっと息を吹きかけた時間は短かった。", 1..3, false),
        ("窓に、はーっと息を吹きかけないでください。", 3..5, true),
        ("窓にはーっと息を吹きかけないでください。", 2..4, true),
        ("窓に、はーっと息を吹きかけちゃだめです。", 3..5, true),
    ] {
        let (pron, pos, accent, chain, phoneme) = if breath {
            ("ハー", "副詞", 1, 0, "h")
        } else {
            ("ワー", "助詞", 0, 1, "w")
        };
        let features = engine.run_frontend(text).unwrap();
        assert_eq!(
            features,
            engine.run_frontend_detailed(text).unwrap().0,
            "{text}"
        );
        let mapping = engine.g2p_mapping_detailed(text).unwrap();
        let word = mapping.iter().find(|word| word.char_span == span).unwrap();
        assert_eq!(word.word, "はー", "{text}");
        assert_eq!(word.pron, pron, "{text}");
        assert_eq!(word.pos, pos, "{text}");
        assert_eq!(word.mora_count, 2, "{text}");
        assert_eq!(word.accent_nucleus, accent, "{text}");
        assert_eq!(word.chain_flag, chain, "{text}");
        assert_eq!(word.phonemes[0].as_str(), phoneme, "{text}");
        assert_eq!(
            engine.g2p_mapping(text).unwrap(),
            engine.g2p_candidates(text).unwrap().candidates[0].words,
            "{text}"
        );
        if text.starts_with("私はは") {
            let particle = mapping
                .iter()
                .find(|word| word.char_span == (1..2))
                .unwrap();
            assert_eq!(particle.pron, "ワ");
        }
    }

    let text = "私はーって息を吹きかけたんです";
    let contiguous = engine.g2k(text).unwrap();
    assert!(
        engine
            .g2k("私は　ーって息を吹きかけたんです")
            .unwrap()
            .starts_with("ワタシワ")
    );
    engine.options.protect_user_dict_readings = true;
    engine.set_morph_filter(|_, _, morphs| {
        if let Some(ha) = morphs.iter_mut().find(|morph| morph.surface == "は") {
            ha.dictionary_index = 1;
        }
    });
    assert!(engine.g2k(text).unwrap().starts_with("ワタシワーッテ"));
    engine.clear_morph_filter();
    assert_eq!(engine.g2k(text).unwrap(), contiguous);

    engine.options.modify_context_reading = false;
    assert!(engine.g2k(text).unwrap().starts_with("ワタシワーッテ"));
}

#[test]
fn breath_reading_respects_space_and_explicit_morph_edits() {
    let mut engine = Haqumei::new().unwrap();
    for text in [
        "私は ーって息を吹きかけたんです",
        "私はー　って息を吹きかけたんです",
        "私はーっと 息を吹きかけたんです",
        "私はーって息を吐か　ずに言った",
    ] {
        assert!(
            engine.g2k(text).unwrap().starts_with("ワタシワー"),
            "{text}"
        );
        assert_eq!(
            engine.run_frontend(text).unwrap(),
            engine.run_frontend_detailed(text).unwrap().0,
            "{text}"
        );
    }
    let kana = engine.g2k("塀を十重 に囲んだ。").unwrap();
    assert_eq!(
        engine
            .g2k("彼はコーヒーを飲み、塀を十重 に囲んだ。")
            .unwrap(),
        format!("カレワコーヒーヲノミ、{kana}")
    );
    let kana = engine.g2k("従 三位になったと告げた。").unwrap();
    assert_eq!(
        engine
            .g2k("彼、はーって息をかけてから、従 三位になったと告げた。")
            .unwrap(),
        format!("カレ、ハーッテイキヲカケテカラ、{kana}")
    );
    engine.set_morph_filter(|_, _, morphs| {
        if let Some(ha) = morphs.iter_mut().find(|morph| morph.surface == "は") {
            ha.feature = "は,助詞,係助詞,*,*,*,*,は,ワ,ワ,0/1,*".to_owned();
        }
    });
    for text in ["私はーって息を吹きかけた", "私はｰって息を吹きかけた"] {
        assert!(engine.g2k(text).unwrap().starts_with("ワタシワーッテ"));
        assert_eq!(
            engine.g2p_mapping(text).unwrap(),
            engine.g2p_candidates(text).unwrap().candidates[0].words
        );
    }
}

#[test]
fn ha_reading_uses_topic_and_utterance_boundaries() {
    let mut engine = Haqumei::new().unwrap();
    for (text, reading) in [
        (
            "私はぁ、別に構わないけど。",
            "ワタシワー、ベツニカマワナイケド。",
        ),
        ("先輩、はぁ、疲れました。", "センパイ、ハー、ツカレマシタ。"),
        (
            "彼がはぁ、もう無理だ、とため息をついた。",
            "カレガハー、モームリダ、トタメイキヲツイタ。",
        ),
        (
            "彼はぁ、うそだろ、と叫んだ。",
            "カレハー、ウソダロ、トサケンダ。",
        ),
        (
            "お前はぁ？何言ってるんだ。",
            "オマエハー？ナニイッテルンダ。",
        ),
        (
            "今日はー、いい天気ですね。",
            "キョーワー、イイテンキデスネ。",
        ),
        ("はー……", "ハー……"),
        ("「 はー、やっと着いた」", "「ハー、ヤットツイタ」"),
        ("「は ー、やっと着いた」", "「ワー、ヤットツイタ」"),
        ("うん …… はー、疲れた。", "ウン……ハー、ツカレタ。"),
        ("うん 、はー、疲れた。", "ウン、ハー、ツカレタ。"),
        ("うん、はー 、疲れた。", "ウン、ハー、ツカレタ。"),
        ("「はー 」と息をついた。", "「ハー」トイキヲツイタ。"),
        (
            "彼はぁ、やっと終わったとため息をついた。",
            "カレハー、ヤットオワッタトタメイキヲツイタ。",
        ),
        (
            "えっ……はー、そういうことか。",
            "エッ……ハー、ソーユウコトカ。",
        ),
        (
            "あの人……はー、もう帰ったよ。",
            "アノヒト……ワー、モーカエッタヨ。",
        ),
        (
            "録音した答えは「私はー、賛成です」だった。",
            "ロクオンシタコタエワ「ワタシワー、サンセーデス」ダッタ。",
        ),
        (
            "舞台裏で息を吐いてから「はー、緊張した」とつぶやいた。",
            "ブタイウラデイキヲハイテカラ「ハー、キンチョーシタ」トツブヤイタ。",
        ),
        (
            "ちょっと待って。はー、はー、息が切れた。",
            "チョットマッテ。ハー、ハー、イキガキレタ。",
        ),
        (
            "辛いものを食べてはーはーする。",
            "ツライモノヲタベテハーハースル。",
        ),
        (
            "うちのははは駅で待っています。",
            "ウチノハハワエキデマッテイマス。",
        ),
        (
            "先輩、は、はい！すぐ行きます！",
            "センパイ、ハ、ハイ！スグイキマス！",
        ),
        (
            "部長、は、はい！分かりました！",
            "ブチョー、ハ、ハイ！ワカリマシタ！",
        ),
        (
            "田中さん、は、はい！分かりました！",
            "タナカサン、ハ、ハイ！ワカリマシタ！",
        ),
        ("「は、はい」と答えた。", "「ハ、ハイ」トコタエタ。"),
        ("は、はぁ", "ハ、ハー"),
        (
            "「は、はぁ、分かりました」と答えた。",
            "「ハ、ハー、ワカリマシタ」トコタエタ。",
        ),
        ("「は、はあ」と返事をした。", "「ハ、ハー」トヘンジヲシタ。"),
        (
            "「は、はーい」と返事をした。",
            "「ハ、ハーイ」トヘンジヲシタ。",
        ),
        ("先輩、は、はぁ……。", "センパイ、ハ、ハー……。"),
        ("先輩は、はぁ……。", "センパイワ、ハー……。"),
        ("は、はぁ、と答えた。", "ハ、ハー、トコタエタ。"),
        (
            "はい、は、はぁ、よりも明瞭に聞こえた。",
            "ハイ、ワ、ハー、ヨリモメーリョーニキコエタ。",
        ),
        (
            "はい、は、はぁ……よりも明瞭に聞こえた。",
            "ハイ、ワ、ハー……ヨリモメーリョーニキコエタ。",
        ),
        (
            "「は、はい」と彼は答えた。",
            "「ハ、ハイ」トカレワコタエタ。",
        ),
        (
            "「は、はい！」 と小さく答えた。",
            "「ハ、ハイ！」トチーサクコタエタ。",
        ),
        ("「は、はい」", "「ハ、ハイ」"),
        ("「は、はい、分かりました」", "「ハ、ハイ、ワカリマシタ」"),
        (
            "「は、はい、分かりました」と彼は言った。",
            "「ハ、ハイ、ワカリマシタ」トカレワイッタ。",
        ),
        (
            "「は、はい、分かりました」の部分を聞き直した。",
            "「ハ、ハイ、ワカリマシタ」ノブブンヲキキナオシタ。",
        ),
        (
            "「は、はい！分かりました」と彼は答えた。",
            "「ハ、ハイ！ワカリマシタ」トカレワコタエタ。",
        ),
        (
            "「は、はい」とすぐに返事した。",
            "「ハ、ハイ」トスグニヘンジシタ。",
        ),
        (
            "「は、はい、いいえ」を読み上げる。",
            "「ワ、ハイ、イーエ」ヲヨミアゲル。",
        ),
        (
            "先輩は、はい！すぐ行きます！",
            "センパイワ、ハイ！スグイキマス！",
        ),
        (
            "助詞の「は」は「わ」と発音する。",
            "ジョシノ「ハ」ワ「ワ」トハツオンスル。",
        ),
        ("助詞の「は」", "ジョシノ「ハ」"),
        ("ひらがなの「は」", "ヒラガナノ「ハ」"),
        (
            "太郎が 助詞の「は」について話した。",
            "タローガジョシノ「ハ」ニツイテハナシタ。",
        ),
        (
            "「は」を平仮名で書いてください。",
            "「ハ」ヲヒラガナデカイテクダサイ。",
        ),
        (
            "次の空欄は助詞です。答えは「は」と読んでください。",
            "ツギノクーランワジョシデス。コタエワ「ワ」トヨンデクダサイ。",
        ),
    ] {
        assert_eq!(engine.g2k(text).unwrap(), reading, "{text}");
        let frontend = engine.run_frontend(text).unwrap();
        assert_eq!(
            frontend,
            engine.run_frontend_detailed(text).unwrap().0,
            "{text}"
        );
        assert_eq!(
            engine.g2p_mapping(text).unwrap(),
            engine.g2p_candidates(text).unwrap().candidates[0].words,
            "{text}"
        );
    }

    let mapping = engine
        .g2p_mapping_detailed("私はぁ、別に構わないけど。")
        .unwrap();
    let particle = mapping.iter().find(|word| word.word == "はぁ").unwrap();
    assert_eq!(particle.pos, "助詞");
    assert_eq!(particle.pos_group1, "係助詞");
    assert_eq!(particle.char_span, 1..3);
    assert_eq!(particle.mora_count, 2);
    assert_eq!(particle.chain_flag, 1);
    assert_eq!(particle.phonemes[0].as_str(), "w");

    let mapping = engine
        .g2p_mapping_detailed("「は、はぁ」と答えた。")
        .unwrap();
    let fragment = mapping
        .iter()
        .find(|word| word.char_span == (1..2))
        .unwrap();
    assert_eq!(fragment.phonemes[0].as_str(), "h");
    let response = mapping
        .iter()
        .find(|word| word.char_span == (3..5))
        .unwrap();
    assert_eq!(response.phonemes[0].as_str(), "h");
    assert_eq!(response.pos, "感動詞");
}

#[test]
fn sigh_after_adverbs_keeps_its_reading() {
    let mut engine = Haqumei::new().unwrap();
    for text in [
        "ああもうはぁ、疲れた。",
        "おもわずはぁ、疲れた。",
        "ついはぁ、疲れた。",
        "あいにくはぁ、もう無理だ。",
    ] {
        let words = engine.g2p_mapping_detailed(text).unwrap();
        let sigh = words.iter().find(|word| word.word == "はぁ").unwrap();
        assert_eq!(sigh.pron, "ハー", "{text}");
        assert_eq!(sigh.pos, "感動詞", "{text}");
        assert_eq!(
            engine.run_frontend(text).unwrap(),
            engine.run_frontend_detailed(text).unwrap().0,
            "{text}"
        );
    }
    for text in [
        "私はぁ、疲れた。",
        "彼にはぁ、何も言わないで。",
        "本来はぁ、ここで降りるんです。",
    ] {
        let words = engine.g2p_mapping_detailed(text).unwrap();
        let topic = words.iter().find(|word| word.word == "はぁ").unwrap();
        assert_eq!(topic.pron, "ワー", "{text}");
        assert_eq!(topic.pos, "助詞", "{text}");
    }
}

#[test]
fn quote_particles_after_exclamation_and_question_keep_sigh_reading() {
    let mut engine = Haqumei::new().unwrap();
    for (text, expected) in [
        ("ときどきはぁ、疲れた！と息をついた。", "ハー"),
        ("ときどきはぁ、疲れた、と息をついた。", "ハー"),
        ("彼にはぁ、何だよ？ と聞き返した。", "ハー"),
        ("彼にはぁ、何だよ！？と聞き返した。", "ハー"),
        ("私はぁ、賛成です！「理由は後で話す」と付け加えた。", "ワー"),
    ] {
        let words = engine.g2p_mapping_detailed(text).unwrap();
        let ha = words.iter().find(|word| word.word == "はぁ").unwrap();
        assert_eq!(ha.pron, expected, "{text}");
        assert_eq!(
            engine.run_frontend(text).unwrap(),
            engine.run_frontend_detailed(text).unwrap().0,
            "{text}"
        );
    }
}

#[test]
fn topic_ha_after_adverbs_preserves_sigh_boundaries() {
    let mut engine = Haqumei::new().unwrap();
    for (left, right) in [
        ("実際", "、予約が必要です。"),
        ("元々", "、住宅だった。"),
        ("少し", "、役に立ったかな。"),
        ("ちょっと", "、反省してよ。"),
        ("もう少し", "、待てます。"),
        ("いつも", "、歩いて帰ります。"),
        ("すぐに", "、決められません。"),
        ("ゆっくり", "、歩けないんです。"),
    ] {
        for (separator, expected) in [("", "ワー"), (" ", "ハー"), ("　", "ハー"), ("、", "ハー")]
        {
            let text = format!("{left}{separator}はぁ{right}");
            let start = left.chars().count() + separator.chars().count();
            let words = engine.g2p_mapping_detailed(&text).unwrap();
            let word = words
                .iter()
                .find(|w| w.char_span == (start..start + 2))
                .unwrap();
            assert_eq!(word.pron, expected, "{text}");
            assert_eq!(
                engine.run_frontend(&text).unwrap(),
                engine.run_frontend_detailed(&text).unwrap().0,
                "{text}"
            );
        }
    }
    for text in [
        "ちょっとはぁ、何それ。",
        "少しはぁ、息を吐いてみて。",
        "ゆっくりはぁ、息を吐かないで。",
    ] {
        let words = engine.g2p_mapping_detailed(text).unwrap();
        let word = words.iter().find(|w| w.word == "はぁ").unwrap();
        assert!(["ハー", "ワー"].contains(&word.pron.as_str()), "{text}");
    }
    let text = "少しはぁ、役に立ったかな。";
    engine.options.protect_user_dict_readings = true;
    engine.set_morph_filter(|_, _, morphs| {
        for morph in morphs.iter_mut().filter(|m| m.surface == "はぁ") {
            morph.dictionary_index = 1;
        }
    });
    let words = engine.g2p_mapping_detailed(text).unwrap();
    assert_eq!(
        words.iter().find(|w| w.word == "はぁ").unwrap().pron,
        "ハー"
    );
    engine.set_morph_filter(|_, _, morphs| {
        for morph in morphs.iter_mut().filter(|m| m.surface == "はぁ") {
            morph.feature = "はぁ,感動詞,*,*,*,*,*,はぁ,ヒー,ヒー,1/2,*".to_owned();
        }
    });
    let words = engine.g2p_mapping_detailed(text).unwrap();
    assert_eq!(
        words.iter().find(|w| w.word == "はぁ").unwrap().pron,
        "ヒー"
    );
}

#[test]
fn laughter_space_boundaries_agree_between_apis() {
    let mut engine = Haqumei::new().unwrap();
    for prefix in ["彼は", "それ見て、", "彼は、", "「"] {
        for space in [" ", "　", "  "] {
            for count in 2..8 {
                let laugh = "は".repeat(count);
                let close = if prefix == "「" { "」" } else { "" };
                let text = format!("{prefix}{space}{laugh}{close}と笑った。");
                let simple = engine.run_frontend(&text).unwrap();
                let detailed = engine.run_frontend_detailed(&text).unwrap().0;
                assert_eq!(simple, detailed, "{text}");
                let start = prefix.chars().count() + space.chars().count();
                let words = engine.g2p_mapping_detailed(&text).unwrap();
                let word = words
                    .iter()
                    .find(|w| w.char_span == (start..start + count))
                    .unwrap();
                assert_eq!(word.pron, "ハ".repeat(count), "{text}");
                if prefix.starts_with("彼は") {
                    let topic = words.iter().find(|w| w.char_span == (1..2)).unwrap();
                    assert_eq!(topic.pron, "ワ", "{text}");
                }
                assert_eq!(
                    engine.g2p_mapping(&text).unwrap(),
                    engine.g2p_candidates(&text).unwrap().candidates[0].words,
                    "{text}"
                );
            }
        }
    }
    for text in ["彼は はは はと笑った。", "彼 はははと笑った。"] {
        let simple = engine.run_frontend(text).unwrap();
        assert_eq!(
            simple,
            engine.run_frontend_detailed(text).unwrap().0,
            "{text}"
        );
        engine.options.modify_context_reading = false;
        assert_eq!(simple, engine.run_frontend(text).unwrap(), "{text}");
        engine.options.modify_context_reading = true;
    }
    let text = "彼は はははと笑った。";
    engine.options.protect_user_dict_readings = true;
    engine.set_morph_filter(|_, _, morphs| {
        for morph in morphs.iter_mut().filter(|m| m.char_span == (5..6)) {
            morph.dictionary_index = 1;
        }
    });
    let words = engine.g2p_mapping_detailed(text).unwrap();
    assert_eq!(
        words.iter().find(|w| w.char_span == (5..6)).unwrap().pron,
        "ワ"
    );
    engine.clear_morph_filter();
    engine.set_morph_filter(|_, _, morphs| {
        for morph in morphs.iter_mut().filter(|m| m.char_span == (5..6)) {
            morph.feature = "は,助詞,係助詞,*,*,*,*,は,ワ,ワ,0/1,*".to_owned();
        }
    });
    let words = engine.g2p_mapping_detailed(text).unwrap();
    assert_eq!(
        words.iter().find(|w| w.char_span == (5..6)).unwrap().pron,
        "ワ"
    );
}

#[test]
fn laughter_uses_quotes_and_the_following_verb() {
    let mut engine = Haqumei::new().unwrap();
    for (text, reading) in [
        ("それ見てははって笑った。", "ソレミテハハッテワラッタ。"),
        ("それ見て、ははと笑った。", "ソレミテ、ハハトワラッタ。"),
        (
            "それ見て、はははって笑った。",
            "ソレミテ、ハハハッテワラッタ。",
        ),
        ("それ見てはははって笑った。", "ソレミテワハハッテワラッタ。"),
        (
            "それ見て、はははははははと笑った。",
            "ソレミテ、ハハハハハハハトワラッタ。",
        ),
        ("彼は、はははっと笑った。", "カレワ、ハハハットワラッタ。"),
        (
            "彼女は「ははは」と笑った。",
            "カノジョワ「ハハハ」トワラッタ。",
        ),
        ("「ははははっ！」と笑った。", "「ハハハハッ！」トワラッタ。"),
        (
            "「ははは」 と小声で笑った。",
            "「ハハハ」トコゴエデワラッタ。",
        ),
        (
            "彼は、少しはははっと笑ったものの、大笑いはしなかった。",
            "カレワ、スコシワハハットワラッタモノノ、オーワライワシナカッタ。",
        ),
        (
            "いつも見てはははっと笑って、また再生しては笑った。",
            "イツモミテワハハットワラッテ、マタサイセーシテワワラッタ。",
        ),
        (
            "録音の『母は』を「ははは」と笑いながら読んだ。",
            "ロクオンノ『ハハワ』ヲ「ハハワ」トワライナガラヨンダ。",
        ),
        (
            "録音の『母は』を「ははは」と笑って読んだ。",
            "ロクオンノ『ハハワ』ヲ「ハハワ」トワラッテヨンダ。",
        ),
        ("「ははは　！」と笑った。", "「ハハハ！」トワラッタ。"),
        (
            "「ははは」と低い声で笑った。",
            "「ハハハ」トヒクイコエデワラッタ。",
        ),
        (
            "「ははは」と朗らかに笑った。",
            "「ハハハ」トホガラカニワラッタ。",
        ),
        (
            "「ははは」と答えてから笑った。",
            "「ハハワ」トコタエテカラワラッタ。",
        ),
        ("彼はははと笑った。", "カレワハハトワラッタ。"),
        ("彼はははっと笑った。", "カレワハハットワラッタ。"),
        ("私はははって言いました。", "ワタシワハハッテイイマシタ。"),
        (
            "「ははは昔からよく笑う人」と書いた。",
            "「ハハワムカシカラヨクワラウヒト」トカイタ。",
        ),
    ] {
        assert_eq!(engine.g2k(text).unwrap(), reading, "{text}");
        assert_eq!(
            engine.run_frontend(text).unwrap(),
            engine.run_frontend_detailed(text).unwrap().0,
            "{text}"
        );
    }

    for count in 2..10 {
        for (particle, katakana) in [("と", "ト"), ("っと", "ット")] {
            let laughter = "は".repeat(count);
            let text = format!("彼は{laughter}{particle}笑った。");
            assert_eq!(
                engine.g2k(&text).unwrap(),
                format!("カレワ{}{katakana}ワラッタ。", "ハ".repeat(count)),
                "{text}"
            );
            let mapping = engine.g2p_mapping_detailed(&text).unwrap();
            assert_eq!(mapping[1].char_span, 1..2, "{text}");
            assert_eq!(mapping[1].pos, "助詞", "{text}");
            assert_eq!(mapping[1].phonemes[0].as_str(), "w", "{text}");
            assert_eq!(
                engine.run_frontend(&text).unwrap(),
                engine.run_frontend_detailed(&text).unwrap().0,
                "{text}"
            );
        }
    }
}

#[test]
fn lexical_nouns_and_abstract_koumuri_keep_their_features() {
    check_context_readings(
        &[
            ("恩情を被り、感謝した。", "被り", "コウムリ", 4),
            ("お願い申し上げ候。", "候", "ソウロウ", 4),
            (
                "恩情を語ってから帽子を被り、出かけた。",
                "被り",
                "カブリ",
                3,
            ),
            ("鬼の居ぬ間に昼寝する。", "間", "マ", 1),
            ("話の「間」を取る。", "間", "マ", 1),
            ("台詞の『間』を置く。", "間", "マ", 1),
            ("変数の値を求める。", "値", "アタイ", 3),
            ("木の根本に座る。", "根本", "ネモト", 3),
        ],
        &[
            "問題の根本を考える。",
            "木の分類の根本を考える。",
            "木の根本的な性質を調べる。",
            "草の根本原因を考える。",
            "木の根本構造を学ぶ。",
            "株の値を調べる。",
            "誰も居ぬ間に昼寝する。",
            "鬼の居る間は静かだ。",
            "話の間に客が帰った。",
            "両者の間を取って折衷した。",
            "暮春の候、お元気でしょうか。",
            "候補を探す。",
        ],
    );
    let mut engine = Haqumei::new().unwrap();
    let features = engine.run_frontend("恩情を被り、感謝した。").unwrap();
    let verb = features.iter().find(|f| f.string == "被り").unwrap();
    assert_eq!(verb.orig, "被る");
    assert_eq!(verb.ctype, "五段・ラ行");
    assert_eq!(verb.cform, "連用形");
    assert_eq!(verb.acc, 3);
    let features = engine.run_frontend("木の根本。").unwrap();
    let noun = features.iter().find(|f| f.string == "根本").unwrap();
    assert_eq!(noun.pos_group1, "一般");
    assert_eq!(noun.acc, 3);
    assert_eq!(noun.chain_rule, "C2");
}

#[test]
fn counters_and_ranks_have_consistent_readings_and_word_boundaries() {
    let mut engine = Haqumei::new().unwrap();
    for (text, word, read, group) in [
        ("三丈を測る。", "丈", "ジョウ", "接尾"),
        ("五部印刷する。", "部", "ブ", "接尾"),
        ("十重に囲む。", "十重", "トエ", "一般"),
        ("金壱万円を納める。", "金", "キン", "数接続"),
    ] {
        let features = engine.run_frontend(text).unwrap();
        let feature = features.iter().find(|f| f.string == word).unwrap();
        assert_eq!(feature.read, read, "{text}");
        assert_eq!(feature.pos_group1, group, "{text}");
        assert_eq!(features, engine.run_frontend_detailed(text).unwrap().0);
        assert_eq!(
            engine.g2p_mapping(text).unwrap(),
            engine.g2p_candidates(text).unwrap().candidates[0].words,
            "{text}"
        );
    }
    let mapping = engine.g2p_mapping_detailed("五部印刷する。").unwrap();
    for (word, expected) in [("五", 0..1), ("部", 1..2), ("印刷", 2..4)] {
        assert_eq!(
            mapping.iter().find(|w| w.word == word).unwrap().char_span,
            expected
        );
    }
    for (rank, reading) in [
        ("従一位", "ジュイチイ"),
        ("従二位", "ジュニイ"),
        ("従三位", "ジュサンミ"),
        ("従四位", "ジュシイ"),
        ("従五位", "ジュゴイ"),
        ("従六位", "ジュロクイ"),
        ("従七位", "ジュシチイ"),
        ("従八位", "ジュハチイ"),
    ] {
        let text = format!("{rank}に就く。");
        assert!(engine.g2k(&text).unwrap().starts_with(reading), "{text}");
        let first = &engine.run_frontend(&text).unwrap()[0];
        assert_eq!(first.pos, "接頭詞", "{text}");
        assert_eq!(first.ctype, "*", "{text}");
        assert_eq!(first.orig, "従", "{text}");
    }
    for text in [
        "丈を測る。",
        "五部に住む。",
        "五部印刷株式会社の社員だ。",
        "十重奏を聞く。",
        "十重の塔。",
        "金が壱万円。",
        "金、壱万円。",
        "三位に入賞した。",
        "従う人が三位になった。",
    ] {
        let corrected = engine.g2k(text).unwrap();
        engine.options.modify_context_reading = false;
        assert_eq!(corrected, engine.g2k(text).unwrap(), "{text}");
        engine.options.modify_context_reading = true;
    }
}

#[test]
fn lexical_toe_does_not_merge_a_numeric_suffix() {
    check_context_readings(
        &[
            ("城を十重に囲む。", "十重", "トエ", 2),
            ("雲が十重に重なる。", "十重", "トエ", 2),
        ],
        &["二十重に囲む。", "百十重に重ねる。", "二百十重に重ねる。"],
    );
}

#[test]
fn headwear_objects_select_kaburu_without_crossing_a_clause() {
    let mut engine = Haqumei::new().unwrap();
    for (text, word, read) in [
        ("帽子を被る。", "被る", "カブル"),
        ("王冠をそっと被った。", "被っ", "カブッ"),
        ("頭巾を被りながら歩く。", "被り", "カブリ"),
        ("帽子を被らない。", "被ら", "カブラ"),
        ("帽子を被ればいい。", "被れ", "カブレ"),
        ("帽子を被ろう。", "被ろ", "カブロ"),
    ] {
        let features = engine.run_frontend(text).unwrap();
        let verb = features.iter().find(|f| f.string == word).unwrap();
        assert_eq!(verb.read, read, "{text}");
        assert_eq!(verb.orig, "被る", "{text}");
        assert_eq!(verb.ctype, "五段・ラ行", "{text}");
        assert_eq!(verb.mora_size, 3, "{text}");
        if matches!(word, "被る" | "被っ" | "被れ") {
            assert_eq!(verb.acc, 2, "{text}");
        }
    }
    for text in [
        "損害を被った。",
        "帽子を売って損害を被った。",
        "冠の恩恵を被った。",
        "帽子を、損害は被った。",
    ] {
        let corrected = engine.g2k(text).unwrap();
        engine.options.modify_context_reading = false;
        assert_eq!(corrected, engine.g2k(text).unwrap(), "{text}");
        engine.options.modify_context_reading = true;
    }
}

#[test]
fn protected_context_prevents_counter_and_rank_rewrites() {
    let mut engine = Haqumei::new().unwrap();
    engine.set_morph_filter(|_, _, morphs| {
        for morph in morphs.iter_mut().filter(|m| {
            matches!(
                m.surface.as_str(),
                "三" | "印刷" | "囲む" | "位" | "円" | "被っ"
            )
        }) {
            morph.dictionary_index = 1;
        }
    });
    engine.options.protect_user_dict_readings = true;
    for text in [
        "三丈を測る。",
        "五部印刷する。",
        "十重に囲む。",
        "従五位に就く。",
        "従三位に就く。",
        "金壱万円。",
        "帽子を被った。",
    ] {
        let protected = engine.g2k(text).unwrap();
        engine.options.modify_context_reading = false;
        assert_eq!(protected, engine.g2k(text).unwrap(), "{text}");
        engine.options.modify_context_reading = true;
    }
}

#[test]
fn kata_suffix_readings_survive_a_following_copula() {
    let mut engine = Haqumei::new().unwrap();
    for (text, expected) in [
        ("典型的な歪み方", "テンケーテキナユガミカタ"),
        ("典型的な歪み方です。", "テンケーテキナユガミカタデス。"),
        ("妙な歪み方ですね。", "ミョーナユガミカタデスネ。"),
        ("特徴的な転び方です。", "トクチョーテキナコロビカタデス。"),
        ("激しい捻れ方です。", "ハゲシイネジレカタデス。"),
    ] {
        assert_eq!(engine.g2k(text).unwrap(), expected, "{text}");
        let features = engine.run_frontend(text).unwrap();
        let suffix = features.iter().find(|f| f.string == "方").unwrap();
        assert_eq!(suffix.read, "カタ", "{text}");
        assert_eq!(suffix.mora_size, 2, "{text}");
    }
    for text in [
        "歪みの方が大きい。",
        "典型的には歪む方です。",
        "強く歪んだ方を選ぶ。",
    ] {
        assert_eq!(
            engine
                .run_frontend(text)
                .unwrap()
                .iter()
                .find(|f| f.string == "方")
                .unwrap()
                .read,
            "ホウ",
            "{text}"
        );
    }
    engine.options.modify_context_reading = false;
    assert!(
        engine
            .g2k("典型的な歪み方です。")
            .unwrap()
            .contains("ユガミホー")
    );
}

#[test]
fn station_distance_keeps_the_count_separate_from_the_place_name() {
    let mut engine = Haqumei::new().unwrap();
    for (text, expected) in [
        (
            "十三駅先には十三駅がある。",
            "ジューサンエキサキニワジューソーエキガアル。",
        ),
        (
            "十三駅は、十三駅先にある。",
            "ジューソーエキワ、ジューサンエキサキニアル。",
        ),
        (
            "阪急十三駅で乗り換える。",
            "ハンキュージューソーエキデノリカエル。",
        ),
        ("十三駅の先にある。", "ジューソーエキノサキニアル。"),
        ("二十三駅先にある。", "ニジューサンエキサキニアル。"),
        ("全部で十三駅ある。", "ゼンブデジューサンエキアル。"),
        ("十三駅目で降りる。", "ジューサンエキメデオリル。"),
    ] {
        assert_eq!(engine.g2k(text).unwrap(), expected, "{text}");
    }
    for (kanji, digits) in [
        ("十三駅先にある。", "13駅先にある。"),
        ("十三駅目で降りる。", "13駅目で降りる。"),
    ] {
        assert_eq!(
            engine.g2p_prosody(kanji).unwrap(),
            engine.g2p_prosody(digits).unwrap(),
            "{kanji}"
        );
    }
    engine.options.modify_context_reading = false;
    assert!(
        engine
            .g2k("十三駅先にある。")
            .unwrap()
            .starts_with("ジューソー")
    );
}

#[test]
fn a_station_pair_keeps_juso_instead_of_counting_thirteen_stations() {
    let mut engine = Haqumei::new().unwrap();
    for text in [
        "大阪梅田駅・十三駅間の線路。",
        "大阪梅田・十三駅間を走る。",
        "大阪梅田駅―十三駅間の線路。",
        "阪急十三駅間の線路。",
    ] {
        assert!(
            engine.g2k(text).unwrap().contains("ジューソーエキカン"),
            "{text}"
        );
    }
    for text in [
        "全十三駅間の運賃。",
        "計十三駅間を走る。",
        "十三駅間を走った。",
    ] {
        assert!(
            engine.g2k(text).unwrap().contains("ジューサンエキカン"),
            "{text}"
        );
    }
}

#[test]
fn registered_kata_and_station_readings_are_preserved() {
    let mut engine = Haqumei::new().unwrap();
    engine.set_morph_filter(|_, _, morphs| {
        for morph in morphs
            .iter_mut()
            .filter(|m| matches!(m.surface.as_str(), "方" | "十三" | "十" | "三"))
        {
            morph.dictionary_index = 1;
        }
    });
    engine.options.protect_user_dict_readings = true;
    for text in [
        "典型的な歪み方です。",
        "十三駅先にある。",
        "阪急十三駅で乗り換える。",
    ] {
        let protected = engine.g2k(text).unwrap();
        engine.options.modify_context_reading = false;
        assert_eq!(engine.g2k(text).unwrap(), protected, "{text}");
        engine.options.modify_context_reading = true;
    }
}

fn check_context_readings(cases: &[(&str, &str, &str, i32)], preserved: &[&str]) {
    let mut engine = Haqumei::new().unwrap();
    for &(text, word, reading, moras) in cases {
        let features = engine.run_frontend(text).unwrap();
        let feature = features.iter().find(|f| f.string == word).unwrap();
        assert_eq!(feature.read, reading, "{text}");
        assert_eq!(feature.mora_size, moras, "{text}");
    }
    for &text in preserved {
        let with_rules = engine.g2k(text).unwrap();
        engine.options.modify_context_reading = false;
        let without_rules = engine.g2k(text).unwrap();
        engine.options.modify_context_reading = true;
        assert_eq!(with_rules, without_rules, "{text}");
    }
}

#[test]
fn lexical_context_rewrites_keep_the_lemma_and_conjugation_consistent() {
    let mut engine = Haqumei::new().unwrap();
    let features = engine.run_frontend("縁を断った。").unwrap();
    let verb = features.iter().find(|f| f.string == "断っ").unwrap();
    assert_eq!(verb.read, "タッ");
    assert_eq!(verb.orig, "断つ");
    assert_eq!(verb.ctype, "五段・タ行");
    assert_eq!(verb.cform, "連用タ接続");
    assert_eq!(verb.mora_size, 2);

    let features = engine.run_frontend("仏の教えを学ぶ。").unwrap();
    let noun = features.iter().find(|f| f.string == "仏").unwrap();
    assert_eq!(noun.read, "ホトケ");
    assert_eq!(noun.pos, "名詞");
    assert_eq!(noun.pos_group1, "一般");
    assert_eq!(noun.mora_size, 3);

    for text in ["誘いを断った。", "仏の大統領が訪れた。"] {
        let with_rules = engine.g2k(text).unwrap();
        engine.options.modify_context_reading = false;
        assert_eq!(engine.g2k(text).unwrap(), with_rules, "{text}");
        engine.options.modify_context_reading = true;
    }
}

#[test]
fn conventional_phrases_select_readings_before_chaining() {
    check_context_readings(
        &[
            ("神の代から続く", "代", "ヨ", 1),
            ("崖の際に立つ", "際", "キワ", 2),
            ("淵の際まで進む", "際", "キワ", 2),
            ("事の本を正す", "本", "モト", 2),
            ("汚れを祓う", "汚れ", "ケガレ", 3),
            ("盲・聾・養護学校", "盲", "モウ", 2),
            ("盲・聾・養護学校", "聾", "ロウ", 2),
        ],
        &[
            "神の代理を務める",
            "その際に読む",
            "洪水の際に逃げる",
            "黒白の写真を撮る",
            "黒白を争う",
            "本を読む",
            "紅色細菌を調べる",
            "紅色の着物を着る",
            "町の聾学校",
            "水の面では問題がある",
            "水の面に月が映る",
            "靴の汚れを清める",
        ],
    );
}

#[test]
fn religious_terms_need_direct_context() {
    check_context_readings(
        &[
            ("天台声明を唱える", "声明", "ショウミョウ", 4),
            ("真言声明が唱えられる", "声明", "ショウミョウ", 4),
            ("この社は神を祀る", "社", "ヤシロ", 3),
            ("在家信者が集まる", "在家", "ザイケ", 3),
        ],
        &[
            "法要の声明を読んだ",
            "声明が唱える理念",
            "声明を唱える",
            "この社は社員を募集する",
            "在家さんが来る",
        ],
    );
}

#[test]
fn cloth_readings_keep_the_place_name() {
    check_context_readings(
        &[
            ("生成りの麻布を買う", "麻布", "アサヌノ", 4),
            ("麻布を織る", "麻布", "アサヌノ", 4),
        ],
        &["麻布に染色の店がある", "麻布を染める"],
    );
}

#[test]
fn idiomatic_face_and_sound_need_local_causes() {
    check_context_readings(
        &[
            ("御前で面を上げよ", "面", "オモテ", 3),
            ("厳しい訓練に音を上げる", "音", "ネ", 1),
        ],
        &[
            "ラケットの面を上げる",
            "号令に従い、面を上げよ",
            "音楽の音を上げる",
        ],
    );
}

#[test]
fn capsizing_reading_keeps_transitive_covering() {
    check_context_readings(
        &[("強風でボートが覆った", "覆っ", "クツガエッ", 5)],
        &[
            "海面をボートが覆った",
            "ボートが覆った",
            "ボートが覆っていた海面",
        ],
    );
}

#[test]
fn water_master_reading_keeps_adverbial_shutoshite() {
    check_context_readings(
        &[("湖の主として恐れられる", "主として", "ヌシトシテ", 5)],
        &["主として魚を飼う"],
    );
}

#[test]
fn explicit_telephone_context_keeps_quantities_and_decimals() {
    let mut engine = Haqumei::new().unwrap();
    for (text, kana) in [
        ("119に電話", "イチイチキューニデンワ"),
        ("番号案内は104です", "バンゴーアンナイワイチゼロヨンデス"),
        ("時報は117です", "ジホーワイチイチナナデス"),
        ("171で伝言", "イチナナイチデデンゴン"),
        (
            "電話は03の1234の5678",
            "デンワワゼロサンノイチニーサンヨンノゴーロクナナハチ",
        ),
        ("電話は100ある", "デンワワヒャクアル"),
        ("FAXは100届いた", "ファックスワヒャクトドイタ"),
        ("得点は117です", "トクテンワヒャクジューナナデス"),
        ("電話番号は1.5です", "デンワバンゴーワイッテンゴデス"),
    ] {
        assert_eq!(engine.g2k(text).unwrap(), kana, "{text}");
    }
}

#[test]
fn punctuation_keeps_written_digit_groups_separate() {
    let mut engine = Haqumei::new().unwrap();
    for (text, kana) in [
        ("一〇一,", "イチゼロイチ，"),
        ("四〇五,電話", "ヨンゼロゴー，デンワ"),
        ("三〇九,三〇八", "サンゼロキュー，サンゼロハチ"),
        ("三〇九,三八一", "サンゼロキュー，サンハチイチ"),
        ("一,〇〇〇", "セン"),
        (
            "1,234,567",
            "ヒャクニジューサンマンヨンセンゴヒャクロクジューナナ",
        ),
        ("1,050.5円", "センゴジュッテンゴエン"),
        ("二〇〇〇年", "ニセンネン"),
    ] {
        assert_eq!(engine.g2k(text).unwrap(), kana, "{text}");
    }
}

#[test]
fn explicit_first_day_keeps_duration_readings() {
    let mut engine = Haqumei::new().unwrap();
    for (text, kana) in [
        ("本日1日付で", "ホンジツツイタチズケデ"),
        ("明日1日午前", "アシタツイタチゴゼン"),
        ("1日午後", "ツイタチゴゴ"),
        ("丸1日午後まで休む", "マルイチニチゴゴマデヤスム"),
        ("今日も1日、", "キョーモイチニチ、"),
        ("1日に3回", "イチニチニサンカイ"),
    ] {
        assert_eq!(engine.g2k(text).unwrap(), kana, "{text}");
    }
}

#[test]
fn school_year_lists_keep_decimal_quantities() {
    let mut engine = Haqumei::new().unwrap();
    for (text, kana) in [
        ("1・2年生", "イチ・ニネンセー"),
        ("二十・五％", "ニジュッテンゴパーセント"),
        ("1.5年", "イッテンゴネン"),
    ] {
        assert_eq!(engine.g2k(text).unwrap(), kana, "{text}");
    }
}

#[test]
fn musashi_needs_an_explicit_name_context() {
    let mut engine = Haqumei::new().unwrap();
    for text in [
        "名前は六三四です",
        "六三四さん",
        "六三四くん",
        "六三四君",
        "六三四という名前",
    ] {
        let features = engine.run_frontend(text).unwrap();
        let name = features
            .iter()
            .find(|f| f.string == "六三四")
            .unwrap_or_else(|| panic!("{text}: {features:?}"));
        assert_eq!(name.read, "ムサシ", "{text}");
        assert_eq!(name.mora_size, 3, "{text}");
    }
    for text in ["六三四", "六三四円", "一六三四", "六三四を数える"] {
        let with_rules = engine.g2k(text).unwrap();
        engine.options.modify_context_reading = false;
        let without_rules = engine.g2k(text).unwrap();
        engine.options.modify_context_reading = true;
        assert_eq!(with_rules, without_rules, "{text}");
    }
}

#[test]
fn melodic_fushi_uses_its_dictionary_accent() {
    let mut engine = Haqumei::new().unwrap();
    for text in ["歌の節を覚えた", "唄の節も凝っている"] {
        let features = engine.run_frontend(text).unwrap();
        let fushi = features.iter().find(|f| f.string == "節").unwrap();
        assert_eq!(
            (fushi.read.as_str(), fushi.acc, fushi.mora_size),
            ("フシ", 2, 2),
            "{text}"
        );
    }
    let before = engine.g2p_prosody("文書の節を参照した").unwrap();
    engine.options.modify_context_reading = false;
    assert_eq!(engine.g2p_prosody("文書の節を参照した").unwrap(), before);
}

#[test]
fn yomigana_is_flat_without_merging_its_words() {
    let mut engine = Haqumei::new().unwrap();
    let features = engine.run_frontend("読み仮名を記す").unwrap();
    let head = features.iter().find(|f| f.string == "読み").unwrap();
    assert_eq!(head.acc, 0);
    let mapping = engine.g2p_mapping("読み仮名を記す").unwrap();
    assert_eq!(mapping[0].char_span, 0..2);
    assert_eq!(mapping[1].char_span, 2..4);
    assert_eq!(engine.g2k("読み仮名を記す").unwrap(), "ヨミガナヲシルス");
    let before = engine.g2p_prosody("本を読み、仮名を書く").unwrap();
    engine.options.modify_context_reading = false;
    assert_eq!(engine.g2p_prosody("本を読み、仮名を書く").unwrap(), before);
}

#[test]
fn ichiban_suki_keeps_compound_zuki_elsewhere() {
    let mut h = Haqumei::new().unwrap();
    for (text, expected) in [
        ("この味が一番好きだ", "コノアジガイチバンスキダ"),
        ("一番好きな道を歩く", "イチバンスキナミチヲアルク"),
        ("旅好きな人だ", "タビズキナヒトダ"),
        ("音楽好きの友人だ", "オンガクズキノユージンダ"),
        ("物好きな客だ", "モノズキナキャクダ"),
        ("一番手を任せる", "イチバンテヲマカセル"),
    ] {
        assert_eq!(h.g2k(text).unwrap(), expected, "{text}");
    }
    h.options.modify_context_reading = false;
    assert_eq!(h.g2k("一番好きだ").unwrap(), "イチバンズキダ");
}

#[test]
fn dated_suffix_keeps_zuke_instead_of_attached_tsuki() {
    let mut h = Haqumei::new().unwrap();
    for (text, expected) in [
        (
            "六月十二日付の通知を送る。",
            "ロクガツジューニニチズケノツーチヲオクル。",
        ),
        (
            "九月十日付で就任する。",
            "クガツトーカズケデシューニンスル。",
        ),
        (
            "四月一日付の辞令を受け取る。",
            "シガツツイタチズケノジレーヲウケトル。",
        ),
        (
            "八月十四日付の公告を読む。",
            "ハチガツジューヨッカズケノコーコクヲヨム。",
        ),
        ("二十日付で異動する。", "ハツカズケデイドースル。"),
        (
            "三十日付で就任する。",
            "サンジューニチズケデシューニンスル。",
        ),
        ("条件付の契約を結ぶ。", "ジョーケンツキノケーヤクヲムスブ。"),
        ("朝食付の宿を探す。", "チョーショクツキノヤドヲサガス。"),
        ("保証付の商品を選ぶ。", "ホショーツキノショーヒンヲエラブ。"),
    ] {
        assert_eq!(h.g2k(text).unwrap(), expected, "{text}");
    }
}

#[test]
fn restored_pronunciation_updates_the_public_mora_count() {
    for (options, expected) in [
        (
            HaqumeiOptions {
                use_read_as_pron: true,
                ..Default::default()
            },
            4,
        ),
        (
            HaqumeiOptions {
                revert_long_vowels: true,
                ..Default::default()
            },
            3,
        ),
    ] {
        let mut engine = Haqumei::with_options(options).unwrap();
        let words = engine.g2p_mapping_detailed("おめぇー").unwrap();
        assert_eq!(words.iter().map(|w| w.mora_count).sum::<i32>(), expected);
        let features = engine.run_frontend("おめぇー").unwrap();
        assert_eq!(features.iter().map(|f| f.mora_size).sum::<i32>(), expected);
    }
}

#[test]
fn karada_uses_its_dictionary_accent_before_chaining() {
    let mut engine = Haqumei::new().unwrap();
    for text in [
        "身体を鍛える",
        "身体の調子",
        "身体が重い",
        "お身体の調子",
        "身体 を 鍛える",
    ] {
        let expected = text.replace("身体", "体");
        assert_eq!(
            engine.g2p_prosody(text).unwrap(),
            engine.g2p_prosody(&expected).unwrap(),
            "{text}"
        );
    }
    for text in ["身体を拘束する", "身体の機能", "身体が不自由だ"] {
        let features = engine.run_frontend(text).unwrap();
        assert_eq!(
            features.iter().find(|f| f.string == "身体").unwrap().read,
            "シンタイ"
        );
    }
}

#[test]
fn numeric_boundaries_and_limited_identifiers() {
    let mut engine = Haqumei::new().unwrap();
    for (text, kana) in [
        ("119に電話する", "イチイチキューニデンワスル"),
        ("110に電話した", "イチイチゼロニデンワシタ"),
        ("119にかける", "イチイチキューニカケル"),
        ("119に掛けた", "イチイチキューニカケタ"),
        ("100に電話料金を足す", "ヒャクニデンワリョーキンヲタス"),
        ("電話は100ある", "デンワワヒャクアル"),
        (
            "119に電話連絡する",
            "ヒャクジューキューニデンワレンラクスル",
        ),
        ("〒1048011", "〒イチゼロヨンハチゼロイチイチ"),
        ("〒104・8011", "〒イチゼロヨン・ハチゼロイチイチ"),
        ("〒〇七〇-〇〇〇〇", "〒ゼロナナゼロ−ゼロゼロゼロゼロ"),
        (
            "070-〇〇〇〇-〇〇〇〇",
            "ゼロナナゼロ−ゼロゼロゼロゼロ−ゼロゼロゼロゼロ",
        ),
        ("1 0", "イチゼロ"),
        ("1　　0", "イチゼロ"),
        ("1 234円", "イチニヒャクサンジューヨエン"),
        (
            "〇七〇-〇〇二四-五六七九",
            "ゼロナナゼロ−ゼロゼロニーヨン−ゴーロクナナキュー",
        ),
        ("〇〇町", "マルマルマチ"),
        ("〇〇人", "マルマルニン"),
        ("〇〇年", "マルマルネン"),
        ("〇〇円", "マルマルエン"),
        ("〇〇", "マルマル"),
        ("〇円", "レーエン"),
        ("二〇〇〇年", "ニセンネン"),
    ] {
        assert_eq!(engine.g2k(text).unwrap(), kana, "{text}");
        assert_eq!(
            engine.run_frontend(text).unwrap(),
            engine.run_frontend_detailed(text).unwrap().0,
            "{text}"
        );
        assert_eq!(
            engine.g2p_mapping(text).unwrap(),
            engine.g2p_candidates(text).unwrap().candidates[0].words,
            "{text}"
        );
    }
    let separated = engine.run_frontend("EF65 1032号機").unwrap();
    assert!(!separated.iter().any(|node| node.string == "万"));
    assert!(
        !engine
            .g2p("65 1032")
            .unwrap()
            .iter()
            .any(|p| p.as_str() == "pau")
    );
    let mut raw = OpenJTalk::new().unwrap();
    assert_eq!(
        raw.run_frontend("65 1032").unwrap(),
        raw.run_frontend_detailed("65 1032").unwrap().0
    );
    let mecab = raw.run_mecab("65 1032").unwrap();
    let njd = raw.run_njd_from_mecab(&mecab).unwrap();
    assert_eq!(raw.run_frontend("65 1032").unwrap(), njd);
    assert_eq!(
        raw.g2p("65 1032").unwrap(),
        raw.extract_phonemes(&njd).unwrap()
    );
    assert_eq!(
        raw.g2p("65 1032").unwrap(),
        raw.g2p_mapping("65 1032")
            .unwrap()
            .into_iter()
            .filter(|word| !word.is_ignored)
            .flat_map(|word| word.phonemes)
            .collect::<Vec<_>>()
    );
    let expanded = raw
        .run_njd_from_mecab(["〇８０−００００,名詞,数,*,*,*,*,*,*,*,*,*,*"])
        .unwrap();
    assert!(expanded.iter().all(|f| f.read != "*"));
    assert_eq!(expanded.iter().filter(|f| f.read == "ゼロ").count(), 6);
    assert!(!expanded.iter().any(|f| f.pron.contains("キュー")));

    for text in ["〒10480112", "1048011", "1119に電話する"] {
        assert!(
            raw.run_frontend(text)
                .unwrap()
                .iter()
                .any(|f| { matches!(f.string.as_str(), "十" | "百" | "千" | "万") }),
            "{text}"
        );
    }
    assert!(
        raw.run_frontend("3.119に電話する")
            .unwrap()
            .iter()
            .any(|f| f.read == "テン")
    );

    for text in [
        "〒〇七〇-〇〇〇〇",
        "070-〇〇〇〇-〇〇〇〇",
        "〇七〇-〇〇二四-五六七九",
    ] {
        let words = engine.g2p_mapping(text).unwrap();
        assert_eq!(words.len(), text.chars().count(), "{text}");
        for (index, word) in words.iter().enumerate() {
            assert_eq!(word.char_span, index..index + 1, "{text}: {}", word.word);
        }
    }
}

#[test]
fn number_and_nannimo_corrections_respect_registered_readings() {
    let mut engine = Haqumei::new().unwrap();
    engine.set_morph_filter(|_, _, morphs| {
        for morph in morphs
            .iter_mut()
            .filter(|m| matches!(m.surface.as_str(), "何" | "〇"))
        {
            morph.dictionary_index = 1;
        }
    });
    engine.options.protect_user_dict_readings = true;
    assert!(
        engine
            .g2k("何にも知らない")
            .unwrap()
            .starts_with("ナニニモ")
    );
    assert!(
        engine
            .run_frontend("〇七〇")
            .unwrap()
            .iter()
            .filter(|f| f.string == "〇")
            .all(|f| f.read != "ゼロ")
    );
}

#[test]
fn negative_nannimo_keeps_case_particle_uses() {
    let mut engine = Haqumei::new().unwrap();
    for (text, reading) in [
        ("何にも知らない", "ナン"),
        ("何にも知りません", "ナン"),
        ("何にも知らず", "ナン"),
        ("何にも知らん", "ナン"),
        ("何にも知らぬ", "ナン"),
        ("何にも分からなかった", "ナン"),
        ("何にも知らなければ", "ナン"),
        ("何にも答えてもらっていない", "ナン"),
        ("何にも返事がなく", "ナン"),
        ("何にもありません", "ナン"),
        ("何にもすることができず", "ナン"),
        ("何にも面白くない", "ナン"),
        ("何にもしないで宣伝ばかり", "ナン"),
        ("何にも似ていない", "ナニ"),
        ("何にも全然似ていない", "ナニ"),
        ("何にも代えがたい", "ナニ"),
        ("何にも依存していない", "ナニ"),
        ("何にも属さない", "ナニ"),
        ("何にも頼らない", "ナニ"),
        ("何にも答えたが教えない", "ナニ"),
        ("何にも。知らない", "ナニ"),
        ("「何にも」という語を知らない", "ナニ"),
        ("何にも似ることができない", "ナニ"),
        ("何にもとづいて決めた", "ナニ"),
    ] {
        let features = engine.run_frontend(text).unwrap();
        assert_eq!(
            features.iter().find(|f| f.string == "何").unwrap().read,
            reading,
            "{text}"
        );
    }
    engine.options.modify_context_reading = false;
    assert!(
        engine
            .g2k("何にも知らない")
            .unwrap()
            .starts_with("ナニニモ")
    );
}

#[test]
fn unvoicing_uses_corrected_readings_in_each_api() {
    let texts = [
        "博士課程",
        "東高洲橋",
        "そうですね",
        "結婚式々場",
        "黒﨑と博士課程",
    ];
    let mut engine = Haqumei::new().unwrap();
    for text in texts {
        let features = engine.run_frontend(text).unwrap();
        assert_eq!(
            features,
            engine.run_frontend_detailed(text).unwrap().0,
            "{text}"
        );
        assert_eq!(
            engine.g2p_mapping(text).unwrap(),
            engine.g2p_candidates(text).unwrap().candidates[0].words,
            "{text}",
        );
    }
    let expected: Vec<_> = texts
        .iter()
        .map(|t| engine.run_frontend(t).unwrap())
        .collect();
    assert_eq!(engine.run_frontend_batch(&texts).unwrap(), expected);
    assert_eq!(expected[0][0].pron, "ハク’シ");
    assert_eq!(expected[2][0].pron, "ソーデス’ネ");

    engine.options.use_read_as_pron = true;
    assert!(
        engine
            .run_frontend("博士課程")
            .unwrap()
            .iter()
            .all(|f| !f.pron.contains('’'))
    );

    let mut raw = OpenJTalk::new().unwrap();
    assert!(raw.run_frontend("です！").unwrap()[0].pron.contains('’'));
}

#[test]
fn decimal_digits_do_not_take_integer_counter_readings() {
    let mut engine = Haqumei::new().unwrap();
    for (text, expected) in [
        ("1.1本", "イッテンイチホン"),
        ("1.3本", "イッテンサンホン"),
        ("1.6本", "イッテンロクホン"),
        ("1.8個", "イッテンハチコ"),
        ("1.1人", "イッテンイチニン"),
        ("1.4日", "イッテンヨンニチ"),
        ("2.11本", "ニーテンイチイチホン"),
        ("2.1万本", "ニーテンイチマンボン"),
        ("1本", "イッポン"),
        ("3本", "サンボン"),
        ("6本", "ロッポン"),
        ("8個", "ハッコ"),
        ("4日", "ヨッカ"),
        ("米・一貫目", "コメ・イッカンメ"),
        ("5分待ち・10分待ち", "ゴフンマチ・ジュップンマチ"),
    ] {
        assert_eq!(engine.g2k(text).unwrap(), expected, "{text}");
    }
    let enumeration = engine.run_frontend("四・六級").unwrap();
    let six = enumeration.iter().find(|f| f.string == "六").unwrap();
    assert_eq!(six.pron, "ロッ");
}

#[test]
fn room_counter_is_not_changed_back_to_beya() {
    let mut engine = Haqumei::new().unwrap();
    for (text, expected) in [
        ("一部屋", "ヒトヘヤ"),
        ("二部屋", "フタヘヤ"),
        ("三部屋", "サンヘヤ"),
        ("子供部屋", "コドモベヤ"),
        ("相撲部屋", "スモーベヤ"),
    ] {
        assert_eq!(engine.g2k(text).unwrap(), expected, "{text}");
    }
    let mut without_context = Haqumei::with_options(HaqumeiOptions {
        modify_context_reading: false,
        ..Default::default()
    })
    .unwrap();
    assert_eq!(
        engine.g2k("三部屋").unwrap(),
        without_context.g2k("三部屋").unwrap()
    );
}

#[test]
fn group_quantities_and_class_numbers() {
    let mut engine = Haqumei::new().unwrap();
    for (text, kana) in [
        ("一組", "ヒトクミ"),
        ("1組", "ヒトクミ"),
        ("二組", "フタクミ"),
        ("2組", "フタクミ"),
        ("一年一組", "イチネンイチクミ"),
        ("1年1組", "イチネンイチクミ"),
        ("三年二組", "サンネンニクミ"),
        ("３年２組", "サンネンニクミ"),
        ("第一組", "ダイイチクミ"),
        ("第1組", "ダイイチクミ"),
        ("第2組", "ダイニクミ"),
        ("二年に一組", "ニネンニヒトクミ"),
        ("11組", "ジューイチクミ"),
        ("12組", "ジューニクミ"),
        ("1.1組", "イッテンイチクミ"),
        ("1.2組", "イッテンニクミ"),
    ] {
        assert_eq!(engine.g2k(text).unwrap(), kana, "{text}");
        assert_eq!(
            engine.run_frontend(text).unwrap(),
            engine.run_frontend_detailed(text).unwrap().0
        );
    }
    for text in ["一組", "1組", "二組", "2組"] {
        let features = engine.run_frontend(text).unwrap();
        assert_eq!(features[0].acc, 2, "{text}");
        assert_eq!(
            features.iter().map(|f| f.mora_size).sum::<i32>(),
            4,
            "{text}"
        );
        assert_eq!(
            engine.g2p_mapping(text).unwrap(),
            engine.g2p_candidates(text).unwrap().candidates[0].words
        );
    }
    let texts = ["1組", "二組", "三年一組", "第2組"];
    let sequential: Vec<_> = texts
        .iter()
        .map(|s| engine.run_frontend(s).unwrap())
        .collect();
    assert_eq!(engine.run_frontend_batch(&texts).unwrap(), sequential);
    engine.options.modify_numeral_reading = false;
    assert_eq!(engine.g2k("一組").unwrap(), "イチクミ");
    assert_eq!(engine.g2k("2組").unwrap(), "ニクミ");
}

#[test]
fn registered_group_reading_is_preserved() {
    let mut engine = Haqumei::new().unwrap();
    engine.set_morph_filter(|_, _, morphs| {
        for morph in morphs.iter_mut().filter(|m| m.surface == "一組") {
            morph.dictionary_index = 1;
        }
    });
    engine.options.protect_user_dict_readings = true;
    assert_eq!(engine.g2k("一組").unwrap(), "イチクミ");
    engine.options.modify_context_reading = false;
    assert_eq!(engine.g2k("一組").unwrap(), "イチクミ");
    engine.options.protect_user_dict_readings = false;
    assert_eq!(engine.g2k("一組").unwrap(), "ヒトクミ");
}

#[test]
fn separated_counters_match_contiguous_readings_and_accents() {
    let mut engine = Haqumei::new().unwrap();
    for (spaced, joined) in [
        ("2024 年", "2024年"),
        ("3 人", "3人"),
        ("2 人", "2人"),
        ("3 本", "3本"),
        ("10 時", "10時"),
        ("3 分", "3分"),
        ("6 匹", "6匹"),
        ("1.5 日", "1.5日"),
        ("0.2 人", "0.2人"),
        ("1 年 1 組", "1年1組"),
    ] {
        assert_eq!(
            engine.g2k(spaced).unwrap(),
            engine.g2k(joined).unwrap(),
            "{spaced}"
        );
        assert_eq!(
            engine.extract_fullcontext(spaced).unwrap(),
            engine.extract_fullcontext(joined).unwrap(),
            "{spaced}"
        );
    }
    for (text, word, pron) in [
        ("3 分の1", "分", "ブン"),
        ("場面6 人前で話す", "人", "ヒト"),
        ("２ 人づくりの基盤", "人", "ヒト"),
        ("1 月ほど待った", "月", "ツキ"),
        ("一 月が過ぎた", "月", "ツキ"),
    ] {
        let features = engine.run_frontend(text).unwrap();
        assert_eq!(
            features
                .iter()
                .find(|f| f.string == word)
                .unwrap()
                .pron
                .replace('’', ""),
            pron,
            "{text}"
        );
    }
}

#[test]
fn separated_counter_preserves_protected_noun_reading() {
    let mut engine = Haqumei::new().unwrap();
    engine.set_morph_filter(|_, _, morphs| {
        for morph in morphs.iter_mut().filter(|m| m.surface == "人") {
            morph.dictionary_index = 1;
        }
    });
    engine.options.protect_user_dict_readings = true;
    for numeral in [true, false] {
        engine.options.modify_numeral_reading = numeral;
        assert_eq!(engine.g2k("2 人").unwrap(), "ニヒト");
    }
    engine.options.protect_user_dict_readings = false;
    assert_eq!(engine.g2k("2 人").unwrap(), "フタリ");
}

#[test]
fn comma_grouped_numbers_match_plain_numbers() {
    let mut engine = Haqumei::new().unwrap();
    let texts = [
        "1,050円",
        "12,005人",
        "1,234,567",
        "1,050.5円",
        "1,050円と12,005円",
    ];
    for text in texts {
        let plain = text.replace(',', "");
        assert_eq!(
            engine.g2k(text).unwrap(),
            engine.g2k(&plain).unwrap(),
            "{text}"
        );
        assert_eq!(
            engine.extract_fullcontext(text).unwrap(),
            engine.extract_fullcontext(&plain).unwrap(),
            "{text}"
        );
        assert_eq!(
            engine.g2p_mapping(text).unwrap(),
            engine.g2p_candidates(text).unwrap().candidates[0].words,
            "{text}"
        );
        assert_eq!(
            engine.run_frontend(text).unwrap(),
            engine.run_frontend_detailed(text).unwrap().0,
            "{text}"
        );
    }
    let expected: Vec<_> = texts.iter().map(|t| engine.g2k(t).unwrap()).collect();
    assert_eq!(engine.g2k_batch(&texts).unwrap(), expected);
    for text in [
        "1,2",
        "01,234円",
        "1,2345円",
        "1,,234",
        "1,234,56",
        "1,000，円",
    ] {
        assert!(engine.g2k(text).unwrap().contains('，'), "{text}");
    }
    let huge = format!("1{}円と2,345円", ",000".repeat(25));
    assert!(
        engine
            .g2k(&huge)
            .unwrap()
            .ends_with("ニセンサンビャクヨンジューゴエン")
    );
}
