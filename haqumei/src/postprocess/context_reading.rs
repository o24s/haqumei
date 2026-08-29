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
//!   その語を含む別の文で確かめる。(例: 「一見さん」に対して「一見して」)
//! - 既定の読みを変えないこと。 既定の読みを変えたい場合は、規則ではなく
//!   辞書の単語コストで調整する。
//!
//! 分割が既に正しい複合語 (「読み方」「夕方」など 1 形態素として解析される語) は
//! 対象の形態素が単独で現れないため、構造的に規則の影響を受けない。
//!
//! # 註釈に付けた数
//!
//! 各規則の直前に置いた `[13 / 239]` は、収集データでその読みが正解だった
//! 箇所を「この規則で直る数 / 規則が無くても読めていた数」に分けたものである。
//!
//! 数えているのは表層形と与える読みの組なので、同じ組に規則が 2 つあるとき
//! (`此処` の 直後が助詞 と 直前が助詞 など) は両方に同じ数が付く。
//!
//! `[0 / 0]` は、この収集データにその語が 1 度も出てこないという意味である。
//! 規則が誤っている証拠ではないが、裏付けも取れていない。

use std::sync::LazyLock;

use rustc_hash::FxHashMap;

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

    /// 指定した語を 1 つ飛ばして遡った形態素の品詞が、いずれかに一致する。
    ///
    /// 例えば、「そう思うより外にない」の `思う` を見るためにあり、
    /// `より` が挟まるので [`Cue::PrevPosIn`] では前の形態素を見られない。
    PrevPosVia(&'static [&'static str], &'static [&'static str]),

    /// 指定した語を 1 つ飛ばして遡った形態素の品詞細分類 1 が、いずれかに一致する。
    PrevPosGroup1Via(&'static [&'static str], &'static [&'static str]),

    /// この形態素より後ろ、文が終わるまでに打ち消しの語がある。
    ///
    /// 「〜より外に〜ない」のように、打ち消しと組でなければ成り立たない読みがある。
    /// [`NEGATIVE_ORIGS`] の原形を持つ形態素を探し、句点に当たったところで止める。
    Negated,

    /// 直前の形態素の品詞が、いずれかに一致する。
    ///
    /// 連濁は「直前に語が直接くっついているか」で決まるので、表層形を列挙する
    /// のではなく品詞で見る。助詞を挟む場合は複合語ではないので発火しない。
    PrevPosIn(&'static [&'static str]),

    /// 直後の形態素の品詞が、いずれかに一致する。
    NextPosIn(&'static [&'static str]),

    /// 直前の形態素の品詞細分類 1 が、いずれかに一致する。
    ///
    /// 名詞のうち代名詞だけを分けたい、のように品詞では粗すぎるときに使う。
    /// `一般` が 名詞-一般 にも 副詞-一般 にも付くように品詞までは絞られないので、
    /// 名詞に限りたいときは [`Cue::All`] で [`Cue::PrevPosIn`] と組み合わせる。
    PrevPosGroup1In(&'static [&'static str]),

    /// 並べた条件をすべて満たす。品詞と品詞細分類のように、片方だけでは
    /// 絞りきれない手がかりを組み合わせるのに使う。
    All(&'static [Cue]),
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
    // 「一見さん」の 一見 は イチゲン と読む。(初めての客のこと)
    // 負の対照: 「一見して分かる」は直後が「し」なので発火しない。
    // [0 / 0]
    Rule {
        surface: "一見",
        pos_group1: None,
        cue: Cue::NextIn(&["さん"]),
        reading: "イチゲン",
    },
    // 「一声かける」= ヒトコエかける。
    // 負の対照: 「一声も出ない」は直後が「も」なので発火しない。
    // [0 / 0]
    Rule {
        surface: "一声",
        pos_group1: None,
        cue: Cue::NextIn(&["かけ", "掛け", "かける", "掛ける"]),
        reading: "ヒトコエ",
    },
    // 「一行ごと」は詩や文章の行を単位とする言い方なので イチギョウ。
    // 一行 (イッコウ = 同行者の集団) は「ごと」を伴わない。
    // 負の対照: 「一行が到着した」は直後が「が」なので発火しない。
    // [0 / 0]
    Rule {
        surface: "一行",
        pos_group1: None,
        cue: Cue::NextIn(&["ごと", "毎"]),
        reading: "イチギョウ",
    },
    // 「兵ども」= ツワモノども。
    // 負の対照: 「兵の数」は直後が「の」なので発火しない。
    // [1 / 0]
    Rule {
        surface: "兵",
        pos_group1: None,
        cue: Cue::NextIn(&["ども", "共"]),
        reading: "ツワモノ",
    },
    // 人を指す名詞に付く接尾辞「方」は複数の敬称なので ガタ。
    // 「読み方」「夕方」のように 1 形態素として解析される語は対象外になる。
    // 「この方」の「方」は接尾辞ではないので発火しない。
    // [19 / 0]
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
    // [7 / 0]
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
    // 名詞に直接続く「不足」は連濁して ブソク になる。(栄養不足 / 資金不足 / 経験不足)
    //
    // 連濁は複合語の後部要素でしか起きないので、直前に語が助詞を挟まずに
    // くっついているかで決まる。表層形を列挙する種類の条件ではないため、
    // 品詞で見る cue を用意した。
    //
    // 負の対照: 「食料が不足する」「不足を補う」は直前が助詞または文頭なので
    // 発火しない。「〜の不足」も同様。
    // 名詞の後の「不足」は連濁して ブソク である。(人材不足・医師不足・資源不足)
    // [19 / 0]
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
    // 正しく読めている件数より誤りが桁で多いものだけ採用した。
    //
    // 「〜焼」は陶磁器の産地に付く。 (笠間焼 / 益子焼)
    // [10 / 2]
    Rule {
        surface: "焼",
        pos_group1: Some("接尾"),
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "ヤキ",
    },
    // 「〜峡」は渓谷の名である。 (匹見峡 / 寒霞渓) カイ は単独の訓読み。
    // [0 / 0]
    Rule {
        surface: "峡",
        pos_group1: Some("一般"),
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "キョー",
    },
    // 「分屯」「駐屯」の 屯 は トン。タムロ は「屯する」の訓読み。
    // [0 / 0]
    Rule {
        surface: "屯",
        pos_group1: Some("サ変接続"),
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "トン",
    },
    // 「九角形」「多角形」の 角形 は カクケー。
    //
    // 促音便になる 三角形 / 六角形 / 八角形 は辞書に単独エントリがあるので、
    // ここには来ない。ここに来るのは 18角形 のように数字が前に付く場合で、
    // 促音便は 六 (ロク -> ロッ) のように数字側で起きる現象なので、
    // 角形 の規則として書くと誤る。直前の数だけを見る例外を置いたら、
    // 十八角形 が 直前の 八 に反応して 23 件壊れた。
    // [0 / 0]
    Rule {
        surface: "角形",
        pos_group1: Some("一般"),
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "カクケー",
    },
    // 「軍艦旗」「連隊旗」の 旗 は キ。単独の 旗 は ハタ のまま。
    // [0 / 0]
    Rule {
        surface: "旗",
        pos_group1: Some("一般"),
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "キ",
    },
    // サ変接続の名詞 + 「前」+ 助詞 は マエ と読む。複合語の内側ではなく句の頭に立つ形で、
    // 日本語としては 就学 + 前の と切れる。直後が名詞なら複合語なので ゼン のままに
    // する。(就学前の児童 マエ / 就学前教育 ゼン / 出生前診断 ゼン)
    //
    // この手がかりを持つ箇所は マエ が 52、ゼン が 1 で、対立する 1 件 (..卒前に) は
    // この規則があってもなくても マエ と読まれている。
    //
    // 負の対照: 紀元前の・門前まで・術前の は直前が 名詞-一般 なので発火しない。
    // 食前・生前・産前 は 1 形態素なので 前 が単独で現れない。
    // [0 / 785]
    Rule {
        surface: "前",
        pos_group1: None,
        cue: Cue::All(&[
            Cue::PrevPosIn(&["名詞"]),
            Cue::PrevPosGroup1In(&["サ変接続"]),
            Cue::NextPosIn(&["助詞"]),
        ]),
        reading: "マエ",
    },
    // 「〜前」は直前の語で読みが決まる。171 件を見ると、ゼン になる語と
    // マエ になる語で直前語がまったく重ならなかったので、集合で書ける。
    //
    // ゼン: 紀元前 (74) / 門前 / 生前 / 就学前 / 出生前 / 公判前
    // マエ: 駅前 (16) / 唇前 (13) / 蔵前 / 千日前 / 公園前 / 錠前
    //
    // 負の対照: 駅前・公園前・蔵前 は集合に無いので発火しない。
    //
    // 「就学」と「出生」は後ろに何が来るかで割れる。直後が名詞なら複合語なので
    // ゼン、助詞なら句の頭なので マエ で、サ変接続を見る上の規則が先に分ける。
    //
    // 手がかりを「直後が の」だけにするとこの規則より先でも後ろでも駄目である。
    // 先に置くと `紀元前の文明` が キゲンマエ になり、後ろに置くと 1 件も
    // 発火しない。 (この一覧に載っていない 前 は、直後が の なら既定で マエ)
    // [29 / 0]
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
    // 直接続く場合は バシ が多数なので、これを採る。
    //
    // バシ と ハシ は地名側の慣用で決まり (川 は両方に現れる) 隣接語では
    // 切れないので、多数の バシ を採って ハシ は諦める。
    //
    // 規則は上から順に評価されるので、キョー の集合を先に置く。
    // [0 / 7]
    Rule {
        surface: "橋",
        pos_group1: None,
        cue: Cue::PrevIn(&[
            "高架", "可動", "水管", "人道", "跨道", "跨線", "連絡", "斜張", "張",
            "河口", "併用", "吊", "桁", "鉄道", "歩道", "陸", "仮設", "道路", "段",
            "アーチ", "トラス", "ラーメン", "ＰＣ",
        ]),
        reading: "キョー",
    },
    // [24 / 0]
    Rule {
        surface: "橋",
        pos_group1: Some("接尾"),
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "バシ",
    },
    // 「〜寺」は ジ 83% / デラ 11%。デラ になるのは和語が前に来るときだが
    // 開いた集合なので、観測された分だけ例外に置いて残りは ジ にする。
    // (清水寺 のように 1 エントリで登録されている寺は影響を受けない)
    // [0 / 0]
    Rule {
        surface: "寺",
        pos_group1: None,
        cue: Cue::PrevIn(&["縁切", "猫", "だるま", "隠れ", "峯", "山", "花"]),
        reading: "デラ",
    },
    // 直前が副詞的な名詞なら、その「寺」は複合語の後部要素ではなく独立した名詞
    // なので テラ になる。「翌日寺へ出かけて」「毎日寺へ通う」「そのまま寺へ転げこんだ」
    // 「若いとき寺に居た」がこれで、下の 名詞 の規則が ジ を書き込んでいた。
    //
    // 収集データ 124 例では、直前が 副詞可能 / 非自立 のとき テラ 以外の読みは
    // 1 件も無い。寺の名前が副詞的な名詞で始まることが無いためである。
    // 負の対照: 仏頂寺 (固有名詞) と 大仏寺 (一般) は影響しない。
    // [0 / 74]
    Rule {
        surface: "寺",
        pos_group1: None,
        cue: Cue::PrevPosGroup1In(&["副詞可能", "非自立"]),
        reading: "テラ",
    },
    // [34 / 0]
    Rule {
        surface: "寺",
        pos_group1: None,
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "ジ",
    },
    // 「叙事詩環」「循環」の 環 は カン になる。単独は タマキ / ワ。
    // [6 / 3]
    Rule {
        surface: "環",
        pos_group1: None,
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "カン",
    },
    // 「諏訪洞」「鍾乳洞」の 洞 は ドー になる。単独は ホラ。
    // [12 / 0]
    Rule {
        surface: "洞",
        pos_group1: None,
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "ドー",
    },
    // 「宮沢湖」「琵琶湖」の 湖 は コ になる。単独は ミズウミ。
    // [3 / 9]
    Rule {
        surface: "湖",
        pos_group1: None,
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "コ",
    },
    // 「唇歯音」「小陰唇」の 唇 は シン になる。単独は クチビル。
    // [0 / 0]
    Rule {
        surface: "唇",
        pos_group1: None,
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "シン",
    },
    // 「風景印」「消印」の 印 は イン になる。単独は シルシ。
    // [7 / 0]
    Rule {
        surface: "印",
        pos_group1: None,
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "イン",
    },
    // 「筆子塚」「一里塚」の 塚 は連濁して ズカ になる。単独は ツカ。
    // [9 / 0]
    Rule {
        surface: "塚",
        pos_group1: None,
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "ズカ",
    },
    // 「水晶小屋」「山小屋」の 小屋 は連濁して ゴヤになる。単独は コヤ。
    // 「寺小屋」は テラコヤ で連濁しない。寺子屋 の異表記である。
    // [0 / 21]
    Rule {
        surface: "小屋",
        pos_group1: None,
        cue: Cue::PrevIn(&["寺"]),
        reading: "コヤ",
    },
    // [23 / 0]
    Rule {
        surface: "小屋",
        pos_group1: None,
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "ゴヤ",
    },
    // 「貴乃花部屋」「子供部屋」の 部屋 は連濁して ベヤ になる。単独は ヘヤ。
    // [18 / 0]
    Rule {
        surface: "部屋",
        pos_group1: None,
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "ベヤ",
    },
    // 「お年玉付郵便はがき」の 付 は ツキ になる。ズケ は 日付 などの別エントリ。
    // [26 / 0]
    Rule {
        surface: "付",
        pos_group1: Some("接尾"),
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "ツキ",
    },
    // 「金車」のような複合語の 金 は キン になる。単独は カネ。
    // [0 / 112]
    Rule {
        surface: "金",
        pos_group1: Some("一般"),
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "キン",
    },
    // 「公同書簡」「公文書」の 公 は コー になる。単独は オーヤケ。
    // 直前が副詞的な名詞なら、その「公」は複合語の後部要素ではないので オーヤケ である。
    // 「いつか公になる」がこれで、下の 名詞 の規則が コー を書き込んでいた。
    // 負の対照: 信長公・秀吉公 は 固有名詞 なので影響しない。
    // [0 / 16]
    Rule {
        surface: "公",
        pos_group1: Some("一般"),
        cue: Cue::PrevPosGroup1In(&["副詞可能"]),
        reading: "オーヤケ",
    },
    // [8 / 30]
    Rule {
        surface: "公",
        pos_group1: Some("一般"),
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "コー",
    },
    // 「歯茎硬口蓋音」の 硬 は コー になる。辞書には形容詞の カタ しか無い。
    // [1 / 0]
    Rule {
        surface: "硬",
        pos_group1: None,
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "コー",
    },
    // 「如何ですか」の 如何 は イカガ になる。「如何なる」「如何にも」は イカ。
    // [3 / 0]
    Rule {
        surface: "如何",
        pos_group1: None,
        cue: Cue::NextIn(&["で", "です", "でし", "でしょ"]),
        reading: "イカガ",
    },

    // 形容動詞語幹 + 「物」は ブツ と読む。(危険物・不要物・貴重物)
    // 負の対照: 「着物」「建物」は 1 形態素なので 物 が単独で現れない。
    // [13 / 239]
    Rule {
        surface: "物",
        pos_group1: None,
        cue: Cue::PrevPosGroup1In(&["形容動詞語幹"]),
        reading: "ブツ",
    },
    // 名詞 + 「尼」は接尾辞の ニ と読む。(修道尼・比丘尼)
    // [10 / 0]
    Rule {
        surface: "尼",
        pos_group1: None,
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "ニ",
    },
    // 活用語 + 「者」は モノ と読む。(成り上がり者・若い者・働く者)
    // 医者・記者 は 1 形態素なので 者 が単独で現れない。
    // [8 / 407]
    Rule {
        surface: "者",
        pos_group1: None,
        cue: Cue::PrevPosGroup1In(&["自立"]),
        reading: "モノ",
    },
    // 活用語 + 「処」は トコロ と読む。(たべる処・住む処)
    // [19 / 18]
    Rule {
        surface: "処",
        pos_group1: None,
        cue: Cue::PrevPosGroup1In(&["自立"]),
        reading: "トコロ",
    },
    // 「に於て」「に於ける」の 於 は オイ と読む。
    // [26 / 0]
    Rule {
        surface: "於",
        pos_group1: None,
        cue: Cue::NextIn(&["て", "ては", "ても"]),
        reading: "オイ",
    },
    // 名詞 + 「茶屋」は連濁して ジャヤ と読む。(料理茶屋・芝居茶屋・三日月茶屋)
    // 副詞的な名詞を外すのは 寺 と同じ理由で、`きょう茶屋町へ` の 茶屋 は
    // 複合語の後部要素ではない。
    // [9 / 0]
    Rule {
        surface: "茶屋",
        pos_group1: None,
        cue: Cue::All(&[
            Cue::PrevPosIn(&["名詞"]),
            Cue::PrevPosGroup1In(&["一般", "固有名詞", "サ変接続"]),
        ]),
        reading: "ジャヤ",
    },
    // 動詞の連用形 + 「入っ」は イッ と読む。(見入った・立ち入った)
    // 負の対照: 「手に入っ」は直前が助詞なので発火しない。
    // [12 / 0]
    Rule {
        surface: "入っ",
        pos_group1: None,
        cue: Cue::PrevPosIn(&["動詞"]),
        reading: "イッ",
    },
    // 「何で」「何でも」の 何 は ナン と読む。
    // [31 / 364]
    Rule {
        surface: "何",
        pos_group1: None,
        cue: Cue::NextIn(&["で", "でも"]),
        reading: "ナン",
    },
    // 「傍へ」「傍に」の 傍 は ソバ と読む。
    // [22 / 0]
    Rule {
        surface: "傍",
        pos_group1: None,
        cue: Cue::NextIn(&["へ", "に"]),
        reading: "ソバ",
    },
    // 助詞の後の「此処」は ココ と読む。
    // [30 / 1]
    Rule {
        surface: "此処",
        pos_group1: None,
        cue: Cue::PrevPosIn(&["助詞"]),
        reading: "ココ",
    },
    // 「〜するより外にない」「君より外にない」の 外 は ホカ と読む。
    //
    // 空間の比較に ソト が使われるため、「より」と打ち消しだけでは足りず、
    // 「物体AはBより外にない」「地球より外に惑星はない」が ソト になるべきである。
    // 分かれるのは「より」の直前の品詞で、動詞と代名詞の後に空間の比較は来ない。
    // [9 / 0]
    Rule {
        surface: "外",
        pos_group1: None,
        cue: Cue::All(&[Cue::PrevPosVia(&["より"], &["動詞"]), Cue::Negated]),
        reading: "ホカ",
    },
    // [9 / 0]
    Rule {
        surface: "外",
        pos_group1: None,
        cue: Cue::All(&[
            Cue::PrevPosGroup1Via(&["より"], &["代名詞"]),
            Cue::Negated,
        ]),
        reading: "ホカ",
    },
    // 接頭詞の 大 は、後ろが和語なら オー になる。漢語なら ダイ で、
    // 「大企業」「大部分」は触らない。和語かどうかは形態素の特徴に無いので、
    // 語を列挙した。大掃除 は後部要素が漢語だが オー で読む慣用である。
    // 一方で、正解が オオジシン の出典と ダイジシン の出典があるため、大地震 は入れていない。
    // 手を入れると 7 箇所で ダイ を オー に変えてしまう。
    // 負の対照: 大企業 ダイキギョー / 大部分 ダイブブン / 大気圧 タイキアツ。
    // [12 / 0]
    Rule {
        surface: "大",
        pos_group1: Some("名詞接続"),
        cue: Cue::NextIn(&[
            "にぎわい", "丸髷", "地主", "旦那", "泥坊", "津波", "掃除",
            "番狂わせ", "番頭", "違い",
        ]),
        reading: "オー",
    },
    // 接頭詞の 御 は、後ろが和語なら オ になる。漢語なら ゴ で、
    // 「御住所」「御多忙」は触らない。「御粗末」は漢語だが オソマツ が慣用である。
    // 負の対照: 御住所 ゴジューショ / 御多忙 ゴタボー。
    // [18 / 0]
    Rule {
        surface: "御",
        pos_group1: Some("名詞接続"),
        cue: Cue::NextIn(&[
            "仕置", "屋敷", "帰り", "急ぎ", "手際", "支払い", "楽しみ",
            "気の毒", "田植祭", "神籤", "粗末", "言葉", "近く", "隣", "嬢",
        ]),
        reading: "オ",
    },
    // 「〜町史」の 町 は チョー と読む。
    // [6 / 0]
    Rule {
        surface: "町",
        pos_group1: None,
        cue: Cue::NextIn(&["史"]),
        reading: "チョー",
    },
    // 名詞 + 「余」は接尾辞の ヨ と読む。(万余・十余)
    // [8 / 80]
    Rule {
        surface: "余",
        pos_group1: None,
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "ヨ",
    },
    // 「前期」「前段階」と、役職が直後に来る「前会長」の 前 は ゼン と読む。
    // 役職は収集データで 会長 280 / 社長 165 / 首相 129 / 理事長 71 / 大統領 56 と
    // 続き、いずれも マエ には出てこない。直後に付く語だけを見るので、
    // 「〜の前、会長が」のように読点が挟まる形とは区別が付く。
    // [29 / 0]
    Rule {
        surface: "前",
        pos_group1: None,
        cue: Cue::NextIn(&[
            "会長", "大統領", "段階", "理事長", "社長", "総裁", "期", "首相",
        ]),
        reading: "ゼン",
    },

    // 「金型」の 金 は カナ と読む。
    // [10 / 0]
    Rule {
        surface: "金",
        pos_group1: None,
        cue: Cue::NextIn(&["型"]),
        reading: "カナ",
    },
    // 「一目で」の 一目 は ヒトメ と読む。
    // [5 / 0]
    Rule {
        surface: "一目",
        pos_group1: None,
        cue: Cue::NextIn(&["で"]),
        reading: "ヒトメ",
    },
    // 格助詞の後の「叩き」は タタキ と読む。
    // [5 / 3]
    Rule {
        surface: "叩き",
        pos_group1: None,
        cue: Cue::PrevPosGroup1In(&["格助詞"]),
        reading: "タタキ",
    },
    // 「此処に」「此処へ」の 此処 は ココ と読む。
    // [30 / 1]
    Rule {
        surface: "此処",
        pos_group1: None,
        cue: Cue::NextIn(&["に", "へ", "で", "が", "は", "を"]),
        reading: "ココ",
    },
    // 活用語 + 「処」は トコロ と読む。助動詞のあとも同じである。(落ち込んでいた処)
    // [19 / 18]
    Rule {
        surface: "処",
        pos_group1: None,
        cue: Cue::PrevPosIn(&["助動詞"]),
        reading: "トコロ",
    },
    // 副詞 + 「間」は マ と読む。(少し間をおいて)
    // [7 / 0]
    Rule {
        surface: "間",
        pos_group1: None,
        cue: Cue::PrevPosIn(&["副詞"]),
        reading: "マ",
    },
    // 「この間の」の この間 は コノアイダ と読む。
    // [6 / 0]
    Rule {
        surface: "この間",
        pos_group1: None,
        cue: Cue::NextIn(&["の"]),
        reading: "コノアイダ",
    },
    // 「識って」は シッテ と読む。(知って の異表記)
    // [7 / 0]
    Rule {
        surface: "識",
        pos_group1: None,
        cue: Cue::NextIn(&["って", "った", "り"]),
        reading: "シ",
    },
    // 名詞 + 「翁」は オー と読む。(芭蕉翁・計算翁)
    // [8 / 8]
    Rule {
        surface: "翁",
        pos_group1: None,
        cue: Cue::PrevPosIn(&["名詞"]),
        reading: "オー",
    },
    // 「仰しゃる」は オッシャル と読む。
    // [6 / 0]
    Rule {
        surface: "仰",
        pos_group1: None,
        cue: Cue::NextIn(&["しゃっ", "しゃる", "しゃい", "しゃら", "しゃり"]),
        reading: "オッ",
    },
    // 「〜て了った」は テシマッタ と読む。
    // [28 / 0]
    Rule {
        surface: "了",
        pos_group1: None,
        cue: Cue::PrevIn(&["て", "で"]),
        reading: "シマ",
    },
    // 「身体を」の 身体 は カラダ と読む。
    // [20 / 0]
    Rule {
        surface: "身体",
        pos_group1: None,
        cue: Cue::NextIn(&["を"]),
        reading: "カラダ",
    },
    // 「一しょ」は イッショ と読む。(一緒 の異表記)
    // [15 / 144]
    Rule {
        surface: "一",
        pos_group1: None,
        cue: Cue::NextIn(&["しょ"]),
        reading: "イッ",
    },
    // 格助詞・係助詞の前の 貴女 は アナタ と読む。
    // 「の」を入れないのは、`若い貴女のために` の キジョ と割れるためである。
    // [21 / 0]
    Rule {
        surface: "貴女",
        pos_group1: None,
        cue: Cue::NextIn(&["が", "は", "を", "に", "も", "から", "と"]),
        reading: "アナタ",
    },

    // 「等」の読みは直前が何かで三分される。収集データでの内訳は
    //
    //   代名詞 + 等           -> ラ    391 件 (それ等 / これ等 / われ等)
    //   自立した名詞 + 等     -> トー  105 件 (機器等 / 部品等 / 主蒸気系等)
    //   活用語・形式名詞 + 等 -> ナド  101 件 (した等 / ない等 / こと等)
    //
    // で、既定の ナド は前の 2 つの文脈では負けている。接尾の ラ と トー は
    // 文脈 ID が同じなのでコストでは分けられない。どちらを安くしても勝敗が
    // 全用例で一斉に決まるので、ここで分ける。
    //
    // 「こと」「もの」のような非自立名詞は句を名詞化しているだけなので、
    // 活用語と同じく ナド の側に残す。そのため直前の品詞細分類 1 を正の側で
    // 列挙する。数詞を入れないのは 一等 / 三等 が助数詞だからである。
    //
    // 辞書の状態によって「等」自体が 名詞-一般 にも 名詞-接尾 にもなるので、
    // 対象側の品詞細分類は問わない。
    // [20 / 0]
    Rule {
        surface: "等",
        pos_group1: None,
        cue: Cue::PrevPosGroup1In(&["代名詞"]),
        reading: "ラ",
    },
    // [88 / 153]
    Rule {
        surface: "等",
        pos_group1: None,
        cue: Cue::PrevPosGroup1In(&["一般", "サ変接続", "固有名詞", "接尾"]),
        reading: "トー",
    },

    // 学位を指す「博士」は ハクシ で、人を指す「〜博士」の ハカセ と割れる。
    //
    // 直後の語は ハクシ が 学位 6,940 / 論文 519 で、ハカセ の
    // (の 842 / は 400 / と 187 / が 177) とは重ならない。直前の語も ハクシ が
    // 医学 272 / 工学 216 / 大学院 150 / 理学 111 で、ハカセ の側は 魔法 244 と人名
    // (宗像 132 / 蛭田 55 / 黒井 53) である。
    //
    // 負の対照: 「広瀬博士は」「物知り博士と」「おもちゃ博士と」は直前が姓や
    // 修飾語、直後が助詞なので発火しない。「修士や博士の学位をもっている」も
    // 直後が「の」なので発火しない。
    // [18 / 0]
    Rule {
        surface: "博士",
        pos_group1: None,
        cue: Cue::NextIn(&["学位", "論文", "号", "課程"]),
        reading: "ハクシ",
    },
    // 直前が学位の分野名か 大学院 のとき。収集データで裏付けがあるのは
    // 医学・工学・理学と 大学院 で、残りは同じ形の分野名として足した。
    // [18 / 0]
    Rule {
        surface: "博士",
        pos_group1: None,
        cue: Cue::PrevIn(&[
            "医学", "工学", "理学", "農学", "薬学", "文学", "法学", "経済学", "大学院",
        ]),
        reading: "ハクシ",
    },
    // 道具の「〜の柄」は エ、刀剣の「〜の柄」は ツカ と読む。
    // 直前は「の」で潰れるので、その 1 つ前の語を見る。
    // 収集データでは エ が 槍 56 / 鍬 7 / 斧 6 / 柄杓 6 / 団扇 4 / 薙刀 4 / 傘 3、
    // ツカ が 刀 72 / 太刀 14 / 脇差 9 / 剣 8 / 大刀 5 / 短剣 5 と分かれる。
    // かな書きの表層形もあるので併記する。
    // 負の対照: 「柄が悪い」「大柄な人」は直前が「の」ではないので発火しない。
    // [4 / 0]
    Rule {
        surface: "柄",
        pos_group1: None,
        cue: Cue::PrevViaNo(&[
            "うちわ", "やり", "傘", "剃刀", "団扇", "提灯", "斧", "柄杓", "槍", "洋傘",
            "箒", "薙刀", "鋏", "鋤", "鋸", "鍬", "錫杖",
        ]),
        reading: "エ",
    },
    // [2 / 0]
    Rule {
        surface: "柄",
        pos_group1: None,
        cue: Cue::PrevViaNo(&[
            "刀", "剣", "大刀", "太刀", "小剣", "懐剣", "木剣", "短刀", "短剣", "脇差",
            "軍刀", "長脇差", "鎧通し",
        ]),
        reading: "ツカ",
    },
    // 器物の「〜の縁」は フチ。既定の エン は 縁がある / 縁側 の意味で、
    // 「の」を挟んで器物が来る形では出てこない。
    // 負の対照: 「縁を切る」「血縁」は直前が「の」ではないので発火しない。
    // [15 / 0]
    Rule {
        surface: "縁",
        pos_group1: None,
        cue: Cue::PrevViaNo(&[
            "カップ", "器", "堀", "崖", "径", "屏風", "内側", "口", "皿", "模様",
            "火鉢", "金",
        ]),
        reading: "フチ",
    },
];

