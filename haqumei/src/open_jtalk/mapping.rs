use super::mapping_utills::{
    adjust_unknown_phonemes, consume_mismatched_morphs, consume_odori_morphs,
};
use crate::errors::HaqumeiError;
use crate::ffi;
use crate::utils::has_odori_chars;
use crate::word_phoneme::WordPhonemeEntry;
use crate::{MecabMorph, OpenJTalk};
use crate::{NjdFeature, WordPhonemeDetail, WordPhonemeMap, WordPhonemePair};

use std::collections::HashMap;
use std::ffi::CStr;

impl OpenJTalk {
    pub(crate) fn assign_and_merge_phonemes<T: WordPhonemeEntry>(
        &mut self,
        njd_features: &[NjdFeature],
        mapping: &mut Vec<T>,
    ) -> Result<(), HaqumeiError> {
        unsafe {
            let ptr_to_idx = self.prepare_jpcommon_label_internal(njd_features)?;
            let jp = self.jp_common.inner.as_mut();

            let mut pause_count = 0;
            for (f_idx, f) in njd_features.iter().enumerate() {
                let is_pause_pron = f.pron == "、" || f.pron == "？" || f.pron == "！";
                if is_pause_pron {
                    mapping[f_idx].phonemes_mut().push("pau".to_string());
                    pause_count += 1;
                }
            }

            let needs_merge = njd_features.len() > ptr_to_idx.len() + pause_count;

            let mut p = (*jp.label).phoneme_head;
            while !p.is_null() {
                let s_ptr = (*p).phoneme;
                if !s_ptr.is_null() {
                    let s = CStr::from_ptr(s_ptr).to_string_lossy().into_owned();

                    if s == "pau" {
                        p = (*p).next;
                        continue;
                    }

                    let mut current_word_ptr = 0usize;
                    let mora = (*p).up;
                    if !mora.is_null() {
                        let word = (*mora).up;
                        if !word.is_null() {
                            current_word_ptr = word as usize;
                        }
                    }

                    if current_word_ptr != 0
                        && let Some(&idx) = ptr_to_idx.get(&current_word_ptr)
                        && let Some(target) = mapping.get_mut(idx)
                    {
                        target.phonemes_mut().push(s);
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
            if needs_merge {
                let mut write_idx = 0;
                for read_idx in 0..mapping.len() {
                    let mut should_merge = false;

                    if read_idx > 0 && mapping[read_idx].phonemes().is_empty() {
                        let prev_phonemes = mapping[write_idx - 1].phonemes();
                        let prev_is_pause = prev_phonemes.len() == 1 && prev_phonemes[0] == "pau";

                        if !prev_is_pause {
                            should_merge = true;
                        }
                    }

                    if should_merge {
                        let (left, right) = mapping.split_at_mut(read_idx);
                        let prev = &mut left[write_idx - 1];
                        let current = &mut right[0];

                        prev.merge_from(current);
                        continue;
                    }

                    if write_idx != read_idx {
                        mapping.swap(write_idx, read_idx);
                    }
                    write_idx += 1;
                }
                mapping.truncate(write_idx);
            }

            Ok(())
        }
    }

    pub(crate) fn make_phoneme_mapping(
        &mut self,
        morphs: Vec<MecabMorph>,
        mapping: Vec<WordPhonemePair>,
    ) -> Result<Vec<WordPhonemeMap>, HaqumeiError> {
        let mut result = Vec::with_capacity(morphs.len());
        let mut morph_idx = 0;
        let mapping_len = mapping.len();

        for (idx, map) in mapping.into_iter().enumerate() {
            // is_ignored な Morph を先に進めておく
            while let Some(m) = morphs.get(morph_idx) {
                if m.is_ignored {
                    result.push(WordPhonemeMap {
                        word: m.surface.clone(),
                        phonemes: vec!["sp".to_string()],
                        is_unknown: m.is_unknown,
                        is_ignored: true,
                    });
                    morph_idx += 1;
                } else {
                    break;
                }
            }

            // morphs が尽きた場合: 後処理で feature 数が変動しうるため出力を継続
            if morph_idx >= morphs.len() {
                let is_ignored = map.phonemes.is_empty();
                result.push(WordPhonemeMap {
                    word: map.word,
                    phonemes: map.phonemes,
                    is_unknown: false,
                    is_ignored,
                });
                continue;
            }

            let morph = &morphs[morph_idx];

            if map.word == morph.surface {
                // 完全一致: morph と NJD feature の surface が一致
                let mut phonemes = map.phonemes.clone();

                if morph.is_unknown {
                    // 先頭の長音のような Open JTalk が破棄するもの、
                    // または pau に置き換えられた未知語は unk にしておく
                    if phonemes.is_empty() || phonemes == ["pau"] {
                        phonemes = vec!["unk".to_string()];
                    }
                }

                result.push(WordPhonemeMap {
                    word: map.word,
                    phonemes,
                    is_unknown: morph.is_unknown,
                    is_ignored: map.phonemes.is_empty(),
                });
                morph_idx += 1;
            } else if map.word.starts_with(&morph.surface) {
                // 先頭一致: NJD が複数の morph を結合したケース
                let mut is_unknown_word = false;
                let mut matched_len = 0;

                while let Some(inner_morph) = morphs.get(morph_idx) {
                    if inner_morph.is_ignored {
                        result.push(WordPhonemeMap {
                            word: inner_morph.surface.clone(),
                            phonemes: vec!["sp".to_string()],
                            is_unknown: inner_morph.is_unknown,
                            is_ignored: true,
                        });
                        morph_idx += 1;
                        continue;
                    }

                    let remaining = &map.word[matched_len..];

                    if remaining.starts_with(&inner_morph.surface) {
                        is_unknown_word |= inner_morph.is_unknown;
                        matched_len += inner_morph.surface.len();
                        morph_idx += 1;

                        if matched_len == map.word.len() {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                let mut phonemes = map.phonemes.clone();
                let is_ignored = phonemes.is_empty();
                adjust_unknown_phonemes(&mut phonemes, is_unknown_word);

                result.push(WordPhonemeMap {
                    word: map.word,
                    phonemes,
                    is_unknown: is_unknown_word,
                    is_ignored,
                });
            } else {
                // 不一致: 数字正規化・踊り字展開等で surface が変化したケース
                result.push(WordPhonemeMap {
                    word: map.word.clone(),
                    phonemes: map.phonemes.clone(),
                    is_unknown: false,
                    is_ignored: map.phonemes.is_empty(),
                });

                if has_odori_chars(&morph.surface) {
                    morph_idx += consume_odori_morphs(&morphs, morph_idx, &map.word);
                } else {
                    morph_idx +=
                        consume_mismatched_morphs(&morphs, morph_idx, mapping_len - idx - 1);
                }
            }
        }

        // mapping 終了後、morphs の末尾に無視トークン(空白等)が残っていれば回収する
        while let Some(m) = morphs.get(morph_idx) {
            if m.is_ignored {
                result.push(WordPhonemeMap {
                    word: m.surface.clone(),
                    phonemes: vec!["sp".to_string()],
                    is_unknown: m.is_unknown,
                    is_ignored: true,
                });
            }
            morph_idx += 1;
        }

        Ok(result)
    }

    pub(crate) fn make_phoneme_mapping_detailed(
        &mut self,
        morphs: Vec<MecabMorph>,
        mapping: Vec<WordPhonemeDetail>,
    ) -> Result<Vec<WordPhonemeDetail>, HaqumeiError> {
        let mut result = Vec::with_capacity(morphs.len());
        let mut morph_idx = 0;
        let mapping_len = mapping.len();

        // 無視トークン(sp)用のダミー値エントリ生成クロージャ
        let sp_entry = |surface: String, is_unknown: bool| WordPhonemeDetail {
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
        };

        for (base_idx, mut map) in mapping.into_iter().enumerate() {
            while let Some(m) = morphs.get(morph_idx) {
                if m.is_ignored {
                    result.push(sp_entry(m.surface.clone(), m.is_unknown));
                    morph_idx += 1;
                } else {
                    break;
                }
            }

            if morph_idx >= morphs.len() {
                map.is_ignored = map.phonemes.is_empty();
                result.push(map);
                continue;
            }

            let current_map_word = map.word.clone();
            let morph = &morphs[morph_idx];

            if current_map_word == morph.surface {
                // 完全一致
                let mut phonemes = map.phonemes.clone();
                if morph.is_unknown && (phonemes.is_empty() || phonemes == ["pau"]) {
                    phonemes = vec!["unk".to_string()];
                }

                map.phonemes = phonemes;
                map.is_unknown = morph.is_unknown;
                map.is_ignored = map.phonemes.is_empty();

                map.features = morph.feature.split(',').map(|s| s.to_string()).collect();

                result.push(map);
                morph_idx += 1;
            } else if current_map_word.starts_with(&morph.surface) {
                // 先頭一致
                let mut is_unknown_word = false;
                let mut matched_len = 0;

                while let Some(inner_morph) = morphs.get(morph_idx) {
                    if inner_morph.is_ignored {
                        result.push(sp_entry(
                            inner_morph.surface.clone(),
                            inner_morph.is_unknown,
                        ));
                        morph_idx += 1;
                        continue;
                    }

                    let remaining = &current_map_word[matched_len..];

                    if remaining.starts_with(&inner_morph.surface) {
                        is_unknown_word |= inner_morph.is_unknown;
                        matched_len += inner_morph.surface.len();
                        morph_idx += 1;

                        if matched_len == current_map_word.len() {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                let mut phonemes = map.phonemes.clone();
                if is_unknown_word && (phonemes.is_empty() || phonemes == ["pau"]) {
                    phonemes = vec!["unk".to_string()];
                }

                map.phonemes = phonemes;
                map.is_unknown = is_unknown_word;
                map.is_ignored = map.phonemes.is_empty();
                // 複数結合された場合は features を空にする
                map.features = Vec::new();

                result.push(map);
            } else {
                // 不一致 (踊り字展開など)
                map.is_unknown = false;
                map.is_ignored = map.phonemes.is_empty();
                map.features = Vec::new();
                result.push(map);

                if has_odori_chars(&morph.surface) {
                    morph_idx += consume_odori_morphs(&morphs, morph_idx, &current_map_word);
                } else {
                    morph_idx +=
                        consume_mismatched_morphs(&morphs, morph_idx, mapping_len - base_idx - 1);
                }
            }
        }

        while let Some(m) = morphs.get(morph_idx) {
            if m.is_ignored {
                result.push(sp_entry(m.surface.clone(), m.is_unknown));
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
