#![cfg_attr(docsrs, feature(doc_cfg))]

mod feature;
mod label;
mod word_attr;

pub use feature::*;
use haqumei_jlabel::Label;
pub use label::*;

use haqumei_jpreprocess_njd::NJDNode;

/// NJD ノードからフルコンテキストラベルを生成します。
pub fn njdnodes_to_features(njd_nodes: &[NJDNode]) -> Vec<Label> {
    let utterance = Utterance::from(njd_nodes);
    utterance_to_features(&utterance)
}

/// フルコンテキストラベルと、その音素を含む入力 NJD ノードの添字。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LabelWithSource {
    pub label: Label,
    /// `sil` と `pau` は `None`、発音を持つ音素は元のノードの添字です。
    /// 長音だけのノードが直前の語に結合された場合は、直前の語の添字を返します。
    pub source_index: Option<usize>,
}

/// NJD ノードからフルコンテキストラベルと音素ごとの元のノードの添字を生成します。
pub fn njdnodes_to_features_with_sources(njd_nodes: &[NJDNode]) -> Vec<LabelWithSource> {
    let utterance = Utterance::from(njd_nodes);
    utterance_to_features_with_sources(&utterance)
}

/// 音素と、その音素を含む入力 NJD ノードの添字。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhonemeWithSource {
    pub phoneme: String,
    /// `sil` と `pau` は `None`、発音を持つ音素は元のノードの添字です。
    /// 長音だけのノードが直前の語に結合された場合は、直前の語の添字を返します。
    pub source_index: Option<usize>,
}

/// NJD ノードから音素と元のノードの添字を生成します。
/// 文頭・文末に `sil`、息継ぎの位置に `pau` を含みます。
pub fn njdnodes_to_phonemes_with_sources(njd_nodes: &[NJDNode]) -> Vec<PhonemeWithSource> {
    utterance_to_phonemes_with_sources(&Utterance::from(njd_nodes))
}

#[cfg(test)]
mod tests {
    use haqumei_jpreprocess_core::{pronunciation::Pronunciation, word_details::WordDetails};

    use super::*;

    fn node(pron: &str, accent: usize, chain: bool) -> NJDNode {
        NJDNode::from_details(
            pron.to_owned(),
            WordDetails {
                pron: Pronunciation::parse(pron, accent).unwrap(),
                chain_flag: Some(chain),
                ..WordDetails::default()
            },
        )
    }

    #[test]
    fn sources_follow_long_vowels_and_pause_boundaries() {
        let nodes = vec![
            node("キ’", 1, false),
            node("ー", 0, false),
            node("、", 0, false),
            node("ー", 0, false),
            node("ア", 0, false),
            node("？", 0, false),
            node("！", 0, false),
            node("イ", 1, false),
        ];
        let features = njdnodes_to_features_with_sources(&nodes);
        let phonemes = njdnodes_to_phonemes_with_sources(&nodes);
        assert_eq!(
            phonemes,
            features
                .iter()
                .map(|feature| PhonemeWithSource {
                    phoneme: feature.label.phoneme.c.clone().unwrap(),
                    source_index: feature.source_index,
                })
                .collect::<Vec<_>>()
        );
        let actual: Vec<_> = features
            .iter()
            .map(|feature| {
                (
                    feature.label.phoneme.c.as_deref().unwrap(),
                    feature.source_index,
                )
            })
            .collect();
        assert_eq!(
            actual,
            [
                ("sil", None),
                ("k", Some(0)),
                ("I", Some(0)),
                ("I", Some(0)),
                ("pau", None),
                ("a", Some(4)),
                ("pau", None),
                ("i", Some(7)),
                ("sil", None),
            ]
        );
        let phrase = features[5].label.accent_phrase_curr.as_ref().unwrap();
        assert!(phrase.is_interrogative);
        assert!(phrase.is_exclamatory);
        assert_eq!(phrase.mora_count, 1);
        let pause = features[6].label.accent_phrase_prev.as_ref().unwrap();
        assert!(pause.is_interrogative);
        assert!(pause.is_exclamatory);
        assert_eq!(features[0].label.utterance.breath_group_count, 3);
        assert_eq!(features[0].label.utterance.mora_count, 4);
    }

