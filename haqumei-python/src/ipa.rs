use ::haqumei::{
    IpaBoundary, IpaPhone, IpaToken, IpaTokenProsody, ProsodicIpa, SpecialPhone, WordIpaMap,
    WordIpaProsody,
};
use pyo3::prelude::*;

use crate::prosody::PyPitchAccent;

#[pyclass(name = "IpaPhone", module = "haqumei", get_all, skip_from_py_object)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PyIpaPhone {
    pub symbol: &'static str,
}

impl From<IpaPhone> for PyIpaPhone {
    fn from(phone: IpaPhone) -> Self {
        Self {
            symbol: phone.as_str(),
        }
    }
}

#[pymethods]
impl PyIpaPhone {
    fn __str__(&self) -> &'static str {
        self.symbol
    }

    fn __repr__(&self) -> String {
        format!("IpaPhone(symbol={:?})", self.symbol)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self == other
    }
}

#[pyclass(
    name = "SpecialPhone",
    module = "haqumei",
    get_all,
    skip_from_py_object
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PySpecialPhone {
    pub symbol: &'static str,
}

impl From<SpecialPhone> for PySpecialPhone {
    fn from(label: SpecialPhone) -> Self {
        Self {
            symbol: label.as_str(),
        }
    }
}

#[pymethods]
impl PySpecialPhone {
    fn __str__(&self) -> &'static str {
        self.symbol
    }

    fn __repr__(&self) -> String {
        format!("SpecialPhone(symbol={:?})", self.symbol)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self == other
    }
}

#[pyclass(name = "IpaBoundary", module = "haqumei", eq, eq_int, from_py_object)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyIpaBoundary {
    AccentPhrase = 0,
    Pause = 1,
    Interrogative = 2,
    Exclamatory = 3,
}

impl From<IpaBoundary> for PyIpaBoundary {
    fn from(boundary: IpaBoundary) -> Self {
        match boundary {
            IpaBoundary::AccentPhrase => Self::AccentPhrase,
            IpaBoundary::Pause => Self::Pause,
            IpaBoundary::Interrogative => Self::Interrogative,
            IpaBoundary::Exclamatory => Self::Exclamatory,
            _ => unreachable!("すべての IpaBoundary variant を Python 側へ公開する"),
        }
    }
}

#[pyclass(name = "IpaToken", module = "haqumei", get_all, skip_from_py_object)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyIpaToken {
    pub kind: &'static str,
    pub phone: Option<PyIpaPhone>,
    pub special: Option<PySpecialPhone>,
    pub symbol: &'static str,
}

impl From<IpaToken> for PyIpaToken {
    fn from(token: IpaToken) -> Self {
        let symbol = token.as_str();
        match token {
            IpaToken::Phone(phone) => Self {
                kind: "phone",
                phone: Some(phone.into()),
                special: None,
                symbol,
            },
            IpaToken::Unknown => Self {
                kind: "unknown",
                phone: None,
                special: None,
                symbol,
            },
            IpaToken::Special(label) => Self {
                kind: "special",
                phone: None,
                special: Some(label.into()),
                symbol,
            },
            _ => unreachable!("すべての IpaToken variant を Python 側へ公開する"),
        }
    }
}

#[pymethods]
impl PyIpaToken {
    fn __str__(&self) -> &'static str {
        self.symbol
    }

    fn __repr__(&self) -> String {
        format!(
            "IpaToken(kind={:?}, phone={:?}, special={:?}, symbol={:?})",
            self.kind, self.phone, self.special, self.symbol,
        )
    }

    fn __eq__(&self, other: &Self) -> bool {
        self == other
    }
}

#[pyclass(
    name = "IpaTokenProsody",
    module = "haqumei",
    get_all,
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyIpaTokenProsody {
    pub kind: &'static str,
    pub pitch: Option<PyPitchAccent>,
    pub boundary: Option<PyIpaBoundary>,
}

