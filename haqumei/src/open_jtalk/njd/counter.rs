use haqumei_jpreprocess_core::{
    accent_rule::ChainRules,
    pos::{Meishi, POS, Setsubi, Settoushi},
    pronunciation::Pronunciation,
};
use haqumei_jpreprocess_njd::NJDNode;

pub(super) fn prepare_unit_letters(nodes: &mut [NJDNode], protected: &[bool]) {
    for i in 1..nodes.len() {
        if protected[i]
            || !nodes[i - 1].get_pos().is_kazu()
            || !matches!(nodes[i].get_pos(), POS::Kigou(_))
        {
            continue;
        }
        // 「2m+1」は数式の変数なので、直後に演算子がある英字は単位へ変えない。
        if nodes
            .get(i + 1)
            .and_then(|n| n.get_string().chars().next())
            .is_some_and(|c| "＋−－＝×÷＊＜＞+-=*<>".contains(c))
        {
            continue;
        }
        let (read, accent, rule) = match nodes[i].get_string() {
            "ｇ" => ("グラム", 1, "C1"),
            "ｍ" => ("メートル", 0, "C2"),
            "ｔ" => ("トン", 1, "C3"),
            "ｌ" => ("リットル", 0, "C2"),
            _ => continue,
        };
        let details = nodes[i].get_details_mut();
        details.pos = POS::Meishi(Meishi::Setsubi(Setsubi::Josuushi));
        details.pos_original = None;
        details.read = Some(read.into());
        details.pron = Pronunciation::parse(read, accent).unwrap();
        details.chain_rule = ChainRules::new(rule);
    }
}

pub(super) fn prepare_old_place_characters(nodes: &mut [NJDNode], protected: &[bool]) {
    for i in 0..nodes.len() {
        if protected[i] || !matches!(nodes[i].get_string(), "拾" | "萬") {
            continue;
        }
        let context = nodes[i].get_pos().is_kazu()
            || (i > 0 && nodes[i - 1].get_pos().is_kazu())
            || nodes.get(i + 1).is_some_and(|n| {
                n.get_pos().is_kazu()
                    || matches!(n.get_string(), "萬" | "円" | "万円" | "億円" | "兆円")
            });
        if context {
            let feature = if nodes[i].get_string() == "拾" {
                "十,名詞,数,*,*,*,*,十,ジュウ,ジュー,1/2,*"
            } else {
                "万,名詞,数,*,*,*,*,万,マン,マン,1/2,*"
            };
            nodes[i].replace_from_csv(feature);
        }
    }
}

pub(super) fn prepare_fraction_denominators(nodes: &mut [NJDNode], protected: &[bool]) {
    for i in 1..nodes.len().saturating_sub(2) {
        if !protected[i]
            && nodes[i - 1].get_pos().is_kazu()
            && nodes[i].get_string() == "分"
            && nodes[i + 1].get_string() == "の"
            && nodes[i + 2].get_pos().is_kazu()
        {
            // 時間量の音便を計算してからブンへ戻すと、分母のイチまでイッに変わる。
            let accent = nodes[i].get_pron().accent();
            nodes[i].set_read("ブン");
            nodes[i].set_pron(Pronunciation::parse("ブン", accent).unwrap());
        }
    }
}

pub(super) fn prepare_ratio_readings(nodes: &mut [NJDNode], protected: &[bool]) {
    for i in 0..nodes.len() {
        if protected[i] {
            continue;
        }
        let surface = nodes[i].get_string();
        let separate = surface == "分" && i > 0 && nodes[i - 1].get_pos().is_kazu();
        let combined = surface.strip_suffix('分').is_some_and(|prefix| {
            !prefix.is_empty()
                && prefix
                    .chars()
                    .all(|c| "一二三四五六七八九十百千〇零".contains(c))
        });
        if !(separate || combined) {
            continue;
        }
        let mut start = i;
        while start > 0 && nodes[start - 1].get_pos().is_kazu() {
            start -= 1;
        }
        let after_wari = start > 0 && nodes[start - 1].get_string() == "割";
        let mut end = i + 1;
        while end < nodes.len() && nodes[end].get_pos().is_kazu() {
            end += 1;
        }
        let before_rin = end > i + 1 && nodes.get(end).is_some_and(|n| n.get_string() == "厘");
        if !(after_wari || before_rin) {
            continue;
        }
        // 「3分5秒」は時間なので、割合の割・厘がある用例だけをブへ戻す。
        let old = nodes[i].get_pron().to_pure_string();
        let pron = if separate {
            "ブ".to_owned()
        } else if let Some(prefix) = old
            .strip_suffix("プン")
            .or_else(|| old.strip_suffix("フン"))
        {
            format!("{prefix}ブ")
        } else {
            continue;
        };
        let accent = nodes[i].get_pron().accent();
        nodes[i].set_read(&pron);
        nodes[i].set_pron(Pronunciation::parse(&pron, accent).unwrap());
    }
}

// 数詞と助数詞の間に空白があると、「3 人」の「人」などが一般名詞になる。
// 助数詞の特徴量へ戻してから既存の数詞処理で音便とアクセントを決める。
// https://github.com/tsukumijima/open_jtalk/commit/46739160128d471996e878c9617d42cbd918a334
// https://github.com/tsukumijima/open_jtalk/commit/7ca709eba57f6d8eb73bfeb74f06c007fe2b67b1
pub(super) fn restore_counter_features(nodes: &mut [NJDNode], protected: &[bool]) {
    for i in 1..nodes.len() {
        if !protected[i] && calendar_range_month(nodes, i) {
            let details = nodes[i].get_details_mut();
            details.pos = POS::Meishi(Meishi::Setsubi(Setsubi::Josuushi));
            details.pos_original = None;
            details.read = Some("ガツ".into());
            details.pron = Pronunciation::parse("ガツ", 2).unwrap();
            details.chain_rule = ChainRules::new("C1");
            details.chain_flag = None;
        }
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

fn calendar_range_month(nodes: &[NJDNode], i: usize) -> bool {
    if nodes[i].get_string() != "月" || nodes.get(i + 1).is_none_or(|n| n.get_string() != "の") {
        return false;
    }
    let mut start = i;
    while start > 0 && nodes[start - 1].get_pos().is_kazu() {
        start -= 1;
    }
    if start < 2
        || start == i
        || !matches!(nodes[start - 1].get_string(), "−" | "〜" | "～")
        || !nodes[start - 2].get_pos().is_kazu()
    {
        return false;
    }
    let spelling: String = nodes[start..i].iter().map(|n| n.get_string()).collect();
    // 範囲の終点が暦月である場合に限り、期間の「ツキ」をガツへ戻す。
    matches!(
        spelling.as_str(),
        "一" | "二"
            | "三"
            | "四"
            | "五"
            | "六"
            | "七"
            | "八"
            | "九"
            | "十"
            | "十一"
            | "十二"
            | "１"
            | "２"
            | "３"
            | "４"
            | "５"
            | "６"
            | "７"
            | "８"
            | "９"
            | "１０"
            | "１１"
            | "１２"
    )
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
