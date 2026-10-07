use haqumei_jpreprocess_core::{
    pos::{Kigou, Meishi, POS},
    pronunciation::Pronunciation,
};
use haqumei_jpreprocess_njd::NJDNode;

fn digit(character: char) -> Option<usize> {
    match character {
        '0'..='9' => Some(character as usize - '0' as usize),
        '０'..='９' => Some(character as usize - '０' as usize),
        '〇' | '零' => Some(0),
        '一' => Some(1),
        '二' => Some(2),
        '三' => Some(3),
        '四' => Some(4),
        '五' => Some(5),
        '六' => Some(6),
        '七' => Some(7),
        '八' => Some(8),
        '九' => Some(9),
        _ => None,
    }
}

fn is_digit(node: &NJDNode) -> bool {
    if node.roman_source().is_some() {
        return false;
    }
    let mut chars = node.get_string().chars();
    chars.next().and_then(digit).is_some() && chars.next().is_none()
}

pub(crate) fn is_numeric_surface(surface: &str) -> bool {
    !surface.is_empty() && surface.chars().all(|c| digit(c).is_some())
}

pub(crate) fn is_numeric_identifier_surface(surface: &str) -> bool {
    let digits = surface.strip_prefix('〒').unwrap_or(surface);
    digits.chars().any(|c| digit(c).is_some())
        && digits
            .chars()
            .all(|c| digit(c).is_some() || is_separator(c.encode_utf8(&mut [0; 4])))
}

fn is_separator(surface: &str) -> bool {
    matches!(
        surface,
        "−" | "－" | "ー" | "‐" | "‑" | "‒" | "–" | "—" | "―" | "-"
    )
}

fn is_space(node: &NJDNode) -> bool {
    matches!(node.get_pos(), POS::Kigou(Kigou::Space))
}

// 未知語としてまとまった数字は、発音設定で記号になる前に分ける。
// 〇だけの語は伏字にも使う。隣の数字につながるか、〒の直後である場合に限る。
// https://github.com/tsukumijima/pyopenjtalk-plus/commit/fc06b27e7dfca76c6dc704c4d48125a370899ae4
pub(super) fn expand_unknown_digits(
    fields: &[&str; 13],
    nodes: &mut Vec<NJDNode>,
    next_surface: Option<&str>,
) -> bool {
    let surface = fields[0];
    if fields[9] != "*" {
        return false;
    }
    let (postal, digits) = surface
        .strip_prefix('〒')
        .map_or((false, surface), |s| (true, s));
    let has_digit = digits.chars().any(|c| digit(c).is_some());
    let numeric = postal
        || digits.chars().any(|c| c != '〇' && digit(c).is_some())
        || nodes.last().is_some_and(|n| {
            n.get_string() == "〒"
                || (is_digit(n) && (n.get_string() != "〇" || n.get_read() == Some("ゼロ")))
        })
        || next_surface
            .and_then(|s| s.chars().next())
            .is_some_and(|c| c != '〇' && digit(c).is_some());
    if !has_digit
        || !numeric
        || surface.chars().count() < 2
        || !digits
            .chars()
            .all(|c| digit(c).is_some() || is_separator(c.encode_utf8(&mut [0; 4])))
    {
        return false;
    }
    if postal {
        nodes.push(NJDNode::new_single("〒,記号,一般,*,*,*,*,〒,〒,〒,*/*,*"));
    }
    for character in digits.chars() {
        let mut node = if let Some(value) = digit(character) {
            NJDNode::new_single(DIGITS[value])
        } else {
            NJDNode::new_single("−,記号,一般,*,*,*,*,−,、,、,0/0,*")
        };
        let mut buffer = [0; 4];
        let original = character.encode_utf8(&mut buffer);
        // 数字の表層形は後段の桁処理でも使う。零は既存の正規化表にない。
        node.replace_string(if character == '零' { "０" } else { original });
        node.set_orig(original);
        nodes.push(node);
    }
    true
}

const DIGITS: [&str; 10] = [
    "０,名詞,数,*,*,*,*,０,ゼロ,ゼロ,1/2,C3",
    "１,名詞,数,*,*,*,*,１,イチ,イチ,2/2,C3",
    "２,名詞,数,*,*,*,*,２,ニ,ニ,1/1,C3",
    "３,名詞,数,*,*,*,*,３,サン,サン,0/2,C3",
    "４,名詞,数,*,*,*,*,４,ヨン,ヨン,1/2,C1",
    "５,名詞,数,*,*,*,*,５,ゴ,ゴ,1/1,C3",
    "６,名詞,数,*,*,*,*,６,ロク,ロク,2/2,C3",
    "７,名詞,数,*,*,*,*,７,ナナ,ナナ,1/2,C3",
    "８,名詞,数,*,*,*,*,８,ハチ,ハチ,2/2,C3",
    "９,名詞,数,*,*,*,*,９,キュウ,キュー,1/2,C3",
];

