#[cfg(test)]
mod tests {
    use haqumei::{
        Haqumei, HaqumeiOptions, Phoneme, ProsodyFormat,
        prosody::{PitchAccent, ProsodicPhoneme},
        utils::default_is_non_pause_symbol,
    };

    /// ピッチ付きの音素を生成する (High)
    fn h(ph: Phoneme) -> ProsodicPhoneme {
        ProsodicPhoneme::Phoneme {
            phoneme: ph,
            pitch: Some(PitchAccent::High),
        }
    }

    /// ピッチ付きの音素を生成する (Low)
    fn l(ph: Phoneme) -> ProsodicPhoneme {
        ProsodicPhoneme::Phoneme {
            phoneme: ph,
            pitch: Some(PitchAccent::Low),
        }
    }

    fn ap() -> ProsodicPhoneme {
        ProsodicPhoneme::AccentPhraseBoundary
    }
    fn pau() -> ProsodicPhoneme {
        ProsodicPhoneme::Pause
    }
    fn inter() -> ProsodicPhoneme {
        ProsodicPhoneme::Interrogative
    }
    fn excl() -> ProsodicPhoneme {
        ProsodicPhoneme::Exclamatory
    }

    #[test]
    fn test_basic_prosody_and_accent_phrase() {
        let mut haqumei = Haqumei::new().unwrap();
        // 青い(中高型 f2=2) 空(頭高型 f2=1)
        let mapping = haqumei.g2p_mapping_prosody("青い空が、好きだ。").unwrap();
        assert_eq!(mapping.len(), 7);

        assert_eq!(mapping[0].word, "青い");
        assert_eq!(
            mapping[0].phonemes,
            vec![
                l(Phoneme::A), // 1モーラ目
                h(Phoneme::O), // 2モーラ目 (アクセント核)
                l(Phoneme::I), // 核を過ぎたのでLow
                ap(),          // 句境界
            ]
        );

        assert_eq!(mapping[1].word, "空");
        assert_eq!(
            mapping[1].phonemes,
            vec![
                h(Phoneme::S), // 1モーラ目 (頭高型なのでHigh)
                h(Phoneme::O), // 同上
                l(Phoneme::R), // 2モーラ目 (核を過ぎたのでLow)
                l(Phoneme::A), // 同上
            ]
        );

        assert_eq!(mapping[2].word, "が");
        assert_eq!(mapping[2].phonemes, vec![l(Phoneme::G), l(Phoneme::A),]);

        assert_eq!(mapping[3].word, "、");
        assert_eq!(mapping[3].phonemes, vec![ProsodicPhoneme::Pause,]);

        assert_eq!(mapping[4].word, "好き");
        assert_eq!(
            mapping[4].phonemes,
            vec![
                l(Phoneme::S),
                l(Phoneme::UnvoicedU),
                h(Phoneme::K),
                h(Phoneme::I),
            ]
        );

        assert_eq!(mapping[5].word, "だ");
        assert_eq!(mapping[5].phonemes, vec![l(Phoneme::D), l(Phoneme::A),]);

        assert_eq!(mapping[6].word, "。");
        assert_eq!(mapping[6].phonemes, vec![ProsodicPhoneme::Pause,]);
    }

    #[test]
    fn test_head_high_accent() {
        let mut haqumei = Haqumei::new().unwrap();
        // 命(頭高型 f2=1): i(High) -> no(Low) -> chi(Low)
        let mapping = haqumei.g2p_mapping_prosody("命").unwrap();

        assert_eq!(mapping[0].word, "命");
        assert_eq!(
            mapping[0].phonemes,
            vec![
                h(Phoneme::I),
                l(Phoneme::N),
                l(Phoneme::O),
                l(Phoneme::Ch),
                l(Phoneme::I),
            ]
        );
    }

