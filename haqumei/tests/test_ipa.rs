use haqumei::{
    Haqumei, HaqumeiOptions, IpaBoundary, IpaPhone, IpaToken, IpaTokenProsody, PitchAccent,
    ProsodicIpa, SpecialPhone, WordIpaMap, WordIpaProsody,
};

fn phone(phone: IpaPhone) -> IpaToken {
    IpaToken::Phone(phone)
}

fn flatten(mapping: &[WordIpaMap]) -> Vec<IpaToken> {
    mapping
        .iter()
        .flat_map(|word| word.tokens.iter().cloned())
        .collect()
}

fn flatten_prosody(mapping: &[WordIpaProsody]) -> Vec<ProsodicIpa> {
    mapping
        .iter()
        .flat_map(|word| word.tokens.iter().cloned())
        .collect()
}

#[test]
fn test_g2ipa_basic() {
    let mut haqumei = Haqumei::new().unwrap();
    let mapping = haqumei.g2ipa("こんにちは").unwrap();

    assert_eq!(
        flatten(&mapping),
        [
            phone(IpaPhone::K),
            phone(IpaPhone::O),
            phone(IpaPhone::N),
            phone(IpaPhone::N),
            phone(IpaPhone::I),
            phone(IpaPhone::Ch),
            phone(IpaPhone::I),
            phone(IpaPhone::W),
            phone(IpaPhone::A),
        ]
    );
}

#[test]
fn test_g2ipa_sequence_rules() {
    let mut haqumei = Haqumei::new().unwrap();

    assert_eq!(
        flatten(&haqumei.g2ipa("学校").unwrap()),
        [
            phone(IpaPhone::G),
            phone(IpaPhone::A),
            phone(IpaPhone::LongK),
            phone(IpaPhone::LongO),
        ]
    );
    assert_eq!(
        flatten(&haqumei.g2ipa("バッグ").unwrap()),
        [
            phone(IpaPhone::B),
            phone(IpaPhone::A),
            phone(IpaPhone::LongG),
            phone(IpaPhone::U),
        ]
    );
    assert_eq!(
        flatten(&haqumei.g2ipa("恋愛").unwrap()),
        [
            phone(IpaPhone::R),
            phone(IpaPhone::E),
            phone(IpaPhone::NasalizedE),
            phone(IpaPhone::A),
            phone(IpaPhone::I),
        ]
    );
}

#[test]
fn test_g2ipa_preserves_affricate_closures() {
    let mut haqumei = Haqumei::new().unwrap();
    for (text, expected) in [
        ("グッズ", "dːz"),
        ("あっち", "tːɕ"),
        ("エッジ", "dːʑ"),
        ("ガッツ", "tːs"),
        ("安全", "dz"),
    ] {
        let tokens = flatten(&haqumei.g2ipa(text).unwrap());
        assert!(
            tokens
                .iter()
                .any(|token| matches!(token, IpaToken::Phone(p) if p.as_str() == expected)),
            "{text}: {tokens:?}"
        );
    }
}

