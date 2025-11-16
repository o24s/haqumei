use crate::{errors::HaqumeiError, ffi, open_jtalk::model::MecabModel};
use std::{mem::MaybeUninit, ptr::NonNull};

#[derive(Debug)]
pub(crate) struct Mecab {
    pub(crate) inner: NonNull<ffi::Mecab>,
    owns_model: bool,
}

impl Mecab {
    pub(crate) fn new() -> Result<Self, HaqumeiError> {
        unsafe {
            let mut mecab_uninit = Box::new(MaybeUninit::<ffi::Mecab>::uninit());

            ffi::Mecab_initialize(mecab_uninit.as_mut_ptr());

            let mecab_init = mecab_uninit.assume_init();

            let raw_ptr = Box::into_raw(mecab_init);

            match NonNull::new(raw_ptr) {
                Some(inner) => Ok(Self { inner, owns_model: true }),
                None => {
                    let _ = Box::from_raw(raw_ptr);
                    Err(HaqumeiError::AllocationError)
                }
            }
        }
    }

    pub(crate) fn from_model(model: &MecabModel) -> Result<Self, HaqumeiError> {
        unsafe {
            let tagger_ptr = ffi::mecab_model_new_tagger(model.ptr);
            if tagger_ptr.is_null() {
                return Err(HaqumeiError::AllocationError);
            }

            let lattice_ptr = ffi::mecab_model_new_lattice(model.ptr);
            if lattice_ptr.is_null() {
                ffi::mecab_destroy(tagger_ptr);

                return Err(HaqumeiError::AllocationError);
            }

            let mut mecab_uninit = Box::new(MaybeUninit::<ffi::Mecab>::uninit());
            let mecab_ptr = mecab_uninit.as_mut_ptr();

            (*mecab_ptr).model = model.ptr as *mut _;
            (*mecab_ptr).tagger = tagger_ptr as *mut _;
            (*mecab_ptr).lattice = lattice_ptr as *mut _;
            (*mecab_ptr).feature = std::ptr::null_mut();
            (*mecab_ptr).size = 0;

            let mecab_init = Box::into_raw(mecab_uninit.assume_init());

            Ok(Self { inner: NonNull::new(mecab_init).unwrap(), owns_model: false })
        }
    }
}

impl Drop for Mecab {
    fn drop(&mut self) {
        unsafe {
            if self.owns_model {
                ffi::Mecab_clear(self.inner.as_ptr());
            } else {
                let m = self.inner.as_mut();

                if !m.feature.is_null() {
                    for i in 0..m.size {
                        libc::free(*m.feature.add(i as usize) as *mut _);
                    }
                    libc::free(m.feature as *mut _);
                }

                if !m.lattice.is_null() {
                    ffi::mecab_lattice_destroy(m.lattice as *mut _);
                }

                if !m.tagger.is_null() {
                    ffi::mecab_destroy(m.tagger as *mut _);
                }

                // Model (m.model) は解放しない
            }

            let _ = Box::from_raw(self.inner.as_ptr());
        }
    }
}