/// [`Cue::Negated`] が探す打ち消しの語の原形。
const NEGATIVE_ORIGS: &[&str] = &["ない", "無い", "ぬ", "ん", "まい", "ず"];

/// [`Cue::All`] が手がかりを入れ子にできるので、判定は再帰で書く。
fn cue_matches(cue: &Cue, njd_features: &[NjdFeature], i: usize) -> bool {
    match cue {
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
        Cue::PrevPosVia(via, candidates) => {
            i >= 2
                && via.contains(&njd_features[i - 1].string.as_str())
                && candidates.contains(&njd_features[i - 2].pos.as_str())
        }
        Cue::PrevPosGroup1Via(via, candidates) => {
            i >= 2
                && via.contains(&njd_features[i - 1].string.as_str())
                && candidates.contains(&njd_features[i - 2].pos_group1.as_str())
        }
        Cue::Negated => njd_features
            .get(i + 1..)
            .unwrap_or_default()
            .iter()
            .take_while(|f| !matches!(f.string.as_str(), "。" | "．" | "！" | "？"))
            .any(|f| NEGATIVE_ORIGS.contains(&f.orig.as_str())),
        Cue::PrevPosIn(candidates) => {
            i > 0 && candidates.contains(&njd_features[i - 1].pos.as_str())
        }
        Cue::PrevPosGroup1In(candidates) => {
            i > 0 && candidates.contains(&njd_features[i - 1].pos_group1.as_str())
        }
        Cue::NextPosIn(candidates) => njd_features
            .get(i + 1)
            .is_some_and(|next| candidates.contains(&next.pos.as_str())),
        Cue::All(cues) => cues.iter().all(|c| cue_matches(c, njd_features, i)),
    }
}

