use crate::{NJD, NJDNode};
use haqumei_jpreprocess_core::{pos::*, pron};

use super::{is_period, lut::*, rule};

fn is_decimal_digit(
    nodes: &[NJDNode],
    mut i: usize,
    decimal_points: &[usize],
    spelled_point: bool,
) -> bool {
    while nodes[i].get_pos().is_kazu() {
        // 「2.1万本」の万は整数として本と結合する。一桁の数字だけを遡る。
        if !matches!(
            nodes[i].get_string(),
            "〇" | "０" | "零" | "一" | "二" | "三" | "四" | "五" | "六" | "七" | "八" | "九"
        ) {
            return false;
        }
        let Some(prev) = i.checked_sub(1) else {
            return false;
        };
        i = prev;
    }
    // 数字に挟まれた小数点は njd_set_digit がテンに変換済み。
    // 「米・一貫目」の中黒まで小数点と扱うと、イッカンメがイチカンメになる。
    if is_period(nodes[i].get_string()) && nodes[i].get_read() == Some("テン") {
        return spelled_point || decimal_points.binary_search(&i).is_ok();
    }
    if !spelled_point {
        return false;
    }
    if nodes[i].get_string() == "一点" && nodes[i].get_read() == Some("イッテン") {
        return true;
    }
    if nodes[i].get_string() != "点" || i == 0 {
        return false;
    }
    i -= 1;
    if nodes[i].get_string() == "ー" {
        let Some(prev) = i.checked_sub(1) else {
            return false;
        };
        i = prev;
    }
    nodes[i].get_pos().is_kazu()
}

fn is_counter(nodes: &[NJDNode], i: usize, decimal_points: &[usize]) -> bool {
    if nodes[i].counter_reading_protected() {
        return false;
    }
    if matches!(
        nodes[i].get_pos(),
        POS::Meishi(Meishi::FukushiKanou | Meishi::Setsubi(Setsubi::Josuushi))
    ) {
        !is_decimal_digit(nodes, i - 1, decimal_points, false)
    } else {
        // 「点」は得点も表すため、漢字表記の小数判定は既存の追加語だけに限る。
        class1::COUNTER_WORDS.contains(nodes[i].get_string())
            && !is_decimal_digit(nodes, i - 1, decimal_points, true)
    }
}

