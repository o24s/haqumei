//! `haqumei::candidates` の型を Python のクラスに写す。
//!
//! `#[pyclass]` は型引数のある構造体に付けられないので、`Candidates<T>` の `T` を
//! `WordPhonemeMap` / `WordPhonemeDetail` / `WordPhonemeProsody` に置いた 3 通りへ
//! 展開し、マクロで並べてある。

use ::haqumei::{
    Candidate, CandidateAlternative, CandidateBranch, CandidateOptions, CandidateReading,
    Candidates, WordPhonemeDetail, WordPhonemeMap, WordPhonemeProsody,
};
use pyo3::prelude::*;

use crate::word_phoneme::{PyWordPhonemeDetail, PyWordPhonemeMap, PyWordPhonemeProsody};

#[pyclass(name = "CandidateOptions", module = "haqumei", get_all, from_py_object)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PyCandidateOptions {
    pub max_delta: i64,
    pub max_alternatives_per_branch: usize,
    pub max_candidates: usize,
    pub branch_on_unknown_words: bool,
}

#[pymethods]
impl PyCandidateOptions {
    #[new]
    #[pyo3(signature = (
        *,
        max_delta = 2000,
        max_alternatives_per_branch = 4,
        max_candidates = 32,
        branch_on_unknown_words = false,
    ))]
    fn new(
        max_delta: i64,
        max_alternatives_per_branch: usize,
        max_candidates: usize,
        branch_on_unknown_words: bool,
    ) -> Self {
        Self {
            max_delta,
            max_alternatives_per_branch,
            max_candidates,
            branch_on_unknown_words,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "CandidateOptions(max_delta={}, max_alternatives_per_branch={}, max_candidates={}, \
             branch_on_unknown_words={})",
            self.max_delta,
            self.max_alternatives_per_branch,
            self.max_candidates,
            self.branch_on_unknown_words,
        )
    }
}

impl From<PyCandidateOptions> for CandidateOptions {
    fn from(o: PyCandidateOptions) -> Self {
        Self {
            max_delta: o.max_delta,
            max_alternatives_per_branch: o.max_alternatives_per_branch,
            max_candidates: o.max_candidates,
            branch_on_unknown_words: o.branch_on_unknown_words,
        }
    }
}

#[pyclass(
    name = "CandidateReading",
    module = "haqumei",
    get_all,
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyCandidateReading {
    pub surface: String,
    pub char_span: (usize, usize),
    pub pron: String,
    pub feature: String,
    pub delta: i64,
    pub left_id: u16,
    pub right_id: u16,
    pub word_cost: i16,
    pub is_unknown: bool,
}

#[pymethods]
impl PyCandidateReading {
    fn __repr__(&self) -> String {
        format!(
            "CandidateReading(surface={:?}, char_span={:?}, pron={:?}, delta={})",
            self.surface, self.char_span, self.pron, self.delta,
        )
    }

    fn __eq__(&self, other: &Self) -> bool {
        self == other
    }
}

impl From<CandidateReading> for PyCandidateReading {
    fn from(r: CandidateReading) -> Self {
        Self {
            surface: r.surface,
            char_span: (r.char_span.start, r.char_span.end),
            pron: r.pron,
            feature: r.feature,
            delta: r.delta,
            left_id: r.left_id,
            right_id: r.right_id,
            word_cost: r.word_cost,
            is_unknown: r.is_unknown,
        }
    }
}

#[pyclass(
    name = "CandidateAlternative",
    module = "haqumei",
    get_all,
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyCandidateAlternative {
    pub nodes: Vec<PyCandidateReading>,
    pub delta: i64,
}

#[pymethods]
impl PyCandidateAlternative {
    fn pron(&self) -> String {
        self.nodes.iter().map(|n| n.pron.as_str()).collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "CandidateAlternative(pron={:?}, nodes={}, delta={})",
            self.pron(),
            self.nodes.len(),
            self.delta,
        )
    }

    fn __eq__(&self, other: &Self) -> bool {
        self == other
    }
}

