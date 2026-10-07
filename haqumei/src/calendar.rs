//! 年が明示された暦の日付を、空白と元の文字位置を保って数詞処理へ渡します。

use std::ops::Range;

use crate::{MecabMorph, NO_DICTIONARY_INDEX};

pub(crate) fn may_contain_date(text: &str) -> bool {
    text.contains('年') && text.contains('月') && text.contains([' ', '　'])
}

fn digit(c: char) -> Option<u16> {
    match c {
        '０'..='９' => Some(c as u16 - '０' as u16),
        '0'..='9' => Some(c as u16 - '0' as u16),
        _ => None,
    }
}

struct Field {
    range: Range<usize>,
    kind: &'static str,
    value: u16,
}

fn skip_spaces(chars: &[char], mut at: usize) -> usize {
    while chars.get(at).is_some_and(|c| matches!(c, ' ' | '　')) {
        at += 1;
    }

    at
}

fn field(chars: &[char], start: usize, unit: char, max_digits: usize) -> Option<Field> {
    let mut at = start;
    let mut value = 0;
    let first_year = unit == '年' && chars.get(at) == Some(&'元');

    if first_year {
        value = 1;
        at += 1;
    } else {
        while let Some(d) = chars.get(at).and_then(|&c| digit(c)) {
            if at - start == max_digits {
                return None;
            }

            value = value * 10 + d;
            at += 1;
        }

        if at == start {
            return None;
        }
    }

    at = skip_spaces(chars, at);
    if chars.get(at) != Some(&unit) {
        return None;
    }

    Some(Field {
        range: start..at + 1,
        kind: match unit {
            '年' if first_year => "暦元年",
            '年' => "暦年",
            '月' => "暦月",
            '日' => "暦日",
            _ => unreachable!(),
        },
        value,
    })
}

fn parse(chars: &[char], start: usize) -> Option<(Range<usize>, Vec<Field>)> {
    let era = [
        (['令', '和'], 2018),
        (['平', '成'], 1988),
        (['昭', '和'], 1925),
        (['大', '正'], 1911),
        (['明', '治'], 1867),
    ]
    .into_iter()
    .find(|(name, _)| chars[start..].starts_with(name));

    let year_start = if era.is_some() {
        skip_spaces(chars, start + 2)
    } else {
        start
    };
    let year = field(chars, year_start, '年', 4)?;
    if (era.is_none() && (year.kind == "暦元年" || year.value < 1000))
        || (era.is_some() && !(1..=99).contains(&year.value))
    {
        return None;
    }

    let month = field(chars, skip_spaces(chars, year.range.end), '月', 2)?;
    if !(1..=12).contains(&month.value) {
        return None;
    }

    let day = field(chars, skip_spaces(chars, month.range.end), '日', 2);
    let gregorian_year = year.value + era.map_or(0, |(_, offset)| offset);
    let max_day = match month.value {
        4 | 6 | 9 | 11 => 30,
        2 if gregorian_year % 400 == 0
            || (gregorian_year % 4 == 0 && gregorian_year % 100 != 0) =>
        {
            29
        }
        2 => 28,
        _ => 31,
    };
    if day
        .as_ref()
        .is_some_and(|d| !(1..=max_day).contains(&d.value))
    {
        return None;
    }

    let end = day.as_ref().map_or(month.range.end, |d| d.range.end);
    if !chars[start..end].iter().any(|c| matches!(c, ' ' | '　')) {
        return None;
    }

    let mut fields = vec![year, month];
    fields.extend(day);

    Some((start..end, fields))
}

