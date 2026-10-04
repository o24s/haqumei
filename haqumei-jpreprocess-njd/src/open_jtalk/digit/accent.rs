use crate::NJD;
use haqumei_jpreprocess_core::{accent_rule::ChainRules, pos::*};

pub(super) fn join_teens(njd: &mut NJD) {
    for i in 0..njd.nodes.len().saturating_sub(1) {
        if njd.nodes[i].get_string() != "十"
            || (i > 0 && njd.nodes[i - 1].get_pos().is_kazu())
            || !njd.nodes[i + 1].get_pos().is_kazu()
            || !matches!(
                njd.nodes[i + 1].get_string(),
                "一" | "二" | "三" | "四" | "五" | "六" | "七" | "八" | "九"
            )
        {
            continue;
        }
        let (head, tail) = njd.nodes.split_at_mut(i + 2);
        if tail
            .first()
            .is_some_and(|n| n.get_pos().is_kazu() || n.get_string() == "世")
        {
            // 十五万などは 11〜19 の範囲外で、世は序数として独立した句を保つ。
            continue;
        }
        let has_counter = tail
            .first()
            .is_some_and(|n| matches!(n.get_pos(), POS::Meishi(_)));
        let mut counter_moras = 0;
        if has_counter {
            for (j, node) in tail.iter().enumerate() {
                if j > 0
                    && (node.get_chain_flag() == Some(false)
                        || !matches!(node.get_pos(), POS::Meishi(Meishi::Setsubi(_))))
                {
                    break;
                }
                counter_moras += node.get_pron().mora_size();
            }
        }
        let short_counter = if has_counter {
            let node = &mut tail[0];
            match node.get_string() {
                "球" | "周" | "週" | "戦" | "層" | "倍" | "場所" => {
                    node.get_details_mut().chain_rule = ChainRules::new("C4")
                }
                "機種" | "地区" => node.get_details_mut().chain_rule = ChainRules::new("C1"),
                _ => (),
            }
            counter_moras <= 2
                && !matches!(
                    node.get_string(),
                    "階" | "級"
                        | "型"
                        | "巡"
                        | "勝"
                        | "乗"
                        | "敗"
                        | "ウォン"
                        | "ギガ"
                        | "か所"
                        | "機種"
                        | "地区"
                        | "球"
                        | "周"
                        | "週"
                        | "戦"
                        | "層"
                        | "倍"
                        | "場所"
                )
        } else {
            false
        };
        let digit = &mut head[i + 1];
        // ヨ・ゴ・クに短い助数詞が続く十四時・十五分・十九時は 2 句に分ける。
        if short_counter
            && digit.get_pron().mora_size() == 1
            && matches!(digit.get_string(), "四" | "五" | "九")
        {
            digit.set_chain_flag(false);
            head[i].get_pron_mut().set_accent(1);
        } else {
            digit.set_chain_flag(true);
            digit.get_details_mut().chain_rule = ChainRules::new(
                if !has_counter && matches!(digit.get_string(), "三" | "五") {
                    "F4@-1"
                } else {
                    "C1"
                },
            );
        }
    }
}
