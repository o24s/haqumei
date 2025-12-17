pub mod dictionary;
mod jp_common;
mod mecab;
mod model;
pub mod njd;

#[cfg(test)]
mod tests;

use crate::NjdFeature;
use crate::open_jtalk::njd::{apply_plus_rules, njd_to_features};
use crate::{errors::HaqumeiError, ffi};
use arc_swap::ArcSwap;
use mecab::Mecab;
#[cfg(not(feature = "embed-dictionary"))]
use model::MecabModel;
use njd::Njd;
use jp_common::JpCommon;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use std::cell::Cell;
use std::ffi::{CStr, CString, c_char};
use std::marker::PhantomData;
use std::path::Path;
use std::sync::{Arc, LazyLock};

pub use dictionary::{Dictionary, MecabDictIndexCompiler};

pub static GLOBAL_MECAB_DICTIONARY: LazyLock<ArcSwap<Dictionary>> = LazyLock::new(|| {
    #[cfg(feature = "embed-dictionary")]
    {
        let default_dict = Dictionary::from_embedded()
            .expect("Failed to load embedded dictionary. This should not happen.");
        ArcSwap::from(Arc::new(default_dict))
    }
    #[cfg(not(feature = "embed-dictionary"))]
    {
        let dummy_model = MecabModel::new_uninitialized();
        let dummy_dict = Dictionary { model: Arc::new(dummy_model) };
        ArcSwap::from(Arc::new(dummy_dict))
    }
});

/// Updates (or sets) the global dictionary used by `OpenJTalk::new()`.
///
/// After calling this function, any new calls to `OpenJTalk::new()` will use this dictionary.
/// Existing instances will update to the new dictionary upon their next method call.
pub fn update_global_mecab_dictionary(new_dict: Dictionary) {
    GLOBAL_MECAB_DICTIONARY.store(Arc::new(new_dict));
}

#[derive(Debug)]
pub struct OpenJTalk {
    mecab: Mecab,
    njd: Njd,
    jp_common: JpCommon,
    dict: Option<Arc<Dictionary>>,
    _marker: PhantomData<Cell<()>>,
}

impl OpenJTalk {
    /// Creates a new `OpenJTalk` instance using the current global dictionary.
    ///
    /// If the `embed-dictionary` feature is enabled, this will automatically use
    /// the embedded dictionary on the first call.
    ///
    /// The global dictionary can be updated at any time using `update_GLOBAL_MECAB_DICTIONARY`.
    pub fn new() -> Result<Self, HaqumeiError> {
        let initial_dict = GLOBAL_MECAB_DICTIONARY.load_full();

        if !initial_dict.model.is_initialized() {
            return Err(HaqumeiError::GlobalDictionaryNotInitialized);
        }

        let mecab = Mecab::from_model(&initial_dict.model)?;
        let njd = Njd::new()?;
        let jp_common = JpCommon::new()?;

        Ok(Self {
            mecab,
            njd,
            jp_common,
            dict: Some(initial_dict),
            _marker: PhantomData,
        })
    }

    fn ensure_dictionary_is_latest(&mut self) -> Result<(), HaqumeiError> {
        let latest_dict = GLOBAL_MECAB_DICTIONARY.load();

        if let Some(active_dict) = &self.dict && !Arc::ptr_eq(active_dict, &*latest_dict) {
            log::info!("OpenJTalk instance detected a dictionary update. Re-initializing Mecab.");
            let new_mecab = Mecab::from_model(&latest_dict.model)?;

            self.dict = Some(latest_dict.clone());
            self.mecab = new_mecab;
        }
        Ok(())
    }

    pub fn from_dictonary(dict: Dictionary) -> Result<Self, HaqumeiError> {
        let mecab = Mecab::from_model(&dict.model)?;
        let njd = Njd::new()?;
        let jp_common = JpCommon::new()?;

        Ok(Self { mecab, njd, jp_common, dict: Some(Arc::new(dict)), _marker: PhantomData })
    }

