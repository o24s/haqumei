//! 号室・号線・型番を認識し、数字の読み方と英字の文字名を指定します。

use std::ops::Range;

use crate::{HaqumeiOptions, MecabMorph, NO_DICTIONARY_INDEX, NumberReading};

pub(crate) fn may_contain_identifier(text: &str) -> bool {
    (text.contains(['号', '型', '形', '系']) || text.contains("品番") || text.contains("製品番号"))
        && text.chars().any(|c| digit(c).is_some())
}

fn digit(c: char) -> Option<u8> {
    match c {
        '0'..='9' => Some(c as u8 - b'0'),
        '０'..='９' => Some((c as u32 - '０' as u32) as u8),
        '〇' | '零' => Some(0),
        '一' => Some(1),
        '二' => Some(2),
        '三' => Some(3),
        '四' => Some(4),
        '五' => Some(5),
        '六' => Some(6),
        '七' => Some(7),
        '八' => Some(8),
        '九' => Some(9),
        _ => None,
    }
}

fn letter(c: char) -> bool {
    matches!(c, 'A'..='Z' | 'Ａ'..='Ｚ')
}

fn hyphen(c: char) -> bool {
    matches!(c, '-' | '−' | '－' | '‐' | '‑')
}

fn token_char(c: char) -> bool {
    digit(c).is_some() || letter(c) || hyphen(c)
}

fn space(c: char) -> bool {
    matches!(c, ' ' | '　')
}

fn outside_number(c: char) -> bool {
    token_char(c)
        || matches!(c, 'a'..='z' | 'ａ'..='ｚ' | 'Ⅰ'..='ↈ')
        || "十百千万億兆第.,．，・/／_＿+＋%％".contains(c)
}

fn starts_with(chars: &[char], at: usize, word: &str) -> bool {
    let mut tail = chars[at..].iter();
    word.chars().all(|c| tail.next() == Some(&c))
}

struct Candidate {
    number: Range<usize>,
    context: Range<usize>,
    reading: NumberReading,
    model: bool,
}

fn model_prefix(chars: &[char], start: usize) -> Option<usize> {
    let mut end = start;
    while end > 0 && space(chars[end - 1]) {
        end -= 1;
    }

    if end > 0 && matches!(chars[end - 1], 'は' | ':' | '：') {
        end -= 1;
        while end > 0 && space(chars[end - 1]) {
            end -= 1;
        }
    }

    ["型番", "型式", "品番", "製品番号"]
        .into_iter()
        .find_map(|word| {
            let begin = end.checked_sub(word.chars().count())?;
            starts_with(chars, begin, word).then_some(begin)
        })
}

fn model_code(token: &[char]) -> bool {
    let letters = token.iter().take_while(|&&c| letter(c)).count();
    let digits = if token.get(letters).is_some_and(|&c| hyphen(c)) {
        letters + 1
    } else {
        letters
    };

    // 「型」だけでは病名や化学式も候補になる。明示的な型番の見出しがない場合は、
    // 短い英字接頭部と数字の形式に限り、英単語や数字の後ろの英字を含めない。
    (1..=3).contains(&letters)
        && token[digits..]
            .iter()
            .all(|&c| digit(c).is_some() || hyphen(c))
}

fn candidates(chars: &[char], options: &HaqumeiOptions) -> Vec<Candidate> {
    let mut found = Vec::new();
    let mut at = 0;

    while at < chars.len() {
        let start = at;
        if !token_char(chars[at]) {
            at += 1;
            continue;
        }

        while at < chars.len() && token_char(chars[at]) {
            at += 1;
        }

        let token = &chars[start..at];
        if token.len() > 64
            || hyphen(token[0])
            || hyphen(*token.last().unwrap())
            || !token.iter().any(|&c| digit(c).is_some())
            || token.windows(2).any(|p| hyphen(p[0]) && hyphen(p[1]))
            || start
                .checked_sub(1)
                .is_some_and(|i| outside_number(chars[i]))
            || chars.get(at).is_some_and(|&c| outside_number(c))
        {
            continue;
        }

        let mut suffix = at;
        while chars.get(suffix).is_some_and(|&c| space(c)) {
            suffix += 1;
        }

        let only_digits = token.iter().all(|&c| digit(c).is_some());
        let (reading, context, model) = if only_digits && starts_with(chars, suffix, "号室") {
            (options.room_number_reading, start..suffix + 2, false)
        } else if only_digits && starts_with(chars, suffix, "号線") {
            (options.route_number_reading, start..suffix + 2, false)
        } else if let Some(prefix) = model_prefix(chars, start) {
            (options.model_number_reading, prefix..at, true)
        } else if (only_digits || model_code(token)) && starts_with(chars, suffix, "号機") {
            (options.model_number_reading, start..suffix + 2, true)
        } else if model_code(token)
            && chars
                .get(suffix)
                .is_some_and(|c| matches!(c, '型' | '形' | '系'))
        {
            (options.model_number_reading, start..suffix + 1, true)
        } else {
            continue;
        };

        found.push(Candidate {
            number: start..at,
            context,
            reading,
            model,
        });
    }

    found
}