impl From<IpaTokenProsody> for PyIpaTokenProsody {
    fn from(item: IpaTokenProsody) -> Self {
        match item {
            IpaTokenProsody::Pitch(pitch) => Self {
                kind: "pitch",
                pitch: pitch.map(Into::into),
                boundary: None,
            },
            IpaTokenProsody::Boundary(boundary) => Self {
                kind: "boundary",
                pitch: None,
                boundary: Some(boundary.into()),
            },
            _ => unreachable!("すべての IpaTokenProsody variant を Python 側へ公開する"),
        }
    }
}

#[pymethods]
impl PyIpaTokenProsody {
    fn __repr__(&self) -> String {
        format!(
            "IpaTokenProsody(kind={:?}, pitch={:?}, boundary={:?})",
            self.kind, self.pitch, self.boundary,
        )
    }

    fn __eq__(&self, other: &Self) -> bool {
        self == other
    }
}

#[pyclass(name = "ProsodicIpa", module = "haqumei", get_all, skip_from_py_object)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyProsodicIpa {
    pub kind: &'static str,
    pub token: Option<PyIpaToken>,
    pub prosody: Vec<PyIpaTokenProsody>,
    pub boundary: Option<PyIpaBoundary>,
}

impl From<ProsodicIpa> for PyProsodicIpa {
    fn from(item: ProsodicIpa) -> Self {
        match item {
            ProsodicIpa::Token { token, prosody } => Self {
                kind: "token",
                token: Some(token.into()),
                prosody: prosody.into_iter().map(Into::into).collect(),
                boundary: None,
            },
            ProsodicIpa::Boundary(boundary) => Self {
                kind: "boundary",
                token: None,
                prosody: Vec::new(),
                boundary: Some(boundary.into()),
            },
            _ => unreachable!("すべての ProsodicIpa variant を Python 側へ公開する"),
        }
    }
}

#[pymethods]
impl PyProsodicIpa {
    fn __repr__(&self) -> String {
        format!(
            "ProsodicIpa(kind={:?}, token={:?}, prosody={:?}, boundary={:?})",
            self.kind, self.token, self.prosody, self.boundary,
        )
    }

    fn __eq__(&self, other: &Self) -> bool {
        self == other
    }
}

#[pyclass(name = "WordIpaMap", module = "haqumei", get_all, skip_from_py_object)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyWordIpaMap {
    pub word: String,
    pub tokens: Vec<PyIpaToken>,
    pub is_unknown: bool,
    pub is_ignored: bool,
    pub char_span: (usize, usize),
}

impl From<WordIpaMap> for PyWordIpaMap {
    fn from(word: WordIpaMap) -> Self {
        Self {
            word: word.word,
            tokens: word.tokens.into_iter().map(Into::into).collect(),
            is_unknown: word.is_unknown,
            is_ignored: word.is_ignored,
            char_span: (word.char_span.start, word.char_span.end),
        }
    }
}

#[pymethods]
impl PyWordIpaMap {
    fn __repr__(&self) -> String {
        format!(
            "WordIpaMap(word={:?}, tokens={:?}, is_unknown={}, is_ignored={}, char_span={:?})",
            self.word, self.tokens, self.is_unknown, self.is_ignored, self.char_span,
        )
    }

    fn __eq__(&self, other: &Self) -> bool {
        self == other
    }
}

#[pyclass(
    name = "WordIpaProsody",
    module = "haqumei",
    get_all,
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyWordIpaProsody {
    pub word: String,
    pub tokens: Vec<PyProsodicIpa>,
    pub is_unknown: bool,
    pub is_ignored: bool,
    pub char_span: (usize, usize),
}

impl From<WordIpaProsody> for PyWordIpaProsody {
    fn from(word: WordIpaProsody) -> Self {
        Self {
            word: word.word,
            tokens: word.tokens.into_iter().map(Into::into).collect(),
            is_unknown: word.is_unknown,
            is_ignored: word.is_ignored,
            char_span: (word.char_span.start, word.char_span.end),
        }
    }
}

#[pymethods]
impl PyWordIpaProsody {
    fn __repr__(&self) -> String {
        format!(
            "WordIpaProsody(word={:?}, tokens={:?}, is_unknown={}, is_ignored={}, char_span={:?})",
            self.word, self.tokens, self.is_unknown, self.is_ignored, self.char_span,
        )
    }

    fn __eq__(&self, other: &Self) -> bool {
        self == other
    }
}
