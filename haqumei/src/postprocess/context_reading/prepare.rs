//! 品詞・単語の核・分割を、数詞処理とアクセント結合の前に確定する。

use std::{ops::Range, sync::LazyLock};

use haqumei_jpreprocess_core::{
    accent_rule::{AccentType, ChainRule, ChainRules},
    cform::CForm,
    ctype::{CType, Special},
    pos::{Fukushi, Joshi, KakuJoshi, Kigou, KoyuMeishi, Meishi, POS, Setsubi},
    pronunciation::{MoraEnum, Pronunciation},
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
                // ローマ数字などの元表記は、展開元を示す識別子から復元する。
                // ノードを作り直すと識別子が失われるため、展開済みの数詞は置換しない。
                if nodes[matched.replace.clone()]
                    .iter()
                    .any(|node| node.roman_source().is_some() || node.surface_source().is_some())
                {
                    return None;
                }
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

pub(crate) fn prepare_context_ha(
    nodes: &mut Vec<NJDNode>,
    protected: &mut Vec<bool>,
    gaps: &[usize],
    edited: &[usize],
) {
    const BREATH: NodeSpec = NodeSpec::new(
        "はー",
        POS::Fukushi(Fukushi::General),
        "ハア",
        "ハー",
        1,
        None,
    );
    const SIGH: NodeSpec = NodeSpec::new(
        "はー",
        POS::Kandoushi,
        "ハア",
        "ハー",
        1,
        Some(AccentType::C3),
    );
    const PANTING: NodeSpec = NodeSpec::new(
        "はーはー",
        POS::Fukushi(Fukushi::General),
        "ハアハア",
        "ハーハー",
        1,
        None,
    );
    const STUTTER: NodeSpec = NodeSpec::new("は", POS::Filler, "ハ", "ハ", 1, None);
    const TOPIC: &str =
        "は,助詞,係助詞,*,*,*,*,は,ハ,ワ,0/1,名詞%F1/動詞%F2@0/形容詞%F2@0/助詞%F2@0";
    const LETTER: NodeSpec = NodeSpec::new(
        "は",
        POS::Meishi(Meishi::General),
        "ハ",
        "ハ",
        1,
        Some(AccentType::C3),
    );

    // 空白と編集箇所の添字は置換前の形態素列を指すため、後ろから調べる。
    let mut i = nodes.len();
    while i > 0 {
        i -= 1;
        if let Some(laugh) = 笑い声の範囲(nodes, i, gaps)
            && ha_context_available(protected, gaps, edited, laugh.context, laugh.continuous)
        {
            let surface: String = nodes[laugh.replace.clone()]
                .iter()
                .map(NJDNode::get_string)
                .collect();
            let spoken = if laugh.restore_topic {
                surface.strip_prefix('は').unwrap()
            } else {
                &surface
            };
            let spoken = if laugh.split_quote {
                spoken.strip_suffix("っと").unwrap()
            } else {
                spoken
            };
            let reading = spoken.replace('は', "ハ").replace('っ', "ッ");
            let node = NJDNode::from_details(
                spoken.to_owned(),
                WordDetails {
                    pos: POS::Filler,
                    orig: Some(spoken.to_owned()),
                    pron: Pronunciation::parse(&reading, 0).unwrap(),
                    read: Some(reading),
                    ..WordDetails::default()
                },
            );
            i = laugh.replace.start;
            let topic = laugh.restore_topic.then(|| NJDNode::new_single(TOPIC));
            let quote = laugh.split_quote.then(|| {
                NJDNode::new_single(
                    "っと,助詞,格助詞,引用,*,*,*,っと,ット,ット,2/2,名詞%F1/動詞%F2@0/形容詞%F1",
                )
            });
            let replacement = topic.into_iter().chain([node]).chain(quote);
            nodes.splice(laugh.replace.clone(), replacement);
            protected.splice(
                laugh.replace,
                std::iter::repeat_n(
                    false,
                    1 + usize::from(laugh.restore_topic) + usize::from(laugh.split_quote),
                ),
            );
            continue;
        }
        let before = &nodes[..i];
        let after = &nodes[i..];
        let (count, spec, context, continuous) = match nodes[i].get_string() {
            "はぁ" if 係助詞のはぁ(before, after) => {
                if !ha_context_available(protected, gaps, edited, i - 1..i + 3, i - 1..i + 3) {
                    continue;
                }
                nodes[i] = NJDNode::new_single(
                    "はぁ,助詞,係助詞,*,*,*,*,は,ハァ,ワー,0/2,名詞%F1/動詞%F2@0/形容詞%F2@0/助詞%F2@0",
                );
                continue;
            }
            "は" if 係助詞のは(&nodes[i]) => {
                if let Some(context) = 息を表すはー(before, after) {
                    (2, &BREATH, context.clone(), i..context.end)
                } else if let Some(context) = 息切れの反復(before, after) {
                    if ha_context_available(
                        protected,
                        gaps,
                        edited,
                        context.clone(),
                        context.clone(),
                    ) {
                        let first = context.start;
                        let comma = nodes[first + 2].clone();
                        nodes.splice(first..i + 2, [SIGH.node(), comma, SIGH.node()]);
                        protected.splice(first..i + 2, [false; 3]);
                        i = first;
                    }
                    continue;
                } else if 反復するはー(after) {
                    (4, &PANTING, i..i + 5, i..i + 5)
                } else if let Some(context) = 独立したはー(before, after) {
                    // 開き括弧や句点の後に空白があっても発話は始まる。「は」と「ー」は隣接が必要。
                    (2, &SIGH, context, i..i + 2)
                } else if let Some(context) = 言い直しのは(before, after) {
                    (1, &STUTTER, context, i..i + 3)
                } else if let Some(context) = 字としてのは(before, after) {
                    (1, &LETTER, context, i - 1..i + 2)
                } else {
                    continue;
                }
            }
            _ => continue,
        };
        if ha_context_available(protected, gaps, edited, context, continuous) {
            nodes[i] = spec.node();
            nodes.drain(i + 1..i + count);
            protected.drain(i + 1..i + count);
        }
    }
}

fn ha_context_available(
    protected: &[bool],
    gaps: &[usize],
    edited: &[usize],
    context: Range<usize>,
    continuous: Range<usize>,
) -> bool {
    !protected[context.clone()].iter().any(|&value| value)
        && !edited
            .get(edited.partition_point(|&i| i < context.start))
            .is_some_and(|&i| i < context.end)
        && !gaps
            .get(gaps.partition_point(|&i| i <= continuous.start))
            .is_some_and(|&i| i < continuous.end)
}

fn 係助詞のは(node: &NJDNode) -> bool {
    node.get_string() == "は"
        && matches!(node.get_pos(), POS::Joshi(Joshi::KakariJoshi))
        && node.get_pron().mora_matches(MoraEnum::Wa)
}

fn 単独の長音(node: &NJDNode) -> bool {
    node.get_string() == "ー"
        && (node.get_pron().is_empty() || node.get_pron().mora_matches(MoraEnum::Long))
}

fn 係助詞のはぁ(before: &[NJDNode], after: &[NJDNode]) -> bool {
    let ([.., topic], [ha, comma, next, rest @ ..]) = (before, after) else {
        return false;
    };
    // 「がはぁ」の「はぁ」は係助詞にならない。「と」で閉じる発話では感動詞の読みを残す。
    matches!(ha.get_pos(), POS::Kandoushi)
        && 係助詞が付く語(topic)
        // 副詞の分類は「は」の接続可否を表さないため、取り立て用法のある語に限定する。
        && (!matches!(topic.get_pos(), POS::Fukushi(_))
            || crate::data::HA_TOPIC_ADVERBS.contains(topic.get_string()))
        && comma.get_string() == "、"
        && matches!(
            next.get_pos(),
            POS::Meishi(_)
                | POS::Fukushi(_)
                | POS::Doushi(_)
                | POS::Keiyoushi(_)
                | POS::Rentaishi
                | POS::Kandoushi
        )
        && !matches!(next.get_string(), "はぁ" | "はあ" | "はっ")
        && !発話を閉じる引用助詞(rest)
}

fn 発話を閉じる引用助詞(nodes: &[NJDNode]) -> bool {
    let mut rest = nodes.iter();
    for node in rest.by_ref() {
        match node.get_string() {
            "。" | "」" | "』" => return false,
            // 「疲れた！と」の感嘆符は引用に含まれる。直後の引用助詞まで確かめる。
            "！" | "？" => {
                return rest
                    .find(|node| !空白(node) && !matches!(node.get_string(), "！" | "？"))
                    .is_some_and(引用のと);
            }
            _ if 引用のと(node) => return true,
            _ => {}
        }
    }
    false
}

fn 係助詞が付く語(node: &NJDNode) -> bool {
    matches!(node.get_pos(), POS::Meishi(_) | POS::Fukushi(_))
        || matches!(
            node.get_pos(),
            POS::Joshi(Joshi::KakuJoshi(KakuJoshi::General) | Joshi::FukuJoshi)
        ) && matches!(
            node.get_string(),
            "に" | "で" | "へ" | "と" | "から" | "まで" | "より"
        )
}

fn 引用のと(node: &NJDNode) -> bool {
    node.get_string() == "と"
        && matches!(
            node.get_pos(),
            POS::Joshi(Joshi::KakuJoshi(KakuJoshi::Quote))
        )
}

// 三点リーダーは名詞の後にも置けるので、直前の感動詞か文の境界まで確かめる。
fn 発話の先頭(before: &[NJDNode]) -> Option<usize> {
    let end = before
        .iter()
        .rposition(|node| !空白(node))
        .map_or(0, |i| i + 1);
    let start = before[..end]
        .iter()
        .rposition(|node| !matches!(node.get_string(), "…" | "‥") && !空白(node));
    let Some(start) = start else { return Some(0) };
    let boundary = &before[start];
    if matches!(boundary.get_string(), "。" | "？" | "！")
        || matches!(boundary.get_pos(), POS::Kigou(Kigou::KakkoOpen))
        || (start + 1 < end && matches!(boundary.get_pos(), POS::Kandoushi))
    {
        Some(start)
    } else if boundary.get_string() == "、"
        && let Some(prior) = before[..start].iter().rposition(|node| !空白(node))
        && matches!(before[prior].get_pos(), POS::Kandoushi)
    {
        Some(prior)
    } else {
        None
    }
}

fn 独立したはー(before: &[NJDNode], after: &[NJDNode]) -> Option<Range<usize>> {
    let [_, long, rest @ ..] = after else {
        return None;
    };
    let boundary = rest.iter().enumerate().find(|(_, node)| !空白(node));
    if !単独の長音(long)
        || !boundary.is_none_or(|(_, node)| {
            matches!(node.get_pos(), POS::Kigou(Kigou::KakkoClose))
                || matches!(node.get_string(), "、" | "。" | "！" | "？" | "…" | "‥")
        })
    {
        return None;
    }
    let end = boundary.map_or(after.len(), |(offset, _)| offset + 3);
    Some(発話の先頭(before)?..before.len() + end)
}

fn 空白(node: &NJDNode) -> bool {
    matches!(node.get_pos(), POS::Kigou(Kigou::Space))
}

fn 息切れ(nodes: &[NJDNode]) -> bool {
    matches!(nodes, [comma, breath, subject, verb, ..]
        if comma.get_string() == "、" && breath.get_string() == "息"
            && subject.get_string() == "が"
            && matches!(verb.get_orig(), Some("切れる" | "上がる" | "あがる" | "続く")))
}

fn 反復するはー(after: &[NJDNode]) -> bool {
    matches!(after, [_, long, repeat, second_long, verb, ..]
        if 単独の長音(long)
            && 係助詞のは(repeat)
            && 単独の長音(second_long)
            && matches!(verb.get_orig(), Some("する" | "言う" | "いう" | "うなる")))
}

fn 息切れの反復(before: &[NJDNode], after: &[NJDNode]) -> Option<Range<usize>> {
    let [_, long, rest @ ..] = after else {
        return None;
    };
    let [.., first, first_long, comma] = before else {
        return None;
    };
    // 読点で区切った反復には、息切れを述べる文が続くことを確かめる。
    (単独の長音(long)
        && 係助詞のは(first)
        && 単独の長音(first_long)
        && comma.get_string() == "、"
        && 息切れ(rest))
    .then(|| before.len() - 3..before.len() + 6)
}

fn 言い直しのは(before: &[NJDNode], after: &[NJDNode]) -> Option<Range<usize>> {
    let [_, comma, interjection, rest @ ..] = after else {
        return None;
    };
    if comma.get_string() != "、"
        || !matches!(interjection.get_pos(), POS::Kandoushi)
        || !interjection
            .get_pron()
            .moras()
            .first()
            .is_some_and(|mora| mora.mora_enum == MoraEnum::Ha)
    {
        return None;
    }
    let i = before.len();
    let mut following = rest.iter().enumerate().filter(|(_, node)| !空白(node));
    let next = following.next();
    let end = next.map_or(after.len(), |(at, _)| at + 4);
    let marked = next.is_some_and(|(_, node)| matches!(node.get_string(), "！" | "？" | "…" | "‥"));
    let following = following.next();
    if interjection.get_string() == "はい"
        && next.is_some_and(|(_, node)| node.get_string() == "、")
        && following.is_some_and(|(_, node)| node.get_string() == "いいえ")
    {
        return None;
    }
    if let Some(start) = 発話の先頭(before)
        && (marked
            || next.is_none_or(|(_, node)| {
                matches!(node.get_string(), "、" | "。")
                    || matches!(node.get_pos(), POS::Kigou(Kigou::KakkoClose))
            }))
    {
        // 感動詞の直後に助詞が続くときは、その感動詞を語として取り上げた文にもなる。
        let after_interjection = rest.iter().find(|node| {
            !空白(node) && !matches!(node.get_string(), "、" | "！" | "？" | "…" | "‥")
        });
        if after_interjection.is_some_and(|node| matches!(node.get_pos(), POS::Joshi(_)))
            && before
                .get(start)
                .is_some_and(|node| matches!(node.get_pos(), POS::Kandoushi))
        {
            return None;
        }
        // 発話頭で感動詞が独立しているときは、「は」を感動詞の語頭の反復として読む。
        let end = if next.is_some_and(|(_, node)| node.get_string() == "、") {
            following.map_or(end, |(at, _)| at + 4)
        } else {
            end
        };
        return Some(start..i + end);
    }
    let [.., address, comma] = before else {
        return None;
    };
    if !marked
        || comma.get_string() != "、"
        || following.is_some_and(|(_, node)| matches!(node.get_pos(), POS::Joshi(_)))
    {
        return None;
    }
    let address_start = match address.get_pos() {
        POS::Meishi(Meishi::General | Meishi::KoyuMeishi(KoyuMeishi::Person(_))) => i - 2,
        POS::Meishi(Meishi::Setsubi(Setsubi::Person))
            if i >= 3 && matches!(before[i - 3].get_pos(), POS::Meishi(_)) =>
        {
            i - 3
        }
        _ => return None,
    };
    let start = 発話の先頭(&before[..address_start])?;
    Some(start..i + following.map_or(end, |(at, _)| at + 4))
}

fn 助詞(node: &NJDNode, surface: &str) -> bool {
    node.get_string() == surface && matches!(node.get_pos(), POS::Joshi(_))
}

fn 引用先の述語(nodes: &[NJDNode]) -> usize {
    let start = 副詞句の後(nodes);
    if let [subject, particle, rest @ ..] = &nodes[start..]
        && matches!(subject.get_pos(), POS::Meishi(_))
        && (係助詞のは(particle)
            || particle.get_string() == "が"
                && matches!(
                    particle.get_pos(),
                    POS::Joshi(Joshi::KakuJoshi(KakuJoshi::General))
                ))
    {
        return start + 2 + 副詞句の後(rest);
    }
    start
}

fn 副詞句の後(nodes: &[NJDNode]) -> usize {
    let mut end = 0;
    while let Some(node) = nodes.get(end) {
        if 空白(node)
            || matches!(node.get_pos(), POS::Fukushi(_))
            || *node.get_cform() == CForm::RenyouConjunctionTe
                && matches!(node.get_pos(), POS::Keiyoushi(_))
        {
            end += 1;
            continue;
        }
        let noun = end + usize::from(matches!(node.get_pos(), POS::Keiyoushi(_) | POS::Rentaishi));
        let [head, particle, ..] = &nodes[noun..] else {
            break;
        };
        if matches!(head.get_pos(), POS::Meishi(Meishi::General)) && 助詞(particle, "で")
            || matches!(head.get_pos(), POS::Meishi(Meishi::KeiyoudoushiGokan))
                && 助詞(particle, "に")
        {
            end = noun + 2;
        } else {
            break;
        }
    }
    end
}

fn 笑い声を構成するは(node: &NJDNode, last: bool) -> bool {
    if 係助詞のは(node) {
        return true;
    }
    let surface = node.get_string();
    if !surface.starts_with('は') {
        return false;
    }
    match node.get_pos() {
        POS::Filler => {
            let repeated = if last {
                surface.strip_suffix('っ').unwrap_or(surface)
            } else {
                surface
            };
            repeated.chars().all(|ch| ch == 'は')
        }
        POS::Meishi(Meishi::General) => surface == "はは" && node.get_read() == Some("ハハ"),
        POS::Kandoushi => last && surface == "はっ",
        _ => false,
    }
}

struct LaughterMatch {
    replace: Range<usize>,
    context: Range<usize>,
    continuous: Range<usize>,
    restore_topic: bool,
    split_quote: bool,
}

fn 笑い声の範囲(nodes: &[NJDNode], end: usize, gaps: &[usize]) -> Option<LaughterMatch> {
    if !笑い声を構成するは(&nodes[end], true) {
        return None;
    }
    let quote_index = end
        + 1
        + nodes[end + 1..]
            .iter()
            .take_while(|node| 空白(node) || matches!(node.get_string(), "！" | "？" | "…" | "‥"))
            .count();
    let (closing, quote_index) = match nodes.get(quote_index)?.get_string() {
        "」" | "』" => {
            let opening = if nodes[quote_index].get_string() == "」" {
                "「"
            } else {
                "『"
            };
            let after_close = quote_index + 1;
            let spaces = nodes[after_close..]
                .iter()
                .take_while(|node| 空白(node))
                .count();
            (Some(opening), after_close + spaces)
        }
        _ => (None, quote_index),
    };
    let [quote, rest @ ..] = &nodes[quote_index..] else {
        return None;
    };
    let quote_particle = matches!(
        (quote.get_string(), quote.get_pos()),
        (
            "と" | "っと",
            POS::Joshi(Joshi::KakuJoshi(KakuJoshi::Quote))
        ) | ("って", POS::Joshi(Joshi::ShuJoshi))
    );
    let te_after_geminate = closing.is_none()
        && quote_index == end + 1
        && nodes[end].get_string().ends_with('っ')
        && quote.get_string() == "て"
        && matches!(quote.get_pos(), POS::Joshi(Joshi::SetsuzokuJoshi));
    // 辞書が末尾の「は」と「っと」を副詞「はっと」にまとめることがある。
    let fused_quote = closing.is_none()
        && quote_index == end + 1
        && !nodes[end].get_string().ends_with('っ')
        && quote.get_string() == "はっと"
        && matches!(quote.get_pos(), POS::Fukushi(_));
    if !(quote_particle || te_after_geminate || fused_quote) {
        return None;
    }
    let predicate = quote_index
        + 1
        + if closing.is_some() {
            引用先の述語(rest)
        } else {
            副詞句の後(rest)
        };
    let verb = nodes.get(predicate)?;
    let continuation = nodes[predicate + 1..]
        .iter()
        .position(|node| !空白(node))
        .filter(|_| verb.is_renyou())
        .map(|offset| predicate + 1 + offset);
    if !matches!(verb.get_pos(), POS::Doushi(_))
        || !matches!(verb.get_orig(), Some("笑う" | "笑い出す"))
        || continuation.is_some_and(|at| !matches!(nodes[at].get_pos(), POS::Jodoushi))
    {
        return None;
    }

    let mut start = end;
    while start > 0
        && gaps.binary_search(&start).is_err()
        && 笑い声を構成するは(&nodes[start - 1], false)
    {
        start -= 1;
    }
    // 笑い声の前の空白は文脈から除くが、笑い声に含めたり名詞との接続に使ったりしない。
    let before_end = nodes[..start]
        .iter()
        .rposition(|node| !空白(node))
        .map_or(0, |i| i + 1);
    let adjacent = before_end == start && gaps.binary_search(&start).is_err();
    let mut restore_topic = false;
    let context_start = if let Some(opening) = closing {
        let open = nodes[..start].iter().rposition(|node| !空白(node))?;
        if nodes[open].get_string() != opening {
            return None;
        }
        open
    } else {
        match &nodes[..before_end] {
            [.., comma] if comma.get_string() == "、" => before_end - 1,
            [.., prior] if 係助詞のは(prior) => before_end - 1,
            [.., topic]
                if adjacent
                    && matches!(topic.get_pos(), POS::Meishi(_))
                    && 係助詞のは(&nodes[start]) =>
            {
                // 「彼はははと笑った」では「彼」の直後に係助詞を残す。
                start += 1;
                start - 2
            }
            [.., topic]
                if adjacent
                    && matches!(
                        topic.get_pos(),
                        POS::Meishi(
                            Meishi::Daimeishi(_)
                                | Meishi::KoyuMeishi(KoyuMeishi::Person(_))
                                | Meishi::Setsubi(Setsubi::Person)
                        )
                    )
                    && matches!(nodes[start].get_pos(), POS::Filler) =>
            {
                restore_topic = true;
                start - 1
            }
            before => 発話の先頭(before)?,
        }
    };
    let laugh = nodes.get(start..end + 1)?;
    let ha_count = usize::from(fused_quote)
        + laugh
            .iter()
            .map(|node| node.get_string().chars().filter(|&ch| ch == 'は').count())
            .sum::<usize>();
    if ha_count < 2 + usize::from(restore_topic) || !(restore_topic || laugh.iter().any(係助詞のは))
    {
        return None;
    }
    let replace = start..end + 1 + usize::from(fused_quote);
    Some(LaughterMatch {
        continuous: if restore_topic {
            context_start..replace.end
        } else {
            replace.clone()
        },
        replace,
        context: context_start..continuation.unwrap_or(predicate) + 1,
        restore_topic,
        split_quote: fused_quote,
    })
}

fn 字としてのは(before: &[NJDNode], after: &[NJDNode]) -> Option<Range<usize>> {
    let i = before.len();
    let ([.., opening], [_, closing, ..]) = (before, after) else {
        return None;
    };
    if !matches!(
        (opening.get_string(), closing.get_string()),
        ("「", "」") | ("『", "』")
    ) {
        return None;
    }
    // 「助詞の『は』」や「五十音表の『は』」は、字を指す名詞句になる。
    if let [.., noun, no, _] = before
        && 助詞(no, "の")
        && (matches!(noun.get_string(), "助詞" | "ひらがな" | "文字" | "五十音表")
            || (noun.get_string() == "表" && i >= 4 && before[i - 4].get_string() == "五十音"))
    {
        let start = i - if noun.get_string() == "表" { 4 } else { 3 };
        return Some(start..i + 2);
    }
    let [_, _, object, kana, de, verb, ..] = after else {
        return None;
    };
    (助詞(object, "を")
        && matches!(kana.get_string(), "ひらがな" | "平仮名")
        && 助詞(de, "で")
        && verb.get_orig() == Some("書く"))
    .then(|| i - 1..i + 6)
}

fn 息を表すはー(before: &[NJDNode], nodes: &[NJDNode]) -> Option<Range<usize>> {
    let [_, long, quote, breath, object, verb, rest @ ..] = nodes else {
        return None;
    };
    let breath_phrase = 単独の長音(long)
        && matches!(quote.get_string(), "って" | "っと")
        && matches!(quote.get_pos(), POS::Joshi(_))
        && breath.get_string() == "息"
        && matches!(breath.get_pos(), POS::Meishi(_))
        && object.get_string() == "を"
        && matches!(object.get_pos(), POS::Joshi(Joshi::KakuJoshi(_)))
        && matches!(verb.get_pos(), POS::Doushi(_))
        && matches!(
            verb.get_orig(),
            Some(
                "吐く"
                    | "はく"
                    | "吹く"
                    | "ふく"
                    | "吹きかける"
                    | "吹き掛ける"
                    | "ふきかける"
                    | "吹き込む"
                    | "ふきこむ"
                    | "吹きつける"
                    | "吹き付ける"
                    | "ふきつける"
                    | "吐き出す"
                    | "はきだす"
                    | "かける"
                    | "掛ける"
            )
        );
    if !breath_phrase {
        return None;
    }
    let i = before.len();
    let prior = before
        .iter()
        .rposition(|node| !matches!(node.get_pos(), POS::Kigou(Kigou::Space)));
    let start = prior.unwrap_or(i);
    // 係助詞が付かない位置では、呼吸の動作を否定しても音の読みは変わらない。
    if prior.is_none_or(|at| !係助詞が付く語(&before[at])) {
        return Some(start..i + 6);
    }
    // 係助詞にも読める位置では、息を吐く動作を述べる文の形を使う。
    // 「息を吐く時間」のように名詞を修飾する場合は、既存の読みを残す。
    let suffix = rest
        .iter()
        .take_while(|node| 空白(node) || affirmative_ending(node))
        .count();
    let suffix = match &rest[suffix..] {
        [nominalizer, copula, ..]
            if matches!(nominalizer.get_string(), "ん" | "の")
                && matches!(nominalizer.get_pos(), POS::Meishi(Meishi::Hijiritsu(_)))
                && matches!(
                    copula.get_ctype(),
                    CType::Special(Special::Desu | Special::Da)
                )
                && *copula.get_cform() == CForm::Basic =>
        {
            suffix + 2
        }
        _ => suffix,
    };
    let after_auxiliaries = rest
        .iter()
        .enumerate()
        .skip(suffix)
        .find(|(_, node)| !空白(node));
    let end = after_auxiliaries.map_or(i + nodes.len(), |(offset, _)| i + 7 + offset);
    if let [negative, particle, request, ..] = rest
        && matches!(negative.get_ctype(), CType::Special(Special::Nai))
        && *negative.get_cform() == CForm::RenyouConjunctionDe
        && 助詞(particle, "で")
        && matches!(request.get_pos(), POS::Doushi(_))
        && matches!(
            request.get_cform(),
            CForm::ImperativeE | CForm::ImperativeI | CForm::ImperativeRo | CForm::ImperativeYo
        )
    {
        return Some(start..i + 9);
    }
    after_auxiliaries
        .is_none_or(|(_, next)| {
            matches!(next.get_pos(), POS::Kigou(Kigou::Kuten | Kigou::KakkoClose))
                || matches!(next.get_string(), "、" | "！" | "？")
                || matches!(next.get_pos(), POS::Joshi(Joshi::ShuJoshi))
                || matches!(next.get_pos(), POS::Meishi(_)) && next.get_string() == "音"
        })
        .then_some(start..end)
}

fn affirmative_ending(node: &NJDNode) -> bool {
    matches!(node.get_pos(), POS::Jodoushi)
        && matches!(
            node.get_ctype(),
            CType::Special(Special::Ta | Special::Masu)
        )
        && *node.get_cform() == CForm::Basic
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
    // 二十重や百十重の末尾を、単独の十重としてまとめない。
    ((i == 0 || !nodes[i - 1].get_pos().is_kazu())
        && nodes
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
    fn expanded_number_sources_survive_context_segmentation() {
        for roman in [true, false] {
            let mut nodes = vec![
                NJDNode::new_single("十,名詞,数,*,*,*,*,十,ジュウ,ジュー,1/2,C3"),
                NJDNode::new_single("重,名詞,接尾,助数詞,*,*,*,重,ジュウ,ジュー,1/2,*"),
                NJDNode::new_single("に,助詞,格助詞,一般,*,*,*,に,ニ,ニ,0/1,*"),
                NJDNode::new_single(
                    "囲む,動詞,自立,*,*,五段・マ行,基本形,囲む,カコム,カコム,0/3,*",
                ),
            ];
            let source = std::num::NonZeroU32::new(1).unwrap();
            if roman {
                nodes[0].set_roman_source(source);
            } else {
                nodes[0].set_surface_source(source);
            }
            let original = nodes.clone();
            let mut protected = vec![false; nodes.len()];
            prepare_context_segments(&mut nodes, &mut protected);
            assert_eq!(nodes, original);
            assert_eq!(protected, [false; 4]);
        }
    }

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
