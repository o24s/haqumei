//! 1 文につき複数の読みを返すためのモジュール。
//!
//! [`Haqumei::g2p_mapping`] は読みを 1 つに決めて返すが、Forced Alignment のように
//! 読みが分かれる箇所を音響モデルに決めさせたいときは、
//! [`Haqumei::g2p_candidates`] で読みの候補を出力できる。
//!
//! # 候補がどこから来るか
//!
//! 候補は MeCab のラティスからだけである。どの経路を通っても必ず形態素が切れる位置で
//! 文を区間に分け、経路が 2 通り以上あり、そのうち発音の違うものがある区間を
//! 分岐点とする。
//! 分岐点ごとに経路を 1 つ選んで feature 文字列を組み直し、MeCab の解析を除いた処理を
//! もう一度実行する。
//! どの候補も形態素列を 1 つに決めてから解析するので、`mecab2njd` より後ろには分岐が
//! 残らない。返る候補の中身は [`Haqumei::g2p_mapping`] の返り値と同じ形である。
//!
//! 経路によってノードの数が違うので、分割の違いも候補になる。「彼の」は
//! `彼` + `の` (カレノ) の 2 ノードと、連体詞 `彼の` (アノ) の 1 ノードに分かれる。
//!
//! # 出ない候補
//!
//! - 辞書のエントリが分かれていない読み: ラティスは同じ表層形に複数のエントリが
//!   あるときしか分岐しない
//! - 未知語の読み: [`CandidateReading::pron`] が `*` で読みを持たないので、既定では
//!   ラティスから外して
//!   いる ([`CandidateOptions::branch_on_unknown_words`])
//! - 後処理が読みを決める箇所: `predict_nani` は「何」の読みを、
//!   `modify_context_reading` は決定リストに載る表層形の読みを、`njd_set_digit` は
//!   数字の発音を、それぞれ無条件に書き込む。ラティスが分岐していても、音素列が
//!   同じになった候補は、コスト差の小さい方だけ残る
//! - 無声化の有無とポーズの実現: ラティスに出てこないので、`Phoneme::UnvoicedI` と
//!   `Phoneme::I` の置き換え、`Phoneme::Pau` の任意化として、返した音素列を
//!   呼び出し側が書き換える

use std::collections::{BinaryHeap, HashMap, HashSet};
use std::ops::Range;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::errors::HaqumeiError;
use crate::open_jtalk::reading_protection::protected_indices;
use crate::prosody::{PitchAccent, ProsodicPhoneme};
use crate::{
    Haqumei, MecabMorph, NjdFeature, WordPhonemeDetail, WordPhonemeMap, WordPhonemeProsody,
    postprocess,
};

/// [`crate::LatticeNode::feature`] の発音の列。
///
/// [`MecabMorph::feature`] は表層形が先頭に付くので 9 番目になる。
const NODE_PRON_COLUMN: usize = 8;

/// [`Haqumei::g2p_candidates`] がラティスから経路を集める範囲と、返す候補の数の上限。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CandidateOptions {
    /// ラティスに残すノードの、最良経路とのコスト差の上限。
    ///
    /// 大きくすると区間が繋がり、経路の数が組み合わせで増える。上限を外すと
    /// 1 文の組み合わせが実行できない大きさになる。
    pub max_delta: i64,

    /// 分岐点ごとに残す経路の上限。[`CandidateBranch::alternatives`] は先頭に 1-best を
    /// 置くので、長さは `max_alternatives_per_branch + 1` までになる。
    ///
    /// 経路の多い分岐点があると、そこを動かしただけの組み合わせで
    /// [`CandidateOptions::max_candidates`] に達し、後ろの分岐点を動かした組み合わせが
    /// 1 つも組み立てられないことがある。分岐点ごとに先に上限を掛けると起きない。
    pub max_alternatives_per_branch: usize,

    /// 返す候補の数の上限。`1` 未満は `1` として扱う。
    ///
    /// 先頭要素は 1-best なので、`1` を指定すると [`Haqumei::g2p_mapping`] と同じものが
    /// 1 件だけ返る。数が減るのは [`Candidates::candidates`] だけで、
    /// [`Candidates::branches`] は `1` にしても埋まる。
    pub max_candidates: usize,

    /// 未知語のノードを経路に含めるか。
    ///
    /// 未知語のノードは [`CandidateReading::pron`] が `*` で、読みは `read_unknown_kanji` と
    /// `restore_loanword_kana` が決める。辞書のエントリと並べても同じ読みになる
    /// ことが多いので、既定では `false` にしてラティスから外す。外すと、1-best が
    /// 未知語の区間では経路が 1 つも残らず、分岐しなくなる。
    pub branch_on_unknown_words: bool,
}

