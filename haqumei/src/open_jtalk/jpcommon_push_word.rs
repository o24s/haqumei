#![allow(unused_unsafe)]
#![allow(clippy::too_many_arguments)]

use super::jpcommon::FreeNode;
use super::jpcommon_rule::*;
use crate::errors::JpCommonLabelError;
use crate::ffi;
use crate::utils::ptr_to_str_unchecked;

use daachorse::{DoubleArrayAhoCorasick, DoubleArrayAhoCorasickBuilder, MatchKind};
use std::os::raw::c_char;
use std::sync::OnceLock;

#[derive(Clone, Copy)]
enum MoraToken {
    LongVowel,
    Unvoice,
    Normal(&'static str, Option<&'static str>), // (ph1, ph2)
}

/// メモリ確保を行いつつ、エラー時にはロールバックとフラグ更新を行う。
/// - 要素が1つの場合は単体の Result<Ptr, Error> を返します。
/// - 要素が複数の場合は Result<(Ptr, Ptr, ...), Error> のタプルを返します。
macro_rules! try_alloc {
    ( $label:expr, $err_msg:expr, $name:ident = $alloc_expr:expr $(,)? ) => {
        match $alloc_expr {
            Ok(ptr) => Ok(ptr),
            Err(_) => {
                unsafe { (*$label).is_valid = 0; }
                Err(JpCommonLabelError::AllocationError($err_msg))
            }
        }
    };
    ( $label:expr, $err_msg:expr, $( $name:ident = $alloc_expr:expr ),+ $(,)? ) => {
        {
            $( let $name = $alloc_expr; )+

            let success = $( $name.is_ok() )&&+;

            if success {
                Ok(( $( $name.unwrap() ),+ ))
            } else {
                $(
                    if let Ok(ptr) = $name {
                        unsafe { ptr.free_node(); }
                    }
                )+
                unsafe { (*$label).is_valid = 0; }
                Err(JpCommonLabelError::AllocationError($err_msg))
            }
        }
    };
}

#[inline(always)]
fn get_mora_automaton() -> &'static DoubleArrayAhoCorasick<MoraToken> {
    static AUTOMATON: OnceLock<DoubleArrayAhoCorasick<MoraToken>> = OnceLock::new();

    AUTOMATON.get_or_init(|| {
        let mut patvals = Vec::new();

        patvals.push((JPCOMMON_MORA_LONG_VOWEL, MoraToken::LongVowel));

        patvals.push((JPCOMMON_MORA_UNVOICE, MoraToken::Unvoice));

        for &(mora, ph1, ph2) in JPCOMMON_MORA_LIST {
            patvals.push((mora, MoraToken::Normal(ph1, ph2)));
        }

        DoubleArrayAhoCorasickBuilder::new()
            .match_kind(MatchKind::LeftmostLongest)
            .build_with_values(patvals)
            .expect("Failed to build Aho-Corasick automaton")
    })
}

#[inline(always)]
unsafe fn duplicate_str_or_nodata(s: Option<&str>) -> *mut c_char {
    let bytes = match s {
        Some(s) => s.as_bytes(),
        None => b"*",
    };

    let len = bytes.len();
    unsafe {
        let ptr = libc::malloc(len + 1) as *mut u8;
        if ptr.is_null() {
            return NODATA.as_ptr() as *mut c_char;
        }

        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, len);
        *ptr.add(len) = 0;
        ptr as *mut c_char
    }
}

#[inline(always)]
fn get_unvoiced_phoneme(p: &str) -> Option<&'static str> {
    match p {
        "a" => Some("A"),
        "i" => Some("I"),
        "u" => Some("U"),
        "e" => Some("E"),
        "o" => Some("O"),
        _ => None,
    }
}

#[inline(always)]
unsafe fn alloc_phoneme(
    phoneme: &str,
    prev: *mut ffi::JPCommonLabelPhoneme,
    next: *mut ffi::JPCommonLabelPhoneme,
    up: *mut ffi::JPCommonLabelMora,
) -> Result<*mut ffi::JPCommonLabelPhoneme, ()> {
    unsafe {
        let p = libc::calloc(1, std::mem::size_of::<ffi::JPCommonLabelPhoneme>())
            as *mut ffi::JPCommonLabelPhoneme;
        if p.is_null() {
            return Err(());
        }
        (*p).phoneme = duplicate_str_or_nodata(Some(phoneme));
        (*p).prev = prev;
        (*p).next = next;
        (*p).up = up;
        Ok(p)
    }
}

