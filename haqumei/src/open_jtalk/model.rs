use std::ffi::{CString, c_char};

use crate::{errors::HaqumeiError, ffi};

#[derive(Debug)]
pub(crate) struct MecabModel {
    pub(crate) ptr: *mut ffi::mecab_model_t,
}

unsafe impl Send for MecabModel {}
unsafe impl Sync for MecabModel {}

impl MecabModel {
    pub fn new(dict_dir: &str, user_dict: Option<&str>) -> Result<Self, HaqumeiError> {
        let mut argv: Vec<*mut c_char> = Vec::new();

        let arg0 = CString::new("mecab").unwrap();
        let arg1 = CString::new("-d").unwrap();
        let arg2 = CString::new(dict_dir)?;
        let arg3 = CString::new("-u").unwrap();

        let user_dic_c: Option<CString> = user_dict.map(CString::new).transpose()?;

        argv.push(arg0.as_ptr() as *mut _);
        argv.push(arg1.as_ptr() as *mut _);
        argv.push(arg2.as_ptr() as *mut _);

        if let Some(udic) = &user_dic_c {
            argv.push(arg3.as_ptr() as *mut _);
            argv.push(udic.as_ptr() as *mut _);
        }

        let model_ptr = unsafe { ffi::mecab_model_new(argv.len() as i32, argv.as_mut_ptr()) };

        if model_ptr.is_null() {
            Err(HaqumeiError::MecabLoadError)
        } else {
            Ok(Self { ptr: model_ptr })
        }
    }

    #[allow(unused)]
    pub(crate) fn new_uninitialized() -> Self {
        Self { ptr: std::ptr::null_mut() }
    }

    pub(crate) fn is_initialized(&self) -> bool {
        !self.ptr.is_null()
    }
}

impl Drop for MecabModel {
    fn drop(&mut self) {
        unsafe { ffi::mecab_model_destroy(self.ptr); }
    }
}
