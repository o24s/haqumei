//! 品詞・単語の核・分割を、数詞処理とアクセント結合の前に確定する。

use std::{ops::Range, sync::LazyLock};

use haqumei_jpreprocess_core::{
    accent_rule::{AccentType, ChainRule, ChainRules},
    cform::CForm,
    ctype::CType,
    pos::{Meishi, POS, Setsubi},
    pronunciation::Pronunciation,
    word_details::WordDetails,
};
use haqumei_jpreprocess_njd::NJDNode;
use rustc_hash::FxHashMap;

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

static RULES_BY_SURFACE: LazyLock<FxHashMap<&'static str, Vec<&'static RewriteRule>>> =
    LazyLock::new(|| {
        let mut rules: FxHashMap<&'static str, Vec<&'static RewriteRule>> = FxHashMap::default();
        for rule in PREPARE_RULES {
            rules.entry(rule.surface).or_default().push(rule);
        }
        rules
    });

/// 駅名を数詞へ分割すると空白を残す条件も変わるため、空白処理より先に実行する。
pub(crate) fn prepare_context_segments(nodes: &mut Vec<NJDNode>, protected: &mut Vec<bool>) {
    apply_rules(nodes, protected, Stage::Segmentation);
}

pub(crate) fn prepare_context_words(nodes: &mut Vec<NJDNode>, protected: &mut Vec<bool>) {
    apply_rules(nodes, protected, Stage::Lexical);
}

