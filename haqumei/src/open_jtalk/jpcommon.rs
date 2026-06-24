use crate::{errors::HaqumeiError, ffi, open_jtalk::jpcommon_rule::NODATA};

use std::{ffi::c_char, mem::MaybeUninit, ptr::NonNull};

#[derive(Debug)]
pub(crate) struct JpCommon {
    pub(crate) inner: NonNull<ffi::JPCommon>,
}

impl JpCommon {
    pub(crate) fn new() -> Result<Self, HaqumeiError> {
        unsafe {
            let mut jp_common_uninit = Box::new(MaybeUninit::<ffi::JPCommon>::uninit());

            ffi::JPCommon_initialize(jp_common_uninit.as_mut_ptr());

            let jp_common_init = jp_common_uninit.assume_init();

            let raw_ptr = Box::into_raw(jp_common_init);

            match NonNull::new(raw_ptr) {
                Some(inner) => Ok(Self { inner }),
                None => {
                    let _ = Box::from_raw(raw_ptr);
                    Err(HaqumeiError::AllocationError("_JPCommon"))
                }
            }
        }
    }
}

impl Drop for JpCommon {
    fn drop(&mut self) {
        unsafe {
            ffi::JPCommon_clear(self.inner.as_ptr());

            let _ = Box::from_raw(self.inner.as_ptr());
        }
    }
}

pub(crate) trait FreeNode {
    unsafe fn free_node(self);
}

impl FreeNode for *mut ffi::JPCommonLabelPhoneme {
    unsafe fn free_node(self) {
        unsafe {
            if !self.is_null() {
                let s = (*self).phoneme;
                if !s.is_null() && s != NODATA.as_ptr() as *mut c_char {
                    libc::free(s as *mut _);
                }
                libc::free(self as *mut _);
            }
        }
    }
}

impl FreeNode for *mut ffi::JPCommonLabelMora {
    unsafe fn free_node(self) {
        unsafe {
            if !self.is_null() {
                let s = (*self).mora;
                if !s.is_null() && s != NODATA.as_ptr() as *mut c_char {
                    libc::free(s as *mut _);
                }
                libc::free(self as *mut _);
            }
        }
    }
}

impl FreeNode for *mut ffi::JPCommonLabelWord {
    unsafe fn free_node(self) {
        unsafe {
            if !self.is_null() {
                let pron = (*self).pron;
                if !pron.is_null() && pron != NODATA.as_ptr() as *mut c_char {
                    libc::free(pron as *mut _);
                }
                let pos = (*self).pos;
                if !pos.is_null() && pos != NODATA.as_ptr() as *mut c_char {
                    libc::free(pos as *mut _);
                }
                let ctype = (*self).ctype;
                if !ctype.is_null() && ctype != NODATA.as_ptr() as *mut c_char {
                    libc::free(ctype as *mut _);
                }
                let cform = (*self).cform;
                if !cform.is_null() && cform != NODATA.as_ptr() as *mut c_char {
                    libc::free(cform as *mut _);
                }
                libc::free(self as *mut _);
            }
        }
    }
}

impl FreeNode for *mut ffi::JPCommonLabelAccentPhrase {
    unsafe fn free_node(self) {
        unsafe {
            if !self.is_null() {
                let emotion = (*self).emotion;
                if !emotion.is_null() && emotion != NODATA.as_ptr() as *mut c_char {
                    libc::free(emotion as *mut _);
                }
                let excl = (*self).excl;
                if !excl.is_null() && excl != NODATA.as_ptr() as *mut c_char {
                    libc::free(excl as *mut _);
                }
                libc::free(self as *mut _);
            }
        }
    }
}

impl FreeNode for *mut ffi::JPCommonLabelBreathGroup {
    unsafe fn free_node(self) {
        if !self.is_null() {
            unsafe { libc::free(self as *mut _) };
        }
    }
}
