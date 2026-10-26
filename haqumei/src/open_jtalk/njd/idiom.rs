//! 固定した語句の読みを、単語のアクセントとともに結合前に決める。

use haqumei_jpreprocess_core::{
    pos::{Meishi, POS},
    pronunciation::Pronunciation,
};
use haqumei_jpreprocess_njd::NJDNode;

pub(super) fn prepare_readings(nodes: &mut [NJDNode], protected: &[bool]) {
    for i in 0..nodes.len() {
        if protected.get(i) == Some(&true) {
            continue;
        }
        if let Some(entry) = noun_entry(nodes, i) {
            // 動詞の「怒り」や接尾辞の「風」が選ばれた場合も、語句の品詞から
            // 助詞との結合を計算する。表層形は変えず、文字区間の対応を保つ。
            nodes[i] = NJDNode::new_single(entry);
        } else if let Some((reading, pronunciation, accent)) = verb_reading(nodes, i) {
            nodes[i].set_read(reading);
            nodes[i].set_pron(Pronunciation::parse(pronunciation, accent).unwrap());
        }
    }
}

fn before(nodes: &[NJDNode], i: usize, words: &[&str]) -> bool {
    i.checked_sub(words.len()).is_some_and(|start| {
        nodes[start..i]
            .iter()
            .zip(words)
            .all(|(node, word)| node.get_string() == *word)
    })
}

fn after(nodes: &[NJDNode], i: usize, words: &[&str]) -> bool {
    nodes.get(i + 1..i + 1 + words.len()).is_some_and(|part| {
        part.iter()
            .zip(words)
            .all(|(node, word)| node.get_string() == *word)
    })
}

fn after_predicate(nodes: &[NJDNode], i: usize, words: &[&str], originals: &[&str]) -> bool {
    after(nodes, i, words)
        && nodes.get(i + words.len() + 1).is_some_and(|node| {
            matches!(node.get_pos(), POS::Doushi(_) | POS::Keiyoushi(_))
                && node
                    .get_orig()
                    .is_some_and(|orig| originals.contains(&orig))
        })
}

