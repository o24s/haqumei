mod ffi {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(dead_code)]
    #![allow(clippy::upper_case_acronyms)]
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

unsafe extern "C" {
    fn setup_cpp_redirect();
    fn teardown_cpp_redirect();
}

/// This function is intended to be called from C code via FFI.
///
/// # Safety
///
/// The caller must ensure that:
/// - `msg` is a valid pointer to a null-terminated C string.
/// - The memory pointed to by `msg` is accessible and not modified concurrently during this call.
#[unsafe(no_mangle)]
unsafe extern "C" fn haqumei_rust_print(msg: *const libc::c_char, is_stderr: libc::c_int) {
    unsafe {
        if msg.is_null() {
            return;
        }
        let c_str = std::ffi::CStr::from_ptr(msg);
        let s = c_str.to_string_lossy();
        let s = s.trim_end();

        if is_stderr != 0 {
            log::warn!("[OpenJTalk] {}", s);
        } else {
            log::info!("[OpenJTalk] {}", s);
        }
    }
}

mod data;
pub mod errors;
pub mod features;
#[macro_use]
mod macros;
pub mod nani_predict;
pub mod open_jtalk;
pub mod phoneme;
mod postprocess;
pub mod prosody;
pub mod utils;
pub mod word_phoneme;

use std::{
    path::{Path, PathBuf},
    sync::{Arc, LazyLock, Mutex, OnceLock},
    thread,
};

use crossbeam_channel::{Sender, bounded};
use haqumei_jlabel::Label;
use moka::sync::Cache;

pub use features::NjdFeature;
pub use open_jtalk::{
    MecabDictIndexCompiler, MecabMorph, OpenJTalk, unset_user_dictionary, update_global_dictionary,
};
pub use phoneme::Phoneme;
pub use prosody::{PitchAccent, ProsodicPhoneme, ProsodyFormat};
pub use word_phoneme::{WordPhonemeDetail, WordPhonemeMap, WordPhonemePair};

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use vibrato_rkyv::dictionary::PresetDictionaryKind;

use crate::{
    errors::HaqumeiError,
    features::UnidicFeature,
    nani_predict::NaniPredictor,
    open_jtalk::{Dictionary, GLOBAL_MECAB_DICTIONARY},
    postprocess::{
        modify_acc_after_chaining, modify_filler_accent, process_odori_features, retreat_acc_nuc,
        vibrato_analysis,
    },
    utils::default_is_non_pause_symbol,
    word_phoneme::WordPhonemeProsody,
};

static VIBRATO_CACHE: LazyLock<Cache<String, Vec<UnidicFeature>>> =
    LazyLock::new(|| Cache::new(1000));
static NANI_PREDICTOR_CACHE: LazyLock<Cache<NjdFeature, bool>> = LazyLock::new(|| Cache::new(1000));
static NANI_PREDICTOR: LazyLock<Mutex<NaniPredictor>> = LazyLock::new(|| {
    Mutex::new(NaniPredictor::new().expect("Failed to initialize NaniPredictor models"))
});
static CACHE_DIR: OnceLock<PathBuf> = OnceLock::new();

type VibratoTask = (String, Sender<Vec<UnidicFeature>>);
static VIBRATO_TASK_TX: OnceLock<Sender<VibratoTask>> = OnceLock::new();

pub(crate) fn init_vibrato_workers_if_needed(tokenizer: &vibrato_rkyv::Tokenizer) {
    VIBRATO_TASK_TX.get_or_init(|| {
        let (tx, rx) = bounded::<VibratoTask>(1024);
        let worker_count = 8;

        for _ in 0..worker_count {
            let rx = rx.clone();
            let tokenizer = tokenizer.clone();
            thread::spawn(move || {
                let mut worker = tokenizer.new_worker();
                while let Ok((text, res_tx)) = rx.recv() {
                    let features = vibrato_analysis(&mut worker, &text);
                    let _ = res_tx.send(features);
                }
            });
        }
        tx
    });
}

/// Open JTalk をバインディングした G2P エンジン。
///
/// [`pyopenjtalk-plus`](https://github.com/tsukumijima/pyopenjtalk-plus) の辞書を使用しています。
///
/// [Haqumei::with_options], [HaqumeiOptions] を使うことで、出力をカスタマイズできます。
pub struct Haqumei {
    pub(crate) open_jtalk: OpenJTalk,
    pub(crate) tokenizer: Option<vibrato_rkyv::Tokenizer>,
    pub(crate) rx: Option<crossbeam_channel::Receiver<Vec<UnidicFeature>>>,
    pub options: HaqumeiOptions,
}

/// `Haqumei` の設定。
/// 詳細は、それぞれのフィールドのドキュメントを見てください。
#[derive(Debug, Clone, Copy)]
pub struct HaqumeiOptions {
    /// 入力テキストを [UnicodeNormalization] の指定された方法で正規化する。
    /// 「か + 濁点」などの結合文字を1文字の「が」に統合できます。
    ///
    /// デフォルトで無効になっています。
    pub normalize_unicode: UnicodeNormalization,

    /// この値が true の場合、発音表記 (`pron`) が文字表記 (`read`) によって上書きされます。
    ///
    /// これにより、長音の自動変換機能が無効化されます。 (e.g., "ジンセー" -> "ジンセイ")
    /// なお、助詞にもこの影響が及び、"は" は「ワ」ではなく「ハ」として、
    /// "へ" は「エ」ではなく「ヘ」として発音されます。
    ///
    /// すなわち、これを有効にした場合、`revert_long_vowels`, `revert_yotsugana` のフラグに関係なく、
    /// 読み (`read`) に強制的に置き換えられます。
    ///
    /// デフォルトで無効になっています。
    pub use_read_as_pron: bool,

