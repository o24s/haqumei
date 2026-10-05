//! Process accent conjugation.
//!
//! Please refer to [UNIDIC Users' Manual](https://clrd.ninjal.ac.jp/unidic/UNIDIC_manual.pdf)「6.7 アクセント結合型」
//! for details on each accent conjugation type.

const ICHI: &str = "一";
const NI: &str = "二";
const SAN: &str = "三";
const YON: &str = "四";
const GO: &str = "五";
const ROKU: &str = "六";
const NANA: &str = "七";
const HACHI: &str = "八";
const KYUU: &str = "九";
const JYUU: &str = "十";
const HYAKU: &str = "百";
const SEN: &str = "千";
const MAN: &str = "万";
const OKU: &str = "億";
const CHOU: &str = "兆";
const NAN: &str = "何";
const IKU: &str = "幾";

use haqumei_jpreprocess_core::accent_rule::AccentType;

use crate::{NJD, NJDNode};

pub fn njd_set_accent_type(njd: &mut NJD) {
    if njd.nodes.is_empty() {
        return;
    }
    let mut top_node_i = 0;
    let mut mora_size = 0;
    let mut original_top_accent = 0;
    let mut original_top_mora_size = 0;
    for i in 0..njd.nodes.len() {
        let mut top_node_acc: Option<usize> = None;
        let mut prev_acc: Option<usize> = None;
        let mut current_acc: Option<usize> = None;

        {
            let (top_node, prev, current, next) = (
                njd.nodes.get(top_node_i).unwrap(),
                (i > 0).then(|| njd.nodes.get(i - 1).unwrap()),
                njd.nodes.get(i).unwrap(),
                njd.nodes.get(i + 1),
            );

            if i == 0 || current.get_chain_flag() != Some(true) {
                top_node_i = i;
                mora_size = 0;
                original_top_accent = current.get_pron().accent();
                original_top_mora_size = current.get_pron().mora_size();

                if current.get_string() == JYUU
                    && next
                        .is_some_and(|n| n.get_pos().is_kazu() && n.get_chain_flag() == Some(true))
                {
                    current_acc = Some(0);
                }
            } else if let Some(prev) = prev {
                top_node_acc = Some(
                    if top_node.get_chain_rule().to_string() == "P2"
                        && original_top_accent != 0
                        && original_top_mora_size > original_top_accent
                    {
                        original_top_accent
                    } else {
                        calc_top_node_acc(current, prev, top_node, mora_size)
                    },
                );
                if prev.get_pos().is_kazu() && current.get_pos().is_kazu() {
                    prev_acc = calc_digit_acc(prev, current, next);
                }
            }

            mora_size += current.get_pron().mora_size();
        }

        if let Some(top_node_acc) = top_node_acc {
            njd.nodes
                .get_mut(top_node_i)
                .unwrap()
                .get_pron_mut()
                .set_accent(top_node_acc);
        }
        if let Some(prev_acc) = prev_acc {
            njd.nodes
                .get_mut(i - 1)
                .unwrap()
                .get_pron_mut()
                .set_accent(prev_acc);
        }
        if let Some(current_acc) = current_acc {
            njd.nodes
                .get_mut(i)
                .unwrap()
                .get_pron_mut()
                .set_accent(current_acc);
        }
    }
}

fn calc_top_node_acc(
    node: &NJDNode,
    prev: &NJDNode,
    top_node: &NJDNode,
    mora_size: usize,
) -> usize {
    let node_acc = node.get_pron().accent();
    let top_node_acc = top_node.get_pron().accent();

    let Some(rule) = node.get_chain_rule().get_rule(prev.get_pos()) else {
        return top_node_acc;
    };

    let add_rule = || (mora_size as isize + rule.add_type).max(0) as usize;

    match rule.accent_type {
        AccentType::F1 => top_node_acc,
        AccentType::F2 if top_node_acc == 0 => add_rule(),
        AccentType::F3 if top_node_acc != 0 => add_rule(),
        AccentType::F4 => add_rule(),
        AccentType::F5 => 0,
        AccentType::C1 => mora_size + node_acc,
        AccentType::C2 => mora_size + 1,
        AccentType::C3 => mora_size,
        AccentType::C4 => 0,
        AccentType::C5 => top_node_acc,
        AccentType::P1 if node_acc == 0 => 0,
        AccentType::P1 => mora_size + node_acc,
        AccentType::P6 => 0,
        AccentType::P14 if node_acc != 0 => mora_size + node_acc,
        _ => top_node_acc,
    }
}