#[inline(always)]
unsafe fn alloc_mora(
    mora: &str,
    head: *mut ffi::JPCommonLabelPhoneme,
    tail: *mut ffi::JPCommonLabelPhoneme,
    prev: *mut ffi::JPCommonLabelMora,
    next: *mut ffi::JPCommonLabelMora,
    up: *mut ffi::JPCommonLabelWord,
) -> Result<*mut ffi::JPCommonLabelMora, ()> {
    unsafe {
        let m = libc::calloc(1, std::mem::size_of::<ffi::JPCommonLabelMora>())
            as *mut ffi::JPCommonLabelMora;
        if m.is_null() {
            return Err(());
        }
        (*m).mora = duplicate_str_or_nodata(Some(mora));
        (*m).head = head;
        (*m).tail = tail;
        (*m).prev = prev;
        (*m).next = next;
        (*m).up = up;
        Ok(m)
    }
}

#[inline(always)]
unsafe fn alloc_word(
    pron: &str,
    pos: &str,
    ctype: &str,
    cform: &str,
    head: *mut ffi::JPCommonLabelMora,
    tail: *mut ffi::JPCommonLabelMora,
    prev: *mut ffi::JPCommonLabelWord,
    next: *mut ffi::JPCommonLabelWord,
) -> Result<*mut ffi::JPCommonLabelWord, ()> {
    unsafe {
        let w = libc::calloc(1, std::mem::size_of::<ffi::JPCommonLabelWord>())
            as *mut ffi::JPCommonLabelWord;
        if w.is_null() {
            return Err(());
        }
        (*w).pron = duplicate_str_or_nodata(Some(pron));
        (*w).pos = duplicate_str_or_nodata(Some(get_pos_id(pos)));
        (*w).ctype = duplicate_str_or_nodata(Some(get_ctype_id(ctype)));
        (*w).cform = duplicate_str_or_nodata(Some(get_cform_id(cform)));
        (*w).head = head;
        (*w).tail = tail;
        (*w).prev = prev;
        (*w).next = next;
        Ok(w)
    }
}

#[inline(always)]
unsafe fn alloc_accent_phrase(
    accent: i32,
    emotion: Option<&str>,
    excl: Option<&str>,
    head: *mut ffi::JPCommonLabelWord,
    tail: *mut ffi::JPCommonLabelWord,
    prev: *mut ffi::JPCommonLabelAccentPhrase,
    next: *mut ffi::JPCommonLabelAccentPhrase,
    up: *mut ffi::JPCommonLabelBreathGroup,
) -> Result<*mut ffi::JPCommonLabelAccentPhrase, ()> {
    unsafe {
        let a = libc::calloc(1, std::mem::size_of::<ffi::JPCommonLabelAccentPhrase>())
            as *mut ffi::JPCommonLabelAccentPhrase;
        if a.is_null() {
            return Err(());
        }
        (*a).accent = accent;
        (*a).emotion = if let Some(e) = emotion {
            duplicate_str_or_nodata(Some(e))
        } else {
            std::ptr::null_mut()
        };
        (*a).excl = if let Some(e) = excl {
            duplicate_str_or_nodata(Some(e))
        } else {
            std::ptr::null_mut()
        };
        (*a).head = head;
        (*a).tail = tail;
        (*a).prev = prev;
        (*a).next = next;
        (*a).up = up;
        Ok(a)
    }
}

#[inline(always)]
unsafe fn alloc_breath_group(
    head: *mut ffi::JPCommonLabelAccentPhrase,
    tail: *mut ffi::JPCommonLabelAccentPhrase,
    prev: *mut ffi::JPCommonLabelBreathGroup,
    next: *mut ffi::JPCommonLabelBreathGroup,
) -> Result<*mut ffi::JPCommonLabelBreathGroup, ()> {
    unsafe {
        let b = libc::calloc(1, std::mem::size_of::<ffi::JPCommonLabelBreathGroup>())
            as *mut ffi::JPCommonLabelBreathGroup;
        if b.is_null() {
            return Err(());
        }
        (*b).head = head;
        (*b).tail = tail;
        (*b).prev = prev;
        (*b).next = next;
        Ok(b)
    }
}

