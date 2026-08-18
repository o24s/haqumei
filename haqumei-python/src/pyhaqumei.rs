use pyo3::pymethods;

use ::haqumei::{
    Haqumei, HaqumeiOptions,
    phoneme::{PhonemeVecExt, PhonemeVecVecExt},
    utils::default_is_non_pause_symbol,
};
use pyo3::prelude::*;
use std::sync::Mutex;

use crate::{
    IuPronunciation, PyHaqumei, PyMecabMorph, PyNjdFeature, UnicodeNormalization,
    prosody::PyProsodyFormat, to_py_err, word_phoneme::PyWordPhonemeProsody,
};
use crate::{
    jlabel::PyLabel,
    word_phoneme::{PyWordPhonemeDetail, PyWordPhonemeMap, PyWordPhonemePair},
};

#[pymethods]
impl PyHaqumei {
    #[allow(clippy::too_many_arguments)]
    #[new]
    #[pyo3(signature = (
        normalize_unicode = UnicodeNormalization::None_,
        use_read_as_pron = false,
        revert_long_vowels = false,
        revert_yotsugana = false,
        normalize_iu = IuPronunciation::None_,
        modify_filler_accent = true,
        predict_nani = true,
        predict_kana_english = true,
        use_unidic_yomi = false,
        retreat_acc_nuc = true,
        modify_acc_after_chaining = true,
        process_odoriji = true,
        use_allophones = false,
        split_n_allophones = false,
        split_n_before_palatal_affricate = false,
        split_n_before_r = false,
        split_q_allophones = false,
        enable_final_glottal_stop = false,
    ))]
    fn new(
        normalize_unicode: UnicodeNormalization,
        use_read_as_pron: bool,
        revert_long_vowels: bool,
        revert_yotsugana: bool,
        normalize_iu: IuPronunciation,
        modify_filler_accent: bool,
        predict_nani: bool,
        predict_kana_english: bool,
        use_unidic_yomi: bool,
        retreat_acc_nuc: bool,
        modify_acc_after_chaining: bool,
        process_odoriji: bool,
        use_allophones: bool,
        split_n_allophones: bool,
        split_n_before_palatal_affricate: bool,
        split_n_before_r: bool,
        split_q_allophones: bool,
        enable_final_glottal_stop: bool,
    ) -> PyResult<Self> {
        let options = HaqumeiOptions {
            normalize_unicode: match normalize_unicode {
                UnicodeNormalization::None_ => ::haqumei::UnicodeNormalization::None,
                UnicodeNormalization::Nfc => ::haqumei::UnicodeNormalization::Nfc,
                UnicodeNormalization::Nfkc => ::haqumei::UnicodeNormalization::Nfkc,
            },
            use_read_as_pron,
            revert_long_vowels,
            revert_yotsugana,
            normalize_iu: match normalize_iu {
                IuPronunciation::Iu => Some(::haqumei::IuPronunciation::Iu),
                IuPronunciation::Yuu => Some(::haqumei::IuPronunciation::Yuu),
                IuPronunciation::KanjiIu => Some(::haqumei::IuPronunciation::KanjiIu),
                IuPronunciation::KanjiYuu => Some(::haqumei::IuPronunciation::KanjiYuu),
IuPronunciation::YuuBase => Some(::haqumei::IuPronunciation::YuuBase),
                IuPronunciation::KanjiYuuBase => {
                    Some(::haqumei::IuPronunciation::KanjiYuuBase)
                }
                IuPronunciation::None_ => None,
            },
            modify_filler_accent,
            predict_nani,
            predict_kana_english,
            use_unidic_yomi,
            retreat_acc_nuc,
            modify_acc_after_chaining,
            process_odoriji,
            is_non_pause_symbol: default_is_non_pause_symbol,
            use_allophones,
            split_n_allophones,
            split_n_before_palatal_affricate,
            split_n_before_r,
            split_q_allophones,
            enable_final_glottal_stop,
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

    fn extract_fullcontext(&self, text: &str) -> PyResult<Vec<PyLabel>> {
        self.inner
            .lock()
            .unwrap()
            .extract_fullcontext(text)
            .map_err(crate::to_py_err)
            .map(|labels| labels.into_iter().map(PyLabel::from).collect())
    }

    fn extract_fullcontext_batch(
        &self,
        py: Python<'_>,
        texts: Vec<String>,
    ) -> PyResult<Vec<Vec<PyLabel>>> {
        py.detach(|| {
            self.inner
                .lock()
                .unwrap()
                .extract_fullcontext_batch(&texts)
                .map_err(crate::to_py_err)
                .map(|batch| {
                    batch
                        .into_iter()
                        .map(|l| l.into_iter().map(PyLabel::from).collect())
                        .collect()
                })
        })
    }

    fn extract_fullcontext_string(&self, text: &str) -> PyResult<Vec<String>> {
        self.inner
            .lock()
            .unwrap()
            .extract_fullcontext_string(text)
            .map_err(crate::to_py_err)
    }

    fn extract_fullcontext_string_batch(
        &self,
        py: Python<'_>,
        texts: Vec<String>,
    ) -> PyResult<Vec<Vec<String>>> {
        py.detach(|| {
            self.inner
                .lock()
                .unwrap()
                .extract_fullcontext_string_batch(&texts)
                .map_err(crate::to_py_err)
        })
    }

    fn g2p(&self, text: &str) -> PyResult<Vec<&'static str>> {
        self.inner
            .lock()
            .unwrap()
            .g2p(text)
            .map_err(to_py_err)
            .map(|p| p.into_strs())
    }

    fn g2p_batch(&self, py: Python<'_>, texts: Vec<String>) -> PyResult<Vec<Vec<&'static str>>> {
        py.detach(|| {
            self.inner
                .lock()
                .unwrap()
                .g2p_batch(&texts)
                .map_err(to_py_err)
                .map(|p| p.into_strs())
        })
    }

    fn g2p_detailed(&self, text: &str) -> PyResult<Vec<&'static str>> {
        self.inner
            .lock()
            .unwrap()
            .g2p_detailed(text)
            .map_err(to_py_err)
            .map(|p| p.into_strs())
    }

    fn g2p_detailed_batch(
        &self,
        py: Python<'_>,
        texts: Vec<String>,
    ) -> PyResult<Vec<Vec<&'static str>>> {
        py.detach(|| {
            self.inner
                .lock()
                .unwrap()
                .g2p_detailed_batch(&texts)
                .map_err(to_py_err)
                .map(|p| p.into_strs())
        })
    }

    fn g2k(&self, text: &str) -> PyResult<String> {
        self.inner.lock().unwrap().g2k(text).map_err(to_py_err)
    }

    fn g2k_batch(&self, py: Python<'_>, texts: Vec<String>) -> PyResult<Vec<String>> {
        py.detach(|| {
            self.inner
                .lock()
                .unwrap()
                .g2k_batch(&texts)
                .map_err(to_py_err)
        })
    }

    fn g2k_per_word(&self, text: &str) -> PyResult<Vec<String>> {
        self.inner
            .lock()
            .unwrap()
            .g2k_per_word(text)
            .map_err(to_py_err)
    }

    fn g2k_per_word_batch(&self, py: Python<'_>, texts: Vec<String>) -> PyResult<Vec<Vec<String>>> {
        py.detach(|| {
            self.inner
                .lock()
                .unwrap()
                .g2k_per_word_batch(&texts)
                .map_err(to_py_err)
        })
    }

    #[pyo3(signature = (text, format = PyProsodyFormat::Default))]
    fn g2p_prosody(&self, text: &str, format: PyProsodyFormat) -> PyResult<Vec<String>> {
        self.inner
            .lock()
            .unwrap()
            .g2p_prosody_with_options(text, format.into())
            .map_err(to_py_err)
    }

    #[pyo3(signature = (texts, format = PyProsodyFormat::Default))]
    fn g2p_prosody_batch(
        &self,
        py: Python<'_>,
        texts: Vec<String>,
        format: PyProsodyFormat,
    ) -> PyResult<Vec<Vec<String>>> {
        py.detach(|| {
            self.inner
                .lock()
                .unwrap()
                .g2p_prosody_with_options_batch(&texts, format.into())
                .map_err(to_py_err)
        })
    }

    fn g2p_per_word(&self, text: &str) -> PyResult<Vec<Vec<&'static str>>> {
        self.inner
            .lock()
            .unwrap()
            .g2p_per_word(text)
            .map_err(to_py_err)
            .map(|p| p.into_strs())
    }

    fn g2p_per_word_batch(
        &self,
        py: Python<'_>,
        texts: Vec<String>,
    ) -> PyResult<Vec<Vec<Vec<&'static str>>>> {
        py.detach(|| {
            self.inner
                .lock()
                .unwrap()
                .g2p_per_word_batch(&texts)
                .map_err(to_py_err)
                .map(|p| p.iter().map(|p| p.to_strs()).collect())
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

    fn g2p_mapping_prosody(&self, text: &str) -> PyResult<Vec<PyWordPhonemeProsody>> {
        self.inner
            .lock()
            .unwrap()
            .g2p_mapping_prosody(text)
            .map_err(crate::to_py_err)
            .map(|mapping| {
                mapping
                    .into_iter()
                    .map(PyWordPhonemeProsody::from)
                    .collect()
            })
    }

    fn g2p_mapping_prosody_batch(
        &self,
        py: Python<'_>,
        texts: Vec<String>,
    ) -> PyResult<Vec<Vec<PyWordPhonemeProsody>>> {
        py.detach(|| {
            self.inner
                .lock()
                .unwrap()
                .g2p_mapping_prosody_batch(&texts)
                .map_err(crate::to_py_err)
                .map(|batch| {
                    batch
                        .into_iter()
                        .map(|m| m.into_iter().map(PyWordPhonemeProsody::from).collect())
                        .collect()
                })
        })
    }
}
