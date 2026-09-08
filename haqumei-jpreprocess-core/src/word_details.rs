use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{
    accent_rule::ChainRules,
    cform::CForm,
    ctype::CType,
    pos::{Meishi, POS},
    pronunciation::Pronunciation,
    word_line::WordDetailsLine,
    JPreprocessResult,
};

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct WordDetails {
    pub pos: POS,
    #[doc(hidden)]
    pub pos_original: Option<(POS, String)>,
    pub ctype: CType,
    pub cform: CForm,
    /// 辞書に登録された原形。
    pub orig: Option<String>,
    pub read: Option<String>,
    pub pron: Pronunciation,
    pub chain_rule: ChainRules,
    pub chain_flag: Option<bool>,
}

impl Default for WordDetails {
    fn default() -> Self {
        Self {
            pos: POS::Meishi(Meishi::None),
            pos_original: None,
            ctype: CType::default(),
            cform: CForm::default(),
            orig: None,
            read: None,
            pron: Pronunciation::default(),
            chain_rule: ChainRules::default(),
            chain_flag: None,
        }
    }
}

impl WordDetails {
    /// 変更されていない品詞は、辞書に登録された細分類を含む形で返します。
    pub fn pos_string(&self) -> String {
        self.pos_original
            .as_ref()
            .filter(|(pos, _)| *pos == self.pos)
            .map(|(_, original)| original.clone())
            .unwrap_or_else(|| self.pos.to_string())
    }

    pub fn load(details: &[&str]) -> JPreprocessResult<Self> {
        WordDetailsLine::from_strs(details).try_into()
    }

    pub fn extend_splited(
        &mut self,
        read: &str,
        pron: &str,
        acc_morasize: &str,
    ) -> JPreprocessResult<()> {
        self.read = match read {
            "*" => None,
            _ => Some(read.to_string()),
        };
        self.pron = Pronunciation::parse_csv_pron(pron, acc_morasize)?;
        self.chain_flag = Some(false);
        Ok(())
    }

    pub fn to_str_vec(&self, orig: String) -> [String; 9] {
        let line = WordDetailsLine::from(self);

        [
            format!(
                "{},{},{},{}",
                line.pos, line.pos_group1, line.pos_group2, line.pos_group3
            ),
            line.ctype.to_string(),
            line.cform.to_string(),
            self.orig.clone().unwrap_or(orig),
            line.read.to_string(),
            line.pron.to_string(),
            line.acc_morasize.to_string(),
            line.chain_rule.to_string(),
            line.chain_flag.to_string(),
        ]
    }
}

impl TryFrom<WordDetailsLine> for WordDetails {
    type Error = crate::JPreprocessError;
    fn try_from(value: WordDetailsLine) -> Result<WordDetails, Self::Error> {
        let pos = POS::from_strs(
            &value.pos,
            &value.pos_group1,
            &value.pos_group2,
            &value.pos_group3,
        )?;
        let original = format!(
            "{},{},{},{}",
            value.pos, value.pos_group1, value.pos_group2, value.pos_group3
        );
        Ok(Self {
            pos,
            pos_original: (original != pos.to_string()).then_some((pos, original)),
            ctype: CType::from_str(&value.ctype)?,
            cform: CForm::from_str(&value.cform)?,
            chain_rule: ChainRules::new(&value.chain_rule),
            chain_flag: match value.chain_flag.as_ref() {
                "1" => Some(true),
                "0" => Some(false),
                _ => None,
            },
            orig: match value.orig.as_str() {
                "*" | "" => None,
                _ => Some(value.orig.clone()),
            },
            read: match value.read.as_ref() {
                "*" => None,
                _ => Some(value.read.to_string()),
            },
            pron: Pronunciation::parse_csv_pron(&value.pron, &value.acc_morasize)?,
        })
    }
}

impl From<&WordDetails> for WordDetailsLine {
    fn from(value: &WordDetails) -> Self {
        let pos = value.pos_string();
        let pos_parts: Vec<&str> = pos.split(',').collect();
        assert_eq!(pos_parts.len(), 4, "POS must have exactly 4 parts");

        Self {
            pos: pos_parts[0].to_string(),
            pos_group1: pos_parts[1].to_string(),
            pos_group2: pos_parts[2].to_string(),
            pos_group3: pos_parts[3].to_string(),
            ctype: value.ctype.to_string(),
            cform: value.cform.to_string(),
            orig: value.orig.as_deref().unwrap_or("*").to_string(),
            read: value.read.as_deref().unwrap_or("*").to_string(),
            pron: value.pron.to_string(),
            acc_morasize: format!("{}/{}", value.pron.accent(), value.pron.mora_size()),
            chain_rule: value.chain_rule.to_original_string(),
            chain_flag: match value.chain_flag {
                Some(true) => "1",
                Some(false) => "0",
                None => "-1",
            }
            .into(),
        }
    }
}