    #[test]
    fn leading_long_vowels_keep_their_source_inside_the_previous_phrase() {
        let nodes = [node("キ", 1, false), node("ーーア", 1, false)];
        let features = njdnodes_to_features_with_sources(&nodes);
        let sources: Vec<_> = features
            .iter()
            .map(|feature| feature.source_index)
            .collect();
        assert_eq!(
            sources,
            [None, Some(0), Some(0), Some(1), Some(1), Some(1), None]
        );
        assert_eq!(
            njdnodes_to_phonemes_with_sources(&nodes),
            features
                .iter()
                .map(|f| PhonemeWithSource {
                    phoneme: f.label.phoneme.c.clone().unwrap(),
                    source_index: f.source_index,
                })
                .collect::<Vec<_>>()
        );
        assert_eq!(
            features[1]
                .label
                .accent_phrase_curr
                .as_ref()
                .unwrap()
                .mora_count,
            3
        );
        assert_eq!(
            features[5]
                .label
                .accent_phrase_curr
                .as_ref()
                .unwrap()
                .mora_count,
            1
        );
    }

    #[test]
    fn a_long_vowel_can_be_unvoiced_without_changing_the_preceding_mora() {
        let features = njdnodes_to_features(&[node("ツォー’", 1, false)]);
        let phonemes: Vec<_> = features
            .iter()
            .map(|feature| feature.phoneme.c.as_deref().unwrap())
            .collect();
        assert_eq!(phonemes, ["sil", "ts", "o", "O", "sil"]);
    }

    #[test]
    fn leading_long_vowels_do_not_take_phonemes_across_a_pause() {
        let nodes = [
            node("キ", 1, false),
            node("、", 0, false),
            node("ーーア", 1, false),
        ];
        let phonemes = njdnodes_to_phonemes_with_sources(&nodes);
        let actual: Vec<_> = phonemes
            .iter()
            .map(|p| (p.phoneme.as_str(), p.source_index))
            .collect();
        assert_eq!(
            actual,
            [
                ("sil", None),
                ("k", Some(0)),
                ("i", Some(0)),
                ("pau", None),
                ("a", Some(2)),
                ("sil", None)
            ]
        );
    }

    #[test]
    fn unvoicing_after_a_long_geminate_keeps_the_consonant() {
        let features = njdnodes_to_features(&[node("ッー’", 1, false)]);
        let phonemes: Vec<_> = features
            .iter()
            .map(|feature| feature.phoneme.c.as_deref().unwrap())
            .collect();
        assert_eq!(phonemes, ["sil", "cl", "cl", "sil"]);
    }

    #[test]
    fn explicit_chaining_keeps_a_pause_inside_the_accent_phrase() {
        let nodes = [
            node("ア", 1, false),
            node("、", 0, false),
            node("イ", 0, true),
        ];
        let features = njdnodes_to_features_with_sources(&nodes);
        let phonemes = njdnodes_to_phonemes_with_sources(&nodes);
        let actual: Vec<_> = phonemes
            .iter()
            .map(|feature| (feature.phoneme.as_str(), feature.source_index))
            .collect();
        assert_eq!(
            actual,
            [
                ("sil", None),
                ("a", Some(0)),
                ("pau", None),
                ("i", Some(2)),
                ("sil", None)
            ]
        );
        assert_eq!(features[1].label.utterance.breath_group_count, 1);
        assert_eq!(features[1].label.utterance.accent_phrase_count, 1);
        assert_eq!(features[3].label.mora.as_ref().unwrap().position_forward, 2);
        assert_eq!(
            features[2]
                .label
                .accent_phrase_prev
                .as_ref()
                .unwrap()
                .mora_count,
            2
        );
        assert_eq!(
            features[2]
                .label
                .accent_phrase_next
                .as_ref()
                .unwrap()
                .mora_count,
            2
        );
    }

    #[test]
    fn empty_and_punctuation_only_inputs_have_no_labels() {
        assert!(njdnodes_to_features(&[]).is_empty());
        assert!(
            njdnodes_to_features(&[
                node("ー", 0, false),
                node("？", 0, false),
                node("！", 0, false),
                node("、", 0, false),
                node("*", 0, false),
            ])
            .is_empty()
        );
    }
}
