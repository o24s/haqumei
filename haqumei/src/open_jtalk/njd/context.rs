//! 単語の読みと核を、助詞との結合より先に決める。

use haqumei_jpreprocess_core::{
    accent_rule::ChainRules,
    pos::{Meishi, POS},
    pronunciation::Pronunciation,
};
use haqumei_jpreprocess_njd::NJDNode;

pub(super) fn prepare_person_names(nodes: &mut Vec<NJDNode>, protected: &mut Vec<bool>) {
    let mut i = 0;
    while i + 2 < nodes.len() {
        let fused_san = nodes[i + 2].get_string() == "四さん";
        let digits = nodes[i].get_string() == "六"
            && nodes[i + 1].get_string() == "三"
            && nodes[i..i + 2].iter().all(|n| n.get_pos().is_kazu())
            && ((nodes[i + 2].get_string() == "四" && nodes[i + 2].get_pos().is_kazu())
                || fused_san);
        let isolated = (i == 0 || !nodes[i - 1].get_pos().is_kazu())
            && nodes.get(i + 3).is_none_or(|n| !n.get_pos().is_kazu());
        let name_label = i >= 2
            && matches!(nodes[i - 1].get_string(), "は" | "が")
            && matches!(nodes[i - 2].get_string(), "名前" | "氏名" | "名");
        let honorific = fused_san
            || nodes
                .get(i + 3)
                .is_some_and(|n| matches!(n.get_string(), "くん" | "君" | "さん"));
        let named = nodes.get(i + 3).is_some_and(|n| n.get_string() == "と")
            && nodes
                .get(i + 4)
                .is_some_and(|n| matches!(n.get_orig(), Some("いう" | "言う")))
            && nodes
                .get(i + 5)
                .is_some_and(|n| matches!(n.get_string(), "名前" | "氏名" | "名"));
        // 名前は六三四人で決めた、という数量を人名へ結合しない。
        let name_end = nodes.get(i + 3).is_none_or(|node| {
            matches!(
                node.get_pos(),
                POS::Joshi(_) | POS::Jodoushi | POS::Kigou(_)
            ) || matches!(node.get_string(), "くん" | "君" | "さん")
        });
        if digits
            && isolated
            && name_end
            && (name_label || honorific || named)
            && !protected[i..i + 3].iter().any(|&value| value)
        {
            // 数の634までムサシへ変えないよう、名前の提示か敬称を確認してからまとめる。
            nodes.splice(
                i..i + 3,
                [NJDNode::new_single(
                    "六三四,名詞,固有名詞,人名,名,*,*,六三四,ムサシ,ムサシ,1/3,*",
                )],
            );
            protected.splice(i..i + 3, [false]);
            if fused_san {
                // 辞書が四さんを一語にした場合も、数の部分だけを人名へまとめる。
                nodes.insert(
                    i + 1,
                    NJDNode::new_single("さん,名詞,接尾,人名,*,*,*,さん,サン,サン,0/2,C5"),
                );
                protected.insert(i + 1, false);
            }
            if nodes.get(i + 1).is_some_and(|n| n.get_string() == "君") && !protected[i + 1] {
                nodes[i + 1] = NJDNode::new_single("君,名詞,接尾,人名,*,*,*,君,クン,クン,1/2,C3");
            }
        }
        i += 1;
    }
}

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
        if let Some((reading, pronunciation, accent)) = lexical_reading(nodes, i) {
            // 読みの長さや単語の核が変わるため、助詞との結合前に設定する。
            nodes[i].set_read(reading);
            nodes[i].set_pron(Pronunciation::parse(pronunciation, accent).unwrap());
            if nodes[i].get_string() == "際" {
                // 時を表す非自立名詞のサイと区別し、端を指すキワを独立した句にする。
                *nodes[i].get_pos_mut() = POS::Meishi(Meishi::General);
                nodes[i].get_details_mut().pos_original = None;
                nodes[i].get_details_mut().chain_rule = ChainRules::new("C3");
                nodes[i].set_chain_flag(false);
            }
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

fn lexical_reading(nodes: &[NJDNode], i: usize) -> Option<(&'static str, &'static str, usize)> {
    // 黒白・紅色・水の面は同じ語義でも読みが複数あるので、読みの好みだけを理由に書き換えていない。
    let node = &nodes[i];
    let previous = i.checked_sub(1).and_then(|at| nodes.get(at));
    let previous_previous = i.checked_sub(2).and_then(|at| nodes.get(at));
    let following = nodes.get(i + 1);
    let predicate = nodes.get(i + 2);
    let via_no = previous.is_some_and(|n| {
        n.get_string() == "の" && n.get_pos().to_string().starts_with("助詞,連体化,")
    });
    if !matches!(node.get_pos(), POS::Meishi(_)) {
        return None;
    }

    match node.get_string() {
        // 神の代理・神の代行は別の形態素なので、独立した代だけを対象にする。
        "代" if matches!(node.get_pos(), POS::Meishi(Meishi::General))
            && via_no
            && previous_previous.is_some_and(|n| {
                n.get_string() == "神" && matches!(n.get_pos(), POS::Meishi(Meishi::General))
            })
            && following.is_none_or(|n| {
                matches!(n.get_pos(), POS::Joshi(_) | POS::Jodoushi | POS::Kigou(_))
            }) =>
        {
            Some(("ヨ", "ヨ", 1))
        }
        // 洪水の際・排水の際は、前の形態素が水ではないのでサイを保つ。
        "際" if via_no
            && previous_previous.is_some_and(|n| {
                matches!(n.get_string(), "崖" | "淵" | "水")
                    && matches!(n.get_pos(), POS::Meishi(_))
            }) =>
        {
            Some(("キワ", "キワ", 2))
        }
        // 書物を直す用法まで変えないよう、事の本を正すという連語に限る。
        "本" if via_no
            && previous_previous.is_some_and(|n| n.get_string() == "事")
            && following.is_some_and(|n| n.get_string() == "を")
            && predicate.is_some_and(|n| n.get_orig() == Some("正す")) =>
        {
            Some(("モト", "モト", 2))
        }
        "汚れ"
            if following.is_some_and(|n| n.get_string() == "を")
                && predicate.is_some_and(|n| n.get_orig() == Some("祓う")) =>
        {
            Some(("ケガレ", "ケガレ", 0))
        }
        "盲" | "聾" if disability_enumeration(nodes, i) => {
            if node.get_string() == "盲" {
                Some(("モウ", "モー", 1))
            } else {
                Some(("ロウ", "ロー", 1))
            }
        }
        _ => None,
    }
}

fn disability_enumeration(nodes: &[NJDNode], i: usize) -> bool {
    const TERMS: &[&str] = &[
        "盲",
        "聾",
        "ろう",
        "盲学校",
        "聾学校",
        "ろう学校",
        "養護学校",
        "視覚障害",
        "聴覚障害",
        "知的障害",
        "肢体不自由",
        "病弱",
    ];
    for step in [-1isize, 1] {
        let Some(separator) = i.checked_add_signed(step) else {
            continue;
        };
        if !nodes
            .get(separator)
            .is_some_and(|n| matches!(n.get_string(), "・" | "、"))
        {
            continue;
        }
        let mut cursor = separator.checked_add_signed(step);
        let mut term = String::new();
        while let Some(at) = cursor {
            let Some(node) = nodes.get(at) else { break };
            if !matches!(node.get_pos(), POS::Meishi(_)) {
                break;
            }
            if step < 0 {
                term.insert_str(0, node.get_string());
            } else {
                term.push_str(node.get_string());
            }
            if TERMS.contains(&term.as_str()) {
                return true;
            }
            if !TERMS.iter().any(|candidate| {
                if step < 0 {
                    candidate.ends_with(&term)
                } else {
                    candidate.starts_with(&term)
                }
            }) {
                break;
            }
            cursor = at.checked_add_signed(step);
        }
    }
    false
}
