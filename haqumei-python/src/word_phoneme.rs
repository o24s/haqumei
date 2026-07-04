use haqumei::{
    WordPhonemeDetail, WordPhonemeMap, WordPhonemePair, phoneme::PhonemeVecExt,
    word_phoneme::WordPhonemeProsody,
};
use pyo3::prelude::*;

use crate::prosody::PyProsodicPhoneme;

#[pyclass(name = "WordPhonemePair", module = "haqumei", skip_from_py_object)]
#[derive(Clone)]
pub struct PyWordPhonemePair {
    #[pyo3(get)]
    word: String,
    #[pyo3(get)]
    phonemes: Vec<&'static str>,
}

impl From<WordPhonemePair> for PyWordPhonemePair {
    fn from(pair: WordPhonemePair) -> Self {
        Self {
            word: pair.word,
            phonemes: pair.phonemes.into_strs(),
        }
    }
}

#[pymethods]
impl PyWordPhonemePair {
    fn __repr__(&self) -> String {
        format!(
            "PyWordPhonemePair(word={:?}, phonemes={:?})",
            self.word, self.phonemes,
        )
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.word == other.word && self.phonemes == other.phonemes
    }
}

#[pyclass(
    name = "WordPhonemeMap",
    module = "haqumei",
    get_all,
    skip_from_py_object,
)]
#[derive(Clone)]
pub struct PyWordPhonemeMap {
    pub word: String,
    pub phonemes: Vec<&'static str>,
    pub is_unknown: bool,
    pub is_ignored: bool,
}

impl From<WordPhonemeMap> for PyWordPhonemeMap {
    fn from(map: WordPhonemeMap) -> Self {
        Self {
            word: map.word,
            phonemes: map.phonemes.into_strs(),
            is_unknown: map.is_unknown,
            is_ignored: map.is_ignored,
        }
    }
}

#[pymethods]
impl PyWordPhonemeMap {
    fn __repr__(&self) -> String {
        format!(
            "WordPhonemeMap(word={:?}, phonemes={:?}, is_unknown={}, is_ignored={})",
            self.word, self.phonemes, self.is_unknown, self.is_ignored,
        )
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.word == other.word
            && self.phonemes == other.phonemes
            && self.is_unknown == other.is_unknown
            && self.is_ignored == other.is_ignored
    }
}

#[pyclass(
    name = "WordPhonemeDetail",
    module = "haqumei",
    get_all,
    skip_from_py_object,
)]
#[derive(Debug, Clone, PartialEq)]
pub struct PyWordPhonemeDetail {
    pub word: String,
    pub phonemes: Vec<&'static str>,
    pub features: Vec<String>,
    pub pos: String,
    pub pos_group1: String,
    pub pos_group2: String,
    pub pos_group3: String,
    pub ctype: String,
    pub cform: String,
    pub orig: String,
    pub read: String,
    pub pron: String,
    pub accent_nucleus: i32,
    pub mora_count: i32,
    pub chain_rule: String,
    pub chain_flag: i32,
    pub is_unknown: bool,
    pub is_ignored: bool,
}

impl From<WordPhonemeDetail> for PyWordPhonemeDetail {
    fn from(detail: WordPhonemeDetail) -> Self {
        Self {
            word: detail.word,
            phonemes: detail.phonemes.into_strs(),
            features: detail.features,
            pos: detail.pos,
            pos_group1: detail.pos_group1,
            pos_group2: detail.pos_group2,
            pos_group3: detail.pos_group3,
            ctype: detail.ctype,
            cform: detail.cform,
            orig: detail.orig,
            read: detail.read,
            pron: detail.pron,
            accent_nucleus: detail.accent_nucleus,
            mora_count: detail.mora_count,
            chain_rule: detail.chain_rule,
            chain_flag: detail.chain_flag,
            is_unknown: detail.is_unknown,
            is_ignored: detail.is_ignored,
        }
    }
}

