//! 単語の読みと核を、助詞との結合より先に決める。

use haqumei_jpreprocess_core::{pos::POS, pronunciation::Pronunciation};
use haqumei_jpreprocess_njd::NJDNode;

pub(super) fn prepare_explicit_first_day(nodes: &mut Vec<NJDNode>, protected: &mut Vec<bool>) {
    let mut i = 0;
    while i + 2 < nodes.len() {
        let is_one = matches!(nodes[i].get_string(), "１" | "一") && nodes[i].get_pos().is_kazu();
        let duration = i > 0
            && (nodes[i - 1].get_pos().is_kazu()
                || matches!(
                    nodes[i - 1].get_string(),
                    "第" | "丸" | "まる" | "毎" | "各" | "約" | "ほぼ" | "たった" | "わずか"
                ));
        let mut following = i + 2;
        let via_no = nodes[following].get_string() == "の";
        if via_no {
            following += 1;
        }
        let time_of_day = nodes.get(following).is_some_and(|n| {
            matches!(
                n.get_string(),
                "午前" | "午後" | "未明" | "早朝" | "深夜" | "夕方" | "正午"
            )
        });
        let month_anchor = i.checked_sub(1).is_some_and(|at| {
            matches!(
                nodes[at].get_string(),
                "月" | "今月" | "来月" | "翌月" | "同月"
            )
        });
        // 月のない一日付にはイチニチズケという読みもある。漢数字の一日は、
        // 月が明示されていなければ既定の読みを保つ。
        let date_suffix = !via_no
            && (nodes[i].get_string() == "１" || month_anchor)
            && nodes.get(following).is_some_and(|n| {
                matches!(n.get_pos(), POS::Meishi(_)) && matches!(n.get_string(), "付" | "付け")
            })
            && nodes.get(following + 1).is_none_or(|n| {
                matches!(n.get_pos(), POS::Joshi(_) | POS::Jodoushi | POS::Kigou(_))
            });
        // 一日の午後だけでは日付と期間を区別できない。のを挟む場合は月の修飾、
        // 挟まない場合は日付の修飾か、文頭の一日午後という形を確認する。
        let date_anchor = i.checked_sub(1).is_some_and(|at| {
            matches!(
                nodes[at].get_string(),
                "月" | "今月" | "来月" | "翌月" | "同月" | "本日" | "今日" | "明日" | "来る"
            )
        });
        let at_start = i == 0
            || nodes
                .get(i - 1)
                .is_some_and(|n| matches!(n.get_string(), "。" | "！" | "？" | "「"));
        let explicit_date = date_suffix
            || (time_of_day
                && if via_no {
                    month_anchor
                } else {
                    date_anchor || at_start
                });
        if is_one
            && nodes[i + 1].get_string() == "日"
            && !duration
            && explicit_date
            && !protected[i]
            && !protected[i + 1]
        {
            // 日付の修飾か付を確認してから、数詞の処理前に一語へまとめる。
            nodes[i] =
                NJDNode::new_single("一日,名詞,副詞可能,*,*,*,*,一日,ツイタチ,ツイタチ,4/4,*");
            nodes.remove(i + 1);
            protected.remove(i + 1);
        }
        i += 1;
    }
}

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
