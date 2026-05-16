#![allow(unused_unsafe)]

use crate::{NjdFeature, OpenJTalk};
use crate::{errors::HaqumeiError, ffi};
use haqumei_jlabel::{
    AccentPhraseCurrent, AccentPhrasePrevNext, BreathGroupCurrent, BreathGroupPrevNext, Label,
    Mora, Phoneme, Utterance, Word,
};
use std::ffi::CStr;
use std::os::raw::c_char;

const MAX_S: i32 = 19;
const MAX_M: i32 = 49;
const MAX_L: i32 = 99;
const MAX_LL: i32 = 199;

#[inline(always)]
fn limit(val: i32, min: i32, max: i32) -> i32 {
    val.clamp(min, max)
}

macro_rules! get_ptr {
    ($ptr:expr, $field:ident) => {
        {
            let p = $ptr;
            if p.is_null() {
                std::ptr::null_mut()
            } else {
                #[allow(unused_unsafe)]
                unsafe { (*p).$field }
            }
        }
    };
    ($ptr:expr, $field:ident $(, $rest:ident)+) => {
        {
            let p = $ptr;
            if p.is_null() {
                std::ptr::null_mut()
            } else {
                get_ptr!(unsafe { (*p).$field } $(, $rest)+)
            }
        }
    };
}

#[inline(always)]
unsafe fn parse_u8(ptr: *const c_char) -> Option<u8> {
    if ptr.is_null() {
        return None;
    }
    let s = unsafe { CStr::from_ptr(ptr) }.to_string_lossy();
    if s == "xx" || s == "*" {
        None
    } else {
        s.parse().ok()
    }
}

#[inline(always)]
unsafe fn parse_bool(ptr: *const c_char) -> bool {
    if ptr.is_null() {
        return false;
    }
    let s = unsafe { CStr::from_ptr(ptr) }.to_string_lossy();
    s != "0" && s != "xx" && s != "*" && !s.is_empty()
}

#[inline(always)]
unsafe fn is_pau(ptr: *mut ffi::JPCommonLabelPhoneme) -> bool {
    if ptr.is_null() {
        return false;
    }
    let s_ptr = unsafe { (*ptr).phoneme };
    if s_ptr.is_null() {
        return false;
    }
    unsafe { CStr::from_ptr(s_ptr) }.to_bytes() == b"pau"
}

#[inline(always)]
unsafe fn get_phoneme_str(ptr: *mut ffi::JPCommonLabelPhoneme) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let s_ptr = unsafe { (*ptr).phoneme };
    if s_ptr.is_null() {
        return None;
    }
    let s = unsafe { CStr::from_ptr(s_ptr) }.to_string_lossy();
    if s == "xx" || s == "*" {
        None
    } else {
        Some(s.into_owned())
    }
}

#[inline(always)]
unsafe fn index_mora_in_accent_phrase(m: *mut ffi::JPCommonLabelMora) -> i32 {
    unsafe {
        let mut i = 0;
        let mut index = get_ptr!(m, up, up, head, head);
        while !index.is_null() {
            i += 1;
            if index == m {
                break;
            }
            index = (*index).next;
        }
        i
    }
}

#[inline(always)]
unsafe fn count_mora_in_accent_phrase(m: *mut ffi::JPCommonLabelMora) -> i32 {
    unsafe {
        let mut i = 0;
        let mut index = get_ptr!(m, up, up, head, head);
        let tail = get_ptr!(m, up, up, tail, tail);
        while !index.is_null() {
            i += 1;
            if index == tail {
                break;
            }
            index = (*index).next;
        }
        i
    }
}

#[inline(always)]
unsafe fn index_accent_phrase_in_breath_group(a: *mut ffi::JPCommonLabelAccentPhrase) -> i32 {
    unsafe {
        let mut i = 0;
        let mut index = get_ptr!(a, up, head);
        while !index.is_null() {
            i += 1;
            if index == a {
                break;
            }
            index = (*index).next;
        }
        i
    }
}