impl Default for CandidateOptions {
    fn default() -> Self {
        Self {
            max_delta: 2000,
            max_alternatives_per_branch: 4,
            max_candidates: 32,
            branch_on_unknown_words: false,
        }
    }
}

/// 1 文ぶんの候補集合。
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Candidates<T> {
    /// 解析に使った文字列。
    ///
    /// 入力に Unicode 正規化と `text2mecab` を掛けたあとのもので、
    /// [`WordPhonemeMap::char_span`] と [`CandidateBranch::char_span`] が指す先で
    /// ある。`text2mecab` は半角カナと濁点の並びを 1 文字にまとめ、ASCII を全角に
    /// するので、入力とは文字数が変わることがある。位置で文字列を切り出すときは
    /// 入力ではなく [`Candidates::text`] を使う。
    pub text: String,

    /// 経路が 2 通り以上ある区間。入力に現れる順に並び、
    /// [`CandidateOptions::max_candidates`] の上限を受けない。
    ///
    /// [`Candidates::candidates`] を並べて FST を組むと、上限に達して組み立てな
    /// かった組み合わせがそのまま欠ける。すべて残したいなら
    /// [`Candidates::branches`] から直積を組む。
    pub branches: Vec<CandidateBranch>,

    /// 候補。コスト差の小さい順に並び、先頭は 1-best である。
    ///
    /// 音素列が同じ候補はコスト差の小さい方だけ残すので、[`CandidateBranch`] の
    /// 直積より少ない。
    pub candidates: Vec<Candidate<T>>,
}

/// 分岐点ごとに経路を 1 つ選んで解析し直した、1 文ぶんの結果。
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Candidate<T> {
    /// 形態素ごとの音素マッピング。`T` は [`WordPhonemeMap`] / [`WordPhonemeDetail`] /
    /// [`WordPhonemeProsody`] のいずれかで、どの Mapping API を呼んだかで決まる。
    pub words: Vec<T>,

    /// 選んだ経路の [`CandidateAlternative::delta`] の和。
    /// [`Candidates::candidates`] はこの値の昇順に並ぶ。
    ///
    /// MeCab のコストは分割と品詞を決めるための値で、読みの確からしさを測ったもの
    /// ではないため、**FST のアークの重みには使えない**。
    ///
    /// 和が経路コストの差と一致するのは、差し替えたノードの左右の文脈 ID が
    /// 1-best と同じときである。違うときは接続コストが動くので近似になる。
    pub delta: i64,

    /// 分岐点ごとに [`CandidateBranch::alternatives`] の何番目を選んだか。
    /// [`Candidates::branches`] と長さが揃う。
    ///
    /// `0` が 1-best なので、先頭の候補はすべて `0` である。
    pub choices: Vec<usize>,
}

/// 経路が 2 通り以上ある区間。
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CandidateBranch {
    /// 分岐する区間の位置 ([`Candidates::text`] における文字単位、半開区間)。
    ///
    /// 両端は 1-best の形態素の境界に揃う。
    pub char_span: Range<usize>,

    /// 分岐する区間の表層形。
    pub surface: String,

    /// その区間を通る経路。`0` 番目が 1-best で、以降はコスト差の小さい順に並ぶ。
    pub alternatives: Vec<CandidateAlternative>,
}

/// 分岐点の区間を通る経路。
///
/// 分割の違いも候補にするので、[`CandidateAlternative::nodes`] の数は経路ごとに
/// 違う。「彼の」なら `彼` + `の` (カレノ) が 2 個、連体詞 `彼の` (アノ) が 1 個。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CandidateAlternative {
    /// 経路のノード。[`CandidateBranch::char_span`] に隙間なく並ぶ。
    pub nodes: Vec<CandidateReading>,

    /// [`CandidateReading::delta`] の和。1-best の経路は `0`。
    pub delta: i64,
}

impl CandidateAlternative {
    /// 経路のノードの発音を連ねた文字列を返します。
    pub fn pron(&self) -> String {
        self.nodes.iter().map(|n| n.pron.as_str()).collect()
    }
}

