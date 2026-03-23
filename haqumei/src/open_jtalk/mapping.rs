use crate::errors::HaqumeiError;
use crate::ffi;
use crate::utils::has_odori_chars;
use crate::{MecabMorph, OpenJTalk};
use crate::{NjdFeature, WordPhonemeDetail, WordPhonemeMap, WordPhonemePair};

use std::collections::HashMap;
use std::ffi::CStr;

pub(crate) trait WordPhonemeEntry {
    fn phonemes_mut(&mut self) -> &mut Vec<String>;
    fn phonemes(&self) -> &[String];

    /// 他の要素が空音素としてマージされる際に、テキストや付随情報を自身に結合する
    fn merge_from(&mut self, other: &mut Self);
}

impl WordPhonemeEntry for WordPhonemePair {
    fn phonemes_mut(&mut self) -> &mut Vec<String> {
        &mut self.phonemes
    }
    fn phonemes(&self) -> &[String] {
        &self.phonemes
    }

    fn merge_from(&mut self, other: &mut Self) {
        let text_to_merge = std::mem::take(&mut other.word);
        self.word.push_str(&text_to_merge);
    }
}

impl WordPhonemeEntry for WordPhonemeDetail {
    fn phonemes_mut(&mut self) -> &mut Vec<String> {
        &mut self.phonemes
    }
    fn phonemes(&self) -> &[String] {
        &self.phonemes
    }

    fn merge_from(&mut self, other: &mut Self) {
        let text_to_merge = std::mem::take(&mut other.word);
        self.word.push_str(&text_to_merge);

        self.mora_count += other.mora_count;

        // orig は辞書の原形を表すため、活用形の吸収では連結しないが、
        // リテラルの長音記号 ("ー") が吸収された場合は入力テキストを保持するため連結する
        if !other.orig.is_empty() && other.orig.chars().all(|c| c == 'ー') {
            let orig_to_merge = std::mem::take(&mut other.orig);
            self.orig.push_str(&orig_to_merge);
        }

        let read_to_merge = std::mem::take(&mut other.read);
        self.read.push_str(&read_to_merge);

        let pron_to_merge = std::mem::take(&mut other.pron);
        self.pron.push_str(&pron_to_merge);
    }
}

pub(crate) trait IntoPhonemeMapItem: Sized {
    type Output;

    fn word(&self) -> &str;

    /// is_ignored な形態素用の出力を生成
    fn new_ignored(surface: String, is_unknown: bool) -> Self::Output;

    /// morphs が尽きた場合の処理
    fn into_unmatched_remainder(self) -> Self::Output;

    /// 完全一致の場合の処理
    fn into_exact_match(self, morph: &MecabMorph) -> Self::Output;

    /// 先頭一致（結合）の場合の処理
    fn into_prefix_match(self, is_unknown_word: bool) -> Self::Output;

    /// 不一致の場合の処理
    fn into_mismatch(self) -> Self::Output;
}

impl IntoPhonemeMapItem for WordPhonemePair {
    type Output = WordPhonemeMap;

    #[inline]
    fn word(&self) -> &str {
        &self.word
    }

    #[inline]
    fn new_ignored(surface: String, is_unknown: bool) -> Self::Output {
        WordPhonemeMap {
            word: surface,
            phonemes: vec!["sp".to_string()],
            is_unknown,
            is_ignored: true,
        }
    }

    #[inline]
    fn into_unmatched_remainder(self) -> Self::Output {
        let is_ignored = self.phonemes.is_empty();
        WordPhonemeMap {
            word: self.word,
            phonemes: self.phonemes,
            is_unknown: false,
            is_ignored,
        }
    }

    #[inline]
    fn into_exact_match(self, morph: &MecabMorph) -> Self::Output {
        // JPCommonが音素を割り当てなかったとき is_ignored にする
        let is_ignored = self.phonemes.is_empty();
        let mut phonemes = self.phonemes;

        if morph.is_unknown && (phonemes.is_empty() || phonemes == ["pau"]) {
            phonemes = vec!["unk".to_string()];
        }

        WordPhonemeMap {
            word: self.word,
            phonemes,
            is_unknown: morph.is_unknown,
            is_ignored,
        }
    }

