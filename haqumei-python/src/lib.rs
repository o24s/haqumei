use ::haqumei::{
    Haqumei, HaqumeiOptions, NjdFeature, OpenJTalk, WordPhonemeDetail, WordPhonemeMap,
    WordPhonemePair, open_jtalk::Dictionary, utils::default_is_non_pause_symbol,
};
use pyo3::prelude::*;
use std::{path::PathBuf, sync::Mutex};

fn to_py_err<E: std::fmt::Debug>(err: E) -> PyErr {
    pyo3::exceptions::PyRuntimeError::new_err(format!("{:?}", err))
}

#[pyclass(name = "NjdFeature", module = "haqumei", get_all, skip_from_py_object)]
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

#[pyclass(name = "MecabMorph", module = "haqumei", get_all, skip_from_py_object)]
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

#[pyclass(name = "WordPhonemePair", module = "haqumei", skip_from_py_object)]
#[derive(Clone)]
struct PyWordPhonemePair {
    #[pyo3(get)]
    word: String,
    #[pyo3(get)]
    phonemes: Vec<String>,
}

impl From<WordPhonemePair> for PyWordPhonemePair {
    fn from(pair: WordPhonemePair) -> Self {
        Self {
            word: pair.word,
            phonemes: pair.phonemes,
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
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyWordPhonemeMap {
    pub word: String,
    pub phonemes: Vec<String>,
    pub is_unknown: bool,
    pub is_ignored: bool,
}

impl From<WordPhonemeMap> for PyWordPhonemeMap {
    fn from(map: WordPhonemeMap) -> Self {
        Self {
            word: map.word,
            phonemes: map.phonemes,
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

#[derive(Debug, Clone, PartialEq)]
#[pyclass(module = "haqumei", get_all, skip_from_py_object)]
pub struct PyWordPhonemeDetail {
    pub word: String,
    pub phonemes: Vec<String>,
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
            phonemes: detail.phonemes,
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

#[pyclass(eq, eq_int, from_py_object)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UnicodeNormalization {
    None = 0,
    Nfc = 1,
    Nfkc = 2,
}

#[pyclass(name = "Dictionary", module = "haqumei")]
struct PyDictionary {
    inner: Dictionary,
}

#[pymethods]
impl PyDictionary {
    #[staticmethod]
    fn from_path(dict_dir: PathBuf, user_dict: Option<PathBuf>) -> PyResult<Self> {
        let inner = Dictionary::from_path(dict_dir, user_dict).map_err(to_py_err)?;
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

#[pymethods]
impl PyOpenJTalk {
    #[new]
    fn new() -> PyResult<Self> {
        let inner = OpenJTalk::new().map_err(to_py_err)?;
        Ok(Self {
            inner: Mutex::new(inner),
        })
    }

    #[staticmethod]
    fn from_dictionary(dict: &PyDictionary) -> PyResult<Self> {
        let inner = OpenJTalk::from_dictionary(dict.inner.clone()).map_err(to_py_err)?;
        Ok(Self {
            inner: Mutex::new(inner),
        })
    }

    #[staticmethod]
    fn from_path(dict_dir: &str, user_dict: Option<&str>) -> PyResult<Self> {
        let inner = if let Some(user_dict) = user_dict {
            OpenJTalk::from_path_with_userdict(dict_dir, user_dict).map_err(to_py_err)?
        } else {
            OpenJTalk::from_path(dict_dir).map_err(to_py_err)?
        };
        Ok(Self {
            inner: Mutex::new(inner),
        })
    }

    fn run_frontend(&self, text: &str) -> PyResult<Vec<PyNjdFeature>> {
        let mut guard = self.inner.lock().unwrap();
        let features = guard.run_frontend(text).map_err(to_py_err)?;
        Ok(features.into_iter().map(PyNjdFeature::from).collect())
    }

    fn run_frontend_batch(
        &self,
        py: Python<'_>,
        texts: Vec<String>,
    ) -> PyResult<Vec<Vec<PyNjdFeature>>> {
        py.detach(|| {
            Ok(self
                .inner
                .lock()
                .unwrap()
                .run_frontend_batch(&texts)
                .map_err(to_py_err)?
                .into_iter()
                .map(|features| features.into_iter().map(PyNjdFeature::from).collect())
                .collect())
        })
    }

    fn run_frontend_detailed(
        &self,
        text: &str,
    ) -> PyResult<(Vec<PyNjdFeature>, Vec<PyMecabMorph>)> {
        let (njd_features, mecab_morphs) = self
            .inner
            .lock()
            .unwrap()
            .run_frontend_detailed(text)
            .map_err(to_py_err)?;

        let py_njd = njd_features.into_iter().map(PyNjdFeature::from).collect();
        let py_mecab = mecab_morphs.into_iter().map(PyMecabMorph::from).collect();

        Ok((py_njd, py_mecab))
    }

    fn run_frontend_detailed_batch(
        &self,
        py: Python<'_>,
        texts: Vec<String>,
    ) -> PyResult<Vec<(Vec<PyNjdFeature>, Vec<PyMecabMorph>)>> {
        py.detach(|| {
            Ok(self
                .inner
                .lock()
                .unwrap()
                .run_frontend_detailed_batch(&texts)
                .map_err(to_py_err)?
                .into_iter()
                .map(|(features, morphs)| {
                    (
                        features.into_iter().map(PyNjdFeature::from).collect(),
                        morphs.into_iter().map(PyMecabMorph::from).collect(),
                    )
                })
                .collect())
        })
    }

    fn extract_fullcontext(&self, text: &str) -> PyResult<Vec<String>> {
        self.inner
            .lock()
            .unwrap()
            .extract_fullcontext(text)
            .map_err(to_py_err)
    }

    fn extract_fullcontext_batch(
        &self,
        py: Python<'_>,
        texts: Vec<String>,
    ) -> PyResult<Vec<Vec<String>>> {
        py.detach(|| {
            self.inner
                .lock()
                .unwrap()
                .extract_fullcontext_batch(&texts)
                .map_err(to_py_err)
        })
    }

    fn g2p(&self, text: &str) -> PyResult<Vec<String>> {
        self.inner.lock().unwrap().g2p(text).map_err(to_py_err)
    }

    fn g2p_batch(&self, py: Python<'_>, texts: Vec<String>) -> PyResult<Vec<Vec<String>>> {
        py.detach(|| {
            self.inner
                .lock()
                .unwrap()
                .g2p_batch(&texts)
                .map_err(to_py_err)
        })
    }

    fn g2p_detailed(&self, text: &str) -> PyResult<Vec<String>> {
        self.inner
            .lock()
            .unwrap()
            .g2p_detailed(text)
            .map_err(to_py_err)
    }

    fn g2p_detailed_batch(&self, py: Python<'_>, texts: Vec<String>) -> PyResult<Vec<Vec<String>>> {
        py.detach(|| {
            self.inner
                .lock()
                .unwrap()
                .g2p_detailed_batch(&texts)
                .map_err(to_py_err)
        })
    }

    fn g2p_kana(&self, text: &str) -> PyResult<String> {
        self.inner.lock().unwrap().g2p_kana(text).map_err(to_py_err)
    }

    fn g2p_kana_batch(&self, py: Python<'_>, texts: Vec<String>) -> PyResult<Vec<String>> {
        py.detach(|| {
            self.inner
                .lock()
                .unwrap()
                .g2p_kana_batch(&texts)
                .map_err(to_py_err)
        })
    }

    fn g2p_per_word(&self, text: &str) -> PyResult<Vec<Vec<String>>> {
        self.inner
            .lock()
            .unwrap()
            .g2p_per_word(text)
            .map_err(to_py_err)
    }

    fn g2p_per_word_batch(
        &self,
        py: Python<'_>,
        texts: Vec<String>,
    ) -> PyResult<Vec<Vec<Vec<String>>>> {
        py.detach(|| {
            self.inner
                .lock()
                .unwrap()
                .g2p_per_word_batch(&texts)
                .map_err(to_py_err)
        })
    }

    fn g2p_pairs(&self, text: &str) -> PyResult<Vec<PyWordPhonemePair>> {
        let mut guard = self.inner.lock().unwrap();
        let mapping = guard.g2p_pairs(text).map_err(to_py_err)?;
        Ok(mapping.into_iter().map(PyWordPhonemePair::from).collect())
    }

    fn g2p_pairs_batch(
        &self,
        py: Python<'_>,
        texts: Vec<String>,
    ) -> PyResult<Vec<Vec<PyWordPhonemePair>>> {
        py.detach(|| {
            Ok(self
                .inner
                .lock()
                .unwrap()
                .g2p_pairs_batch(&texts)
                .map_err(to_py_err)?
                .into_iter()
                .map(|map| map.into_iter().map(PyWordPhonemePair::from).collect())
                .collect())
        })
    }

    fn g2p_mapping(&self, text: &str) -> PyResult<Vec<PyWordPhonemeMap>> {
        let mut guard = self.inner.lock().unwrap();
        let mapping = guard.g2p_mapping(text).map_err(to_py_err)?;
        Ok(mapping.into_iter().map(PyWordPhonemeMap::from).collect())
    }

    fn g2p_mapping_batch(
        &self,
        py: Python<'_>,
        texts: Vec<String>,
    ) -> PyResult<Vec<Vec<PyWordPhonemeMap>>> {
        py.detach(|| {
            Ok(self
                .inner
                .lock()
                .unwrap()
                .g2p_mapping_batch(&texts)
                .map_err(to_py_err)?
                .into_iter()
                .map(|map| map.into_iter().map(PyWordPhonemeMap::from).collect())
                .collect())
        })
    }

    fn g2p_mapping_detailed(&self, text: &str) -> PyResult<Vec<PyWordPhonemeDetail>> {
        let mut guard = self.inner.lock().unwrap();
        let mapping = guard.g2p_mapping_detailed(text).map_err(to_py_err)?;
        Ok(mapping.into_iter().map(PyWordPhonemeDetail::from).collect())
    }

    fn g2p_mapping_detailed_batch(
        &self,
        py: Python<'_>,
        texts: Vec<String>,
    ) -> PyResult<Vec<Vec<PyWordPhonemeDetail>>> {
        py.detach(|| {
            Ok(self
                .inner
                .lock()
                .unwrap()
                .g2p_mapping_detailed_batch(&texts)
                .map_err(to_py_err)?
                .into_iter()
                .map(|map| map.into_iter().map(PyWordPhonemeDetail::from).collect())
                .collect())
        })
    }
}

#[pyclass(name = "Haqumei", module = "haqumei")]
struct PyHaqumei {
    inner: Mutex<Haqumei>,
}

#[pymethods]
impl PyHaqumei {
    #[allow(clippy::too_many_arguments)]
    #[new]
    #[pyo3(signature = (
        normalize_unicode = UnicodeNormalization::None,
        use_read_as_pron = false,
        revert_long_vowels = false,
        revert_yotsugana = false,
        modify_filler_accent = true,
        predict_nani = true,
        use_unidic_yomi = false,
        retreat_acc_nuc = true,
        modify_acc_after_chaining = true,
        process_odoriji = true
    ))]
    fn new(
        normalize_unicode: UnicodeNormalization,
        use_read_as_pron: bool,
        revert_long_vowels: bool,
        revert_yotsugana: bool,
        modify_filler_accent: bool,
        predict_nani: bool,
        use_unidic_yomi: bool,
        retreat_acc_nuc: bool,
        modify_acc_after_chaining: bool,
        process_odoriji: bool,
    ) -> PyResult<Self> {
        let options = HaqumeiOptions {
            normalize_unicode: match normalize_unicode {
                UnicodeNormalization::None => ::haqumei::UnicodeNormalization::None,
                UnicodeNormalization::Nfc => ::haqumei::UnicodeNormalization::Nfc,
                UnicodeNormalization::Nfkc => ::haqumei::UnicodeNormalization::Nfkc,
            },
            use_read_as_pron,
            revert_long_vowels,
            revert_yotsugana,
            modify_filler_accent,
            predict_nani,
            use_unidic_yomi,
            retreat_acc_nuc,
            modify_acc_after_chaining,
            process_odoriji,
            is_non_pause_symbol: default_is_non_pause_symbol,
        };

        let inner = Haqumei::with_options(options).map_err(to_py_err)?;
        Ok(Self {
            inner: Mutex::new(inner),
        })
    }

    fn run_frontend(&self, text: &str) -> PyResult<Vec<PyNjdFeature>> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .run_frontend(text)
            .map_err(to_py_err)?
            .into_iter()
            .map(PyNjdFeature::from)
            .collect())
    }

    fn run_frontend_batch(
        &self,
        py: Python<'_>,
        texts: Vec<String>,
    ) -> PyResult<Vec<Vec<PyNjdFeature>>> {
        py.detach(|| {
            Ok(self
                .inner
                .lock()
                .unwrap()
                .run_frontend_batch(&texts)
                .map_err(to_py_err)?
                .into_iter()
                .map(|features| features.into_iter().map(PyNjdFeature::from).collect())
                .collect())
        })
    }

    fn run_frontend_detailed(
        &self,
        text: &str,
    ) -> PyResult<(Vec<PyNjdFeature>, Vec<PyMecabMorph>)> {
        let (njd_features, mecab_morphs) = self
            .inner
            .lock()
            .unwrap()
            .run_frontend_detailed(text)
            .map_err(to_py_err)?;

        let py_njd = njd_features.into_iter().map(PyNjdFeature::from).collect();
        let py_mecab = mecab_morphs.into_iter().map(PyMecabMorph::from).collect();

        Ok((py_njd, py_mecab))
    }

    fn run_frontend_detailed_batch(
        &self,
        py: Python<'_>,
        texts: Vec<String>,
    ) -> PyResult<Vec<(Vec<PyNjdFeature>, Vec<PyMecabMorph>)>> {
        py.detach(|| {
            Ok(self
                .inner
                .lock()
                .unwrap()
                .run_frontend_detailed_batch(&texts)
                .map_err(to_py_err)?
                .into_iter()
                .map(|(features, morphs)| {
                    (
                        features.into_iter().map(PyNjdFeature::from).collect(),
                        morphs.into_iter().map(PyMecabMorph::from).collect(),
                    )
                })
                .collect())
        })
    }

    fn extract_fullcontext(&self, text: &str) -> PyResult<Vec<String>> {
        self.inner
            .lock()
            .unwrap()
            .extract_fullcontext(text)
            .map_err(to_py_err)
    }

    fn extract_fullcontext_batch(
        &self,
        py: Python<'_>,
        texts: Vec<String>,
    ) -> PyResult<Vec<Vec<String>>> {
        py.detach(|| {
            self.inner
                .lock()
                .unwrap()
                .extract_fullcontext_batch(&texts)
                .map_err(to_py_err)
        })
    }

    fn g2p(&self, text: &str) -> PyResult<Vec<String>> {
        self.inner.lock().unwrap().g2p(text).map_err(to_py_err)
    }

    fn g2p_batch(&self, py: Python<'_>, texts: Vec<String>) -> PyResult<Vec<Vec<String>>> {
        py.detach(|| {
            self.inner
                .lock()
                .unwrap()
                .g2p_batch(&texts)
                .map_err(to_py_err)
        })
    }

    fn g2p_detailed(&self, text: &str) -> PyResult<Vec<String>> {
        self.inner
            .lock()
            .unwrap()
            .g2p_detailed(text)
            .map_err(to_py_err)
    }

    fn g2p_detailed_batch(&self, py: Python<'_>, texts: Vec<String>) -> PyResult<Vec<Vec<String>>> {
        py.detach(|| {
            self.inner
                .lock()
                .unwrap()
                .g2p_detailed_batch(&texts)
                .map_err(to_py_err)
        })
    }

    fn g2p_kana(&self, text: &str) -> PyResult<String> {
        self.inner.lock().unwrap().g2p_kana(text).map_err(to_py_err)
    }

    fn g2p_kana_batch(&self, py: Python<'_>, texts: Vec<String>) -> PyResult<Vec<String>> {
        py.detach(|| {
            self.inner
                .lock()
                .unwrap()
                .g2p_kana_batch(&texts)
                .map_err(to_py_err)
        })
    }

    fn g2p_kana_per_word(&self, text: &str) -> PyResult<Vec<String>> {
        self.inner
            .lock()
            .unwrap()
            .g2p_kana_per_word(text)
            .map_err(to_py_err)
    }

    fn g2p_kana_per_word_batch(
        &self,
        py: Python<'_>,
        texts: Vec<String>,
    ) -> PyResult<Vec<Vec<String>>> {
        py.detach(|| {
            self.inner
                .lock()
                .unwrap()
                .g2p_kana_per_word_batch(&texts)
                .map_err(to_py_err)
        })
    }

    fn g2p_per_word(&self, text: &str) -> PyResult<Vec<Vec<String>>> {
        self.inner
            .lock()
            .unwrap()
            .g2p_per_word(text)
            .map_err(to_py_err)
    }

    fn g2p_per_word_batch(
        &self,
        py: Python<'_>,
        texts: Vec<String>,
    ) -> PyResult<Vec<Vec<Vec<String>>>> {
        py.detach(|| {
            self.inner
                .lock()
                .unwrap()
                .g2p_per_word_batch(&texts)
                .map_err(to_py_err)
        })
    }

    fn g2p_pairs(&self, text: &str) -> PyResult<Vec<PyWordPhonemePair>> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .g2p_pairs(text)
            .map_err(to_py_err)?
            .into_iter()
            .map(PyWordPhonemePair::from)
            .collect())
    }

    fn g2p_pairs_batch(
        &self,
        py: Python<'_>,
        texts: Vec<String>,
    ) -> PyResult<Vec<Vec<PyWordPhonemePair>>> {
        py.detach(|| {
            Ok(self
                .inner
                .lock()
                .unwrap()
                .g2p_pairs_batch(&texts)
                .map_err(to_py_err)?
                .into_iter()
                .map(|map| map.into_iter().map(PyWordPhonemePair::from).collect())
                .collect())
        })
    }

    fn g2p_mapping(&self, text: &str) -> PyResult<Vec<PyWordPhonemeMap>> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .g2p_mapping(text)
            .map_err(to_py_err)?
            .into_iter()
            .map(PyWordPhonemeMap::from)
            .collect())
    }

    fn g2p_mapping_batch(
        &self,
        py: Python<'_>,
        texts: Vec<String>,
    ) -> PyResult<Vec<Vec<PyWordPhonemeMap>>> {
        py.detach(|| {
            Ok(self
                .inner
                .lock()
                .unwrap()
                .g2p_mapping_batch(&texts)
                .map_err(to_py_err)?
                .into_iter()
                .map(|map| map.into_iter().map(PyWordPhonemeMap::from).collect())
                .collect())
        })
    }

    fn g2p_mapping_detailed(&self, text: &str) -> PyResult<Vec<PyWordPhonemeDetail>> {
        let mut guard = self.inner.lock().unwrap();
        let mapping = guard.g2p_mapping_detailed(text).map_err(to_py_err)?;
        Ok(mapping.into_iter().map(PyWordPhonemeDetail::from).collect())
    }

    fn g2p_mapping_detailed_batch(
        &self,
        py: Python<'_>,
        texts: Vec<String>,
    ) -> PyResult<Vec<Vec<PyWordPhonemeDetail>>> {
        py.detach(|| {
            Ok(self
                .inner
                .lock()
                .unwrap()
                .g2p_mapping_detailed_batch(&texts)
                .map_err(to_py_err)?
                .into_iter()
                .map(|map| map.into_iter().map(PyWordPhonemeDetail::from).collect())
                .collect())
        })
    }
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
    m.add_class::<UnicodeNormalization>()?;
    m.add_class::<PyOpenJTalk>()?;
    m.add_class::<PyMecabMorph>()?;
    m.add_class::<PyNjdFeature>()?;
    m.add_class::<PyWordPhonemePair>()?;
    m.add_class::<PyWordPhonemeMap>()?;
    m.add_class::<PyDictionary>()?;

    m.add_function(wrap_pyfunction!(update_global_dictionary, m)?)?;
    m.add_function(wrap_pyfunction!(unset_user_dictionary, m)?)?;
    Ok(())
}