pub(super) fn expand_identifier_digits(
    fields: &[&str; 13],
    source: std::num::NonZeroU32,
    nodes: &mut Vec<NJDNode>,
) -> bool {
    use haqumei_jpreprocess_njd::{NJD, digit_sequence};

    let digits = fields[7];
    if digits.is_empty() || digits.len() > 64 || !digits.bytes().all(|c| c.is_ascii_digit()) {
        return false;
    }

    let cardinal = fields[3] == "番号位" && !digits.starts_with('0');
    let mut number = NJD {
        nodes: Vec::with_capacity(digits.len()),
    };

    for digit in digits.bytes() {
        let mut node = NJDNode::new_single(DIGITS[(digit - b'0') as usize]);
        node.set_digit_sequence_reading(Some(cardinal));

        if digit == b'0' && fields[3] == "番号丸" {
            node.set_read("マル");
            node.set_pron(Pronunciation::parse("マル", 1).unwrap());
        }

        number.nodes.push(node);
    }

    digit_sequence::njd_digit_sequence(&mut number);
    nodes.extend(number.nodes.into_iter().map(|mut node| {
        node.set_surface_source(source);
        node
    }));

    true
}

fn cardinal_nodes(value: u16) -> Vec<NJDNode> {
    use haqumei_jpreprocess_njd::{NJD, digit_sequence};

    let mut numeral = NJD {
        nodes: value
            .to_string()
            .bytes()
            .map(|c| {
                let mut node = NJDNode::new_single(DIGITS[(c - b'0') as usize]);
                node.set_digit_sequence_reading(Some(true));
                node
            })
            .collect(),
    };

    // ローマ数字と暦の数値は、前後の番号表現に左右されないよう、文へ戻す前に位取りを確定する。
    digit_sequence::njd_digit_sequence(&mut numeral);

    numeral.nodes
}

pub(super) fn expand_roman(value: u16, source: std::num::NonZeroU32, nodes: &mut Vec<NJDNode>) {
    nodes.extend(cardinal_nodes(value).into_iter().map(|mut node| {
        node.set_roman_source(source);
        node
    }));
}

pub(super) fn expand_calendar(
    value: u16,
    kind: &str,
    source: std::num::NonZeroU32,
    nodes: &mut Vec<NJDNode>,
) {
    if kind == "暦元年" {
        // 「元年」は一般名詞として前の元号に結合する。数詞の「一年」とは句の区切りが異なる。
        let mut node = NJDNode::new_single("元年,名詞,一般,*,*,*,*,元年,ガンネン,ガンネン,1/4,C1");
        node.set_surface_source(source);
        nodes.push(node);
        return;
    }

    let mut number = cardinal_nodes(value);
    let counter = match kind {
        "暦年" => "年,名詞,接尾,助数詞,*,*,*,年,ネン,ネン,1/2,C3",
        "暦月" => "月,名詞,接尾,助数詞,*,*,*,月,ガツ,ガツ,2/2,C1",
        "暦日" => "日,名詞,接尾,助数詞,*,*,*,日,ニチ,ニチ,1/2,C3",
        _ => unreachable!(),
    };
    let mut counter = NJDNode::new_single(counter);

    if kind == "暦月" {
        counter.get_details_mut().chain_rule =
            haqumei_jpreprocess_core::accent_rule::ChainRules::new(match value {
                3 => "F4@-1",
                5 | 9 => "F4@0",
                _ => "C1",
            });
    }

    number.push(counter);
    nodes.extend(number.into_iter().map(|mut node| {
        node.set_surface_source(source);
        node
    }));
}

pub(super) fn retain_number_spaces(nodes: &mut Vec<NJDNode>, protected: &mut Vec<bool>) {
    let mut read = 0;
    let mut write = 0;
    let mut previous_digit = false;
    while read < nodes.len() {
        if !is_space(&nodes[read]) {
            previous_digit = nodes[read].get_pos().is_kazu() || is_digit(&nodes[read]);
            nodes.swap(write, read);
            protected.swap(write, read);
            write += 1;
            read += 1;
            continue;
        }
        let start = read;
        while read < nodes.len() && is_space(&nodes[read]) {
            read += 1;
        }
        // 助数詞の前の空白は除き、数字どうしの空白だけを一時的な境界にする。
        let retain = previous_digit
            && read < nodes.len()
            && (nodes[read].get_pos().is_kazu() || is_digit(&nodes[read]));
        if retain {
            nodes[start].set_pron(Pronunciation::parse("、", 0).unwrap());
            nodes[start].get_pron_mut().set_mora_size(1);
            nodes.swap(write, start);
            protected.swap(write, start);
            write += 1;
        }
        previous_digit = false;
    }
    nodes.truncate(write);
    protected.truncate(write);
}

