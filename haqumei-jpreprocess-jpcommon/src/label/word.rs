use haqumei_jpreprocess_core::pronunciation::Pronunciation;

use haqumei_jpreprocess_njd::NJDNode;

use crate::word_attr::*;

#[derive(Clone, Debug)]
pub struct Word {
    pos: Option<u8>,
    ctype: Option<u8>,
    cform: Option<u8>,
    pub moras: Pronunciation,
    /// 単語を作った入力 NJD ノードの添字。
    pub source_index: Option<usize>,
    /// アクセント句の途中で、語の直前にポーズが入るか。
    pub pause_before: bool,
}

impl Word {
    pub fn count_mora(&self) -> usize {
        self.moras.moras().len()
    }
}

impl From<&NJDNode> for Word {
    fn from(njdnode: &NJDNode) -> Self {
        Self {
            pos: pos_details_to_id(njdnode.get_details()),
            ctype: ctype_to_id(njdnode.get_ctype()),
            cform: cform_to_id(njdnode.get_cform()),
            moras: njdnode.get_pron().clone(),
            source_index: None,
            pause_before: false,
        }
    }
}

impl From<&Word> for haqumei_jlabel::Word {
    fn from(val: &Word) -> Self {
        haqumei_jlabel::Word {
            pos: val.pos,
            ctype: val.ctype,
            cform: val.cform,
        }
    }
}
