//! 品詞・単語の核・分割を、数詞処理とアクセント結合の前に確定する。

use std::ops::Range;

use haqumei_jpreprocess_core::{
    accent_rule::{AccentType, ChainRule, ChainRules},
    cform::CForm,
    ctype::CType,
    pos::{Meishi, POS},
    pronunciation::Pronunciation,
    word_details::WordDetails,
};
use haqumei_jpreprocess_njd::NJDNode;

use super::PREPARE_RULES;

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Stage {
    Segmentation,
    Lexical,
}

pub(super) struct NodeSpec {
    surface: &'static str,
    pos: POS,
    ctype: CType,
    cform: CForm,
    orig: &'static str,
    read: &'static str,
    pron: &'static str,
    /// 結合前の単語の核。0 は平板で、句全体の核とは区別する。
    accent: usize,
    chain_rule: Option<AccentType>,
}

impl NodeSpec {
    pub(super) const fn new(
        surface: &'static str,
        pos: POS,
        read: &'static str,
        pron: &'static str,
        accent: usize,
        chain_rule: Option<AccentType>,
    ) -> Self {
        Self {
            surface,
            pos,
            read,
            pron,
            accent,
            chain_rule,
            ctype: CType::None,
            cform: CForm::None,
            orig: surface,
        }
    }

    pub(super) const fn conjugation(
        mut self,
        ctype: CType,
        cform: CForm,
        orig: &'static str,
    ) -> Self {
        self.ctype = ctype;
        self.cform = cform;
        self.orig = orig;
        self
    }

    fn node(&self) -> NJDNode {
        let mut chain_rule = ChainRules::default();
        if let Some(kind) = &self.chain_rule {
            chain_rule.default = Some(ChainRule::new(kind.clone(), 0));
        }
        NJDNode::from_details(
            self.surface.into(),
            WordDetails {
                pos: self.pos,
                ctype: self.ctype,
                cform: self.cform,
                orig: Some(self.orig.into()),
                read: Some(self.read.into()),
                pron: Pronunciation::parse(self.pron, self.accent).unwrap(),
                chain_rule,
                ..WordDetails::default()
            },
        )
    }
}

/// 置換区間と、ユーザー辞書の読みを保護する区間。
/// 駅は置換しなくても、駅の直前の数詞・地名の判定に関わる。
pub(super) struct RewriteMatch {
    replace: Range<usize>,
    protect: Range<usize>,
}

pub(super) struct RewriteRule {
    pub surface: &'static str,
    pub stage: Stage,
    pub context: fn(&[NJDNode], usize) -> Option<RewriteMatch>,
    pub replacement: &'static [NodeSpec],
}

/// 駅名を数詞へ分割すると空白を残す条件も変わるため、空白処理より先に実行する。
pub(crate) fn prepare_context_segments(nodes: &mut Vec<NJDNode>, protected: &mut Vec<bool>) {
    apply_rules(nodes, protected, Stage::Segmentation);
}

pub(crate) fn prepare_context_words(nodes: &mut Vec<NJDNode>, protected: &mut Vec<bool>) {
    apply_rules(nodes, protected, Stage::Lexical);
}

fn apply_rules(nodes: &mut Vec<NJDNode>, protected: &mut Vec<bool>, stage: Stage) {
    debug_assert_eq!(nodes.len(), protected.len());
    let mut i = 0;
    while i < nodes.len() {
        let matched = PREPARE_RULES.iter().find_map(|rule| {
            if rule.stage != stage || rule.surface != nodes[i].get_string() {
                return None;
            }
            let matched = (rule.context)(nodes, i)?;
            (!protected[matched.protect.clone()]
                .iter()
                .any(|&value| value))
            .then_some((rule, matched))
        });
        if let Some((rule, matched)) = matched {
            let next = matched.replace.start + rule.replacement.len();
            nodes.splice(
                matched.replace.clone(),
                rule.replacement.iter().map(NodeSpec::node),
            );
            protected.splice(
                matched.replace,
                std::iter::repeat_n(false, rule.replacement.len()),
            );
            i = next;
        } else {
            i += 1;
        }
    }
}

fn kata_suffix_candidate(node: &NJDNode) -> bool {
    node.get_string() == "方" && node.get_read() == Some("ホウ")
}

pub(super) fn nominal_kata_stem(nodes: &[NJDNode], i: usize, read: &str) -> Option<RewriteMatch> {
    (matches!(nodes[i].get_pos(), POS::Meishi(Meishi::General))
        && nodes[i].get_read() == Some(read)
        && nodes.get(i + 1).is_some_and(kata_suffix_candidate))
    .then_some(RewriteMatch {
        replace: i..i + 2,
        protect: i..i + 2,
    })
}

pub(super) fn verb_kata_suffix(nodes: &[NJDNode], i: usize) -> Option<RewriteMatch> {
    (i > 0
        && kata_suffix_candidate(&nodes[i])
        && matches!(nodes[i - 1].get_pos(), POS::Doushi(_))
        && *nodes[i - 1].get_cform() == CForm::Renyou)
        .then(|| RewriteMatch {
            replace: i..i + 1,
            protect: i - 1..i + 1,
        })
}

pub(super) fn station_count(nodes: &[NJDNode], i: usize) -> Option<RewriteMatch> {
    (nodes[i].get_read() == Some("ジュウソウ") && station_context(nodes, i, 1) == Some(true))
        .then_some(RewriteMatch {
            replace: i..i + 1,
            protect: i..i + 2,
        })
}

