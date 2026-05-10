#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// The structure representing a single line of HTS-style full-context label.
///
/// The parser from str, and the serializer to String are both implemented.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Label {
    /// Phoneme
    pub phoneme: Phoneme,
    /// A: Mora
    pub mora: Option<Mora>,
    /// B: Previous Word
    pub word_prev: Option<Word>,
    /// C: Current Word
    pub word_curr: Option<Word>,
    /// D: Next Word
    pub word_next: Option<Word>,
    /// E: Previous Accent Phrase
    pub accent_phrase_prev: Option<AccentPhrasePrevNext>,
    /// F: Current Accent Phrase
    pub accent_phrase_curr: Option<AccentPhraseCurrent>,
    /// G: Next Accent Phrase
    pub accent_phrase_next: Option<AccentPhrasePrevNext>,
    /// H: Previous Breath Group
    pub breath_group_prev: Option<BreathGroupPrevNext>,
    /// I: Current Breath Group
    pub breath_group_curr: Option<BreathGroupCurrent>,
    /// J: Next Breath Group
    pub breath_group_next: Option<BreathGroupPrevNext>,
    /// K: Utterance
    pub utterance: Utterance,
}

/// `Phoneme` field of full-context label.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Phoneme {
    /// P1: the phoneme identity before the previous phoneme
    pub p2: Option<String>,
    /// P2: the previous phoneme identity
    pub p1: Option<String>,
    /// P3: the current phoneme identity
    pub c: Option<String>,
    /// P4: the next phoneme identity
    pub n1: Option<String>,
    /// P5: the phoneme after the next phoneme identity
    pub n2: Option<String>,
}

/// `Mora` field of full-context label (`A` field).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Mora {
    /// A1: the difference between accent type and position of the current mora identity
    pub relative_accent_position: i8,
    /// A2: position of the current mora identity in the current accent phrase (forward)
    pub position_forward: u8,
    /// A3: position of the current mora identity in the current accent phrase (backward)
    pub position_backward: u8,
}

/// `Word` field of full-context label (`B`, `C`, and `D` field).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Word {
    /// B1/C1/D1: pos (part-of-speech) of the word
    pub pos: Option<u8>,
    /// B2/C2/D2: conjugation type of the word
    pub ctype: Option<u8>,
    /// B3/C3/D3: inflected forms of the word
    pub cform: Option<u8>,
}

/// `AccentPhrase` field of full-context label for current accent phrase (`F` field).
///
/// F4 is undefined.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AccentPhraseCurrent {
    /// F1: the number of moras in the current accent phrase
    pub mora_count: u8,
    /// F2: accent type in the current accent phrase
    pub accent_position: u8,
    /// F3: whether the current accent phrase interrogative or not
    pub is_interrogative: bool,
    /// F5: position of the current accent phrase identity in the current breath group by the accent phrase (forward)
    pub accent_phrase_position_forward: u8,
    /// F6: position of the current accent phrase identity in the current breath group by the accent phrase (backward)
    pub accent_phrase_position_backward: u8,
    /// F7: position of the current accent phrase identity in the current breath group by the mora (forward)
    pub mora_position_forward: u8,
    /// F8: position of the current accent phrase identity in the current breath group by the mora (backward)
    pub mora_position_backward: u8,

    /// F4: an out-of-spec extension
    pub is_exclamatory: bool,
}

/// `AccentPhrase` field of full-context label for previous or next accent phrase (`E` and `G` field).
///
/// E4/G4 is undefined.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AccentPhrasePrevNext {
    /// E1/G1: the number of moras in the accent phrase
    pub mora_count: u8,
    /// E2/G2: accent type in the accent phrase
    pub accent_position: u8,
    /// E3/G3: whether the accent phrase interrogative or not
    pub is_interrogative: bool,
    /// E5/G5: whether pause insertion or not in between the accent phrase and the current accent phrase
    ///
    /// <div class="warning">
    ///
    /// The logic of this field is inverted from the E5/G5 of full-context label:
    /// "1" is false and "0" is true.
    ///
    /// </div>
    pub is_pause_insertion: Option<bool>,

    /// E4/G4: an out-of-spec extension
    pub is_exclamatory: bool,
}

/// `BreathGroup` field of full-context label for current breath group (`I` field).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BreathGroupCurrent {
    /// I1: the number of accent phrases in the current breath group
    pub accent_phrase_count: u8,
    /// I2: the number of moras in the current breath group
    pub mora_count: u8,
    /// I3: position of the current breath group identity by breath group (forward)
    pub breath_group_position_forward: u8,
    /// I4: position of the current breath group identity by breath group (backward)
    pub breath_group_position_backward: u8,
    /// I5: position of the current breath group identity by accent phrase (forward)
    pub accent_phrase_position_forward: u8,
    /// I6: position of the current breath group identity by accent phrase (backward)
    pub accent_phrase_position_backward: u8,
    /// I7: position of the current breath group identity by mora (forward)
    pub mora_position_forward: u8,
    /// I8: position of the current breath group identity by mora (backward)
    pub mora_position_backward: u8,
}

/// `BreathGroup` field of full-context label for previous or next breath group (`H` and `J` field).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BreathGroupPrevNext {
    /// H1/J1: the number of accent phrases in the breath group
    pub accent_phrase_count: u8,
    /// H2/J2: the number of moras in the breath group
    pub mora_count: u8,
}

/// `Utterance` field of full-context label (`K` field).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Utterance {
    /// K1: the number of breath groups in this utterance
    pub breath_group_count: u8,
    /// K2: the number of accent phrases in this utterance
    pub accent_phrase_count: u8,
    /// K3: the number of moras in this utterance
    pub mora_count: u8,
}
