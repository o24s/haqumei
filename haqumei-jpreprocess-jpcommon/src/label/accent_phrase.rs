use haqumei_jpreprocess_njd::NJDNode;

use crate::limit::Limit;

use super::*;

#[derive(Clone, Debug)]
pub struct AccentPhrase {
    accent: usize,
    is_interrogative: bool,
    is_exclamatory: bool,
    pub words: Vec<Word>,
}

impl AccentPhrase {
    pub fn new(start_node: &NJDNode) -> Self {
        Self::with_word(start_node, start_node.into())
    }
    pub(super) fn with_word(start_node: &NJDNode, word: Word) -> Self {
        Self {
            accent: start_node.get_pron().accent(),
            is_interrogative: false,
            is_exclamatory: false,
            words: vec![word],
        }
    }
    pub(super) fn set_interrogative(&mut self) {
        self.is_interrogative = true;
    }

    pub(super) fn set_exclamatory(&mut self) {
        self.is_exclamatory = true;
    }

    pub fn to_e(&self, is_prev_pause: Option<bool>) -> haqumei_jlabel::AccentPhrasePrevNext {
        let mora_count = self.count_mora();
        haqumei_jlabel::AccentPhrasePrevNext {
            mora_count: Limit::M.ulimit(mora_count),
            accent_position: Limit::M.ulimit(if self.accent == 0 {
                mora_count
            } else {
                self.accent
            }),
            is_interrogative: self.is_interrogative,
            is_exclamatory: self.is_exclamatory,
            is_pause_insertion: is_prev_pause,
        }
    }
    pub fn to_f(
        &self,
        accent_phrase_count_in_breath_group: usize,
        accent_phrase_index_in_breath_group: usize,
        mora_count_in_breath_group: usize,
        mora_index_in_breath_group: usize,
    ) -> haqumei_jlabel::AccentPhraseCurrent {
        let mora_count = self.count_mora();
        haqumei_jlabel::AccentPhraseCurrent {
            mora_count: Limit::M.ulimit(mora_count),
            accent_position: Limit::M.ulimit(if self.accent == 0 {
                mora_count
            } else {
                self.accent
            }),
            is_interrogative: self.is_interrogative,
            is_exclamatory: self.is_exclamatory,
            accent_phrase_position_forward: Limit::M
                .ulimit(accent_phrase_index_in_breath_group + 1),
            accent_phrase_position_backward: Limit::M
                .ulimit(accent_phrase_count_in_breath_group - accent_phrase_index_in_breath_group),
            mora_position_forward: Limit::L.ulimit(mora_index_in_breath_group + 1),
            mora_position_backward: Limit::L
                .ulimit(mora_count_in_breath_group - mora_index_in_breath_group),
        }
    }
    pub fn to_g(&self, is_next_pause: Option<bool>) -> haqumei_jlabel::AccentPhrasePrevNext {
        let mora_count = self.count_mora();
        haqumei_jlabel::AccentPhrasePrevNext {
            mora_count: Limit::M.ulimit(mora_count),
            accent_position: Limit::M.ulimit(if self.accent == 0 {
                mora_count
            } else {
                self.accent
            }),
            is_interrogative: self.is_interrogative,
            is_exclamatory: self.is_exclamatory,
            is_pause_insertion: is_next_pause,
        }
    }

    pub fn generate_mora_a(&self) -> Vec<haqumei_jlabel::Mora> {
        let mora_count = self.count_mora();
        let accent = if self.accent == 0 {
            mora_count
        } else {
            self.accent
        };
        (0..mora_count)
            .map(|mora_index| haqumei_jlabel::Mora {
                relative_accent_position: Limit::M
                    .ilimit(mora_index as isize - accent as isize + 1),
                position_forward: Limit::M.ulimit(mora_index + 1),
                position_backward: Limit::M.ulimit(mora_count - mora_index),
            })
            .collect()
    }

    pub fn count_mora(&self) -> usize {
        self.words.iter().map(|word| word.count_mora()).sum()
    }
}
