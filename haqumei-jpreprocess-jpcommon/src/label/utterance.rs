use haqumei_jpreprocess_core::pronunciation::MoraEnum;
use haqumei_jpreprocess_njd::NJDNode;

use crate::limit::Limit;

use super::*;

#[derive(Clone, Debug)]
pub struct Utterance {
    pub breath_groups: Vec<BreathGroup>,
}

impl Utterance {
    pub fn to_k(&self) -> haqumei_jlabel::Utterance {
        haqumei_jlabel::Utterance {
            breath_group_count: Limit::S.ulimit(self.breath_groups.len()),
            accent_phrase_count: Limit::M.ulimit(self.count_accent_phrase()),
            mora_count: Limit::LL.ulimit(self.count_mora()),
        }
    }

    pub fn count_accent_phrase(&self) -> usize {
        self.breath_groups
            .iter()
            .map(|bg| bg.count_accent_phrase())
            .sum()
    }
    pub fn count_mora(&self) -> usize {
        self.breath_groups.iter().map(|bg| bg.count_mora()).sum()
    }
}

impl From<&[NJDNode]> for Utterance {
    fn from(nodes: &[NJDNode]) -> Self {
        let mut breath_groups: Vec<BreathGroup> = Vec::new();
        let mut accent_phrases: Vec<AccentPhrase> = Vec::with_capacity(nodes.len());

        let mut pause_pending = false;

        for (source_index, node) in nodes.iter().enumerate() {
            let pron = node.get_pron();
            if pron.is_question() || pron.is_exclamation() {
                // 句読点はまだ音素を作らないので、連続する「？」「！」も同じ句に付く。
                let accent_phrase = accent_phrases.last_mut().or_else(|| {
                    breath_groups
                        .last_mut()
                        .and_then(|group| group.accent_phrases.last_mut())
                });
                if let Some(accent_phrase) = accent_phrase {
                    if pron.is_question() {
                        accent_phrase.set_interrogative();
                    } else {
                        accent_phrase.set_exclamatory();
                    }
                }
            }
            if pron.is_touten() || pron.is_question() || pron.is_exclamation() {
                pause_pending = true;
                continue;
            }

            let mut word = Word::from(node);
            word.source_index = Some(source_index);
            let leading_long = word
                .moras
                .moras()
                .iter()
                .take_while(|mora| mora.mora_enum == MoraEnum::Long)
                .count();
            if leading_long > 0 {
                // 語頭の長音は直前の語を伸ばす。ポーズの直後や文頭には伸ばす音がない。
                if let Some(previous) = (!pause_pending)
                    .then(|| {
                        accent_phrases
                            .last_mut()
                            .and_then(|phrase| phrase.words.last_mut())
                    })
                    .flatten()
                {
                    previous.moras.moras.to_mut().extend(
                        word.moras.moras()[..leading_long]
                            .iter()
                            .cloned()
                            .map(|mut mora| {
                                mora.is_voiced = true;
                                mora
                            }),
                    );
                }
                word.moras.moras.to_mut().drain(..leading_long);
            }
            if word.moras.is_empty() {
                continue;
            }

            let is_chained = matches!(node.get_chain_flag(), Some(true));
            if pause_pending && !is_chained && !accent_phrases.is_empty() {
                breath_groups.push(BreathGroup::new(std::mem::take(&mut accent_phrases)));
            }
            if is_chained {
                if let Some(accent_phrase) = accent_phrases.last_mut() {
                    word.pause_before = pause_pending;
                    accent_phrase.words.push(word);
                } else {
                    accent_phrases.push(AccentPhrase::with_word(node, word));
                }
            } else {
                accent_phrases.push(AccentPhrase::with_word(node, word));
            }
            pause_pending = false;
        }
        if !accent_phrases.is_empty() {
            breath_groups.push(BreathGroup::new(accent_phrases));
        }

        Self { breath_groups }
    }
}

#[cfg(test)]
mod tests {
    use crate::Utterance;

    #[test]
    fn test_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Utterance>();
    }

    #[test]
    fn test_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<Utterance>();
    }
}
