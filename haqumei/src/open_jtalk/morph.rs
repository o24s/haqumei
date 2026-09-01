use std::ops::Range;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MecabMorph {
    /// 形態素の表層形。
    pub surface: String,

    /// MeCab の特徴量文字列に、表層形を先頭に付けたもの。
    ///
    /// Open JTalk の `mecab.cpp` が `mecab2njd` に渡す形に揃えてあるので、
    /// [`crate::LatticeNode::feature`] とは列が 1 つずれる。原形が 7 番目、読みが
    /// 8 番目、発音が 9 番目で、[`crate::LatticeNode::feature`] は 1 つ手前になる。
    pub feature: String,

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

    /// 解析対象の文字列における位置 (文字単位、半開区間)。
    ///
    /// 入力そのものではなく、`text2mecab` が正規化したあとの文字列を指す。
    /// 形態素どうしの重なりを見たり、NJD の形態素列と突き合わせたりするのに使う。
    pub char_span: Range<usize>,

    /// この形態素を引いた辞書。
    ///
    /// `0` がシステム辞書で、`1` 以降が読み込み順のユーザー辞書に対応する。
    /// **辞書を引かずに作られたノード (未知語、BOS/EOS) は
    /// [`NO_DICTIONARY_INDEX`] になる。** `1` 以上かどうかで判定すると未知語を
    /// ユーザー辞書と取り違えるので、[`MecabMorph::is_from_user_dictionary`] を使う。
    pub dictionary_index: u8,

    /// `pyopenjtalk` のパイプラインで無視される対象かどうか。 ("記号,空白")
    ///
    /// ここでは、`pyopenjtalk` は Mecab の出力に対して、どのように必要のないトークンを除去していたか、
    /// ということをフラグによって明確にするものであって、JPCommon の音素割り当てと実際には関係がありません。
    pub is_ignored: bool,
}

/// 辞書を引かずに作られたノードの [`MecabMorph::dictionary_index`]。
///
/// 未知語と BOS/EOS が該当する (MeCab の `MECAB_NO_DICTIONARY_INDEX`)。
pub const NO_DICTIONARY_INDEX: u8 = crate::ffi::MECAB_NO_DICTIONARY_INDEX as u8;

impl MecabMorph {
    /// ユーザー辞書から引かれた形態素かどうか。
    ///
    /// 未知語の [`MecabMorph::dictionary_index`] は [`NO_DICTIONARY_INDEX`] で、
    /// これも `1` 以上なので、大小比較だけでは取り違える。
    pub fn is_from_user_dictionary(&self) -> bool {
        self.dictionary_index >= 1 && self.dictionary_index != NO_DICTIONARY_INDEX
    }
}