#[test]
fn test_g2ipa_classifies_variable_nasals() {
    let mut haqumei = Haqumei::new().unwrap();
    for (text, expected) in [
        ("検査", SpecialPhone::NBeforeS),
        ("感謝", SpecialPhone::NBeforeSh),
        ("関与", SpecialPhone::NBeforeY),
        ("新票", SpecialPhone::NBeforeHy),
        ("カンヒ", SpecialPhone::NBeforeHy),
    ] {
        let tokens = flatten(&haqumei.g2ipa(text).unwrap());
        assert!(
            tokens.contains(&IpaToken::Special(expected)),
            "{text}: {tokens:?}"
        );
        let prosody_mapping = haqumei.g2ipa_prosody(text).unwrap();
        let prosodic_tokens: Vec<_> = prosody_mapping
            .iter()
            .flat_map(|word| word.tokens.iter())
            .filter_map(|item| match item {
                ProsodicIpa::Token { token, .. } => Some(*token),
                _ => None,
            })
            .collect();
        assert_eq!(tokens, prosodic_tokens, "{text}");
        let original = haqumei.g2p_mapping_prosody(text).unwrap();
        let (word_index, pitch) = original
            .iter()
            .enumerate()
            .find_map(|(word_index, word)| {
                word.phonemes.iter().find_map(|item| match item {
                    haqumei::ProsodicPhoneme::Phoneme {
                        phoneme: haqumei::Phoneme::Nn,
                        pitch,
                    } => Some((word_index, pitch)),
                    _ => None,
                })
            })
            .unwrap();
        assert!(
            prosody_mapping[word_index]
                .tokens
                .iter()
                .any(|item| matches!(
                    item, ProsodicIpa::Token { token, prosody }
                        if *token == IpaToken::Special(expected)
                            && *prosody == [IpaTokenProsody::Pitch(*pitch)]
                )),
            "{text}"
        );
    }
    assert!(flatten(&haqumei.g2ipa("カンヒ").unwrap()).contains(&phone(IpaPhone::Hy)));
}

#[test]
fn test_g2ipa_returns_word_mapping() {
    let mut haqumei = Haqumei::new().unwrap();
    let mapping = haqumei.g2ipa("学校").unwrap();

    assert_eq!(mapping.len(), 1);
    assert_eq!(mapping[0].word, "学校");
    assert_eq!(
        mapping[0].tokens,
        [
            phone(IpaPhone::G),
            phone(IpaPhone::A),
            phone(IpaPhone::LongK),
            phone(IpaPhone::LongO),
        ]
    );
    assert!(!mapping[0].is_unknown);
    assert!(!mapping[0].is_ignored);
    assert_eq!(mapping[0].char_span, 0..2);
}

#[test]
fn test_g2ipa_preserves_word_and_space_mapping() {
    let mut haqumei = Haqumei::new().unwrap();
    let phoneme_mapping = haqumei.g2p_mapping("I have").unwrap();
    let ipa_mapping = haqumei.g2ipa("I have").unwrap();

    assert_eq!(ipa_mapping.len(), phoneme_mapping.len());
    for (ipa, phonemes) in ipa_mapping.iter().zip(phoneme_mapping) {
        assert_eq!(ipa.word, phonemes.word);
        assert_eq!(ipa.is_unknown, phonemes.is_unknown);
        assert_eq!(ipa.is_ignored, phonemes.is_ignored);
        assert_eq!(ipa.char_span, phonemes.char_span);
    }

    let space = ipa_mapping
        .iter()
        .find(|word| word.is_ignored)
        .expect("空白の mapping が残る");
    assert!(space.tokens.is_empty());
}

#[test]
fn test_g2ipa_prosody_keeps_boundaries_and_pitch() {
    let mut haqumei = Haqumei::new().unwrap();

    let pause = flatten_prosody(&haqumei.g2ipa_prosody("あっ。").unwrap());
    assert!(pause.contains(&ProsodicIpa::Boundary(IpaBoundary::Pause)));

    let accent_phrase = flatten_prosody(&haqumei.g2ipa_prosody("青い空").unwrap());
    assert!(accent_phrase.contains(&ProsodicIpa::Boundary(IpaBoundary::AccentPhrase)));
    assert!(
        accent_phrase
            .iter()
            .any(|item| matches!(item, ProsodicIpa::Token { prosody, .. }
                if prosody.iter().any(|item| matches!(item, IpaTokenProsody::Pitch(Some(_))))))
    );

    // 通常の IPA mapping には韻律境界を混ぜない。
    assert!(haqumei.g2ipa("あ あ").unwrap().iter().all(|word| {
        word.tokens
            .iter()
            .all(|token| matches!(token, IpaToken::Phone(_)))
    }));

    // 英単語間の空白と英単語内のハイフンは、休止境界に変えない。
    for text in ["I have", "end-to-end"] {
        assert!(
            !flatten_prosody(&haqumei.g2ipa_prosody(text).unwrap())
                .contains(&ProsodicIpa::Boundary(IpaBoundary::Pause))
        );
    }
}

