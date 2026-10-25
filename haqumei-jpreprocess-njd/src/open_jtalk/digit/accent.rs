use crate::NJD;
use haqumei_jpreprocess_core::{accent_rule::ChainRules, pos::*};

pub(super) fn prepare_number_markers(njd: &mut NJD) {
    for i in 1..njd.nodes.len() {
        if njd.nodes[i].get_string() == "数"
            && !njd.nodes[i].counter_reading_protected()
            && njd.nodes[i - 1].get_pos().is_kazu()
            && matches!(njd.nodes[i - 1].get_string(), "十" | "百" | "千")
        {
            njd.nodes[i].get_details_mut().pos = POS::Meishi(Meishi::Kazu);
            njd.nodes[i].get_details_mut().pos_original = None;
        }
    }
}

pub(super) fn join_places(njd: &mut NJD) {
    for i in 1..njd.nodes.len() {
        let prev = &njd.nodes[i - 1];
        let node = &njd.nodes[i];
        if !prev.get_pos().is_kazu()
            || !node.get_pos().is_kazu()
            || node.counter_reading_protected()
        {
            continue;
        }
        let small = matches!(prev.get_string(), "十" | "百" | "千");
        if (small
            && (node.get_string() == "数"
                || matches!(node.get_string(), "万" | "億" | "兆" | "京" | "垓")))
            || (matches!(prev.get_string(), "数" | "何" | "幾")
                && node.get_string().starts_with(['十', '百', '千']))
        {
            njd.nodes[i].set_chain_flag(true);
        }
        if njd.nodes[i - 1].get_string() == "数"
            && i > 1
            && matches!(njd.nodes[i - 2].get_string(), "十" | "百" | "千")
        {
            njd.nodes[i - 1].set_chain_flag(true);
        }
    }
}

pub(super) fn man_yen(njd: &mut NJD) {
    for i in 0..njd.nodes.len().saturating_sub(1) {
        if !njd.nodes[i].get_pos().is_kazu()
            || !matches!(njd.nodes[i].get_string(), "万" | "十万" | "千万")
            || njd.nodes[i + 1].get_string() != "円"
            || njd.nodes[i + 1].counter_reading_protected()
        {
            continue;
        }
        let number = if njd.nodes[i].get_string() == "万" {
            i.checked_sub(1)
        } else {
            Some(i)
        };
        let Some(number) = number else { continue };
        if !njd.nodes[number].get_pos().is_kazu() {
            continue;
        }
        // 二十万円ではマに核を置き、一万円・十万円では平板にする。
        let rule = if number > 0
            && njd.nodes[number].get_chain_flag() == Some(true)
            && njd.nodes[number - 1].get_pos().is_kazu()
        {
            "F4@-1"
        } else {
            "F5"
        };
        njd.nodes[i + 1].get_details_mut().chain_rule = ChainRules::new(rule);
    }
}

pub(super) fn yen(njd: &mut NJD) {
    for i in 1..njd.nodes.len() {
        if njd.nodes[i].get_string() != "円"
            || njd.nodes[i].counter_reading_protected()
            || !njd.nodes[i - 1].get_pos().is_kazu()
        {
            continue;
        }
        let compound = i > 1 && njd.nodes[i - 2].get_pos().is_kazu();
        let prev = njd.nodes[i - 1].get_string();
        if (!compound && matches!(prev, "一" | "二" | "三" | "六" | "八" | "十"))
            || prev == "百"
            || (compound && prev == "千")
        {
            njd.nodes[i].get_details_mut().chain_rule = ChainRules::new("F5");
        }
    }
}

pub(super) fn flat_short_counters(njd: &mut NJD) {
    for i in 1..njd.nodes.len() {
        let previous = &njd.nodes[i - 1];
        let node = &njd.nodes[i];
        if !previous.get_pos().is_kazu()
            || node.counter_reading_protected()
            || !matches!(
                node.get_pos(),
                POS::Meishi(Meishi::Setsubi(Setsubi::Josuushi) | Meishi::FukushiKanou)
            )
        {
            continue;
        }
        let five = previous.get_string() == "五"
            && matches!(
                node.get_string(),
                "本" | "枚" | "台" | "代" | "番" | "年" | "題" | "段" | "問"
            );
        let thirteen = i >= 2
            && njd.nodes[i - 2].get_string() == "十"
            && (i == 2 || !njd.nodes[i - 3].get_pos().is_kazu());
        let three = previous.get_string() == "三" && node.get_string() == "番" && !thirteen;
        if five || three {
            njd.nodes[i].get_details_mut().chain_rule = ChainRules::new("F5");
        }
    }
}

pub(super) fn sokuon_counters(njd: &mut NJD) {
    for i in 1..njd.nodes.len() {
        let previous = &njd.nodes[i - 1];
        let counter = &njd.nodes[i];
        if !previous.get_pos().is_kazu()
            || counter.counter_reading_protected()
            || !matches!(
                counter.get_pos(),
                POS::Meishi(Meishi::Setsubi(Setsubi::Josuushi) | Meishi::FukushiKanou)
            )
        {
            continue;
        }
        let tens = i > 1 && previous.get_string() == "十" && njd.nodes[i - 2].get_pos().is_kazu();
        let odaka = matches!(
            counter.get_string(),
            "発" | "匹"
                | "冊"
                | "室"
                | "隻"
                | "拍"
                | "泊"
                | "客"
                | "脚"
                | "曲"
                | "局"
                | "尺"
                | "色"
                | "食"
                | "節"
                | "滴"
        );
        let rule = if !tens && odaka && previous.get_pron().to_pure_string().ends_with('ッ') {
            // 一発・六匹の促音後は尾高になるが、二十冊の核は助数詞の前に残る。
            Some(format!("F4@{}", counter.get_pron().mora_size()))
        } else if tens && (odaka || matches!(counter.get_string(), "点" | "戦")) {
            Some("F4@-1".into())
        } else {
            None
        };
        if let Some(rule) = rule {
            njd.nodes[i].get_details_mut().chain_rule = ChainRules::new(&rule);
        }
    }
}

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
