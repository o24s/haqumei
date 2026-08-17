//! 文脈によって読みが決まる語の補正。
//!
//! 同形異音語のうち、隣接する形態素だけで読みが決まるものを、語ごとの
//! 順序付き規則で解決する。
//!
//! 同形異音語の曖昧性は語ごとに手がかりが異なり、しかも大半の語では
//! 特定の読みが圧倒的多数を占める。
//!
//! - 学習には語ごとのラベル付きデータが必要になるが、入手できるコーパス
//!   (ルビ付きテキストなど) は「読みが自明でないとき」に注記される性質上、
//!   少数派の読みに偏っており、既定の読みを壊す方向に働く
//! - 規則そのものが説明になるので、1 規則ずつ検証・修正できる
//! - 追加の実行コストも依存も要らない
//!
//! よって、まず決定リストで取れるところを取る方針にしている。
//!
//! # 規則を追加するときの条件
//!
//! - 閉じた条件であること。 「直後が『さん』」のように列挙できる形にする。
//!   「文意から」のような条件は書けないので、その語は対象にしない。
//! - 負の対照を確認すること。 条件に合わないときに発火しないことを、
//!   その語を含む別の文で確かめる (例: 「一見さん」に対して「一見して」)。
//! - 既定の読みを変えないこと。 既定の読みを変えたい場合は、規則ではなく
//!   辞書の単語コストで調整する。
//!
//! 分割が既に正しい複合語 (「読み方」「夕方」など 1 形態素として解析される語) は
//! 対象の形態素が単独で現れないため、構造的に規則の影響を受けない。

use crate::NjdFeature;
use crate::utils::count_mora;

/// 規則が発火する条件。
enum Cue {
    /// 直後の形態素の表層形が、いずれかに一致する。
    NextIn(&'static [&'static str]),
    /// 直前の形態素の表層形が、いずれかに一致する。
    PrevIn(&'static [&'static str]),
}

struct Rule {
    /// 対象の形態素の表層形。
    surface: &'static str,
    /// 対象の形態素の品詞細分類 1。`None` なら問わない。
    pos_group1: Option<&'static str>,
    cue: Cue,
    /// 与える読み。読みと発音が異なる語が出てきたら分けること。
    reading: &'static str,
}

/// 語ごとの規則。上から順に評価し、最初に一致したものを適用する。
///
/// 各規則の根拠と、確認した負の対照をコメントに残す。
const RULES: &[Rule] = &[
    // 「一見さん」= イチゲンさん (初めての客)。
    // 負の対照: 「一見して分かる」は直後が「し」なので発火しない。
    Rule {
        surface: "一見",
        pos_group1: None,
        cue: Cue::NextIn(&["さん"]),
        reading: "イチゲン",
    },
    // 「一声かける」= ヒトコエかける。
    // 負の対照: 「一声も出ない」は直後が「も」なので発火しない。
    Rule {
        surface: "一声",
        pos_group1: None,
        cue: Cue::NextIn(&["かけ", "掛け", "かける", "掛ける"]),
        reading: "ヒトコエ",
    },
    // 「一行ごと」は詩や文章の行を単位とする言い方なので イチギョウ。
    // 一行 (イッコウ = 同行者の集団) は「ごと」を伴わない。
    // 負の対照: 「一行が到着した」は直後が「が」なので発火しない。
    Rule {
        surface: "一行",
        pos_group1: None,
        cue: Cue::NextIn(&["ごと", "毎"]),
        reading: "イチギョウ",
    },
    // 「兵ども」= ツワモノども。
    // 負の対照: 「兵の数」は直後が「の」なので発火しない。
    Rule {
        surface: "兵",
        pos_group1: None,
        cue: Cue::NextIn(&["ども", "共"]),
        reading: "ツワモノ",
    },
    // 仏号に続く「仏」は ブツ (阿弥陀仏 = アミダブツ)。
    Rule {
        surface: "仏",
        pos_group1: None,
        cue: Cue::PrevIn(&["阿弥陀", "釈迦", "大日", "薬師", "毘盧遮那"]),
        reading: "ブツ",
    },
    // 人を指す名詞に付く接尾辞「方」は複数の敬称なので ガタ。
    // 「読み方」「夕方」のように 1 形態素として解析される語は対象外になる。
    // 「この方」の「方」は接尾辞ではないので発火しない。
    Rule {
        surface: "方",
        pos_group1: Some("接尾"),
        cue: Cue::PrevIn(&[
            "皆様",
            "皆",
            "みんな",
            "あなた",
            "先生",
            "奥様",
            "お客様",
            "親御",
            "殿",
        ]),
        reading: "ガタ",
    },
];

/// 隣接する形態素で読みが決まる語を補正する。
pub(crate) fn modify_context_reading(njd_features: &mut [NjdFeature]) {
    for i in 0..njd_features.len() {
        let Some(rule) = RULES.iter().find(|rule| {
            let node = &njd_features[i];
            if node.string != rule.surface {
                return false;
            }
            if let Some(pos_group1) = rule.pos_group1
                && node.pos_group1 != pos_group1
            {
                return false;
            }
            match rule.cue {
                Cue::NextIn(candidates) => njd_features
                    .get(i + 1)
                    .is_some_and(|next| candidates.contains(&next.string.as_str())),
                Cue::PrevIn(candidates) => i > 0
                    && candidates.contains(&njd_features[i - 1].string.as_str()),
            }
        }) else {
            continue;
        };

        let node = &mut njd_features[i];
        node.read = rule.reading.to_string();
        node.pron = rule.reading.to_string();
        // 読みが変わるとモーラ数も変わりうるため数え直す
        node.mora_size = count_mora(rule.reading) as i32;
    }
}