#[pymethods]
impl PyWordPhonemeDetail {
    fn __repr__(&self) -> String {
        format!(
            "WordPhonemeDetail(word={:?}, phonemes={:?}, features={:?}, pos={:?}, pos_group1={:?}, \
             pos_group2={:?}, pos_group3={:?}, ctype={:?}, cform={:?}, orig={:?}, \
             read={:?}, pron={:?}, accent_nucleus={}, mora_count={}, chain_rule={:?}, \
             chain_flag={}, is_unknown={}, is_ignored={})",
            self.word,
            self.phonemes,
            self.features,
            self.pos,
            self.pos_group1,
            self.pos_group2,
            self.pos_group3,
            self.ctype,
            self.cform,
            self.orig,
            self.read,
            self.pron,
            self.accent_nucleus,
            self.mora_count,
            self.chain_rule,
            self.chain_flag,
            self.is_unknown,
            self.is_ignored,
        )
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.word == other.word
            && self.phonemes == other.phonemes
            && self.features == other.features
            && self.pos == other.pos
            && self.pos_group1 == other.pos_group1
            && self.pos_group2 == other.pos_group2
            && self.pos_group3 == other.pos_group3
            && self.ctype == other.ctype
            && self.cform == other.cform
            && self.orig == other.orig
            && self.read == other.read
            && self.pron == other.pron
            && self.accent_nucleus == other.accent_nucleus
            && self.mora_count == other.mora_count
            && self.chain_rule == other.chain_rule
            && self.chain_flag == other.chain_flag
            && self.is_unknown == other.is_unknown
            && self.is_ignored == other.is_ignored
    }
}

#[pyclass(
    name = "WordPhonemeProsody",
    module = "haqumei",
    get_all,
    skip_from_py_object,
)]
#[derive(Debug, Clone)]
pub struct PyWordPhonemeProsody {
    pub word: String,
    pub phonemes: Vec<PyProsodicPhoneme>,
    pub pos: String,
    pub pos_group1: String,
    pub pos_group2: String,
    pub pos_group3: String,
    pub ctype: String,
    pub cform: String,
    pub orig: String,
    pub read: String,
    pub pron: String,
    pub accent_nucleus: i32,
    pub mora_count: i32,
    pub chain_rule: String,
    pub chain_flag: i32,
    pub is_unknown: bool,
    pub is_ignored: bool,
}

#[pymethods]
impl PyWordPhonemeProsody {
    fn __repr__(&self) -> String {
        format!(
            "WordPhonemeDetail(word={:?}, phonemes={:?}, pos={:?}, pos_group1={:?}, \
             pos_group2={:?}, pos_group3={:?}, ctype={:?}, cform={:?}, orig={:?}, \
             read={:?}, pron={:?}, accent_nucleus={}, mora_count={}, chain_rule={:?}, \
             chain_flag={}, is_unknown={}, is_ignored={})",
            self.word,
            self.phonemes,
            self.pos,
            self.pos_group1,
            self.pos_group2,
            self.pos_group3,
            self.ctype,
            self.cform,
            self.orig,
            self.read,
            self.pron,
            self.accent_nucleus,
            self.mora_count,
            self.chain_rule,
            self.chain_flag,
            self.is_unknown,
            self.is_ignored,
        )
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.word == other.word
            && self.phonemes == other.phonemes
            && self.pos == other.pos
            && self.pos_group1 == other.pos_group1
            && self.pos_group2 == other.pos_group2
            && self.pos_group3 == other.pos_group3
            && self.ctype == other.ctype
            && self.cform == other.cform
            && self.orig == other.orig
            && self.read == other.read
            && self.pron == other.pron
            && self.accent_nucleus == other.accent_nucleus
            && self.mora_count == other.mora_count
            && self.chain_rule == other.chain_rule
            && self.chain_flag == other.chain_flag
            && self.is_unknown == other.is_unknown
            && self.is_ignored == other.is_ignored
    }
}

impl From<WordPhonemeProsody> for PyWordPhonemeProsody {
    fn from(p: WordPhonemeProsody) -> Self {
        Self {
            word: p.word,
            phonemes: p
                .phonemes
                .into_iter()
                .map(PyProsodicPhoneme::from)
                .collect(),
            pos: p.pos,
            pos_group1: p.pos_group1,
            pos_group2: p.pos_group2,
            pos_group3: p.pos_group3,
            ctype: p.ctype,
            cform: p.cform,
            orig: p.orig,
            read: p.read,
            pron: p.pron,
            accent_nucleus: p.accent_nucleus,
            mora_count: p.mora_count,
            chain_rule: p.chain_rule,
            chain_flag: p.chain_flag,
            is_unknown: p.is_unknown,
            is_ignored: p.is_ignored,
        }
    }
}