    pub fn from_path<P: AsRef<Path>>(dict_dir: P, user_dict: Option<P>) -> Result<Self, HaqumeiError> {
        let mecab = Mecab::new()?;
        let njd = Njd::new()?;
        let jp_common = JpCommon::new()?;

        let path_to_cstring = |p: &Path| -> Result<CString, HaqumeiError> {
            let path_str = p.to_str().ok_or_else(|| {
                HaqumeiError::InvalidDictionaryPath(p.to_string_lossy().into_owned())
            })?;
            CString::new(path_str).map_err(|_| {
                HaqumeiError::InvalidDictionaryPath(path_str.to_string())
            })
        };

        let c_dict_dir = path_to_cstring(dict_dir.as_ref())?;

        let c_user_dict: Option<CString> = user_dict
            .as_ref()
            .map(|p| path_to_cstring(p.as_ref()))
            .transpose()?;

        let result = unsafe {
            if let Some(user_dict) = c_user_dict.as_ref().filter(|s| !s.to_bytes().is_empty()) {
                ffi::Mecab_load_with_userdic(
                    mecab.inner.as_ptr(),
                    c_dict_dir.as_ptr() as *mut c_char,
                    user_dict.as_ptr() as *mut c_char,
                )
            } else {
                ffi::Mecab_load(
                    mecab.inner.as_ptr(),
                    c_dict_dir.as_ptr() as *mut c_char,
                )
            }
        };

        if result != 1 {
            return Err(HaqumeiError::MecabLoadError);
        }

        Ok(Self { mecab, njd, jp_common, dict: None, _marker: PhantomData })
    }

    pub fn from_shared_dictionary(dict: Arc<Dictionary>) -> Result<Self, HaqumeiError> {
        let mecab = Mecab::from_model(&dict.model)?;
        let njd = Njd::new()?;
        let jp_common = JpCommon::new()?;

        Ok(Self {
            mecab,
            njd,
            jp_common,
            dict: Some(dict),
            _marker: PhantomData,
        })
    }

    pub fn run_frontend(&mut self, text: &str) -> Result<Vec<NjdFeature>, HaqumeiError> {
        self.ensure_dictionary_is_latest()?;
        let mecab_features = self.run_mecab(text)?;
        self.run_njd_from_mecab(&mecab_features)
    }

    pub fn g2p(&mut self, text: &str, kana: bool) -> Result<String, HaqumeiError> {
        self.ensure_dictionary_is_latest()?;
        let mecab_features = self.run_mecab(text)?;
        let njd_features = self.run_njd_from_mecab(&mecab_features)?;

        if njd_features.is_empty() {
            return Ok(String::new());
        }

        if !kana {
            let labels = self.make_label(&njd_features)?;

            // python: `lambda s: s.split("-")[1].split("+")[0]`
            let phonemes: Vec<_> = labels
                .iter()
                .skip(1)
                .take(labels.len() - 2)
                .filter_map(|s| {
                    s.split_once('-')
                    .and_then(|(_, after_minus)| after_minus.split_once('+'))
                    .map(|(p, _)| p)
                })
                .collect();

            Ok(phonemes.join(" "))
        } else {
            let kana_string: String = njd_features
                .iter()
                .map(|f| {
                    let p = if f.pos == "記号" { &f.string } else { &f.pron };
                    p.replace('’', "")
                })
                .collect();
            Ok(kana_string)
        }
    }

    pub fn run_mecab(&mut self, text: &str) -> Result<Vec<String>, HaqumeiError> {
        self.ensure_dictionary_is_latest()?;
        const BUFFER_SIZE: usize = 16384;

        let c_text = CString::new(text)?;

        let mut buffer = vec![0u8; BUFFER_SIZE];

        let result = unsafe {
            ffi::text2mecab(
                buffer.as_mut_ptr() as *mut i8,
                BUFFER_SIZE,
                c_text.as_ptr(),
            )
        };

        match result {
            ffi::text2mecab_result_t_TEXT2MECAB_RESULT_SUCCESS => {},
            ffi::text2mecab_result_t_TEXT2MECAB_RESULT_RANGE_ERROR => {
                return Err(HaqumeiError::Text2MecabError("Text is too long".to_string()));
            },
            ffi::text2mecab_result_t_TEXT2MECAB_RESULT_INVALID_ARGUMENT => {
                return Err(HaqumeiError::Text2MecabError("Invalid argument for text2mecab".to_string()));
            },
            _ => {
                return Err(HaqumeiError::Text2MecabError(format!("Unknown error from text2mecab: {}", result)));
            }
        }

        unsafe {
            ffi::Mecab_analysis(
                self.mecab.inner.as_ptr(),
                buffer.as_ptr() as *const i8,
            );
        }

        let morphs = unsafe {
            let size = ffi::Mecab_get_size(self.mecab.inner.as_ptr()) as usize;
            let features_ptr = ffi::Mecab_get_feature(self.mecab.inner.as_ptr());

            let mut result_vec = Vec::with_capacity(size);
            for i in 0..size {
                let c_feature_ptr = *features_ptr.add(i);
                let c_feature = CStr::from_ptr(c_feature_ptr);
                result_vec.push(c_feature.to_string_lossy().into_owned());
            }
            result_vec
        };
        unsafe {
            ffi::Mecab_refresh(self.mecab.inner.as_ptr());
        }

        let filtered_morphs: Vec<String> = morphs
            .into_iter()
            .filter(|m| !m.contains("記号,空白"))
            .collect();

        Ok(filtered_morphs)
    }