    /// 辞書によって自動的に長音化された発音を、元のテキストに忠実な読みに復元するかどうか。
    ///
    /// `true` に設定すると、発音 (`pron`) に「ー」が含まれている単語について、
    /// 元のテキスト (`orig`) に「ー」が含まれていない場合のみ、発音を読み (`read`) の値で上書きします。
    /// (e.g., 「効果」 pron: コーカ -> コウカ / 「人生」 pron: ジンセー -> ジンセイ)
    ///
    /// 助詞 (は、へ、を) などの発音は「ー」を含まないため影響を受けず、
    /// そのまま音声合成に適した発音 (ワ、エ、オ) が維持されます。
    ///
    /// デフォルトで無効になっています。
    pub revert_long_vowels: bool,

    /// 現代仮名遣いにおいて発音上統合される四つ仮名 (ヅ・ヂ) を、
    /// 元のテキスト通りの表記に復元するかどうか。
    ///
    /// `true` に設定すると、発音 (`pron`) において「ズ」「ジ」に変換されたものを、
    /// 読み (`read`) に基づいて「ヅ」「ヂ」に復元します。
    /// (e.g., 「気づかず」 pron: キズカズ -> キヅカズ / 「鼻血」 pron: ハナジ -> ハナヂ)
    ///
    /// デフォルトで無効になっています。
    pub revert_yotsugana: bool,

    /// 形態素解析辞書の仕様により「イウ」と「ユウ」のどちらに解析されるか不定な
    /// 動詞「言う」 (およびその活用形や複合語) の読み・発音を、指定した方に強制的に統一します。
    ///
    /// 辞書には「言う」に対して「イウ」「ユウ」の両方が登録されており、
    /// 形態素解析のコスト計算によって出力が変動します。
    /// `Some` を指定することで、解析結果に関わらず意図した発音に固定できます。
    ///
    /// デフォルトは `None` (形態素解析辞書の出力結果をそのまま使用する) です。
    pub normalize_iu: Option<IuPronunciation>,

    /// - フィラーが acc > mora_size のときに、平版型 (acc = 0) にする
    /// - フィラー直後の形態素が名詞だったとき、その前のフィラーに結合しない (chain_flag = 0) ようにする
    ///
    /// デフォルトで有効になっています。
    pub modify_filler_accent: bool,

    /// Nani Predictor を使って、「何」 の読みを修正する。
    ///
    /// デフォルトで有効になっています。
    pub predict_nani: bool,

    /// Unidic を使って、漢字の読みを修正する。
    /// 有効にした初回実行時には、辞書のダウンロードが発生します。
    ///
    /// デフォルトで無効になっています。
    pub use_unidic_yomi: bool,

    /// 長母音、重母音、撥音がアクセント核に来た場合に、
    /// ひとつ前のモーラにアクセント核がズレるルールを適用する。
    ///
    /// デフォルトで有効になっています。
    pub retreat_acc_nuc: bool,

    /// 品詞「特殊・マス」の直前に接続する動詞にアクセント核がある場合、アクセント核を「ま」に移動させる。
    ///
    ///   書きます -> か\[きま\]す, 参ります -> ま\[いりま\]す
    ///   書いております -> \[か\]いております
    ///
    /// デフォルトで有効になっています。
    pub modify_acc_after_chaining: bool,

    /// 踊り字 (e.g., 々, ヽ, ヾ) の展開を有効にする。
    ///
    /// デフォルトで有効になっています。
    pub process_odoriji: bool,

    /// `*_detailed` API において、記号に対して `pau` を割り当てるべきか判定する関数を設定するフィールド。
    ///
    /// `true` を返した記号には `pau` が付与されません。
    /// 閉じ括弧に `pau` を割り当てたくないようなケースに使ってください。
    ///
    /// デフォルトでは、以下の表層系に `pau` が割り当てられません。
    /// `「` , `」` , `『` , `』` , ` (` , `) ` , `(` , `)` ,
    /// `【` , `】` , `［` , `］` , `[` , `]` , `〈` , `〉` ,
    /// `《` , `》` , `〔` , `〕` , `｛` , `｝` , `{` , `}` ,
    /// `"` , `'` , `”` , `“` , `’` , `‘`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use haqumei::utils::default_is_non_pause_symbol;
    ///
    /// fn my_custom_pause_rule(s: &str) -> bool {
    ///     if s == "「" {
    ///         return false; // false を返すと pau が割り当てられる
    ///     }
    ///     // それ以外はデフォルトの挙動を継承
    ///     default_is_non_pause_symbol(s)
    /// }
    /// ```
    pub is_non_pause_symbol: fn(&str) -> bool,
}

impl Default for HaqumeiOptions {
    fn default() -> Self {
        Self {
            normalize_unicode: UnicodeNormalization::None,
            use_read_as_pron: false,
            revert_long_vowels: false,
            revert_yotsugana: false,
            normalize_iu: None,
            modify_filler_accent: true,
            predict_nani: true,
            use_unidic_yomi: false,
            retreat_acc_nuc: true,
            modify_acc_after_chaining: true,
            process_odoriji: true,
            is_non_pause_symbol: default_is_non_pause_symbol,
        }
    }
}

/// 入力テキストをどのように正規化するかを指定します。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnicodeNormalization {
    /// 正規化を行わない (デフォルト)
    #[default]
    None,
    /// NFC (正準等価性による合成: 結合文字の合体のみ)
    Nfc,
    /// NFKC (互換等価性による分解と合成: 半角カナ -> 全角カナ、全角英数 -> 半角英数など)
    Nfkc,
}

