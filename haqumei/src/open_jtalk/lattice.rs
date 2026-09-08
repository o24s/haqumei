//! MeCab のラティスを、候補ごとの経路コスト差とともに取り出す。
//!
//! Viterbi の経路コストは各形態素の単語コストについて線形なので、ある候補を
//! 通る最良経路のコストが分かれば、その候補を勝たせるのに必要な引き下げ量が
//! 一度の計算で決まる。
//!
//! ```text
//! delta = (その候補を通る最良経路のコスト) - (文全体の最良経路のコスト)
//! ```
//!
//! 前向きと後ろ向きの累積コストを合わせると、文に現れる全候補の `delta` が求まる。
//!
//! ```no_run
//! # use haqumei::OpenJTalk;
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut oj = OpenJTalk::new()?;
//! for node in oj.analyze_lattice("若死にした")? {
//!     if node.surface == "若死に" {
//!         // 0 なら最良経路上にある。正なら、その分だけ負けている
//!         println!("{}", node.delta);
//!     }
//! }
//! # Ok(())
//! # }
//! ```

use std::ops::Range;

#[cfg(doc)]
use crate::MecabMorph;
use crate::cursor::CharCursor;
use crate::errors::HaqumeiError;
use crate::open_jtalk::OpenJTalk;

/// ラティス上の候補ノード 1つ。
///
/// 最良経路に選ばれなかったものも含めて、その位置に立ちうる候補がすべて出る。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LatticeNode {
    /// 候補の表層形。
    pub surface: String,

    /// MeCab のノードが持つ特徴量文字列。
    ///
    /// MeCab が出力したままの形なので、[`MecabMorph::feature`] とは列が 1 つずれる。
    /// 原形が 6 番目、読みが 7 番目、発音が 8 番目である。[`MecabMorph::feature`] は
    /// 表層形が先頭に付くぶん 1 つうしろなので、同じ添字で読むと 1 つ手前の列が返る。
    pub feature: String,

    /// 解析対象の文字列における位置 (文字単位、半開区間)。
    ///
    /// 入力そのものではなく、`text2mecab` が正規化したあとの文字列を指す。
    /// 候補どうしが重なっているかどうかを見るのに使う。単位は
    /// [`MecabMorph::char_span`] と揃えてある。
    pub char_span: Range<usize>,

    /// left-id.def で定義された左文脈 ID。
    pub left_id: u16,

    /// right-id.def で定義された右文脈 ID。
    pub right_id: u16,

    /// pos-id.def で定義された品詞 ID。
    pub pos_id: u16,

    /// 辞書に定義された単語コスト。
    pub word_cost: i16,

    /// MeCab が未知語 (`MECAB_UNK_NODE`) と判定したかどうか。
    pub is_unknown: bool,

    /// この候補を何番目の辞書から引いたか。0 がシステム辞書。
    pub dictionary_index: u8,

    /// この候補を通る最良経路のコストと、文全体の最良経路のコストの差。
    ///
    /// `0` なら最良経路上にある。正なら、その値だけ負けている。単語コストを
    /// `delta + 1` 下げれば最良経路に乗る。
    pub delta: i64,

    /// 最良経路上にあるかどうか (`delta == 0` と同じ)。
    pub is_best: bool,
}

impl OpenJTalk {
    /// MeCab の全候補を、候補を通る最良経路のコスト差とともに返します。
    pub fn analyze_lattice(&mut self, text: &str) -> Result<Vec<LatticeNode>, HaqumeiError> {
        self.ensure_dictionary_is_latest()?;
        let text = self.text2mecab_string(text)?;
        let analysis = self.mecab.analyze_lattice(&text)?;
        let mut cursor = CharCursor::new(text.as_bytes());
        Ok(analysis
            .nodes
            .into_iter()
            .map(|node| {
                let start = cursor.char_at(node.byte_span.start);
                let end = cursor.char_at(node.byte_span.end);
                LatticeNode {
                    surface: text[node.byte_span].to_owned(),
                    feature: node.feature,
                    char_span: start..end,
                    left_id: node.left_id,
                    right_id: node.right_id,
                    pos_id: node.pos_id,
                    word_cost: node.word_cost,
                    is_unknown: node.is_unknown,
                    dictionary_index: node.dictionary_index,
                    delta: node.delta,
                    is_best: node.delta == 0,
                }
            })
            .collect())
    }
}
