//! 正規化前の形と、解析後の語の境界を照合して顔文字の範囲を決めます。
//!
//! 半角濁点や丸数字は正規化で形が変わるため、形の判定には元入力を使います。
//! 元入力の範囲を正規化後の文字位置へ移し、ユーザー登録語や境界をまたぐ語を保護します。

#[cfg(test)]
mod tests;

pub(crate) mod normalization;
mod objects;
mod shape;

use crate::{MecabMorph, NO_DICTIONARY_INDEX, UnicodeNormalization as Mode};
use std::ops::Range;

pub(crate) struct Prepared {
    pub unicode: String,
    pub normalized: String,
    candidates: Vec<Candidate>,
}

struct Candidate {
    raw_core: Range<usize>,
    core: Range<usize>,
    extent: Range<usize>,
    base: Range<usize>,
}

fn grapheme_boundary(input: &str, at: usize) -> bool {
    unicode_segmentation::GraphemeCursor::new(at, input.len(), true)
        .is_boundary(input, 0)
        .expect("complete input")
}

fn complete_face_end(input: &str, at: usize) -> Option<usize> {
    if grapheme_boundary(input, at) {
        return Some(at);
    }
    let mut end = at;
    for c in input[at..].chars().take(8) {
        let decoration = unicode_normalization::char::canonical_combining_class(c) != 0
            || matches!(c, 'ﾞ' | 'ﾟ' | '\u{200d}')
            || ('\u{fe00}'..='\u{fe0f}').contains(&c)
            || ('\u{20d0}'..='\u{20ff}').contains(&c);
        if !decoration {
            return None;
        }
        end += c.len_utf8();
        if grapheme_boundary(input, end) {
            return Some(end);
        }
    }
    None
}

pub(crate) fn map_positions(input: &str, queries: &[usize]) -> (String, Vec<usize>) {
    let mut result = Vec::with_capacity(queries.len());
    let normalized =
        haqumei_jpreprocess::normalize_text_for_open_jtalk_with_mapping(input, |source, output| {
            while queries
                .get(result.len())
                .is_some_and(|&q| q <= source.start)
            {
                result.push(output.start);
            }
            while queries.get(result.len()).is_some_and(|&q| q <= source.end) {
                result.push(output.end);
            }
        });
    (normalized, result)
}