fn covered<'a>(morphs: &'a [MecabMorph], range: &Range<usize>) -> Option<&'a [MecabMorph]> {
    let start = morphs.partition_point(|m| m.char_span.end <= range.start);
    let end = morphs.partition_point(|m| m.char_span.start < range.end);
    let parts = &morphs[start..end];

    (!parts.is_empty()
        && parts[0].char_span.start == range.start
        && parts.last().unwrap().char_span.end == range.end
        && parts
            .windows(2)
            .all(|p| p[0].char_span.end == p[1].char_span.start))
    .then_some(parts)
}

fn alphabet_feature(c: char) -> &'static str {
    let c = if c.is_ascii_uppercase() {
        char::from_u32(c as u32 + 0xfee0).unwrap()
    } else {
        c
    };

    crate::utils::get_known_symbol_feature(c.encode_utf8(&mut [0; 4])).unwrap()
}

fn alphabet_reading(text: &str) -> String {
    text.chars()
        .map(|c| alphabet_feature(c).split(',').nth(7).unwrap())
        .collect()
}

fn replaceable(morph: &MecabMorph) -> bool {
    if morph.is_unknown {
        return true;
    }

    if morph.surface.chars().all(|c| digit(c).is_some()) {
        return morph.feature.split(',').nth(2) == Some("数")
            || morph.surface.chars().all(|c| matches!(c, '〇' | '零'));
    }

    if morph.surface.chars().all(letter) {
        // 固有の読みを持つ略称は文字名で上書きしない。
        return morph.feature.split(',').nth(8) == Some(alphabet_reading(&morph.surface).as_str());
    }

    morph.surface.chars().all(hyphen)
}

fn synthetic(surface: String, range: Range<usize>, details: String) -> MecabMorph {
    MecabMorph {
        feature: format!("{surface},{details}"),
        surface,
        char_span: range,
        is_unknown: false,
        is_ignored: false,
        dictionary_index: NO_DICTIONARY_INDEX,
        left_id: 0,
        right_id: 0,
        pos_id: 0,
        word_cost: 0,
    }
}

fn expand(chars: &[char], candidate: &Candidate, output: &mut Vec<MecabMorph>) {
    let mut at = candidate.number.start;

    while at < candidate.number.end {
        let start = at;
        let kind = if digit(chars[at]).is_some() {
            0
        } else if letter(chars[at]) {
            1
        } else {
            2
        };
        at += 1;

        while at < candidate.number.end
            && match kind {
                0 => digit(chars[at]).is_some(),
                1 => letter(chars[at]),
                _ => false,
            }
        {
            at += 1;
        }

        let surface: String = chars[start..at].iter().collect();
        let details = match kind {
            0 => {
                let digits: String = chars[start..at]
                    .iter()
                    .map(|&c| char::from(b'0' + digit(c).unwrap()))
                    .collect();
                let mode = match candidate.reading {
                    NumberReading::Cardinal if !digits.starts_with('0') => "番号位",
                    NumberReading::DigitsWithMaru => "番号丸",
                    _ => "番号桁",
                };

                format!("名詞,数,{mode},*,*,*,{digits},*,*,0/0,*")
            }
            1 => {
                let reading = alphabet_reading(&surface);
                let moras = crate::utils::count_mora(&reading);
                let last = alphabet_feature(chars[at - 1]).split(',').nth(7).unwrap();
                let accent = moras - crate::utils::count_mora(last) + 1;

                format!(
                    "名詞,一般,番号英字,*,*,*,{surface},{reading},{reading},{accent}/{moras},*,0"
                )
            }
            _ => format!("その他,番号区切り,*,*,*,*,{surface},*,*,0/0,*"),
        };

        output.push(synthetic(surface, start..at, details));
    }
}

