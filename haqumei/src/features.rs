use std::ops::Range;

use vibrato_rkyv::dictionary::{LexType, WordIdx};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NjdFeature {
    pub string: String,
    pub pos: String,
    pub pos_group1: String,
    pub pos_group2: String,
    pub pos_group3: String,
    pub ctype: String,
    pub cform: String,
    pub orig: String,
    pub read: String,
    pub pron: String,
    pub acc: i32,
    pub mora_size: i32,
    pub chain_rule: String,
    pub chain_flag: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnidicFeature {
    pub surface: String,
    pub feature: String,
    pub range_char: Range<usize>,
    pub range_byte: Range<usize>,
    pub lex_type: LexType,
    pub word_id: WordIdx,
    pub left_id: u16,
    pub right_id: u16,
    pub word_cost: i16,
    pub total_cost: i32,
    pub feature_ranges: Vec<Range<usize>>,
}

// Unidic の features
/*
f[0]:  pos1
f[1]:  pos2
f[2]:  pos3
f[3]:  pos4
f[4]:  cType
f[5]:  cForm
f[6]:  lForm
f[7]:  lemma
f[8]:  orth
f[9]:  pron
f[10]: orthBase
f[11]: pronBase
f[12]: goshu
f[13]: iType
f[14]: iForm
f[15]: fType
f[16]: fForm
f[17]: iConType
f[18]: fConType
f[19]: type
f[20]: kana
f[21]: kanaBase
f[22]: form
f[23]: formBase
f[24]: aType
f[25]: aConType
f[26]: aModType
f[27]: lid
f[28]: lemma_id
*/

impl UnidicFeature {
    #[track_caller]
    pub fn pos1(&self) -> &str {
        &self.feature[self.feature_ranges[0].clone()]
    }
    #[track_caller]
    pub fn pos2(&self) -> &str {
        &self.feature[self.feature_ranges[1].clone()]
    }
    #[track_caller]
    pub fn pos3(&self) -> &str {
        &self.feature[self.feature_ranges[2].clone()]
    }
    #[track_caller]
    pub fn pos4(&self) -> &str {
        &self.feature[self.feature_ranges[3].clone()]
    }
    #[track_caller]
    pub fn ctype(&self) -> &str {
        &self.feature[self.feature_ranges[4].clone()]
    }
    #[track_caller]
    pub fn cform(&self) -> &str {
        &self.feature[self.feature_ranges[5].clone()]
    }
    #[track_caller]
    pub fn lform(&self) -> &str {
        &self.feature[self.feature_ranges[6].clone()]
    }
    #[track_caller]
    pub fn lemma(&self) -> &str {
        &self.feature[self.feature_ranges[7].clone()]
    }
    #[track_caller]
    pub fn orth(&self) -> &str {
        &self.feature[self.feature_ranges[8].clone()]
    }
    #[track_caller]
    pub fn pron(&self) -> &str {
        &self.feature[self.feature_ranges[9].clone()]
    }
    #[track_caller]
    pub fn orth_base(&self) -> &str {
        &self.feature[self.feature_ranges[10].clone()]
    }
    #[track_caller]
    pub fn pron_base(&self) -> &str {
        &self.feature[self.feature_ranges[11].clone()]
    }
    #[track_caller]
    pub fn goshu(&self) -> &str {
        &self.feature[self.feature_ranges[12].clone()]
    }
    #[track_caller]
    pub fn i_type(&self) -> &str {
        &self.feature[self.feature_ranges[13].clone()]
    }
    #[track_caller]
    pub fn i_form(&self) -> &str {
        &self.feature[self.feature_ranges[14].clone()]
    }
    #[track_caller]
    pub fn f_type(&self) -> &str {
        &self.feature[self.feature_ranges[15].clone()]
    }
    #[track_caller]
    pub fn f_form(&self) -> &str {
        &self.feature[self.feature_ranges[16].clone()]
    }
    #[track_caller]
    pub fn i_con_type(&self) -> &str {
        &self.feature[self.feature_ranges[17].clone()]
    }
    #[track_caller]
    pub fn f_con_type(&self) -> &str {
        &self.feature[self.feature_ranges[18].clone()]
    }
    #[track_caller]
    pub fn r#type(&self) -> &str {
        &self.feature[self.feature_ranges[19].clone()]
    }
    #[track_caller]
    pub fn kana(&self) -> &str {
        &self.feature[self.feature_ranges[20].clone()]
    }
    #[track_caller]
    pub fn kana_base(&self) -> &str {
        &self.feature[self.feature_ranges[21].clone()]
    }
    #[track_caller]
    pub fn form(&self) -> &str {
        &self.feature[self.feature_ranges[22].clone()]
    }
    #[track_caller]
    pub fn form_base(&self) -> &str {
        &self.feature[self.feature_ranges[23].clone()]
    }
    #[track_caller]
    pub fn a_type(&self) -> &str {
        &self.feature[self.feature_ranges[24].clone()]
    }
    #[track_caller]
    pub fn a_con_type(&self) -> &str {
        &self.feature[self.feature_ranges[25].clone()]
    }
    #[track_caller]
    pub fn a_mod_type(&self) -> &str {
        &self.feature[self.feature_ranges[26].clone()]
    }
    #[track_caller]
    pub fn lid(&self) -> &str {
        &self.feature[self.feature_ranges[27].clone()]
    }
    #[track_caller]
    pub fn lemma_id(&self) -> &str {
        &self.feature[self.feature_ranges[28].clone()]
    }
}
