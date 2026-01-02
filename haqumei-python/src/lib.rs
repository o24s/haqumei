use ::haqumei::{Haqumei, NjdFeature, OpenJTalk, ParallelJTalk, WordPhonemeMap, open_jtalk::Dictionary};
use pyo3::prelude::*;
use std::{path::PathBuf, sync::Mutex};

/// RustのエラーをPythonの例外に変換するヘルパー関数
fn to_py_err<E: std::fmt::Debug>(err: E) -> PyErr {
    pyo3::exceptions::PyRuntimeError::new_err(format!("{:?}", err))
}

#[pyclass(name = "NjdFeature", module = "haqumei")]
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

/// 単語とその音素列の対応関係を表すデータクラス。
#[pyclass(name = "WordPhonemeMap", module = "haqumei")]
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
            "<WordPhonemeMap word='{}', phonemes={:?}>",
            self.word, self.phonemes
        )
    }
}

/// OpenJTalk用のMeCab辞書データ。
///
/// 一度ロードした辞書データをメモリ上で保持します。
/// 複数の `OpenJTalk` や `ParallelJTalk` インスタンス間で共有可能です。
#[pyclass(name = "Dictionary", module = "haqumei")]
struct PyDictionary {
    inner: Dictionary,
}

#[pymethods]
impl PyDictionary {
    /// 指定されたパスから辞書をロードします。
    ///
    /// Args:
    ///     dict_dir (str): システム辞書のディレクトリパス
    ///     user_dict (str, optional): ユーザー辞書のパス
    #[staticmethod]
    fn from_path(dict_dir: PathBuf, user_dict: Option<PathBuf>) -> PyResult<Self> {
        let inner = Dictionary::from_path(dict_dir, user_dict).map_err(to_py_err)?;
        Ok(Self { inner })
    }

    /// ライブラリに埋め込まれた辞書データをロードします。
    #[staticmethod]
    fn from_embedded() -> PyResult<Self> {
        {
            let inner = Dictionary::from_embedded().map_err(to_py_err)?;
            Ok(Self { inner })
        }
    }
}

/// OpenJTalkの基本的なラッパークラス。
///
/// このクラスは内部状態を持ち、Rust の安全性モデルを Python に持ち出せないために、メソッド呼び出し時に排他ロックを取得します。
/// そのためスレッドセーフですが、並列処理には向きません。
/// 並列処理を行いたい場合は `ParallelJTalk` を使用してください。
#[pyclass(name = "OpenJTalk", module = "haqumei")]
struct PyOpenJTalk {
    inner: Mutex<OpenJTalk>,
}

#[pymethods]
impl PyOpenJTalk {
    /// 新しいOpenJTalkインスタンスを初期化します。
    #[new]
    fn new() -> PyResult<Self> {
        let inner = OpenJTalk::new().map_err(to_py_err)?;
        Ok(Self {
            inner: Mutex::new(inner),
        })
    }

    /// 既存のDictionaryオブジェクトからインスタンスを作成します。
    /// 辞書データを共有するため、Mecab 側の処理を走らせて Minor Page Fault するオーバーヘッドがありません。
    #[staticmethod]
    fn from_dictionary(dict: &PyDictionary) -> PyResult<Self> {
        let inner = OpenJTalk::from_dictonary(dict.inner.clone()).map_err(to_py_err)?;
        Ok(Self {
            inner: Mutex::new(inner),
        })
    }

    /// 辞書をパスからロードします。
    #[staticmethod]
    fn from_path(dict_dir: &str, user_dict: Option<&str>) -> PyResult<Self> {
        let inner = OpenJTalk::from_path(dict_dir, user_dict).map_err(to_py_err)?;
        Ok(Self {
            inner: Mutex::new(inner),
        })
    }

    /// テキストを音素リストに変換します。
    fn g2p(&self, text: &str) -> PyResult<Vec<String>> {
        self.inner.lock().unwrap().g2p(text).map_err(to_py_err)
    }

    /// テキストをカタカナ読みに変換します。
    fn g2p_kana(&self, text: &str) -> PyResult<String> {
        self.inner.lock().unwrap().g2p_kana(text).map_err(to_py_err)
    }

    /// 単語単位で分割された音素リストを取得します。
    fn g2p_per_word(&self, text: &str) -> PyResult<Vec<Vec<String>>> {
        self.inner
            .lock()
            .unwrap()
            .g2p_per_word(text)
            .map_err(to_py_err)
    }

    /// 単語と音素のマッピング情報を取得します。
    fn g2p_mapping(&self, text: &str) -> PyResult<Vec<PyWordPhonemeMap>> {
        let mut guard = self.inner.lock().unwrap();
        let mapping = guard.g2p_mapping(text).map_err(to_py_err)?;
        Ok(mapping.into_iter().map(PyWordPhonemeMap::from).collect())
    }

    /// 詳細な特徴量 (NJDFeature) を取得します。
    fn run_frontend(&self, text: &str) -> PyResult<Vec<PyNjdFeature>> {
        let mut guard = self.inner.lock().unwrap();
        let features = guard.run_frontend(text).map_err(to_py_err)?;
        Ok(features.iter().map(PyNjdFeature::from).collect())
    }
}

/// pyopenjtalk-plus が行う精度向上のための処理を部分的に行います。
///
/// 内部で状態を持つためスレッドセーフになるよう排他制御が行われます。
#[pyclass(name = "Haqumei", module = "haqumei")]
struct PyHaqumei {
    inner: Mutex<Haqumei>,
}

#[pymethods]
impl PyHaqumei {
    #[new]
    fn new() -> PyResult<Self> {
        let inner = Haqumei::new().map_err(to_py_err)?;
        Ok(Self {
            inner: Mutex::new(inner),
        })
    }