#[inline(always)]
unsafe fn count_accent_phrase_in_breath_group(a: *mut ffi::JPCommonLabelAccentPhrase) -> i32 {
    unsafe {
        let mut i = 0;
        let mut index = get_ptr!(a, up, head);
        let tail = get_ptr!(a, up, tail);
        while !index.is_null() {
            i += 1;
            if index == tail {
                break;
            }
            index = (*index).next;
        }
        i
    }
}

#[inline(always)]
unsafe fn index_mora_in_breath_group(m: *mut ffi::JPCommonLabelMora) -> i32 {
    unsafe {
        let mut i = 0;
        let mut index = get_ptr!(m, up, up, up, head, head, head);
        while !index.is_null() {
            i += 1;
            if index == m {
                break;
            }
            index = (*index).next;
        }
        i
    }
}

#[inline(always)]
unsafe fn count_mora_in_breath_group(m: *mut ffi::JPCommonLabelMora) -> i32 {
    unsafe {
        let mut i = 0;
        let mut index = get_ptr!(m, up, up, up, head, head, head);
        let tail = get_ptr!(m, up, up, up, tail, tail, tail);
        while !index.is_null() {
            i += 1;
            if index == tail {
                break;
            }
            index = (*index).next;
        }
        i
    }
}

#[inline(always)]
unsafe fn index_breath_group_in_utterance(b: *mut ffi::JPCommonLabelBreathGroup) -> i32 {
    unsafe {
        let mut i = 0;
        let mut index = b;
        while !index.is_null() {
            i += 1;
            index = (*index).prev;
        }
        i
    }
}

#[inline(always)]
unsafe fn count_breath_group_in_utterance(b: *mut ffi::JPCommonLabelBreathGroup) -> i32 {
    unsafe {
        if b.is_null() {
            return 0;
        }
        let mut i = 0;
        let mut index = (*b).next;
        while !index.is_null() {
            i += 1;
            index = (*index).next;
        }
        index_breath_group_in_utterance(b) + i
    }
}

#[inline(always)]
unsafe fn index_accent_phrase_in_utterance(a: *mut ffi::JPCommonLabelAccentPhrase) -> i32 {
    unsafe {
        let mut i = 0;
        let mut index = a;
        while !index.is_null() {
            i += 1;
            index = (*index).prev;
        }
        i
    }
}

#[inline(always)]
unsafe fn count_accent_phrase_in_utterance(a: *mut ffi::JPCommonLabelAccentPhrase) -> i32 {
    unsafe {
        if a.is_null() {
            return 0;
        }
        let mut i = 0;
        let mut index = (*a).next;
        while !index.is_null() {
            i += 1;
            index = (*index).next;
        }
        index_accent_phrase_in_utterance(a) + i
    }
}

#[inline(always)]
unsafe fn index_mora_in_utterance(m: *mut ffi::JPCommonLabelMora) -> i32 {
    unsafe {
        let mut i = 0;
        let mut index = m;
        while !index.is_null() {
            i += 1;
            index = (*index).prev;
        }
        i
    }
}

#[inline(always)]
unsafe fn count_mora_in_utterance(m: *mut ffi::JPCommonLabelMora) -> i32 {
    unsafe {
        if m.is_null() {
            return 0;
        }
        let mut i = 0;
        let mut index = (*m).next;
        while !index.is_null() {
            i += 1;
            index = (*index).next;
        }
        index_mora_in_utterance(m) + i
    }
}