    pub fn run_njd_from_mecab(&mut self, mecab_features: &[String]) -> Result<Vec<NjdFeature>, HaqumeiError> {
        if mecab_features.is_empty() {
            return Ok(Vec::with_capacity(0));
        }

        let c_strings: Vec<CString> = mecab_features
            .iter()
            .map(|s| CString::new(s.as_str()))
            .collect::<Result<Vec<_>, _>>()?;

        let mut c_string_pointers: Vec<*const c_char> = c_strings
            .iter()
            .map(|cs| cs.as_ptr())
            .collect();


        unsafe {
            ffi::mecab2njd(
                self.njd.inner.as_mut(),
                c_string_pointers.as_mut_ptr() as *mut *mut c_char,
                c_string_pointers.len() as i32,
            );
            ffi::njd_set_pronunciation(self.njd.inner.as_mut());
        }

        let mut features = njd_to_features(&self.njd);
        apply_plus_rules(&mut features);

        Self::features_to_njd(&features, &mut self.njd)?;

        unsafe {
            ffi::njd_set_digit(self.njd.inner.as_mut());
            ffi::njd_set_accent_phrase(self.njd.inner.as_mut());
            ffi::njd_set_accent_type(self.njd.inner.as_mut());
            ffi::njd_set_unvoiced_vowel(self.njd.inner.as_mut());
            ffi::njd_set_long_vowel(self.njd.inner.as_mut());
        }

        let final_features = njd_to_features(&self.njd);
        unsafe {
            ffi::NJD_refresh(self.njd.inner.as_mut());
        }

        Ok(final_features)
    }

    pub(crate) fn make_label(&mut self, features: &[NjdFeature]) -> Result<Vec<String>, HaqumeiError> {
        Self::features_to_njd(features, &mut self.njd)?;

        let (label_size, label_feature_ptr) = unsafe {
            ffi::njd2jpcommon(self.jp_common.inner.as_mut(), self.njd.inner.as_mut());

            ffi::JPCommon_make_label(self.jp_common.inner.as_mut());

            let size = ffi::JPCommon_get_label_size(self.jp_common.inner.as_mut());
            let ptr = ffi::JPCommon_get_label_feature(self.jp_common.inner.as_mut());
            (size, ptr)
        };

        if label_feature_ptr.is_null() {
            return Ok(Vec::new());
        }

        let labels = unsafe {
            let mut result = Vec::with_capacity(label_size as usize);
            for i in 0..(label_size as isize) {
                let label_ptr = *label_feature_ptr.offset(i);
                let c_label = CStr::from_ptr(label_ptr);
                result.push(c_label.to_string_lossy().into_owned());
            }
            result
        };

        unsafe {
            ffi::JPCommon_refresh(self.jp_common.inner.as_mut());
            ffi::NJD_refresh(self.njd.inner.as_mut());
        }

        Ok(labels)
    }

