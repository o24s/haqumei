use ::haqumei::prosody::{PitchAccent, ProsodicPhoneme};
use haqumei::ProsodyFormat;
use pyo3::prelude::*;

#[pyclass(name = "PitchAccent", module = "haqumei", eq, eq_int, from_py_object)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyPitchAccent {
    Low,
    High,
}

#[pyclass(name = "ProsodyFormat", module = "haqumei", eq, eq_int, from_py_object)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyProsodyFormat {
    Default = 0,
    Prefix = 1,
    Numeric = 2,
}

impl From<PyProsodyFormat> for ProsodyFormat {
    fn from(format: PyProsodyFormat) -> Self {
        match format {
            PyProsodyFormat::Default => ProsodyFormat::Default,
            PyProsodyFormat::Prefix => ProsodyFormat::Prefix,
            PyProsodyFormat::Numeric => ProsodyFormat::Numeric,
        }
    }
}

#[pyclass(
    name = "ProsodicPhoneme",
    module = "haqumei",
    get_all,
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq)]
pub struct PyProsodicPhoneme {
    pub kind: &'static str,
    pub phoneme: Option<&'static str>,
    pub pitch: Option<PyPitchAccent>,
}

impl From<ProsodicPhoneme> for PyProsodicPhoneme {
    fn from(p: ProsodicPhoneme) -> Self {
        match p {
            ProsodicPhoneme::Phoneme { phoneme, pitch } => Self {
                kind: "phoneme",
                phoneme: Some(phoneme.as_str()),
                pitch: pitch.map(|p| match p {
                    PitchAccent::Low => PyPitchAccent::Low,
                    PitchAccent::High => PyPitchAccent::High,
                }),
            },
            ProsodicPhoneme::AccentPhraseBoundary => Self {
                kind: "accent_phrase_boundary",
                phoneme: None,
                pitch: None,
            },
            ProsodicPhoneme::Pause => Self {
                kind: "pause",
                phoneme: None,
                pitch: None,
            },
            ProsodicPhoneme::Interrogative => Self {
                kind: "interrogative",
                phoneme: None,
                pitch: None,
            },
            ProsodicPhoneme::Exclamatory => Self {
                kind: "exclamatory",
                phoneme: None,
                pitch: None,
            },
        }
    }
}
