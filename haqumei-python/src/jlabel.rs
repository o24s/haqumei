use pyo3::prelude::*;

use ::haqumei_jlabel::{
    AccentPhraseCurrent, AccentPhrasePrevNext, BreathGroupCurrent, BreathGroupPrevNext, Label,
    Mora, Phoneme as LabelPhoneme, Utterance, Word,
};

#[pyclass(name = "LabelPhoneme", module = "haqumei", get_all, from_py_object)]
#[derive(Clone)]
pub struct PyLabelPhoneme {
    pub p2: Option<String>,
    pub p1: Option<String>,
    pub c: Option<String>,
    pub n1: Option<String>,
    pub n2: Option<String>,
}

impl From<LabelPhoneme> for PyLabelPhoneme {
    fn from(p: LabelPhoneme) -> Self {
        Self {
            p2: p.p2,
            p1: p.p1,
            c: p.c,
            n1: p.n1,
            n2: p.n2,
        }
    }
}

#[pyclass(name = "Mora", module = "haqumei", get_all, from_py_object)]
#[derive(Clone)]
pub struct PyMora {
    pub relative_accent_position: i8,
    pub position_forward: u8,
    pub position_backward: u8,
}

impl From<Mora> for PyMora {
    fn from(m: Mora) -> Self {
        Self {
            relative_accent_position: m.relative_accent_position,
            position_forward: m.position_forward,
            position_backward: m.position_backward,
        }
    }
}

#[pyclass(name = "Word", module = "haqumei", get_all, from_py_object)]
#[derive(Clone)]
pub struct PyWord {
    pub pos: Option<u8>,
    pub ctype: Option<u8>,
    pub cform: Option<u8>,
}

impl From<Word> for PyWord {
    fn from(w: Word) -> Self {
        Self {
            pos: w.pos,
            ctype: w.ctype,
            cform: w.cform,
        }
    }
}

#[pyclass(
    name = "AccentPhraseCurrent",
    module = "haqumei",
    get_all,
    from_py_object
)]
#[derive(Clone)]
pub struct PyAccentPhraseCurrent {
    pub mora_count: u8,
    pub accent_position: u8,
    pub is_interrogative: bool,
    pub accent_phrase_position_forward: u8,
    pub accent_phrase_position_backward: u8,
    pub mora_position_forward: u8,
    pub mora_position_backward: u8,
    pub is_exclamatory: bool,
}

impl From<AccentPhraseCurrent> for PyAccentPhraseCurrent {
    fn from(a: AccentPhraseCurrent) -> Self {
        Self {
            mora_count: a.mora_count,
            accent_position: a.accent_position,
            is_interrogative: a.is_interrogative,
            accent_phrase_position_forward: a.accent_phrase_position_forward,
            accent_phrase_position_backward: a.accent_phrase_position_backward,
            mora_position_forward: a.mora_position_forward,
            mora_position_backward: a.mora_position_backward,
            is_exclamatory: a.is_exclamatory,
        }
    }
}

#[pyclass(
    name = "AccentPhrasePrevNext",
    module = "haqumei",
    get_all,
    from_py_object
)]
#[derive(Clone)]
pub struct PyAccentPhrasePrevNext {
    pub mora_count: u8,
    pub accent_position: u8,
    pub is_interrogative: bool,
    pub is_pause_insertion: Option<bool>,
    pub is_exclamatory: bool,
}

impl From<AccentPhrasePrevNext> for PyAccentPhrasePrevNext {
    fn from(a: AccentPhrasePrevNext) -> Self {
        Self {
            mora_count: a.mora_count,
            accent_position: a.accent_position,
            is_interrogative: a.is_interrogative,
            is_pause_insertion: a.is_pause_insertion,
            is_exclamatory: a.is_exclamatory,
        }
    }
}

#[pyclass(
    name = "BreathGroupCurrent",
    module = "haqumei",
    get_all,
    from_py_object
)]
#[derive(Clone)]
pub struct PyBreathGroupCurrent {
    pub accent_phrase_count: u8,
    pub mora_count: u8,
    pub breath_group_position_forward: u8,
    pub breath_group_position_backward: u8,
    pub accent_phrase_position_forward: u8,
    pub accent_phrase_position_backward: u8,
    pub mora_position_forward: u8,
    pub mora_position_backward: u8,
}

impl From<BreathGroupCurrent> for PyBreathGroupCurrent {
    fn from(b: BreathGroupCurrent) -> Self {
        Self {
            accent_phrase_count: b.accent_phrase_count,
            mora_count: b.mora_count,
            breath_group_position_forward: b.breath_group_position_forward,
            breath_group_position_backward: b.breath_group_position_backward,
            accent_phrase_position_forward: b.accent_phrase_position_forward,
            accent_phrase_position_backward: b.accent_phrase_position_backward,
            mora_position_forward: b.mora_position_forward,
            mora_position_backward: b.mora_position_backward,
        }
    }
}

#[pyclass(
    name = "BreathGroupPrevNext",
    module = "haqumei",
    get_all,
    from_py_object
)]
#[derive(Clone)]
pub struct PyBreathGroupPrevNext {
    pub accent_phrase_count: u8,
    pub mora_count: u8,
}

impl From<BreathGroupPrevNext> for PyBreathGroupPrevNext {
    fn from(b: BreathGroupPrevNext) -> Self {
        Self {
            accent_phrase_count: b.accent_phrase_count,
            mora_count: b.mora_count,
        }
    }
}

#[pyclass(name = "Utterance", module = "haqumei", get_all, from_py_object)]
#[derive(Clone)]
pub struct PyUtterance {
    pub breath_group_count: u8,
    pub accent_phrase_count: u8,
    pub mora_count: u8,
}

impl From<Utterance> for PyUtterance {
    fn from(u: Utterance) -> Self {
        Self {
            breath_group_count: u.breath_group_count,
            accent_phrase_count: u.accent_phrase_count,
            mora_count: u.mora_count,
        }
    }
}

#[pyclass(name = "Label", module = "haqumei", get_all, from_py_object)]
#[derive(Clone)]
pub struct PyLabel {
    pub phoneme: PyLabelPhoneme,
    pub mora: Option<PyMora>,
    pub word_prev: Option<PyWord>,
    pub word_curr: Option<PyWord>,
    pub word_next: Option<PyWord>,
    pub accent_phrase_prev: Option<PyAccentPhrasePrevNext>,
    pub accent_phrase_curr: Option<PyAccentPhraseCurrent>,
    pub accent_phrase_next: Option<PyAccentPhrasePrevNext>,
    pub breath_group_prev: Option<PyBreathGroupPrevNext>,
    pub breath_group_curr: Option<PyBreathGroupCurrent>,
    pub breath_group_next: Option<PyBreathGroupPrevNext>,
    pub utterance: PyUtterance,
}

impl From<Label> for PyLabel {
    fn from(l: Label) -> Self {
        Self {
            phoneme: l.phoneme.into(),
            mora: l.mora.map(Into::into),
            word_prev: l.word_prev.map(Into::into),
            word_curr: l.word_curr.map(Into::into),
            word_next: l.word_next.map(Into::into),
            accent_phrase_prev: l.accent_phrase_prev.map(Into::into),
            accent_phrase_curr: l.accent_phrase_curr.map(Into::into),
            accent_phrase_next: l.accent_phrase_next.map(Into::into),
            breath_group_prev: l.breath_group_prev.map(Into::into),
            breath_group_curr: l.breath_group_curr.map(Into::into),
            breath_group_next: l.breath_group_next.map(Into::into),
            utterance: l.utterance.into(),
        }
    }
}
