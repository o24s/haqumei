use std::fmt::Debug;
use std::fmt::Display;

use haqumei_jpreprocess_core::word_entry::WordEntry;
use haqumei_jpreprocess_core::{
    cform::CForm, ctype::CType, pos::*, pronunciation::Pronunciation, word_details::WordDetails,
};

use haqumei_jpreprocess_core::accent_rule::ChainRules;

#[derive(Clone, PartialEq, Debug)]
pub struct NJDNode {
    string: String, //*は空文字列として扱う
    details: WordDetails,
}

impl Display for NJDNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{},{}",
            self.string,
            self.details.to_str_vec(self.string.to_owned()).join(",")
        )
    }
}

impl NJDNode {
    pub fn new_single(s: &str) -> Self {
        let nodes = Self::load_csv(s);
        if nodes.len() == 1 {
            nodes.into_iter().next().unwrap()
        } else {
            panic!("input string must contain exactly one node.");
        }
    }
    /// 単一ノードの特徴量を書き換え、未指定の連結フラグは保持します。
    pub fn replace_from_csv(&mut self, s: &str) {
        let mut replacement = Self::new_single(s);
        if replacement.details.chain_flag.is_none() {
            replacement.details.chain_flag = self.details.chain_flag;
        }
        *self = replacement;
    }

    pub fn load_csv(s: &str) -> Vec<Self> {
        let splited = {
            let mut splited: Vec<&str> = s.split(',').collect();
            splited.resize(13, "");
            splited
        };
        Self::load_str(splited[0], &splited[1..splited.len()])
    }
    pub fn load_str(string: &str, details: &[&str]) -> Vec<Self> {
        let entry = WordEntry::load(details).unwrap();
        Self::load(string, &entry)
    }
    pub fn load(string: &str, entry: &WordEntry) -> Vec<Self> {
        entry
            .get_with_string(string)
            .into_iter()
            .map(|(string, details)| Self { string, details })
            .collect()
    }

    pub fn transfer_from(&mut self, node: &mut Self) {
        self.string.push_str(&node.string);
        if let Some(orig) = &node.details.orig {
            self.details.orig.get_or_insert_default().push_str(orig);
        }
        if let Some(add) = &node.details.read {
            if let Some(read) = &mut self.details.read {
                read.push_str(add);
            } else {
                self.details.read = Some(add.to_string());
            }
        }
        self.get_pron_mut().transfer_from(&node.details.pron);
        node.reset();
    }
    pub fn reset(&mut self) {
        self.string.clear();
        self.details = WordDetails::default();
    }
}

/// Getters and setters
impl NJDNode {
    pub fn get_chain_flag(&self) -> Option<bool> {
        self.details.chain_flag
    }
    pub fn set_chain_flag(&mut self, chain_flag: bool) {
        self.details.chain_flag = Some(chain_flag);
    }

    pub fn get_chain_rule(&self) -> &ChainRules {
        &self.details.chain_rule
    }
    pub fn unset_chain_rule(&mut self) {
        self.details.chain_rule.unset();
    }

    pub fn get_pos(&self) -> &POS {
        &self.details.pos
    }
    pub fn get_pos_mut(&mut self) -> &mut POS {
        &mut self.details.pos
    }

    pub fn is_renyou(&self) -> bool {
        self.details.cform.is_renyou()
    }
    pub fn get_ctype(&self) -> &CType {
        &self.details.ctype
    }
    pub fn get_cform(&self) -> &CForm {
        &self.details.cform
    }

    pub fn get_string(&self) -> &str {
        self.string.as_str()
    }
    pub fn replace_string(&mut self, new_string: &str) {
        self.string = new_string.to_string();
    }

