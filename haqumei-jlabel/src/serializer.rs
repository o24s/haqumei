use std::fmt::{Display, Formatter, Result, Write};

use crate::fullcontext_label::{
    AccentPhraseCurrent, AccentPhrasePrevNext, BreathGroupCurrent, BreathGroupPrevNext, Label,
    Mora, Phoneme, Utterance, Word,
};

struct Serializer<'a, 'b> {
    f: &'b mut Formatter<'a>,
}

impl<'a, 'b> Serializer<'a, 'b> {
    fn new(f: &'b mut Formatter<'a>) -> Self {
        Self { f }
    }

    fn xx(&mut self) -> Result {
        self.f.write_str("xx")
    }

    fn all_xx<const N: usize>(&mut self, sep: &[char; N]) -> Result {
        self.xx()?;
        for c in sep {
            self.f.write_char(*c)?;
            self.xx()?;
        }

        Ok(())
    }

    fn or_xx<T: Display>(&mut self, value: &Option<T>) -> Result {
        match value {
            Some(v) => v.fmt(self.f),
            None => self.xx(),
        }
    }

    fn d01_or_xx<T: Display>(&mut self, value: &Option<T>) -> Result {
        match value {
            Some(v) => write!(self.f, "{v:01}"),
            None => self.xx(),
        }
    }

    fn d02_or_xx<T: Display>(&mut self, value: &Option<T>) -> Result {
        match value {
            Some(v) => write!(self.f, "{v:02}"),
            None => self.xx(),
        }
    }

    fn bool(&mut self, value: bool) -> Result {
        match value {
            true => self.f.write_char('1'),
            false => self.f.write_char('0'),
        }
    }

    fn bool_or_xx(&mut self, value: &Option<bool>) -> Result {
        match value {
            Some(v) => self.bool(*v),
            None => self.xx(),
        }
    }

    /// `p1ˆp2-p3+p4=p5`
    fn p(&mut self, phoneme: &Phoneme) -> Result {
        self.or_xx(&phoneme.p2)?;
        self.f.write_char('^')?;
        self.or_xx(&phoneme.p1)?;
        self.f.write_char('-')?;
        self.or_xx(&phoneme.c)?;
        self.f.write_char('+')?;
        self.or_xx(&phoneme.n1)?;
        self.f.write_char('=')?;
        self.or_xx(&phoneme.n2)?;

        Ok(())
    }

    /// `/A:a1+a2+a3`
    fn a(&mut self, mora: &Option<Mora>) -> Result {
        self.f.write_str("/A:")?;

        if let Some(mora) = mora {
            mora.relative_accent_position.fmt(self.f)?;
            self.f.write_char('+')?;
            mora.position_forward.fmt(self.f)?;
            self.f.write_char('+')?;
            mora.position_backward.fmt(self.f)?;
        } else {
            self.all_xx(&['+', '+'])?;
        }

        Ok(())
    }

    /// `/B:b1-b2_b3`
    fn b(&mut self, word_prev: &Option<Word>) -> Result {
        self.f.write_str("/B:")?;

        if let Some(word_prev) = word_prev {
            self.d02_or_xx(&word_prev.pos)?;
            self.f.write_char('-')?;
            self.d01_or_xx(&word_prev.ctype)?;
            self.f.write_char('_')?;
            self.d01_or_xx(&word_prev.cform)?;
        } else {
            self.all_xx(&['-', '_'])?;
        }

        Ok(())
    }

    /// `/C:c1_c2+c3`
    fn c(&mut self, word_curr: &Option<Word>) -> Result {
        self.f.write_str("/C:")?;

        if let Some(word_curr) = word_curr {
            self.d02_or_xx(&word_curr.pos)?;
            self.f.write_char('_')?;
            self.d01_or_xx(&word_curr.ctype)?;
            self.f.write_char('+')?;
            self.d01_or_xx(&word_curr.cform)?;
        } else {
            self.all_xx(&['_', '+'])?;
        }

        Ok(())
    }

    /// `/D:d1+d2_d3`
    fn d(&mut self, word_next: &Option<Word>) -> Result {
        self.f.write_str("/D:")?;

        if let Some(word_next) = word_next {
            self.d02_or_xx(&word_next.pos)?;
            self.f.write_char('+')?;
            self.d01_or_xx(&word_next.ctype)?;
            self.f.write_char('_')?;
            self.d01_or_xx(&word_next.cform)?;
        } else {
            self.all_xx(&['+', '_'])?;
        }

        Ok(())
    }

    ///`/E:e1_e2!e3_e4-e5`
    fn e(&mut self, accent_phrase_prev: &Option<AccentPhrasePrevNext>) -> Result {
        self.f.write_str("/E:")?;

        if let Some(accent_phrase_prev) = accent_phrase_prev {
            accent_phrase_prev.mora_count.fmt(self.f)?;
            self.f.write_char('_')?;
            accent_phrase_prev.accent_position.fmt(self.f)?;
            self.f.write_char('!')?;
            self.bool(accent_phrase_prev.is_interrogative)?;
            self.f.write_char('_')?;
            self.bool(accent_phrase_prev.is_exclamatory)?;
            self.f.write_char('-')?;
            self.bool_or_xx(&accent_phrase_prev.is_pause_insertion.map(|value| !value))?;
        } else {
            self.all_xx(&['_', '!', '_', '-'])?;
        }

        Ok(())
    }

