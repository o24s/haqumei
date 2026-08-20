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
//! MeCab は前向きの累積コストを解析の過程で全ノードに書き込んでいるので、
//! 後ろ向きを 1 回走らせるだけで文に現れる全候補の `delta` が同時に出る。
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

use std::ffi::{CStr, CString};
use std::ops::Range;

#[cfg(doc)]
use crate::MecabMorph;
use crate::cursor::CharCursor;
use crate::errors::HaqumeiError;
use crate::ffi;
use crate::open_jtalk::OpenJTalk;

/// ラティス上の候補ノード 1つ。
///
/// 最良経路に選ばれなかったものも含めて、その位置に立ちうる候補がすべて出る。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LatticeNode {
    /// 候補の表層形。
    pub surface: String,

    /// MeCab が出力した特徴量文字列。
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
    /// MeCab のラティスを、候補ごとの経路コスト差とともに返します。
    ///
    /// 最良経路に選ばれなかった候補も含めてすべて返します。各候補の
    /// [`LatticeNode::delta`] は「その候補を通る最良経路」と「文全体の最良経路」の
    /// コスト差で、辞書の単語コストをいくつ下げればその候補が選ばれるようになるかを
    /// 表します。
    pub fn analyze_lattice(&mut self, text: &str) -> Result<Vec<LatticeNode>, HaqumeiError> {
        self.ensure_dictionary_is_latest()?;

        let c_text = CString::new(text)?;
        let mut buffer = vec![0u8; Self::BUFFER_SIZE];

        let result = unsafe {
            ffi::text2mecab(
                buffer.as_mut_ptr() as *mut _,
                Self::BUFFER_SIZE,
                c_text.as_ptr(),
            )
        };
        if result != ffi::text2mecab_result_t_TEXT2MECAB_RESULT_SUCCESS {
            return Err(HaqumeiError::Text2MecabError(format!(
                "text2mecab failed: {result}"
            )));
        }

        // 候補どうしを繋ぐ辺 (`mecab_path_t`) は、要求種別に MECAB_NBEST か
        // MECAB_MARGINAL_PROB が立っているときにしか作られない。
        // (viterbi.cpp の Viterbi::analyze が IsAllPath を決める分岐)
        // 辺が無いと後ろ向きのコストを求められないので、この解析の間だけ立てる。
        // MECAB_ALL_MORPHS ではない点に注意しなければならず、`node.next` が
        // ラティス全体を辿るかどうかを決めるだけで、辺は作られない。
        //
        // MECAB_MARGINAL_PROB は毎回 forward-backward を回して周辺確率まで出すが、
        // ここでは使わないので、生成器を用意するだけの MECAB_NBEST を選ぶ。
        // 最良経路の組み立ては MECAB_ONE_BEST の側なので両方立てる。
        //
        // `Mecab_refresh` (lattice->clear()) は要求種別を戻さないので、ここで必ず
        // 戻す。立てたままにすると以降の解析の挙動が変わる。
        let lattice = unsafe { (*self.mecab.inner.as_ptr()).lattice as *mut ffi::mecab_lattice_t };
        let previous_request = unsafe { ffi::mecab_lattice_get_request_type(lattice) };
        unsafe {
            ffi::mecab_lattice_set_request_type(
                lattice,
                (ffi::MECAB_ONE_BEST | ffi::MECAB_NBEST) as i32,
            );
        }

        let analysed =
            unsafe { ffi::Mecab_analysis(self.mecab.inner.as_ptr(), buffer.as_ptr() as *const _) };
        let nodes = if analysed == 1 {
            Some(unsafe { self.collect_lattice(buffer.as_ptr()) })
        } else {
            None
        };

        unsafe {
            ffi::Mecab_refresh(self.mecab.inner.as_ptr());
            ffi::mecab_lattice_set_request_type(lattice, previous_request);
        }

        nodes.ok_or_else(|| {
            HaqumeiError::MecabError("Mecab_analysis failed to parse the text".to_string())
        })
    }

    /// 解析済みのラティスを走査して [`LatticeNode`] を組み立てる。
    ///
    /// # Safety
    ///
    /// 直前に `Mecab_analysis` が成功していること。`sentence` は解析に渡した
    /// バッファの先頭で、ラティスが指す領域と同じでなければならない。
    unsafe fn collect_lattice(&mut self, sentence: *const u8) -> Vec<LatticeNode> {
        let lattice = unsafe { (*self.mecab.inner.as_ptr()).lattice as *mut ffi::mecab_lattice_t };
        let size = unsafe { ffi::mecab_lattice_get_size(lattice) };
        let eos = unsafe { ffi::mecab_lattice_get_eos_node(lattice) };

        // `size` は解析対象のバイト長。ノードは同じ位置から複数始まるので、
        // 前にも後ろにも進めるカーソルで位置を文字単位に直す
        let bytes_len = size;
        let mut cursor =
            CharCursor::new(unsafe { std::slice::from_raw_parts(sentence, bytes_len) });

        // 後ろ向きの累積コスト。EOS から見て、そのノードの右側だけを足した値。
        //
        // `mecab_path_t::cost` は接続コストと右側ノードの単語コストの両方を含む
        // (MeCab の connector->cost() 由来) ので、右側のノードの単語コストも
        // この値に含まれる。
        // 前向き (`node.cost`) は MeCab が解析中に全ノードへ書き込んでいるため、
        // 足し合わせても二重にならない。
        //
        // ノードの id は 1 文ごとに 0 から連番で振られ (`Allocator::newNode` が
        // `id_++`)、`clear()` で戻るので密になる。そのまま添字に使えば、ポインタを
        // 鍵にしたハッシュ表より速く、確保も 1 回で済む。
        let mut backward: Vec<i64> = Vec::new();
        set_backward(&mut backward, unsafe { (*eos).id } as usize, 0);

        // ある位置に始まるノードの右隣は必ず後ろの位置に始まるので、位置を後ろから
        // 前へ見ていけば、右側は先に埋まっている。
        for pos in (0..=size).rev() {
            let mut node = unsafe { ffi::mecab_lattice_get_begin_nodes(lattice, pos) };
            while !node.is_null() {
                let mut best: Option<i64> = None;
                let mut path = unsafe { (*node).rpath };
                while !path.is_null() {
                    let rnode = unsafe { (*path).rnode };
                    if let Some(rest) = get_backward(&backward, unsafe { (*rnode).id } as usize) {
                        let total = i64::from(unsafe { (*path).cost }) + rest;
                        best = Some(best.map_or(total, |b: i64| b.min(total)));
                    }
                    path = unsafe { (*path).rnext };
                }
                if let Some(best) = best {
                    set_backward(&mut backward, unsafe { (*node).id } as usize, best);
                }
                node = unsafe { (*node).bnext };
            }
        }

        // `node.cost` は `c_long` で、Windows では 32 ビット、Unix では 64 ビットに
        // なる。
        #[allow(clippy::useless_conversion)]
        let total = i64::from(unsafe { (*eos).cost });
        let mut out = Vec::new();

        for pos in 0..=size {
            let mut node = unsafe { ffi::mecab_lattice_get_begin_nodes(lattice, pos) };
            while !node.is_null() {
                let stat = unsafe { (*node).stat };
                // 2 = BOS, 3 = EOS
                if stat == 2 || stat == 3 {
                    node = unsafe { (*node).bnext };
                    continue;
                }
                let Some(rest) = get_backward(&backward, unsafe { (*node).id } as usize) else {
                    // EOS へ到達できないノード。経路を作れないので飛ばす
                    node = unsafe { (*node).bnext };
                    continue;
                };

                let length = unsafe { (*node).length } as usize;
                let surface_ptr = unsafe { (*node).surface };
                let surface = if surface_ptr.is_null() || length == 0 {
                    String::new()
                } else {
                    let bytes =
                        unsafe { std::slice::from_raw_parts(surface_ptr as *const u8, length) };
                    String::from_utf8_lossy(bytes).into_owned()
                };
                let feat_ptr = unsafe { (*node).feature };
                let feature = if feat_ptr.is_null() {
                    String::new()
                } else {
                    unsafe { CStr::from_ptr(feat_ptr) }
                        .to_string_lossy()
                        .into_owned()
                };

                // sentence と同じバッファを指すノードだけ位置に直す。
                // 外部で確保されたノードの surface はこの範囲の外を指しうる
                let char_span = if !surface_ptr.is_null()
                    && (surface_ptr as usize) >= sentence as usize
                    && (surface_ptr as usize) - sentence as usize + length <= bytes_len
                {
                    let byte_start = (surface_ptr as usize) - sentence as usize;
                    cursor.char_at(byte_start)..cursor.char_at(byte_start + length)
                } else {
                    0..0
                };
                #[allow(clippy::useless_conversion)]
                let delta = i64::from(unsafe { (*node).cost }) + rest - total;

                out.push(LatticeNode {
                    surface,
                    feature,
                    char_span,
                    left_id: unsafe { (*node).lcAttr },
                    right_id: unsafe { (*node).rcAttr },
                    pos_id: unsafe { (*node).posid },
                    word_cost: unsafe { (*node).wcost },
                    is_unknown: stat == 1,
                    dictionary_index: unsafe { (*node).dictionary_index },
                    delta,
                    is_best: delta == 0,
                });

                node = unsafe { (*node).bnext };
            }
        }

        out
    }
}

/// 後ろ向きの累積コストを置く場所が無ければ広げる。
///
/// 最初に入れるのは EOS で、その id は 1 文の中で最大になる。そこで必要な
/// 大きさになるので、以降は広げ直さない。
fn set_backward(backward: &mut Vec<i64>, id: usize, value: i64) {
    if backward.len() <= id {
        backward.resize(id + 1, UNREACHED);
    }
    backward[id] = value;
}

/// 後ろ向きの累積コストを引く。EOS へ到達できないノードは `None`。
fn get_backward(backward: &[i64], id: usize) -> Option<i64> {
    match backward.get(id) {
        Some(&v) if v != UNREACHED => Some(v),
        _ => None,
    }
}

/// EOS へ到達できないことを表す値。
///
/// 走査の途中はまだ決まっていないノードも同じ値だが、走査を終えた時点で残って
/// いるものは、そこから EOS までの経路が無いノードである。
const UNREACHED: i64 = i64::MAX;