    #[test]
    fn test_split_prefix_accent_phrase() {
        // 指示的な漢語接頭辞は後続語と融合せず、別のアクセント句になる
        //   本論文 → ホ]ン | ロンブン
        let mut haqumei = Haqumei::new().unwrap();
        let mapping = haqumei.g2p_mapping_prosody("本論文").unwrap();

        assert_eq!(mapping[0].word, "本");
        assert_eq!(mapping[0].phonemes.last(), Some(&ap()));
        // 平板なので句頭のモーラだけ Low で、2 モーラ目から High になる
        assert_eq!(
            mapping[1].phonemes,
            vec![
                l(Phoneme::R),
                l(Phoneme::O),
                h(Phoneme::Nn),
                h(Phoneme::B),
                h(Phoneme::U),
                h(Phoneme::Nn),
            ]
        );

        // 無効にすると Open JTalk と同じ、接頭辞ごとひとつのアクセント句になる
        let mut haqumei = Haqumei::with_options(HaqumeiOptions {
            split_prefix_accent_phrase: false,
            ..Default::default()
        })
        .unwrap();
        let mapping = haqumei.g2p_mapping_prosody("本論文").unwrap();

        assert!(mapping.iter().all(|m| !m.phonemes.contains(&ap())));
        // 接頭辞の核が句全体を支配するので、以降がすべて Low に潰れる
        assert!(
            mapping[1]
                .phonemes
                .iter()
                .all(|p| matches!(p, ProsodicPhoneme::Phoneme { pitch: Some(PitchAccent::Low), .. }))
        );
    }

    #[test]
    fn test_explicit_punctuation_marks() {
        let mut haqumei = Haqumei::new().unwrap();
        let mapping = haqumei.g2p_mapping_prosody("えっ？嘘！").unwrap();

        let has_inter = mapping.iter().any(|m| m.phonemes.contains(&inter()));
        let has_excl = mapping.iter().any(|m| m.phonemes.contains(&excl()));

        assert!(has_inter, "Should contain Interrogative marker");
        assert!(has_excl, "Should contain Exclamatory marker");
    }

    #[test]
    fn test_is_non_pause_symbol_respected() {
        let mut haqumei = Haqumei::with_options(HaqumeiOptions {
            is_non_pause_symbol: default_is_non_pause_symbol,
            ..Default::default()
        })
        .unwrap();

        let mapping = haqumei.g2p_mapping_prosody("「こんにちは」").unwrap();

        for map in &mapping {
            if map.word == "「" || map.word == "」" {
                assert!(
                    !map.phonemes.contains(&pau()),
                    "Bracket '{}' should NOT contain Pause due to is_non_pause_symbol",
                    map.word
                );
            }
        }
    }

    #[test]
    fn test_long_vowel_merging() {
        let mut haqumei = Haqumei::new().unwrap();
        let mapping = haqumei
            .g2p_mapping_prosody("つまみ出されようとした")
            .unwrap();

        let has_merged_word = mapping.iter().any(|m| m.word == "れよう");
        let has_empty_u = mapping.iter().any(|m| m.word == "う");

        assert!(
            has_merged_word,
            "The word 'う' should be merged into 'れよ'"
        );
        assert!(!has_empty_u, "The standalone 'う' should be removed");
    }

    #[test]
    fn test_bos_eos_silence_handling() {
        let mut haqumei = Haqumei::new().unwrap();
        let mapping = haqumei.g2p_mapping_prosody("あ").unwrap();

        assert_eq!(mapping.len(), 1);
        assert_eq!(mapping[0].word, "あ");

        assert_eq!(mapping[0].phonemes[0], h(Phoneme::A));
    }

    #[test]
    fn test_g2p_prosody_exclamation_support() {
        let mut haqumei = Haqumei::new().unwrap();

        let prosody = haqumei.g2p_prosody("そうですか！").unwrap();
        let mut prosody = prosody.iter().rev();
        prosody.next();
        assert_eq!(prosody.next().unwrap(), "!",);

        let prosody = haqumei.g2p_prosody("そうですか！？").unwrap();
        let mut prosody = prosody.iter().rev();
        assert_eq!(prosody.next().unwrap(), "$",);
        assert_eq!(prosody.next().unwrap(), "?",);
        assert_eq!(prosody.next().unwrap(), "!",);
    }

