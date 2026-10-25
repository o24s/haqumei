//! 漢数字の途中から始まる語を数詞として解析し直す。

use std::collections::BTreeSet;

use haqumei_jpreprocess_dictionary::mecab::{Analysis, Node, Worker};

fn numeral(c: char) -> bool {
    "〇一二三四五六七八九十百千万億兆".contains(c)
}

fn numeral_word(text: &str, node: &Node) -> bool {
    text[node.byte_span.clone()].chars().all(numeral)
}

fn number(node: &Node) -> bool {
    node.feature.starts_with("名詞,数,")
}

fn numeric_pronunciation(text: &str, node: &Node) -> bool {
    let Some(pron) = node.feature.split(',').nth(8) else {
        return false;
    };
    let Some(first) = text[node.byte_span.clone()].chars().next() else {
        return false;
    };
    let prefixes: &[&str] = match first {
        '一' => &["イチ", "イッ"],
        '二' => &["ニ"],
        '三' => &["サン"],
        '四' => &["ヨン", "シ"],
        '五' => &["ゴ"],
        '六' => &["ロク", "ロッ"],
        '七' => &["ナナ", "シチ"],
        '八' => &["ハチ", "ハッ"],
        '九' => &["キュー", "ク"],
        '十' => &["ジュー", "ジュッ"],
        _ => &[],
    };
    prefixes.iter().any(|prefix| pron.starts_with(prefix))
}

pub(super) fn resolve(
    worker: &mut Worker,
    text: &str,
    original: Analysis,
    lattice: bool,
) -> std::io::Result<Analysis> {
    let path: Vec<_> = original
        .best_path
        .iter()
        .map(|&i| &original.nodes[i])
        .collect();
    let mut boundaries = BTreeSet::new();
    let mut chain_begin = 0;
    for (i, node) in path.iter().enumerate() {
        let start = node.byte_span.start;
        if i > 0 {
            let previous = path[i - 1];
            let adjacent = previous.byte_span.end == start;
            // 「唯一一人」の唯一まで遡らず、漢数字だけの語が続いた範囲を使う。
            if adjacent
                && numeral_word(text, previous)
                && text[start..].chars().next().is_some_and(numeral)
                && ((number(previous) && text[node.byte_span.clone()].chars().count() > 1)
                    || (number(node) && text[previous.byte_span.clone()].chars().count() > 1))
            {
                let end = text[node.byte_span.clone()]
                    .char_indices()
                    .find(|&(_, c)| !numeral(c))
                    .map_or(node.byte_span.end, |(at, _)| start + at);
                boundaries.extend(
                    text[chain_begin..end]
                        .char_indices()
                        .map(|(at, _)| chain_begin + at),
                );
                boundaries.insert(end);
            }
        }
        if numeral_word(text, node)
            && (i == 0 || path[i - 1].byte_span.end != start || !numeral_word(text, path[i - 1]))
        {
            chain_begin = start;
        }
    }
    // 番号中の九九・〇〇も数として分けるが、語全体を登録した読みは後で保護する。
    let chars: Vec<_> = text.char_indices().collect();
    let digit = |c: char| {
        "〇一二三四五六七八九".contains(c) || c.is_ascii_digit() || ('０'..='９').contains(&c)
    };
    let hyphen = |c| matches!(c, '-' | '−' | '－' | '‐' | '‑' | '‒' | '–' | '—' | 'ー');
    let mut i = 0;
    while i < chars.len() {
        if !"〇一二三四五六七八九".contains(chars[i].1) {
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && "〇一二三四五六七八九".contains(chars[i].1) {
            i += 1
        }
        let linked = (start >= 2 && hyphen(chars[start - 1].1) && digit(chars[start - 2].1))
            || (i + 1 < chars.len() && hyphen(chars[i].1) && digit(chars[i + 1].1));
        if linked {
            boundaries.extend(chars[start..i].iter().map(|&(at, _)| at));
            boundaries.insert(chars.get(i).map_or(text.len(), |&(at, _)| at));
        }
    }
    boundaries.retain(|&at| {
        !path.iter().any(|node| {
            if node.feature.starts_with("名詞,固有名詞,")
                && !numeral_word(text, node)
                && node.byte_span.start < at
                && at < node.byte_span.end
            {
                return true;
            }
            if node.dictionary_index == 0
                || node.is_unknown
                || !(node.byte_span.start < at && at < node.byte_span.end)
            {
                return false;
            }
            let start = node.byte_span.start;
            let inside_number = text[..start].chars().next_back().is_some_and(numeral)
                && text[start..].chars().next().is_some_and(numeral);
            // 三重塔・十回転は登録した数詞の読みを使い、二人のフタリは複合数の中で分ける。
            !inside_number || (!numeral_word(text, node) && numeric_pronunciation(text, node))
        })
    });
    if boundaries.is_empty() {
        return Ok(original);
    }

    // 解析器に境界制約の API がないため空白で語を分け、位置は元のバイト位置へ戻す。
    let mut separated = String::with_capacity(text.len() + boundaries.len());
    let mut offsets = Vec::with_capacity(separated.capacity() + 1);
    for (at, ch) in text.char_indices() {
        if boundaries.contains(&at) && at > 0 {
            separated.push(' ');
            offsets.push(at);
        }
        separated.push(ch);
        offsets.extend(at..at + ch.len_utf8());
    }
    offsets.push(text.len());
    let mut result = if lattice {
        worker.analyze_lattice(&separated)?
    } else {
        worker.analyze(&separated)?
    };
    for node in &mut result.nodes {
        node.byte_span = offsets[node.byte_span.start]..offsets[node.byte_span.end];
    }
    Ok(result)
}