#[test]
fn test_g2ipa_prosody_keeps_pitch_changes_inside_long_vowels() {
    let mut haqumei = Haqumei::new().unwrap();
    let prosody = flatten_prosody(&haqumei.g2ipa_prosody("コーヒー").unwrap());

    assert!(prosody.iter().any(|item| matches!(
        item,
        ProsodicIpa::Token {
            token: IpaToken::Phone(IpaPhone::LongO),
            prosody,
        } if prosody == &[
            IpaTokenProsody::Pitch(Some(PitchAccent::Low)),
            IpaTokenProsody::Pitch(Some(PitchAccent::High)),
        ]
    )));
    assert!(prosody.iter().any(|item| matches!(
        item,
        ProsodicIpa::Token {
            token: IpaToken::Phone(IpaPhone::LongI),
            prosody,
        } if prosody == &[
            IpaTokenProsody::Pitch(Some(PitchAccent::High)),
            IpaTokenProsody::Pitch(Some(PitchAccent::Low)),
        ]
    )));
}

#[test]
fn test_g2ipa_prosody_keeps_boundary_inside_compound_phone() {
    let mut haqumei = Haqumei::new().unwrap();
    let prosody = flatten_prosody(&haqumei.g2ipa_prosody("あっこれ").unwrap());

    assert!(prosody.iter().any(|item| matches!(
        item,
        ProsodicIpa::Token {
            token: IpaToken::Phone(IpaPhone::LongK),
            prosody,
        } if prosody == &[
            IpaTokenProsody::Pitch(Some(PitchAccent::Low)),
            IpaTokenProsody::Boundary(IpaBoundary::AccentPhrase),
            IpaTokenProsody::Pitch(Some(PitchAccent::Low)),
        ]
    )));
}

#[test]
fn test_g2ipa_ignores_trailing_spaces_for_utterance_final_allophones() {
    let mut haqumei = Haqumei::new().unwrap();

    for text in ["あっ", "本"] {
        let without_space = flatten(&haqumei.g2ipa(text).unwrap());
        let with_space = flatten(&haqumei.g2ipa(&format!("{text} ")).unwrap());
        assert_eq!(with_space, without_space, "{text}");
    }
}

#[test]
fn test_g2ipa_uses_context_across_accent_phrase_boundary() {
    let mut haqumei = Haqumei::new().unwrap();

    assert_eq!(
        flatten(&haqumei.g2ipa("5.6").unwrap()),
        [
            phone(IpaPhone::G),
            phone(IpaPhone::LongO),
            phone(IpaPhone::T),
            phone(IpaPhone::E),
            phone(IpaPhone::N),
            phone(IpaPhone::R),
            phone(IpaPhone::O),
            phone(IpaPhone::K),
            phone(IpaPhone::U),
        ]
    );

    let prosody = flatten_prosody(&haqumei.g2ipa_prosody("5.6").unwrap());
    assert!(prosody.windows(3).any(|window| matches!(
        window,
        [
            ProsodicIpa::Token {
                token: IpaToken::Phone(IpaPhone::N),
                ..
            },
            ProsodicIpa::Boundary(IpaBoundary::AccentPhrase),
            ProsodicIpa::Token {
                token: IpaToken::Phone(IpaPhone::R),
                ..
            }
        ]
    )));
}

