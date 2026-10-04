use std::collections::HashMap;

use crate::{MecabMorph, NjdFeature, njd_char_spans};

/// ユーザー辞書から引かれた形態素と重なる NJD 形態素を集め、
/// その位置 (文字単位の開始位置) から添字への対応を返す。
///
/// `dictionary_index` は `0` がシステム辞書で、`1` 以降が読み込み順の
/// ユーザー辞書に対応する。表層形ではなく位置で決めるので、同じ表層形が
/// 同じ文にシステム辞書側からも現れる場合に巻き込まない。
pub(crate) fn protected_indices(
    features: &[NjdFeature],
    morphs: &[MecabMorph],
) -> HashMap<usize, usize> {
    if !morphs.iter().any(MecabMorph::is_from_user_dictionary) {
        return HashMap::new();
    }
    njd_char_spans(features, morphs)
        .into_iter()
        .enumerate()
        .filter(|(_, span)| {
            span.start < span.end
                && morphs.iter().any(|m| {
                    m.is_from_user_dictionary()
                        && m.char_span.start < span.end
                        && span.start < m.char_span.end
                })
        })
        .map(|(idx, span)| (span.start, idx))
        .collect()
}

/// 表層形・文字区間・発音・モーラ数が登録時と一致する語の登録核を返す。
/// 読みや語の区切りが変わると登録核の位置を使えないため、保護対象から外す。
pub(crate) fn registered_accent_nuclei(
    features: &[NjdFeature],
    morphs: &[MecabMorph],
) -> Vec<Option<i32>> {
    if !morphs.iter().any(MecabMorph::is_from_user_dictionary) {
        return Vec::new();
    }
    let spans = njd_char_spans(features, morphs);
    let mut nuclei = vec![None; features.len()];
    let mut cursor = 0;
    for morph in morphs.iter().filter(|m| m.is_from_user_dictionary()) {
        let mut fields = morph.feature.split(',').skip(7);
        let Some(orig) = fields.next() else { continue };
        let Some(_) = fields.next() else { continue };
        let Some(pron) = fields.next() else { continue };
        let Some(accent) = fields.next() else {
            continue;
        };

        // 連語は原形の各要素に分割される。数字の展開や英数字の結合でできた語は
        // 登録時の区切りと一致しないので、核を保護しない。
        let words = if orig.contains(':') {
            orig
        } else {
            &morph.surface
        };
        if words
            .chars()
            .filter(|&c| c != ':')
            .ne(morph.surface.chars())
        {
            continue;
        }
        let count = words.split(':').count();
        if pron.split(':').count() != count || accent.split(':').count() != count {
            continue;
        }
        let mut start = morph.char_span.start;
        for ((word, pron), accent) in words.split(':').zip(pron.split(':')).zip(accent.split(':')) {
            let end = start + word.chars().count();
            while cursor < spans.len() && (spans[cursor].is_empty() || spans[cursor].end <= start) {
                cursor += 1;
            }
            if let Some(feature) = features.get(cursor)
                && spans[cursor] == (start..end)
                && end <= morph.char_span.end
                && feature.string == word
                && feature
                    .pron
                    .chars()
                    .filter(|&c| c != '’')
                    .eq(pron.chars().filter(|&c| c != '’'))
                && let Some((acc, mora)) = accent.split_once('/')
                && let (Ok(acc), Ok(mora)) = (acc.parse::<i32>(), mora.parse::<i32>())
                && acc > 0
                && acc <= mora
                && feature.mora_size == mora
            {
                nuclei[cursor] = Some(acc);
            }
            start = end;
        }
    }
    nuclei
}