/// 経路を組み立てているラティスのノード。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CandidateReading {
    /// 表層形
    pub surface: String,

    /// [`Candidates::text`] における位置 (文字単位、半開区間)
    pub char_span: Range<usize>,

    /// [`CandidateReading::feature`] の pron フィールド
    /// 未知語のノードでは `*` になる。
    ///
    /// `mecab2njd` に渡す前の値である。`predict_nani` や `modify_context_reading` が
    /// 読みを書き換えることがあるので、実際に出る音素は [`Candidate::words`] を見る。
    pub pron: String,

    /// `mecab2njd` に渡す feature 文字列
    ///
    /// [`MecabMorph::feature`] と同じ形で、表層形が先頭に付く。
    pub feature: String,

    /// [`CandidateReading`] を通る最良経路と、文全体の最良経路のコスト差
    ///
    /// 1-best は `0` になる。
    ///
    /// 重みとして使えない理由は [`Candidate::delta`] にある。
    pub delta: i64,

    /// left-id.def で定義された左文脈 ID
    pub left_id: u16,

    /// right-id.def で定義された右文脈 ID
    pub right_id: u16,

    /// 単語コスト
    pub word_cost: i16,

    /// MeCab が未知語 (`MECAB_UNK_NODE`) と判定したかどうか
    pub is_unknown: bool,
}

/// NJD の結果から候補 1 件ぶんの語を組み立て、音素列が同じ候補を見分ける鍵を作る。
///
/// `run_candidates` は [`WordPhonemeMap`] / [`WordPhonemeDetail`] /
/// [`WordPhonemeProsody`] のどれでも同じ手順を踏むので、違うところだけを実装する。
pub(crate) trait CandidateWords: Sized {
    fn build(
        haqumei: &mut Haqumei,
        njd_features: &[NjdFeature],
        morphs: Vec<MecabMorph>,
    ) -> Result<Vec<Self>, HaqumeiError>;

    /// 音素列が同じ候補を見分ける鍵。
    fn dedup_key(words: &[Self]) -> String;
}

impl CandidateWords for WordPhonemeMap {
    fn build(
        haqumei: &mut Haqumei,
        njd_features: &[NjdFeature],
        morphs: Vec<MecabMorph>,
    ) -> Result<Vec<Self>, HaqumeiError> {
        let njd_spans = crate::njd_char_spans(njd_features, &morphs);
        let seeds = haqumei.open_jtalk.g2p_seed_inner(
            njd_features,
            &njd_spans,
            haqumei.options.is_non_pause_symbol,
        )?;
        let mut mapping = haqumei.open_jtalk.make_phoneme_mapping(morphs, seeds)?;
        postprocess::apply_allophones(
            mapping.iter_mut().flat_map(|m| m.phonemes.iter_mut()),
            &haqumei.options,
        );
        Ok(mapping)
    }

    fn dedup_key(words: &[Self]) -> String {
        let mut key = String::new();
        for w in words {
            for p in &w.phonemes {
                key.push_str(p.as_str());
                key.push(' ');
            }
        }
        key
    }
}

impl CandidateWords for WordPhonemeDetail {
    fn build(
        haqumei: &mut Haqumei,
        njd_features: &[NjdFeature],
        morphs: Vec<MecabMorph>,
    ) -> Result<Vec<Self>, HaqumeiError> {
        let njd_spans = crate::njd_char_spans(njd_features, &morphs);
        let mapping = haqumei.open_jtalk.g2p_mapping_inner(
            njd_features,
            &njd_spans,
            haqumei.options.is_non_pause_symbol,
        )?;
        let mut mapping = haqumei.open_jtalk.make_phoneme_mapping(morphs, mapping)?;
        postprocess::apply_allophones(
            mapping.iter_mut().flat_map(|m| m.phonemes.iter_mut()),
            &haqumei.options,
        );
        Ok(mapping)
    }

    fn dedup_key(words: &[Self]) -> String {
        let mut key = String::new();
        for w in words {
            for p in &w.phonemes {
                key.push_str(p.as_str());
                key.push(' ');
            }
        }
        key
    }
}

