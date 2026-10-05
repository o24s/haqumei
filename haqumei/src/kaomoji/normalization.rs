use crate::UnicodeNormalization;
use std::ops::Range;
use unicode_normalization::char::{
    canonical_combining_class, compose, decompose_canonical, decompose_compatible,
};

#[derive(Clone, Debug)]
pub struct Character {
    pub value: char,
    pub source: Range<usize>,
}

/// 正規化で合成・展開した文字にも、元入力での区間を残します。
pub fn annotated(text: &str, mode: UnicodeNormalization) -> Vec<Character> {
    let mut decomposed = Vec::new();
    for (at, c) in text.char_indices() {
        let source = at..at + c.len_utf8();
        let mut emit = |value| {
            decomposed.push(Character {
                value,
                source: source.clone(),
            })
        };
        match mode {
            UnicodeNormalization::None => emit(c),
            UnicodeNormalization::Nfc => decompose_canonical(c, &mut emit),
            UnicodeNormalization::Nfkc => decompose_compatible(c, &mut emit),
        }
    }
    if matches!(mode, UnicodeNormalization::None) {
        return decomposed;
    }
    let mut start = 0;
    for end in 0..=decomposed.len() {
        if end == decomposed.len() || canonical_combining_class(decomposed[end].value) == 0 {
            decomposed[start..end].sort_by_key(|c| canonical_combining_class(c.value));
            start = end + 1;
        }
    }
    let mut output: Vec<Character> = Vec::with_capacity(decomposed.len());
    let mut starter: Option<usize> = None;
    let mut last_class = 0;
    for character in decomposed {
        let class = canonical_combining_class(character.value);
        if let Some(i) = starter
            && (last_class == 0 || last_class < class)
            && let Some(value) = compose(output[i].value, character.value)
        {
            output[i].value = value;
            output[i].source.start = output[i].source.start.min(character.source.start);
            output[i].source.end = output[i].source.end.max(character.source.end);
        } else {
            if class == 0 {
                starter = Some(output.len());
            }
            last_class = class;
            output.push(character);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use unicode_normalization::UnicodeNormalization as _;

    #[test]
    fn annotated_normalization_matches_standard_composition_and_ordering() {
        for text in [
            "ｶﾞ(^_^)㈱",
            "か\u{3099}(^_^)後",
            "\u{0301}\u{0323}顔",
            "a\u{0315}\u{0300}\u{05ae}\u{0301}",
            "\u{1100}\u{1161}\u{11a8}",
            "ÅΩﬃ①㌘(^_^)つ☕️",
            "(^_^)\u{200d}💻",
        ] {
            for mode in [
                UnicodeNormalization::None,
                UnicodeNormalization::Nfc,
                UnicodeNormalization::Nfkc,
            ] {
                let expected: String = match mode {
                    UnicodeNormalization::None => text.to_owned(),
                    UnicodeNormalization::Nfc => text.nfc().collect(),
                    UnicodeNormalization::Nfkc => text.nfkc().collect(),
                };
                let characters = annotated(text, mode);
                assert_eq!(
                    characters.iter().map(|c| c.value).collect::<String>(),
                    expected
                );
                assert!(
                    characters
                        .iter()
                        .all(|c| !c.source.is_empty() && text.get(c.source.clone()).is_some())
                );
            }
        }
        let kana = annotated("か\u{3099}", UnicodeNormalization::Nfc);
        assert_eq!(kana[0].source, 0..6);
        let expansion = annotated("㈱", UnicodeNormalization::Nfkc);
        assert!(expansion.iter().all(|c| c.source == (0..3)));
    }
}