fn calc_digit_acc(prev: &NJDNode, current: &NJDNode, next: Option<&NJDNode>) -> Option<usize> {
    let prev_str = prev.get_string();
    let current_str = current.get_string();
    let next_str = next.map(|node| node.get_string());
    match (prev_str, current_str, next_str) {
        (
            GO | ROKU | HACHI,
            JYUU,
            Some(ICHI | NI | SAN | YON | GO | ROKU | NANA | HACHI | KYUU),
        ) => Some(0),
        (GO | ROKU | HACHI, JYUU, _) => Some(prev.get_pron().mora_size() + 1),
        (NANA, JYUU, _) => Some(2),
        (_, JYUU, _) => Some(1),

        (NANA, HYAKU, _) => Some(2),
        (SAN | YON | KYUU | NAN, HYAKU, _) => Some(1),
        (_, HYAKU, _) => Some(prev.get_pron().mora_size() + current.get_pron().mora_size()),

        (_, SEN, _) => Some(prev.get_pron().mora_size() + 1),

        (_, MAN, _) => Some(prev.get_pron().mora_size() + 1),

        (ICHI | ROKU | NANA | HACHI | IKU, OKU, _) => Some(2),
        (_, OKU, _) => Some(1),

        (ROKU | NANA, CHOU, _) => Some(2),
        (_, CHOU, _) => Some(1),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::{NJD, accent_type::njd_set_accent_type};

    #[test]
    fn tens_keep_their_nucleus_when_units_are_absent() {
        for (digit, pron, moras, accent, with_units) in [
            ("五", "ゴ", 1, 2, 0),
            ("六", "ロク", 2, 3, 0),
            ("七", "ナナ", 2, 2, 2),
            ("八", "ハチ", 2, 3, 0),
        ] {
            for tail in [false, true] {
                let mut entries = vec![
                    format!("{digit},名詞,数,*,*,*,*,{digit},{pron},{pron},1/{moras},C3,0"),
                    "十,名詞,数,*,*,*,*,十,ジュウ,ジュー,1/2,*,1".to_owned(),
                ];
                if tail {
                    entries.push("一,名詞,数,*,*,*,*,一,イチ,イチ,2/2,C3,0".to_owned());
                }
                let mut njd: NJD = entries.iter().map(String::as_str).collect();
                njd_set_accent_type(&mut njd);
                assert_eq!(
                    njd.nodes[0].get_pron().accent(),
                    if tail { with_units } else { accent }
                );
            }
        }
    }

    #[test]
    fn prefix_p2_preserves_its_original_nucleus() {
        let mut njd: NJD = [
            "各,接頭詞,名詞接続,*,*,*,*,各,カク,カク,1/2,P2,-1",
            "部,名詞,一般,*,*,*,*,部,ブ,ブ,0/1,C4,1",
            "長,名詞,接尾,一般,*,*,*,長,チョウ,チョー,1/2,C1,1",
        ]
        .into_iter()
        .collect();
        njd_set_accent_type(&mut njd);
        assert_eq!(njd.nodes[0].get_pron().accent(), 1);
    }

    #[test]
    fn prefix_rules_use_the_following_word_accent() {
        for (rule, initial, following, expected) in [
            ("P1", 0, 1, 3),
            ("P1", 1, 0, 0),
            ("P14", 0, 1, 3),
            ("P2", 1, 0, 1),
        ] {
            let mut njd: NJD = [
                format!("語,名詞,一般,*,*,*,*,語,カク,カク,{initial}/2,*,-1"),
                format!("語,名詞,接尾,一般,*,*,*,語,ゴ,ゴ,{following}/1,{rule},1"),
            ]
            .iter()
            .map(String::as_str)
            .collect();
            njd_set_accent_type(&mut njd);
            assert_eq!(njd.nodes[0].get_pron().accent(), expected, "{rule}");
        }
    }

    #[test]
    fn cow() {
        let mut njd: NJD = [
            "牛飼,名詞,固有名詞,地域,一般,*,*,牛飼,ウシカイ,ウシカイ,2/4,C2,-1",
            "じゃ,助詞,副助詞,*,*,*,*,じゃ,ジャ,ジャ,0/1,名詞%F1,1",
            "あり,助動詞,*,*,*,五段・ラ行アル,連用形,あり,アリ,アリ,2/2,動詞%F1,1",
            "ませ,助動詞,*,*,*,特殊・マス,未然形,ませ,マセ,マセ,1/2,動詞%F4@1/助詞%F2@1,1",
            "ん,助動詞,*,*,*,不変化型,基本形,ん,ン,ン,1/1,動詞%F4,1",
            "よ,助詞,終助詞,*,*,*,*,よ,ヨ,ヨ,0/1,動詞%F1/形容詞%F1/名詞%F1,1",
        ]
        .into_iter()
        .collect();

        njd_set_accent_type(&mut njd);

        // Open JTalk treats "助動詞" as a match for "動詞%F1".
        assert_eq!(njd.nodes[0].get_pron().accent(), 9);
    }
}
