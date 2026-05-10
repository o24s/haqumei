use std::{num::ParseIntError, str::FromStr};

use crate::fullcontext_label::{
    AccentPhraseCurrent, AccentPhrasePrevNext, BreathGroupCurrent, BreathGroupPrevNext, Label,
    Mora, Phoneme, Utterance, Word,
};

/// Errors from jlabel parser.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    /// The required symbol was not found.
    #[error("Symbol not found: expected {0}")]
    SymbolNotFound(&'static str),
    /// The position was supposed to be integer, but failed to parse it as integer.
    #[error("Parse int error: {0}")]
    ParseIntError(#[from] ParseIntError),
    /// The position was supposed to be boolean (0 or 1), but failed to parse it as boolean.
    #[error("Parse bool error")]
    ParseBoolError,
    /// The position must always be undefined.
    #[error("Not undefined")]
    NotUndefined,
}

#[derive(Debug)]
struct LabelTokenizer<'a> {
    input: &'a str,
    index: usize,
}

impl<'a> LabelTokenizer<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, index: 0 }
    }

    fn until(&mut self, symbol: &'static str) -> Result<&'a str, ParseError> {
        match self.input[self.index..].find(symbol) {
            Some(i) => {
                let result = &self.input[self.index..(self.index + i)];
                self.index += i + symbol.len();
                Ok(result)
            }
            None => Err(ParseError::SymbolNotFound(symbol)),
        }
    }

    fn string_or_xx(input: &'a str) -> Option<String> {
        if input == "xx" {
            None
        } else {
            Some(input.to_string())
        }
    }

    fn parse_or_xx<T: FromStr>(input: &'a str) -> Result<Option<T>, T::Err> {
        if input == "xx" {
            Ok(None)
        } else {
            input.parse().map(Some)
        }
    }

    fn parse_bool_or_xx(input: &'a str) -> Result<Option<bool>, ParseError> {
        match input {
            "xx" => Ok(None),
            "0" => Ok(Some(false)),
            "1" => Ok(Some(true)),
            _ => Err(ParseError::ParseBoolError),
        }
    }

    /// `p1ˆp2-p3+p4=p5`
    fn p(&mut self) -> Result<Phoneme, ParseError> {
        let p1 = Self::string_or_xx(self.until("^")?);
        let p2 = Self::string_or_xx(self.until("-")?);
        let p3 = Self::string_or_xx(self.until("+")?);
        let p4 = Self::string_or_xx(self.until("=")?);
        let p5 = Self::string_or_xx(self.until("/A:")?);
        Ok(Phoneme {
            p2: p1,
            p1: p2,
            c: p3,
            n1: p4,
            n2: p5,
        })
    }

    /// `/A:a1+a2+a3`
    fn a(&mut self) -> Result<Option<Mora>, ParseError> {
        let a1 = Self::parse_or_xx(self.until("+")?)?;
        let a2 = Self::parse_or_xx(self.until("+")?)?;
        let a3 = Self::parse_or_xx(self.until("/B:")?)?;

        if let (Some(a1), Some(a2), Some(a3)) = (a1, a2, a3) {
            Ok(Some(Mora {
                relative_accent_position: a1,
                position_forward: a2,
                position_backward: a3,
            }))
        } else {
            Ok(None)
        }
    }

    /// `/B:b1-b2_b3`
    fn b(&mut self) -> Result<Option<Word>, ParseError> {
        let b1 = Self::parse_or_xx(self.until("-")?)?;
        let b2 = Self::parse_or_xx(self.until("_")?)?;
        let b3 = Self::parse_or_xx(self.until("/C:")?)?;

        if [b1, b2, b3].iter().all(Option::is_none) {
            Ok(None)
        } else {
            Ok(Some(Word {
                pos: b1,
                ctype: b2,
                cform: b3,
            }))
        }
    }

    /// `/C:c1_c2+c3`
    fn c(&mut self) -> Result<Option<Word>, ParseError> {
        let c1 = Self::parse_or_xx(self.until("_")?)?;
        let c2 = Self::parse_or_xx(self.until("+")?)?;
        let c3 = Self::parse_or_xx(self.until("/D:")?)?;

        if [c1, c2, c3].iter().all(Option::is_none) {
            Ok(None)
        } else {
            Ok(Some(Word {
                pos: c1,
                ctype: c2,
                cform: c3,
            }))
        }
    }

    /// `/D:d1+d2_d3`
    fn d(&mut self) -> Result<Option<Word>, ParseError> {
        let d1 = Self::parse_or_xx(self.until("+")?)?;
        let d2 = Self::parse_or_xx(self.until("_")?)?;
        let d3 = Self::parse_or_xx(self.until("/E:")?)?;

        if [d1, d2, d3].iter().all(Option::is_none) {
            Ok(None)
        } else {
            Ok(Some(Word {
                pos: d1,
                ctype: d2,
                cform: d3,
            }))
        }
    }

    /// `/E:e1_e2!e3_e4-e5`
    fn e(&mut self) -> Result<Option<AccentPhrasePrevNext>, ParseError> {
        let e1 = Self::parse_or_xx(self.until("_")?)?;
        let e2 = Self::parse_or_xx(self.until("!")?)?;
        let e3 = Self::parse_bool_or_xx(self.until("_")?)?;
        let e4 = Self::parse_bool_or_xx(self.until("-")?)?;
        let e5 = Self::parse_bool_or_xx(self.until("/F:")?)?;

        if let (Some(e1), Some(e2), Some(e3)) = (e1, e2, e3) {
            Ok(Some(AccentPhrasePrevNext {
                mora_count: e1,
                accent_position: e2,
                is_interrogative: e3,
                is_exclamatory: e4.unwrap_or(false),
                is_pause_insertion: e5.map(|e5| !e5),
            }))
        } else {
            Ok(None)
        }
    }

    /// `/F:f1_f2#f3_f4@f5_f6|f7_f8`
    fn f(&mut self) -> Result<Option<AccentPhraseCurrent>, ParseError> {
        let f1 = Self::parse_or_xx(self.until("_")?)?;
        let f2 = Self::parse_or_xx(self.until("#")?)?;
        let f3 = Self::parse_bool_or_xx(self.until("_")?)?;
        let f4 = Self::parse_bool_or_xx(self.until("@")?)?;
        let f5 = Self::parse_or_xx(self.until("_")?)?;
        let f6 = Self::parse_or_xx(self.until("|")?)?;
        let f7 = Self::parse_or_xx(self.until("_")?)?;
        let f8 = Self::parse_or_xx(self.until("/G:")?)?;

        if let (Some(f1), Some(f2), Some(f3), Some(f5), Some(f6), Some(f7), Some(f8)) =
            (f1, f2, f3, f5, f6, f7, f8)
        {
            Ok(Some(AccentPhraseCurrent {
                mora_count: f1,
                accent_position: f2,
                is_interrogative: f3,
                is_exclamatory: f4.unwrap_or(false),
                accent_phrase_position_forward: f5,
                accent_phrase_position_backward: f6,
                mora_position_forward: f7,
                mora_position_backward: f8,
            }))
        } else {
            Ok(None)
        }
    }

    /// `/G:g1_g2%g3_g4_g5`
    fn g(&mut self) -> Result<Option<AccentPhrasePrevNext>, ParseError> {
        let g1 = Self::parse_or_xx(self.until("_")?)?;
        let g2 = Self::parse_or_xx(self.until("%")?)?;
        let g3 = Self::parse_bool_or_xx(self.until("_")?)?;
        let g4 = Self::parse_bool_or_xx(self.until("_")?)?;
        let g5 = Self::parse_bool_or_xx(self.until("/H:")?)?;

        if let (Some(g1), Some(g2), Some(g3)) = (g1, g2, g3) {
            Ok(Some(AccentPhrasePrevNext {
                mora_count: g1,
                accent_position: g2,
                is_interrogative: g3,
                is_exclamatory: g4.unwrap_or(false),
                is_pause_insertion: g5.map(|g5| !g5),
            }))
        } else {
            Ok(None)
        }
    }

    /// `/H:h1_h2`
    fn h(&mut self) -> Result<Option<BreathGroupPrevNext>, ParseError> {
        let h1 = Self::parse_or_xx(self.until("_")?)?;
        let h2 = Self::parse_or_xx(self.until("/I:")?)?;

        if let (Some(h1), Some(h2)) = (h1, h2) {
            Ok(Some(BreathGroupPrevNext {
                accent_phrase_count: h1,
                mora_count: h2,
            }))
        } else {
            Ok(None)
        }
    }

    /// `/I:i1-i2@i3+i4&i5-i6|i7+i8`
    fn i(&mut self) -> Result<Option<BreathGroupCurrent>, ParseError> {
        let i1 = Self::parse_or_xx(self.until("-")?)?;
        let i2 = Self::parse_or_xx(self.until("@")?)?;
        let i3 = Self::parse_or_xx(self.until("+")?)?;
        let i4 = Self::parse_or_xx(self.until("&")?)?;
        let i5 = Self::parse_or_xx(self.until("-")?)?;
        let i6 = Self::parse_or_xx(self.until("|")?)?;
        let i7 = Self::parse_or_xx(self.until("+")?)?;
        let i8 = Self::parse_or_xx(self.until("/J:")?)?;

        if let (Some(i1), Some(i2), Some(i3), Some(i4), Some(i5), Some(i6), Some(i7), Some(i8)) =
            (i1, i2, i3, i4, i5, i6, i7, i8)
        {
            Ok(Some(BreathGroupCurrent {
                accent_phrase_count: i1,
                mora_count: i2,
                breath_group_position_forward: i3,
                breath_group_position_backward: i4,
                accent_phrase_position_forward: i5,
                accent_phrase_position_backward: i6,
                mora_position_forward: i7,
                mora_position_backward: i8,
            }))
        } else {
            Ok(None)
        }
    }

    /// `/J:j1_j2`
    fn j(&mut self) -> Result<Option<BreathGroupPrevNext>, ParseError> {
        let j1 = Self::parse_or_xx(self.until("_")?)?;
        let j2 = Self::parse_or_xx(self.until("/K:")?)?;

        if let (Some(j1), Some(j2)) = (j1, j2) {
            Ok(Some(BreathGroupPrevNext {
                accent_phrase_count: j1,
                mora_count: j2,
            }))
        } else {
            Ok(None)
        }
    }

    /// `/K:k1+k2-k3`
    fn k(&mut self) -> Result<Utterance, ParseError> {
        let k1 = self.until("+")?.parse()?;
        let k2 = self.until("-")?.parse()?;
        let k3 = self.input[self.index..].parse()?;

        Ok(Utterance {
            breath_group_count: k1,
            accent_phrase_count: k2,
            mora_count: k3,
        })
    }

    fn consume(mut self) -> Result<Label, ParseError> {
        Ok(Label {
            phoneme: self.p()?,
            mora: self.a()?,
            word_prev: self.b()?,
            word_curr: self.c()?,
            word_next: self.d()?,
            accent_phrase_prev: self.e()?,
            accent_phrase_curr: self.f()?,
            accent_phrase_next: self.g()?,
            breath_group_prev: self.h()?,
            breath_group_curr: self.i()?,
            breath_group_next: self.j()?,
            utterance: self.k()?,
        })
    }
}

impl FromStr for Label {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        LabelTokenizer::new(s).consume()
    }
}
