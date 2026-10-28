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
//! - 学習済みモデルを読み込まず、形態素列だけで判定できる
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

use haqumei_jpreprocess_core::{
    accent_rule::AccentType,
    cform::CForm,
    ctype::{CType, Five, One},
    pos::{
        Doushi, Fukushi, Joshi, KakuJoshi, Keiyoushi, Kigou, KoyuMeishi, Meishi, POS, Region,
        Setsubi, Settoushi,
    },
    pronunciation::Pronunciation,
};
use haqumei_jpreprocess_njd::NJDNode;
use rustc_hash::FxHashMap;

use crate::NjdFeature;
use crate::utils::count_mora;

mod prepare;
use prepare::{NodeSpec, RewriteRule, Stage};
pub(crate) use prepare::{prepare_context_segments, prepare_context_words};

// 品詞と分割は句の結合結果を変えるため、PREPARE_RULES は結合前に実行する。
// RULES は word_accent を持つ規則を結合前に、読みだけを変える規則を結合後に実行する。
const KATA_SUFFIX: NodeSpec = NodeSpec::new(
    "方",
    POS::Meishi(Meishi::Setsubi(Setsubi::Special)),
    "カタ",
    "カタ",
    1,
    Some(AccentType::C4),
);

const 従の接頭辞: NodeSpec = NodeSpec::new(
    "従",
    POS::Settoushi(Settoushi::SuuSetsuzoku),
    "ジュ",
    "ジュ",
    1,
    Some(AccentType::P2),
);

#[rustfmt::skip]
const PREPARE_RULES: &[RewriteRule] = &[
    // 住所の市区町村に続く字はアザ。文字を数える助数詞のジとは区別する。
    RewriteRule {
        surface: "字",
        stage: Stage::Lexical,
        context: prepare::住所の字,
        replacement: &[
            NodeSpec::new("字", POS::Meishi(Meishi::General), "アザ", "アザ", 1, Some(AccentType::C3)),
        ],
    },
    // 人の所在を表す「担当の方がいらっしゃる」と、世話になった相手の「方」は敬称。
    // 比較の「お世話になった方がいい」は含めず、名詞の品詞と核を結合前に揃える。
    RewriteRule {
        surface: "方",
        stage: Stage::Lexical,
        context: prepare::敬称の方,
        replacement: &[
            NodeSpec::new("方", POS::Meishi(Meishi::General), "カタ", "カタ", 2, Some(AccentType::C3)),
        ],
    },
    // 話や台詞の間を取る・置くはマ。話の間に移動するなど、時間の範囲とは区別する。
    RewriteRule {
        surface: "間",
        stage: Stage::Lexical,
        context: prepare::話の間,
        replacement: &[
            NodeSpec::new("間", POS::Meishi(Meishi::General), "マ", "マ", 0, Some(AccentType::C2)),
        ],
    },
    // 文末で動詞の連用形に続く候は、丁寧の補助動詞。時候を表す名詞のコウは残す。
    RewriteRule {
        surface: "候",
        stage: Stage::Lexical,
        context: prepare::候,
        replacement: &[
            NodeSpec::new("候", POS::Doushi(Doushi::Hijiritsu), "ソウロウ", "ソーロー", 3, None)
                .conjugation(CType::Five(Five::WaDouble), CForm::Basic, "候う"),
        ],
    },
    // 植物の根本は根元と同じネモト。根本的・根本原因などの複合語は変更しない。
    RewriteRule {
        surface: "根本",
        stage: Stage::Lexical,
        context: prepare::根本,
        replacement: &[
            NodeSpec::new("根本", POS::Meishi(Meishi::General), "ネモト", "ネモト", 3, Some(AccentType::C2)),
        ],
    },
    // カムリも辞書にあるが、恩情や被害を受ける連用形にはコウムリを使う。
    RewriteRule {
        surface: "被り",
        stage: Stage::Lexical,
        context: prepare::被り,
        replacement: &[
            NodeSpec::new("被り", POS::Doushi(Doushi::Jiritsu), "コウムリ", "コームリ", 3, None)
                .conjugation(CType::Five(Five::Ra), CForm::Renyou, "被る"),
        ],
    },
    // 鬼の居ぬ間は慣用句。否定節に続く間を一律にマへ変えることはしない。
    RewriteRule {
        surface: "間",
        stage: Stage::Lexical,
        context: prepare::鬼の居ぬ間,
        replacement: &[
            NodeSpec::new("間", POS::Meishi(Meishi::General), "マ", "マ", 0, Some(AccentType::C2)),
        ],
    },
    // 長さの単位は数詞に続く。単独の丈や、人名の丈夫には適用しない。
    RewriteRule {
        surface: "丈",
        stage: Stage::Lexical,
        context: prepare::丈,
        replacement: &[
            NodeSpec::new("丈", POS::Meishi(Meishi::Setsubi(Setsubi::Josuushi)), "ジョウ", "ジョー", 1, Some(AccentType::C3)),
        ],
    },
    // 部数を示す動詞かサ変動詞に続く五部は、数詞と助数詞へ分ける。会社名は変更しない。
    RewriteRule {
        surface: "五部",
        stage: Stage::Segmentation,
        context: prepare::五部,
        replacement: &[
            NodeSpec::new("五", POS::Meishi(Meishi::Kazu), "ゴ", "ゴ", 1, Some(AccentType::C3)),
            NodeSpec::new("部", POS::Meishi(Meishi::Setsubi(Setsubi::Josuushi)), "ブ", "ブ", 1, Some(AccentType::C3)),
        ],
    },
    // 十重に囲む・重なるはトエ。十重奏と、層数を数える十重は変更しない。
    RewriteRule {
        surface: "十",
        stage: Stage::Segmentation,
        context: prepare::十重,
        replacement: &[
            NodeSpec::new("十重", POS::Meishi(Meishi::General), "トエ", "トエ", 1, Some(AccentType::C3)),
        ],
    },
    // 位階の従はジュ。三位には連声したサンミを使い、順位のサンイと区別する。
    RewriteRule {
        surface: "従",
        stage: Stage::Segmentation,
        context: prepare::従三位,
        replacement: &[
            従の接頭辞,
            NodeSpec::new("三位", POS::Meishi(Meishi::General), "サンミ", "サンミ", 1, Some(AccentType::C1)),
        ],
    },
    RewriteRule {
        surface: "従",
        stage: Stage::Segmentation,
        context: prepare::従,
        replacement: &[従の接頭辞],
    },
    // 位階の四・七はシ・シチ。助数詞の処理がヨン・ナナへ戻さないよう、位までまとめる。
    RewriteRule {
        surface: "四",
        stage: Stage::Segmentation,
        context: prepare::位階の数詞,
        replacement: &[
            NodeSpec::new("四位", POS::Meishi(Meishi::General), "シイ", "シイ", 1, Some(AccentType::C1)),
        ],
    },
    RewriteRule {
        surface: "七",
        stage: Stage::Segmentation,
        context: prepare::位階の数詞,
        replacement: &[
            NodeSpec::new("七位", POS::Meishi(Meishi::General), "シチイ", "シチイ", 2, Some(AccentType::C1)),
        ],
    },
    // 金額の前置きはキン。円まで数の表記が続く場合に限り、助詞や読点を越えない。
    RewriteRule {
        surface: "金",
        stage: Stage::Lexical,
        context: prepare::金額の金,
        replacement: &[
            NodeSpec::new("金", POS::Settoushi(Settoushi::SuuSetsuzoku), "キン", "キン", 1, Some(AccentType::P2)),
        ],
    },
    // 地理上の大字はオオアザ。正書法と発音を分けて持つ。
    RewriteRule {
        surface: "大字",
        stage: Stage::Lexical,
        context: prepare::大字,
        replacement: &[
            NodeSpec::new("大字", POS::Meishi(Meishi::General), "オオアザ", "オーアザ", 1, Some(AccentType::C1)),
        ],
    },
    // 五分前・五分後は時間を表す。割合の五分五分や五分の入りは変更しない。
    RewriteRule {
        surface: "五分",
        stage: Stage::Lexical,
        context: prepare::五分,
        replacement: &[
            NodeSpec::new("五分", POS::Meishi(Meishi::FukushiKanou), "ゴフン", "ゴフン", 1, Some(AccentType::C1)),
        ],
    },

    // 断ると断つでは活用型が違うため、読みだけでなく原形と活用型も結合前に直す。
    RewriteRule {
        surface: "断っ",
        stage: Stage::Lexical,
        context: prepare::断つ,
        replacement: &[
            NodeSpec::new("断っ", POS::Doushi(Doushi::Jiritsu), "タッ", "タッ", 1, None)
                .conjugation(CType::Five(Five::Ta), CForm::RenyouConjunctionTa, "断つ"),
        ],
    },
    // 国名として解析された仏は、教え・慈悲・加護の用法では一般名詞に戻す。
    RewriteRule {
        surface: "仏",
        stage: Stage::Lexical,
        context: prepare::仏,
        replacement: &[
            NodeSpec::new("仏", POS::Meishi(Meishi::General), "ホトケ", "ホトケ", 0, Some(AccentType::C2)),
        ],
    },
    // 十三駅の先・十三駅前は地名にもなる。駅に直結する距離・序数か、
    // 数量の提示があるときだけ、地名の十三を数詞へ分ける。
    RewriteRule {
        surface: "十三",
        stage: Stage::Segmentation,
        context: prepare::station_count,
        replacement: &[
            NodeSpec::new("十", POS::Meishi(Meishi::Kazu), "ジュウ", "ジュー", 1, Some(AccentType::C3)),
            NodeSpec::new("三", POS::Meishi(Meishi::Kazu), "サン", "サン", 0, Some(AccentType::C3)),
        ],
    },
    // 数詞のまま核を計算してから読みを変えると、駅名の結合核にならない。
    // 鉄道名・目的地・乗降の手がかりを確認し、数詞処理前に地名へまとめる。
    RewriteRule {
        surface: "十",
        stage: Stage::Segmentation,
        context: prepare::station_name,
        replacement: &[
            NodeSpec::new("十三", POS::Meishi(Meishi::KoyuMeishi(KoyuMeishi::Region(Region::General))),
                "ジュウソウ", "ジューソー", 0, Some(AccentType::C2)),
        ],
    },
    // 方法の接尾辞は連用形に付く。歪み・転び・捻れには名詞の候補もあるため、
    // ですの有無でホウへ戻らないよう、動詞と接尾辞の結合を先に決める。
    RewriteRule {
        surface: "歪み",
        stage: Stage::Lexical,
        context: |nodes, i| prepare::nominal_kata_stem(nodes, i, "ユガミ"),
        replacement: &[
            NodeSpec::new("歪み", POS::Doushi(Doushi::Jiritsu), "ユガミ", "ユガミ", 0, None)
                .conjugation(CType::Five(Five::Ma), CForm::Renyou, "歪む"),
            KATA_SUFFIX,
        ],
    },
    RewriteRule {
        surface: "転び",
        stage: Stage::Lexical,
        context: |nodes, i| prepare::nominal_kata_stem(nodes, i, "コロビ"),
        replacement: &[
            NodeSpec::new("転び", POS::Doushi(Doushi::Jiritsu), "コロビ", "コロビ", 0, None)
                .conjugation(CType::Five(Five::Ba), CForm::Renyou, "転ぶ"),
            KATA_SUFFIX,
        ],
    },
    RewriteRule {
        surface: "捻れ",
        stage: Stage::Lexical,
        context: |nodes, i| prepare::nominal_kata_stem(nodes, i, "ネジレ"),
        replacement: &[
            NodeSpec::new("捻れ", POS::Doushi(Doushi::Jiritsu), "ネジレ", "ネジレ", 3, None)
                .conjugation(CType::One(One::None), CForm::Renyou, "捻れる"),
            KATA_SUFFIX,
        ],
    },
    RewriteRule {
        surface: "方",
        stage: Stage::Lexical,
        context: prepare::verb_kata_suffix,
        replacement: &[KATA_SUFFIX],
    },
];