impl From<CandidateAlternative> for PyCandidateAlternative {
    fn from(a: CandidateAlternative) -> Self {
        Self {
            nodes: a.nodes.into_iter().map(Into::into).collect(),
            delta: a.delta,
        }
    }
}

#[pyclass(
    name = "CandidateBranch",
    module = "haqumei",
    get_all,
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyCandidateBranch {
    pub char_span: (usize, usize),
    pub surface: String,
    pub alternatives: Vec<PyCandidateAlternative>,
}

#[pymethods]
impl PyCandidateBranch {
    fn __repr__(&self) -> String {
        format!(
            "CandidateBranch(char_span={:?}, surface={:?}, alternatives={:?})",
            self.char_span,
            self.surface,
            self.alternatives
                .iter()
                .map(PyCandidateAlternative::pron)
                .collect::<Vec<_>>(),
        )
    }

    fn __eq__(&self, other: &Self) -> bool {
        self == other
    }
}

impl From<CandidateBranch> for PyCandidateBranch {
    fn from(b: CandidateBranch) -> Self {
        Self {
            char_span: (b.char_span.start, b.char_span.end),
            surface: b.surface,
            alternatives: b.alternatives.into_iter().map(Into::into).collect(),
        }
    }
}

macro_rules! py_candidates {
    ($cand:ident, $cands:ident, $cand_name:literal, $cands_name:literal, $py_word:ty, $word:ty) => {
        #[pyclass(name = $cand_name, module = "haqumei", get_all, skip_from_py_object)]
        #[derive(Clone)]
        pub struct $cand {
            pub words: Vec<$py_word>,
            pub delta: i64,
            pub choices: Vec<usize>,
        }

        #[pymethods]
        impl $cand {
            fn __repr__(&self) -> String {
                format!(
                    concat!($cand_name, "(words={}, delta={}, choices={:?})"),
                    self.words.len(),
                    self.delta,
                    self.choices,
                )
            }
        }

        impl From<Candidate<$word>> for $cand {
            fn from(c: Candidate<$word>) -> Self {
                Self {
                    words: c.words.into_iter().map(<$py_word>::from).collect(),
                    delta: c.delta,
                    choices: c.choices,
                }
            }
        }

        #[pyclass(name = $cands_name, module = "haqumei", get_all, skip_from_py_object)]
        #[derive(Clone)]
        pub struct $cands {
            pub text: String,
            pub branches: Vec<PyCandidateBranch>,
            pub candidates: Vec<$cand>,
        }

        #[pymethods]
        impl $cands {
            fn __repr__(&self) -> String {
                format!(
                    concat!($cands_name, "(text={:?}, branches={}, candidates={})"),
                    self.text,
                    self.branches.len(),
                    self.candidates.len(),
                )
            }

            fn __len__(&self) -> usize {
                self.candidates.len()
            }
        }

        impl From<Candidates<$word>> for $cands {
            fn from(c: Candidates<$word>) -> Self {
                Self {
                    text: c.text,
                    branches: c.branches.into_iter().map(Into::into).collect(),
                    candidates: c.candidates.into_iter().map(<$cand>::from).collect(),
                }
            }
        }
    };
}

py_candidates!(
    PyCandidate,
    PyCandidates,
    "Candidate",
    "Candidates",
    PyWordPhonemeMap,
    WordPhonemeMap
);
py_candidates!(
    PyCandidateDetail,
    PyCandidatesDetail,
    "CandidateDetail",
    "CandidatesDetail",
    PyWordPhonemeDetail,
    WordPhonemeDetail
);
py_candidates!(
    PyCandidateProsody,
    PyCandidatesProsody,
    "CandidateProsody",
    "CandidatesProsody",
    PyWordPhonemeProsody,
    WordPhonemeProsody
);