impl CandidateWords for WordPhonemeProsody {
    fn build(
        haqumei: &mut Haqumei,
        njd_features: &[NjdFeature],
        morphs: Vec<MecabMorph>,
    ) -> Result<Vec<Self>, HaqumeiError> {
        let njd_spans = crate::njd_char_spans(njd_features, &morphs);
        let mapping = haqumei.open_jtalk.g2p_mapping_prosody_inner(
            njd_features,
            &njd_spans,
            haqumei.options.is_non_pause_symbol,
        )?;
        let mut mapping = haqumei.open_jtalk.make_phoneme_mapping(morphs, mapping)?;
        postprocess::apply_allophones_to_prosody(
            mapping.iter_mut().flat_map(|m| m.phonemes.iter_mut()),
            &haqumei.options,
        );
        Ok(mapping)
    }

    /// アクセント核の位置が変われば別の候補なので、ピッチと句境界もキーに入れる。
    fn dedup_key(words: &[Self]) -> String {
        let mut key = String::new();
        for w in words {
            for p in &w.phonemes {
                match p {
                    ProsodicPhoneme::Phoneme { phoneme, pitch } => {
                        key.push_str(phoneme.as_str());
                        match pitch {
                            Some(PitchAccent::High) => key.push('^'),
                            Some(PitchAccent::Low) => key.push('_'),
                            None => {}
                        }
                    }
                    ProsodicPhoneme::AccentPhraseBoundary => key.push('#'),
                    ProsodicPhoneme::Pause => key.push('|'),
                    ProsodicPhoneme::Interrogative => key.push('?'),
                    ProsodicPhoneme::Exclamatory => key.push('!'),
                }
                key.push(' ');
            }
        }
        key
    }
}

/// [`enumerate_combinations`] が `BinaryHeap` に積む要素
struct Combination {
    delta: i64,
    choices: Vec<usize>,
}

impl PartialEq for Combination {
    fn eq(&self, other: &Self) -> bool {
        self.delta == other.delta && self.choices == other.choices
    }
}
impl Eq for Combination {}
impl Ord for Combination {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // BinaryHeap は最大値から出すので、比較を逆にして最小値から取り出す
        other
            .delta
            .cmp(&self.delta)
            .then_with(|| other.choices.cmp(&self.choices))
    }
}
impl PartialOrd for Combination {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// 分岐点ごとの選択を、コスト差の和の小さい順に `limit` 個まで並べる。
///
/// 経路はコスト差の昇順に並んでいるので、分岐点 1 つの選択を 1 つ進めると和は
/// 必ず増える。最小のものから取り出して隣を積めば、直積を作らずに昇順のまま
/// `limit` 個で止められる。
fn enumerate_combinations(branches: &[CandidateBranch], limit: usize) -> Vec<(Vec<usize>, i64)> {
    let start = vec![0usize; branches.len()];
    if limit == 0 {
        return Vec::new();
    }
    let mut heap = BinaryHeap::new();
    let mut seen: HashSet<Vec<usize>> = HashSet::new();
    seen.insert(start.clone());
    heap.push(Combination {
        delta: 0,
        choices: start,
    });

    // `limit` は利用者が決めるので、そのまま確保すると `usize::MAX` で落ちる
    let mut out = Vec::with_capacity(limit.min(1024));
    while let Some(top) = heap.pop() {
        for i in 0..branches.len() {
            let next_choice = top.choices[i] + 1;
            if next_choice >= branches[i].alternatives.len() {
                continue;
            }
            let mut choices = top.choices.clone();
            choices[i] = next_choice;
            if !seen.insert(choices.clone()) {
                continue;
            }
            let delta = top.delta - branches[i].alternatives[top.choices[i]].delta
                + branches[i].alternatives[next_choice].delta;
            heap.push(Combination { delta, choices });
        }
        out.push((top.choices, top.delta));
        if out.len() >= limit {
            break;
        }
    }
    out
}

impl Haqumei {
    /// 読みの候補を、形態素ごとの音素マッピングとして返します。
    ///
    /// 既定の [`CandidateOptions`] を使います。返る [`Candidates::candidates`] は
    /// 空にならず、その先頭は [`Haqumei::g2p_mapping`] の出力と一致します。
    ///
    /// どのような候補が挙げられるかは [モジュールの説明](crate::candidates) にあります。
    ///
    /// # Examples
    ///
    /// ```rust
    /// use haqumei::Haqumei;
    ///
    /// let mut haqumei = Haqumei::new().unwrap();
    /// let got = haqumei.g2p_candidates("彼の話を聞いた。").unwrap();
    ///
    /// // 「彼の」は 彼 + の (カレノ) と 連体詞 彼の (アノ) に分かれる
    /// let branch = &got.branches[0];
    /// assert_eq!(branch.surface, "彼の");
    /// assert_eq!(branch.char_span, 0..2);
    /// assert_eq!(branch.alternatives[0].pron(), "カレノ");
    /// assert_eq!(branch.alternatives[0].nodes.len(), 2);
    /// assert_eq!(branch.alternatives[1].pron(), "アノ");
    /// assert_eq!(branch.alternatives[1].nodes.len(), 1);
    ///
    /// // 先頭は g2p_mapping と同じ
    /// assert_eq!(got.candidates[0].delta, 0);
    /// assert_eq!(got.candidates[0].words[0].word, "彼");
    /// assert_eq!(got.candidates[1].words[0].word, "彼の");
    /// assert_eq!(got.candidates[1].choices, vec![1]);
    /// ```
    pub fn g2p_candidates(
        &mut self,
        text: &str,
    ) -> Result<Candidates<WordPhonemeMap>, HaqumeiError> {
        self.run_candidates(text, CandidateOptions::default())
    }