/// 文脈判定に使う文字列と型付きの品詞・活用形。
trait ContextWord {
    fn surface(&self) -> &str;
    fn orig(&self) -> &str;
    fn pos(&self) -> Option<POS>;
    fn cform(&self) -> Option<CForm>;

    fn pos_matches(&self, predicate: PosPredicate) -> bool {
        self.pos().is_some_and(predicate)
    }
}

impl ContextWord for NJDNode {
    fn surface(&self) -> &str {
        self.get_string()
    }

    fn orig(&self) -> &str {
        self.get_orig().unwrap_or(self.get_string())
    }

    fn pos(&self) -> Option<POS> {
        Some(*self.get_pos())
    }

    fn cform(&self) -> Option<CForm> {
        Some(*self.get_cform())
    }
}

impl ContextWord for NjdFeature {
    fn surface(&self) -> &str {
        &self.string
    }

    fn orig(&self) -> &str {
        &self.orig
    }

    fn pos(&self) -> Option<POS> {
        // 表層形で規則が見つかり、品詞を条件にする場合だけ解析する。
        POS::from_strs(
            &self.pos,
            &self.pos_group1,
            &self.pos_group2,
            &self.pos_group3,
        )
        .map_err(|error| log::warn!("文脈読みの品詞を解析できません: {}: {error}", self.string))
        .ok()
    }

    fn cform(&self) -> Option<CForm> {
        self.cform
            .parse()
            .map_err(|error| {
                log::warn!("文脈読みの活用形を解析できません: {}: {error}", self.string)
            })
            .ok()
    }
}

type PosPredicate = fn(POS) -> bool;

fn is_suffix(pos: POS) -> bool {
    matches!(
        pos,
        POS::Meishi(Meishi::Setsubi(_))
            | POS::Doushi(Doushi::Setsubi)
            | POS::Keiyoushi(Keiyoushi::Setsubi)
    )
}

fn is_general(pos: POS) -> bool {
    matches!(
        pos,
        POS::Meishi(Meishi::General) | POS::Fukushi(Fukushi::General) | POS::Kigou(Kigou::General)
    )
}

fn is_independent(pos: POS) -> bool {
    matches!(
        pos,
        POS::Doushi(Doushi::Jiritsu) | POS::Keiyoushi(Keiyoushi::Jiritsu)
    )
}

