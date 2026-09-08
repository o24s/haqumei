use crate::{word_details::WordDetails, word_line::WordDetailsLine, JPreprocessResult};
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub enum WordEntry {
    Single(WordDetails),
    Multiple(Vec<(String, WordDetails)>),
}

impl Default for WordEntry {
    fn default() -> Self {
        Self::Single(WordDetails::default())
    }
}

impl WordEntry {
    pub fn load(details: &[&str]) -> JPreprocessResult<Self> {
        WordDetailsLine::from_strs(details).try_into()
    }

    pub fn get_with_string(&self, string: &str) -> Vec<(String, WordDetails)> {
        match &self {
            Self::Single(word_details) => vec![(string.to_string(), word_details.to_owned())],
            Self::Multiple(word_details_vec) => {
                let mut remaining = string;
                let valid_prefix = word_details_vec
                    .iter()
                    .take(word_details_vec.len().saturating_sub(1))
                    .all(|(orig, _)| {
                        if let Some(rest) =
                            remaining.strip_prefix(orig).filter(|_| !orig.is_empty())
                        {
                            remaining = rest;
                            true
                        } else {
                            false
                        }
                    });
                if !valid_prefix || remaining.is_empty() {
                    return word_details_vec
                        .iter()
                        .enumerate()
                        .map(|(i, (orig, details))| {
                            (
                                (if i == 0 { string } else { orig }).to_owned(),
                                details.clone(),
                            )
                        })
                        .collect();
                }
                let mut result = Vec::with_capacity(word_details_vec.len());
                let mut len = 0;
                for (i, (orig, word_details)) in word_details_vec.iter().enumerate() {
                    if i + 1 < word_details_vec.len() {
                        // not the last word
                        result.push((orig.to_string(), word_details.to_owned()));
                        len += orig.len();
                    } else {
                        // the last word
                        result.push((string[len..].to_string(), word_details.to_owned()));
                    }
                }
                result
            }
        }
    }

    pub fn to_str_vec(&self, orig: String) -> [String; 9] {
        let mut line = WordDetailsLine::from(self);

        if matches!(self, Self::Single(_)) {
            line.orig = orig;
        }

        [
            format!(
                "{},{},{},{}",
                line.pos, line.pos_group1, line.pos_group2, line.pos_group3
            ),
            line.ctype.to_string(),
            line.cform.to_string(),
            line.orig.to_string(),
            line.read.to_string(),
            line.pron.to_string(),
            line.acc_morasize.to_string(),
            line.chain_rule.to_string(),
            line.chain_flag.to_string(),
        ]
    }
}

impl TryFrom<WordDetailsLine> for WordEntry {
    type Error = crate::JPreprocessError;
    fn try_from(value: WordDetailsLine) -> Result<Self, Self::Error> {
        let count = value.acc_morasize.matches('/').count();
        if count > 1 {
            let mut origs = value.orig.split(':');
            let mut reads = value.read.split(':');
            let mut prons = value.pron.split(':');
            let mut accents = value.acc_morasize.split(':');
            let mut word_details = Vec::with_capacity(count);
            for i in 0..count {
                let orig = origs
                    .next()
                    .filter(|value| !value.is_empty())
                    .unwrap_or("*");
                let details = WordDetailsLine {
                    orig: orig.to_owned(),
                    read: reads.next().unwrap_or("*").to_owned(),
                    pron: prons.next().unwrap_or("*").to_owned(),
                    acc_morasize: accents.next().unwrap_or("0/0").to_owned(),
                    chain_flag: if i == 0 {
                        value.chain_flag.clone()
                    } else {
                        "0".to_owned()
                    },
                    ..value.clone()
                };
                word_details.push((orig.to_owned(), WordDetails::try_from(details)?));
            }
            Ok(Self::Multiple(word_details))
        } else {
            Ok(Self::Single(WordDetails::try_from(value)?))
        }
    }
}

impl From<&WordEntry> for WordDetailsLine {
    fn from(value: &WordEntry) -> Self {
        match value {
            WordEntry::Single(details) => details.into(),
            WordEntry::Multiple(details_vec) => {
                details_vec.iter().skip(1).fold(
                    {
                        let first_elem = &details_vec[0];
                        Self {
                            orig: first_elem.0.to_owned(),
                            ..(&first_elem.1).into()
                        }
                    },
                    |acc, (orig, details)| {
                        let v: Self = details.into();

                        Self {
                            orig: format!("{}:{}", acc.orig, orig),   // orig
                            read: format!("{}:{}", acc.read, v.read), // read
                            pron: format!("{}:{}", acc.pron, v.pron), // pron
                            acc_morasize: format!("{}:{}", acc.acc_morasize, v.acc_morasize), // acc/mora_size
                            ..acc
                        }
                    },
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{cform::CForm, ctype::CType, pos::*, pron, word_entry::WordEntry};

    #[test]
    fn missing_original_segments_keep_every_accent_segment() {
        let input = "赤青,名詞,一般,*,*,*,*,赤青,アカ:アオ,アカ:アオ,0/2:1/2,C1,-1";
        let fields: Vec<&str> = input.split(',').collect();
        let entry = WordEntry::load(&fields[1..]).unwrap();
        let nodes = entry.get_with_string(fields[0]);
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].0, "赤青");
        assert_eq!(nodes[1].0, "*");
        assert_eq!(nodes[0].1.pron.to_string(), "アカ");
        assert_eq!(nodes[1].1.pron.to_string(), "アオ");
        assert_eq!(nodes[1].1.chain_flag, Some(false));
    }

    #[test]
    fn load_single_node() {
        let input: Vec<&str> = "．,名詞,接尾,助数詞,*,*,*,．,テン,テン,0/2,*,"
            .split(',')
            .collect();
        let entry = WordEntry::load(&input[1..]).unwrap();
        let details_vec = entry.get_with_string(input[0]);
        assert_eq!(details_vec.len(), 1);

        let (string, details) = &details_vec[0];

        assert_eq!(string, "．");
        assert!(matches!(
            details.pos,
            POS::Meishi(Meishi::Setsubi(Setsubi::Josuushi))
        ));
        assert_eq!(details.ctype, CType::None);
        assert_eq!(details.cform, CForm::None);
        assert_eq!(details.read.as_ref().unwrap(), "テン");
        assert_eq!(details.pron, pron!([Te, N], 0));
        assert_eq!(details.chain_rule.get_rule(&POS::Filler), None);
        assert_eq!(details.chain_flag, None);

        let v = entry.to_str_vec(input[0].to_owned());
        assert_eq!(v[0..8].join(","), input[1..12].join(","));
    }

    #[test]
    fn load_multiple_nodes() {
        let input: Vec<&str> = "あーあ,感動詞,*,*,*,*,*,あー:あ,アー:ア,アー:ア,1/2:1/1,C1,"
            .split(',')
            .collect();
        let entry = WordEntry::load(&input[1..]).unwrap();
        let details_vec = entry.get_with_string(input[0]);
        assert_eq!(details_vec.len(), 2);

        assert_eq!(details_vec[0].0, "あー");
        assert_eq!(details_vec[1].0, "あ");

        let details0 = &details_vec[0].1;
        let details1 = &details_vec[1].1;

        assert_eq!(&details0.pron, &pron!([A, Long], 1));
        assert_eq!(&details1.pron, &pron!([A], 1));

        let v = entry.to_str_vec(input[0].to_owned());
        assert_eq!(v[0..8].join(","), input[1..12].join(","));
    }
}