    /// [`CandidateOptions`] を指定して読みの候補を返します。
    pub fn g2p_candidates_with_options(
        &mut self,
        text: &str,
        options: CandidateOptions,
    ) -> Result<Candidates<WordPhonemeMap>, HaqumeiError> {
        self.run_candidates(text, options)
    }

    /// 読みの候補を、NJD が付与する情報を含めて返します。
    ///
    /// [`Candidates::candidates`] の先頭は [`Haqumei::g2p_mapping_detailed`] の出力と
    /// 一致します。
    ///
    /// # Examples
    ///
    /// ```rust
    /// use haqumei::Haqumei;
    ///
    /// let mut haqumei = Haqumei::new().unwrap();
    /// let got = haqumei.g2p_candidates_detailed("今日は良い天気だ。").unwrap();
    ///
    /// // 候補ごとに、その候補が使った辞書のエントリが入る
    /// assert_eq!(got.candidates[0].words[0].pron, "キョー");
    /// assert_eq!(got.candidates[1].words[0].pron, "コンニチ");
    /// assert_eq!(got.candidates[0].words[0].pos, "名詞");
    /// ```
    pub fn g2p_candidates_detailed(
        &mut self,
        text: &str,
    ) -> Result<Candidates<WordPhonemeDetail>, HaqumeiError> {
        self.run_candidates(text, CandidateOptions::default())
    }

    /// [`CandidateOptions`] を指定して、NJD の情報を含む読みの候補を返します。
    pub fn g2p_candidates_detailed_with_options(
        &mut self,
        text: &str,
        options: CandidateOptions,
    ) -> Result<Candidates<WordPhonemeDetail>, HaqumeiError> {
        self.run_candidates(text, options)
    }

    /// 読みの候補を、プロソディ記号付きの音素として返します。
    ///
    /// 音素が同じでもアクセント核の位置か句の切れ目が違えば、別の候補として残ります。
    /// [`Candidates::candidates`] の先頭は [`Haqumei::g2p_mapping_prosody`] の出力と
    /// 一致します。
    ///
    /// # Examples
    ///
    /// ```rust
    /// use haqumei::{Haqumei, PitchAccent, ProsodicPhoneme};
    ///
    /// let mut haqumei = Haqumei::new().unwrap();
    /// let got = haqumei.g2p_candidates_prosody("今日は良い天気だ。").unwrap();
    ///
    /// // 音素にピッチアクセントが付く
    /// assert!(matches!(
    ///     got.candidates[0].words[0].phonemes[0],
    ///     ProsodicPhoneme::Phoneme { pitch: Some(PitchAccent::High), .. }
    /// ));
    /// assert_eq!(got.candidates[1].words[0].pron, "コンニチ");
    /// ```
    pub fn g2p_candidates_prosody(
        &mut self,
        text: &str,
    ) -> Result<Candidates<WordPhonemeProsody>, HaqumeiError> {
        self.run_candidates(text, CandidateOptions::default())
    }

    /// [`CandidateOptions`] を指定して、プロソディ記号付きの読みの候補を返します。
    pub fn g2p_candidates_prosody_with_options(
        &mut self,
        text: &str,
        options: CandidateOptions,
    ) -> Result<Candidates<WordPhonemeProsody>, HaqumeiError> {
        self.run_candidates(text, options)
    }

