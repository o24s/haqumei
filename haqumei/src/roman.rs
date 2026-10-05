//! ローマ数字を正規化前に認識し、数詞処理へ渡す形態素にまとめます。

use crate::{MecabMorph, NO_DICTIONARY_INDEX, UnicodeNormalization};
use std::ops::Range;
const TOKENS: &[(u16, &[u8])] = &[
    (1000, b"M"),
    (900, b"CM"),
    (500, b"D"),
    (400, b"CD"),
    (100, b"C"),
    (90, b"XC"),
    (50, b"L"),
    (40, b"XL"),
    (10, b"X"),
    (9, b"IX"),
    (5, b"V"),
    (4, b"IV"),
    (1, b"I"),
];
const SYMBOLS: [&[u8]; 16] = [
    b"I", b"II", b"III", b"IV", b"V", b"VI", b"VII", b"VIII", b"IX", b"X", b"XI", b"XII", b"L",
    b"C", b"D", b"M",
];

pub(crate) fn parse(text: &str) -> Option<u16> {
    if text.is_empty() || text.len() > 45 {
        return None;
    }
    let mut buf = [0u8; 15];
    let mut used = 0;
    let mut dedicated = None;
    for c in text.chars() {
        let is_dedicated = ('Ⅰ'..='ⅿ').contains(&c);
        if dedicated.is_some_and(|before| before != is_dedicated) {
            return None;
        }
        dedicated = Some(is_dedicated);
        if is_dedicated {
            let expansion = SYMBOLS[(c as usize - 'Ⅰ' as usize) % 16];
            if used + expansion.len() > buf.len() {
                return None;
            }
            buf[used..used + expansion.len()].copy_from_slice(expansion);
            used += expansion.len();
        } else {
            let c = if ('Ａ'..='Ｚ').contains(&c) || ('ａ'..='ｚ').contains(&c) {
                char::from_u32(c as u32 - 0xfee0)?
            } else {
                c
            };
            if !c.is_ascii() || used == buf.len() {
                return None;
            }
            let c = (c as u8).to_ascii_uppercase();
            if !b"IVXLCDM".contains(&c) {
                return None;
            }
            buf[used] = c;
            used += 1;
        }
    }
    let s = &buf[..used];
    let value = |c| match c {
        b'I' => 1i32,
        b'V' => 5,
        b'X' => 10,
        b'L' => 50,
        b'C' => 100,
        b'D' => 500,
        b'M' => 1000,
        _ => unreachable!(),
    };
    let n: i32 = s
        .iter()
        .enumerate()
        .map(|(i, &c)| {
            let v = value(c);
            if s.get(i + 1).is_some_and(|&next| value(next) > v) {
                -v
            } else {
                v
            }
        })
        .sum();
    if !(1..=3999).contains(&n) {
        return None;
    }
    // 減算可能な組と反復上限は、最短表記への再生成でまとめて検証する。
    let mut rest = n as u16;
    let mut at = 0;
    for &(v, token) in TOKENS {
        while rest >= v {
            if s.get(at..at + token.len()) != Some(token) {
                return None;
            }
            at += token.len();
            rest -= v;
        }
    }
    (at == s.len()).then_some(n as u16)
}

pub(crate) struct Prepared {
    pub unicode: String,
    pub normalized: String,
    candidates: Vec<(Range<usize>, u16)>,
}

fn token_character(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(c, 'Ａ'..='Ｚ' | 'ａ'..='ｚ' | '０'..='９' | 'Ⅰ'..='ↈ' | '_' | '＿')
}

fn numeral(c: char) -> bool {
    matches!(
        c,
        '〇' | '零'
            | '一'
            | '二'
            | '三'
            | '四'
            | '五'
            | '六'
            | '七'
            | '八'
            | '九'
            | '十'
            | '百'
            | '千'
            | '万'
            | '億'
            | '兆'
    )
}

fn boundary(input: &str, at: usize) -> bool {
    unicode_segmentation::GraphemeCursor::new(at, input.len(), true)
        .is_boundary(input, 0)
        .expect("complete input")
}