/// 表層形から、その表層形を対象にする規則を引く。並びは [`RULES`] の順のままで、
/// 上から評価して最初に一致したものを使う性質を保つ。
static RULES_BY_SURFACE: LazyLock<FxHashMap<&'static str, Vec<&'static Rule>>> =
    LazyLock::new(|| {
        let mut by_surface: FxHashMap<&'static str, Vec<&'static Rule>> = FxHashMap::default();
        for rule in RULES.iter() {
            by_surface.entry(rule.surface).or_default().push(rule);
        }
        by_surface
    });

/// 隣接する形態素で読みが決まる語を補正する。
pub(crate) fn modify_context_reading(njd_features: &mut [NjdFeature]) {
    let by_surface = &*RULES_BY_SURFACE;
    for i in 0..njd_features.len() {
        let Some(rules) = by_surface.get(njd_features[i].string.as_str()) else {
            continue;
        };
        let Some(rule) = rules.iter().copied().find(|rule| {
            let node = &njd_features[i];
            if let Some(pos_group1) = rule.pos_group1
                && node.pos_group1 != pos_group1
            {
                return false;
            }
            cue_matches(&rule.cue, njd_features, i)
        }) else {
            continue;
        };

        let node = &mut njd_features[i];
        node.read = rule.reading.to_string();
        node.pron = rule.reading.to_string();
        // 読みが変わるとモーラ数も変わりうるため数え直す
        let old_mora = node.mora_size;
        node.mora_size = count_mora(rule.reading) as i32;
        shift_accent_nucleus(njd_features, i, old_mora);
    }
}