    /// テキストを音素リストに変換します。
    fn g2p(&self, text: &str) -> PyResult<Vec<String>> {
        self.inner.lock().unwrap().g2p(text).map_err(to_py_err)
    }

    /// テキストをカタカナ読みに変換します。
    fn g2p_kana(&self, text: &str) -> PyResult<String> {
        self.inner.lock().unwrap().g2p_kana(text).map_err(to_py_err)
    }

    /// 単語単位で分割された音素リストを取得します。
    fn g2p_per_word(&self, text: &str) -> PyResult<Vec<Vec<String>>> {
        self.inner
            .lock()
            .unwrap()
            .g2p_per_word(text)
            .map_err(to_py_err)
    }

    /// 単語と音素のマッピング情報を取得します。
    fn g2p_mapping(&self, text: &str) -> PyResult<Vec<PyWordPhonemeMap>> {
        let mut guard = self.inner.lock().unwrap();
        let result = guard.g2p_mapping(text).map_err(to_py_err)?;
        Ok(result.into_iter().map(PyWordPhonemeMap::from).collect())
    }

    /// 詳細な特徴量 (NJDFeature) を取得します。
    fn run_frontend(&self, text: &str) -> PyResult<Vec<PyNjdFeature>> {
        let mut guard = self.inner.lock().unwrap();
        let features = guard.run_frontend(text).map_err(to_py_err)?;
        Ok(features.iter().map(PyNjdFeature::from).collect())
    }

    /// フルコンテキストラベルを抽出します。
    fn extract_fullcontext(&self, text: &str) -> PyResult<Vec<String>> {
        self.inner
            .lock()
            .unwrap()
            .extract_fullcontext(text)
            .map_err(to_py_err)
    }
}

/// OpenJTalkの並列処理用ラッパー。
///
/// 内部状態を持たず、リクエストごとに軽量なワーカーを生成するため、
/// マルチスレッド環境で GILを解放して
/// 効率的に動作します。大量のテキスト処理に適しています。
#[pyclass(name = "ParallelJTalk", module = "haqumei")]
struct PyParallelJTalk {
    inner: ParallelJTalk,
}

#[pymethods]
impl PyParallelJTalk {
    #[new]
    fn new() -> PyResult<Self> {
        let inner = ParallelJTalk::new().map_err(to_py_err)?;
        Ok(Self { inner })
    }

    /// 複数のテキストを並列処理で音素変換します。
    ///
    /// GILを解放してRayonによる並列処理を行います。
    fn g2p(&self, py: Python<'_>, texts: Vec<String>) -> PyResult<Vec<Vec<String>>> {
        py.detach(|| self.inner.g2p(&texts).map_err(to_py_err))
    }

    /// 複数のテキストを並列処理でカタカナ変換します。
    fn g2p_kana(&self, py: Python<'_>, texts: Vec<String>) -> PyResult<Vec<String>> {
        py.detach(|| self.inner.g2p_kana(&texts).map_err(to_py_err))
    }

    /// 複数のテキストを並列処理で単語ごとの音素リストに変換します。
    fn g2p_per_word(&self, py: Python<'_>, texts: Vec<String>) -> PyResult<Vec<Vec<Vec<String>>>> {
        py.detach(|| self.inner.g2p_per_word(&texts).map_err(to_py_err))
    }

    /// 複数のテキストを並列処理で単語マッピング情報に変換します。
    fn g2p_mapping(
        &self,
        py: Python<'_>,
        texts: Vec<String>,
    ) -> PyResult<Vec<Vec<PyWordPhonemeMap>>> {
        let results = py.detach(|| self.inner.g2p_mapping(&texts).map_err(to_py_err))?;

        Ok(results
            .into_iter()
            .map(|inner_vec| {
                inner_vec
                    .into_iter()
                    .map(PyWordPhonemeMap::from)
                    .collect()
            })
            .collect())
    }

    /// 複数のテキストを並列処理で特徴量抽出します。
    fn run_frontend(
        &self,
        py: Python<'_>,
        texts: Vec<String>,
    ) -> PyResult<Vec<Vec<PyNjdFeature>>> {
        let results = py.detach(|| self.inner.run_frontend(&texts).map_err(to_py_err))?;

        Ok(results
            .into_iter()
            .map(|inner_vec| inner_vec.iter().map(PyNjdFeature::from).collect())
            .collect())
    }
}

/// OpenJTalk で使用されるグローバル辞書を更新します (設定します)。
///
/// この関数を呼び出した後、新たに `OpenJTalk::new()` を呼び出す際には、この辞書が使用されるようになります。
/// 既存のインスタンスについては、次のメソッド呼び出し時に新しい辞書に更新されます。
#[pyfunction]
fn update_global_dictionary(dict: &PyDictionary) {
    ::haqumei::open_jtalk::update_global_dictionary(dict.inner.clone());
}

/// グローバル辞書からユーザー辞書を解除します。
#[pyfunction]
fn unset_user_dictionary() -> PyResult<()> {
    ::haqumei::open_jtalk::unset_user_dictionary().map_err(to_py_err)
}

#[pymodule]
fn haqumei(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyHaqumei>()?;
    m.add_class::<PyOpenJTalk>()?;
    m.add_class::<PyParallelJTalk>()?;
    m.add_class::<PyNjdFeature>()?;
    m.add_class::<PyWordPhonemeMap>()?;
    m.add_class::<PyDictionary>()?;

    m.add_function(wrap_pyfunction!(update_global_dictionary, m)?)?;
    m.add_function(wrap_pyfunction!(unset_user_dictionary, m)?)?;
    Ok(())
}