pub(crate) fn prepare(input: &str, mode: UnicodeNormalization) -> Option<Prepared> {
    // 専用文字のUTF-8表現はE2で始まる。E2も「第」もない文は文字へ展開せず返す。
    if !input.as_bytes().contains(&0xe2) && !input.contains('第') {
        return None;
    }
    let mut candidates = Vec::new();
    let mut chars = input.char_indices().peekable();
    while let Some((start, c)) = chars.next() {
        if !token_character(c) {
            continue;
        }
        let mut end = start + c.len_utf8();
        while chars.peek().is_some_and(|&(_, c)| token_character(c)) {
            let (at, c) = chars.next().unwrap();
            end = at + c.len_utf8();
        }
        let token = &input[start..end];
        let dedicated = ('Ⅰ'..='ⅿ').contains(&c);
        // CD・MIX などを数値と読む根拠は綴りだけでは得られない。
        // ラテン文字は「第 + 数値 + 章・節・巻・部・項・編」の形に限る。
        let ordinal = input[..start].ends_with('第')
            && input[end..].starts_with(['章', '節', '巻', '部', '項', '編']);
        if !(dedicated || ordinal)
            || input[..start].chars().next_back().is_some_and(numeral)
            || input[end..].chars().next().is_some_and(numeral)
            || !boundary(input, start)
            || !boundary(input, end)
        {
            continue;
        }
        if let Some(value) = parse(token) {
            candidates.push((start..end, value));
        }
    }
    if candidates.is_empty() {
        return None;
    }
    let characters = crate::kaomoji::normalization::annotated(input, mode);
    let unicode: String = characters.iter().map(|c| c.value).collect();
    let bytes: Vec<_> = unicode
        .char_indices()
        .map(|(i, _)| i)
        .chain([unicode.len()])
        .collect();
    let mut queries = Vec::with_capacity(candidates.len() * 2);
    let mut cursor = 0;
    for (range, _) in &candidates {
        while cursor < characters.len() && characters[cursor].source.end <= range.start {
            cursor += 1;
        }
        queries.push(bytes[cursor]);
        while cursor < characters.len() && characters[cursor].source.start < range.end {
            cursor += 1;
        }
        queries.push(bytes[cursor]);
    }
    let (normalized, positions) = crate::kaomoji::map_positions(&unicode, &queries);
    let candidates = candidates
        .into_iter()
        .zip(positions.as_chunks::<2>().0)
        .map(|((_, value), &[start, end])| (start..end, value))
        .collect();
    Some(Prepared {
        unicode,
        normalized,
        candidates,
    })
}

pub(crate) fn is_roman(morph: &MecabMorph) -> bool {
    morph.dictionary_index == NO_DICTIONARY_INDEX
        && morph.feature.split(',').nth(3) == Some("ローマ数字")
}

impl Prepared {
    pub(crate) fn merge(
        &self,
        morphs: &mut Vec<MecabMorph>,
        edited: &[Range<usize>],
    ) -> Vec<Range<usize>> {
        let mut accepted = Vec::new();
        let mut cursor = 0;
        for (range, value) in &self.candidates {
            let edited_at = edited.partition_point(|r| r.end <= range.start);
            while morphs
                .get(cursor)
                .is_some_and(|m| m.char_span.end <= range.start)
            {
                cursor += 1;
            }
            let start = cursor;
            while morphs
                .get(cursor)
                .is_some_and(|m| m.char_span.start < range.end)
            {
                cursor += 1;
            }
            let parts = &morphs[start..cursor];
            // 登録語の一部や呼び出し側が変更した区間は、数詞として分割し直さない。
            if parts.is_empty()
                || parts[0].char_span.start != range.start
                || parts.last().unwrap().char_span.end != range.end
                || parts
                    .iter()
                    .any(|m| m.is_from_user_dictionary() || crate::kaomoji::is_face(m))
                || parts
                    .windows(2)
                    .any(|p| p[0].char_span.end != p[1].char_span.start)
                || edited.get(edited_at).is_some_and(|r| r.start < range.end)
            {
                continue;
            }
            accepted.push((range.clone(), *value));
        }
        if accepted.is_empty() {
            return Vec::new();
        }
        let offsets: Vec<_> = self
            .normalized
            .char_indices()
            .map(|(i, _)| i)
            .chain([self.normalized.len()])
            .collect();
        let mut source = std::mem::take(morphs).into_iter().peekable();
        for (range, value) in &accepted {
            while source
                .peek()
                .is_some_and(|m| m.char_span.end <= range.start)
            {
                morphs.push(source.next().unwrap());
            }
            while source.peek().is_some_and(|m| m.char_span.start < range.end) {
                source.next();
            }
            let surface = self.normalized[offsets[range.start]..offsets[range.end]].to_owned();
            morphs.push(MecabMorph {
                feature: format!("{surface},名詞,数,ローマ数字,*,*,*,{value},*,*,0/0,*"),
                surface,
                char_span: range.clone(),
                is_unknown: false,
                is_ignored: false,
                dictionary_index: NO_DICTIONARY_INDEX,
                left_id: 0,
                right_id: 0,
                pos_id: 0,
                word_cost: 0,
            });
        }
        morphs.extend(source);
        accepted.into_iter().map(|(range, _)| range).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn render(mut n: u16) -> String {
        let mut s = String::new();
        for &(v, t) in TOKENS {
            while n >= v {
                s.push_str(std::str::from_utf8(t).unwrap());
                n -= v;
            }
        }
        s
    }
    #[test]
    fn canonical_parser_roundtrips_all_values_and_rejects_noncanonical_strings() {
        for n in 1..=3999 {
            let s = render(n);
            let dedicated: String = s
                .chars()
                .map(|c| {
                    SYMBOLS
                        .iter()
                        .position(|s| *s == [c as u8])
                        .map(|i| char::from_u32('Ⅰ' as u32 + i as u32).unwrap())
                        .unwrap()
                })
                .collect();
            for form in [
                s.clone(),
                s.to_lowercase(),
                s.chars()
                    .map(|c| char::from_u32(c as u32 + 0xfee0).unwrap())
                    .collect(),
                dedicated.clone(),
                dedicated
                    .chars()
                    .map(|c| char::from_u32(c as u32 + 16).unwrap())
                    .collect(),
            ] {
                assert_eq!(parse(&form), Some(n), "{form}");
            }
        }
        for len in 1..=6 {
            for mut index in 0..7usize.pow(len) {
                let mut buf = [0; 6];
                for c in &mut buf[..len as usize] {
                    *c = b"IVXLCDM"[index % 7];
                    index /= 7;
                }
                let s = std::str::from_utf8(&buf[..len as usize]).unwrap();
                if let Some(n) = parse(s) {
                    assert_eq!(render(n), s);
                }
            }
        }
        for s in [
            "",
            "IIII",
            "IIX",
            "VX",
            "IC",
            "IL",
            "XM",
            "IIV",
            "VV",
            "MMMM",
            "Ⅳ4",
            "4Ⅳ",
            "ⅫⅪ",
            "ⅣⅢ",
            "ⅩIV",
            "Ⅳ\u{0305}",
            "ↁ",
            "N",
        ] {
            assert_eq!(parse(s), None, "{s}");
        }
        assert_eq!(parse(&"Ⅿ".repeat(100000)), None);
    }
}

#[cfg(test)]
mod user_dictionary_tests {
    use crate::{
        Haqumei, HaqumeiOptions, MecabDictIndexCompiler, UnicodeNormalization,
        open_jtalk::Dictionary,
    };