    fn features_to_njd(features: &[NjdFeature], njd: &mut Njd) -> Result<(), HaqumeiError> {
        unsafe {
            ffi::NJD_clear(njd.inner.as_mut());
        }

        for feature in features {
            let c_string = CString::new(feature.string.as_str())?;
            let c_pos = CString::new(feature.pos.as_str())?;
            let c_pos_group1 = CString::new(feature.pos_group1.as_str())?;
            let c_pos_group2 = CString::new(feature.pos_group2.as_str())?;
            let c_pos_group3 = CString::new(feature.pos_group3.as_str())?;
            let c_ctype = CString::new(feature.ctype.as_str())?;
            let c_cform = CString::new(feature.cform.as_str())?;
            let c_orig = CString::new(feature.orig.as_str())?;
            let c_read = CString::new(feature.read.as_str())?;
            let c_pron = CString::new(feature.pron.as_str())?;
            let c_chain_rule = CString::new(feature.chain_rule.as_str())?;

            // SAFETY: This block interfaces with C FFI to build and manage an `NJDNode`.
            // Safety is ensured because `libc::calloc` is used for allocation and null pointers are checked,
            // `CString` data is safely handled since the C functions make deep copies of the strings,
            // and ownership of each allocated node is correctly transferred to the C `NJD` struct,
            // preventing Rust from freeing it and avoiding double-free errors.
            unsafe {
                let node = libc::calloc(1, std::mem::size_of::<ffi::NJDNode>()) as *mut ffi::NJDNode;
                if node.is_null() {
                    return Err(HaqumeiError::AllocationError);
                }

                ffi::NJDNode_initialize(node);

                ffi::NJDNode_set_string(node, c_string.as_ptr());
                ffi::NJDNode_set_pos(node, c_pos.as_ptr());
                ffi::NJDNode_set_pos_group1(node, c_pos_group1.as_ptr());
                ffi::NJDNode_set_pos_group2(node, c_pos_group2.as_ptr());
                ffi::NJDNode_set_pos_group3(node, c_pos_group3.as_ptr());
                ffi::NJDNode_set_ctype(node, c_ctype.as_ptr());
                ffi::NJDNode_set_cform(node, c_cform.as_ptr());
                ffi::NJDNode_set_orig(node, c_orig.as_ptr());
                ffi::NJDNode_set_read(node, c_read.as_ptr());
                ffi::NJDNode_set_pron(node, c_pron.as_ptr());
                ffi::NJDNode_set_acc(node, feature.acc);
                ffi::NJDNode_set_mora_size(node, feature.mora_size);
                ffi::NJDNode_set_chain_rule(node, c_chain_rule.as_ptr());
                ffi::NJDNode_set_chain_flag(node, feature.chain_flag);

                ffi::NJD_push_node(njd.inner.as_mut(), node);
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ParallelJTalk {
    dict: Arc<Dictionary>,
}

impl ParallelJTalk {
    pub fn new() -> Result<Self, HaqumeiError> {
        let dict = GLOBAL_MECAB_DICTIONARY.load_full();
        if !dict.model.is_initialized() {
            return Err(HaqumeiError::GlobalDictionaryNotInitialized);
        }
        Ok(Self { dict })
    }

    pub fn from_dictionary(dict: Dictionary) -> Self {
        Self { dict: Arc::new(dict) }
    }

    pub fn from_arc_dictionary(dict: Arc<Dictionary>) -> Self {
        Self { dict }
    }

    /// 複数のテキストに対して並列に `g2p` を実行します。
    pub fn g2p(&self, texts: &[String], kana: bool) -> Result<Vec<String>, HaqumeiError> {
        texts
            .par_iter()
            .map_init(
                || OpenJTalk::from_shared_dictionary(self.dict.clone())
                    .expect("Failed to initialize OpenJTalk worker"),
                |ojt, text| ojt.g2p(text, kana)
            )
            .collect()
    }

    pub fn run_frontend(&self, texts: &[String]) -> Result<Vec<Vec<NjdFeature>>, HaqumeiError> {
        texts
            .par_iter()
            .map_init(
                || OpenJTalk::from_shared_dictionary(self.dict.clone())
                    .expect("Failed to initialize OpenJTalk worker"),
                |ojt, text| ojt.run_frontend(text)
            )
            .collect()
    }
}

pub fn build_mecab_dictionary<P: AsRef<Path>>(path: P) -> Result<(), dictionary::DictCompilerError> {
    MecabDictIndexCompiler::new()
        .dict_dir(&path)
        .out_dir(&path)
        .run()
}
