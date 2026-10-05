use haqumei_jpreprocess_core::{
    accent_rule::ChainRules,
    pos::{Meishi, POS, Setsubi, Settoushi},
    pronunciation::Pronunciation,
};
use haqumei_jpreprocess_njd::NJDNode;

// 数詞と助数詞の間に空白があると、「3 人」の「人」などが一般名詞になる。
// 助数詞の特徴量へ戻してから既存の数詞処理で音便とアクセントを決める。
// https://github.com/tsukumijima/open_jtalk/commit/46739160128d471996e878c9617d42cbd918a334
// https://github.com/tsukumijima/open_jtalk/commit/7ca709eba57f6d8eb73bfeb74f06c007fe2b67b1
pub(super) fn restore_counter_features(nodes: &mut [NJDNode], protected: &[bool]) {
    for i in 1..nodes.len() {
        let roman_counter = nodes[i - 1].roman_source().is_some()
            && matches!(
                nodes[i].get_string(),
                "月" | "巻" | "章" | "節" | "項" | "編" | "世"
            );
        if protected[i]
            || !nodes[i - 1].get_pos().is_kazu()
            || (!roman_counter
                && matches!(
                    nodes[i].get_pos(),
                    POS::Meishi(Meishi::Setsubi(Setsubi::Josuushi))
                ))
            || (!roman_counter
                && !matches!(
                    nodes[i].get_pos(),
                    POS::Meishi(Meishi::General | Meishi::Hijiritsu(_) | Meishi::Setsubi(_))
                        | POS::Settoushi(Settoushi::SuuSetsuzoku)
                ))
        {
            continue;
        }
        // ローマ数字の直後は数詞との接続コストが使われず、「巻」がマキにもなる。
        let entry = if roman_counter {
            match nodes[i].get_string() {
                "月" => Some(&("ガツ", "ガツ", 2, "C1")),
                "巻" => Some(&("カン", "カン", 1, "C3")),
                "章" => Some(&("ショウ", "ショー", 1, "C3")),
                "節" => Some(&("セツ", "セツ", 1, "C3")),
                "項" => Some(&("コウ", "コー", 1, "C3")),
                "編" => Some(&("ヘン", "ヘン", 1, "C3")),
                "世" => Some(&("セイ", "セー", 1, "C3")),
                _ => None,
            }
        } else {
            COUNTERS.get(nodes[i].get_string())
        };
        let Some(&(read, pron, accent, rule)) = entry else {
            continue;
        };
        let mut start = i - 1;
        while start > 0 && nodes[start - 1].get_pos().is_kazu() {
            start -= 1;
        }
        // 「場面6 人前で話す」の6は見出し番号で、後ろの「人」はヒトと読む。
        if start > 0
            && matches!(
                nodes[start - 1].get_string(),
                "場面" | "発言" | "図表" | "図" | "表"
            )
        {
            continue;
        }
        let next = nodes.get(i + 1).map(NJDNode::get_string);
        if (nodes[i].get_string() == "人" && next == Some("づくり"))
            || (nodes[i].get_string() == "分"
                && next == Some("の")
                && nodes.get(i + 2).is_some_and(|n| n.get_pos().is_kazu()))
        {
            continue;
        }
        let rule = if roman_counter
            && nodes[i].get_string() == "月"
            && (i < 2 || nodes[i - 2].roman_source() != nodes[i - 1].roman_source())
        {
            // 三月・五月・九月は頭高で、ほかの月名はガツの末尾に核がある。
            match nodes[i - 1].get_string() {
                "三" => "F4@-1",
                "五" | "九" => "F4@0",
                _ => rule,
            }
        } else {
            rule
        };
        let details = nodes[i].get_details_mut();
        details.pos = POS::Meishi(Meishi::Setsubi(Setsubi::Josuushi));
        details.pos_original = None;
        details.read = Some(read.to_owned());
        details.pron = Pronunciation::parse(pron, accent).unwrap();
        details.chain_rule = ChainRules::new(rule);
        details.chain_flag = None;
    }
}

static COUNTERS: phf::Map<&'static str, (&'static str, &'static str, usize, &'static str)> = phf::phf_map! {
    "年" => ("ネン", "ネン", 1, "C3"),
    "人" => ("ニン", "ニン", 1, "C1"),
    "時" => ("ジ", "ジ", 1, "C3"),
    "本" => ("ホン", "ホン", 1, "C3"),
    "分" => ("フン", "フン", 1, "C3"),
    "秒" => ("ビョウ", "ビョー", 1, "C3"),
    "日" => ("ニチ", "ニチ", 1, "C3"),
    "個" => ("コ", "コ", 1, "C3"),
    "円" => ("エン", "エン", 1, "C3"),
    "回" => ("カイ", "カイ", 1, "C1"),
    "便" => ("ビン", "ビン", 1, "C3"),
    "階" => ("カイ", "カイ", 1, "C4"),
    "番" => ("バン", "バン", 1, "C3"),
    "台" => ("ダイ", "ダイ", 1, "C3"),
    "件" => ("ケン", "ケン", 1, "C3"),
    "枚" => ("マイ", "マイ", 1, "C3"),
    "歳" => ("サイ", "サイ", 1, "C3"),
    "才" => ("サイ", "サイ", 1, "C3"),
    "冊" => ("サツ", "サツ", 0, "C3"),
    "組" => ("クミ", "クミ", 2, "C3"),
    "匹" => ("ヒキ", "ヒキ", 2, "C3"),
    "羽" => ("ワ", "ワ", 0, "C3"),
    "軒" => ("ケン", "ケン", 0, "C3"),
    "杯" => ("ハイ", "ハイ", 1, "C3"),
    "点" => ("テン", "テン", 0, "C3"),
    "割" => ("ワリ", "ワリ", 0, "C3"),
    "倍" => ("バイ", "バイ", 0, "C3"),
    "度" => ("ド", "ド", 0, "C3"),
    "部" => ("ブ", "ブ", 1, "C3"),
    "頭" => ("トウ", "トー", 2, "C1"),
    "曲" => ("キョク", "キョク", 0, "C3"),
};