    #[test]
    fn registered_counter_readings_take_precedence_without_protection_option() {
        let dictionary = Dictionary::from_embedded().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let csv = temp.path().join("counter.csv");
        let dic = temp.path().join("counter.dic");
        std::fs::write(
            &csv,
            ["巻", "日", "人", "日間"]
                .map(|s| {
                    format!("{s},1345,1345,-20000,名詞,一般,*,*,*,*,{s},テスト,テスト,1/3,C1\n")
                })
                .join(""),
        )
        .unwrap();
        MecabDictIndexCompiler::new()
            .dict_dir(&dictionary.dict_dir)
            .userdict_out_path(&dic)
            .add_input_file(&csv)
            .run()
            .unwrap();
        let mut engine =
            Haqumei::from_path_with_userdict(&dictionary.dict_dir, &dic, HaqumeiOptions::default())
                .unwrap();
        for text in ["Ⅻ巻", "Ⅻ 巻", "Ⅳ日", "Ⅰ人", "ⅩⅣ日間", "ⅩⅩⅣ日"] {
            assert!(engine.g2k(text).unwrap().ends_with("テスト"), "{text}");
            assert_eq!(
                engine.g2p_mapping_detailed(text).unwrap(),
                engine.g2p_candidates_detailed(text).unwrap().candidates[0].words
            );
        }
    }

    #[test]
    fn registered_numerals_and_compounds_take_precedence() {
        let dictionary = Dictionary::from_embedded().unwrap();
        let temp = tempfile::tempdir().unwrap();
        for mode in [
            UnicodeNormalization::None,
            UnicodeNormalization::Nfc,
            UnicodeNormalization::Nfkc,
        ] {
            let surface = if mode == UnicodeNormalization::Nfkc {
                "ＸＩＩ"
            } else {
                "Ⅻ"
            };
            for suffix in ["", "章"] {
                let surface = format!("{surface}{suffix}");
                let csv = temp.path().join("user.csv");
                let dic = temp.path().join("user.dic");
                std::fs::write(&csv, format!("{surface},1345,1345,-20000,名詞,一般,*,*,*,*,{surface},テスト,テスト,1/3,C1\n")).unwrap();
                MecabDictIndexCompiler::new()
                    .dict_dir(&dictionary.dict_dir)
                    .userdict_out_path(&dic)
                    .add_input_file(&csv)
                    .run()
                    .unwrap();
                for protect in [false, true] {
                    let mut engine = Haqumei::from_path_with_userdict(
                        &dictionary.dict_dir,
                        &dic,
                        HaqumeiOptions {
                            normalize_unicode: mode,
                            protect_user_dict_readings: protect,
                            ..Default::default()
                        },
                    )
                    .unwrap();
                    let text = format!("Ⅻ{suffix}");
                    assert_eq!(engine.g2k(&text).unwrap(), "テスト");
                    assert_eq!(
                        engine.g2p_mapping_detailed(&text).unwrap(),
                        engine.g2p_candidates_detailed(&text).unwrap().candidates[0].words
                    );
                }
            }
        }
    }
}
