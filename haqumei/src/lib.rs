mod ffi {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(dead_code)]
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

/// # Safety
///
/// This function is intended to be called from C code via FFI.
/// The caller must ensure that:
/// - `msg` is a valid pointer to a null-terminated C string.
/// - The memory pointed to by `msg` is accessible and not modified concurrently during this call.
/// - `msg` is not null (though the function checks for this, the pointer itself must be valid).
///
/// この関数はFFI経由でC言語のコードから呼び出されることを想定しています。
/// 呼び出し元 (C側のコード) は以下の点を保証する責任があります:
/// - `msg` が有効な、ヌル (`\0`) 終端されたC文字列を指していること。
/// - `msg` が指すメモリ領域が読み取り可能であり、この呼び出し中に他から変更されないこと。
/// - `msg` がダングリングポインタ (無効なメモリを指すポインタ) ではないこと。
#[unsafe(no_mangle)]
unsafe extern "C" fn rust_log_redirect(msg: *const libc::c_char, is_stderr: libc::c_int) { unsafe {
    if msg.is_null() { return; }
    let c_str = std::ffi::CStr::from_ptr(msg);
    let s = c_str.to_string_lossy();
    let s = s.trim_end();

    if is_stderr != 0 {
        log::warn!("[OpenJTalk] {}", s);
    } else {
        log::info!("[OpenJTalk] {}", s);
    }
}}

mod data;
mod errors;
pub mod features;
mod nani_predict;
pub mod open_jtalk;
mod utils;

use std::{path::PathBuf, sync::{LazyLock, Mutex}};

use moka::sync::Cache;

pub use open_jtalk::{OpenJTalk, ParallelJTalk, update_global_dictionary, unset_user_dictionary};
pub use features::NjdFeature;

use vibrato_rkyv::dictionary::PresetDictionaryKind;

use crate::{
    errors::HaqumeiError,
    features::UnidicFeature,
    nani_predict::NaniPredictor,
    utils::{modify_acc_after_chaining, modify_filler_accent, process_odori_features, retreat_acc_nuc, vibrato_analysis},
};

static VIBRATO_CACHE: LazyLock<Cache<String, Vec<UnidicFeature>>> = LazyLock::new(|| Cache::new(1000));
static NANI_PREDICTOR_CACHE: LazyLock<Cache<NjdFeature, bool>> = LazyLock::new(|| Cache::new(1000));
pub static NANI_PREDICTOR: LazyLock<Mutex<NaniPredictor>> = LazyLock::new(|| {
    Mutex::new(NaniPredictor::new().expect("Failed to initialize NaniPredictor models"))
});

#[allow(unused)]
pub struct Haqumei {
    open_jtalk: OpenJTalk,
    tokenizer: vibrato_rkyv::Tokenizer,
    data_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WordPhonemeMap {
    pub word: String,
    pub phonemes: Vec<String>,
}

impl Haqumei {
    pub fn new() -> Result<Self, HaqumeiError> {
        Self::from_open_jtalk(OpenJTalk::new()?)
    }

    #[inline]
    pub fn from_open_jtalk(open_jtalk: OpenJTalk) -> Result<Self, HaqumeiError> {
        let Some(data_dir) = dirs::data_local_dir().map(|dir| dir.join("haqumei")) else {
            Err(HaqumeiError::DataDirectoryNotFound)?
        };

        let vibrato_dict = vibrato_rkyv::Dictionary::from_preset_with_download(
            PresetDictionaryKind::UnidicCsj,
            &data_dir,
        )?;

        let tokenizer = vibrato_rkyv::Tokenizer::new(vibrato_dict);

        Ok(Haqumei {
            open_jtalk,
            data_dir,
            tokenizer,
        })
    }

    /// 入力テキストを音素列 (フラットなリスト) に変換します。
    ///
    /// pyopenjtalk と同様の出力を得るためには、`.join(" ")` をチェーンしてください。
    ///
    /// # Examples
    /// ```rust
    /// use haqumei::Haqumei;
    ///
    /// let mut haqumei = Haqumei::new().unwrap();
    /// // Ok(["k", "o", "N", "n", "i", "ch", "i", "w", "a"])
    /// println!("{:?}", haqumei.g2p("こんにちは"));
    /// ```
    pub fn g2p(&mut self, text: &str) -> Result<Vec<String>, HaqumeiError> {
        let features = self.run_frontend(text)?;

        if features.is_empty() {
            return Ok(Vec::new());
        }

        self.open_jtalk.extract_phonemes(&features)
    }