    fn run_candidates<T: CandidateWords>(
        &mut self,
        text: &str,
        options: CandidateOptions,
    ) -> Result<Candidates<T>, HaqumeiError> {
        self.open_jtalk.ensure_dictionary_is_latest()?;

        if text.is_empty() {
            return Ok(Candidates {
                text: String::new(),
                branches: Vec::new(),
                candidates: Vec::new(),
            });
        }

        // `char_span` は `text2mecab` を通したバッファを基準に決まるので、
        // [`Candidates::text`] にも同じ文字列を入れないと、切り出した部分文字列が
        // 表層形に戻らない (`ｶﾞム` の `ガム` が `ｶﾞ` になる)。`run_mecab_detailed` と
        // `analyze_lattice` は中でもう一度 `text2mecab` を呼ぶが、出力をもう一度
        // 通しても変わらない
        let text = self.normalize_unicode_if_needed(text);
        let text = self.open_jtalk.text2mecab_string(text.as_ref())?;

        let morphs = self.open_jtalk.run_mecab_detailed(&text)?;
        let nodes = self.open_jtalk.analyze_lattice(&text)?;
        let branches = collect_branches(&morphs, &nodes, &options, &self.options);

        let mut candidates = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();

        for (choices, delta) in
            enumerate_combinations(&branches.public, options.max_candidates.max(1))
        {
            // 分割が変われば形態素の数も変わるので、候補ごとに組み直す。1-best の
            // 列を使い回すと、`make_phoneme_mapping` が読む `feature` と `is_unknown`
            // が 1-best のエントリのままになる
            let cand_morphs = build_morphs(&morphs, &branches, &choices);
            let features: Vec<&str> = cand_morphs
                .iter()
                .filter(|m| !m.is_ignored)
                .map(|m| m.feature.as_str())
                .collect();
            let njd_features = self.open_jtalk.run_njd_from_mecab(&features)?;
            if njd_features.is_empty() {
                continue;
            }

            let protected = if self.options.protect_user_dict_readings {
                protected_indices(&njd_features, &cand_morphs)
            } else {
                HashMap::new()
            };
            let njd_features =
                self.apply_postprocessing(&text, njd_features, &protected, &cand_morphs)?;

            let words = T::build(self, &njd_features, cand_morphs)?;
            if seen.insert(T::dedup_key(&words)) {
                candidates.push(Candidate {
                    words,
                    delta,
                    choices,
                });
            }
        }

        Ok(Candidates {
            text,
            branches: branches.public,
            candidates,
        })
    }
}

/// [`collect_branches`] の返り値。
///
/// [`MecabMorph`] を持つ 2 つのフィールドは公開せず、候補ごとに形態素列を組み直す
/// ためだけに使う。
struct Branches {
    public: Vec<CandidateBranch>,
    /// 分岐点に含まれる 1-best 形態素の添字の範囲
    morph_ranges: Vec<Range<usize>>,
    /// 経路ごとの形態素列。
    /// `morph_ranges` の形態素と入れ替えて使う。
    morphs: Vec<Vec<Vec<MecabMorph>>>,
}

/// 経路が 2 通り以上ある区間を集める。
///
/// 位置 `b` をまたぐノード (`char_span.start < b < char_span.end`) が 1 つも無ければ、
/// どの経路を通っても `b` で形態素が切れる。1-best の経路も例外ではないので、そのような
/// `b` は必ず 1-best の形態素の境界である。
///
/// 隣り合う `b` の間を 1 区間として、その中の経路を独立に数え上げる。区間が形態素
/// 1 つぶんでノードが 1 個のときは、同じ区間にある別のエントリに差し替えるのと同じ
/// 結果になる。
fn collect_branches(
    morphs: &[MecabMorph],
    nodes: &[crate::LatticeNode],
    options: &CandidateOptions,
    haqumei_options: &crate::HaqumeiOptions,
) -> Branches {
    let mut out = Branches {
        public: Vec::new(),
        morph_ranges: Vec::new(),
        morphs: Vec::new(),
    };
    if options.max_alternatives_per_branch == 0 {
        return out;
    }

    // 未知語のノードは発音の列が `*` なので、辞書のエントリと並べると必ず発音が
    // 違うことになる。
    // ラティスから外すと、1-best が未知語の区間も経路が残らず分岐しなくなる
    let kept: Vec<&crate::LatticeNode> = nodes
        .iter()
        .filter(|n| {
            n.delta <= options.max_delta
                && n.char_span.start < n.char_span.end
                && (options.branch_on_unknown_words || column(&n.feature, NODE_PRON_COLUMN) != "*")
        })
        .collect();

    // `is_ignored` な形態素 (記号,空白) に対応するノードはラティスに無いので、
    // またぐと区間に並ぶ経路が見つからない。空白で区切られた 1 つながりごとに
    // 独立して境界を探す
    let mut run_start = 0usize;
    while run_start < morphs.len() {
        if morphs[run_start].is_ignored {
            run_start += 1;
            continue;
        }
        let mut run_end = run_start;
        while run_end < morphs.len() && !morphs[run_end].is_ignored {
            run_end += 1;
        }
        collect_in_run(
            morphs,
            run_start..run_end,
            &kept,
            options,
            haqumei_options,
            &mut out,
        );
        run_start = run_end;
    }
    out
}

/// 空白で区切られた 1 つながりの形態素列について、経路が 2 通り以上ある区間を集める。
fn collect_in_run(
    morphs: &[MecabMorph],
    run: Range<usize>,
    kept: &[&crate::LatticeNode],
    options: &CandidateOptions,
    haqumei_options: &crate::HaqumeiOptions,
    out: &mut Branches,
) {
    // 1-best の形態素の境界のうち、区間の内側にその位置を持つノードが 1 つも無いもの
    let mut bounds: Vec<usize> = Vec::with_capacity(run.len() + 1);
    bounds.push(morphs[run.start].char_span.start);
    for m in &morphs[run.clone()] {
        bounds.push(m.char_span.end);
    }
    let pinches: Vec<usize> = bounds
        .iter()
        .copied()
        .filter(|&b| {
            !kept
                .iter()
                .any(|n| n.char_span.start < b && b < n.char_span.end)
        })
        .collect();
    if pinches.len() < 2 {
        return;
    }

    let mut morph_idx = run.start;
    for w in pinches.windows(2) {
        let (a, b) = (w[0], w[1]);
        // 区間 [a, b) に含まれる 1-best 形態素
        let from = morph_idx;
        while morph_idx < run.end && morphs[morph_idx].char_span.end <= b {
            morph_idx += 1;
        }
        let covered = from..morph_idx;
        if covered.is_empty() {
            continue;
        }
        // ユーザー辞書が与えた読みを守る設定なら、その区間は分岐させない
        if haqumei_options.protect_user_dict_readings
            && morphs[covered.clone()]
                .iter()
                .any(MecabMorph::is_from_user_dictionary)
        {
            continue;
        }

        let paths = paths_in_region(kept, a, b, options);
        if paths.len() < 2 {
            continue;
        }

        // 1-best の経路を先頭に置く。`run_mecab_detailed` が 2 文字以上の記号の
        // 未知語を 1 文字ずつに割った区間では、割ったあとの区間に対応するノードが
        // 無いので 1-best の経路が見つからない。その区間は分岐させない
        let best_spans: Vec<(usize, usize)> = morphs[covered.clone()]
            .iter()
            .map(|m| (m.char_span.start, m.char_span.end))
            .collect();
        let Some(best_at) = paths.iter().position(|p| {
            p.iter()
                .map(|n| (n.char_span.start, n.char_span.end))
                .eq(best_spans.iter().copied())
                && p.iter().all(|n| n.delta == 0)
        }) else {
            continue;
        };

        let mut ordered: Vec<Vec<&crate::LatticeNode>> = Vec::with_capacity(paths.len());
        ordered.push(paths[best_at].clone());
        let base_pron = path_pron(&paths[best_at]);
        let mut seen: HashSet<String> = HashSet::from([base_pron]);
        let mut rest: Vec<Vec<&crate::LatticeNode>> = Vec::new();
        for (i, p) in paths.iter().enumerate() {
            if i == best_at {
                continue;
            }
            if !seen.insert(path_pron(p)) {
                continue;
            }
            rest.push(p.clone());
        }
        if rest.is_empty() {
            continue;
        }
        rest.sort_by_key(|p| (path_delta(p), path_pron(p)));
        rest.truncate(options.max_alternatives_per_branch);
        ordered.extend(rest);

        let alternatives: Vec<CandidateAlternative> = ordered
            .iter()
            .map(|p| CandidateAlternative {
                nodes: p.iter().map(|n| reading_from_node(n)).collect(),
                delta: path_delta(p),
            })
            .collect();
        let surface: String = morphs[covered.clone()]
            .iter()
            .map(|m| m.surface.as_str())
            .collect();

        out.morphs.push(
            ordered
                .iter()
                .map(|p| p.iter().map(|n| morph_from_node(n)).collect())
                .collect(),
        );
        out.morph_ranges.push(covered);
        out.public.push(CandidateBranch {
            char_span: a..b,
            surface,
            alternatives,
        });
    }
}

/// 先頭のノードが `start` から始まり、隣り合うノードの区間が接し、末尾が `end` で
/// 終わる並びをすべて返す。
fn paths_in_region<'a>(
    kept: &[&'a crate::LatticeNode],
    start: usize,
    end: usize,
    options: &CandidateOptions,
) -> Vec<Vec<&'a crate::LatticeNode>> {
    // `max_delta` を効かせているあいだ区間は数文字しかないので、深さ優先で数えて
    // 足りる。`max_delta` を外すと区間が繋がるため、経路の数と探索の歩数の両方に
    // 上限を置く
    let budget = options
        .max_alternatives_per_branch
        .saturating_add(1)
        .saturating_mul(64)
        .max(64);
    let mut out: Vec<Vec<&crate::LatticeNode>> = Vec::new();
    let mut stack: Vec<(usize, Vec<&crate::LatticeNode>)> = vec![(start, Vec::new())];
    let mut steps = 0usize;
    while let Some((at, path)) = stack.pop() {
        steps += 1;
        if steps > budget.saturating_mul(16) {
            break;
        }
        if at == end {
            out.push(path);
            if out.len() >= budget {
                break;
            }
            continue;
        }
        for n in kept {
            if n.char_span.start != at || n.char_span.end > end {
                continue;
            }
            let mut next = path.clone();
            next.push(n);
            stack.push((n.char_span.end, next));
        }
    }
    out
}