/// 規則が発火する条件。
enum Cue {
    /// 対象の原形が、いずれかに一致する。
    OrigIn(&'static [&'static str]),
    /// 「何にも」に続く述語が、ナンニモと読む打ち消しの用法である。
    NegativeNannimo,
    /// 「外ならぬ」に続く語が、人や特別な事情を指す。
    外ならぬ,
    /// 「外でもない」が用件を切り出す慣用表現である。
    外でもない,
    /// 食品だけを主語にした述語で、負担を表す修飾語を伴わない。
    辛味を表す,
    /// 文や引用の冒頭で、日付を導入する。
    日付を導く来る,
    /// 直後の形態素の表層形が、いずれかに一致する。
    NextIn(&'static [&'static str]),

    /// 直後から並ぶ形態素の表層形が、指定した列と一致する。
    NextSequence(&'static [&'static str]),

    /// 直前の形態素の表層形が、いずれかに一致する。
    PrevIn(&'static [&'static str]),

    /// 直前が「の」で、その 1 つ前の形態素の表層形がいずれかに一致する。
    ///
    /// 「〜の下」のように連体助詞を挟む構造のために用意している。
    PrevViaNo(&'static [&'static str]),

    /// 指定した助詞を挟む前の語が、候補に含まれる。
    /// 「完全に」「ぴたりと」などの副詞を4形態素まで飛ばす。
    /// 他の助詞・述語・記号は越えない。
    PrevViaParticle(&'static [&'static str], &'static [&'static str]),

    /// 対象の形態素の活用形が、いずれかに一致する。
    CformIn(&'static [CForm]),

    /// 指定した語を 1 つ飛ばして遡った形態素の品詞が、条件を満たす。
    ///
    /// 例えば、「そう思うより外にない」の `思う` を見るためにあり、
    /// `より` が挟まるので [`Cue::PrevPos`] では前の形態素を見られない。
    PrevPosVia(&'static [&'static str], PosPredicate),

    /// この形態素より後ろ、文が終わるまでに打ち消しの語がある。
    ///
    /// 「〜より外に〜ない」のように、打ち消しと組でなければ成り立たない読みがある。
    /// [`NEGATIVE_ORIGS`] の原形を持つ形態素を探し、句点に当たったところで止める。
    Negated,

    /// 直前の形態素の品詞が、条件を満たす。
    ///
    /// 連濁は「直前に語が直接くっついているか」で決まるので、表層形を列挙する
    /// のではなく品詞で見る。助詞を挟む場合は複合語ではないので発火しない。
    PrevPos(PosPredicate),

    /// 直前の形態素の活用形が、いずれかに一致する。
    PrevCform(&'static [CForm]),

    /// 直後の形態素の品詞が、条件を満たす。
    NextPos(PosPredicate),

    /// 直後の形態素の表層形が前者のいずれかで、その 1 つ後の形態素の原形が
    /// 後者のいずれかに一致する。
    ///
    /// 「身体を鍛える」のように、助詞の先の述語で読みが決まるときに使う。
    /// 活用する語は表層形が変わるので、原形で照合する。
    NextViaOrig(&'static [&'static str], &'static [&'static str]),

    /// 直前の形態素の表層形が、指定した文字数以上である。
    PrevMinChars(usize),

    /// 指定した条件を満たさない。
    Not(&'static Cue),

    /// 並べた条件をすべて満たす。品詞と品詞細分類のように、片方だけでは
    /// 絞りきれない手がかりを組み合わせるのに使う。
    All(&'static [Cue]),
}

struct Rule {
    /// 対象の形態素の表層形。
    surface: &'static str,
    /// 対象の形態素の品詞条件。`None` なら問わない。
    pos: Option<PosPredicate>,
    cue: Cue,
    /// 与える読み。読みと発音が異なる語が出てきたら分けること。
    reading: &'static str,
    /// 単語の核を指定した規則は、句の結合前に読みと核を確定する。
    word_accent: Option<usize>,
}

const KABURU_OBJECTS: &[&str] = &[
    "帽子",
    "冠",
    "王冠",
    "ヘルメット",
    "頭巾",
    "兜",
    "笠",
    "仮面",
];

const オビヤカスの目的語: &[&str] = &[
    "存続", "安全", "地位", "生命", "生存", "平和", "権利", "生活", "健康",
];

/// 語ごとの規則。上から順に評価し、最初に一致したものを適用する。
///
/// 各規則の根拠と、確認した負の対照をコメントに残す。
#[rustfmt::skip]
const RULES: &[Rule] = &[
    // 「三食カレーは辛い」は負担を表すため、食品名の前後も確認する。
    // 食品名だけの主語と程度の副詞に限り、連体用法や「さすがに」は含めない。
    Rule {
        surface: "辛い",
        pos: Some(|pos| matches!(pos, POS::Keiyoushi(Keiyoushi::Jiritsu))),
        cue: Cue::辛味を表す,
        reading: "カライ",
        word_accent: Some(2),
    },
    // 日付を導入する「来る金曜日」はキタル。「彼が来る金曜日」はクルのまま。
    Rule {
        surface: "来る",
        pos: Some(is_independent),
        cue: Cue::All(&[Cue::CformIn(&[CForm::Basic]), Cue::日付を導く来る]),
        reading: "キタル",
        word_accent: Some(0),
    },
    // 存続や安全を危うくする用法はオビヤカス。人を驚かせるオドカスとは区別する。
    Rule {
        surface: "脅かす",
        pos: Some(is_independent),
        cue: Cue::PrevViaParticle(&["を"], オビヤカスの目的語),
        reading: "オビヤカス",
        word_accent: Some(4),
    },
    Rule {
        surface: "脅かし",
        pos: Some(is_independent),
        cue: Cue::PrevViaParticle(&["を"], オビヤカスの目的語),
        reading: "オビヤカシ",
        word_accent: Some(4),
    },
    Rule {
        surface: "脅かさ",
        pos: Some(is_independent),
        cue: Cue::PrevViaParticle(&["を"], オビヤカスの目的語),
        reading: "オビヤカサ",
        word_accent: Some(4),
    },
    Rule {
        surface: "脅かせ",
        pos: Some(is_independent),
        cue: Cue::PrevViaParticle(&["を"], オビヤカスの目的語),
        reading: "オビヤカセ",
        word_accent: Some(4),
    },
    Rule {
        surface: "脅かそ",
        pos: Some(is_independent),
        cue: Cue::PrevViaParticle(&["を"], オビヤカスの目的語),
        reading: "オビヤカソ",
        word_accent: Some(4),
    },
    // 数学上の量はアタイ。株や商品の値段を表すネとは区別する。
    Rule {
        surface: "値",
        pos: Some(|pos| matches!(pos, POS::Meishi(_))),
        cue: Cue::PrevViaNo(&["変数", "関数", "定数", "パラメータ", "係数"]),
        reading: "アタイ",
        word_accent: Some(0),
    },
    // 頭に着ける物を直接の目的語に取る被るはカブル。被害や恩恵のコウムルを残す。
    Rule {
        surface: "被る",
        pos: Some(is_independent),
        cue: Cue::All(&[Cue::OrigIn(&["被る"]), Cue::PrevViaParticle(&["を"], KABURU_OBJECTS)]),
        reading: "カブル",
        word_accent: Some(2),
    },
    Rule {
        surface: "被っ",
        pos: Some(is_independent),
        cue: Cue::All(&[Cue::OrigIn(&["被る"]), Cue::PrevViaParticle(&["を"], KABURU_OBJECTS)]),
        reading: "カブッ",
        word_accent: Some(2),
    },
    Rule {
        surface: "被り",
        pos: Some(is_independent),
        cue: Cue::All(&[Cue::OrigIn(&["被る"]), Cue::PrevViaParticle(&["を"], KABURU_OBJECTS)]),
        reading: "カブリ",
        word_accent: Some(2),
    },
    Rule {
        surface: "被ら",
        pos: Some(is_independent),
        cue: Cue::All(&[Cue::OrigIn(&["被る"]), Cue::PrevViaParticle(&["を"], KABURU_OBJECTS)]),
        reading: "カブラ",
        word_accent: Some(2),
    },
    Rule {
        surface: "被れ",
        pos: Some(is_independent),
        cue: Cue::All(&[Cue::OrigIn(&["被る"]), Cue::PrevViaParticle(&["を"], KABURU_OBJECTS)]),
        reading: "カブレ",
        word_accent: Some(2),
    },
    Rule {
        surface: "被ろ",
        pos: Some(is_independent),
        cue: Cue::All(&[Cue::OrigIn(&["被る"]), Cue::PrevViaParticle(&["を"], KABURU_OBJECTS)]),
        reading: "カブロ",
        word_accent: Some(2),
    },
    // 全能の主は宗教上のシュ、将棋の角は駒のカクなので、修飾語を確認する。
    Rule {
        surface: "主",
        pos: Some(|pos| matches!(pos, POS::Meishi(_))),
        cue: Cue::PrevViaNo(&["全能"]),
        reading: "シュ",
        word_accent: Some(1),
    },
    Rule {
        surface: "角",
        pos: Some(|pos| matches!(pos, POS::Meishi(_))),
        cue: Cue::PrevViaNo(&["将棋"]),
        reading: "カク",
        word_accent: Some(1),
    },
    // 骨董市はイチ、幸あれと笑う門は固定した言い方の読みを使う。
    Rule {
        surface: "市",
        pos: Some(|pos| matches!(pos, POS::Meishi(_))),
        cue: Cue::PrevIn(&["骨董", "骨とう"]),
        reading: "イチ",
        word_accent: Some(1),
    },
    Rule {
        surface: "幸",
        pos: Some(|pos| matches!(pos, POS::Meishi(_))),
        cue: Cue::NextIn(&["あれ"]),
        reading: "サチ",
        word_accent: Some(1),
    },
    Rule {
        surface: "門",
        pos: Some(|pos| matches!(pos, POS::Meishi(_))),
        cue: Cue::PrevIn(&["笑う"]),
        reading: "カド",
        word_accent: Some(1),
    },
    // 親の敵を討つ用法はカタキ。敵と対立するだけの用法は含めない。
    Rule {
        surface: "敵",
        pos: Some(|pos| matches!(pos, POS::Meishi(_))),
        cue: Cue::All(&[Cue::PrevViaNo(&["親", "父", "母", "兄", "弟", "姉", "妹", "夫", "妻", "師"]), Cue::NextViaOrig(&["を"], &["討つ", "討ち取る"])]),
        reading: "カタキ",
        word_accent: Some(3),
    },
    // 二歩の反則はニフ。歩数や地名の読みは変えない。
    Rule {
        surface: "二歩",
        pos: Some(|pos| matches!(pos, POS::Meishi(_))),
        cue: Cue::NextViaOrig(&["の"], &["反則"]),
        reading: "ニフ",
        word_accent: Some(0),
    },
    Rule {
        surface: "二歩",
        pos: Some(|pos| matches!(pos, POS::Meishi(_))),
        cue: Cue::NextSequence(&["は", "将棋", "の", "反則"]),
        reading: "ニフ",
        word_accent: Some(0),
    },
    // 意見を擦り合わせる用法はスリアワセル。手や物を擦る用法は対象にしない。
    Rule {
        surface: "擦り合わせる",
        pos: Some(|pos| matches!(pos, POS::Doushi(_))),
        cue: Cue::PrevViaParticle(&["を"], &["意見", "認識", "考え", "条件", "日程", "計画", "予定", "仕様"]),
        reading: "スリアワセル",
        word_accent: Some(5),
    },
    Rule {
        surface: "擦り合わせ",
        pos: Some(|pos| matches!(pos, POS::Doushi(_))),
        cue: Cue::All(&[Cue::CformIn(&[CForm::Mizen, CForm::Renyou]), Cue::PrevViaParticle(&["を"], &["意見", "認識", "考え", "条件", "日程", "計画", "予定", "仕様"])]),
        reading: "スリアワセ",
        word_accent: Some(0),
    },
    // 車が留まる用法と枝に留まる用法はトマル。会社に留まる用法は含めない。
    Rule {
        surface: "留まっ",
        pos: Some(|pos| matches!(pos, POS::Doushi(_))),
        cue: Cue::PrevViaParticle(&["が", "は"], &["車", "列車", "電車", "小鳥", "鳥"]),
        reading: "トマッ",
        word_accent: Some(0),
    },
    Rule {
        surface: "留まっ",
        pos: Some(|pos| matches!(pos, POS::Doushi(_))),
        cue: Cue::PrevViaParticle(&["に"], &["枝", "止まり木", "電線"]),
        reading: "トマッ",
        word_accent: Some(0),
    },

    // 交通の便はベン、郵便や航空便はビンなので、名詞が「の」を挟む形に限る。
    Rule {
        surface: "便",
        pos: Some(|pos| matches!(pos, POS::Meishi(_))),
        cue: Cue::PrevViaNo(&["交通", "通勤", "通学", "通行"]),
        reading: "ベン",
        word_accent: Some(1),
    },
    // 「供をする」は随行を表す。供物や供養の読みは変更しない。
    Rule {
        surface: "供",
        pos: Some(|pos| matches!(pos, POS::Meishi(_))),
        cue: Cue::NextViaOrig(&["を"], &["する"]),
        reading: "トモ",
        word_accent: Some(1),
    },
    // 池や湖の主はヌシ、家の主はアルジなので、直前の修飾語を確認する。
    Rule {
        surface: "主",
        pos: Some(|pos| matches!(pos, POS::Meishi(_))),
        cue: Cue::PrevViaNo(&["池", "湖", "沼"]),
        reading: "ヌシ",
        word_accent: Some(1),
    },
    // 動物の角はツノと読む。建物の角や角の駒は候補に含めない。
    Rule {
        surface: "角",
        pos: Some(|pos| matches!(pos, POS::Meishi(_))),
        cue: Cue::PrevViaNo(&["鹿", "牛", "羊", "山羊", "ヤギ", "犀", "サイ"]),
        reading: "ツノ",
        word_accent: Some(2),
    },
    // 草薙の剣と奉納の剣の舞はツルギと読む。剣道や刀剣のケンは変更しない。
    Rule {
        surface: "剣",
        pos: Some(|pos| matches!(pos, POS::Meishi(_))),
        cue: Cue::PrevViaNo(&["草薙", "天叢雲"]),
        reading: "ツルギ",
        word_accent: Some(3),
    },
    Rule {
        surface: "剣",
        pos: Some(|pos| matches!(pos, POS::Meishi(_))),
        cue: Cue::All(&[Cue::PrevViaNo(&["奉納"]), Cue::NextViaOrig(&["の"], &["舞"])]),
        reading: "ツルギ",
        word_accent: Some(3),
    },
    // 「木の囲い」は複合語ではないため、雪囲いの連濁形を使わない。
    Rule {
        surface: "囲い",
        pos: Some(|pos| matches!(pos, POS::Meishi(_))),
        cue: Cue::PrevIn(&["の"]),
        reading: "カコイ",
        word_accent: Some(0),
    },
    // 「この期に及ぶ」は固定表現であり、会計の期や時期のキとは区別する。
    Rule {
        surface: "期",
        pos: Some(|pos| matches!(pos, POS::Meishi(_))),
        cue: Cue::All(&[Cue::PrevIn(&["この"]), Cue::NextViaOrig(&["に"], &["及ぶ"])]),
        reading: "ゴ",
        word_accent: Some(1),
    },
    // 病気で床を離れる用法はトコ。床を磨く用法や、宙に浮く描写は含めない。
    Rule {
        surface: "床",
        pos: Some(|pos| matches!(pos, POS::Meishi(_))),
        cue: Cue::All(&[Cue::PrevViaParticle(&["で"], &["病気", "病", "風邪"]), Cue::NextViaOrig(&["を"], &["離れる"])]),
        reading: "トコ",
        word_accent: Some(0),
    },
    // 文学や芸術の類はタグイ。分類の大項目を示す数字には適用しない。
    Rule {
        surface: "類",
        pos: Some(|pos| matches!(pos, POS::Meishi(_))),
        cue: Cue::PrevViaNo(&["文学", "芸術", "音楽", "小説"]),
        reading: "タグイ",
        word_accent: Some(0),
    },
    // 「遥か上」は位置を表す。上巻や上流などの漢語は対象にならない。
    Rule {
        surface: "上",
        pos: Some(|pos| matches!(pos, POS::Meishi(_))),
        cue: Cue::PrevIn(&["遥か", "はるか"]),
        reading: "ウエ",
        word_accent: Some(0),
    },
    // たばこの火口はホクチ。火山の火口には適用しない。
    Rule {
        surface: "火口",
        pos: Some(|pos| matches!(pos, POS::Meishi(_))),
        cue: Cue::PrevViaNo(&["たばこ", "タバコ", "煙草"]),
        reading: "ホクチ",
        word_accent: Some(0),
    },
    // 抽象的な対象はイダク、腕に抱える対象はダク。助詞「を」と目的語を確認する。
    Rule {
        surface: "抱く",
        pos: Some(|pos| matches!(pos, POS::Doushi(_))),
        cue: Cue::PrevViaParticle(&["を"], &["期待", "不安", "夢", "希望", "疑念", "疑問", "疑惑", "理想", "野心", "大志", "憧れ", "恐怖", "懸念", "敵意", "好意", "敬意", "思い"]),
        reading: "イダク",
        word_accent: Some(2),
    },
    Rule {
        surface: "抱く",
        pos: Some(|pos| matches!(pos, POS::Doushi(_))),
        cue: Cue::PrevViaParticle(&["を"], &["赤ちゃん", "赤ん坊", "幼子", "乳児", "子供", "子ども", "息子", "娘", "恋人", "猫", "犬", "子犬"]),
        reading: "ダク",
        word_accent: Some(0),
    },
    Rule {
        surface: "抱い",
        pos: Some(|pos| matches!(pos, POS::Doushi(_))),
        cue: Cue::PrevViaParticle(&["を"], &["期待", "不安", "夢", "希望", "疑念", "疑問", "疑惑", "理想", "野心", "大志", "憧れ", "恐怖", "懸念", "敵意", "好意", "敬意", "思い"]),
        reading: "イダイ",
        word_accent: Some(2),
    },
    Rule {
        surface: "抱い",
        pos: Some(|pos| matches!(pos, POS::Doushi(_))),
        cue: Cue::PrevViaParticle(&["を"], &["赤ちゃん", "赤ん坊", "幼子", "乳児", "子供", "子ども", "息子", "娘", "恋人", "猫", "犬", "子犬"]),
        reading: "ダイ",
        word_accent: Some(0),
    },
    Rule {
        surface: "抱か",
        pos: Some(|pos| matches!(pos, POS::Doushi(_))),
        cue: Cue::PrevViaParticle(&["を"], &["期待", "不安", "夢", "希望", "疑念", "疑問", "疑惑", "理想", "野心", "大志", "憧れ", "恐怖", "懸念", "敵意", "好意", "敬意", "思い"]),
        reading: "イダカ",
        word_accent: Some(2),
    },
    Rule {
        surface: "抱か",
        pos: Some(|pos| matches!(pos, POS::Doushi(_))),
        cue: Cue::PrevViaParticle(&["を"], &["赤ちゃん", "赤ん坊", "幼子", "乳児", "子供", "子ども", "息子", "娘", "恋人", "猫", "犬", "子犬"]),
        reading: "ダカ",
        word_accent: Some(0),
    },
    Rule {
        surface: "抱き",
        pos: Some(|pos| matches!(pos, POS::Doushi(_))),
        cue: Cue::PrevViaParticle(&["を"], &["期待", "不安", "夢", "希望", "疑念", "疑問", "疑惑", "理想", "野心", "大志", "憧れ", "恐怖", "懸念", "敵意", "好意", "敬意", "思い"]),
        reading: "イダキ",
        word_accent: Some(2),
    },
    Rule {
        surface: "抱き",
        pos: Some(|pos| matches!(pos, POS::Doushi(_))),
        cue: Cue::PrevViaParticle(&["を"], &["赤ちゃん", "赤ん坊", "幼子", "乳児", "子供", "子ども", "息子", "娘", "恋人", "猫", "犬", "子犬"]),
        reading: "ダキ",
        word_accent: Some(0),
    },
    // 肩を怒らせる用法はイカラセル。人を怒らせる用法は対象にしない。
    Rule {
        surface: "怒ら",
        pos: Some(|pos| matches!(pos, POS::Doushi(_))),
        cue: Cue::PrevViaParticle(&["を"], &["肩"]),
        reading: "イカラ",
        word_accent: Some(2),
    },
    // 評判や名誉を汚す用法はケガス。衣服や床を汚す用法は対象にしない。
    Rule {
        surface: "汚し",
        pos: Some(|pos| matches!(pos, POS::Doushi(_))),
        cue: Cue::PrevViaParticle(&["を"], &["評判", "名誉", "名声", "家名"]),
        reading: "ケガシ",
        word_accent: Some(2),
    },
    // 穴が開く用法はアク。扉や本が開く用法には、読みを決める条件が足りない。
    Rule {
        surface: "開い",
        pos: Some(|pos| matches!(pos, POS::Doushi(_))),
        cue: Cue::PrevViaParticle(&["が"], &["穴", "鍵"]),
        reading: "アイ",
        word_accent: Some(0),
    },
    Rule {
        surface: "開く",
        pos: Some(|pos| matches!(pos, POS::Doushi(_))),
        cue: Cue::PrevViaParticle(&["が"], &["穴", "鍵"]),
        reading: "アク",
        word_accent: Some(0),
    },
    // 視界や運が開ける用法はヒラケル。窓を開ける用法と、命令形は対象にしない。
    Rule {
        surface: "開け",
        pos: Some(|pos| matches!(pos, POS::Doushi(_))),
        cue: Cue::All(&[Cue::CformIn(&[CForm::Mizen, CForm::Renyou]), Cue::PrevViaParticle(&["が", "は"], &["目の前", "視界", "景色", "視野", "展望", "運", "運勢", "道", "地域", "町", "街", "村"])]),
        reading: "ヒラケ",
        word_accent: Some(3),
    },
    // 「一番好き」は最上級であり、「旅好き」のような名詞との複合語ではない。
    // ズキのコストを変えると旅好きも清音化するため、一番に続く接尾辞だけを直す。
    Rule {
        surface: "好き",
        pos: Some(is_suffix),
        cue: Cue::PrevIn(&["一番"]),
        reading: "スキ",
        word_accent: None,
    },
    // 「何にも知らない」はナンニモ、「何にも似ていない」はナニニモと読む。
    // 辞書で副詞を優先すると格助詞の用法も変わるため、述語と打ち消しを確認する。
    // https://github.com/tsukumijima/pyopenjtalk-plus/commit/17aa989573807971094af00fad6a22227ba90c82
    Rule {
        surface: "何",
        pos: None,
        cue: Cue::NegativeNannimo,
        reading: "ナン",
        word_accent: None,
    },
    // 「一見さん」の 一見 は イチゲン と読む。(初めての客のこと)
    // 負の対照: 「一見して分かる」は直後が「し」なので発火しない。
    // [0 / 0]
    Rule {
        surface: "一見",
        pos: None,
        cue: Cue::NextIn(&["さん"]),
        reading: "イチゲン",
        word_accent: None,
    },
    // 「一声かける」= ヒトコエかける。
    // 負の対照: 「一声も出ない」は直後が「も」なので発火しない。
    // [0 / 0]
    Rule {
        surface: "一声",
        pos: None,
        cue: Cue::NextIn(&["かけ", "掛け", "かける", "掛ける"]),
        reading: "ヒトコエ",
        word_accent: None,
    },
    // 「一行ごと」は詩や文章の行を単位とする言い方なので イチギョウ。
    // 一行 (イッコウ = 同行者の集団) は「ごと」を伴わない。
    // 負の対照: 「一行が到着した」は直後が「が」なので発火しない。
    // [0 / 0]
    Rule {
        surface: "一行",
        pos: None,
        cue: Cue::NextIn(&["ごと", "毎"]),
        reading: "イチギョウ",
        word_accent: None,
    },
    // 「兵ども」= ツワモノども。
    // 負の対照: 「兵の数」は直後が「の」なので発火しない。
    // [1 / 0]
    Rule {
        surface: "兵",
        pos: None,
        cue: Cue::NextIn(&["ども", "共"]),
        reading: "ツワモノ",
        word_accent: None,
    },
    // 人を指す名詞に付く接尾辞「方」は複数の敬称なので ガタ。
    // 「読み方」「夕方」のように 1 形態素として解析される語は対象外になる。
    // 「この方」の「方」は接尾辞ではないので発火しない。
    // [19 / 0]
    Rule {
        surface: "方",
        pos: Some(is_suffix),
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
        word_accent: None,
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
        pos: Some(is_general),
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
        word_accent: None,
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
        pos: Some(|pos| matches!(pos, POS::Meishi(Meishi::SahenSetsuzoku))),
        cue: Cue::PrevPos(|pos| matches!(pos, POS::Meishi(_))),
        reading: "ブソク",
        word_accent: None,
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
        pos: Some(is_suffix),
        cue: Cue::PrevPos(|pos| matches!(pos, POS::Meishi(_))),
        reading: "ヤキ",
        word_accent: None,
    },
    // 「〜峡」は渓谷の名である。 (匹見峡 / 寒霞渓) カイ は単独の訓読み。
    // [0 / 0]
    Rule {
        surface: "峡",
        pos: Some(is_general),
        cue: Cue::PrevPos(|pos| matches!(pos, POS::Meishi(_))),
        reading: "キョー",
        word_accent: None,
    },
    // 「分屯」「駐屯」の 屯 は トン。タムロ は「屯する」の訓読み。
    // [0 / 0]
    Rule {
        surface: "屯",
        pos: Some(|pos| matches!(pos, POS::Meishi(Meishi::SahenSetsuzoku))),
        cue: Cue::PrevPos(|pos| matches!(pos, POS::Meishi(_))),
        reading: "トン",
        word_accent: None,
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
        pos: Some(is_general),
        cue: Cue::PrevPos(|pos| matches!(pos, POS::Meishi(_))),
        reading: "カクケー",
        word_accent: None,
    },
    // 「軍艦旗」「連隊旗」の 旗 は キ。単独の 旗 は ハタ のまま。
    // [0 / 0]
    Rule {
        surface: "旗",
        pos: Some(is_general),
        cue: Cue::PrevPos(|pos| matches!(pos, POS::Meishi(_))),
        reading: "キ",
        word_accent: None,
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
        pos: None,
        cue: Cue::All(&[
            Cue::PrevPos(|pos| matches!(pos, POS::Meishi(_))),
            Cue::PrevPos(|pos| matches!(pos, POS::Meishi(Meishi::SahenSetsuzoku))),
            Cue::NextPos(|pos| matches!(pos, POS::Joshi(_))),
        ]),
        reading: "マエ",
        word_accent: None,
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
        pos: None,
        cue: Cue::PrevIn(&[
            "紀元", "門", "生", "就学", "出生", "公判", "産", "術", "食",
            "陸", "膝蓋", "祝典", "患難", "停滞", "閉塞", "寒帯", "温暖", "暴露",
        ]),
        reading: "ゼン",
        word_accent: None,
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
        pos: None,
        cue: Cue::PrevIn(&[
            "高架", "可動", "水管", "人道", "跨道", "跨線", "連絡", "斜張", "張",
            "河口", "併用", "吊", "桁", "鉄道", "歩道", "陸", "仮設", "道路", "段",
            "アーチ", "トラス", "ラーメン", "ＰＣ",
        ]),
        reading: "キョー",
        word_accent: None,
    },
    // [24 / 0]
    Rule {
        surface: "橋",
        pos: Some(is_suffix),
        cue: Cue::PrevPos(|pos| matches!(pos, POS::Meishi(_))),
        reading: "バシ",
        word_accent: None,
    },
    // 「〜寺」は ジ 83% / デラ 11%。デラ になるのは和語が前に来るときだが
    // 開いた集合なので、観測された分だけ例外に置いて残りは ジ にする。
    // (清水寺 のように 1 エントリで登録されている寺は影響を受けない)
    // [0 / 0]
    Rule {
        surface: "寺",
        pos: None,
        cue: Cue::PrevIn(&["縁切", "猫", "だるま", "隠れ", "峯", "山", "花"]),
        reading: "デラ",
        word_accent: None,
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
        pos: None,
        cue: Cue::PrevPos(|pos| matches!(pos, POS::Meishi(Meishi::FukushiKanou) | POS::Meishi(Meishi::Hijiritsu(_)) | POS::Doushi(Doushi::Hijiritsu) | POS::Keiyoushi(Keiyoushi::Hijiritsu))),
        reading: "テラ",
        word_accent: None,
    },
    // [34 / 0]
    Rule {
        surface: "寺",
        pos: None,
        cue: Cue::PrevPos(|pos| matches!(pos, POS::Meishi(_))),
        reading: "ジ",
        word_accent: None,
    },
    // 「叙事詩環」「循環」の 環 は カン になる。単独は タマキ / ワ。
    // [6 / 3]
    Rule {
        surface: "環",
        pos: None,
        cue: Cue::PrevPos(|pos| matches!(pos, POS::Meishi(_))),
        reading: "カン",
        word_accent: None,
    },
    // 「諏訪洞」「鍾乳洞」の 洞 は ドー になる。単独は ホラ。
    // [12 / 0]
    Rule {
        surface: "洞",
        pos: None,
        cue: Cue::PrevPos(|pos| matches!(pos, POS::Meishi(_))),
        reading: "ドー",
        word_accent: None,
    },
    // 「宮沢湖」「琵琶湖」の 湖 は コ になる。単独は ミズウミ。
    // [3 / 9]
    Rule {
        surface: "湖",
        pos: None,
        cue: Cue::PrevPos(|pos| matches!(pos, POS::Meishi(_))),
        reading: "コ",
        word_accent: None,
    },
    // 「唇歯音」「小陰唇」の 唇 は シン になる。単独は クチビル。
    // [0 / 0]
    Rule {
        surface: "唇",
        pos: None,
        cue: Cue::PrevPos(|pos| matches!(pos, POS::Meishi(_))),
        reading: "シン",
        word_accent: None,
    },
    // 「風景印」「消印」の 印 は イン になる。単独は シルシ。
    // [7 / 0]
    Rule {
        surface: "印",
        pos: None,
        cue: Cue::PrevPos(|pos| matches!(pos, POS::Meishi(_))),
        reading: "イン",
        word_accent: None,
    },
    // 「筆子塚」「一里塚」の 塚 は連濁して ズカ になる。単独は ツカ。
    // [9 / 0]
    Rule {
        surface: "塚",
        pos: None,
        cue: Cue::PrevPos(|pos| matches!(pos, POS::Meishi(_))),
        reading: "ズカ",
        word_accent: None,
    },
    // 「水晶小屋」「山小屋」の 小屋 は連濁して ゴヤになる。単独は コヤ。
    // 「寺小屋」は テラコヤ で連濁しない。寺子屋 の異表記である。
    // [0 / 21]
    Rule {
        surface: "小屋",
        pos: None,
        cue: Cue::PrevIn(&["寺"]),
        reading: "コヤ",
        word_accent: None,
    },
    // [23 / 0]
    Rule {
        surface: "小屋",
        pos: None,
        cue: Cue::PrevPos(|pos| matches!(pos, POS::Meishi(_))),
        reading: "ゴヤ",
        word_accent: None,
    },
    // 「貴乃花部屋」「子供部屋」の 部屋 は連濁して ベヤ になる。単独は ヘヤ。
    // [18 / 0]
    Rule {
        surface: "部屋",
        pos: None,
        cue: Cue::All(&[
            Cue::PrevPos(|pos| matches!(pos, POS::Meishi(_))),
            Cue::Not(&Cue::PrevPos(|pos| matches!(pos, POS::Meishi(Meishi::Kazu) | POS::Kigou(Kigou::Kazu)))),
        ]),
        reading: "ベヤ",
        word_accent: None,
    },
    // 「お年玉付郵便はがき」の 付 は ツキ になる。ズケ は 日付 などの別エントリ。
    // [26 / 0]
    Rule {
        surface: "付",
        pos: Some(is_suffix),
        cue: Cue::PrevPos(|pos| matches!(pos, POS::Meishi(_))),
        reading: "ツキ",
        word_accent: None,
    },
    // 「金車」のような複合語の 金 は キン になる。単独は カネ。
    // [0 / 112]
    Rule {
        surface: "金",
        pos: Some(is_general),
        cue: Cue::PrevPos(|pos| matches!(pos, POS::Meishi(_))),
        reading: "キン",
        word_accent: None,
    },
    // 「公同書簡」「公文書」の 公 は コー になる。単独は オーヤケ。
    // 直前が副詞的な名詞なら、その「公」は複合語の後部要素ではないので オーヤケ である。
    // 「いつか公になる」がこれで、下の 名詞 の規則が コー を書き込んでいた。
    // 負の対照: 信長公・秀吉公 は 固有名詞 なので影響しない。
    // [0 / 16]
    Rule {
        surface: "公",
        pos: Some(is_general),
        cue: Cue::PrevPos(|pos| matches!(pos, POS::Meishi(Meishi::FukushiKanou))),
        reading: "オーヤケ",
        word_accent: None,
    },
    // [8 / 30]
    Rule {
        surface: "公",
        pos: Some(is_general),
        cue: Cue::PrevPos(|pos| matches!(pos, POS::Meishi(_))),
        reading: "コー",
        word_accent: None,
    },
    // 「歯茎硬口蓋音」の 硬 は コー になる。辞書には形容詞の カタ しか無い。
    // [1 / 0]
    Rule {
        surface: "硬",
        pos: None,
        cue: Cue::PrevPos(|pos| matches!(pos, POS::Meishi(_))),
        reading: "コー",
        word_accent: None,
    },
    // 「如何ですか」の 如何 は イカガ になる。「如何なる」「如何にも」は イカ。
    // [3 / 0]
    Rule {
        surface: "如何",
        pos: None,
        cue: Cue::NextIn(&["で", "です", "でし", "でしょ"]),
        reading: "イカガ",
        word_accent: None,
    },

    // 形容動詞語幹 + 「物」は ブツ と読む。(危険物・不要物・貴重物)
    // 負の対照: 「着物」「建物」は 1 形態素なので 物 が単独で現れない。
    // [13 / 239]
    Rule {
        surface: "物",
        pos: None,
        cue: Cue::PrevPos(|pos| matches!(pos, POS::Meishi(Meishi::KeiyoudoushiGokan))),
        reading: "ブツ",
        word_accent: None,
    },
    // 名詞 + 「尼」は接尾辞の ニ と読む。(修道尼・恵信尼)
    // 「翌日尼に会う」のような副詞的な名詞の後では、独立した アマ を残す。
    // [10 / 0]
    Rule {
        surface: "尼",
        pos: None,
        cue: Cue::All(&[
            Cue::PrevPos(|pos| matches!(pos, POS::Meishi(_))),
            Cue::Not(&Cue::PrevPos(|pos| matches!(pos, POS::Meishi(Meishi::FukushiKanou) | POS::Meishi(Meishi::Hijiritsu(_)) | POS::Doushi(Doushi::Hijiritsu) | POS::Keiyoushi(Keiyoushi::Hijiritsu)))),
        ]),
        reading: "ニ",
        word_accent: None,
    },
    // 活用語 + 「者」は モノ と読む。(成り上がり者・若い者・働く者)
    // 医者・記者 は 1 形態素なので 者 が単独で現れない。
    // 「受者」の「受」が動詞の連用形になるため、1 文字の語幹は除く。
    // [8 / 407]
    Rule {
        surface: "者",
        pos: None,
        cue: Cue::All(&[
            Cue::PrevPos(is_independent),
            Cue::PrevMinChars(2),
        ]),
        reading: "モノ",
        word_accent: None,
    },
    // 活用語 + 「処」は トコロ と読む。(たべる処・住む処)
    // [19 / 18]
    Rule {
        surface: "処",
        pos: None,
        cue: Cue::PrevPos(is_independent),
        reading: "トコロ",
        word_accent: None,
    },
    // 「に於て」「に於ける」の 於 は オイ と読む。
    // [26 / 0]
    Rule {
        surface: "於",
        pos: None,
        cue: Cue::NextIn(&["て", "ては", "ても"]),
        reading: "オイ",
        word_accent: None,
    },
    // 名詞 + 「茶屋」は連濁して ジャヤ と読む。(料理茶屋・芝居茶屋・三日月茶屋)
    // 副詞的な名詞を外すのは 寺 と同じ理由で、`きょう茶屋町へ` の 茶屋 は
    // 複合語の後部要素ではない。
    // [9 / 0]
    Rule {
        surface: "茶屋",
        pos: None,
        cue: Cue::All(&[
            Cue::PrevPos(|pos| matches!(pos, POS::Meishi(_))),
            Cue::PrevPos(|pos| matches!(pos, POS::Meishi(Meishi::General) | POS::Fukushi(Fukushi::General) | POS::Kigou(Kigou::General) | POS::Meishi(Meishi::KoyuMeishi(_)) | POS::Meishi(Meishi::SahenSetsuzoku))),
        ]),
        reading: "ジャヤ",
        word_accent: None,
    },
    // 「視入った」「駈け入った」など、前の動詞が連用形の複合動詞を イッ と読む。
    // 「ひがみ入っている」の ひがみ も動詞の連用形と解析されるが、
    // 名詞の「ひがみ」が入る意味なので、ハイッ を残す。
    // [12 / 0]
    Rule {
        surface: "入っ",
        pos: None,
        cue: Cue::All(&[
            Cue::PrevPos(|pos| matches!(pos, POS::Doushi(_))),
            Cue::PrevCform(&[CForm::Renyou]),
            Cue::Not(&Cue::PrevIn(&["ひがみ", "僻み"])),
        ]),
        reading: "イッ",
        word_accent: None,
    },
    // 「何で」「何でも」の 何 は ナン と読む。
    // [31 / 364]
    Rule {
        surface: "何",
        pos: None,
        cue: Cue::NextIn(&["で", "でも"]),
        reading: "ナン",
        word_accent: None,
    },
    // 「傍へ」「傍に」の 傍 は ソバ と読む。
    // [22 / 0]
    Rule {
        surface: "傍",
        pos: None,
        cue: Cue::NextIn(&["へ", "に"]),
        reading: "ソバ",
        word_accent: None,
    },
    // 助詞の後の「此処」は ココ と読む。
    // [30 / 1]
    Rule {
        surface: "此処",
        pos: None,
        cue: Cue::PrevPos(|pos| matches!(pos, POS::Joshi(_))),
        reading: "ココ",
        word_accent: None,
    },
    // 「外ならぬ君の頼み」は相手を特に指す慣用表現。
    // 「外ならぬ内側」は位置の否定なので、人や事情を指す後続語に限る。
    Rule {
        surface: "外",
        pos: Some(|pos| matches!(pos, POS::Meishi(_))),
        cue: Cue::外ならぬ,
        reading: "ホカ",
        word_accent: Some(0),
    },
    // 用件を切り出す「外でもない」はホカ。「内でも外でもない」は位置の否定。
    // 導入の位置と後続する用件の名詞を確認し、並列された否定は変更しない。
    Rule {
        surface: "外",
        pos: Some(|pos| matches!(pos, POS::Meishi(_))),
        cue: Cue::外でもない,
        reading: "ホカ",
        word_accent: Some(0),
    },
    // 「〜するより外にない」「君より外にない」の 外 は ホカ と読む。
    //
    // 「地球より外に惑星はない」は空間の比較なので、
    // 「より」の直前が動詞か代名詞の場合に限る。
    // [9 / 0]
    Rule {
        surface: "外",
        pos: None,
        cue: Cue::All(&[Cue::PrevPosVia(&["より"], |pos| matches!(pos, POS::Doushi(_))), Cue::Negated]),
        reading: "ホカ",
        word_accent: None,
    },
    // [9 / 0]
    Rule {
        surface: "外",
        pos: None,
        cue: Cue::All(&[
            Cue::PrevPosVia(&["より"], |pos| matches!(pos, POS::Meishi(Meishi::Daimeishi(_)))),
            Cue::Negated,
        ]),
        reading: "ホカ",
        word_accent: None,
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
        pos: Some(|pos| matches!(pos, POS::Settoushi(Settoushi::MeishiSetsuzoku))),
        cue: Cue::NextIn(&[
            "にぎわい", "丸髷", "地主", "旦那", "泥坊", "津波", "掃除",
            "番狂わせ", "番頭", "違い",
        ]),
        reading: "オー",
        word_accent: None,
    },
    // 接頭詞の 御 は、後ろが和語なら オ になる。漢語なら ゴ で、
    // 「御住所」「御多忙」は触らない。「御粗末」は漢語だが オソマツ が慣用である。
    // 負の対照: 御住所 ゴジューショ / 御多忙 ゴタボー。
    // [18 / 0]
    Rule {
        surface: "御",
        pos: Some(|pos| matches!(pos, POS::Settoushi(Settoushi::MeishiSetsuzoku))),
        cue: Cue::NextIn(&[
            "仕置", "屋敷", "帰り", "急ぎ", "手際", "支払い", "楽しみ",
            "気の毒", "田植祭", "神籤", "粗末", "言葉", "近く", "隣", "嬢",
        ]),
        reading: "オ",
        word_accent: None,
    },
    // 「〜町史」の 町 は チョー と読む。
    // [6 / 0]
    Rule {
        surface: "町",
        pos: None,
        cue: Cue::NextIn(&["史"]),
        reading: "チョー",
        word_accent: None,
    },
    // 名詞 + 「余」は接尾辞の ヨ と読む。(万余・十余)
    // [8 / 80]
    Rule {
        surface: "余",
        pos: None,
        cue: Cue::PrevPos(|pos| matches!(pos, POS::Meishi(_))),
        reading: "ヨ",
        word_accent: None,
    },
    // 「前期」「前段階」と、役職が直後に来る「前会長」の 前 は ゼン と読む。
    // 役職は収集データで 会長 280 / 社長 165 / 首相 129 / 理事長 71 / 大統領 56 と
    // 続き、いずれも マエ には出てこない。直後に付く語だけを見るので、
    // 「〜の前、会長が」のように読点が挟まる形とは区別が付く。
    // [29 / 0]
    Rule {
        surface: "前",
        pos: None,
        cue: Cue::NextIn(&[
            "会長", "大統領", "段階", "理事長", "社長", "総裁", "期", "首相",
        ]),
        reading: "ゼン",
        word_accent: None,
    },

    // 「金型」の 金 は カナ と読む。
    // 「返戻金型」は「返戻金」+「型」なので キン を残す。
    // [10 / 0]
    Rule {
        surface: "金",
        pos: None,
        cue: Cue::All(&[
            Cue::NextIn(&["型"]),
            Cue::Not(&Cue::PrevIn(&["返戻"])),
        ]),
        reading: "カナ",
        word_accent: None,
    },
    // 「一目で」の 一目 は ヒトメ と読む。
    // [5 / 0]
    Rule {
        surface: "一目",
        pos: None,
        cue: Cue::NextIn(&["で"]),
        reading: "ヒトメ",
        word_accent: None,
    },
    // 格助詞の後の「叩き」は タタキ と読む。
    // [5 / 3]
    Rule {
        surface: "叩き",
        pos: None,
        cue: Cue::PrevPos(|pos| matches!(pos, POS::Joshi(Joshi::KakuJoshi(_)))),
        reading: "タタキ",
        word_accent: None,
    },
    // 「此処に」「此処へ」の 此処 は ココ と読む。
    // [30 / 1]
    Rule {
        surface: "此処",
        pos: None,
        cue: Cue::NextIn(&["に", "へ", "で", "が", "は", "を"]),
        reading: "ココ",
        word_accent: None,
    },
    // 活用語 + 「処」は トコロ と読む。助動詞のあとも同じである。(落ち込んでいた処)
    // [19 / 18]
    Rule {
        surface: "処",
        pos: None,
        cue: Cue::PrevPos(|pos| matches!(pos, POS::Jodoushi)),
        reading: "トコロ",
        word_accent: None,
    },
    // 副詞 + 「間」は マ と読む。(少し間をおいて)
    // [7 / 0]
    Rule {
        surface: "間",
        pos: None,
        cue: Cue::PrevPos(|pos| matches!(pos, POS::Fukushi(_))),
        reading: "マ",
        word_accent: None,
    },
    // 「この間の」の この間 は コノアイダ と読む。
    // [6 / 0]
    Rule {
        surface: "この間",
        pos: None,
        cue: Cue::NextIn(&["の"]),
        reading: "コノアイダ",
        word_accent: None,
    },
    // 「識って」は シッテ と読む。(知って の異表記)
    // [7 / 0]
    Rule {
        surface: "識",
        pos: None,
        cue: Cue::NextIn(&["って", "った", "り"]),
        reading: "シ",
        word_accent: None,
    },
    // 名詞 + 「翁」は オー と読む。(芭蕉翁・計算翁)
    // 「明日翁が来る」のような副詞的な名詞の後では、独立した オキナ を残す。
    // [8 / 8]
    Rule {
        surface: "翁",
        pos: None,
        cue: Cue::All(&[
            Cue::PrevPos(|pos| matches!(pos, POS::Meishi(_))),
            Cue::Not(&Cue::PrevPos(|pos| matches!(pos, POS::Meishi(Meishi::FukushiKanou) | POS::Meishi(Meishi::Hijiritsu(_)) | POS::Doushi(Doushi::Hijiritsu) | POS::Keiyoushi(Keiyoushi::Hijiritsu)))),
        ]),
        reading: "オー",
        word_accent: None,
    },
    // 「仰しゃる」は オッシャル と読む。
    // [6 / 0]
    Rule {
        surface: "仰",
        pos: None,
        cue: Cue::NextIn(&["しゃっ", "しゃる", "しゃい", "しゃら", "しゃり"]),
        reading: "オッ",
        word_accent: None,
    },
    // 「〜て了った」は テシマッタ と読む。
    // [28 / 0]
    Rule {
        surface: "了",
        pos: None,
        cue: Cue::PrevIn(&["て", "で"]),
        reading: "シマ",
        word_accent: None,
    },
    // 日常の手入れや運動の動詞が「身体を」に続くとき、「身体」は カラダ。
    // 助詞だけでは「身体を拘束する」の シンタイと分けられないので、助詞を飛ばして
    // 述語の原形で見る。活用形が変わっても原形は変わらない。
    //
    // 負の対照: 「身体を拘束する」「身体を害する」は発火しない (シンタイのまま)。
    // 述語でない語が続く「身体を温かい風呂で」にも発火しない。
    // [14 / 0]
    Rule {
        surface: "身体",
        pos: None,
        cue: Cue::NextViaOrig(
            &["を"],
            &[
                "動かす", "休める", "張る", "持て余す", "冷やす", "鍛える", "洗う",
                "温める", "持つ", "拭く", "支える", "起こす", "伸び縮む", "触る",
                "揺する", "見る", "投げ出す", "折り曲げる", "ずらす", "かばう",
                "覆う",
            ],
        ),
        reading: "カラダ",
        word_accent: Some(0),
    },
    // からだの調子や手入れを表す述語が「身体に」に続くとき、「身体」は カラダ。
    // 「身体に障害が」の シンタイとは、助詞の先の語で分かれる。
    //
    // 負の対照: 「身体に障害が残った」「身体に異常が」は発火しない。
    // [2 / 0]
    Rule {
        surface: "身体",
        pos: None,
        cue: Cue::NextViaOrig(
            &["に"],
            &["気", "合う", "障る", "残る", "良い", "持つ", "かなう", "触る"],
        ),
        reading: "カラダ",
        word_accent: Some(0),
    },
    // 「身体の調子」のように、からだの状態を表す名詞が続くとき、「身体」は カラダ。
    // 「身体の機能」の シンタイとは、続く名詞で分かれる。
    // 「身体の衰え」は硬い書き言葉では シンタイにも読めるので入れない。
    //
    // 負の対照: 「身体の機能」「身体の自由」「老化や身体の衰え」は発火しない。
    // [1 / 0]
    Rule {
        surface: "身体",
        pos: None,
        cue: Cue::NextViaOrig(&["の"], &["調子", "ため", "具合"]),
        reading: "カラダ",
        word_accent: Some(0),
    },
    // 「身体が丈夫だ」のように、からだの状態を表す語が「身体が」に続くときは カラダ。
    // 「身体が不自由だ」の シンタイとは、続く語で分かれる。
    //
    // 負の対照: 「身体が不自由だ」は発火しない。
    // [3 / 0]
    Rule {
        surface: "身体",
        pos: None,
        cue: Cue::NextViaOrig(
            &["が"],
            &[
                "資本", "鈍る", "丈夫", "重い", "軽い", "悪い", "弱い", "汚れる",
                "小さい", "浮く",
            ],
        ),
        reading: "カラダ",
        word_accent: Some(0),
    },
    // 「一しょ」は イッショ と読む。(一緒 の異表記)
    // [15 / 144]
    Rule {
        surface: "一",
        pos: None,
        cue: Cue::NextIn(&["しょ"]),
        reading: "イッ",
        word_accent: None,
    },
    // 格助詞・係助詞の前の 貴女 は アナタ と読む。
    // 「の」を入れないのは、`若い貴女のために` の キジョ と割れるためである。
    // [21 / 0]
    Rule {
        surface: "貴女",
        pos: None,
        cue: Cue::NextIn(&["が", "は", "を", "に", "も", "から", "と"]),
        reading: "アナタ",
        word_accent: None,
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
        pos: None,
        cue: Cue::PrevPos(|pos| matches!(pos, POS::Meishi(Meishi::Daimeishi(_)))),
        reading: "ラ",
        word_accent: None,
    },
    // [88 / 153]
    Rule {
        surface: "等",
        pos: None,
        cue: Cue::PrevPos(|pos| matches!(pos, POS::Meishi(Meishi::General) | POS::Fukushi(Fukushi::General) | POS::Kigou(Kigou::General) | POS::Meishi(Meishi::SahenSetsuzoku) | POS::Meishi(Meishi::KoyuMeishi(_)) | POS::Meishi(Meishi::Setsubi(_)) | POS::Doushi(Doushi::Setsubi) | POS::Keiyoushi(Keiyoushi::Setsubi))),
        reading: "トー",
        word_accent: None,
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
        pos: None,
        cue: Cue::NextIn(&["学位", "論文", "号", "課程"]),
        reading: "ハクシ",
        word_accent: None,
    },
    // 直前が学位の分野名か 大学院 のとき。収集データで裏付けがあるのは
    // 医学・工学・理学と 大学院 で、残りは同じ形の分野名として足した。
    // [18 / 0]
    Rule {
        surface: "博士",
        pos: None,
        cue: Cue::PrevIn(&[
            "医学", "工学", "理学", "農学", "薬学", "文学", "法学", "経済学", "大学院",
        ]),
        reading: "ハクシ",
        word_accent: None,
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
        pos: None,
        cue: Cue::PrevViaNo(&[
            "うちわ", "やり", "傘", "剃刀", "団扇", "提灯", "斧", "柄杓", "槍", "洋傘",
            "箒", "薙刀", "鋏", "鋤", "鋸", "鍬", "錫杖",
        ]),
        reading: "エ",
        word_accent: None,
    },
    // [2 / 0]
    Rule {
        surface: "柄",
        pos: None,
        cue: Cue::PrevViaNo(&[
            "刀", "剣", "大刀", "太刀", "小剣", "懐剣", "木剣", "短刀", "短剣", "脇差",
            "軍刀", "長脇差", "鎧通し",
        ]),
        reading: "ツカ",
        word_accent: None,
    },
    // 器物の「〜の縁」は フチ。既定の エン は 縁がある / 縁側 の意味で、
    // 「の」を挟んで器物が来る形では出てこない。
    // 負の対照: 「縁を切る」「血縁」は直前が「の」ではないので発火しない。
    // [15 / 0]
    Rule {
        surface: "縁",
        pos: None,
        cue: Cue::PrevViaNo(&[
            "カップ", "器", "堀", "崖", "径", "屏風", "内側", "口", "皿", "模様",
            "火鉢", "金",
        ]),
        reading: "フチ",
        word_accent: None,
    },
];

/// [`Cue::Negated`] が探す打ち消しの語の原形。
const NEGATIVE_ORIGS: &[&str] = &["ない", "無い", "ぬ", "ん", "まい", "ず"];

fn 辛味を表す(words: &[impl ContextWord], i: usize) -> bool {
    if words.get(i + 1).is_some_and(|word| {
        word.pos_matches(|pos| matches!(pos, POS::Meishi(_)))
            && !matches!(word.surface(), "の" | "ん")
    }) {
        return false;
    }
    let mut end = i;
    while end > 0 && i - end < 3 {
        let word = &words[end - 1];
        if matches!(
            word.surface(),
            "とても" | "非常" | "かなり" | "すごく" | "少し" | "ちょっと"
        ) && word.pos_matches(|pos| matches!(pos, POS::Fukushi(_) | POS::Keiyoushi(_)))
        {
            end -= 1;
        } else if end >= 2
            && words[end - 1].surface() == "に"
            && words[end - 2].surface() == "非常"
            && adverbial_pair(&words[end - 2], &words[end - 1])
        {
            end -= 2;
        } else {
            break;
        }
    }
    if end < 2
        || !matches!(words[end - 1].surface(), "が" | "は")
        || !words[end - 1].pos_matches(|pos| matches!(pos, POS::Joshi(_)))
        || !matches!(
            words[end - 2].surface(),
            "カレー"
                | "唐辛子"
                | "キムチ"
                | "わさび"
                | "山葵"
                | "胡椒"
                | "コショウ"
                | "ソース"
                | "ラーメン"
                | "スープ"
                | "味付け"
                | "麻婆豆腐"
        )
    {
        return false;
    }
    let mut start = end - 2;
    if start > 0 && words[start - 1].pos_matches(|pos| matches!(pos, POS::Rentaishi)) {
        start -= 1;
    }
    // 「一週間カレー」「毎日カレー」のような名詞を伴う主語は推測しない。
    start == 0 || matches!(words[start - 1].surface(), "。" | "、" | "「" | "『")
}

fn 日付を導く来る(words: &[impl ContextWord], i: usize) -> bool {
    if i > 0 && !matches!(words[i - 1].surface(), "。" | "、" | "「" | "『") {
        return false;
    }
    let Some(next) = words.get(i + 1) else {
        return false;
    };
    if !next.pos_matches(|pos| matches!(pos, POS::Meishi(_))) {
        return false;
    }
    matches!(
        next.surface(),
        "月曜日" | "火曜日" | "水曜日" | "木曜日" | "金曜日" | "土曜日" | "日曜日"
    ) || next.surface().strip_suffix('月').is_some_and(|number| {
        !number.is_empty()
            && number
                .chars()
                .all(|c| "0123456789０１２３４５６７８９一二三四五六七八九十".contains(c))
    }) || next.pos_matches(|pos| matches!(pos, POS::Meishi(Meishi::Kazu)))
        && words
            .get(i + 2)
            .is_some_and(|unit| matches!(unit.surface(), "月" | "日" | "年"))
}

fn 用件の名詞(word: &impl ContextWord) -> bool {
    word.pos_matches(|pos| matches!(pos, POS::Meishi(_)))
        && matches!(
            word.surface(),
            "用件"
                | "用事"
                | "件"
                | "話"
                | "相談"
                | "予定"
                | "お願い"
                | "頼み"
                | "依頼"
                | "問題"
                | "質問"
                | "連絡"
                | "報告"
                | "計画"
                | "事情"
                | "理由"
        )
}

fn 外ならぬ(words: &[impl ContextWord], i: usize) -> bool {
    if !words
        .get(i + 1)
        .is_some_and(|word| word.surface() == "なら")
        || !words
            .get(i + 2)
            .is_some_and(|word| matches!(word.surface(), "ぬ" | "ない"))
        || i > 0 && matches!(words[i - 1].surface(), "の" | "も" | "より")
    {
        return false;
    }
    let mut next = i + 3;
    if words
        .get(next)
        .is_some_and(|word| word.pos_matches(|pos| matches!(pos, POS::Rentaishi)))
    {
        next += 1;
    }
    words.get(next).is_some_and(|word| {
        word.pos_matches(|pos| {
            matches!(pos, POS::Meishi(Meishi::KoyuMeishi(KoyuMeishi::Person(_))))
        }) || word.pos_matches(|pos| matches!(pos, POS::Meishi(_)))
            && matches!(
                word.surface(),
                "君" | "あなた"
                    | "貴方"
                    | "私"
                    | "僕"
                    | "俺"
                    | "彼"
                    | "彼女"
                    | "自分"
                    | "先生"
                    | "恩師"
                    | "師匠"
                    | "陛下"
                    | "本人"
                    | "当人"
                    | "事情"
                    | "理由"
                    | "原因"
                    | "事実"
                    | "目的"
            )
    })
}

fn 外でもない(words: &[impl ContextWord], i: usize) -> bool {
    if !words.get(i + 1..i + 4).is_some_and(|after| {
        after
            .iter()
            .map(ContextWord::surface)
            .eq(["で", "も", "ない"])
    }) {
        return false;
    }

    let mut before = i;
    if before > 0 && words[before - 1].surface() == "、" {
        before -= 1;
    } else if before > 0 && !matches!(words[before - 1].surface(), "「" | "『" | "は") {
        return false;
    }
    if before > 0 && words[before - 1].surface() == "は" {
        // 「電話したのは」「用件は」は導入になるが、「家の中は」は場所を表す。
        if before < 2
            || !((words[before - 2].surface() == "の"
                && words[before - 2]
                    .pos_matches(|pos| matches!(pos, POS::Meishi(Meishi::Hijiritsu(_)))))
                || 用件の名詞(&words[before - 2]))
        {
            return false;
        }
    } else if before > 0 && NEGATIVE_ORIGS.contains(&words[before - 1].orig()) {
        return false;
    }

    let mut start = i + 4;
    if words.get(start).is_some_and(|word| word.surface() == "が") {
        start += 1;
    }
    if words.get(start).is_some_and(|word| word.surface() == "、") {
        start += 1;
    }
    if words
        .get(start)
        .is_some_and(|word| word.surface() == "ちょっと")
    {
        start += 1;
    }

    let mut topic = false;
    // 6形態素以内で名詞句の終端まで確認できた場合だけ、最後の名詞で用件と判断する。
    for (offset, word) in words.iter().skip(start).take(7).enumerate() {
        if matches!(
            word.surface(),
            "内" | "中" | "内側" | "内部" | "室内" | "屋内"
        ) {
            return false;
        }
        if !word.pos_matches(|pos| matches!(pos, POS::Meishi(_) | POS::Rentaishi))
            && !(word.surface() == "の"
                && word.pos_matches(|pos| matches!(pos, POS::Joshi(Joshi::Rentaika))))
        {
            break;
        }
        if offset == 6 {
            return false;
        }
        if word.pos_matches(|pos| matches!(pos, POS::Meishi(_))) {
            topic = 用件の名詞(word);
        }
    }
    topic
}

fn negative_nannimo(features: &[impl ContextWord], i: usize) -> bool {
    if !features[i].pos_matches(|pos| matches!(pos, POS::Meishi(_)))
        || !features.get(i + 1).is_some_and(|f| {
            f.surface() == "に"
                && f.pos_matches(|pos| matches!(pos, POS::Joshi(Joshi::KakuJoshi(_))))
        })
        || !features.get(i + 2).is_some_and(|f| {
            f.surface() == "も"
                && f.pos_matches(|pos| matches!(pos, POS::Joshi(Joshi::KakariJoshi)))
        })
    {
        return false;
    }
    let start = i + 3;
    let mut index = start;
    let mut predicate_found = false;
    while let Some(feature) = features.get(index) {
        if !predicate_found {
            if index == start
                && feature.surface() == "しない"
                && feature.pos_matches(|pos| matches!(pos, POS::Meishi(_)))
            {
                return true;
            }
            if feature.pos_matches(|pos| matches!(pos, POS::Fukushi(_))) {
                index += 1;
                continue;
            }
            // 「返事がない」は打ち消しの用法だが、名詞全般を飛ばすと別の主語へ進む。
            if index == start
                && feature.surface() == "返事"
                && features.get(index + 1).is_some_and(|f| f.surface() == "が")
            {
                index += 2;
                continue;
            }
            if !feature.pos_matches(|pos| {
                matches!(pos, POS::Doushi(_) | POS::Keiyoushi(_) | POS::Jodoushi)
            }) || !matches!(
                feature.orig(),
                "知る"
                    | "わかる"
                    | "分かる"
                    | "分る"
                    | "する"
                    | "出来る"
                    | "できる"
                    | "なる"
                    | "言う"
                    | "やる"
                    | "食べる"
                    | "聞く"
                    | "答える"
                    | "ある"
                    | "ない"
                    | "無い"
                    | "面白い"
            ) {
                return false;
            }
            predicate_found = true;
        } else if matches!(feature.surface(), "こと" | "事")
            && feature.pos_matches(|pos| {
                matches!(
                    pos,
                    POS::Meishi(Meishi::Hijiritsu(_))
                        | POS::Doushi(Doushi::Hijiritsu)
                        | POS::Keiyoushi(Keiyoushi::Hijiritsu)
                )
            })
            && features.get(index + 1).is_some_and(|f| f.surface() == "が")
            && features
                .get(index + 2)
                .is_some_and(|f| matches!(f.surface(), "でき" | "出来"))
        {
            predicate_found = false;
            index += 2;
            continue;
        } else if !(feature
            .pos_matches(|pos| matches!(pos, POS::Jodoushi | POS::Doushi(Doushi::Hijiritsu)))
            || (feature.pos_matches(|pos| matches!(pos, POS::Joshi(_)))
                && matches!(feature.surface(), "て" | "で" | "は" | "も")))
        {
            return false;
        }
        if feature.pos_matches(|pos| matches!(pos, POS::Jodoushi | POS::Keiyoushi(_)))
            && NEGATIVE_ORIGS.contains(&feature.orig())
        {
            return true;
        }
        index += 1;
    }
    false
}

fn adverbial_pair(modifier: &impl ContextWord, particle: &impl ContextWord) -> bool {
    match particle.surface() {
        "に" => {
            particle.pos_matches(|pos| {
                matches!(
                    pos,
                    POS::Joshi(Joshi::Fukushika | Joshi::KakuJoshi(KakuJoshi::General))
                )
            }) && modifier.pos_matches(|pos| matches!(pos, POS::Meishi(Meishi::KeiyoudoushiGokan)))
        }
        "と" => {
            particle.pos_matches(|pos| matches!(pos, POS::Joshi(Joshi::Fukushika)))
                && modifier.pos_matches(|pos| matches!(pos, POS::Fukushi(_)))
        }
        _ => false,
    }
}

fn previous_via_particle(
    words: &[impl ContextWord],
    i: usize,
    particles: &[&str],
    candidates: &[&str],
) -> bool {
    let mut end = i;
    // 副詞を4形態素まで遡り、修飾語が長い文では推測しない。
    while i - end < 4 && end > 0 {
        let word = &words[end - 1];
        if word
            .pos_matches(|pos| matches!(pos, POS::Fukushi(_) | POS::Meishi(Meishi::FukushiKanou)))
            || (word.pos_matches(|pos| matches!(pos, POS::Keiyoushi(_)))
                && word.cform().is_some_and(|form| form.is_renyou()))
        {
            end -= 1;
        } else if end >= 2 && i - end + 2 <= 4 && adverbial_pair(&words[end - 2], word) {
            end -= 2;
        } else {
            break;
        }
    }
    end >= 2
        && particles.contains(&words[end - 1].surface())
        && words[end - 1].pos_matches(|pos| matches!(pos, POS::Joshi(_)))
        && candidates.contains(&words[end - 2].surface())
}

/// [`Cue::All`] が手がかりを入れ子にできるので、判定は再帰で書く。
fn cue_matches(cue: &Cue, njd_features: &[impl ContextWord], i: usize) -> bool {
    match cue {
        Cue::OrigIn(candidates) => candidates.contains(&njd_features[i].orig()),
        Cue::NegativeNannimo => negative_nannimo(njd_features, i),
        Cue::外ならぬ => 外ならぬ(njd_features, i),
        Cue::外でもない => 外でもない(njd_features, i),
        Cue::辛味を表す => 辛味を表す(njd_features, i),
        Cue::日付を導く来る => 日付を導く来る(njd_features, i),
        Cue::NextIn(candidates) => njd_features
            .get(i + 1)
            .is_some_and(|next| candidates.contains(&next.surface())),
        Cue::NextSequence(sequence) => {
            njd_features
                .get(i + 1..i + 1 + sequence.len())
                .is_some_and(|words| {
                    words
                        .iter()
                        .map(ContextWord::surface)
                        .eq(sequence.iter().copied())
                })
        }
        Cue::CformIn(candidates) => njd_features[i]
            .cform()
            .is_some_and(|form| candidates.contains(&form)),
        Cue::PrevViaParticle(particles, candidates) => {
            previous_via_particle(njd_features, i, particles, candidates)
        }
        Cue::PrevIn(candidates) => i > 0 && candidates.contains(&njd_features[i - 1].surface()),
        Cue::PrevViaNo(candidates) => {
            i > 1
                && njd_features[i - 1].surface() == "の"
                && candidates.contains(&njd_features[i - 2].surface())
        }
        Cue::PrevPosVia(via, predicate) => {
            i >= 2
                && via.contains(&njd_features[i - 1].surface())
                && njd_features[i - 2].pos_matches(*predicate)
        }
        Cue::Negated => njd_features
            .get(i + 1..)
            .unwrap_or_default()
            .iter()
            .take_while(|f| !matches!(f.surface(), "。" | "．" | "！" | "？"))
            .any(|f| NEGATIVE_ORIGS.contains(&f.orig())),
        Cue::PrevPos(predicate) => i > 0 && njd_features[i - 1].pos_matches(*predicate),
        Cue::PrevCform(candidates) => {
            i > 0
                && njd_features[i - 1]
                    .cform()
                    .is_some_and(|form| candidates.contains(&form))
        }
        Cue::PrevMinChars(min) => i > 0 && njd_features[i - 1].surface().chars().count() >= *min,
        Cue::Not(cue) => !cue_matches(cue, njd_features, i),
        Cue::NextPos(predicate) => njd_features
            .get(i + 1)
            .is_some_and(|next| next.pos_matches(*predicate)),
        Cue::NextViaOrig(via, candidates) => {
            njd_features
                .get(i + 1)
                .is_some_and(|next| via.contains(&next.surface()))
                && njd_features
                    .get(i + 2)
                    .is_some_and(|head| candidates.contains(&head.orig()))
        }
        Cue::All(cues) => cues.iter().all(|c| cue_matches(c, njd_features, i)),
    }
}

#[derive(Default)]
struct SurfaceRules {
    rules: Vec<&'static Rule>,
    has_word_accent: bool,
}

/// 表層形から、その表層形を対象にする規則を引く。並びは [`RULES`] の順のままで、
/// 上から評価して最初に一致したものを使う性質を保つ。
static RULES_BY_SURFACE: LazyLock<FxHashMap<&'static str, SurfaceRules>> = LazyLock::new(|| {
    let mut by_surface: FxHashMap<&'static str, SurfaceRules> = FxHashMap::default();
    for rule in RULES.iter() {
        let group = by_surface.entry(rule.surface).or_default();
        group.rules.push(rule);
        group.has_word_accent |= rule.word_accent.is_some();
    }
    by_surface
});