pub(crate) fn prepare(input: &str, mode: Mode) -> Option<Prepared> {
    let mut candidates = Vec::new();
    shape::find_parts(input, |c, e| candidates.push((c, e)));
    if candidates.is_empty() {
        return None;
    }
    let mut stack = Vec::new();
    let mut quotes = Vec::new();
    let mut face_at = 0;
    for (i, c) in input.char_indices() {
        while candidates
            .get(face_at)
            .is_some_and(|(core, _)| core.end <= i)
        {
            face_at += 1;
        }
        // 顔の中の「は曲げた腕にも使われる。本文の括弧と組み合わせると外の「が残る。
        if candidates
            .get(face_at)
            .is_some_and(|(core, _)| core.start <= i && i < core.end)
        {
            continue;
        }
        if matches!(c, '「' | '『') {
            stack.push((i, c));
        }
        if matches!(c, '」' | '』')
            && let Some(&(a, opening)) = stack.last()
            && matches!((opening, c), ('「', '」') | ('『', '』'))
        {
            stack.pop();
            quotes.push(a..a + opening.len_utf8());
            quotes.push(i..i + c.len_utf8());
        }
    }
    quotes.sort_unstable_by_key(|r| r.start);
    for (core, extent) in &mut candidates {
        let first = quotes.partition_point(|q| q.end <= extent.start);
        for quote in &quotes[first..] {
            if quote.start >= extent.end {
                break;
            }
            if quote.end <= core.start {
                extent.start = extent.start.max(quote.end);
            }
            if quote.start >= core.end {
                extent.end = extent.end.min(quote.start);
            }
        }
    }
    candidates.retain(|(c, e)| shape::valid_core(input, c.clone(), c != e));
    // 閉じ括弧に続くﾟも顔の装飾になる。結合文字を含めた範囲を最大8文字まで調べる。
    candidates.retain_mut(|(core, e)| {
        let boundary = |at| grapheme_boundary(input, at);
        // ｶﾞのﾞまで腕に含めた場合は、ｶﾞの直後から始める。
        if !boundary(e.start) {
            e.start = unicode_segmentation::GraphemeCursor::new(e.start, input.len(), true)
                .next_boundary(input, 0)
                .expect("complete input")
                .unwrap_or(input.len());
            if e.start > core.start {
                return false;
            }
        }
        if let Some(end) = complete_face_end(input, e.end) {
            e.end = end;
        } else {
            e.end = core.end;
        }
        boundary(e.start) && boundary(e.end) && shape::valid_core(input, core.clone(), core != e)
    });
    if candidates.is_empty() {
        return None;
    }
    let bases: Vec<_> = candidates.iter().map(|(_, e)| e.clone()).collect();
    for (core, extent) in &mut candidates {
        let extended = objects::extend(input, core, extent).extent;
        // 図形に結合文字が続く場合も、図形と文字を分けずに残す。
        if grapheme_boundary(input, extended.start) && grapheme_boundary(input, extended.end) {
            *extent = extended;
        }
    }
    let characters = normalization::annotated(input, mode);
    let unicode: String = characters.iter().map(|c| c.value).collect();
    let bytes: Vec<_> = unicode
        .char_indices()
        .map(|(i, _)| i)
        .chain([unicode.len()])
        .collect();
    struct Projection {
        core: Option<Range<usize>>,
        extent: Option<Range<usize>>,
        base: Option<Range<usize>>,
        clipped_start: bool,
        clipped_end: bool,
    }
    let mut ranges: Vec<_> = candidates
        .iter()
        .map(|_| Projection {
            core: None,
            extent: None,
            base: None,
            clipped_start: false,
            clipped_end: false,
        })
        .collect();
    for (i, c) in characters.iter().enumerate() {
        let first = candidates.partition_point(|(_, e)| e.end <= c.source.start);
        for j in first..candidates.len() {
            let (core, extent) = &candidates[j];
            if c.source.end <= extent.start {
                break;
            }
            ranges[j].extent.get_or_insert(i..i).end = i + 1;
            ranges[j].clipped_start |= c.source.start < extent.start;
            ranges[j].clipped_end |= extent.end < c.source.end;
            if c.source.start < core.end && core.start < c.source.end {
                ranges[j].core.get_or_insert(i..i).end = i + 1;
            }
            if c.source.start < bases[j].end && bases[j].start < c.source.end {
                ranges[j].base.get_or_insert(i..i).end = i + 1;
            }
        }
    }
    let queries: Vec<_> = ranges
        .into_iter()
        .flat_map(|r| {
            let core = r.core.expect("normalized face core");
            let mut extent = r.extent.expect("normalized face extent");
            let base = r.base.expect("normalized face without objects");
            if r.clipped_start {
                extent.start = core.start;
            }
            if r.clipped_end {
                extent.end = core.end;
            }
            [
                extent.start,
                core.start,
                core.end,
                extent.end,
                base.start,
                base.end,
            ]
        })
        .map(|i| bytes[i])
        .collect();
    let mut order: Vec<_> = queries.iter().copied().enumerate().collect();
    order.sort_unstable_by_key(|&(_, at)| at);
    let sorted: Vec<_> = order.iter().map(|&(_, at)| at).collect();
    let (normalized, mapped) = map_positions(&unicode, &sorted);
    let mut positions = vec![0; queries.len()];
    for ((index, _), at) in order.into_iter().zip(mapped) {
        positions[index] = at;
    }
    Some(Prepared {
        unicode,
        normalized,
        candidates: candidates
            .into_iter()
            .zip(positions.as_chunks::<6>().0)
            .map(
                |((raw_core, _), &[start, core_start, core_end, end, base_start, base_end])| {
                    Candidate {
                        raw_core,
                        core: core_start..core_end,
                        extent: start..end,
                        base: base_start..base_end,
                    }
                },
            )
            .collect(),
    })
}
impl Prepared {
    pub(crate) fn select(&self, input: &str, morphs: &[MecabMorph]) -> Vec<Range<usize>> {
        let normalized = self.normalized.as_str();
        let candidates = &self.candidates;
        let byte_offsets: Vec<_> = normalized
            .char_indices()
            .map(|(i, _)| i)
            .chain([normalized.len()])
            .collect();
        let mut cursor = 0;
        let rows = candidates.iter().filter_map(|candidate| {
            let raw_core = &candidate.raw_core;
            let core = candidate.core.clone();
            let proposed = candidate.extent.clone();
            let base = candidate.base.clone();
            let mut extent = proposed.clone();
            while morphs
                .get(cursor)
                .is_some_and(|m| m.char_span.end <= extent.start)
            {
                cursor += 1;
            }
            // 持ち物を取り消す条件を先に調べる。「ノー」を手と棒と扱った後では、ノだけが残る。
            if proposed != base {
                for m in &morphs[cursor..] {
                    if m.char_span.start >= proposed.end {
                        if base.end < proposed.end
                            && m.char_span.start == proposed.end
                            && m.feature.split(',').nth(3) == Some("助数詞")
                        {
                            extent.end = base.end;
                        }
                        break;
                    }
                    let carrier = core.end <= m.char_span.start
                        && m.char_span.start < base.end
                        && base.end < m.char_span.end
                        && m.char_span.end <= proposed.end
                        && objects::hand_and_bar(&m.surface);
                    let lexical = !m.is_unknown
                        && m.feature.split(',').nth(1) != Some("記号")
                        && m.surface.chars().count() > 1
                        && !carrier;
                    let crossing =
                        m.char_span.start < proposed.start || proposed.end < m.char_span.end;
                    if m.is_from_user_dictionary() || lexical || crossing {
                        if m.char_span.start < base.start {
                            extent.start = base.start;
                        }
                        if base.end < m.char_span.end {
                            extent.end = base.end;
                        }
                    }
                }
            }
            for m in &morphs[cursor..] {
                if m.char_span.start >= proposed.end {
                    break;
                }
                let intersects_core = m.char_span.start < core.end && core.start < m.char_span.end;
                if m.is_from_user_dictionary() && intersects_core {
                    return None;
                }
                let object_handle = base.end < extent.end
                    && core.end <= m.char_span.start
                    && m.char_span.start < base.end
                    && base.end < m.char_span.end
                    && m.char_span.end <= proposed.end
                    && objects::hand_and_bar(&m.surface);
                let lexical = !m.is_unknown
                    && m.feature.split(',').nth(1) != Some("記号")
                    && m.surface.chars().count() > 1
                    && !object_handle;
                let crosses_extent =
                    m.char_span.start < extent.start || extent.end < m.char_span.end;
                if crosses_extent && intersects_core {
                    return None;
                }
                if !intersects_core && (lexical || crosses_extent || m.is_from_user_dictionary()) {
                    if m.char_span.end <= core.start {
                        extent.start = extent.start.max(m.char_span.end).max(base.start);
                    }
                    if core.end <= m.char_span.start {
                        extent.end = extent.end.min(m.char_span.start).min(base.end);
                    }
                }
            }
            // 辞書語を残すために範囲を狭めた後も、腕と結合文字を分けない。
            {
                if !grapheme_boundary(normalized, byte_offsets[extent.start]) {
                    let next = unicode_segmentation::GraphemeCursor::new(
                        byte_offsets[extent.start],
                        normalized.len(),
                        true,
                    )
                    .next_boundary(normalized, 0)
                    .expect("complete normalized input")
                    .unwrap();
                    extent.start = byte_offsets.binary_search(&next).unwrap();
                }
                if !grapheme_boundary(normalized, byte_offsets[extent.end]) {
                    let previous = unicode_segmentation::GraphemeCursor::new(
                        byte_offsets[extent.end],
                        normalized.len(),
                        true,
                    )
                    .prev_boundary(normalized, 0)
                    .expect("complete normalized input")
                    .unwrap();
                    extent.end = byte_offsets.binary_search(&previous).unwrap();
                }
                if extent.start > core.start || extent.end < core.end {
                    return None;
                }
            }
            shape::valid_core(input, raw_core.clone(), extent != core).then_some(extent)
        });
        let mut groups: Vec<Range<usize>> = Vec::new();
        for range in rows {
            if let Some(previous) = groups.last_mut()
                && range.start < previous.end
            {
                previous.end = previous.end.max(range.end);
            } else {
                groups.push(range);
            }
        }
        groups
    }
}