pub(super) fn remove_number_spaces(nodes: &mut Vec<NJDNode>) {
    let mut after_space = false;
    nodes.retain_mut(|node| {
        if is_space(node) {
            after_space = true;
            false
        } else {
            if after_space {
                node.set_chain_flag(false);
                after_space = false;
            }
            true
        }
    });
}

pub(super) fn restore_numeric_zeros(nodes: &mut [NJDNode], protected: &[bool]) {
    let mut start = 0;
    while start < nodes.len() {
        if !is_digit(&nodes[start]) {
            start += 1;
            continue;
        }
        let mut end = start;
        let mut has_other_digit = false;
        while end < nodes.len() {
            if is_digit(&nodes[end]) {
                has_other_digit |= nodes[end].get_string() != "〇";
                end += 1;
            } else if is_separator(nodes[end].get_string())
                && nodes.get(end + 1).is_some_and(is_digit)
            {
                end += 1;
            } else {
                break;
            }
        }
        let postal = start > 0 && nodes[start - 1].get_string() == "〒";
        let numeric = has_other_digit || postal;
        if numeric && end - start > 1 {
            for i in start..end {
                if nodes[i].get_string() == "〇" && !protected[i] {
                    let node = &mut nodes[i];
                    node.get_details_mut().pos = POS::Meishi(Meishi::Kazu);
                    node.get_details_mut().pos_original = None;
                    node.set_read("ゼロ");
                    node.set_pron(Pronunciation::parse("ゼロ", 1).unwrap());
                }
            }
        }
        start = end;
    }
}

// 電話・郵便の語を広く探すと「電話は100ある」「〒の費用は1234567」も番号になる。
// 発信先の3桁と、〒に直結する7桁だけを桁読みに指定する。
// https://github.com/tsukumijima/open_jtalk/commit/e3cc6330c6afff76ab1b580d8bc67a591265ef1b
pub(super) fn mark_identifiers(nodes: &mut [NJDNode], protected: &[bool]) {
    let mut start = 0;
    while start < nodes.len() {
        if !is_digit(&nodes[start]) || !nodes[start].get_pos().is_kazu() {
            start += 1;
            continue;
        }
        let mut end = start + 1;
        while end < nodes.len() && is_digit(&nodes[end]) && nodes[end].get_pos().is_kazu() {
            end += 1;
        }
        let postal = start > 0 && nodes[start - 1].get_string() == "〒";
        let mut postal_end = end;
        if postal
            && end - start == 3
            && nodes
                .get(end)
                .is_some_and(|n| is_separator(n.get_string()) || n.get_string() == "・")
        {
            postal_end += 1;
            while postal_end < nodes.len()
                && is_digit(&nodes[postal_end])
                && nodes[postal_end].get_pos().is_kazu()
            {
                postal_end += 1;
            }
        }
        let continues_number = nodes.get(postal_end).is_some_and(|n| {
            (is_separator(n.get_string()) || matches!(n.get_string(), "．" | "・"))
                && nodes.get(postal_end + 1).is_some_and(is_digit)
        });
        let is_postal = postal
            && !continues_number
            && ((end - start == 7 && postal_end == end)
                || (end - start == 3 && postal_end - end == 5));
        let separated = start == 0
            || (!nodes[start - 1].get_pos().is_kazu()
                && !matches!(nodes[start - 1].get_string(), "．" | "・")
                && !is_separator(nodes[start - 1].get_string()));
        let phone = end - start == 3
            && separated
            && nodes
                .get(end)
                .is_some_and(|n| n.get_string() == "に" && matches!(n.get_pos(), POS::Joshi(_)))
            && (nodes.get(end + 1).is_some_and(|n| {
                matches!(n.get_pos(), POS::Doushi(_))
                    && matches!(n.get_orig(), Some("かける" | "掛ける"))
            }) || (nodes.get(end + 1).is_some_and(|n| n.get_string() == "電話")
                && nodes.get(end + 2).is_some_and(|n| {
                    matches!(n.get_pos(), POS::Doushi(_)) && n.get_orig() == Some("する")
                })));
        let stop = if is_postal { postal_end } else { end };
        if (is_postal || phone) && !protected[start..stop].iter().any(|&p| p) {
            for node in &mut nodes[start..stop] {
                node.set_digit_sequence_reading(Some(false));
            }
        }
        start = stop;
    }
}