impl OpenJTalk {
    #[inline(always)]
    pub(crate) fn extract_fullcontext_labels(
        &mut self,
        njd_features: &[NjdFeature],
    ) -> Result<Vec<Label>, HaqumeiError> {
        if njd_features.is_empty() {
            return Ok(Vec::new());
        }

        unsafe {
            Self::features_to_njd(njd_features, &mut self.njd)?;

            let jp = self.jp_common.inner.as_mut();
            let njd = self.njd.inner.as_mut();
            ffi::njd2jpcommon(jp, njd);

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
            while !node.is_null() {
                ffi::JPCommonLabel_push_word(
                    jp.label,
                    ffi::JPCommonNode_get_pron(node),
                    ffi::JPCommonNode_get_pos(node),
                    ffi::JPCommonNode_get_ctype(node),
                    ffi::JPCommonNode_get_cform(node),
                    ffi::JPCommonNode_get_acc(node),
                    ffi::JPCommonNode_get_chain_flag(node),
                );
                node = (*node).next;
            }

            let mut phonemes = Vec::new();
            let mut p_iter = (*jp.label).phoneme_head;
            while !p_iter.is_null() {
                phonemes.push(p_iter);
                p_iter = (*p_iter).next;
            }

            let size = phonemes.len() as isize;

            if size == 0 {
                ffi::JPCommon_refresh(jp);
                ffi::NJD_refresh(self.njd.inner.as_mut());
                return Ok(Vec::new());
            }

            let mut labels = Vec::with_capacity((size + 2) as usize);

            let get_ph = |idx: isize| -> Option<String> {
                if idx < -1 || idx > size {
                    None
                } else if idx == -1 || idx == size {
                    Some("sil".to_string())
                } else {
                    get_phoneme_str(phonemes[idx as usize])
                }
            };

            let utterance = Utterance {
                breath_group_count: limit(
                    count_breath_group_in_utterance((*jp.label).breath_head),
                    1,
                    MAX_S,
                ) as u8,
                accent_phrase_count: limit(
                    count_accent_phrase_in_utterance((*jp.label).accent_head),
                    1,
                    MAX_M,
                ) as u8,
                mora_count: limit(count_mora_in_utterance((*jp.label).mora_head), 1, MAX_LL) as u8,
            };

            // idx = -1 は先頭 sil、idx = size は末尾 sil
            for idx in -1..=size {
                let p_curr = if idx == -1 {
                    phonemes[0]
                } else if idx == size {
                    phonemes[(size - 1) as usize]
                } else {
                    phonemes[idx as usize]
                };

                let is_sil = idx == -1 || idx == size;
                let short_pause_flag = if is_sil { false } else { is_pau(p_curr) };

                let phoneme = Phoneme {
                    p2: get_ph(idx - 2),
                    p1: get_ph(idx - 1),
                    c: get_ph(idx),
                    n1: get_ph(idx + 1),
                    n2: get_ph(idx + 2),
                };

                let mora = if is_sil || short_pause_flag {
                    None
                } else {
                    let m = get_ptr!(p_curr, up);
                    let a = get_ptr!(m, up, up);
                    if m.is_null() || a.is_null() {
                        None
                    } else {
                        let tmp1 = index_mora_in_accent_phrase(m);
                        let m_cnt = count_mora_in_accent_phrase(m);
                        let tmp2 = if (*a).accent == 0 { m_cnt } else { (*a).accent };
                        Some(Mora {
                            relative_accent_position: limit(tmp1 - tmp2, -MAX_M, MAX_M) as i8,
                            position_forward: limit(tmp1, 1, MAX_M) as u8,
                            position_backward: limit(m_cnt - tmp1 + 1, 1, MAX_M) as u8,
                        })
                    }
                };

                let build_word = |w: *mut ffi::JPCommonLabelWord| -> Option<Word> {
                    if w.is_null() {
                        return None;
                    }
                    Some(Word {
                        pos: parse_u8((*w).pos),
                        ctype: parse_u8((*w).ctype),
                        cform: parse_u8((*w).cform),
                    })
                };

                let w_prev = if short_pause_flag {
                    get_ptr!(p_curr, prev, up, up)
                } else if get_ptr!(p_curr, up, up, prev).is_null() {
                    std::ptr::null_mut()
                } else if idx == size {
                    get_ptr!(p_curr, up, up)
                } else {
                    get_ptr!(p_curr, up, up, prev)
                };
                let word_prev = build_word(w_prev);

                let w_curr = if is_sil || short_pause_flag {
                    std::ptr::null_mut()
                } else {
                    get_ptr!(p_curr, up, up)
                };
                let word_curr = build_word(w_curr);

                let w_next = if short_pause_flag {
                    get_ptr!(p_curr, next, up, up)
                } else if get_ptr!(p_curr, up, up, next).is_null() {
                    std::ptr::null_mut()
                } else if idx == -1 {
                    get_ptr!(p_curr, up, up)
                } else {
                    get_ptr!(p_curr, up, up, next)
                };
                let word_next = build_word(w_next);

                let build_ap_prevnext = |ap: *mut ffi::JPCommonLabelAccentPhrase,
                                         is_prev: bool|
                 -> Option<AccentPhrasePrevNext> {
                    if ap.is_null() {
                        return None;
                    }
                    let m_head = get_ptr!(ap, head, head);
                    if m_head.is_null() {
                        return None;
                    }

                    let m_cnt = count_mora_in_accent_phrase(m_head);
                    let acc = if (*ap).accent == 0 {
                        m_cnt
                    } else {
                        (*ap).accent
                    };

                    let is_pause_insertion = if is_sil || short_pause_flag {
                        None
                    } else {
                        let has_pau = if is_prev {
                            let t_ph = get_ptr!(ap, tail, tail, tail, next);
                            is_pau(t_ph)
                        } else {
                            let h_ph = get_ptr!(ap, head, head, head, prev);
                            is_pau(h_ph)
                        };
                        Some(has_pau)
                    };

                    Some(AccentPhrasePrevNext {
                        mora_count: limit(m_cnt, 1, MAX_M) as u8,
                        accent_position: limit(acc, 1, MAX_M) as u8,
                        is_interrogative: parse_bool((*ap).emotion),
                        is_exclamatory: unsafe { parse_bool((*ap).excl) },
                        is_pause_insertion,
                    })
                };

                let a_prev = if short_pause_flag {
                    get_ptr!(p_curr, prev, up, up, up)
                } else if idx == size {
                    get_ptr!(p_curr, up, up, up)
                } else {
                    get_ptr!(p_curr, up, up, up, prev)
                };
                let accent_phrase_prev = build_ap_prevnext(a_prev, true);

                let a_curr = if is_sil || short_pause_flag {
                    std::ptr::null_mut()
                } else {
                    get_ptr!(p_curr, up, up, up)
                };
                let accent_phrase_curr = if a_curr.is_null() {
                    None
                } else {
                    let m_head = get_ptr!(a_curr, head, head);
                    if m_head.is_null() {
                        None
                    } else {
                        let m_cnt = count_mora_in_accent_phrase(m_head);
                        let acc = if (*a_curr).accent == 0 {
                            m_cnt
                        } else {
                            (*a_curr).accent
                        };
                        let tmp1 = index_accent_phrase_in_breath_group(a_curr);
                        let tmp2 = index_mora_in_breath_group(m_head);

                        Some(AccentPhraseCurrent {
                            mora_count: limit(m_cnt, 1, MAX_M) as u8,
                            accent_position: limit(acc, 1, MAX_M) as u8,
                            is_interrogative: parse_bool((*a_curr).emotion),
                            is_exclamatory: unsafe { parse_bool((*a_curr).excl) },
                            accent_phrase_position_forward: limit(tmp1, 1, MAX_M) as u8,
                            accent_phrase_position_backward: limit(
                                count_accent_phrase_in_breath_group(a_curr) - tmp1 + 1,
                                1,
                                MAX_M,
                            ) as u8,
                            mora_position_forward: limit(tmp2, 1, MAX_L) as u8,
                            mora_position_backward: limit(
                                count_mora_in_breath_group(m_head) - tmp2 + 1,
                                1,
                                MAX_L,
                            ) as u8,
                        })
                    }
                };

                let a_next = if short_pause_flag {
                    get_ptr!(p_curr, next, up, up, up)
                } else if idx == -1 {
                    get_ptr!(p_curr, up, up, up)
                } else {
                    get_ptr!(p_curr, up, up, up, next)
                };
                let accent_phrase_next = build_ap_prevnext(a_next, false);

                let build_bg_prevnext =
                    |bg: *mut ffi::JPCommonLabelBreathGroup| -> Option<BreathGroupPrevNext> {
                        if bg.is_null() {
                            return None;
                        }
                        Some(BreathGroupPrevNext {
                            accent_phrase_count: limit(
                                count_accent_phrase_in_breath_group(get_ptr!(bg, head)),
                                1,
                                MAX_M,
                            ) as u8,
                            mora_count: limit(
                                count_mora_in_breath_group(get_ptr!(bg, head, head, head)),
                                1,
                                MAX_L,
                            ) as u8,
                        })
                    };

                let b_prev = if short_pause_flag {
                    get_ptr!(p_curr, prev, up, up, up, up)
                } else if idx == size {
                    get_ptr!(p_curr, up, up, up, up)
                } else {
                    get_ptr!(p_curr, up, up, up, up, prev)
                };
                let breath_group_prev = build_bg_prevnext(b_prev);

                let b_curr = if is_sil || short_pause_flag {
                    std::ptr::null_mut()
                } else {
                    get_ptr!(p_curr, up, up, up, up)
                };
                let breath_group_curr = if b_curr.is_null() {
                    None
                } else {
                    let tmp1 = index_breath_group_in_utterance(b_curr);
                    let tmp2 = index_accent_phrase_in_utterance(get_ptr!(b_curr, head));
                    let tmp3 = index_mora_in_utterance(get_ptr!(b_curr, head, head, head));

                    Some(BreathGroupCurrent {
                        accent_phrase_count: limit(
                            count_accent_phrase_in_breath_group(get_ptr!(b_curr, head)),
                            1,
                            MAX_M,
                        ) as u8,
                        mora_count: limit(
                            count_mora_in_breath_group(get_ptr!(b_curr, head, head, head)),
                            1,
                            MAX_L,
                        ) as u8,
                        breath_group_position_forward: limit(tmp1, 1, MAX_S) as u8,
                        breath_group_position_backward: limit(
                            count_breath_group_in_utterance(b_curr) - tmp1 + 1,
                            1,
                            MAX_S,
                        ) as u8,
                        accent_phrase_position_forward: limit(tmp2, 1, MAX_M) as u8,
                        accent_phrase_position_backward: limit(
                            count_accent_phrase_in_utterance(get_ptr!(b_curr, head)) - tmp2 + 1,
                            1,
                            MAX_M,
                        ) as u8,
                        mora_position_forward: limit(tmp3, 1, MAX_LL) as u8,
                        mora_position_backward: limit(
                            count_mora_in_utterance(get_ptr!(b_curr, head, head, head)) - tmp3 + 1,
                            1,
                            MAX_LL,
                        ) as u8,
                    })
                };

                let b_next = if short_pause_flag {
                    get_ptr!(p_curr, next, up, up, up, up)
                } else if idx == -1 {
                    get_ptr!(p_curr, up, up, up, up)
                } else {
                    get_ptr!(p_curr, up, up, up, up, next)
                };
                let breath_group_next = build_bg_prevnext(b_next);

                labels.push(Label {
                    phoneme,
                    mora,
                    word_prev,
                    word_curr,
                    word_next,
                    accent_phrase_prev,
                    accent_phrase_curr,
                    accent_phrase_next,
                    breath_group_prev,
                    breath_group_curr,
                    breath_group_next,
                    utterance: utterance.clone(),
                });
            }

            ffi::JPCommon_refresh(jp);
            ffi::NJD_refresh(self.njd.inner.as_mut());

            Ok(labels)
        }
    }
}