    /// `/F:f1_f2#f3_f4@f5_f6|f7_f8`
    fn f(&mut self, accent_phrase_curr: &Option<AccentPhraseCurrent>) -> Result {
        self.f.write_str("/F:")?;

        if let Some(accent_phrase_curr) = accent_phrase_curr {
            accent_phrase_curr.mora_count.fmt(self.f)?;
            self.f.write_char('_')?;
            accent_phrase_curr.accent_position.fmt(self.f)?;
            self.f.write_char('#')?;
            self.bool(accent_phrase_curr.is_interrogative)?;
            self.f.write_char('_')?;
            self.bool(accent_phrase_curr.is_exclamatory)?;
            self.f.write_char('@')?;
            accent_phrase_curr
                .accent_phrase_position_forward
                .fmt(self.f)?;
            self.f.write_char('_')?;
            accent_phrase_curr
                .accent_phrase_position_backward
                .fmt(self.f)?;
            self.f.write_char('|')?;
            accent_phrase_curr.mora_position_forward.fmt(self.f)?;
            self.f.write_char('_')?;
            accent_phrase_curr.mora_position_backward.fmt(self.f)?;
        } else {
            self.all_xx(&['_', '#', '_', '@', '_', '|', '_'])?;
        }

        Ok(())
    }

    /// `/G:g1_g2%g3_g4_g5`
    fn g(&mut self, accent_phrase_next: &Option<AccentPhrasePrevNext>) -> Result {
        self.f.write_str("/G:")?;

        if let Some(accent_phrase_next) = accent_phrase_next {
            accent_phrase_next.mora_count.fmt(self.f)?;
            self.f.write_char('_')?;
            accent_phrase_next.accent_position.fmt(self.f)?;
            self.f.write_char('%')?;
            self.bool(accent_phrase_next.is_interrogative)?;
            self.f.write_char('_')?;
            self.bool(accent_phrase_next.is_exclamatory)?;
            self.f.write_char('_')?;
            self.bool_or_xx(&accent_phrase_next.is_pause_insertion.map(|value| !value))?;
        } else {
            self.all_xx(&['_', '%', '_', '_'])?;
        }

        Ok(())
    }

    /// `/H:h1_h2`
    fn h(&mut self, breath_group_prev: &Option<BreathGroupPrevNext>) -> Result {
        self.f.write_str("/H:")?;

        if let Some(breath_group_prev) = breath_group_prev {
            breath_group_prev.accent_phrase_count.fmt(self.f)?;
            self.f.write_char('_')?;
            breath_group_prev.mora_count.fmt(self.f)?;
        } else {
            self.all_xx(&['_'])?;
        }

        Ok(())
    }

    /// `/I:i1-i2@i3+i4&i5-i6|i7+i8`
    fn i(&mut self, breath_group_curr: &Option<BreathGroupCurrent>) -> Result {
        self.f.write_str("/I:")?;

        if let Some(breath_group_curr) = breath_group_curr {
            breath_group_curr.accent_phrase_count.fmt(self.f)?;
            self.f.write_char('-')?;
            breath_group_curr.mora_count.fmt(self.f)?;
            self.f.write_char('@')?;
            breath_group_curr
                .breath_group_position_forward
                .fmt(self.f)?;
            self.f.write_char('+')?;
            breath_group_curr
                .breath_group_position_backward
                .fmt(self.f)?;
            self.f.write_char('&')?;
            breath_group_curr
                .accent_phrase_position_forward
                .fmt(self.f)?;
            self.f.write_char('-')?;
            breath_group_curr
                .accent_phrase_position_backward
                .fmt(self.f)?;
            self.f.write_char('|')?;
            breath_group_curr.mora_position_forward.fmt(self.f)?;
            self.f.write_char('+')?;
            breath_group_curr.mora_position_backward.fmt(self.f)?;
        } else {
            self.all_xx(&['-', '@', '+', '&', '-', '|', '+'])?;
        }

        Ok(())
    }

    /// `/J:j1_j2`
    fn j(&mut self, breath_group_next: &Option<BreathGroupPrevNext>) -> Result {
        self.f.write_str("/J:")?;

        if let Some(breath_group_next) = breath_group_next {
            breath_group_next.accent_phrase_count.fmt(self.f)?;
            self.f.write_char('_')?;
            breath_group_next.mora_count.fmt(self.f)?;
        } else {
            self.all_xx(&['_'])?;
        }

        Ok(())
    }

    /// `/K:k1+k2-k3`
    fn k(&mut self, utterance: &Utterance) -> Result {
        self.f.write_str("/K:")?;

        utterance.breath_group_count.fmt(self.f)?;
        self.f.write_char('+')?;
        utterance.accent_phrase_count.fmt(self.f)?;
        self.f.write_char('-')?;
        utterance.mora_count.fmt(self.f)?;

        Ok(())
    }

    fn fmt(&mut self, label: &Label) -> Result {
        self.p(&label.phoneme)?;
        self.a(&label.mora)?;
        self.b(&label.word_prev)?;
        self.c(&label.word_curr)?;
        self.d(&label.word_next)?;
        self.e(&label.accent_phrase_prev)?;
        self.f(&label.accent_phrase_curr)?;
        self.g(&label.accent_phrase_next)?;
        self.h(&label.breath_group_prev)?;
        self.i(&label.breath_group_curr)?;
        self.j(&label.breath_group_next)?;
        self.k(&label.utterance)?;

        Ok(())
    }
}

impl Display for Label {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        Serializer::new(f).fmt(self)
    }
}
