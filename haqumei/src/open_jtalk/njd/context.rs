//! 単語の読みと核を、助詞との結合より先に決める。

use haqumei_jpreprocess_core::pronunciation::Pronunciation;
use haqumei_jpreprocess_njd::NJDNode;

pub(super) fn prepare_readings(nodes: &mut [NJDNode], protected: &[bool]) {
    for i in 0..nodes.len() {
        if protected[i] {
            continue;
        }
        let table = nodes[i].get_string() == "表"
            && (nodes
                .get(i + 1)
                .is_some_and(|n| n.get_string() == "計算" || n.get_pos().is_kazu())
                || (i >= 2
                    && nodes[i - 1].get_string() == "の"
                    && nodes[i - 2].get_string() == "統計"));
        if table {
            // オモテの核を残さず、表の平板型から助詞・助動詞との核を計算する。
            nodes[i].set_read("ヒョウ");
            nodes[i].set_pron(Pronunciation::parse("ヒョー", 0).unwrap());
        }
        let weekday = |s: &str| match s {
            "月" => Some("ゲツ"),
            "火" => Some("カ"),
            "水" => Some("スイ"),
            "木" => Some("モク"),
            "金" => Some("キン"),
            "土" => Some("ド"),
            "日" => Some("ニチ"),
            _ => None,
        };
        if i + 2 < nodes.len()
            && nodes[i + 2].get_string() == "金"
            && nodes[i].get_string() != "金"
            && matches!(nodes[i + 1].get_string(), "〜" | "～" | "ー" | "－" | "−")
            && let (Some(start), Some(end)) = (
                weekday(nodes[i].get_string()),
                weekday(nodes[i + 2].get_string()),
            )
        {
            for (at, reading) in [(i, start), (i + 2, end)] {
                if !protected[at] {
                    nodes[at].set_read(reading);
                    nodes[at].set_pron(Pronunciation::parse(reading, 1).unwrap());
                }
            }
        }
        if i >= 2
            && nodes[i].get_string() == "仮名"
            && matches!(nodes[i - 1].get_string(), "（" | "(")
            && nodes[i - 2].get_pos().to_string().split(',').nth(2) == Some("人名")
            && nodes
                .get(i + 1)
                .is_some_and(|n| matches!(n.get_string(), "）" | ")"))
        {
            nodes[i].set_read("カメイ");
            nodes[i].set_pron(Pronunciation::parse("カメー", 0).unwrap());
        }
    }
}