    #[inline]
    fn into_prefix_match(self, is_unknown_word: bool) -> Self::Output {
        let mut phonemes = self.phonemes;
        let is_ignored = phonemes.is_empty();

        if is_unknown_word && (phonemes.is_empty() || phonemes == ["pau"]) {
            phonemes = vec!["unk".to_string()];
        }

        WordPhonemeMap {
            word: self.word,
            phonemes,
            is_unknown: is_unknown_word,
            is_ignored,
        }
    }

    #[inline]
    fn into_mismatch(self) -> Self::Output {
        let is_ignored = self.phonemes.is_empty();
        WordPhonemeMap {
            word: self.word,
            phonemes: self.phonemes,
            is_unknown: false,
            is_ignored,
        }
    }
}

impl IntoPhonemeMapItem for WordPhonemeDetail {
    type Output = WordPhonemeDetail;

    #[inline]
    fn word(&self) -> &str {
        &self.word
    }

    #[inline]
    fn new_ignored(surface: String, is_unknown: bool) -> Self::Output {
        WordPhonemeDetail {
            word: surface.clone(),
            phonemes: vec!["sp".to_string()],
            features: Vec::new(),
            pos: "記号".to_string(),
            pos_group1: "空白".to_string(),
            pos_group2: "*".to_string(),
            pos_group3: "*".to_string(),
            ctype: "*".to_string(),
            cform: "*".to_string(),
            orig: surface.clone(),
            read: surface.clone(),
            pron: surface,
            accent_nucleus: 0,
            mora_count: 0,
            chain_rule: "*".to_string(),
            chain_flag: -1,
            is_unknown,
            is_ignored: true,
        }
    }

    #[inline]
    fn into_unmatched_remainder(mut self) -> Self::Output {
        self.is_ignored = self.phonemes.is_empty();
        self
    }

    #[inline]
    fn into_exact_match(mut self, morph: &MecabMorph) -> Self::Output {
        if morph.is_unknown && (self.phonemes.is_empty() || self.phonemes == ["pau"]) {
            self.phonemes = vec!["unk".to_string()];
        }
        self.is_unknown = morph.is_unknown;

        // JPCommonが音素を割り当てなかったとき is_ignored にする
        self.is_ignored = self.phonemes.is_empty();
        self.features = morph.feature.split(',').map(|s| s.to_string()).collect();
        self
    }

    #[inline]
    fn into_prefix_match(mut self, is_unknown_word: bool) -> Self::Output {
        if is_unknown_word && (self.phonemes.is_empty() || self.phonemes == ["pau"]) {
            self.phonemes = vec!["unk".to_string()];
        }
        self.is_unknown = is_unknown_word;
        self.is_ignored = self.phonemes.is_empty();
        self.features = Vec::new();
        self
    }

    #[inline]
    fn into_mismatch(mut self) -> Self::Output {
        self.is_unknown = false;
        self.is_ignored = self.phonemes.is_empty();
        self.features = Vec::new();
        self
    }
}

#[inline(always)]
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

#[inline(always)]
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

impl OpenJTalk {
    pub(crate) fn g2p_pairs_inner(
        &mut self,
        njd_features: &[NjdFeature],
        is_non_pause_symbol: fn(&str) -> bool,
    ) -> Result<Vec<WordPhonemePair>, HaqumeiError> {
        let mut mapping: Vec<WordPhonemePair> = njd_features
            .iter()
            .map(|f| WordPhonemePair {
                word: f.string.clone(),
                phonemes: Vec::new(),
            })
            .collect();

        self.assign_and_merge_phonemes(njd_features, &mut mapping, is_non_pause_symbol)?;
        Ok(mapping)
    }

    pub(crate) fn g2p_mapping_inner(
        &mut self,
        njd_features: &[NjdFeature],
        is_non_pause_symbol: fn(&str) -> bool,
    ) -> Result<Vec<WordPhonemeDetail>, HaqumeiError> {
        let mut mapping: Vec<WordPhonemeDetail> = njd_features
            .iter()
            .map(|f| WordPhonemeDetail {
                word: f.string.clone(),
                phonemes: Vec::new(),
                features: Vec::new(),
                pos: f.pos.clone(),
                pos_group1: f.pos_group1.clone(),
                pos_group2: f.pos_group2.clone(),
                pos_group3: f.pos_group3.clone(),
                ctype: f.ctype.clone(),
                cform: f.cform.clone(),
                orig: f.orig.clone(),
                read: f.read.clone(),
                pron: f.pron.clone(),
                accent_nucleus: f.acc,
                mora_count: f.mora_size,
                chain_rule: f.chain_rule.clone(),
                chain_flag: f.chain_flag,
                is_unknown: false,
                is_ignored: false,
            })
            .collect();

        self.assign_and_merge_phonemes(njd_features, &mut mapping, is_non_pause_symbol)?;
        Ok(mapping)
    }