fn noun_entry(nodes: &[NJDNode], i: usize) -> Option<&'static str> {
    match nodes[i].get_string() {
        // 「腸が煮える」だけでは加熱する文も含む。怒りの慣用句の述語に限る。
        "腸" if after_predicate(nodes, i, &["が"], &["煮えくり返る"]) => {
            Some("腸,名詞,一般,*,*,*,*,腸,ハラワタ,ハラワタ,0/4,C2")
        }
        // 単独で引用された「鶴の一声」と区別し、手段を表す「で」が続く形に限る。
        "一声" if before(nodes, i, &["鶴", "の"]) && after(nodes, i, &["で"]) => {
            Some("一声,名詞,一般,*,*,*,*,一声,ヒトコエ,ヒトコエ,2/4,C1")
        }
        "供" if before(nodes, i, &["を"]) && after(nodes, i, &["に"]) => {
            Some("供,名詞,一般,*,*,*,*,供,トモ,トモ,1/2,C3")
        }
        "怒り" if after_predicate(nodes, i, &["に"], &["震える"]) => {
            Some("怒り,名詞,一般,*,*,*,*,怒り,イカリ,イカリ,3/3,C2")
        }
        // 時間を惜しむ用法に限り、空間を表す「柱と柱の間」を変えない。
        "間" if (after_predicate(nodes, i, &["も"], &["惜しむ"])
            && i.checked_sub(1).is_some_and(|at| {
                nodes[at]
                    .get_orig()
                    .is_some_and(|orig| matches!(orig, "寝る" | "休む"))
            }))
            || (after_predicate(nodes, i, &["が"], &["悪い"])
                && i.checked_sub(1).is_some_and(|at| {
                    nodes[at]
                        .get_orig()
                        .is_some_and(|orig| matches!(orig, "話しかける" | "訪ねる"))
                })) =>
        {
            Some("間,名詞,一般,*,*,*,*,間,マ,マ,0/1,C2")
        }
        // 「桜の」が姓として一語になっている辞書でも、後続語の区間は保つ。
        "下" if (before(nodes, i, &["桜", "の"]) || before(nodes, i, &["桜の"]))
            && nodes
                .get(i + 1)
                .is_some_and(|node| matches!(node.get_string(), "で" | "に" | "を")) =>
        {
            Some("下,名詞,一般,*,*,*,*,下,シタ,シタ,0/2,C3")
        }
        "金" if after(nodes, i, &["屏風"]) || before(nodes, i, &["元素", "の"]) => {
            Some("金,名詞,一般,*,*,*,*,金,キン,キン,1/2,C3")
        }
        // 「季節・風」「北西・風」のフウも吹くため、名詞へ直結する複合語は変えない。
        "風" if after_predicate(nodes, i, &["が"], &["吹く"])
            && (i == 0 || !matches!(nodes[i - 1].get_pos(), POS::Meishi(_))) =>
        {
            Some("風,名詞,一般,*,*,*,*,風,カゼ,カゼ,0/2,C3")
        }
        // 建物の「造作もなくなった」はゾウサク。仕事の容易さか、できるという述語を確認する。
        "造作"
            if after(nodes, i, &["も", "ない", "仕事"])
                || after(nodes, i, &["も", "ない", "作業"])
                || (after(nodes, i, &["も", "なく"])
                    && nodes.get(i + 3).is_some_and(is_ability_verb)) =>
        {
            Some("造作,名詞,一般,*,*,*,*,造作,ゾウサ,ゾーサ,0/3,C2")
        }
        "空" if before(nodes, i, &["中身", "が"])
            && after_predicate(nodes, i, &["に"], &["なる"]) =>
        {
            Some("空,名詞,一般,*,*,*,*,空,カラ,カラ,2/2,C3")
        }
        // 「民主・政」「君主・政」のセイを、独立したマツリゴトへ変えない。
        "政" if after_predicate(nodes, i, &["を"], &["行う"])
            && (i == 0 || !matches!(nodes[i - 1].get_pos(), POS::Meishi(_))) =>
        {
            Some("政,名詞,一般,*,*,*,*,政,マツリゴト,マツリゴト,0/5,C2")
        }
        // 果実の「実のところだけ」「実のところが」を談話表現と取り違えない。
        "実" if after(nodes, i, &["の", "ところ", "、"])
            && (i == 0 || matches!(nodes[i - 1].get_pos(), POS::Kigou(_))) =>
        {
            Some("実,名詞,一般,*,*,*,*,実,ジツ,ジツ,2/2,C3")
        }
        // 「手術後が辛い」のゴは保ち、名詞へ直接付く接尾辞を対象にしない。
        // 「の」を挟む場合も、時間を表す語に限って場所のウシロと区別する。
        "後" if after_predicate(nodes, i, &["が"], &["怖い", "辛い"])
            && (i == 0 || !matches!(nodes[i - 1].get_pos(), POS::Meishi(_)))
            && (!before(nodes, i, &["の"])
                || ["失敗", "事故", "手術", "霜焼け"]
                    .iter()
                    .any(|word| before(nodes, i, &[word, "の"]))) =>
        {
            Some("後,名詞,副詞可能,*,*,*,*,後,アト,アト,1/2,C3")
        }
        // 「声の主な特徴」のオモや「主効果」の接頭辞シュと区別する。
        "主" if before(nodes, i, &["声", "の"])
            && matches!(nodes[i].get_pos(), POS::Meishi(Meishi::General))
            && nodes.get(i + 1).is_none_or(|node| {
                matches!(
                    node.get_pos(),
                    POS::Joshi(_) | POS::Jodoushi | POS::Kigou(_)
                )
            }) =>
        {
            Some("主,名詞,一般,*,*,*,*,主,ヌシ,ヌシ,1/2,C3")
        }
        "額" if before(nodes, i, &["法外", "な"]) => {
            Some("額,名詞,一般,*,*,*,*,額,ガク,ガク,0/2,C2")
        }
        // 魚の身を粉末にする文はコナ。労働の述語まで続く慣用句に限る。
        "粉" if before(nodes, i, &["身", "を"])
            && after_predicate(nodes, i, &["に", "し", "て"], &["働く"]) =>
        {
            Some("粉,名詞,一般,*,*,*,*,粉,コ,コ,1/1,C3")
        }
        // 心理的な性向を述べる「職人気質が強い」のキシツを変えず、
        // 行動様式を貫くという連語だけをカタギにする。
        "気質" if before(nodes, i, &["職人"]) && after_predicate(nodes, i, &["を"], &["貫く"]) => {
            Some("気質,名詞,一般,*,*,*,*,気質,カタギ,カタギ,0/3,C2")
        }
        // 「山田氏より育児について」の接尾辞は変えず、「育ち」と一致する場合だけを選ぶ。
        "氏" if after(nodes, i, &["より", "育ち"])
            && (i == 0 || !matches!(nodes[i - 1].get_pos(), POS::Meishi(_))) =>
        {
            Some("氏,名詞,一般,*,*,*,*,氏,ウジ,ウジ,1/2,C3")
        }
        "仇" if (before(nodes, i, &["恩", "を"])
            && after_predicate(nodes, i, &["で"], &["返す"]))
            || (["善意", "親切", "恩"]
                .iter()
                .any(|word| before(nodes, i, &[word, "が"]))
                && after_predicate(nodes, i, &["に"], &["なる"])) =>
        {
            Some("仇,名詞,一般,*,*,*,*,仇,アダ,アダ,2/2,C3")
        }
        // 太鼓や剣道具の面の皮はメン。人を主題にした慣用句だけをツラにする。
        "面" if after_predicate(nodes, i, &["の", "皮", "が"], &["厚い"])
            && ["男", "女", "人", "彼", "彼女", "師"]
                .iter()
                .any(|word| before(nodes, i, &[word, "は"])) =>
        {
            Some("面,名詞,一般,*,*,*,*,面,ツラ,ツラ,2/2,C3")
        }
        _ => None,
    }
}

