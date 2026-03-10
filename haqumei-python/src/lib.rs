use ::haqumei::{
    Haqumei, HaqumeiOptions, NjdFeature, OpenJTalk, WordPhonemeDetail, WordPhonemeMap,
    open_jtalk::Dictionary,
};
use pyo3::prelude::*;
use std::{path::PathBuf, sync::Mutex};

fn to_py_err<E: std::fmt::Debug>(err: E) -> PyErr {
    pyo3::exceptions::PyRuntimeError::new_err(format!("{:?}", err))
}

#[pyclass(name = "NjdFeature", module = "haqumei", skip_from_py_object)]
#[derive(Clone)]
struct PyNjdFeature {
    #[pyo3(get)]
    string: String,
    #[pyo3(get)]
    pos: String,
    #[pyo3(get)]
    pos_group1: String,
    #[pyo3(get)]
    pos_group2: String,
    #[pyo3(get)]
    pos_group3: String,
    #[pyo3(get)]
    ctype: String,
    #[pyo3(get)]
    cform: String,
    #[pyo3(get)]
    orig: String,
    #[pyo3(get)]
    read: String,
    #[pyo3(get)]
    pron: String,
    #[pyo3(get)]
    acc: i32,
    #[pyo3(get)]
    mora_size: i32,
    #[pyo3(get)]
    chain_rule: String,
    #[pyo3(get)]
    chain_flag: i32,
}

impl From<&NjdFeature> for PyNjdFeature {
    fn from(f: &NjdFeature) -> Self {
        Self {
            string: f.string.clone(),
            pos: f.pos.clone(),
            pos_group1: f.pos_group1.clone(),
            pos_group2: f.pos_group2.clone(),
            pos_group3: f.pos_group3.clone(),
            ctype: f.ctype.clone(),
            cform: f.cform.clone(),
            orig: f.orig.clone(),
            read: f.read.clone(),
            pron: f.pron.clone(),
            acc: f.acc,
            mora_size: f.mora_size,
            chain_rule: f.chain_rule.clone(),
            chain_flag: f.chain_flag,
        }
    }
}

#[pyclass(name = "WordPhonemeMap", module = "haqumei", skip_from_py_object)]
#[derive(Clone)]
struct PyWordPhonemeMap {
    #[pyo3(get)]
    word: String,
    #[pyo3(get)]
    phonemes: Vec<String>,
}

impl From<WordPhonemeMap> for PyWordPhonemeMap {
    fn from(map: WordPhonemeMap) -> Self {
        Self {
            word: map.word,
            phonemes: map.phonemes,
        }
    }
}

#[pymethods]
impl PyWordPhonemeMap {
    fn __repr__(&self) -> String {
        format!(
            "PyWordPhonemeMap(word={:?}, phonemes={:?})",
            self.word, self.phonemes,
        )
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.word == other.word && self.phonemes == other.phonemes
    }
}

#[pyclass(name = "WordPhonemeDetail", module = "haqumei", skip_from_py_object)]
#[derive(Clone)]
pub struct PyWordPhonemeDetail {
    #[pyo3(get)]
    pub word: String,
    #[pyo3(get)]
    pub phonemes: Vec<String>,
    #[pyo3(get)]
    pub is_unknown: bool,
    #[pyo3(get)]
    pub is_ignored: bool,
}

impl From<WordPhonemeDetail> for PyWordPhonemeDetail {
    fn from(map: WordPhonemeDetail) -> Self {
        Self {
            word: map.word,
            phonemes: map.phonemes,
            is_unknown: map.is_unknown,
            is_ignored: map.is_ignored,
        }
    }
}

#[pymethods]
impl PyWordPhonemeDetail {
    fn __repr__(&self) -> String {
        format!(
            "WordPhonemeDetail(word={:?}, phonemes={:?}, is_unknown={}, is_ignored={})",
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
        let inner = OpenJTalk::from_dictonary(dict.inner.clone()).map_err(to_py_err)?;
        Ok(Self {
            inner: Mutex::new(inner),
        })
    }

    #[staticmethod]
    fn from_path(dict_dir: &str, user_dict: Option<&str>) -> PyResult<Self> {
        let inner = OpenJTalk::from_path(dict_dir, user_dict).map_err(to_py_err)?;
        Ok(Self {
            inner: Mutex::new(inner),
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

    fn run_frontend(&self, text: &str) -> PyResult<Vec<PyNjdFeature>> {
        let mut guard = self.inner.lock().unwrap();
        let features = guard.run_frontend(text).map_err(to_py_err)?;
        Ok(features.iter().map(PyNjdFeature::from).collect())
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
}

#[pyclass(name = "Haqumei", module = "haqumei")]
struct PyHaqumei {
    inner: Mutex<Haqumei>,
}

#[pymethods]
impl PyHaqumei {
    #[new]
    #[pyo3(signature = (
        normalize_unicode = false,
        modify_filler_accent = true,
        predict_nani = false,
        modify_kanji_yomi = false,
        retreat_acc_nuc = true,
        modify_acc_after_chaining = true,
        process_odoriji = true
    ))]
    fn new(
        normalize_unicode: bool,
        modify_filler_accent: bool,
        predict_nani: bool,
        modify_kanji_yomi: bool,
        retreat_acc_nuc: bool,
        modify_acc_after_chaining: bool,
        process_odoriji: bool,
    ) -> PyResult<Self> {
        let options = HaqumeiOptions {
            normalize_unicode,
            modify_filler_accent,
            predict_nani,
            modify_kanji_yomi,
            retreat_acc_nuc,
            modify_acc_after_chaining,
            process_odoriji,
        };

        let inner = Haqumei::with_options(options).map_err(to_py_err)?;
        Ok(Self {
            inner: Mutex::new(inner),
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
        Ok(self
            .inner
            .lock()
            .unwrap()
            .g2p_mapping_detailed(text)
            .map_err(to_py_err)?
            .into_iter()
            .map(PyWordPhonemeDetail::from)
            .collect())
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

    fn run_frontend(&self, text: &str) -> PyResult<Vec<PyNjdFeature>> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .run_frontend(text)
            .map_err(to_py_err)?
            .iter()
            .map(PyNjdFeature::from)
            .collect())
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
    m.add_class::<PyNjdFeature>()?;
    m.add_class::<PyWordPhonemeMap>()?;
    m.add_class::<PyWordPhonemeDetail>()?;
    m.add_class::<PyDictionary>()?;

    m.add_function(wrap_pyfunction!(update_global_dictionary, m)?)?;
    m.add_function(wrap_pyfunction!(unset_user_dictionary, m)?)?;
    Ok(())
}