    pub(crate) fn assign_and_merge_phonemes<T: WordPhonemeEntry>(
        &mut self,
        njd_features: &[NjdFeature],
        mapping: &mut Vec<T>,
        is_non_pause_symbol: fn(&str) -> bool,
    ) -> Result<(), HaqumeiError> {
        unsafe {
            let ptr_to_idx = self.prepare_jpcommon_label_internal(njd_features)?;
            let jp = self.jp_common.inner.as_mut();

            for (f_idx, f) in njd_features.iter().enumerate() {
                let is_pause_pron = f.pron == "、" || f.pron == "？" || f.pron == "！";

                if is_pause_pron && !is_non_pause_symbol(&f.string) {
                    mapping[f_idx].phonemes_mut().push("pau".to_string());
                }
            }

            let mut p = (*jp.label).phoneme_head;
            while !p.is_null() {
                let s_ptr = (*p).phoneme;
                if !s_ptr.is_null() {
                    let s = CStr::from_ptr(s_ptr).to_string_lossy();

                    if s != "pau" {
                        let mora = (*p).up;
                        if !mora.is_null() {
                            let word = (*mora).up;
                            if !word.is_null()
                                && let Some(&idx) = ptr_to_idx.get(&(word as usize))
                                && let Some(target) = mapping.get_mut(idx)
                            {
                                target.phonemes_mut().push(s.into_owned());
                            }
                        }
                    }
                }
                p = (*p).next;
            }

            ffi::JPCommon_refresh(jp);
            ffi::NJD_refresh(self.njd.inner.as_mut());

            // 長音によって、先行する Word のモーラとして吸収されるケースがあるため、
            // 前方の Word に結合する。
            //
            // 例:
            // "つまみ出されようとした"
            // - つまみ出さ: [ts u m a m i d a s a]
            // - れよ: [r e y o o]
            // - う: []
            // - と: [t o]
            // - し: [sh I]
            // - た: [t a]
            //
            // 音素が空になった "う" を先行する "れよ" に結合する。
            // このとき、`njd_features` の "う" の pron は長音に置き換えられている。
            let mut write_idx = 0;
            for read_idx in 0..mapping.len() {
                let mut should_merge = false;

                if read_idx > 0 && mapping[read_idx].phonemes().is_empty() {
                    let pron = &njd_features[read_idx].pron;
                    let is_absorbed_long_vowel =
                        !pron.is_empty() && pron.chars().all(|c| c == 'ー');

                    if is_absorbed_long_vowel {
                        let prev_phonemes = mapping[write_idx - 1].phonemes();
                        let prev_is_pause = prev_phonemes.len() == 1 && prev_phonemes[0] == "pau";

                        if !prev_is_pause && !prev_phonemes.is_empty() {
                            should_merge = true;
                        }
                    }
                }

                if should_merge {
                    let (left, right) = mapping.split_at_mut(read_idx);
                    left[write_idx - 1].merge_from(&mut right[0]);
                    continue;
                }

                if write_idx != read_idx {
                    mapping.swap(write_idx, read_idx);
                }
                write_idx += 1;
            }
            mapping.truncate(write_idx);

            Ok(())
        }
    }