#[inline(always)]
unsafe fn insert_short_pause(label: *mut ffi::JPCommonLabel) -> Result<(), JpCommonLabelError> {
    unsafe {
        if (*label).short_pause_flag == 1 {
            if !(*label).phoneme_tail.is_null() {
                let tail_ph = ptr_to_str_unchecked((*(*label).phoneme_tail).phoneme);
                if tail_ph != JPCOMMON_PHONEME_SHORT_PAUSE {
                    let p_next = try_alloc!(
                        label,
                        "phoneme",
                        p_res = alloc_phoneme(
                            JPCOMMON_PHONEME_SHORT_PAUSE,
                            (*label).phoneme_tail,
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                        )
                    )?;
                    (*(*label).phoneme_tail).next = p_next;
                    (*label).phoneme_tail = p_next;
                } else {
                    log::warn!("JPCommonLabel_insert_pause(): Short pause should not be chained.");
                }
            } else {
                log::warn!("JPCommonLabel_insert_pause(): First mora should not be short pause.");
            }
            (*label).short_pause_flag = 0;
        }
        Ok(())
    }
}

/// ワードの情報を構造化された JPCommonLabel へ格納する。
#[allow(non_snake_case)]
pub(crate) unsafe fn JPCommonLabel_push_word(
    label: *mut ffi::JPCommonLabel,
    pron_ptr: *const c_char,
    pos_ptr: *const c_char,
    ctype_ptr: *const c_char,
    cform_ptr: *const c_char,
    acc: i32,
    chain_flag: i32,
) -> Result<(), JpCommonLabelError> {
    unsafe {
        if (*label).is_valid == 0 {
            return Err(JpCommonLabelError::AlreadyInvalid);
        }

        let pron = ptr_to_str_unchecked(pron_ptr);
        let pos = ptr_to_str_unchecked(pos_ptr);
        let ctype = ptr_to_str_unchecked(ctype_ptr);
        let cform = ptr_to_str_unchecked(cform_ptr);
        let mut is_first_word = true;

        if pron == JPCOMMON_MORA_SHORT_PAUSE {
            (*label).short_pause_flag = 1;
            return Ok(());
        }

        if pron == JPCOMMON_MORA_QUESTION || pron == JPCOMMON_MORA_EXCLAMATION {
            let flag = if pron == JPCOMMON_MORA_QUESTION {
                JPCOMMON_FLAG_QUESTION
            } else {
                JPCOMMON_FLAG_EXCLAMATION
            };

            if !(*label).phoneme_tail.is_null() {
                let tail_ph = ptr_to_str_unchecked((*(*label).phoneme_tail).phoneme);
                let ap = if tail_ph == JPCOMMON_PHONEME_SHORT_PAUSE {
                    get_ptr!((*label).phoneme_tail, prev, up, up, up)
                } else {
                    get_ptr!((*label).phoneme_tail, up, up, up)
                };

                if !ap.is_null() {
                    if pron == JPCOMMON_MORA_QUESTION && (*ap).emotion.is_null() {
                        (*ap).emotion = duplicate_str_or_nodata(Some(flag));
                    } else if pron == JPCOMMON_MORA_EXCLAMATION && (*ap).excl.is_null() {
                        (*ap).excl = duplicate_str_or_nodata(Some(flag));
                    }
                }
            } else {
                log::warn!(
                    "JPCommonLabel_push_word(): First mora should not be {} flag.",
                    if pron == JPCOMMON_MORA_QUESTION {
                        "question"
                    } else {
                        "exclamation"
                    }
                );
            }
            (*label).short_pause_flag = 1;
            return Ok(());
        }

        let pma = get_mora_automaton();
        let mut current_offset = 0;

        for m in pma.leftmost_find_iter(pron) {
            if m.start() != current_offset {
                log::warn!(
                    "JPCommonLabel_push_word(): {} is wrong mora list.",
                    &pron[current_offset..]
                );
                break;
            }

            match m.value() {
                MoraToken::LongVowel => {
                    if !(*label).phoneme_tail.is_null() && (*label).short_pause_flag == 0 {
                        insert_short_pause(label)?;

                        let prev_ph = ptr_to_str_unchecked((*(*label).phoneme_tail).phoneme);

                        let (p_next, m_next) = try_alloc!(
                            label,
                            "long-vowel nodes",
                            p_res = alloc_phoneme(
                                prev_ph,
                                (*label).phoneme_tail,
                                std::ptr::null_mut(),
                                std::ptr::null_mut()
                            ),
                            m_res = alloc_mora(
                                JPCOMMON_MORA_LONG_VOWEL,
                                std::ptr::null_mut(),
                                std::ptr::null_mut(),
                                (*label).mora_tail,
                                std::ptr::null_mut(),
                                (*(*label).mora_tail).up
                            )
                        )?;

                        (*m_next).head = p_next;
                        (*m_next).tail = p_next;

                        (*p_next).up = m_next;
                        (*(*label).phoneme_tail).next = p_next;
                        (*(*label).mora_tail).next = m_next;

                        (*label).phoneme_tail = p_next;
                        (*label).mora_tail = m_next;
                        (*(*label).word_tail).tail = m_next;
                    } else {
                        log::warn!(
                            "JPCommonLabel_push_word(): First mora should not be long vowel symbol."
                        );
                    }
                }
                MoraToken::Unvoice => {
                    if !(*label).phoneme_tail.is_null() && !is_first_word {
                        let tail_ph_str = ptr_to_str_unchecked((*(*label).phoneme_tail).phoneme);
                        if let Some(unvoiced) = get_unvoiced_phoneme(tail_ph_str) {
                            let ptr = (*(*label).phoneme_tail).phoneme as *mut u8;
                            if !std::ptr::eq(ptr, NODATA.as_ptr()) {
                                *ptr = unvoiced.as_bytes()[0];
                            } else {
                                (*(*label).phoneme_tail).phoneme =
                                    duplicate_str_or_nodata(Some(unvoiced));
                            }
                        } else {
                            log::warn!(
                                "JPCommonLabelPhoneme_convert_unvoice(): {} cannot be unvoiced.",
                                tail_ph_str
                            );
                        }
                    } else {
                        log::warn!(
                            "JPCommonLabel_push_word(): First mora should not be unvoice flag."
                        );
                    }
                }
                MoraToken::Normal(ph1, ph2_opt) => {
                    let mora_str = &pron[m.start()..m.end()];

                    if (*label).phoneme_tail.is_null() {
                        insert_short_pause(label)?;

                        let (p, m_node, w) = try_alloc!(
                            label,
                            "initial word nodes",
                            p_res = alloc_phoneme(
                                ph1,
                                std::ptr::null_mut(),
                                std::ptr::null_mut(),
                                std::ptr::null_mut()
                            ),
                            m_res = alloc_mora(
                                mora_str,
                                std::ptr::null_mut(),
                                std::ptr::null_mut(),
                                std::ptr::null_mut(),
                                std::ptr::null_mut(),
                                std::ptr::null_mut()
                            ),
                            w_res = alloc_word(
                                pron,
                                pos,
                                ctype,
                                cform,
                                std::ptr::null_mut(),
                                std::ptr::null_mut(),
                                std::ptr::null_mut(),
                                std::ptr::null_mut()
                            )
                        )?;

                        (*m_node).head = p;
                        (*m_node).tail = p;
                        (*w).head = m_node;
                        (*w).tail = m_node;

                        (*p).up = m_node;
                        (*m_node).up = w;

                        (*label).phoneme_head = p;
                        (*label).phoneme_tail = p;
                        (*label).mora_head = m_node;
                        (*label).mora_tail = m_node;
                        (*label).word_head = w;
                        (*label).word_tail = w;

                        is_first_word = false;
                    } else {
                        if is_first_word {
                            insert_short_pause(label)?;

                            let (p, m_node, w) = try_alloc!(
                                label,
                                "first-word continuation nodes",
                                p_res = alloc_phoneme(
                                    ph1,
                                    (*label).phoneme_tail,
                                    std::ptr::null_mut(),
                                    std::ptr::null_mut()
                                ),
                                m_res = alloc_mora(
                                    mora_str,
                                    std::ptr::null_mut(),
                                    std::ptr::null_mut(),
                                    (*label).mora_tail,
                                    std::ptr::null_mut(),
                                    std::ptr::null_mut()
                                ),
                                w_res = alloc_word(
                                    pron,
                                    pos,
                                    ctype,
                                    cform,
                                    std::ptr::null_mut(),
                                    std::ptr::null_mut(),
                                    (*label).word_tail,
                                    std::ptr::null_mut()
                                )
                            )?;

                            (*m_node).head = p;
                            (*m_node).tail = p;
                            (*w).head = m_node;
                            (*w).tail = m_node;

                            (*p).up = m_node;
                            (*m_node).up = w;

                            (*(*label).phoneme_tail).next = p;
                            (*(*label).mora_tail).next = m_node;
                            (*(*label).word_tail).next = w;

                            (*label).phoneme_tail = p;
                            (*label).mora_tail = m_node;
                            (*label).word_tail = w;

                            is_first_word = false;
                        } else {
                            insert_short_pause(label)?;

                            let (p, m_node) = try_alloc!(
                                label,
                                "mora continuation nodes",
                                p_res = alloc_phoneme(
                                    ph1,
                                    (*label).phoneme_tail,
                                    std::ptr::null_mut(),
                                    std::ptr::null_mut()
                                ),
                                m_res = alloc_mora(
                                    mora_str,
                                    std::ptr::null_mut(),
                                    std::ptr::null_mut(),
                                    (*label).mora_tail,
                                    std::ptr::null_mut(),
                                    (*(*label).mora_tail).up
                                )
                            )?;

                            (*m_node).head = p;
                            (*m_node).tail = p;

                            (*p).up = m_node;

                            (*(*label).phoneme_tail).next = p;
                            (*(*label).mora_tail).next = m_node;

                            (*label).phoneme_tail = p;
                            (*label).mora_tail = m_node;
                            (*(*label).word_tail).tail = m_node;
                        }
                    }

                    // 2音素目の追加
                    if let Some(ph2) = ph2_opt {
                        insert_short_pause(label)?;

                        let p = try_alloc!(
                            label,
                            "second phoneme",
                            p_res = alloc_phoneme(
                                ph2,
                                (*label).phoneme_tail,
                                std::ptr::null_mut(),
                                (*label).mora_tail
                            )
                        )?;

                        (*(*label).phoneme_tail).next = p;
                        (*label).phoneme_tail = p;
                        (*(*label).mora_tail).tail = p;
                    }
                }
            }

            current_offset = m.end();
        }

        // 末尾の未解析文字列が残っていないか
        if current_offset != pron.len() {
            log::warn!(
                "JPCommonLabel_push_word(): {} is wrong mora list.",
                &pron[current_offset..]
            );
        }

        if is_first_word || (*label).phoneme_tail.is_null() {
            return Ok(());
        }
        if ptr_to_str_unchecked((*(*label).phoneme_tail).phoneme) == JPCOMMON_PHONEME_SHORT_PAUSE {
            return Ok(());
        }

        // アクセント句、BreathGroupの生成と結合
        if (*label).word_head == (*label).word_tail {
            let (a, b) = try_alloc!(
                label,
                "initial accent/breath group",
                a_res = alloc_accent_phrase(
                    acc,
                    None,
                    None,
                    (*label).word_tail,
                    (*label).word_tail,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut()
                ),
                b_res = alloc_breath_group(
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut()
                )
            )?;
            (*b).head = a;
            (*b).tail = a;

            (*a).up = b;
            (*(*label).word_tail).up = a;

            (*label).accent_head = a;
            (*label).accent_tail = a;
            (*label).breath_head = b;
            (*label).breath_tail = b;
        } else if chain_flag == 1 {
            (*(*label).word_tail).up = (*label).accent_tail;
            (*(*label).accent_tail).tail = (*label).word_tail;
        } else {
            let prev_tail_ph_ptr = get_ptr!((*label).word_tail, prev, tail, tail, next);
            let prev_tail_ph = if prev_tail_ph_ptr.is_null() {
                ""
            } else {
                ptr_to_str_unchecked((*prev_tail_ph_ptr).phoneme)
            };

            if prev_tail_ph != JPCOMMON_PHONEME_SHORT_PAUSE {
                let a = try_alloc!(
                    label,
                    "accent phrase",
                    a_res = alloc_accent_phrase(
                        acc,
                        None,
                        None,
                        (*label).word_tail,
                        (*label).word_tail,
                        (*label).accent_tail,
                        std::ptr::null_mut(),
                        (*label).breath_tail
                    )
                )?;
                (*(*label).word_tail).up = a;
                (*(*label).accent_tail).next = a;
                (*(*label).breath_tail).tail = a;
                (*label).accent_tail = a;
            } else {
                let (a, b) = try_alloc!(
                    label,
                    "accent/breath next nodes",
                    a_res = alloc_accent_phrase(
                        acc,
                        None,
                        None,
                        (*label).word_tail,
                        (*label).word_tail,
                        (*label).accent_tail,
                        std::ptr::null_mut(),
                        std::ptr::null_mut()
                    ),
                    b_res = alloc_breath_group(
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        (*label).breath_tail,
                        std::ptr::null_mut()
                    )
                )?;
                (*b).head = a;
                (*b).tail = a;

                (*a).up = b;
                (*(*label).word_tail).up = a;
                (*(*label).accent_tail).next = a;
                (*(*label).breath_tail).next = b;

                (*label).accent_tail = a;
                (*label).breath_tail = b;
            }
        }
        Ok(())
    }
}
