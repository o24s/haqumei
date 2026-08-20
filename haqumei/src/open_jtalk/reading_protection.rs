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
