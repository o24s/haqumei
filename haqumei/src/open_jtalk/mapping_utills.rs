use crate::MecabMorph;

#[inline]
pub(super) fn adjust_unknown_phonemes(phonemes: &mut Vec<String>, is_unknown: bool) {
    if is_unknown && (phonemes.is_empty() || *phonemes == ["pau"]) {
        *phonemes = vec!["unk".to_string()];
    }
}

#[inline]
pub(super) fn consume_odori_morphs(
    morphs: &[MecabMorph],
    morph_idx: usize,
    map_word: &str,
) -> usize {
    let mut consumed = 1;
    if let Some(ahead) = morphs.get(morph_idx + 1)
        && !ahead.is_ignored
        && map_word.ends_with(&ahead.surface)
    {
        consumed += 1;
    }
    consumed
}

#[inline]
pub(super) fn consume_mismatched_morphs(
    morphs: &[MecabMorph],
    morph_idx: usize,
    bases_after_current: usize,
) -> usize {
    // 残りの有効な morph 数と、残りの base_mapping 数を計算
    let non_ignored_remaining = morphs[morph_idx..].iter().filter(|m| !m.is_ignored).count();

    // NJD 挿入ノード: morph を消費しない (e.g, "10" -> "十" などで桁が挿入された)
    if non_ignored_remaining <= bases_after_current {
        return 0;
    }

    let mut consumed = 1;
    while let Some(ahead) = morphs.get(morph_idx + consumed) {
        if ahead.is_ignored {
            break;
        }
        if !matches!(
            ahead.surface.as_str(),
            "０" | "１" | "２" | "３" | "４" | "５" | "６" | "７" | "８" | "９"
        ) {
            break;
        }

        let non_ign = morphs[(morph_idx + consumed)..]
            .iter()
            .filter(|m| !m.is_ignored)
            .count();

        if non_ign > bases_after_current {
            consumed += 1;
        } else {
            break;
        }
    }
    consumed
}