    #[inline(always)]
    pub(crate) fn make_phoneme_mapping<T: IntoPhonemeMapItem>(
        &self,
        morphs: Vec<MecabMorph>,
        mapping: Vec<T>,
    ) -> Result<Vec<T::Output>, HaqumeiError> {
        let mut result = Vec::with_capacity(morphs.len());
        let mut morph_idx = 0;
        let mapping_len = mapping.len();

        for (idx, map) in mapping.into_iter().enumerate() {
            // is_ignored な Morph を先に進めておく
            while let Some(m) = morphs.get(morph_idx) {
                if m.is_ignored {
                    result.push(T::new_ignored(m.surface.clone(), m.is_unknown));
                    morph_idx += 1;
                } else {
                    break;
                }
            }

            // morphs が尽きた場合
            if morph_idx >= morphs.len() {
                result.push(map.into_unmatched_remainder());
                continue;
            }

            let morph = &morphs[morph_idx];

            if map.word() == morph.surface {
                // 完全一致
                result.push(map.into_exact_match(morph));
                morph_idx += 1;
            } else if map.word().starts_with(&morph.surface) {
                // 先頭一致
                let mut is_unknown_word = false;
                let mut matched_len = 0;
                let mut internal_ignored = Vec::new();

                while let Some(inner_morph) = morphs.get(morph_idx) {
                    if inner_morph.is_ignored {
                        internal_ignored.push(T::new_ignored(
                            inner_morph.surface.clone(),
                            inner_morph.is_unknown,
                        ));
                        morph_idx += 1;
                        continue;
                    }

                    let remaining = &map.word()[matched_len..];

                    if remaining.starts_with(&inner_morph.surface) {
                        is_unknown_word |= inner_morph.is_unknown;
                        matched_len += inner_morph.surface.len();
                        morph_idx += 1;

                        if matched_len == map.word().len() {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                result.push(map.into_prefix_match(is_unknown_word));
                result.extend(internal_ignored);
            } else {
                // 不一致 (踊り字展開など)
                // map を into_mismatch で消費する前に、借用して文字列を参照する
                if has_odori_chars(&morph.surface) {
                    morph_idx += consume_odori_morphs(&morphs, morph_idx, map.word());
                } else {
                    morph_idx +=
                        consume_mismatched_morphs(&morphs, morph_idx, mapping_len - idx - 1);
                }

                result.push(map.into_mismatch());
            }
        }

        // 余った ignored morphs を回収
        while let Some(m) = morphs.get(morph_idx) {
            if m.is_ignored {
                result.push(T::new_ignored(m.surface.clone(), m.is_unknown));
            }
            morph_idx += 1;
        }

        Ok(result)
    }

    /// 呼び出し後は必ず JPCommon_refresh / NJD_refresh を行わなければならない。
    /// NJDFeature を元に JPCommon の内部構造体 (Word/Mora/Phoneme階層) を構築する。
    /// 戻り値として、JPCommonLabelWord のポインタから、対応する NJDFeature のインデックスへのマッピングを返す。
    unsafe fn prepare_jpcommon_label_internal(
        &mut self,
        features: &[NjdFeature],
    ) -> Result<HashMap<usize, usize>, HaqumeiError> {
        Self::features_to_njd(features, &mut self.njd)?;

        let mut ptr_to_idx = HashMap::with_capacity(features.len());

        unsafe {
            let jp = self.jp_common.inner.as_mut();
            let njd = self.njd.inner.as_mut();

            ffi::njd2jpcommon(jp, njd);

            // JPCommon_make_label(JPCommon * jpcommon) の部分的な移植
            if !jp.label.is_null() {
                ffi::JPCommonLabel_clear(jp.label);
            } else {
                let ptr = libc::calloc(1, std::mem::size_of::<ffi::JPCommonLabel>());
                if ptr.is_null() {
                    return Err(HaqumeiError::AllocationError("ffi::JPCommonLabel"));
                }
                jp.label = ptr as *mut ffi::JPCommonLabel;
            }

            ffi::JPCommonLabel_initialize(jp.label);

            let mut node = jp.head;
            let mut f_idx = 0;

            while !node.is_null() {
                let prev_word_tail = (*jp.label).word_tail;

                ffi::JPCommonLabel_push_word(
                    jp.label,
                    ffi::JPCommonNode_get_pron(node),
                    ffi::JPCommonNode_get_pos(node),
                    ffi::JPCommonNode_get_ctype(node),
                    ffi::JPCommonNode_get_cform(node),
                    ffi::JPCommonNode_get_acc(node),
                    ffi::JPCommonNode_get_chain_flag(node),
                );

                // 追加後の末尾のWordポインタ
                let curr_word_tail = (*jp.label).word_tail;

                // JPCommonLabel_push_word によって新しい Word が生成された場合のみマッピングを記録する。
                // (「ー」などで直前のWordに吸収された場合や、pau で Word が生成されなかった場合はスキップされる)
                if prev_word_tail != curr_word_tail && !curr_word_tail.is_null() {
                    ptr_to_idx.insert(curr_word_tail as usize, f_idx);
                }

                node = (*node).next;
                f_idx += 1;
            }
        }

        Ok(ptr_to_idx)
    }
}