fn first_matching_rule<'a>(
    mut rules: impl Iterator<Item = &'a Rule>,
    words: &[impl ContextWord],
    i: usize,
) -> Option<&'a Rule> {
    rules.find(|rule| {
        rule.pos
            .is_none_or(|predicate| words[i].pos_matches(predicate))
            && cue_matches(&rule.cue, words, i)
    })
}

fn matching_rule(words: &[impl ContextWord], i: usize) -> Option<&'static Rule> {
    RULES_BY_SURFACE
        .get(words[i].surface())
        .and_then(|group| first_matching_rule(group.rules.iter().copied(), words, i))
}

pub(crate) fn prepare_word_readings(nodes: &mut [NJDNode], protected: &[bool]) {
    let by_surface = &*RULES_BY_SURFACE;
    for i in 0..nodes.len() {
        if protected.get(i) == Some(&true) {
            continue;
        }
        let Some(group) = by_surface
            .get(nodes[i].get_string())
            .filter(|group| group.has_word_accent)
        else {
            continue;
        };
        // 核を指定しない規則が先に一致する場合もあるため、規則は全件を元の順に調べる。
        // 規則の条件は読みと発音を参照しないため、後続語の判定に影響しない。
        if let Some(rule) = first_matching_rule(group.rules.iter().copied(), nodes, i)
            && let Some(accent) = rule.word_accent
        {
            nodes[i].set_read(rule.reading);
            nodes[i].set_pron(Pronunciation::parse(rule.reading, accent).unwrap());
        }
    }
}