fn is_ability_verb(node: &NJDNode) -> bool {
    // 「できる」の原形が「する」になっている辞書もある。
    matches!(node.get_pos(), POS::Doushi(_))
        && (matches!(node.get_orig(), Some("できる" | "出来る"))
            || (node.get_orig() == Some("する")
                && matches!(node.get_string(), "できる" | "でき" | "出来る" | "出来")))
}

fn verb_reading(nodes: &[NJDNode], i: usize) -> Option<(&'static str, &'static str, usize)> {
    let node = &nodes[i];
    if !matches!(node.get_pos(), POS::Doushi(_)) {
        return None;
    }
    match node.get_orig()? {
        "被る"
            if before(nodes, i, &["帽子", "を"])
                || before(nodes, i, &["ヘルメット", "を"])
                || before(nodes, i, &["麦わら帽子", "を"]) =>
        {
            let reading = match node.get_string() {
                "被る" => "カブル",
                "被っ" => "カブッ",
                "被り" => "カブリ",
                "被ら" => "カブラ",
                "被れ" => "カブレ",
                _ => return None,
            };
            Some((reading, reading, 2))
        }
        "抱く" if before(nodes, i, &["念", "を"]) => {
            let reading = match node.get_string() {
                "抱く" => "イダク",
                "抱い" => "イダイ",
                "抱き" => "イダキ",
                "抱か" => "イダカ",
                "抱け" => "イダケ",
                _ => return None,
            };
            Some((reading, reading, 2))
        }
        "開ける" if before(nodes, i, &["未来", "が"]) => {
            let reading = match node.get_string() {
                "開ける" => "ヒラケル",
                "開け" => "ヒラケ",
                "開けれ" => "ヒラケレ",
                _ => return None,
            };
            Some((reading, reading, 3))
        }
        "堪える" if before(nodes, i, &["涙", "を"]) || before(nodes, i, &["落涙", "を"]) => {
            let reading = match node.get_string() {
                "堪える" => "コラエル",
                "堪え" => "コラエ",
                "堪えれ" => "コラエレ",
                _ => return None,
            };
            Some((reading, reading, 3))
        }
        "堪える" if before(nodes, i, &["使用", "に"]) || before(nodes, i, &["批判", "に"]) =>
        {
            let reading = match node.get_string() {
                "堪える" => "タエル",
                "堪え" => "タエ",
                "堪えれ" => "タエレ",
                _ => return None,
            };
            Some((reading, reading, 2))
        }
        "堪える" if before(nodes, i, &["身", "に"]) => {
            let reading = match node.get_string() {
                "堪える" => "コタエル",
                "堪え" => "コタエ",
                "堪えれ" => "コタエレ",
                _ => return None,
            };
            Some((reading, reading, 3))
        }
        _ => None,
    }
}