fn path_delta(path: &[&crate::LatticeNode]) -> i64 {
    path.iter().map(|n| n.delta).sum()
}

fn path_pron(path: &[&crate::LatticeNode]) -> String {
    path.iter()
        .map(|n| column(&n.feature, NODE_PRON_COLUMN))
        .collect()
}

fn reading_from_node(node: &crate::LatticeNode) -> CandidateReading {
    CandidateReading {
        surface: node.surface.clone(),
        char_span: node.char_span.clone(),
        pron: column(&node.feature, NODE_PRON_COLUMN).to_string(),
        // `mecab2njd` は表層形が先頭に付いた形しか読めない
        feature: format!("{},{}", node.surface, node.feature),
        delta: node.delta,
        left_id: node.left_id,
        right_id: node.right_id,
        word_cost: node.word_cost,
        is_unknown: node.is_unknown,
    }
}

fn morph_from_node(node: &crate::LatticeNode) -> MecabMorph {
    MecabMorph {
        surface: node.surface.clone(),
        feature: format!("{},{}", node.surface, node.feature),
        left_id: node.left_id,
        right_id: node.right_id,
        pos_id: node.pos_id,
        word_cost: node.word_cost,
        is_unknown: node.is_unknown,
        char_span: node.char_span.clone(),
        dictionary_index: node.dictionary_index,
        is_ignored: node.feature.contains("記号,空白"),
    }
}

/// 分岐点ごとに選んだ経路を差し込んだ形態素列を組む。
fn build_morphs(morphs: &[MecabMorph], branches: &Branches, choices: &[usize]) -> Vec<MecabMorph> {
    let mut out = Vec::with_capacity(morphs.len());
    let mut at = 0usize;
    for (bi, range) in branches.morph_ranges.iter().enumerate() {
        out.extend_from_slice(&morphs[at..range.start]);
        out.extend(branches.morphs[bi][choices[bi]].iter().cloned());
        at = range.end;
    }
    out.extend_from_slice(&morphs[at..]);
    out
}

/// feature 文字列の `n` 番目の列を返す。列の数が足りなければ `*`。
fn column(feature: &str, n: usize) -> &str {
    feature.split(',').nth(n).unwrap_or("*")
}