fn reading_after_chaining(
    features: &[NjdFeature],
    i: usize,
) -> Option<(&'static str, &'static str)> {
    // 日付の「付」はヅケであり、付属を表すツキとは異なる。
    if features[i].string == "付" && i > 0
        && let Some(day) = features[i - 1].string.strip_suffix('日')
        && (features[i - 1].pos_matches(|pos| matches!(pos, POS::Meishi(Meishi::Setsubi(Setsubi::Josuushi))))
            || (!day.is_empty() && day.chars().all(|c| matches!(c,
                '一' | '二' | '三' | '四' | '五' | '六' | '七' | '八' | '九' | '十' | '〇' | '零' | '0'..='9' | '０'..='９'))))
    { return None; }
    // 日数の一日を対象とし、十一日や日付のツイタチとは区別する。
    if features[i].string == "中"
        && features[i].read == "チュウ"
        && i > 0
        && features[i - 1].string == "一日"
        && features[i - 1].read == "イチニチ"
        && (i < 2 || !features[i - 2].pos_matches(|pos| pos.is_kazu()))
    {
        return Some(("ジュウ", "ジュー"));
    }
    let rule = matching_rule(features, i)?;
    rule.word_accent
        .is_none()
        .then_some((rule.reading, rule.reading))
}

/// 隣接する形態素で読みが決まる語を補正する。
pub(crate) fn modify_context_reading(features: &mut [NjdFeature], protected: &[bool]) {
    // 「一日中」は補正前の読みも条件にするため、全候補を確定してから読みと句の核を変える。
    let changes: Vec<_> = (0..features.len())
        .filter(|&i| protected.get(i) != Some(&true))
        .filter_map(|i| reading_after_chaining(features, i).map(|change| (i, change)))
        .collect();
    for (i, (reading, pronunciation)) in changes {
        let node = &mut features[i];
        // 同じ発音を書き直すと、NJD が付けた無声化の記号が消える。
        node.read = reading.into();
        if node
            .pron
            .chars()
            .filter(|&c| c != '’')
            .eq(pronunciation.chars())
        {
            continue;
        }
        let shortened_lake = node.string == "湖"
            && node
                .pron
                .chars()
                .filter(|&c| c != '’')
                .eq("ミズウミ".chars())
            && pronunciation == "コ";
        let old_mora = node.mora_size;
        node.pron = pronunciation.into();
        node.mora_size = count_mora(pronunciation) as i32;
        shift_accent_nucleus(features, i, old_mora, shortened_lake);
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
fn shift_accent_nucleus(
    njd_features: &mut [NjdFeature],
    i: usize,
    old_mora: i32,
    shortened_lake: bool,
) {
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
    // 3 モーラ以下の湖名は「コ」の直前に核を置く。
    // ミズウミの核を単に末尾へ丸めると、湖名が尾高型になる。
    njd_features[head].acc = if shortened_lake && before + new_mora <= 3 && acc > before + new_mora
    {
        before
    } else if acc > before + old_mora {
        acc + new_mora - old_mora
    } else {
        before + (acc - before).min(new_mora)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestWord<'a> {
        surface: &'a str,
        orig: &'a str,
        pos: Option<POS>,
        cform: Option<CForm>,
    }

    impl ContextWord for TestWord<'_> {
        fn surface(&self) -> &str {
            self.surface
        }

        fn orig(&self) -> &str {
            self.orig
        }

        fn pos(&self) -> Option<POS> {
            self.pos
        }

        fn cform(&self) -> Option<CForm> {
            self.cform
        }
    }

    fn noun(surface: &str, pron: &str, acc: i32, chain_flag: i32) -> NjdFeature {
        NjdFeature {
            string: surface.into(),
            pos: "名詞".into(),
            pos_group1: "一般".into(),
            pos_group2: "*".into(),
            pos_group3: "*".into(),
            ctype: "*".into(),
            cform: "*".into(),
            orig: surface.into(),
            read: pron.into(),
            pron: pron.into(),
            acc,
            mora_size: count_mora(pron) as i32,
            chain_rule: "C1".into(),
            chain_flag,
        }
    }

    #[test]
    fn kaburu_does_not_change_the_archaic_oou_conjugation() {
        let mut words = [
            TestWord {
                surface: "帽子",
                orig: "帽子",
                pos: Some(POS::Meishi(Meishi::General)),
                cform: Some(CForm::None),
            },
            TestWord {
                surface: "を",
                orig: "を",
                pos: Some(POS::Joshi(Joshi::KakuJoshi(KakuJoshi::General))),
                cform: Some(CForm::None),
            },
            TestWord {
                surface: "被っ",
                orig: "被る",
                pos: Some(POS::Doushi(Doushi::Jiritsu)),
                cform: Some(CForm::RenyouConjunctionTa),
            },
        ];
        assert_eq!(matching_rule(&words, 2).unwrap().reading, "カブッ");
        words[2].orig = "被う";
        assert!(matching_rule(&words, 2).is_none());
    }

    #[test]
    fn invalid_grammar_is_distinct_from_a_valid_unknown_pos() {
        let mut feature = noun("前", "マエ", 0, 0);
        feature.pos_group1 = "未定義の品詞".into();
        feature.cform = "未定義の活用形".into();
        assert_eq!(feature.pos(), None);
        assert_eq!(feature.cform(), None);
        assert!(!feature.pos_matches(|pos| matches!(pos, POS::Meishi(_))));
        feature.pos = "*".into();
        assert_eq!(feature.pos(), Some(POS::Unknown));
    }

    #[test]
    fn previous_particle_skips_at_most_four_modifiers_and_stops_at_a_clause() {
        let word = |surface: &'static str, pos, cform| TestWord {
            surface,
            orig: surface,
            pos: Some(pos),
            cform: Some(cform),
        };
        let noun = || word("甲", POS::Meishi(Meishi::General), CForm::None);
        let particle = || {
            word(
                "を",
                POS::Joshi(Joshi::KakuJoshi(KakuJoshi::General)),
                CForm::None,
            )
        };
        let adverb = || word("とても", POS::Fukushi(Fukushi::General), CForm::None);
        for count in 0..=5 {
            let mut words = vec![noun(), particle()];
            words.extend((0..count).map(|_| adverb()));
            assert_eq!(
                previous_via_particle(&words, words.len(), &["を"], &["甲"]),
                count <= 4
            );
        }
        for ni in [Joshi::Fukushika, Joshi::KakuJoshi(KakuJoshi::General)] {
            for count in 0..=3 {
                let mut words = vec![noun(), particle()];
                words.extend((0..count).map(|_| adverb()));
                words.push(word(
                    "完全",
                    POS::Meishi(Meishi::KeiyoudoushiGokan),
                    CForm::None,
                ));
                words.push(word("に", POS::Joshi(ni), CForm::None));
                assert_eq!(
                    previous_via_particle(&words, words.len(), &["を"], &["甲"]),
                    count <= 2
                );
            }
        }
        for count in 0..=3 {
            let mut words = vec![noun(), particle()];
            words.extend((0..count).map(|_| adverb()));
            words.push(word(
                "ぴたり",
                POS::Fukushi(Fukushi::JoshiruiSetsuzoku),
                CForm::None,
            ));
            words.push(word("と", POS::Joshi(Joshi::Fukushika), CForm::None));
            assert_eq!(
                previous_via_particle(&words, words.len(), &["を"], &["甲"]),
                count <= 2
            );
        }
        for boundary in [
            word("、", POS::Kigou(Kigou::General), CForm::None),
            word("乙", POS::Meishi(Meishi::General), CForm::None),
            word("見る", POS::Doushi(Doushi::Jiritsu), CForm::Basic),
            word(
                "に",
                POS::Joshi(Joshi::KakuJoshi(KakuJoshi::General)),
                CForm::None,
            ),
        ] {
            let words = [noun(), particle(), boundary, adverb()];
            assert!(!previous_via_particle(
                &words,
                words.len(),
                &["を"],
                &["甲"]
            ));
        }
    }

    #[test]
    fn word_accents_use_the_first_rule_with_matching_pos_and_context() {
        let nodes = [
            NJDNode::new_single("身体,名詞,一般,*,*,*,*,身体,シンタイ,シンタイ,1/4,C2"),
            NJDNode::new_single("を,助詞,格助詞,一般,*,*,*,を,ヲ,ヲ,0/1,*"),
        ];
        let preserve = Rule {
            surface: "身体",
            pos: Some(|pos| matches!(pos, POS::Meishi(_))),
            cue: Cue::NextIn(&["を"]),
            reading: "シンタイ",
            word_accent: Some(1),
        };
        let change = Rule {
            surface: "身体",
            pos: None,
            cue: Cue::NextIn(&["を"]),
            reading: "カラダ",
            word_accent: Some(0),
        };
        let selected = first_matching_rule([&preserve, &change].into_iter(), &nodes, 0).unwrap();
        assert_eq!(selected.reading, "シンタイ");
        assert_eq!(selected.word_accent, Some(1));
        let phrase_only = Rule {
            surface: "身体",
            pos: None,
            cue: Cue::NextIn(&["を"]),
            reading: "シンタイ",
            word_accent: None,
        };
        let selected = first_matching_rule([&phrase_only, &change].into_iter(), &nodes, 0).unwrap();
        assert_eq!(selected.word_accent, None);
        let incompatible = Rule {
            pos: Some(|pos| matches!(pos, POS::Doushi(_))),
            ..preserve
        };
        let selected =
            first_matching_rule([&incompatible, &change].into_iter(), &nodes, 0).unwrap();
        assert_eq!(selected.reading, "カラダ");
    }

    #[test]
    fn short_lake_retreats_only_the_nucleus_removed_with_mizuumi() {
        let mut nodes = [noun("西", "サイ", 5, 0), noun("湖", "ミズウミ", 3, 1)];
        modify_context_reading(&mut nodes, &[]);
        assert_eq!(
            (&*nodes[1].pron, nodes[1].mora_size, nodes[0].acc),
            ("コ", 1, 2)
        );

        let mut already_correct = [noun("西", "サイ", 3, 0), noun("湖", "コ", 1, 1)];
        let before = already_correct.clone();
        modify_context_reading(&mut already_correct, &[]);
        assert_eq!(already_correct, before);

        let mut protected = [noun("西", "サイ", 5, 0), noun("湖", "ミズウミ", 3, 1)];
        let before = protected.clone();
        modify_context_reading(&mut protected, &[false, true]);
        assert_eq!(protected, before);

        let mut earlier_nucleus = [noun("西", "サイ", 1, 0), noun("湖", "ミズウミ", 3, 1)];
        modify_context_reading(&mut earlier_nucleus, &[]);
        assert_eq!(earlier_nucleus[0].acc, 1);
    }
}