    /// 辞書に登録された原形を返します。
    pub fn get_orig(&self) -> Option<&str> {
        self.details.orig.as_deref()
    }
    /// 辞書に登録された原形を変更します。
    pub fn set_orig(&mut self, orig: &str) {
        self.details.orig = Some(orig.to_owned());
    }
    /// 語の品詞、読み、アクセント結合規則を返します。
    pub fn get_details(&self) -> &WordDetails {
        &self.details
    }
    /// 語の品詞、読み、アクセント結合規則を変更する参照を返します。
    pub fn get_details_mut(&mut self) -> &mut WordDetails {
        &mut self.details
    }
    /// 表層形と語の特徴量からノードを作ります。
    pub fn from_details(string: String, details: WordDetails) -> Self {
        Self { string, details }
    }

    pub fn get_read(&self) -> Option<&str> {
        self.details.read.as_deref()
    }
    pub fn set_read(&mut self, read: &str) {
        self.details.read = Some(read.to_string());
    }

    pub fn get_pron(&self) -> &Pronunciation {
        &self.details.pron
    }
    pub fn get_pron_mut(&mut self) -> &mut Pronunciation {
        &mut self.details.pron
    }
    pub fn set_pron(&mut self, pron: Pronunciation) {
        self.details.pron = pron;
    }
}

#[cfg(test)]
mod tests {
    use super::NJDNode;

    #[test]
    fn single_node() {
        let node = NJDNode::new_single("．,名詞,接尾,助数詞,*,*,*,．,テン,テン,0/2,*,-1");
        assert_eq!(node.string, "．");
        assert!(!node.is_renyou());

        assert_eq!(
            node.to_string(),
            "．,名詞,接尾,助数詞,*,*,*,．,テン,テン,0/2,*,-1"
        )
    }

    #[test]
    fn multiple_nodes() {
        let nodes = NJDNode::load_csv("あーあ,感動詞,*,*,*,*,*,あー:あ,アー:ア,アー:ア,1/2:1/1,C1");
        assert_eq!(nodes.len(), 2);

        assert_eq!(
            nodes[0].to_string(),
            "あー,感動詞,*,*,*,*,*,あー,アー,アー,1/2,C1,-1"
        );
        assert_eq!(
            nodes[1].to_string(),
            "あ,感動詞,*,*,*,*,*,あ,ア,ア,1/1,C1,0"
        );
    }

    #[test]
    fn original_form_survives_inflection() {
        let node = NJDNode::new_single("食べ,動詞,自立,*,*,一段,連用形,食べる,タベ,タベ,2/2,*,1");
        assert_eq!(node.get_string(), "食べ");
        assert_eq!(node.get_orig(), Some("食べる"));
        assert!(node.to_string().contains(",食べる,"));
    }

    #[test]
    fn malformed_chained_surface_does_not_panic() {
        let nodes = NJDNode::load_csv("あ,感動詞,*,*,*,*,*,いい:う,イイ:ウ,イイ:ウ,1/2:1/1,C1");
        assert_eq!(nodes[0].get_string(), "あ");
        assert_eq!(nodes[1].get_string(), "う");
        assert_eq!(nodes[1].get_orig(), Some("う"));
    }

    #[test]
    fn dictionary_replacement_keeps_unspecified_chain_flag() {
        let mut node = NJDNode::new_single("二,名詞,数,*,*,*,*,二,ニ,ニ,1/1,C3,0");
        node.replace_from_csv("二十,名詞,副詞可能,*,*,*,*,二十,ニジュウ,ニジュー,1/3,*");
        assert_eq!(node.get_chain_flag(), Some(false));
        node.replace_from_csv("語,名詞,一般,*,*,*,*,語,ゴ,ゴ,1/1,*,1");
        assert_eq!(node.get_chain_flag(), Some(true));
    }

    #[test]
    fn part_of_speech_subcategories_survive_conversion() {
        let node = NJDNode::new_single("そう,名詞,特殊,助動詞語幹,*,*,*,そう,ソウ,ソー,1/2,*,0");
        assert_eq!(node.get_details().pos_string(), "名詞,特殊,助動詞語幹,*");
        assert!(node.to_string().contains("名詞,特殊,助動詞語幹,*"));
    }

    #[test]
    fn test_send() {
        fn assert_send<T: Send>() {}
        assert_send::<NJDNode>();
    }

    #[test]
    fn test_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<NJDNode>();
    }
}
