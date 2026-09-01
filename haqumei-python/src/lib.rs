#![allow(clippy::clone_on_copy)]

pub mod candidates;
pub mod jlabel;
pub mod prosody;
pub mod pyhaqumei;
pub mod pyopenjtalk;
pub mod word_phoneme;

use ::haqumei::{Haqumei, NjdFeature, OpenJTalk, open_jtalk::Dictionary};

use pyo3::{prelude::*, types::PyTuple};
use std::{path::PathBuf, sync::Mutex};

use crate::{
    prosody::{PyPitchAccent, PyProsodicPhoneme, PyProsodyFormat},
    word_phoneme::{
        PyWordPhonemeDetail, PyWordPhonemeMap, PyWordPhonemePair, PyWordPhonemeProsody,
    },
};

pub(crate) fn to_py_err<E: std::fmt::Debug>(err: E) -> PyErr {
    pyo3::exceptions::PyRuntimeError::new_err(format!("{:?}", err))
}

#[pyclass(name = "NjdFeature", module = "haqumei", get_all, from_py_object)]
#[derive(Clone)]
struct PyNjdFeature {
    string: String,
    pos: String,
    pos_group1: String,
    pos_group2: String,
    pos_group3: String,
    ctype: String,
    cform: String,
    orig: String,
    read: String,
    pron: String,
    acc: i32,
    mora_size: i32,
    chain_rule: String,
    chain_flag: i32,
}

impl From<NjdFeature> for PyNjdFeature {
    fn from(f: NjdFeature) -> Self {
        Self {
            string: f.string,
            pos: f.pos,
            pos_group1: f.pos_group1,
            pos_group2: f.pos_group2,
            pos_group3: f.pos_group3,
            ctype: f.ctype,
            cform: f.cform,
            orig: f.orig,
            read: f.read,
            pron: f.pron,
            acc: f.acc,
            mora_size: f.mora_size,
            chain_rule: f.chain_rule,
            chain_flag: f.chain_flag,
        }
    }
}

#[pyclass(name = "MecabMorph", module = "haqumei", get_all, from_py_object)]
#[derive(Debug, Clone, PartialEq)]
pub struct PyMecabMorph {
    pub surface: String,
    pub feature: String,
    pub left_id: u16,
    pub right_id: u16,
    pub pos_id: u16,
    pub word_cost: i16,
    pub is_unknown: bool,
    pub is_ignored: bool,
}

impl From<::haqumei::MecabMorph> for PyMecabMorph {
    fn from(m: ::haqumei::MecabMorph) -> Self {
        Self {
            surface: m.surface,
            feature: m.feature,
            left_id: m.left_id,
            right_id: m.right_id,
            pos_id: m.pos_id,
            word_cost: m.word_cost,
            is_unknown: m.is_unknown,
            is_ignored: m.is_ignored,
        }
    }
}

#[pyclass(
    name = "UnicodeNormalization",
    module = "haqumei",
    eq,
    eq_int,
    from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UnicodeNormalization {
    None_ = 0,
    Nfc = 1,
    Nfkc = 2,
}

#[pyclass(
    name = "IuPronunciation",
    module = "haqumei",
    eq,
    eq_int,
    from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum IuPronunciation {
    None_ = 0,
    Iu = 1,
    Yuu = 2,
    KanjiIu = 3,
    KanjiYuu = 4,
    YuuBase = 5,
    KanjiYuuBase = 6,
}

#[pyclass(name = "Dictionary", module = "haqumei")]
struct PyDictionary {
    inner: Dictionary,
}

#[pymethods]
impl PyDictionary {
    #[staticmethod]
    #[pyo3(signature = (dict_dir, user_dict = None))]
    fn from_path(dict_dir: PathBuf, user_dict: Option<PathBuf>) -> PyResult<Self> {
        let inner = Dictionary::from_path(dict_dir, user_dict).map_err(to_py_err)?;
        Ok(Self { inner })
    }

    #[staticmethod]
    fn from_paths(dict_dir: PathBuf, user_dicts: Vec<PathBuf>) -> PyResult<Self> {
        let inner = Dictionary::from_paths(&dict_dir, &user_dicts).map_err(to_py_err)?;
        Ok(Self { inner })
    }

    #[staticmethod]
    fn from_embedded() -> PyResult<Self> {
        {
            let inner = Dictionary::from_embedded().map_err(to_py_err)?;
            Ok(Self { inner })
        }
    }
}

#[pyclass(name = "OpenJTalk", module = "haqumei")]
struct PyOpenJTalk {
    inner: Mutex<OpenJTalk>,
}

#[pyclass(name = "Haqumei", module = "haqumei")]
struct PyHaqumei {
    inner: Mutex<Haqumei>,
}

#[pyfunction]
fn update_global_dictionary(dict: &PyDictionary) {
    ::haqumei::open_jtalk::update_global_dictionary(dict.inner.clone());
}

#[pyfunction]
fn unset_user_dictionary() -> PyResult<()> {
    ::haqumei::open_jtalk::unset_user_dictionary().map_err(to_py_err)
}

#[pymodule]
fn haqumei(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyHaqumei>()?;
    m.add_class::<PyOpenJTalk>()?;

    m.add_class::<UnicodeNormalization>()?;
    m.add_class::<IuPronunciation>()?;

    m.add_class::<PyNjdFeature>()?;
    m.add_class::<PyDictionary>()?;

    m.add_class::<PyMecabMorph>()?;
    m.add_class::<PyWordPhonemePair>()?;
    m.add_class::<PyWordPhonemeMap>()?;
    m.add_class::<PyWordPhonemeDetail>()?;
    m.add_class::<PyWordPhonemeProsody>()?;

    m.add_class::<crate::candidates::PyCandidateOptions>()?;
    m.add_class::<crate::candidates::PyCandidateReading>()?;
    m.add_class::<crate::candidates::PyCandidateAlternative>()?;
    m.add_class::<crate::candidates::PyCandidateBranch>()?;
    m.add_class::<crate::candidates::PyCandidate>()?;
    m.add_class::<crate::candidates::PyCandidates>()?;
    m.add_class::<crate::candidates::PyCandidateDetail>()?;
    m.add_class::<crate::candidates::PyCandidatesDetail>()?;
    m.add_class::<crate::candidates::PyCandidateProsody>()?;
    m.add_class::<crate::candidates::PyCandidatesProsody>()?;

    m.add_class::<PyProsodicPhoneme>()?;
    m.add_class::<PyProsodyFormat>()?;
    m.add_class::<PyPitchAccent>()?;

    m.add_class::<crate::jlabel::PyLabel>()?;
    m.add_class::<crate::jlabel::PyLabelPhoneme>()?;
    m.add_class::<crate::jlabel::PyMora>()?;
    m.add_class::<crate::jlabel::PyWord>()?;
    m.add_class::<crate::jlabel::PyAccentPhraseCurrent>()?;
    m.add_class::<crate::jlabel::PyAccentPhrasePrevNext>()?;
    m.add_class::<crate::jlabel::PyBreathGroupCurrent>()?;
    m.add_class::<crate::jlabel::PyBreathGroupPrevNext>()?;
    m.add_class::<crate::jlabel::PyUtterance>()?;

    m.add_function(wrap_pyfunction!(update_global_dictionary, m)?)?;
    m.add_function(wrap_pyfunction!(unset_user_dictionary, m)?)?;

    m.add(
        "ALL_PHONEMES",
        PyTuple::new(m.py(), ::haqumei::Phoneme::ALL.iter().map(|i| i.as_str()))?,
    )?;
    Ok(())
}