/// 顔文字の範囲に含まれる形態素を一つの無読要素に置き換えます。
pub(crate) fn merge(normalized: &str, morphs: &mut Vec<MecabMorph>, ranges: &[Range<usize>]) {
    if ranges.is_empty() {
        return;
    }
    let offsets: Vec<_> = normalized
        .char_indices()
        .map(|(i, _)| i)
        .chain([normalized.len()])
        .collect();
    let mut source = std::mem::take(morphs).into_iter().peekable();
    for range in ranges {
        while source
            .peek()
            .is_some_and(|m| m.char_span.end <= range.start)
        {
            morphs.push(source.next().unwrap());
        }
        while source.peek().is_some_and(|m| m.char_span.start < range.end) {
            source.next();
        }
        let surface = normalized[offsets[range.start]..offsets[range.end]].to_owned();
        morphs.push(MecabMorph {
            feature: format!("{surface},その他,顔文字,*,*,*,*,{surface},,,0/0,*,1"),
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
}

pub(crate) fn is_face(morph: &MecabMorph) -> bool {
    let mut fields = morph.feature.split(',');
    fields.next();
    fields.next() == Some("その他") && fields.next() == Some("顔文字")
}

// 呼び出し側が書き換えた読みは、顔の部品であっても残す。
pub(crate) fn edited_ranges(before: &[MecabMorph], after: &[MecabMorph]) -> Vec<Range<usize>> {
    let mut changed = Vec::new();
    for i in 0..before.len().max(after.len()) {
        if before.get(i) != after.get(i) {
            changed.extend(before.get(i).map(|m| m.char_span.clone()));
            changed.extend(after.get(i).map(|m| m.char_span.clone()));
        }
    }
    changed.sort_unstable_by_key(|r| r.start);
    // 書き換え前後で範囲が異なる場合も、両方を保護する。
    let mut merged: Vec<Range<usize>> = Vec::new();
    for range in changed {
        if let Some(last) = merged.last_mut()
            && range.start <= last.end
        {
            last.end = last.end.max(range.end);
        } else {
            merged.push(range);
        }
    }
    merged
}

pub(crate) fn protect_edits(
    ranges: &mut Vec<Range<usize>>,
    before: &[MecabMorph],
    after: &[MecabMorph],
) {
    let merged = edited_ranges(before, after);
    ranges.retain(|range| {
        let i = merged.partition_point(|r| r.end <= range.start);
        !merged.get(i).is_some_and(|r| r.start < range.end)
    });
}
