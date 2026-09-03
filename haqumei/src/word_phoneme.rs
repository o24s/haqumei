#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use std::ops::Range;

use crate::{phoneme::Phoneme, prosody::ProsodicPhoneme};

/// Word と `Phoneme` リストに加えて、未知語かどうか・OpenJTalk などで無視されるかどうかを表すフラグをもつ構造体。
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct WordPhonemeMap {
    /// Word (表層形・辞書エントリを意味しない)
    pub word: String,
    pub phonemes: Vec<Phoneme>,

    /// 元となった形態素について、MeCab が未知語 (`MECAB_UNK_NODE`) と判定したかどうか。
    ///
    /// NJDの処理によって複数の形態素が結合された場合は、その中に1つでも未知語が含まれていれば `true` となる。
    pub is_unknown: bool,

    /// `pyopenjtalk` のパイプラインで無視される対象 ("記号,空白") として空白 (`sp`) に置き換えられたか、
    /// または NJD/JPCommon の処理結果として音素が1つも割り当てられなかったかどうか。
    ///
    /// (e.g., 先頭の `ー` など、他の形態素に長音として吸収されず破棄されたケース)
    pub is_ignored: bool,

    /// 解析対象の文字列における位置 (文字単位、半開区間)。
    ///
    /// 指すのは入力そのものではなく、Unicode 正規化と `text2mecab` を通したあとの
    /// 文字列である。`text2mecab` は制御文字と範囲外の文字を出力せず、半角カナと
    /// 濁点の並び (`ｶﾞ`) を 1 文字にまとめるので、入力と文字数が変わることがある。
    /// 入力を切り出す位置として使えるのは、
    /// [`crate::HaqumeiOptions::normalize_unicode`] が
    /// [`crate::UnicodeNormalization::None`] で、かつ入力に制御文字も半角カナも
    /// 無いときだけである。
    ///
    /// 複数の形態素がまとまった語では、まとめた範囲全体を指す。位取りとして
    /// 差し込まれた語 (「１４７３」から作られる「百」「十」) は元の文字を持た
    /// ないので、空の区間になる。
    ///
    /// 数字が縮約された語は間に挟まった空白まで含むので、区間どうしが重なる
    /// ことがある。「1 0 個」の「十」は `0..3` で、`1..2` の空白を含む。
    /// 語の並びが位置の昇順になるとも限らない (縮約に使った空白は、縮約後の語の
    /// うしろに置かれる)。位置の順に見たいなら並べ替える。
    ///
    /// [`crate::njd_char_spans`] が NJD の形態素列について返す区間と同じものが
    /// 入っている。
    pub char_span: Range<usize>,
}

/// Word と `Phoneme` リスト、未知語・無視フラグに加えて、Mecab の解析情報をもつ構造体。
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct WordPhonemeDetail {
    /// Word (表層形・辞書エントリを意味しない)
    pub word: String,
    pub phonemes: Vec<Phoneme>,

    /// Mecab が出力した features。
    /// 既知語は 12 列、未知語は 8 列 (read, pron, acc, chain_rule がない)
    pub features: Vec<String>,

    /// 品詞
    pub pos: String,
    /// 品詞細分類1
    pub pos_group1: String,
    /// 品詞細分類2
    pub pos_group2: String,
    /// 品詞細分類3
    pub pos_group3: String,

    /// 活用型
    pub ctype: String,
    /// 活用形
    pub cform: String,

    /// 原形
    pub orig: String,
    /// 読み
    pub read: String,
    /// 発音形式
    pub pron: String,

    /// アクセント核位置 (0: 平板型, 1-n: n番目のモーラにアクセント核)
    pub accent_nucleus: i32,
    /// モーラ数
    pub mora_count: i32,
    /// アクセント結合規則 (C1-C5/F1-F5/P1-P2 等)
    pub chain_rule: String,
    /// アクセント句連結フラグ
    pub chain_flag: i32,

    /// 元となった形態素について、MeCab が未知語 (`MECAB_UNK_NODE`) と判定したかどうか。
    ///
    /// NJDの処理によって複数の形態素が結合された場合は、その中に1つでも未知語が含まれていれば `true` となる。
    pub is_unknown: bool,

    /// `pyopenjtalk` のパイプラインで無視される対象 ("記号,空白") として空白 (`sp`) に置き換えられたか、
    /// または NJD/JPCommon の処理結果として音素が1つも割り当てられなかったかどうか。
    ///
    /// (e.g., 先頭の `ー` など、他の形態素に長音として吸収されず破棄されたケース)
    pub is_ignored: bool,

    /// 解析対象の文字列における位置 (文字単位、半開区間)。
    ///
    /// 意味は [`WordPhonemeMap::char_span`] と同じ。
    pub char_span: Range<usize>,
}

/// プロソディ情報つきの [ProsodicPhoneme] のリストや表層形、未知語・無視フラグ、Mecab の解析情報を表す構造体。
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct WordPhonemeProsody {
    /// Word (表層形・辞書エントリを意味しない)
    pub word: String,
    pub phonemes: Vec<ProsodicPhoneme>,

    /// 品詞
    pub pos: String,
    /// 品詞細分類1
    pub pos_group1: String,
    /// 品詞細分類2
    pub pos_group2: String,
    /// 品詞細分類3
    pub pos_group3: String,

    /// 活用型
    pub ctype: String,
    /// 活用形
    pub cform: String,

    /// 原形
    pub orig: String,
    /// 読み
    pub read: String,
    /// 発音形式
    pub pron: String,

    /// アクセント核位置 (0: 平板型, 1-n: n番目のモーラにアクセント核)
    pub accent_nucleus: i32,
    /// モーラ数
    pub mora_count: i32,
    /// アクセント結合規則 (C1-C5/F1-F5/P1-P2 等)
    pub chain_rule: String,
    /// アクセント句連結フラグ
    pub chain_flag: i32,

    /// 元となった形態素について、MeCab が未知語 (`MECAB_UNK_NODE`) と判定したかどうか。
    ///
    /// NJDの処理によって複数の形態素が結合された場合は、その中に1つでも未知語が含まれていれば `true` となる。
    pub is_unknown: bool,

    /// `pyopenjtalk` のパイプラインで無視される対象 ("記号,空白") として空白 (`sp`) に置き換えられたか、
    /// または NJD/JPCommon の処理結果として音素が1つも割り当てられなかったかどうか。
    ///
    /// (e.g., 先頭の `ー` など、他の形態素に長音として吸収されず破棄されたケース)
    pub is_ignored: bool,

    /// 解析対象の文字列における位置 (文字単位、半開区間)。
    ///
    /// 意味は [`WordPhonemeMap::char_span`] と同じ。
    pub char_span: Range<usize>,
}