    #[test]
    fn test_g2p_prosody_basic_patterns() {
        let mut haqumei = Haqumei::new().unwrap();

        // 平板型: 「こんにちは」
        // [k o [ N n i ch i w a $]
        let p1 = haqumei.g2p_prosody("こんにちは。").unwrap();
        assert_eq!(
            p1,
            vec![
                "^", "k", "o", "[", "N", "n", "i", "ch", "i", "w", "a", "_", "$"
            ]
        );

        // 頭高型: 「テスト」
        // [t e ] s u t o $]
        let p2 = haqumei.g2p_prosody("テスト").unwrap();
        assert_eq!(p2, vec!["^", "t", "e", "]", "s", "U", "t", "o", "$"]);

        // 中高型: 「バドミントン」
        let p3 = haqumei.g2p_prosody("バドミントン。").unwrap();
        // b a [ d o m i ] N t o N _ $
        assert_eq!(
            p3,
            vec![
                "^", "b", "a", "[", "d", "o", "m", "i", "]", "N", "t", "o", "N", "_", "$"
            ]
        );
    }

    #[test]
    fn test_g2p_prosody_boundaries_and_pauses() {
        let mut haqumei = Haqumei::new().unwrap();

        // アクセント句境界 (#) と 読点ポーズ (_)
        // 「青い空、広がる」
        let p1 = haqumei.g2p_prosody("青い空、広がる。").unwrap();

        // 「青い」と「空」の間には境界 # が入る
        // 読点の場所にはポーズ _ が入る
        assert_eq!(
            p1,
            [
                "^", "a", "[", "o", "]", "i", "#", "s", "o", "]", "r", "a", "_", "h", "i", "[",
                "r", "o", "g", "a", "r", "u", "_", "$"
            ]
        );
    }

    #[test]
    fn test_g2p_prosody_complex_mixed() {
        let mut haqumei = Haqumei::new().unwrap();

        let text = "えっ、本当！？?!！？！？すごい！";
        let p = haqumei.g2p_prosody(text).unwrap().join(" ");

        assert_eq!(p.chars().next().unwrap(), '^');

        assert!(p.contains(&"! ? ? ! ! ? ! ?".to_string()));

        assert_eq!(p.chars().next_back().unwrap(), '$');

        assert!(p.contains(&"cl".to_string()));
    }

    #[test]
    fn test_prosody_no_crash_on_empty() {
        let mut haqumei = Haqumei::new().unwrap();
        let p = haqumei.g2p_prosody("").unwrap().join("");
        assert_eq!(p, "^$");
    }

    #[test]
    fn test_prosody_long_vowels_and_special_kana() {
        let mut haqumei = Haqumei::new().unwrap();
        // 「東京」 -> t o o [ ky o o $
        let p = haqumei.g2p_prosody("東京").unwrap();
        assert!(p.contains(&"o".to_string()));
        assert!(p.contains(&"[".to_string()));
    }

    #[test]
    fn test_prosody_interrogative_exclamatory() {
        let mut haqumei = Haqumei::new().unwrap();

        let mut p = haqumei.g2p_mapping_prosody("こんにちは!??!").unwrap();

        assert_eq!(p.pop().unwrap().phonemes, [ProsodicPhoneme::Exclamatory]);
        assert_eq!(p.pop().unwrap().phonemes, [ProsodicPhoneme::Interrogative]);
        assert_eq!(p.pop().unwrap().phonemes, [ProsodicPhoneme::Interrogative]);
        assert_eq!(p.pop().unwrap().phonemes, [ProsodicPhoneme::Exclamatory]);
    }

    #[test]
    fn test_format_default() {
        let mut haqumei = Haqumei::new().unwrap();

        // Nür が未知語となる
        let result = haqumei
            .g2p_prosody_with_options("私は Nür で走る", ProsodyFormat::Default)
            .unwrap()
            .join(" ");

        let expected = "^ w a [ t a sh i w a # sp { e [ n u a a r u } sp d e # h a [ sh i ] r u $";

        assert_eq!(result, expected);
    }