/// 読みを書き換えた形態素のモーラ数が変わったぶん、アクセント句の核の位置をずらす。
///
/// `modify_context_reading` は `njd_set_accent_type` の後に走るので、
/// 核の位置は書き換える前のモーラ数で数えたものである。
/// そのため、アクセント核の位置をモーラ数の変更に合わせてシフトする。
///
/// 例えば、`唐詩選余師` の `余` が アマリ (3 モーラ) から ヨ (1 モーラ) になると、
/// 句が 9 モーラから 7 モーラになるのに核は 8 のままで、句の外を指すので核が消える。
///
/// 読みが短くなると、核が書き換えた形態素の中にあったときにその位置が無くなるので、
/// その形態素の末尾のモーラに置く。
fn shift_accent_nucleus(njd_features: &mut [NjdFeature], i: usize, old_mora: i32) {
    let new_mora = njd_features[i].mora_size;
    if new_mora == old_mora {
        return;
    }
    // アクセント句の先頭を探す。chain_flag が 1 の形態素は前に繋がっている
    let mut head = i;
    while head > 0 && njd_features[head].chain_flag == 1 {
        head -= 1;
    }
    let before: i32 = njd_features[head..i].iter().map(|f| f.mora_size).sum();
    let acc = njd_features[head].acc;
    if acc <= before {
        // 核は書き換えた形態素より前にあるので変更しない。
        return;
    }
    njd_features[head].acc = if acc > before + old_mora {
        acc + new_mora - old_mora
    } else {
        before + (acc - before).min(new_mora)
    };
}