// 「1 月に一度」は期間にも読めるため、西暦4桁か元号を伴う年月に限る。
// https://github.com/tsukumijima/open_jtalk/commit/46739160128d471996e878c9617d42cbd918a334
pub(crate) fn merge(
    text: &str,
    morphs: &mut Vec<MecabMorph>,
    edited: &[Range<usize>],
) -> Vec<Range<usize>> {
    if !may_contain_date(text) {
        return Vec::new();
    }

    let mut fields = Vec::new();
    let mut accepted = Vec::new();
    let offsets: Vec<_> = text
        .char_indices()
        .map(|(i, _)| i)
        .chain([text.len()])
        .collect();
    let chars: Vec<_> = text.chars().collect();
    let mut cursor = 0;

    while cursor < chars.len() {
        let at = cursor;
        cursor += 1;

        if at.checked_sub(1).is_some_and(|i| {
            let c = chars[i];
            c.is_ascii_alphanumeric()
                || matches!(c, '０'..='９' | 'Ａ'..='Ｚ' | 'ａ'..='ｚ' | 'Ⅰ'..='ⅿ')
                || "第〇零一二三四五六七八九十百千万億兆".contains(c)
        }) {
            continue;
        }

        let Some((range, date_fields)) = parse(&chars, at) else {
            continue;
        };
        cursor = range.end;

        let after = text[offsets[range.end]..].trim_start_matches([' ', '　']);
        if [
            "間",
            "ほど",
            "余り",
            "あまり",
            "かかった",
            "経過",
            "が経",
            "が過",
            "日間",
            "曜日",
            "曜",
            "面",
            "次",
            "額",
            "刊",
            "目",
        ]
        .iter()
        .any(|s| after.starts_with(s))
        {
            continue;
        }

        let start = morphs.partition_point(|m| m.char_span.end <= range.start);
        let end = morphs.partition_point(|m| m.char_span.start < range.end);
        let parts = &morphs[start..end];
        let edit = edited.partition_point(|r| r.end <= range.start);

        if parts.is_empty()
            || parts[0].char_span.start != range.start
            || parts.last().unwrap().char_span.end != range.end
            || parts
                .iter()
                .any(|m| m.is_from_user_dictionary() || crate::kaomoji::is_face(m))
            || parts
                .windows(2)
                .any(|p| p[0].char_span.end != p[1].char_span.start)
            || edited.get(edit).is_some_and(|r| r.start < range.end)
        {
            continue;
        }

        // 「日用品」が「日」「用品」に分かれても、日付の終端とは見なさない。
        if morphs[end..]
            .iter()
            .find(|m| !m.is_ignored)
            .is_some_and(|m| {
                !crate::kaomoji::is_face(m)
                    && !matches!(
                        m.feature.split(',').nth(1),
                        Some("助詞" | "助動詞" | "記号")
                    )
            })
        {
            continue;
        }

        if date_fields.iter().any(|field| {
            let range = &field.range;
            let start = morphs.partition_point(|m| m.char_span.end <= range.start);
            let end = morphs.partition_point(|m| m.char_span.start < range.end);

            // 数字と単位の両端が語境界にあることも確認する。「日誌」の一部は使わない。
            start == end
                || morphs[start].char_span.start != range.start
                || morphs[end - 1].char_span.end != range.end
        }) {
            continue;
        }

        fields.extend(date_fields);
        accepted.push(range);
    }

    if fields.is_empty() {
        return accepted;
    }

    let mut source = std::mem::take(morphs).into_iter().peekable();
    for Field { range, kind, value } in fields {
        while source
            .peek()
            .is_some_and(|m| m.char_span.end <= range.start)
        {
            morphs.push(source.next().unwrap());
        }

        while source.peek().is_some_and(|m| m.char_span.start < range.end) {
            source.next();
        }

        let surface = text[offsets[range.start]..offsets[range.end]].to_owned();
        morphs.push(MecabMorph {
            feature: format!("{surface},名詞,数,{kind},*,*,*,{value},*,*,0/0,*"),
            surface,
            char_span: range,
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

    accepted
}

#[cfg(test)]
mod tests {
    use crate::{Haqumei, HaqumeiOptions, MecabDictIndexCompiler, open_jtalk::Dictionary};

    #[test]
    fn user_dictionary_month_is_not_replaced_by_calendar_reading() {
        let dictionary = Dictionary::from_embedded().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let csv = temp.path().join("month.csv");
        let dic = temp.path().join("month.dic");

        std::fs::write(
            &csv,
            "月,1345,1345,-20000,名詞,一般,*,*,*,*,月,テスト,テスト,1/3,C1\n",
        )
        .unwrap();

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

            for input in ["2008 年 05 月 05 日", "令和 6 年 04 月 01 日"] {
                assert!(engine.g2k(input).unwrap().contains("テスト"));

                assert_eq!(
                    engine.run_frontend_detailed(input).unwrap().1,
                    engine.run_mecab_detailed(input).unwrap()
                );

                assert_eq!(
                    engine.g2p_mapping_detailed(input).unwrap(),
                    engine.g2p_candidates_detailed(input).unwrap().candidates[0].words
                );
            }
        }
    }
}
