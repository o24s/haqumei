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
            if nodes[i].get_string() == "面"
                && nodes[i].get_pos().to_string().starts_with("名詞,接尾,")
            {
                *nodes[i].get_pos_mut() = POS::Meishi(Meishi::General);
                nodes[i].get_details_mut().pos_original = None;
                nodes[i].set_chain_flag(false);
            }
            if reading == "クツガエッ" {
                nodes[i].set_orig("覆る");
                nodes[i].get_details_mut().ctype = "五段・ラ行".parse().unwrap();
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
    if node.get_string() == "覆っ"
        && node.get_orig() == Some("覆う")
        && previous.is_some_and(|n| n.get_string() == "が")
        && previous_previous.is_some_and(|n| matches!(n.get_string(), "船" | "舟" | "ボート"))
        && capsizing_predicate(nodes, i)
    {
        return Some(("クツガエッ", "クツガエッ", 3));
    }
    // 湖の主として北側、の主としては副詞のまま。主の役割を述べる述語を確認する。
    if node.get_string() == "主として"
        && matches!(node.get_pos(), POS::Fukushi(_))
        && via_no
        && previous_previous.is_some_and(|n| {
            matches!(n.get_pos(), POS::Meishi(_))
                && (matches!(n.get_string(), "湖" | "沼" | "池") || n.get_string().ends_with('湖'))
        })
        && water_master_predicate(nodes, i)
    {
        return Some(("ヌシトシテ", "ヌシトシテ", 1));
    }
    if !matches!(node.get_pos(), POS::Meishi(_)) {
        return None;
    }

    match node.get_string() {
        // 姓や組織名のアリイエを保ち、宗教の語が直接続く場合だけザイケにする。
        "在家"
            if following.is_some_and(|n| {
                matches!(n.get_string(), "信者" | "者" | "僧" | "信徒" | "仏教徒")
            }) =>
        {
            Some(("ザイケ", "ザイケ", 0))
        }
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
        // 反戦の声明も唱えられる。唱えるだけでは声明の意味を区別できないため、
        // 宗派を冠した仏教音楽の名称か、旋律を述べる形に限る。
        "声明"
            if ((previous.is_some_and(|n| matches!(n.get_string(), "天台" | "真言"))
                || nodes[i.saturating_sub(5)..i]
                    .iter()
                    .map(|n| n.get_string())
                    .collect::<String>()
                    .ends_with("仏教音楽である"))
                && following.is_some_and(|n| matches!(n.get_string(), "を" | "が"))
                && predicate.is_some_and(|n| {
                    n.get_orig() == Some("唱える") && matches!(n.get_pos(), POS::Doushi(_))
                })
                && (following.unwrap().get_string() == "を"
                    || nodes
                        .get(i + 3)
                        .is_some_and(|n| n.get_orig() == Some("られる"))))
                || (following.is_some_and(|n| n.get_string() == "の")
                    && predicate.is_some_and(|n| n.get_string() == "旋律")) =>
        {
            Some(("ショウミョウ", "ショーミョー", 1))
        }
        // 夕日が麻布を染めるという地名の用法もある。染める単独は根拠にせず、
        // 布の織り方か、織る・染色するという加工を確認する。
        "麻布"
            if (via_no
                && previous_previous.is_some_and(|n| {
                    n.get_string() == "生成り" || n.get_string().ends_with("織り")
                }))
                || cloth_processing(nodes, i) =>
        {
            Some(("アサヌノ", "アサヌノ", 0))
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
        // 号令だけでは剣道の防具も含む。顔の動作と分かる述語の続きか、
        // 御前で顔を上げる命令に限り、物を伏せて置く動作と区別する。
        "面" if following.is_some_and(|n| n.get_string() == "を")
            && !via_no
            && (matches!(node.get_pos(), POS::Meishi(Meishi::General))
                || (previous.is_some_and(|n| n.get_string() == "あまり")
                    && previous_previous.is_some_and(|n| n.get_string() == "の")))
            && face_predicate(nodes, i) =>
        {
            Some(("オモテ", "オモテ", 3))
        }
        // 書物を直す用法まで変えないよう、事の本を正すという連語に限る。
        "本" if via_no
            && previous_previous.is_some_and(|n| n.get_string() == "事")
            && following.is_some_and(|n| n.get_string() == "を")
            && predicate.is_some_and(|n| n.get_orig() == Some("正す")) =>
        {
            Some(("モト", "モト", 2))
        }
        "社" if previous.is_some_and(|n| {
            matches!(n.get_pos(), POS::Rentaishi)
                && matches!(n.get_string(), "この" | "その" | "あの")
        }) && shrine_context(nodes, i) =>
        {
            Some(("ヤシロ", "ヤシロ", 1))
        }
        "汚れ"
            if following.is_some_and(|n| n.get_string() == "を")
                && predicate.is_some_and(|n| n.get_orig() == Some("祓う")) =>
        {
            Some(("ケガレ", "ケガレ", 0))
        }
        "音" if following.is_some_and(|n| n.get_string() == "を")
            && predicate.is_some_and(|n| n.get_orig() == Some("上げる"))
            && giving_up_cause(nodes, i) =>
        {
            Some(("ネ", "ネ", 0))
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

fn capsizing_predicate(nodes: &[NJDNode], i: usize) -> bool {
    // 船が主語でも覆うは他動詞になり得る。転覆の原因が述べられた文に限る。
    if !sentence_nodes(nodes, i).iter().any(|n| {
        matches!(
            n.get_orig(),
            Some("強風" | "突風" | "大波" | "荒波" | "事故" | "転覆")
        )
    }) {
        return false;
    }
    // 海面を黒い船が覆った、では形容詞を越えた位置に目的語がある。
    if nodes[..i.saturating_sub(2)]
        .iter()
        .rev()
        .take_while(|n| !matches!(n.get_pos(), POS::Kigou(_)))
        .any(|n| n.get_string() == "を")
    {
        return false;
    }
    let end = after_auxiliaries(nodes, i + 1);
    // 船が覆っていた海面という連体修飾では、省略された目的語を排除できない。
    nodes.get(end).is_none_or(|n| {
        !matches!(n.get_pos(), POS::Meishi(_))
            || (n.get_pos().to_string().starts_with("名詞,非自立,") && n.get_string() != "の")
    })
}

fn after_auxiliaries(nodes: &[NJDNode], mut at: usize) -> usize {
    while let Some(node) = nodes.get(at) {
        if matches!(node.get_pos(), POS::Jodoushi)
            || node.get_pos().to_string().starts_with("動詞,非自立,")
            || (node.get_string() == "て"
                && nodes
                    .get(at + 1)
                    .is_some_and(|n| n.get_pos().to_string().starts_with("動詞,非自立,")))
        {
            at += 1;
        } else {
            break;
        }
    }
    at
}

fn water_master_predicate(nodes: &[NJDNode], i: usize) -> bool {
    let Some(predicate) = nodes.get(i + 1) else {
        return false;
    };
    let passive = matches!(
        predicate.get_orig(),
        Some("恐れる" | "崇める" | "知る" | "語り継ぐ")
    ) && nodes
        .get(i + 2)
        .is_some_and(|n| matches!(n.get_orig(), Some("れる" | "られる")));
    passive
        || predicate.get_orig() == Some("祀る")
        || (predicate.get_string() == "君臨"
            && nodes
                .get(i + 2)
                .is_some_and(|n| n.get_orig() == Some("する")))
}

fn face_predicate(nodes: &[NJDNode], i: usize) -> bool {
    let Some(predicate) = nodes.get(i + 2) else {
        return false;
    };
    if !matches!(predicate.get_orig(), Some("伏せる" | "上げる")) {
        return false;
    }
    let before: Vec<_> = nodes[..i]
        .iter()
        .rev()
        .take_while(|n| !matches!(n.get_pos(), POS::Kigou(_)))
        .collect();
    if predicate.get_orig() == Some("上げる")
        && predicate.get_cform().to_string() == "命令ｙｏ"
        && i.checked_sub(1).is_none_or(|at| {
            !matches!(
                nodes[at].get_pos(),
                POS::Doushi(_) | POS::Keiyoushi(_) | POS::Meishi(_)
            )
        })
        && nodes[..i]
            .iter()
            .rev()
            .take_while(|n| !matches!(n.get_string(), "。" | "！" | "？" | "「" | "」"))
            .any(|n| matches!(n.get_orig(), Some("殿" | "御前" | "将軍" | "平伏" | "跪く")))
    {
        return true;
    }
    let ashamed = before.iter().any(|n| {
        matches!(
            n.get_orig(),
            Some("恥ずかしい" | "恥ずかしさ" | "恥" | "恥じる" | "恥じらう" | "照れる")
        )
    });
    let end = after_auxiliaries(nodes, i + 3);
    ashamed
        && (nodes
            .get(end)
            .is_none_or(|n| matches!(n.get_pos(), POS::Kigou(_)) || n.get_string() == "まま")
            || (nodes.get(end).is_some_and(|n| n.get_string() == "て")
                && nodes.get(end + 1).is_some_and(|n| {
                    matches!(n.get_orig(), Some("黙る" | "泣く" | "話す" | "笑う"))
                })))
}

fn shrine_context(nodes: &[NJDNode], i: usize) -> bool {
    let Some(particle) = nodes.get(i + 1) else {
        return false;
    };
    let Some(next) = nodes.get(i + 2) else {
        return false;
    };
    if particle.get_string() == "の"
        && matches!(next.get_pos(), POS::Meishi(Meishi::General))
        && matches!(
            next.get_string(),
            "祭神" | "社殿" | "拝殿" | "本殿" | "境内" | "御神体"
        )
    {
        return true;
    }
    // 鳥居は姓でもある。一般名詞でも、設計を担当する鳥居は人物を指す。
    if particle.get_string() == "の"
        && next.get_string() == "鳥居"
        && matches!(next.get_pos(), POS::Meishi(Meishi::General))
        && nodes.get(i + 4).is_some_and(|n| {
            matches!(
                n.get_orig(),
                Some("古い" | "赤い" | "建つ" | "立つ" | "建てる" | "修復" | "修理")
            )
        })
    {
        return true;
    }
    if matches!(particle.get_string(), "を" | "に")
        && matches!(next.get_orig(), Some("参拝" | "拝む"))
    {
        return true;
    }
    if !matches!(particle.get_string(), "は" | "に") {
        return false;
    }
    let at = i + 2 + usize::from(next.get_string() == "は");
    nodes
        .get(at)
        .is_some_and(|n| matches!(n.get_string(), "神" | "祭神" | "御神体"))
        && nodes
            .get(at + 1)
            .is_some_and(|n| matches!(n.get_string(), "を" | "が"))
        && nodes
            .get(at + 2)
            .is_some_and(|n| n.get_orig() == Some("祀る"))
        && nodes
            .get(after_auxiliaries(nodes, at + 3))
            .is_none_or(|n| !matches!(n.get_pos(), POS::Meishi(_)))
}

fn giving_up_cause(nodes: &[NJDNode], i: usize) -> bool {
    let Some(previous) = i.checked_sub(1).and_then(|at| nodes.get(at)) else {
        return false;
    };
    // 音を上げた警報器、という連体修飾では、上げるの対象は音量である。
    let relative_at = after_auxiliaries(nodes, i + 3);
    let participant = nodes
        .get(relative_at)
        .is_some_and(|n| n.get_string() == "参加")
        && nodes
            .get(relative_at + 1)
            .is_some_and(|n| n.get_string() == "者");
    if !participant
        && nodes.get(relative_at).is_some_and(|n| {
            matches!(n.get_pos(), POS::Meishi(_))
                && !n.get_pos().to_string().starts_with("名詞,接尾,")
                && !n.get_pos().to_string().starts_with("名詞,非自立,")
                && !n.get_pos().to_string().starts_with("名詞,固有名詞,人名,")
                && !matches!(
                    n.get_string(),
                    "隊員" | "新兵" | "兵士" | "参加者" | "人" | "者" | "彼" | "彼女"
                )
        })
    {
        return false;
    }
    // 訓練の語が離れた場所にあっても、楽器の音や訓練に使う音はオトである。
    // 苦痛に音を上げる、という原因の格か、辛いと訴える形を確認する。
    if previous.get_string() == "に" {
        let cause: String = nodes[i.saturating_sub(3)..i - 1]
            .iter()
            .map(|n| n.get_string())
            .collect();
        return ["訓練", "苦痛", "痛み", "寒さ", "暑さ", "重労働"]
            .iter()
            .any(|word| cause.ends_with(word));
    }
    previous.get_string() == "と"
        && i >= 2
        && matches!(nodes[i - 2].get_orig(), Some("辛い" | "苦しい"))
}

fn sentence_nodes(nodes: &[NJDNode], i: usize) -> &[NJDNode] {
    let boundary = |node: &NJDNode| matches!(node.get_string(), "。" | "？" | "！" | "「" | "」");
    let start = nodes[..i].iter().rposition(boundary).map_or(0, |at| at + 1);
    let end = nodes[i + 1..]
        .iter()
        .position(boundary)
        .map_or(nodes.len(), |at| i + 1 + at);
    &nodes[start..end]
}

fn cloth_processing(nodes: &[NJDNode], i: usize) -> bool {
    let at = i + 1 + usize::from(nodes.get(i + 1).is_some_and(|n| n.get_string() == "など"));
    if !nodes
        .get(at)
        .is_some_and(|n| matches!(n.get_string(), "を" | "は"))
    {
        return false;
    }
    let Some(predicate) = nodes.get(at + 1) else {
        return false;
    };
    matches!(predicate.get_orig(), Some("織る"))
        || (predicate.get_string() == "染色"
            && nodes
                .get(at + 2)
                .is_some_and(|n| n.get_orig() == Some("する")))
        || (predicate.get_string() == "漆"
            && nodes.get(at + 2).is_some_and(|n| n.get_string() == "で"))
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