/// 動詞「言う」およびその派生語の発音・読みをどのように正規化するかを指定します。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IuPronunciation {
    /// すべての「言う」「いう」を「イウ」に統一します。
    Iu,
    /// すべての「言う」「いう」を「ユウ」に統一します。
    Yuu,
    /// 漢字表記 (`言う`, `云う`) が含まれる場合のみ「イウ」に統一し、
    /// 平仮名表記 (`いう`, `そういう`) は辞書の解析結果をそのまま使用します。
    KanjiIu,
    /// 漢字表記 (`言う`, `云う`) が含まれる場合のみ「ユウ」に統一し、
    /// 平仮名表記 (`いう`, `そういう`) は辞書の解析結果をそのまま使用します。
    KanjiYuu,
}

impl Haqumei {
    /// [Haqumei] を生成します。
    pub fn new() -> Result<Self, HaqumeiError> {
        Self::from_open_jtalk(OpenJTalk::new()?, HaqumeiOptions::default())
    }

    /// [HaqumeiOptions] を使って、出力をカスタマイズします。
    pub fn with_options(options: HaqumeiOptions) -> Result<Self, HaqumeiError> {
        Self::from_open_jtalk(OpenJTalk::new()?, options)
    }

    #[inline]
    /// [OpenJTalk] から [Haqumei] を生成します。
    pub fn from_open_jtalk(
        open_jtalk: OpenJTalk,
        options: HaqumeiOptions,
    ) -> Result<Self, HaqumeiError> {
        let mut haqumei = Haqumei {
            open_jtalk,
            tokenizer: None,
            rx: None,
            options,
        };

        if options.use_unidic_yomi {
            haqumei.init_tokenizer_if_needed()?;
        }

        Ok(haqumei)
    }

    pub(crate) fn init_tokenizer_if_needed(&mut self) -> Result<(), HaqumeiError> {
        if self.tokenizer.is_some() {
            return Ok(());
        }

        if CACHE_DIR.get().is_none() {
            let base = dirs::cache_dir().ok_or(HaqumeiError::CacheDirectoryNotFound)?;
            CACHE_DIR.get_or_init(|| base.join("haqumei"));
        }
        let cache_dir = CACHE_DIR.get().unwrap();

        let kind = PresetDictionaryKind::UnidicCsj;
        log::info!("Downloading {} dictionary...", kind.name());
        let vibrato_dict =
            vibrato_rkyv::Dictionary::from_preset_with_download(kind, cache_dir.join(kind.name()))?;
        log::info!("Downloaded {} dictionary.", kind.name());

        self.tokenizer = Some(vibrato_rkyv::Tokenizer::new(vibrato_dict));

        Ok(())
    }

    pub(crate) fn init_tokenizer_if_needed_and_modify_kanji_yomi_enabled(
        &mut self,
    ) -> Result<Option<vibrato_rkyv::Tokenizer>, HaqumeiError> {
        if self.options.use_unidic_yomi {
            self.init_tokenizer_if_needed()?;
            Ok(self.tokenizer.clone()) // かなり無料
        } else {
            Ok(None)
        }
    }

    /// [open_jtalk::Dictionary] から [Haqumei] を作ります。
    pub fn from_dictionary(
        dict: Dictionary,
        options: HaqumeiOptions,
    ) -> Result<Self, HaqumeiError> {
        Self::from_open_jtalk(OpenJTalk::from_dictionary(dict)?, options)
    }

    /// `Arc` でラップされた [Dictionary] から [Haqumei] を作ります
    pub fn from_shared_dictionary(
        dict: Arc<Dictionary>,
        options: HaqumeiOptions,
    ) -> Result<Self, HaqumeiError> {
        Self::from_open_jtalk(OpenJTalk::from_shared_dictionary(dict)?, options)
    }

    /// 辞書パスから [Haqumei] を生成します。
    pub fn from_path<P: AsRef<Path>>(
        dict_dir: P,
        options: HaqumeiOptions,
    ) -> Result<Self, HaqumeiError> {
        Self::from_open_jtalk(OpenJTalk::from_path(dict_dir)?, options)
    }