// 発音の長さでは番号の読み方を決められないため、用途と利用者の設定から指定する。
// https://github.com/tsukumijima/open_jtalk/commit/e3cc6330c6afff76ab1b580d8bc67a591265ef1b
pub(crate) fn merge(
    text: &str,
    morphs: &mut Vec<MecabMorph>,
    edited: &[Range<usize>],
    options: &HaqumeiOptions,
) -> Vec<Range<usize>> {
    if !options.resolve_number_identifiers || !may_contain_identifier(text) {
        return Vec::new();
    }

    let chars: Vec<_> = text.chars().collect();
    let accepted: Vec<_> = candidates(&chars, options)
        .into_iter()
        .filter(|candidate| {
            let Some(context) = covered(morphs, &candidate.context) else {
                return false;
            };
            let Some(parts) = covered(morphs, &candidate.number) else {
                return false;
            };
            let edit = edited.partition_point(|r| r.end <= candidate.context.start);

            if context
                .iter()
                .any(|m| m.is_from_user_dictionary() || crate::kaomoji::is_face(m))
                || edited
                    .get(edit)
                    .is_some_and(|r| r.start < candidate.context.end)
                || !parts.iter().all(replaceable)
            {
                return false;
            }

            // 「型番123円」の数量や、既知語の「系統」の一部を型番の接尾辞に使わない。
            if candidate.model && candidate.context.end == candidate.number.end {
                let after = morphs.partition_point(|m| m.char_span.start < candidate.number.end);
                if morphs[after..]
                    .iter()
                    .find(|m| !m.is_ignored)
                    .is_some_and(|m| {
                        m.feature.split(',').nth(3) == Some("助数詞")
                            || matches!(m.surface.as_str(), "円" | "万" | "億" | "兆")
                    })
                {
                    return false;
                }
            }

            true
        })
        .collect();

    if accepted.is_empty() {
        return Vec::new();
    }

    let mut source = std::mem::take(morphs).into_iter().peekable();
    for candidate in &accepted {
        while source
            .peek()
            .is_some_and(|m| m.char_span.end <= candidate.number.start)
        {
            morphs.push(source.next().unwrap());
        }

        while source
            .peek()
            .is_some_and(|m| m.char_span.start < candidate.number.end)
        {
            source.next();
        }

        expand(&chars, candidate, morphs);

        if candidate.model && chars[candidate.context.end - 1] == '形' {
            while source
                .peek()
                .is_some_and(|m| m.char_span.end <= candidate.context.end)
            {
                let mut morph = source.next().unwrap();

                // 型番に続く形は、単独名詞のカタチではなく接尾辞のガタとして読む。
                if morph.surface == "形"
                    && matches!(
                        morph.feature.split(',').nth(8),
                        Some("カタチ" | "ナリ" | "カタ")
                    )
                {
                    morph.feature = "形,名詞,接尾,一般,*,*,*,形,ガタ,ガタ,0/2,C2".into();
                }

                morphs.push(morph);
            }
        }
    }

    morphs.extend(source);

    accepted.into_iter().map(|c| c.context).collect()
}

#[cfg(test)]
mod tests {
    use crate::{Haqumei, HaqumeiOptions, MecabDictIndexCompiler, open_jtalk::Dictionary};

    #[test]
    fn dictionary_entries_in_the_number_or_its_context_are_preserved() {
        let dictionary = Dictionary::from_embedded().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let csv = temp.path().join("identifiers.csv");
        let dic = temp.path().join("identifiers.dic");

        let rows: String = ["８０２", "号線", "ＡＢ１２３", "型番", "形"]
            .into_iter()
            .map(|word| {
                format!("{word},1345,1345,-20000,名詞,一般,*,*,*,*,{word},テスト,テスト,1/3,C1\n")
            })
            .collect();
        std::fs::write(&csv, rows).unwrap();
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
                    protect_user_dict_readings: protect,
                    ..Default::default()
                },
            )
            .unwrap();

            for text in ["802号室", "409号線", "AB123型", "型番A320", "E5000形"] {
                engine.options.resolve_number_identifiers = false;
                let expected = engine.g2p_prosody(text).unwrap();
                engine.options.resolve_number_identifiers = true;

                assert!(engine.g2k(text).unwrap().contains("テスト"), "{text}");
                assert_eq!(engine.g2p_prosody(text).unwrap(), expected, "{text}");
                assert_eq!(
                    engine.g2p_mapping_detailed(text).unwrap(),
                    engine.g2p_candidates_detailed(text).unwrap().candidates[0].words,
                    "{text}"
                );
            }
        }
    }
}