    #[test]
    fn test_format_absolute_prefix() {
        let mut haqumei = Haqumei::new().unwrap();

        let result = haqumei
            .g2p_prosody_with_options("私は Nür で走る", ProsodyFormat::Prefix)
            .unwrap()
            .join(" ");

        let expected = "^ L_w L_a H_t H_a H_sh H_i H_w H_a # sp { L_e H_n H_u H_a H_a H_r H_u } sp H_d H_e # L_h L_a H_sh H_i L_r L_u $";

        assert_eq!(result, expected);
    }

    #[test]
    fn test_format_absolute_numeric() {
        let mut haqumei = Haqumei::new().unwrap();

        let result = haqumei
            .g2p_prosody_with_options("私は Nür で走る", ProsodyFormat::Numeric)
            .unwrap()
            .join(" ");

        let expected = "^ w:0 a:0 t:1 a:1 sh:1 i:1 w:1 a:1 # sp { e:0 n:1 u:1 a:1 a:1 r:1 u:1 } sp d:1 e:1 # h:0 a:0 sh:1 i:1 r:0 u:0 $";

        assert_eq!(result, expected);
    }

    #[test]
    fn test_format_consecutive_symbols() {
        let mut haqumei = Haqumei::new().unwrap();

        let result = haqumei
            .g2p_prosody_with_options("えっ？？！！", ProsodyFormat::Numeric)
            .unwrap()
            .join(" ");

        let expected = "^ e:1 cl:0 ? ? ! ! $";

        assert_eq!(result, expected);
    }

    #[test]
    fn test_merged_symbols_decomposition_to_prosody() {
        let mut haqumei = Haqumei::new().unwrap();

        let mapping1 = haqumei
            .g2p_mapping_prosody("うおお！！！！！！！！！！！！！！！！")
            .unwrap();

        let exclamations: Vec<_> = mapping1.iter().filter(|m| m.word == "！").collect();
        assert_eq!(exclamations.len(), 16);
        assert!(exclamations.iter().all(|m| !m.is_unknown),);
        assert!(
            exclamations
                .iter()
                .all(|m| m.phonemes == vec![ProsodicPhoneme::Exclamatory]),
        );

        let mapping2 = haqumei
            .g2p_mapping_prosody("マジ！？！？！？！？！？！？！？！？")
            .unwrap();

        let symbols2: Vec<_> = mapping2
            .iter()
            .filter(|m| m.word == "！" || m.word == "？")
            .collect();
        assert_eq!(symbols2.len(), 16);

        for (i, sym) in symbols2.iter().enumerate() {
            assert!(!sym.is_unknown);
            if i % 2 == 0 {
                assert_eq!(sym.word, "！");
                assert_eq!(sym.phonemes, vec![ProsodicPhoneme::Exclamatory]);
            } else {
                assert_eq!(sym.word, "？");
                assert_eq!(sym.phonemes, vec![ProsodicPhoneme::Interrogative]);
            }
        }

        let prosody_str = haqumei
            .g2p_prosody("マジ！？！？！？！？！？！？！？！？")
            .unwrap()
            .join(" ");
        assert!(prosody_str.contains("! ? ! ? ! ? ! ? ! ? ! ? ! ? ! ?"));

        let mapping3 = haqumei
            .g2p_mapping_prosody("＼！－！－！？？＼！－！－！？？")
            .unwrap();

        let hyphens: Vec<_> = mapping3.iter().filter(|m| m.word == "－").collect();
        assert_eq!(hyphens.len(), 4);
        assert!(hyphens.iter().all(|m| m.is_unknown),);

        let bangs: Vec<_> = mapping3.iter().filter(|m| m.word == "！").collect();
        assert_eq!(bangs.len(), 6);
        assert!(
            bangs
                .iter()
                .all(|m| !m.is_unknown && m.phonemes == vec![ProsodicPhoneme::Exclamatory]),
        );
    }
}