pub(super) fn convert_counters(njd: &mut NJD, decimal_points: &[usize]) {
    for i in 1..njd.nodes.len() {
        if !njd.nodes[i - 1].get_pos().is_kazu() || !is_counter(&njd.nodes, i, decimal_points) {
            continue;
        }
        let (before, after) = njd.nodes.split_at_mut(i);
        let (prev, earlier) = before.split_last_mut().unwrap();
        let (node, later) = after.split_first_mut().unwrap();
        let compound = earlier.last().is_some_and(|n| n.get_pos().is_kazu());
        let ordinal = earlier.last().is_some_and(|n| n.get_string() == "第");
        let next = later.first().map(|n| n.get_string()).unwrap_or("");

        if node.get_string() == "部屋" && node.get_read() == Some("ベヤ") {
            node.set_read("ヘヤ");
            node.set_pron(pron!([He, Ya], node.get_pron().accent()));
        }

        // 「分袖」「階級」「年生」は、後続語まで見ないと時間や階数の読みになる。
        let conversion = match (node.get_string(), next) {
            ("分", "袖") => {
                node.set_read("ブ");
                node.set_pron(pron!([Bu], node.get_pron().accent()));
                class1::CONV_TABLE1E.get(prev.get_string())
            }
            ("階", "級") => class1::CONV_TABLE1L.get(prev.get_string()),
            ("カラット", _) => class1::CONV_TABLE_TEN_HUNDRED.get(prev.get_string()),
            ("とおり", _) => class1::CONV_TABLE_EIGHT_TEN.get(prev.get_string()),
            ("棟", _) if node.get_read() == Some("ムネ") => None,
            ("組", _) => class1::CONV_TABLE1I.get(prev.get_string()),
            ("試合", _) => (!ordinal)
                .then(|| class1::CONV_TABLE1J.get(prev.get_string()))
                .flatten(),
            // 石にはセキとコクがあるため、辞書がコクを選んだ場合だけ促音化する。
            ("石", _) if node.get_read() == Some("コク") => {
                class1::CONV_TABLE1L.get(prev.get_string())
            }
            ("年生", _) | ("年", "生") => class1::CONV_TABLE1B.get(prev.get_string()),
            ("里", _) => class1::CONV_TABLE_SEVEN.get(prev.get_string()),
            ("夜", _) if node.get_read() == Some("ヤ") => {
                class1::CONV_TABLE_SEVEN.get(prev.get_string())
            }
            ("アンダー", _) if !compound => class1::CONV_TABLE_UNDER.get(prev.get_string()),
            _ => find_pron_conv_set(
                &class1::CONVERSION_TABLE,
                node.get_string(),
                prev.get_string(),
            ),
        };
        if let Some(pron) = conversion {
            prev.set_pron(pron.clone());
        }
        if node.get_string() == "把"
            && node.get_read() == Some("ワ")
            && let Some(pron) = class1::CONV_TABLE1K.get(prev.get_string())
        {
            prev.set_pron(pron.clone());
        }

        // ヒト・フタに変わる数詞の後では、箱などを半濁音にしない。
        let native = !compound
            && find_pron_conv_map(
                &class3::CONVERSION_TABLE,
                node.get_string(),
                node.get_read().unwrap_or("*"),
                prev.get_string(),
            )
            .is_some();
        let keep_sound = native
            || (node.get_string() == "分" && node.get_read() == Some("ブ"))
            || (node.get_string() == "階" && next == "級")
            || (node.get_string() == "波" && prev.get_string() == "八")
            || (node.get_string() == "鉢" && prev.get_string() == "四");
        let voicing = if keep_sound {
            None
        } else {
            match node.get_string() {
                "袋" => (prev.get_string() == "十").then_some(DigitType::SemiVoiced),
                "寸" => (prev.get_string() == "三").then_some(DigitType::Voiced),
                "把" if node.get_read() == Some("ワ") && prev.get_string() == "十" => {
                    node.set_pron(pron!([Pa], node.get_pron().accent()));
                    None
                }
                "羽" if node.get_read() == Some("ワ")
                    && matches!(prev.get_string(), "千" | "万") =>
                {
                    node.set_pron(pron!([Ba], node.get_pron().accent()));
                    None
                }
                _ => find_pron_conv_set(
                    &class2::CONVERSION_TABLE,
                    node.get_string(),
                    prev.get_string(),
                )
                .copied(),
            }
        };
        if let Some(mora) = node.get_pron_mut().moras_mut().first_mut() {
            match voicing {
                Some(DigitType::Voiced) => mora.convert_to_voiced_sound(),
                Some(DigitType::SemiVoiced) => mora.convert_to_semivoiced_sound(),
                None => (),
            }
        }
        prev.set_chain_flag(false);
        node.set_chain_flag(true);
    }
}

pub(super) fn convert_native(njd: &mut NJD, decimal_points: &[usize]) {
    for i in 0..njd.nodes.len().saturating_sub(1) {
        if !njd.nodes[i].get_pos().is_kazu()
            || (i > 0 && njd.nodes[i - 1].get_pos().is_kazu())
            || njd.nodes[i + 1].get_string().is_empty()
            || !is_counter(&njd.nodes, i + 1, decimal_points)
        {
            continue;
        }
        let (before, after) = njd.nodes.split_at_mut(i);
        let (node, after) = after.split_first_mut().unwrap();
        let (next, after) = after.split_first_mut().unwrap();
        let suffix = after.first().map(|n| n.get_string()).unwrap_or("");
        if !(next.get_string() == "幕" && suffix == "目")
            && let Some(conversion) = find_pron_conv_map(
                &class3::CONVERSION_TABLE,
                next.get_string(),
                next.get_read().unwrap_or("*"),
                node.get_string(),
            )
        {
            node.set_read(&conversion.to_pure_string());
            node.set_pron(conversion.clone());
        }
        if next.get_string() == "柱"
            && next.get_read() == Some("ハシラ")
            && let Some(pron) = class1::CONV_TABLE_NATIVE.get(node.get_string())
        {
            node.set_pron(pron.clone());
        }
        // 「一人前」はヒトリに併合するとイチニンマエに戻せなくなる。
        if next.get_string() == "人" && suffix == "前" {
            continue;
        }
        if let Some(new_node) = find_pron_conv_set(
            &others::CONVERSION_TABLE,
            next.get_string(),
            node.get_string(),
        ) {
            if before
                .last()
                .is_some_and(|p| p.get_string().contains(rule::GATSU))
                && node.get_string() == rule::ONE
                && next.get_string() == rule::NICHI
            {
                node.replace_from_csv(rule::TSUITACHI);
            } else {
                node.replace_from_csv(new_node);
            }
            next.reset();
        }
    }
}
