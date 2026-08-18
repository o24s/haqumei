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
    /// 直前が「の」で、その 1 つ前の形態素の表層形がいずれかに一致する。
    ///
    /// 「〜の下」のように連体助詞を挟む構造のために用意している。
    PrevViaNo(&'static [&'static str]),
    /// 直前の形態素の品詞が、いずれかに一致する。
    ///
    /// 連濁は「直前に語が直接くっついているか」で決まるので、表層形を列挙する
    /// のではなく品詞で見る。助詞を挟む場合は複合語ではないので発火しない。
    PrevPosIn(&'static [&'static str]),
    /// 直前の形態素の品詞細分類 1 が、いずれかに一致する。
    ///
    /// 名詞のうち代名詞だけを分けたい、のように品詞では粗すぎるときに使う。
    PrevPosGroup1In(&'static [&'static str]),
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
#[rustfmt::skip]
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
    // 「X の下」で X が抽象名詞・権威を表す語のとき、「下」は モト。
    //
    // 「机の下」「木の下」のように具体物が来る場合の シタ とは、直前の名詞で
    // 区別できる。集合を閉じた形にできるのはそのためで、意味を判定している
    // わけではない。
    //
    // 負の対照: 「机の下に置いた」「橋の下をくぐる」は集合に無いので発火しない。
    // 「下を向く」「階段の下」も同様。
    Rule {
        surface: "下",
        pos_group1: Some("一般"),
        cue: Cue::PrevViaNo(&[
            // 権威・統制
            "支配", "統治", "指導", "指揮", "監督", "管理", "監視", "命令", "号令",
            "庇護", "保護", "援助", "協力", "後援", "統制", "占領",
            // 枠組み・条件
            "名", "法", "条件", "前提", "仮定", "原則", "方針", "契約", "規定",
            "制度", "計画", "設定",
            // 心情・関係
            "愛情", "信頼", "理解", "合意", "影響", "配慮",
            // 人 (師弟・君臣)
            "恩師", "陛下", "殿下", "親方",
        ]),
        reading: "モト",
    },
    // 名詞に直接続く「不足」は連濁して ブソク (栄養不足 / 資金不足 / 経験不足)。
    //
    // 連濁は複合語の後部要素でしか起きないので、直前に語が**助詞を挟まずに**
    // くっついているかで決まる。表層形を列挙する種類の条件ではないため、
    // 品詞で見る cue を用意した。
    //
    // 負の対照: 「食料が不足する」「不足を補う」は直前が助詞または文頭なので
    // 発火しない。「〜の不足」も同様。
    Rule {
        surface: "不足",
        pos_group1: Some("サ変接続"),
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "ブソク",
    },
    // 名詞に直接続く後部要素は、単独のときと読みが変わる。単独用法の読みが
    // 必要なので単語コストでは分けられず (山 は単独なら ヤマ)、複合語かどうかで
    // 切る必要がある。
    //
    // 収集データ 67,300 対で、正しく読めている件数より誤りが桁で多いものだけ採用した。
    // 括弧内は (誤り / 正しく読めていた数)。
    //
    // 「〜焼」は陶磁器の産地に付く (笠間焼 / 益子焼)。(59 / 0)
    Rule {
        surface: "焼",
        pos_group1: Some("接尾"),
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "ヤキ",
    },
    // 「〜峡」は渓谷の名 (匹見峡 / 寒霞渓)。カイ は単独の訓読み。(34 / 0)
    Rule {
        surface: "峡",
        pos_group1: Some("一般"),
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "キョー",
    },
    // 「分屯」「駐屯」の 屯 は トン。タムロ は「屯する」の訓読み。(28 / 0)
    Rule {
        surface: "屯",
        pos_group1: Some("サ変接続"),
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "トン",
    },
    // 「九角形」「多角形」の 角形 は カクケー。(41 / 0)
    //
    // 促音便になる 三角形 / 六角形 / 八角形 は辞書に単独エントリがあるので、
    // ここには来ない。ここに来るのは 18角形 のように数字が前に付く場合で、
    // 促音便は 六 (ロク -> ロッ) のように数字側で起きる現象なので、
    // 角形 の規則として書くと誤る。直前の数だけを見る例外を置いたら、
    // 十八角形 が 直前の 八 に反応して 23 件壊れた。
    Rule {
        surface: "角形",
        pos_group1: Some("一般"),
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "カクケー",
    },
    // 「室町通」「四条通」の 通 は ドーリ。人名の トール が勝っていた。(28 / 0)
    Rule {
        surface: "通",
        pos_group1: Some("固有名詞"),
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "ドーリ",
    },
    // 「軍艦旗」「連隊旗」の 旗 は キ。単独の 旗 は ハタ のまま。(58 / 9)
    Rule {
        surface: "旗",
        pos_group1: Some("一般"),
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "キ",
    },
    // 「〜前」は直前の語で読みが決まる。171 件を見ると、ゼン になる語と
    // マエ になる語で直前語がまったく重ならなかったので、集合で書ける。
    //
    // ゼン: 紀元前 (74) / 門前 / 生前 / 就学前 / 出生前 / 公判前
    // マエ: 駅前 (16) / 唇前 (13) / 蔵前 / 千日前 / 公園前 / 錠前
    //
    // 負の対照: 駅前・公園前・蔵前 は集合に無いので発火しない。
    Rule {
        surface: "前",
        pos_group1: None,
        cue: Cue::PrevIn(&[
            "紀元", "門", "生", "就学", "出生", "公判", "産", "術", "食",
            "陸", "膝蓋", "祝典", "患難", "停滞", "閉塞", "寒帯", "温暖", "暴露",
        ]),
        reading: "ゼン",
    },
    // 「〜橋」は バシ 68% / キョー 17% / ハシ 13%。キョー になるのは橋梁の
    // 構造や種別を表す語のときで、そこは閉じた集合になる。それ以外で名詞に
    // 直接続く場合は バシ が多数なのでそちらにする。
    //
    // バシ と ハシ は地名側の慣用で決まり (川 は両方に現れる) 隣接語では
    // 切れないので、多数の バシ を採って ハシ は諦める。
    //
    // 規則は上から順に評価されるので、キョー の集合を先に置く。
    Rule {
        surface: "橋",
        pos_group1: None,
        cue: Cue::PrevIn(&[
            "高架", "可動", "水管", "人道", "跨道", "跨線", "連絡", "斜張", "張",
            "河口", "併用", "吊", "桁", "鉄道", "歩道", "陸", "仮設",
            "アーチ", "トラス", "ラーメン",
        ]),
        reading: "キョー",
    },
    Rule {
        surface: "橋",
        pos_group1: Some("接尾"),
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "バシ",
    },
    // 「〜寺」は ジ 83% / デラ 11%。デラ になるのは和語が前に来るときだが
    // 開いた集合なので、観測された分だけ例外に置いて残りは ジ にする。
    // (清水寺 のように 1 エントリで登録されている寺は影響を受けない)
    Rule {
        surface: "寺",
        pos_group1: None,
        cue: Cue::PrevIn(&["縁切", "猫", "だるま", "隠れ", "峯", "山", "花"]),
        reading: "デラ",
    },
    Rule {
        surface: "寺",
        pos_group1: None,
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "ジ",
    },
    // 「防衛記念章」「褒章」の 章 は ショー。人名の アキラ が勝っていた。(28 / 4)
    Rule {
        surface: "章",
        pos_group1: Some("固有名詞"),
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "ショー",
    },
    // 「叙事詩環」「循環」の 環 は カン。単独は タマキ / ワ。(24 / 1)
    Rule {
        surface: "環",
        pos_group1: None,
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "カン",
    },
    // 「諏訪洞」「鍾乳洞」の 洞 は ドー。単独は ホラ。(23 / 1)
    Rule {
        surface: "洞",
        pos_group1: None,
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "ドー",
    },
    // 「宮沢湖」「琵琶湖」の 湖 は コ。単独は ミズウミ。(20 / 3)
    Rule {
        surface: "湖",
        pos_group1: None,
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "コ",
    },
    // 「唇歯音」「小陰唇」の 唇 は シン。単独は クチビル。(17 / 0)
    Rule {
        surface: "唇",
        pos_group1: None,
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "シン",
    },
    // 「風景印」「消印」の 印 は イン。単独は シルシ。(16 / 1)
    Rule {
        surface: "印",
        pos_group1: None,
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "イン",
    },
    // 「筆子塚」「一里塚」の 塚 は連濁して ズカ。単独は ツカ。(25 / 3)
    Rule {
        surface: "塚",
        pos_group1: None,
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "ズカ",
    },
    // 「水晶小屋」「山小屋」の 小屋 は連濁して ゴヤ。単独は コヤ。(17 / 3)
    Rule {
        surface: "小屋",
        pos_group1: None,
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "ゴヤ",
    },
    // 「貴乃花部屋」「子供部屋」の 部屋 は連濁して ベヤ。単独は ヘヤ。(45 / 9)
    Rule {
        surface: "部屋",
        pos_group1: None,
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "ベヤ",
    },
    // 「お年玉付郵便はがき」の 付 は ツキ。ズケ は 日付 などの別エントリ。(16 / 0)
    Rule {
        surface: "付",
        pos_group1: Some("接尾"),
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "ツキ",
    },
    // 「金車」のような複合語の 金 は キン。単独は カネ。(16 / 4)
    Rule {
        surface: "金",
        pos_group1: Some("一般"),
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "キン",
    },
    // 「公同書簡」「公文書」の 公 は コー。単独は オーヤケ。(15 / 1)
    Rule {
        surface: "公",
        pos_group1: Some("一般"),
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "コー",
    },
    // 「歯茎硬口蓋音」の 硬 は コー。辞書には形容詞の カタ しか無い。(15 / 2)
    Rule {
        surface: "硬",
        pos_group1: None,
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "コー",
    },
    // 「同じ様に」の 様 は ヨー。「田中様」「王様」の接尾辞は サマ。
    //
    // 「この様に」「以下の様に」は 様 が非自立名詞になり元から ヨー だが、
    // 「同じ様に」だけは人名の接尾辞と解析されて サマ になる。
    Rule {
        surface: "様",
        pos_group1: None,
        cue: Cue::PrevIn(&["同じ"]),
        reading: "ヨー",
    },
    // 「如何ですか」の 如何 は イカガ。「如何なる」「如何にも」は イカ。
    Rule {
        surface: "如何",
        pos_group1: None,
        cue: Cue::NextIn(&["で", "です", "でし", "でしょ"]),
        reading: "イカガ",
    },
    // 「等」の読みは直前が何かで三分される。収集データでの内訳は
    //
    //   代名詞 + 等           -> ラ    391 件 (それ等 / これ等 / われ等)
    //   自立した名詞 + 等     -> トー  105 件 (機器等 / 部品等 / 主蒸気系等)
    //   活用語・形式名詞 + 等 -> ナド  101 件 (した等 / ない等 / こと等)
    //
    // で、既定の ナド は前の 2 つの文脈では負けている。接尾の ラ と トー は
    // 文脈 ID が同じなのでコストでは分けられない (どちらを安くしても全用例で
    // 一斉に決まる)。ここで分ける。
    //
    // 「こと」「もの」のような非自立名詞は句を名詞化しているだけなので、
    // 活用語と同じく ナド の側に残す。そのため直前の品詞細分類 1 を正の側で
    // 列挙する。数詞を入れないのは 一等 / 三等 が助数詞だからである。
    //
    // 辞書の状態によって「等」自体が 名詞-一般 にも 名詞-接尾 にもなるので、
    // 対象側の品詞細分類は問わない。
    Rule {
        surface: "等",
        pos_group1: None,
        cue: Cue::PrevPosGroup1In(&["代名詞"]),
        reading: "ラ",
    },
    Rule {
        surface: "等",
        pos_group1: None,
        cue: Cue::PrevPosGroup1In(&["一般", "サ変接続", "固有名詞", "接尾"]),
        reading: "トー",
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
                Cue::PrevIn(candidates) => {
                    i > 0 && candidates.contains(&njd_features[i - 1].string.as_str())
                }
                Cue::PrevViaNo(candidates) => {
                    i > 1
                        && njd_features[i - 1].string == "の"
                        && candidates.contains(&njd_features[i - 2].string.as_str())
                }
                Cue::PrevPosIn(candidates) => {
                    i > 0 && candidates.contains(&njd_features[i - 1].pos.as_str())
                }
                Cue::PrevPosGroup1In(candidates) => {
                    i > 0 && candidates.contains(&njd_features[i - 1].pos_group1.as_str())
                }
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