fn apply_rules(nodes: &mut Vec<NJDNode>, protected: &mut Vec<bool>, stage: Stage) {
    debug_assert_eq!(nodes.len(), protected.len());
    let by_surface = &*RULES_BY_SURFACE;
    let mut i = 0;
    while i < nodes.len() {
        let matched = by_surface.get(nodes[i].get_string()).and_then(|rules| {
            rules.iter().copied().find_map(|rule| {
                if rule.stage != stage {
                    return None;
                }
                let matched = (rule.context)(nodes, i)?;
                (!protected[matched.protect.clone()]
                    .iter()
                    .any(|&value| value))
                .then_some((rule, matched))
            })
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

pub(super) fn 住所の字(nodes: &[NJDNode], i: usize) -> Option<RewriteMatch> {
    (i > 0
        && matches!(nodes[i].get_pos(), POS::Meishi(_))
        && matches!(nodes[i - 1].get_string(), "市" | "区" | "町" | "村")
        && matches!(
            nodes[i - 1].get_pos(),
            POS::Meishi(Meishi::Setsubi(Setsubi::Region))
        )
        && nodes.get(i + 1).is_some_and(|word| {
            matches!(
                word.get_pos(),
                POS::Meishi(Meishi::General | Meishi::KoyuMeishi(_))
            )
        }))
    .then(|| RewriteMatch {
        replace: i..i + 1,
        protect: i - 1..i + 2,
    })
}

pub(super) fn 敬称の方(nodes: &[NJDNode], i: usize) -> Option<RewriteMatch> {
    if !matches!(nodes[i].get_pos(), POS::Meishi(_)) {
        return None;
    }
    let next = nodes.get(i + 1)?;
    let 世話になった相手 = i >= 4
        && matches!(next.get_string(), "に" | "へ" | "を")
        && nodes[i - 1].get_string() == "た"
        && nodes[i - 2].get_orig() == Some("なる")
        && nodes[i - 3].get_string() == "に"
        && nodes[i - 4].get_string() == "お世話";
    let mut predicate = i + 2;
    if nodes
        .get(predicate)
        .is_some_and(|word| matches!(word.get_string(), "どちら" | "どこ"))
        && nodes
            .get(predicate + 1)
            .is_some_and(|word| word.get_string() == "に")
    {
        predicate += 2;
    }
    let 人の所在 = i >= 2
        && nodes[i - 1].get_string() == "の"
        && matches!(nodes[i - 2].get_string(), "担当" | "受付" | "係")
        && matches!(next.get_string(), "は" | "が")
        && nodes
            .get(predicate)
            .is_some_and(|word| word.get_orig() == Some("いらっしゃる"));
    (世話になった相手 || 人の所在).then_some(RewriteMatch {
        replace: i..i + 1,
        protect: i.saturating_sub(4)..(i + 5).min(nodes.len()),
    })
}

/// 数詞の直後で「タケ」と解析された「丈」の置換区間を返す。
pub(super) fn 丈(nodes: &[NJDNode], i: usize) -> Option<RewriteMatch> {
    (i > 0
        && matches!(nodes[i - 1].get_pos(), POS::Meishi(Meishi::Kazu))
        && matches!(nodes[i].get_pos(), POS::Meishi(Meishi::General))
        && nodes[i].get_read() == Some("タケ"))
    .then_some(RewriteMatch {
        replace: i..i + 1,
        protect: i.saturating_sub(1)..i + 1,
    })
}

/// 「ゴヘ」と解析された「五部」が部数を表す場合に、置換区間を返す。
pub(super) fn 五部(nodes: &[NJDNode], i: usize) -> Option<RewriteMatch> {
    if nodes[i].get_read() != Some("ゴヘ") {
        return None;
    }
    let next = nodes.get(i + 1)?;
    let end = match next.get_orig() {
        Some("買う" | "刷る") if matches!(next.get_pos(), POS::Doushi(_)) => i + 2,
        Some("用意" | "印刷" | "配布" | "複製" | "増刷" | "作成")
            if matches!(next.get_pos(), POS::Meishi(Meishi::SahenSetsuzoku))
                && nodes.get(i + 2).is_some_and(|verb| {
                    verb.get_orig() == Some("する") && matches!(verb.get_pos(), POS::Doushi(_))
                }) =>
        {
            i + 3
        }
        _ => return None,
    };
    Some(RewriteMatch {
        replace: i..i + 1,
        protect: i..end,
    })
}

/// 「十重に囲む」などの「十・重」を一語にまとめる区間を返す。
pub(super) fn 十重(nodes: &[NJDNode], i: usize) -> Option<RewriteMatch> {
    (nodes
        .get(i + 1)
        .is_some_and(|next| next.get_string() == "重")
        && next_via(nodes, i + 1, "に", &["囲む", "囲う", "重なる", "重ねる"]))
    .then_some(RewriteMatch {
        replace: i..i + 2,
        protect: i..i + 4,
    })
}

/// 「従」の直後に続く一位〜八位の終端と、三位かどうかを返す。
fn 位階の末尾(nodes: &[NJDNode], i: usize) -> Option<(usize, bool)> {
    let next = nodes.get(i + 1)?.get_string();
    if matches!(
        next,
        "一位" | "二位" | "三位" | "四位" | "五位" | "六位" | "七位" | "八位"
    ) {
        return Some((i + 2, next == "三位"));
    }
    (matches!(next, "一" | "二" | "三" | "四" | "五" | "六" | "七" | "八")
        && nodes
            .get(i + 2)
            .is_some_and(|node| node.get_string() == "位"))
    .then_some((i + 3, next == "三"))
}

/// 「従・三位」「従・三・位」の置換区間を返す。
pub(super) fn 従三位(nodes: &[NJDNode], i: usize) -> Option<RewriteMatch> {
    let (end, third) = 位階の末尾(nodes, i)?;
    third.then_some(RewriteMatch {
        replace: i..end,
        protect: i..end,
    })
}

/// 一位〜八位が続く「従」の置換区間を返す。
pub(super) fn 従(nodes: &[NJDNode], i: usize) -> Option<RewriteMatch> {
    let (end, _) = 位階の末尾(nodes, i)?;
    Some(RewriteMatch {
        replace: i..i + 1,
        protect: i..end,
    })
}

/// 「従」に続く数詞と「位」の置換区間を返す。
pub(super) fn 位階の数詞(nodes: &[NJDNode], i: usize) -> Option<RewriteMatch> {
    (i > 0
        && nodes[i - 1].get_string() == "従"
        && nodes[i - 1].get_read() == Some("ジュ")
        && nodes
            .get(i + 1)
            .is_some_and(|node| node.get_string() == "位"))
    .then_some(RewriteMatch {
        replace: i..i + 2,
        protect: i.saturating_sub(1)..i + 2,
    })
}

/// 「円」で終わる金額の前に付く「金」の置換区間を返す。
pub(super) fn 金額の金(nodes: &[NJDNode], i: usize) -> Option<RewriteMatch> {
    let mut saw_number = false;
    for (end, node) in nodes.iter().enumerate().skip(i + 1) {
        let surface = node.get_string();
        if surface == "円" {
            return saw_number.then_some(RewriteMatch {
                replace: i..i + 1,
                protect: i..end + 1,
            });
        }
        if surface.is_empty()
            || !surface.chars().all(|c| {
                matches!(c, '0'..='9' | '０'..='９' | '〇' | '零' | '一' | '二' | '三'
                    | '四' | '五' | '六' | '七' | '八' | '九' | '十' | '百' | '千'
                    | '万' | '億' | '兆' | '壱' | '弐' | '貳' | '参')
            })
        {
            break;
        }
        saw_number = true;
    }
    None
}

/// 恩恵・被害を受ける「被り」の置換区間を返す。
pub(super) fn 被り(nodes: &[NJDNode], i: usize) -> Option<RewriteMatch> {
    if nodes[i].get_orig() != Some("被る")
        || *nodes[i].get_cform() != CForm::Renyou
        || !matches!(nodes[i].get_pos(), POS::Doushi(_))
    {
        return None;
    }
    super::previous_via_particle(
        nodes,
        i,
        &["を"],
        &[
            "恩情",
            "恩恵",
            "損害",
            "被害",
            "迷惑",
            "損失",
            "不利益",
            "災難",
            "災厄",
        ],
    )
    .then_some(RewriteMatch {
        replace: i..i + 1,
        protect: i..i + 1,
    })
}

/// 植物の根元を指す「根本」の置換区間を返す。
pub(super) fn 根本(nodes: &[NJDNode], i: usize) -> Option<RewriteMatch> {
    (i >= 2
        && matches!(nodes[i].get_pos(), POS::Meishi(_))
        && nodes[i - 1].get_string() == "の"
        && matches!(
            nodes[i - 2].get_string(),
            "木" | "草" | "樹木" | "幹" | "茎"
        )
        && nodes.get(i + 1).is_none_or(|node| {
            matches!(
                node.get_pos(),
                POS::Joshi(_) | POS::Jodoushi | POS::Kigou(_)
            )
        }))
    .then_some(RewriteMatch {
        replace: i..i + 1,
        protect: i.saturating_sub(2)..i + 1,
    })
}

/// 文末で動詞の連用形に続く「候」の置換区間を返す。
pub(super) fn 候(nodes: &[NJDNode], i: usize) -> Option<RewriteMatch> {
    (i > 0
        && matches!(nodes[i].get_pos(), POS::Meishi(_))
        && matches!(nodes[i - 1].get_pos(), POS::Doushi(_))
        && *nodes[i - 1].get_cform() == CForm::Renyou
        && nodes
            .get(i + 1)
            .is_none_or(|node| matches!(node.get_pos(), POS::Kigou(_))))
    .then_some(RewriteMatch {
        replace: i..i + 1,
        protect: i.saturating_sub(1)..i + 1,
    })
}

/// 「鬼の居ぬ間」の「間」の置換区間を返す。
pub(super) fn 鬼の居ぬ間(nodes: &[NJDNode], i: usize) -> Option<RewriteMatch> {
    (i >= 4
        && nodes[i - 4].get_string() == "鬼"
        && nodes[i - 3].get_string() == "の"
        && nodes[i - 2].get_orig() == Some("居る")
        && nodes[i - 1].get_string() == "ぬ")
        .then_some(RewriteMatch {
            replace: i..i + 1,
            protect: i.saturating_sub(4)..i + 1,
        })
}

/// 「話の間を取る」などの「間」の置換区間を返す。
pub(super) fn 話の間(nodes: &[NJDNode], i: usize) -> Option<RewriteMatch> {
    if !matches!(
        nodes[i].get_pos(),
        POS::Meishi(Meishi::General | Meishi::FukushiKanou)
    ) {
        return None;
    }
    let mut before = i.checked_sub(1)?;
    if matches!(nodes[before].get_string(), "「" | "『") {
        before = before.checked_sub(1)?;
    }
    let head = before.checked_sub(1)?;
    if nodes[before].get_string() != "の"
        || !matches!(
            nodes[head].get_string(),
            "話" | "会話" | "台詞" | "せりふ" | "セリフ"
        )
    {
        return None;
    }
    let mut after = i + 1;
    if nodes
        .get(after)
        .is_some_and(|node| matches!(node.get_string(), "」" | "』"))
    {
        after += 1;
    }
    (nodes
        .get(after)
        .is_some_and(|node| node.get_string() == "を")
        && nodes.get(after + 1).is_some_and(|node| {
            matches!(node.get_pos(), POS::Doushi(_))
                && matches!(node.get_orig(), Some("取る" | "とる" | "置く" | "おく"))
        }))
    .then_some(RewriteMatch {
        replace: i..i + 1,
        protect: head..after + 2,
    })
}

/// 「縁を断った」などの「断っ」の置換区間を返す。
pub(super) fn 断つ(nodes: &[NJDNode], i: usize) -> Option<RewriteMatch> {
    if !matches!(nodes[i].get_pos(), POS::Doushi(_)) {
        return None;
    }
    super::previous_via_particle(
        nodes,
        i,
        &["を"],
        &["縁", "関係", "絆", "退路", "悪習", "悪癖"],
    )
    .then_some(RewriteMatch {
        replace: i..i + 1,
        protect: i..i + 1,
    })
}

/// 「仏の教え」など、信仰の対象を指す「仏」の置換区間を返す。
pub(super) fn 仏(nodes: &[NJDNode], i: usize) -> Option<RewriteMatch> {
    let is_country_pair = i > 0
        && matches!(
            nodes[i - 1].get_string(),
            "と" | "・" | "日" | "米" | "英" | "独" | "伊"
        );
    (!is_country_pair
        && nodes.get(i + 1).is_some_and(|n| n.get_string() == "の")
        && nodes
            .get(i + 2)
            .is_some_and(|n| matches!(n.get_string(), "教え" | "慈悲" | "加護")))
    .then_some(RewriteMatch {
        replace: i..i + 1,
        protect: i..i + 1,
    })
}

fn next_via(nodes: &[NJDNode], i: usize, particle: &str, candidates: &[&str]) -> bool {
    nodes
        .get(i + 1)
        .is_some_and(|node| node.get_string() == particle)
        && nodes.get(i + 2).is_some_and(|node| {
            node.get_orig()
                .is_some_and(|orig| candidates.contains(&orig))
        })
}

/// 地理上の区画を指す「大字」の置換区間を返す。
pub(super) fn 大字(nodes: &[NJDNode], i: usize) -> Option<RewriteMatch> {
    next_via(nodes, i, "の", &["境界", "境界線", "区域", "地番", "住所"]).then_some(RewriteMatch {
        replace: i..i + 1,
        protect: i..i + 1,
    })
}

/// 「五分前」など、時間を表す「五分」の置換区間を返す。
pub(super) fn 五分(nodes: &[NJDNode], i: usize) -> Option<RewriteMatch> {
    // 前金が「前」「金」に分かれる場合も、支払う割合のゴブを残す。
    let is_payment = nodes
        .get(i + 1)
        .is_some_and(|node| matches!(node.get_string(), "前" | "後"))
        && nodes.get(i + 2).is_some_and(|node| {
            matches!(
                node.get_string(),
                "金" | "払" | "払い" | "渡" | "渡し" | "渡金"
            )
        });
    (nodes
        .get(i + 1)
        .is_some_and(|node| matches!(node.get_string(), "前" | "後" | "間"))
        && !is_payment)
        .then_some(RewriteMatch {
            replace: i..i + 1,
            protect: i..i + 1,
        })
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
    fn 敬称の方は名詞と単語の核を結合前に確定する() {
        let nodes = vec![
            NJDNode::new_single("受付,名詞,サ変接続,*,*,*,*,受付,ウケツケ,ウケツケ,0/4,C2"),
            NJDNode::new_single("の,助詞,連体化,*,*,*,*,の,ノ,ノ,1/1,*"),
            NJDNode::new_single("方,名詞,非自立,一般,*,*,*,方,ホウ,ホー,1/2,F2@1"),
            NJDNode::new_single("が,助詞,格助詞,一般,*,*,*,が,ガ,ガ,0/1,*"),
            NJDNode::new_single(
                "いらっしゃる,動詞,自立,*,*,五段・ラ行特殊,基本形,いらっしゃる,イラッシャル,イラッシャル,4/5,*",
            ),
        ];
        let mut protected_nodes = nodes.clone();
        let mut protection = vec![false; nodes.len()];
        protection[2] = true;
        prepare_context_words(&mut protected_nodes, &mut protection);
        assert_eq!(protected_nodes, nodes);

        let mut corrected = nodes;
        prepare_context_words(&mut corrected, &mut vec![false; 5]);
        assert_eq!(corrected[2].get_pos(), &POS::Meishi(Meishi::General));
        assert_eq!(corrected[2].get_read(), Some("カタ"));
        assert_eq!(corrected[2].get_pron().accent(), 2);
        assert_eq!(corrected[2].get_chain_rule().to_string(), "C3");
    }

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
