use std::{mem::MaybeUninit, ptr::NonNull};

use crate::{errors::HaqumeiError, ffi};

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