pub(super) fn station_name(nodes: &[NJDNode], i: usize) -> Option<RewriteMatch> {
    (nodes[i].get_pos().is_kazu()
        && nodes[i].roman_source().is_none()
        && nodes.get(i + 1).is_some_and(|n| {
            n.get_string() == "三" && n.get_pos().is_kazu() && n.roman_source().is_none()
        })
        && station_context(nodes, i, 2) == Some(false))
    .then_some(RewriteMatch {
        replace: i..i + 2,
        protect: i..i + 3,
    })
}

/// 数量なら true、駅名の手がかりがあれば false、決まらなければ None。
fn station_context(nodes: &[NJDNode], i: usize, width: usize) -> Option<bool> {
    let station = i + width;
    if nodes.get(station).is_none_or(|n| n.get_string() != "駅") {
        return None;
    }
    let previous = i.checked_sub(1).map(|at| &nodes[at]);
    let prefix = nodes[i.saturating_sub(5)..i]
        .iter()
        .map(NJDNode::get_string)
        .collect::<String>();
    let numeric_prefix = previous.is_some_and(|n| {
        n.get_pos().is_kazu()
            || matches!(
                n.get_string(),
                "第" | "計"
                    | "全"
                    | "合計"
                    | "約"
                    | "およそ"
                    | "あと"
                    | "残り"
                    | "たった"
                    | "わずか"
            )
    }) || ["全部で", "合計で", "合わせて", "残りは"]
        .iter()
        .any(|cue| prefix.ends_with(cue));
    let next = nodes.get(station + 1);
    let railway = ["阪急", "阪急の", "阪急電鉄", "阪急電鉄の"]
        .iter()
        .any(|cue| prefix.ends_with(cue));
    let station_pair = previous.is_some_and(|n| {
        matches!(
            n.get_string(),
            "・" | "―" | "—" | "－" | "-" | "〜" | "～" | "と"
        )
    }) && i >= 2
        && (nodes[i - 2].get_string() == "駅"
            || matches!(nodes[i - 2].get_pos(), POS::Meishi(Meishi::KoyuMeishi(_))));
    let interval_count = next.is_some_and(|n| n.get_string() == "間") && !railway && !station_pair;
    if numeric_prefix
        // 「十三駅間を走る」は距離だが、鉄道名や二駅の列挙があれば駅名を残す。
        || interval_count
        || next.is_some_and(|n| matches!(n.get_string(), "先" | "目" | "分"))
        || (next.is_some_and(|n| n.get_string() == "から")
            && nodes
                .get(station + 2)
                .is_some_and(|n| matches!(n.get_orig(), Some("なる" | "成る"))))
    {
        return Some(true);
    }
    let destination = ["駅先に", "駅先には"]
        .iter()
        .any(|cue| prefix.ends_with(cue));
    let arrival = next.is_some_and(|n| matches!(n.get_string(), "で" | "に" | "へ" | "から"))
        && nodes.get(station + 2).is_some_and(|n| {
            matches!(
                n.get_orig(),
                Some(
                    "乗る"
                        | "降りる"
                        | "乗り換える"
                        | "乗換える"
                        | "到着"
                        | "集合"
                        | "待ち合わせ"
                        | "乗車"
                        | "下車"
                )
            )
        });
    (railway || station_pair || destination || arrival).then_some(false)
}

#[cfg(test)]
mod tests {
    use haqumei_jpreprocess_njd::{NJD, accent_phrase, accent_type};

    use super::*;

    #[test]
    fn word_accents_and_suffix_rule_precede_phrase_accent() {
        let mut nodes = vec![
            NJDNode::new_single("捻れ,名詞,一般,*,*,*,*,捻れ,ネジレ,ネジレ,0/3,C1"),
            NJDNode::new_single("方,名詞,非自立,一般,*,*,*,方,ホウ,ホー,1/2,*"),
        ];
        prepare_context_words(&mut nodes, &mut vec![false; 2]);
        assert_eq!(nodes[0].get_pron().accent(), 3);
        assert_eq!(nodes[1].get_pron().accent(), 1);
        assert_eq!(nodes[1].get_pron().mora_size(), 2);
        assert_eq!(nodes[1].get_chain_rule().to_string(), "C4");

        let mut njd = NJD { nodes };
        accent_phrase::njd_set_accent_phrase(&mut njd);
        accent_type::njd_set_accent_type(&mut njd);
        assert_eq!(njd.nodes[1].get_chain_flag(), Some(true));
        assert_eq!(njd.nodes[0].get_pron().accent(), 0);
    }

    #[test]
    fn protected_station_keeps_both_segmentation_and_protection_indices() {
        let mut nodes = vec![
            NJDNode::new_single(
                "十三,名詞,固有名詞,地域,一般,*,*,十三,ジュウソウ,ジューソー,0/4,C2",
            ),
            NJDNode::new_single("駅,名詞,接尾,地域,*,*,*,駅,エキ,エキ,1/2,C1"),
            NJDNode::new_single("先,名詞,接尾,一般,*,*,*,先,サキ,サキ,0/2,C3"),
        ];
        let original = nodes.clone();
        let mut protected = vec![false, true, false];
        prepare_context_segments(&mut nodes, &mut protected);
        assert_eq!(nodes, original);
        assert_eq!(protected, [false, true, false]);

        protected[1] = false;
        prepare_context_segments(&mut nodes, &mut protected);
        assert_eq!(nodes.len(), 4);
        assert_eq!(nodes[0].get_string(), "十");
        assert_eq!(nodes[1].get_string(), "三");
        assert_eq!(protected, [false; 4]);
    }
}