#[test]
fn test_g2ipa_ignores_explicit_n_allophones() {
    let mut haqumei = Haqumei::new().unwrap();
    assert_eq!(
        flatten(&haqumei.g2ipa("漢字").unwrap()),
        [
            phone(IpaPhone::K),
            phone(IpaPhone::A),
            phone(IpaPhone::N),
            phone(IpaPhone::J),
            phone(IpaPhone::I),
        ]
    );

    let mut haqumei = Haqumei::with_options(HaqumeiOptions {
        split_n_allophones: true,
        split_n_before_r: true,
        split_n_before_palatal_affricate: true,
        ..Default::default()
    })
    .unwrap();

    assert_eq!(
        flatten(&haqumei.g2ipa("漢字").unwrap()),
        [
            phone(IpaPhone::K),
            phone(IpaPhone::A),
            phone(IpaPhone::N),
            phone(IpaPhone::J),
            phone(IpaPhone::I),
        ]
    );
    assert_eq!(
        flatten(&haqumei.g2ipa("新緑").unwrap()),
        [
            phone(IpaPhone::Sh),
            phone(IpaPhone::I),
            phone(IpaPhone::N),
            phone(IpaPhone::Ry),
            phone(IpaPhone::O),
            phone(IpaPhone::K),
            phone(IpaPhone::U),
        ]
    );
}

#[test]
fn test_g2ipa_is_independent_of_allophone_options() {
    let texts = [
        "漢字",
        "新緑",
        "学校",
        "バッグ",
        "あっ。",
        "あっ あ",
        "本 も",
        "5.6",
        "検査",
        "グッズ",
        "カンヒ",
    ];
    let mut baseline = Haqumei::new().unwrap();
    let expected: Vec<_> = texts
        .iter()
        .map(|text| baseline.g2ipa(text).unwrap())
        .collect();
    let expected_prosody: Vec<_> = texts
        .iter()
        .map(|text| baseline.g2ipa_prosody(text).unwrap())
        .collect();

    for mask in 0_u8..64 {
        let options = HaqumeiOptions {
            use_allophones: mask & 1 != 0,
            split_n_allophones: mask & 2 != 0,
            split_n_before_r: mask & 4 != 0,
            split_n_before_palatal_affricate: mask & 8 != 0,
            split_q_allophones: mask & 16 != 0,
            enable_final_glottal_stop: mask & 32 != 0,
            ..Default::default()
        };
        let mut haqumei = Haqumei::with_options(options).unwrap();
        for ((text, expected), expected_prosody) in
            texts.iter().zip(&expected).zip(&expected_prosody)
        {
            assert_eq!(haqumei.g2ipa(text).unwrap(), *expected, "{text}: {mask}");
            assert_eq!(
                haqumei.g2ipa_prosody(text).unwrap(),
                *expected_prosody,
                "{text}: {mask}"
            );
        }
    }
}

#[test]
fn test_g2ipa_batch_matches_sequential_calls() {
    let texts = ["学校", "恋愛", "あっ。"];
    let mut sequential = Haqumei::new().unwrap();
    let expected: Vec<_> = texts
        .iter()
        .map(|text| sequential.g2ipa(text).unwrap())
        .collect();
    let expected_prosody: Vec<_> = texts
        .iter()
        .map(|text| sequential.g2ipa_prosody(text).unwrap())
        .collect();

    let mut batched = Haqumei::new().unwrap();
    assert_eq!(batched.g2ipa_batch(&texts).unwrap(), expected);
    assert_eq!(
        batched.g2ipa_prosody_batch(&texts).unwrap(),
        expected_prosody
    );
}

#[test]
fn test_ipa_phone_is_typed_and_parseable() {
    assert_eq!("ɡʲː".parse::<IpaPhone>().unwrap(), IpaPhone::LongGy);
    assert_eq!(IpaPhone::NasalizedU.as_str(), "ɯ̃");
    assert!("not-ipa".parse::<IpaPhone>().is_err());
}

#[test]
fn test_ipa_phone_supports_string_hash_lookup() {
    use std::collections::HashSet;

    let phones: HashSet<_> = IpaPhone::ALL.iter().copied().collect();
    for phone in IpaPhone::ALL {
        assert!(phones.contains(phone.as_str()), "{phone}");
    }
    assert!(!phones.contains("not-ipa"));
}