    /// システム辞書パスとユーザー辞書パスから [Haqumei] を生成します。
    pub fn from_path_with_userdict<P: AsRef<Path>, Q: AsRef<Path>>(
        dict_dir: P,
        user_dict: Q,
        options: HaqumeiOptions,
    ) -> Result<Self, HaqumeiError> {
        Self::from_open_jtalk(
            OpenJTalk::from_path_with_userdict(dict_dir, user_dict)?,
            options,
        )
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
    pub fn g2p(&mut self, text: &str) -> Result<Vec<Phoneme>, HaqumeiError> {
        if text.is_empty() {
            self.open_jtalk.ensure_dictionary_is_latest()?;
            return Ok(Vec::new());
        }

        let features = self.run_frontend(text)?;

        if features.is_empty() {
            return Ok(Vec::new());
        }

        self.open_jtalk.extract_phonemes(&features)
    }

    /// すべてのトークンを保持する詳細な G2P 変換。
    ///
    /// - 既知語: 通常の音素列 (読点などは `pau`)
    /// - 未知語: `unk`
    /// - 空白等: `sp` (Space)
    ///
    /// pyopenjtalk のような音素文字列を得るためには、`.join(" ")` をチェーンしてください。
    ///
    /// # Examples
    /// ```rust
    /// use haqumei::Haqumei;
    ///
    /// let mut haqumei = Haqumei::new().unwrap();
    /// // Ok(["k", "o", "N", "n", "i", "ch", "i", "w", "a", "sp", "unk", "m", "e", "N"])
    /// println!("{:?}", haqumei.g2p_detailed("こんにちは 𰻞𰻞麺"));
    /// ```
    pub fn g2p_detailed(&mut self, text: &str) -> Result<Vec<Phoneme>, HaqumeiError> {
        if text.is_empty() {
            self.open_jtalk.ensure_dictionary_is_latest()?;
            return Ok(Vec::new());
        }

        let detailed_mapping = self.g2p_mapping(text)?;

        let mut result_phonemes = Vec::new();
        for map in detailed_mapping {
            result_phonemes.extend(map.phonemes);
        }

        Ok(result_phonemes)
    }

    /// 入力テキストをカタカナに変換します。
    ///
    /// pyopenjtalk と同様に、記号や未知語などの文字は、元の表記が使用されます。
    pub fn g2p_kana(&mut self, text: &str) -> Result<String, HaqumeiError> {
        if text.is_empty() {
            self.open_jtalk.ensure_dictionary_is_latest()?;
            return Ok(String::new());
        }

        let features = self.run_frontend(text.as_ref())?;

        let kana_string: String = features
            .iter()
            .map(|f| {
                let p = if f.pos == "記号" {
                    &f.string
                } else {
                    &f.pron
                };
                p.replace('’', "")
            })
            .collect();

        Ok(kana_string)
    }

    /// 入力テキストを単語 (形態素) ごとのカタカナリストに変換します。
    pub fn g2p_kana_per_word(&mut self, text: &str) -> Result<Vec<String>, HaqumeiError> {
        if text.is_empty() {
            self.open_jtalk.ensure_dictionary_is_latest()?;
            return Ok(Vec::new());
        }

        let features = self.run_frontend(text.as_ref())?;

        let kana_list: Vec<String> = features
            .iter()
            .map(|f| {
                let p = if f.pos == "記号" {
                    &f.string
                } else {
                    &f.pron
                };
                p.replace('’', "")
            })
            .collect();

        Ok(kana_list)
    }

    /// 入力テキストをプロソディ記号付き音素リストに変換します。
    ///
    /// 音素ごとにピッチ情報が欲しい場合は、[Haqumei::g2p_prosody_with_options] を使用してください。
    /// この関数は、[Haqumei::g2p_prosody_with_options] で [ProsodyFormat::Default] を選択したときの動作に相当します。
    ///
    /// 出力には通常の音素に加えて、以下の制御記号が含まれます:
    ///
    /// | 記号 | 意味 | 出現位置 |
    /// | :--- | :--- | :--- |
    /// | `^` | 発話の開始 (BOS) | 文頭 |
    /// | `$` | 発話の終結 (EOS) | 文末 |
    /// | `?` | 疑問文の終結 (？) | 文中 |
    /// | `!` | 感嘆の終結 (独自拡張) | 文中 |
    /// | `_` | ポーズ・読点 (、) | 文中 |
    /// | `#` | アクセント句境界 | 文中 |
    /// | `[` | ピッチ上昇 (句頭) | 句の開始付近 |
    /// | `]` | ピッチ下降 (アクセント核) | 核モーラの直後 |
    /// | `{...}` | 未知語 | 文中 |
    ///
    /// 記号 `[` および `]` は、tdmelodic 等で一般的なアクセント記法に基づいています。
    /// "Prosodic Features Control by Symbols as Input of Sequence-to-Sequence Acoustic Modeling for Neural TTS"
    /// (Kurihara et al., 2021) のアルゴリズムにおける `^` および `!` に相当します。
    ///
    /// 日本語のアクセントについて: [tdmelodic 利用マニュアル/予備知識](https://tdmelodic.readthedocs.io/ja/latest/pages/introduction.html)
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use haqumei::Haqumei;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut haqumei = Haqumei::new()?;
    ///
    /// let phones = haqumei.g2p_prosody("こんにちは、世界！")?;
    /// assert_eq!(phones.join(" "), "^ k o [ N n i ch i w a _ s e ] k a i ! $");
    ///
    /// let phones = haqumei.g2p_prosody("青い空が、好きだ。")?;
    /// assert_eq!(phones.join(" "), "^ a [ o ] i # s o ] r a g a _ s U [ k i ] d a _ $");
    ///
    /// # Ok(())
    /// # }
    /// ```
    pub fn g2p_prosody(&mut self, text: &str) -> Result<Vec<String>, HaqumeiError> {
        self.g2p_prosody_with_options(text, ProsodyFormat::Default)
    }

    /// 入力テキストを [ProsodyFormat] の設定をもとにプロソディ記号付き音素リストに変換します。
    ///
    /// 出力には、共通して以下のプロソディ記号が含まれます。
    ///
    /// | 記号 | 意味 | 出現位置 |
    /// | :--- | :--- | :--- |
    /// | `^` | 発話の開始 (BOS) | 文頭 |
    /// | `$` | 発話の終結 (EOS) | 文末 |
    /// | `?` | 疑問文の終結 (？) | 文中 |
    /// | `!` | 感嘆の終結 (独自拡張) | 文中 |
    /// | `_` | ポーズ・読点 (、) | 文中 |
    /// | `#` | アクセント句境界 | 文中 |
    /// | `{...}` | 未知語 | 文中 |
    ///
    /// 日本語のアクセントについて: [tdmelodic 利用マニュアル/予備知識](https://tdmelodic.readthedocs.io/ja/latest/pages/introduction.html)
    ///
    /// ## [ProsodyFormat::Default]
    ///
    /// 出力には上記のものに追加して、以下のプロソディ記号が含まれます。
    ///
    /// | 記号 | 意味 | 出現位置 |
    /// | :--- | :--- | :--- |
    /// | `[` | ピッチ上昇 (句頭) | 句の開始付近 |
    /// | `]` | ピッチ下降 (アクセント核) | 核モーラの直後 |
    ///
    /// 記号 `[` および `]` は、tdmelodic 等で一般的なアクセント記法に基づいています。
    /// "Prosodic Features Control by Symbols as Input of Sequence-to-Sequence Acoustic Modeling for Neural TTS"
    /// (Kurihara et al., 2021) のアルゴリズムにおける `^` および `!` に相当します。
    ///
    /// ## [ProsodyFormat::Prefix]
    ///
    /// ピッチ上昇/下降記号 (`[` や `]`) を使用せず、各音素のプレフィックスとしてピッチの高低を付与します。
    /// - `H_` : ピッチが高い (High)
    /// - `L_` : ピッチが低い (Low)
    ///
    /// 音素ごとにピッチが明示されます。
    /// 例: `"青い空"` -> `["^", "L_a", "H_o", "L_i", "#", "H_s", "H_o", "L_r", "L_a", "$"]`
    ///
    /// ## [ProsodyFormat::Numeric]
    ///
    /// 各音素のサフィックスとして、ピッチの高低を数値で付与します。
    /// - `:1` : ピッチが高い (High)
    /// - `:0` : ピッチが低い (Low)
    ///
    /// 例: `"青い空"` -> `["^", "a:0", "o:1", "i:0", "#", "s:1", "o:1", "r:0", "a:0", "$"]`
    pub fn g2p_prosody_with_options(
        &mut self,
        text: &str,
        format: ProsodyFormat,
    ) -> Result<Vec<String>, HaqumeiError> {
        let mapping = self.g2p_mapping_prosody(text)?;

        let mut output = Vec::new();

        // BOS
        output.push("^".to_string());

        let mut prev_pitch: Option<PitchAccent> = None;

        for word_prosody in mapping {
            output.extend(word_prosody.to_formatted_strings(format, &mut prev_pitch));
        }

        // EOS
        output.push("$".to_string());

        Ok(output)
    }

    /// 単語 (形態素) 単位に分割された音素リストを返します。
    ///
    /// # Returns
    ///
    /// 単語ごとの音素リストのベクタ。
    ///
    /// (e.g., [["k", "o", "N", "n", "i", "ch", "i", "w", "a"], ["pau"], ["s", "e", "k", "a", "i"]])
    pub fn g2p_per_word(&mut self, text: &str) -> Result<Vec<Vec<Phoneme>>, HaqumeiError> {
        if text.is_empty() {
            self.open_jtalk.ensure_dictionary_is_latest()?;
            return Ok(Vec::new());
        }

        let mapping = self.g2p_pairs(text.as_ref())?;

        let result = mapping.into_iter().map(|m| m.phonemes).collect();

        Ok(result)
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
    /// let pairs = haqumei.g2p_pairs("𰻞𰻞麺＆お冷を頼んだ").unwrap();
    ///
    /// // 結果:
    /// // [WordPhonemePair {
    /// //     word: "𰻞𰻞",
    /// //     phonemes: ["pau"]
    /// // }, WordPhonemePair {
    /// //     word: "麺",
    /// //     phonemes: ["m", "e", "N"]
    /// // }, WordPhonemePair {
    /// //     word: "＆",
    /// //     phonemes: ["a", "N", "d", "o"]
    /// // }, WordPhonemePair {
    /// //     word: "お冷",
    /// //     phonemes: ["o", "h", "i", "y", "a"]
    /// // }, WordPhonemePair {
    /// //     word: "を",
    /// //     phonemes: ["o"]
    /// // }, WordPhonemePair {
    /// //     word: "頼ん",
    /// //     phonemes: ["t", "a", "n", "o", "N"]
    /// // }, WordPhonemePair {
    /// //     word: "だ",
    /// //     phonemes: ["d", "a"]
    /// // }]
    /// // ```
    pub fn g2p_pairs(&mut self, text: &str) -> Result<Vec<WordPhonemePair>, HaqumeiError> {
        if text.is_empty() {
            self.open_jtalk.ensure_dictionary_is_latest()?;
            return Ok(Vec::new());
        }

        let features = self.run_frontend(text)?;

        if features.is_empty() {
            return Ok(Vec::new());
        }

        self.open_jtalk
            .g2p_pairs_inner(&features, self.options.is_non_pause_symbol)
    }

    /// 入力テキストの形態素ごとの音素マッピングを未知語などの情報とともに返します。
    ///
    /// MeCab による形態素解析の結果と 1:1 に対応するマッピング情報を生成します。
    ///
    /// - 既知語: 通常の音素列 (読点などは `pau`)
    /// - 未知語: `unk`
    /// - 空白等: `sp` (Space)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use haqumei::Haqumei;
    ///
    /// let mut haqumei = Haqumei::new().unwrap();
    /// let mapping = haqumei.g2p_mapping("𰻞𰻞麺 お冷を頼んだ").unwrap();
    ///
    /// // 結果:
    /// // [WordPhonemeMap {
    /// //     word: "𰻞𰻞",
    /// //     phonemes: ["unk"],
    /// //     is_unknown: true,
    /// //     is_ignored: false,
    /// // },
    /// // WordPhonemeMap {
    /// //     word: "麺",
    /// //     phonemes: ["m", "e", "N"],
    /// //     is_unknown: false,
    /// //     is_ignored: false,
    /// // },
    /// // WordPhonemeMap {
    /// //     word: "\u{3000}",
    /// //     phonemes: ["sp"],
    /// //     is_unknown: false,
    /// //     is_ignored: true,
    /// // },
    /// // WordPhonemeMap {
    /// //     word: "を",
    /// //     phonemes: ["o"],
    /// //     is_unknown: false,
    /// //     is_ignored: false,
    /// // },
    /// // WordPhonemeMap {
    /// //     word: "\u{3000}",
    /// //     phonemes: ["sp"],
    /// //     is_unknown: false,
    /// //     is_ignored: true,
    /// // },
    /// // WordPhonemeMap {
    /// //     word: "食べる",
    /// //     phonemes: ["t", "a", "b", "e", "r", "u"],
    /// //     is_unknown: false,
    /// //     is_ignored: false,
    /// // }]
    /// // ```
    pub fn g2p_mapping(&mut self, text: &str) -> Result<Vec<WordPhonemeMap>, HaqumeiError> {
        if text.is_empty() {
            self.open_jtalk.ensure_dictionary_is_latest()?;
            return Ok(Vec::new());
        }

        let (njd_features, morphs) = self.run_frontend_detailed(text)?;

        if njd_features.is_empty() {
            return Ok(Vec::new());
        }

        let mapping = self
            .open_jtalk
            .g2p_pairs_inner(&njd_features, self.options.is_non_pause_symbol)?;

        self.open_jtalk.make_phoneme_mapping(morphs, mapping)
    }

    /// 入力テキストの形態素ごとの音素マッピングを、NJD が付与する情報を含めて返します。
    ///
    /// - 既知語: 通常の音素列 (読点などは `pau`)
    /// - 未知語: `unk`
    /// - 空白等: `sp` (Space)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use haqumei::Haqumei;
    ///
    /// let mut haqumei = Haqumei::new().unwrap();
    /// let mapping = haqumei.g2p_mapping_detailed("薄明").unwrap();
    ///
    /// // 結果:
    /// // [ WordPhonemeDetail {
    /// //   word: "薄明",
    /// //   phonemes: [
    /// //       "h",
    /// //       "a",
    /// //       "k",
    /// //       "u",
    /// //       "m",
    /// //       "e",
    /// //       "e",
    /// //   ],
    /// //   features: [
    /// //       "薄明",
    /// //       "名詞",
    /// //       "一般",
    /// //       "*",
    /// //       "*",
    /// //       "*",
    /// //       "*",
    /// //       "薄明",
    /// //       "ハクメイ",
    /// //       "ハクメー",
    /// //       "0/4",
    /// //       "C2",
    /// //   ],
    /// //   pos: "名詞",
    /// //   pos_group1: "一般",
    /// //   pos_group2: "*",
    /// //   pos_group3: "*",
    /// //   ctype: "*",
    /// //   cform: "*",
    /// //   orig: "薄明",
    /// //   read: "ハクメイ",
    /// //   pron: "ハクメー",
    /// //   accent_nucleus: 0,
    /// //   mora_count: 4,
    /// //   chain_rule: "C2",
    /// //   chain_flag: -1,
    /// //   is_unknown: false,
    /// //   is_ignored: false,
    /// // }
    /// // ```
    pub fn g2p_mapping_detailed(
        &mut self,
        text: &str,
    ) -> Result<Vec<WordPhonemeDetail>, HaqumeiError> {
        if text.is_empty() {
            self.open_jtalk.ensure_dictionary_is_latest()?;
            return Ok(Vec::new());
        }

        // normalize_unicode_if_needed, revert_pron_to_read はここで実行される
        let (njd_features, morphs) = self.run_frontend_detailed(text)?;

        let mapping = self
            .open_jtalk
            .g2p_mapping_inner(&njd_features, self.options.is_non_pause_symbol)?;

        self.open_jtalk.make_phoneme_mapping(morphs, mapping)
    }

    /// 入力テキストを解析し、形態素 (単語) ごとの詳細な言語情報と、プロソディ (韻律) 記号付き音素をマッピングして取得します。
    ///
    /// [`Haqumei::g2p_prosody`] や [`Haqumei::g2p_prosody_with_options`] がフラットな文字列リスト (`Vec<String>`) を返すのに対し、
    /// この関数は品詞、アクセント型、読み、およびピッチ情報が付与された構造化データ (`Vec<WordPhonemeProsody>`) を返します。
    ///
    /// 音声合成のフロントエンド処理において、形態素と音素の対応関係を維持したい場合や、ピッチの高低 ([`PitchAccent`]) を
    /// 個別に取得・操作したい場合、あるいは未知語のハンドリングを行いたい場合に適しています。
    ///
    /// ## `WordPhonemeProsody` に含まれる主な情報
    ///
    /// 形態素ごとのデータとして、以下の情報が含まれます。
    ///
    /// | フィールド | 説明 | 例 |
    /// | :--- | :--- | :--- |
    /// | `word` | 形態素の表層形 | `"空"` |
    /// | `pos`, `pos_group1`~`3` | 品詞およびその細分類 | `"名詞"`, `"一般"` |
    /// | `orig`, `read`, `pron` | 原形、読み、発音形式 | `"空"`, `"ソラ"`, `"ソラ"` |
    /// | `accent_nucleus` | アクセント核位置 (0: 平板型, 1~: n番目のモーラ) | `1` |
    /// | `mora_count` | モーラ数 | `2` |
    /// | `is_unknown` | MeCabによって未知語判定されたかどうか | `false` |
    /// | `is_ignored` | 音素が割り当てられなかったか | `false` |
    ///
    /// ## プロソディ音素 (`ProsodicPhoneme`)
    ///
    /// `phonemes` フィールドには、以下の要素からなるリストが格納されます。
    ///
    /// | 列挙子 | 意味 | `g2p_prosody` 等での出力記号 |
    /// | :--- | :--- | :--- |
    /// | `Phoneme` | 音素本体と、そのピッチの高低 (`High` / `Low`) | `a`, `a:0`, `H_a` など |
    /// | `AccentPhraseBoundary` | アクセント句境界 | `#` |
    /// | `Pause` | 通常のポーズ・読点 | `_` |
    /// | `Interrogative` | 疑問文の終結・ポーズ | `?` |
    /// | `Exclamatory` | 感嘆の終結・ポーズ | `!` |
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use haqumei::{Haqumei, PitchAccent, ProsodicPhoneme};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut haqumei = Haqumei::new()?;
    ///
    /// // テキストを形態素ごとの構造化データとして取得
    /// let mapping = haqumei.g2p_mapping_prosody("青い空が、好きだ！")?;
    ///
    /// // 1単語目「青い」の形態素情報
    /// let aoi = &mapping[0];
    /// assert_eq!(aoi.word, "青い");
    /// assert_eq!(aoi.pos, "形容詞");
    /// assert_eq!(aoi.read, "アオイ");
    /// assert_eq!(aoi.accent_nucleus, 2); // 中高型
    ///
    /// // 「青い」の音素とピッチ情報 (a: Low, o: High, i: Low)
    /// assert!(matches!(
    ///     aoi.phonemes[0],
    ///     ProsodicPhoneme::Phoneme { pitch: Some(PitchAccent::Low), .. }
    /// ));
    ///
    /// let da = mapping.last().unwrap();
    /// assert_eq!(da.word, "！");
    /// assert!(da.phonemes.contains(&ProsodicPhoneme::Exclamatory));
    ///
    /// # Ok(())
    /// # }
    /// ```
    pub fn g2p_mapping_prosody(
        &mut self,
        text: &str,
    ) -> Result<Vec<WordPhonemeProsody>, HaqumeiError> {
        if text.is_empty() {
            self.open_jtalk.ensure_dictionary_is_latest()?;
            return Ok(Vec::new());
        }

        // normalize_unicode_if_needed, revert_pron_to_read はここで実行される
        let (njd_features, morphs) = self.run_frontend_detailed(text)?;

        let mapping = self
            .open_jtalk
            .g2p_mapping_prosody_inner(&njd_features, self.options.is_non_pause_symbol)?;

        self.open_jtalk.make_phoneme_mapping(morphs, mapping)
    }

    /// OpenJTalk のテキスト処理フロントエンドを実行する。
    pub fn run_frontend(&mut self, text: &str) -> Result<Vec<NjdFeature>, HaqumeiError> {
        if text.is_empty() {
            self.open_jtalk.ensure_dictionary_is_latest()?;
            return Ok(Vec::new());
        }

        let text = self.normalize_unicode_if_needed(text);
        let text = text.as_ref();

        self.rx = if let Some(tokenizer) =
            self.init_tokenizer_if_needed_and_modify_kanji_yomi_enabled()?
        {
            init_vibrato_workers_if_needed(&tokenizer);
            let (tx, rx) = bounded(1);
            if let Some(task_tx) = VIBRATO_TASK_TX.get() {
                let _ = task_tx.send((text.to_string(), tx));
            }
            Some(rx)
        } else {
            None
        };

        let njd_features = self.open_jtalk.run_frontend(text)?;

        self.apply_postprocessing(text, njd_features)
    }

    /// OpenJTalk のテキスト処理フロントエンドを実行する。
    /// [NjdFeature] だけでなく、Mecab の解析結果の [MecabMorph] のリスト
    /// を取得することができる。
    pub fn run_frontend_detailed(
        &mut self,
        text: &str,
    ) -> Result<(Vec<NjdFeature>, Vec<MecabMorph>), HaqumeiError> {
        if text.is_empty() {
            self.open_jtalk.ensure_dictionary_is_latest()?;
            return Ok((Vec::new(), Vec::new()));
        }

        let text = self.normalize_unicode_if_needed(text);
        let text = text.as_ref();

        self.rx = if let Some(tokenizer) =
            self.init_tokenizer_if_needed_and_modify_kanji_yomi_enabled()?
        {
            init_vibrato_workers_if_needed(&tokenizer);
            let (tx, rx) = bounded(1);
            if let Some(task_tx) = VIBRATO_TASK_TX.get() {
                let _ = task_tx.send((text.to_string(), tx));
            }
            Some(rx)
        } else {
            None
        };

        let (njd_features, mecab_morphs) = self.open_jtalk.run_frontend_detailed(text)?;

        Ok((self.apply_postprocessing(text, njd_features)?, mecab_morphs))
    }

    /// テキストから [haqumei_jlabel::Label] のリストとしてフルコンテキストラベルを抽出する。
    ///
    /// pyopenjtalk の `extract_fullcontext` に相当する文字列が
    /// 欲しい場合は、 `extract_fullcontext_string` を使用してください。
    pub fn extract_fullcontext(&mut self, text: &str) -> Result<Vec<Label>, HaqumeiError> {
        if text.is_empty() {
            self.open_jtalk.ensure_dictionary_is_latest()?;
            return Ok(Vec::new());
        }

        let njd_features = self.run_frontend(text.as_ref())?;
        self.open_jtalk.extract_fullcontext_labels(&njd_features)
    }

    /// テキストから jpcommon が出力するフルコンテキストラベルを抽出する。
    /// pyopenjtalk の `extract_fullcontext` に相当します。
    ///
    /// 構造化された [haqumei_jlabel::Label] が欲しい場合は、 `extract_fullcontext` を使用してください。
    pub fn extract_fullcontext_string(&mut self, text: &str) -> Result<Vec<String>, HaqumeiError> {
        if text.is_empty() {
            self.open_jtalk.ensure_dictionary_is_latest()?;
            return Ok(Vec::new());
        }

        let njd_features = self.run_frontend(text.as_ref())?;
        self.open_jtalk.make_label(&njd_features)
    }

    fn apply_postprocessing(
        &mut self,
        text: &str,
        mut njd_features: Vec<NjdFeature>,
    ) -> Result<Vec<NjdFeature>, HaqumeiError> {
        let options = self.options;

        if options.modify_filler_accent {
            modify_filler_accent(&mut njd_features);
        }
        if options.predict_nani {
            self.predict_nani_reading(&mut njd_features);
        }
        if options.use_unidic_yomi {
            self.modify_kanji_yomi(text, &mut njd_features);
        }
        if options.retreat_acc_nuc {
            retreat_acc_nuc(&mut njd_features);
        }
        if options.modify_acc_after_chaining {
            modify_acc_after_chaining(&mut njd_features);
        }
        if options.process_odoriji {
            process_odori_features(&mut njd_features, &mut self.open_jtalk)?;
        }
        if options.use_read_as_pron | options.revert_long_vowels | options.revert_yotsugana {
            self.revert_pron_to_read(&mut njd_features);
        }
        if let Some(iu_pron) = options.normalize_iu {
            self.normalize_iu(&mut njd_features, iu_pron);
        }

        Ok(njd_features)
    }

    pub(crate) fn predict_is_nan(&mut self, prev_node: Option<&NjdFeature>) -> bool {
        let prev_node = match prev_node {
            Some(node) => node,
            None => return false,
        };

        NANI_PREDICTOR_CACHE.get_with(prev_node.clone(), || {
            NANI_PREDICTOR
                .lock()
                .unwrap()
                .predict_is_nan(Some(prev_node))
        })
    }

    impl_batch_method_haqumei!(
        /// 複数のテキストに対して `run_frontend` を実行します。
        run_frontend_batch => run_frontend -> Vec<NjdFeature>
    );

    impl_batch_method_haqumei!(
        /// 複数のテキストに対して `run_frontend_detailed` を実行します。
        run_frontend_detailed_batch => run_frontend_detailed -> (Vec<NjdFeature>, Vec<MecabMorph>)
    );

    impl_batch_method_haqumei!(
        /// 複数のテキストに対して `g2p` を実行します。
        g2p_batch => g2p -> Vec<Phoneme>
    );

    impl_batch_method_haqumei!(
        /// すべてのトークンを保持する詳細な G2P 変換のバッチ処理。
        ///
        /// - 既知語: 通常の音素列 (読点などは `pau`)
        /// - 未知語: `unk`
        /// - 空白等: `sp` (Space)
        g2p_detailed_batch => g2p_detailed -> Vec<Phoneme>
    );

    impl_batch_method_haqumei!(
        /// カタカナ変換のバッチ処理。
        g2p_kana_batch => g2p_kana -> String
    );

    impl_batch_method_haqumei!(
        /// 単語ごとに分割されたカタカナ変換のバッチ処理。
        g2p_kana_per_word_batch => g2p_kana_per_word -> Vec<String>
    );

    impl_batch_method_haqumei!(
        /// 入力テキストのリストから、プロソディ記号付き音素リストを抽出するバッチ処理。
        g2p_prosody_batch => g2p_prosody -> Vec<String>
    );

    impl_batch_method_haqumei!(
        /// 入力テキストのリストから、プロソディ記号付き音素リストを抽出するバッチ処理。
        g2p_prosody_with_options_batch => g2p_prosody_with_options(format: ProsodyFormat) -> Vec<String>
    );

    impl_batch_method_haqumei!(
        /// 単語ごとに分割された音素リストのバッチ処理。
        g2p_per_word_batch => g2p_per_word -> Vec<Vec<Phoneme>>
    );

    impl_batch_method_haqumei!(
        /// 形態素ごとの音素マッピングのバッチ処理。
        ///
        /// MeCab による形態素解析の結果と 1:1 に対応するマッピング情報を生成します。
        ///
        /// **記号・未知語の処理**: 読点 (`、`) や未知語など、OpenJTalk が発音を生成しないトークンに対しては、
        ///   音素リストとして `["pau"]` が割り当てられます。
        g2p_pairs_batch => g2p_pairs -> Vec<WordPhonemePair>
    );

    impl_batch_method_haqumei!(
        /// 形態素ごとの未知語を含めたより詳細な音素マッピングのバッチ処理。
        ///
        /// MeCab による形態素解析の結果と 1:1 に対応するマッピング情報を生成します。
        ///
        /// - 既知語: 通常の音素列 (読点などは `pau`)
        /// - 未知語: `unk`
        /// - 空白等: `sp` (Space)
        g2p_mapping_batch => g2p_mapping -> Vec<WordPhonemeMap>
    );

    impl_batch_method_haqumei!(
        /// 形態素ごとの未知語や NJD の情報を含めたより詳細な音素マッピングのバッチ処理。
        ///
        /// MeCab による形態素解析の結果と 1:1 に対応するマッピング情報を生成します。
        ///
        /// - 既知語: 通常の音素列 (読点などは `pau`)
        /// - 未知語: `unk`
        /// - 空白等: `sp` (Space)
        g2p_mapping_detailed_batch => g2p_mapping_detailed -> Vec<WordPhonemeDetail>
    );

    impl_batch_method_haqumei!(
        /// プロソディ記号付き音素マッピングのバッチ処理。
        g2p_mapping_prosody_batch => g2p_mapping_prosody -> Vec<WordPhonemeProsody>
    );

    impl_batch_method_haqumei!(
        /// haqumei_jlabel::Label を返すフルコンテキストラベル抽出のバッチ処理。
        extract_fullcontext_batch => extract_fullcontext -> Vec<Label>
    );

    impl_batch_method_haqumei!(
        /// フルコンテキストラベル抽出のバッチ処理。
        extract_fullcontext_string_batch => extract_fullcontext_string -> Vec<String>
    );
}