    /// 入力テキストをカタカナに変換します。
    ///
    /// pyopenjtalk と同様に、記号や未知語などの文字は、元の表記が使用されます。
    pub fn g2p_kana(&mut self, text: &str) -> Result<String, HaqumeiError> {
        let features = self.run_frontend(text)?;

        let kana_string: String = features
            .iter()
            .map(|f| {
                let p = if f.pos == "記号" { &f.string } else { &f.pron };
                p.replace('’', "")
            })
            .collect();

        Ok(kana_string)
    }

    /// 単語（形態素）単位に分割された音素リストを返します。
    ///
    /// # Returns
    ///
    /// 単語ごとの音素リストのベクタ。
    ///
    /// (e.g., [["k", "o", "N", "n", "i", "ch", "i", "w", "a"], ["pau"], ["s", "e", "k", "a", "i"]])
    pub fn g2p_per_word(
        &mut self,
        text: &str,
    ) -> Result<Vec<Vec<String>>, HaqumeiError> {
        let features = self.run_frontend(text)?;

        if features.is_empty() {
            return Ok(Vec::new());
        }

        self.open_jtalk.extract_phonemes_per_word(&features)
    }

    /// 入力テキストの形態素ごとの音素マッピングを返します。
    ///
    /// MeCab による形態素解析の結果と 1:1 に対応するマッピング情報を生成します。
    ///
    /// **記号・未知語の処理**: 読点 (`、`) や未知語など、OpenJTalk が発音を生成しないトークンに対しては、
    ///   音素リストとして `["pau"]` が割り当てられます。
    ///
    /// # Examples
    ///
    /// ```rust
    /// use haqumei::Haqumei;
    ///
    /// let mut haqumei = Haqumei::new().unwrap();
    /// let mapping = haqumei.g2p_mapping("𰻞𰻞麺＆お冷を頼んだ").unwrap();
    ///
    /// // 出力:
    /// // [WordPhonemeMap {
    /// //     word: "𰻞𰻞",
    /// //     phonemes: ["pau"]
    /// // }, WordPhonemeMap {
    /// //     word: "麺",
    /// //     phonemes: ["m", "e", "N"]
    /// // }, WordPhonemeMap {
    /// //     word: "＆",
    /// //     phonemes: ["a", "N", "d", "o"]
    /// // }, WordPhonemeMap {
    /// //     word: "お冷",
    /// //     phonemes: ["o", "h", "i", "y", "a"]
    /// // }, WordPhonemeMap {
    /// //     word: "を",
    /// //     phonemes: ["o"]
    /// // }, WordPhonemeMap {
    /// //     word: "頼ん",
    /// //     phonemes: ["t", "a", "n", "o", "N"]
    /// // }, WordPhonemeMap {
    /// //     word: "だ",
    /// //     phonemes: ["d", "a"]
    /// // }]
    /// // ```
    pub fn g2p_mapping(
        &mut self,
        text: &str,
    ) -> Result<Vec<WordPhonemeMap>, HaqumeiError> {
        let features = self.run_frontend(text)?;

        if features.is_empty() {
            return Ok(Vec::new());
        }

        self.open_jtalk.g2p_mapping_inner(&features)
    }

    pub fn run_frontend(
        &mut self,
        text: &str,
    ) -> Result<Vec<NjdFeature>, HaqumeiError> {
        let (njd_features, _) = rayon::join(
            || OpenJTalk::new()?.run_frontend(text),
            || {
            let mut worker = self.tokenizer.new_worker();
            vibrato_analysis(&mut worker, text);
        });
        self.apply_postprocessing(text, njd_features?)
    }

    pub fn extract_fullcontext(
        &mut self,
        text: &str,
    ) -> Result<Vec<String>, HaqumeiError> {
        let njd_features = self.run_frontend(text)?;
        self.open_jtalk.make_label(&njd_features)
    }

    fn apply_postprocessing(
        &mut self,
        text: &str,
        mut njd_features: Vec<NjdFeature>,
    ) -> Result<Vec<NjdFeature>, HaqumeiError> {
        modify_filler_accent(&mut njd_features);
        self.modify_kanji_yomi(text, &mut njd_features);
        retreat_acc_nuc(&mut njd_features);
        modify_acc_after_chaining(&mut njd_features);
        process_odori_features(&mut njd_features, &mut self.open_jtalk)?;
        Ok(njd_features)
    }

    pub(crate) fn predict_is_nan(&mut self, prev_node: Option<&NjdFeature>) -> bool {
        let prev_node = match prev_node {
            Some(node) => node,
            None => return false,
        };

        NANI_PREDICTOR_CACHE.get_with(prev_node.clone(), || {
            NANI_PREDICTOR.lock().unwrap().predict_is_nan(Some(prev_node))
        })
    }
}
