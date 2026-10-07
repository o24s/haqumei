pub mod candidates;
mod cursor;
mod data;
pub mod errors;
pub mod features;
mod kaomoji;
#[macro_use]
mod macros;
mod calendar;
mod identifier;
pub mod ipa;
pub mod nani_predict;
pub mod open_jtalk;
pub mod options;
pub mod phoneme;
mod postprocess;
pub mod prosody;
mod roman;
pub mod utils;
pub mod word_phoneme;

use std::{
    path::Path,
    sync::{Arc, LazyLock, Mutex},
};

pub use haqumei_jlabel::Label;
use haqumei_kanalizer::Kanalizer;
use moka::sync::Cache;

use std::collections::HashMap;

pub use candidates::{
    Candidate, CandidateAlternative, CandidateBranch, CandidateOptions, CandidateReading,
    Candidates,
};
pub use features::NjdFeature;
pub use ipa::{
    IpaBoundary, IpaPhone, IpaToken, IpaTokenProsody, ProsodicIpa, SpecialPhone, WordIpaMap,
    WordIpaProsody,
};
pub use open_jtalk::{
    LatticeNode, MecabDictIndexCompiler, MecabMorph, NO_DICTIONARY_INDEX, OpenJTalk,
    njd_char_spans, unset_user_dictionary, update_global_dictionary,
};
pub use options::*;
pub use phoneme::Phoneme;
pub use prosody::{PitchAccent, ProsodicPhoneme, ProsodyFormat};
pub use word_phoneme::{WordPhonemeDetail, WordPhonemeMap, WordPhonemeProsody};

use crate::{
    errors::HaqumeiError,
    nani_predict::NaniPredictor,
    open_jtalk::{
        Dictionary, GLOBAL_MECAB_DICTIONARY,
        reading_protection::{protected_indices, registered_accent_nuclei},
    },
    postprocess::{
        merge_english_alphanumeric_words, modify_acc_after_chaining, modify_context_reading,
        modify_english_words, modify_filler_accent, modify_fraction_denominator,
        modify_old_province_yomi, predict_kana_english, process_odori_features, read_unknown_kanji,
        restore_loanword_kana, retreat_acc_nuc, suppress_english_hyphen_pause,
    },
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

static NANI_PREDICTOR_CACHE: LazyLock<Cache<NjdFeature, bool>> = LazyLock::new(|| Cache::new(1000));
static NANI_PREDICTOR: LazyLock<Mutex<NaniPredictor>> = LazyLock::new(|| {
    Mutex::new(NaniPredictor::new().expect("Failed to initialize NaniPredictor models"))
});
static KANALIZER_CACHE: LazyLock<Cache<String, String>> = LazyLock::new(|| Cache::new(1000));
static KANALIZER: LazyLock<Mutex<Kanalizer>> =
    LazyLock::new(|| Mutex::new(Kanalizer::new().expect("Failed to initialize Kanalizer models")));

/// MeCab の出力を NJD に渡す前に書き換える手続き。
///
/// 受け取るのは `text2mecab` を通した文、その文のラティスに立った全ノード、そして
/// 最良経路の形態素列である。ノードの `char_span` は渡された文の文字位置を指す。
///
/// 形態素の `feature` を書き換えると、NJD はそれを入力として読み直す。**NJD の後で
/// `pron` を触ると `njd_set_accent_type` が打った核をモーラ数の差だけ自分でずらす
/// 必要が出るが、ここで書き換えれば NJD が最初から数え直す。**
pub type MorphFilter = dyn Fn(&str, &[LatticeNode], &mut [MecabMorph]) + Send + Sync;

/// 日本語の読み・音素・韻律を生成する G2P エンジン。
///
/// [`pyopenjtalk-plus`](https://github.com/tsukumijima/pyopenjtalk-plus) の辞書をベースに、様々な変更を加えています。
/// 詳細は README を参照してください。
///
/// [Haqumei::with_options], [HaqumeiOptions] を使うことで、出力をカスタマイズできます。
pub struct Haqumei {
    pub(crate) open_jtalk: OpenJTalk,
    pub options: HaqumeiOptions,
    /// MeCab の出力を NJD に渡す前に書き換える手続き。`None` なら何もしない。
    ///
    /// [`HaqumeiOptions`] ではなくここに持つのは、設定ではなく呼び出し側が持ち込む
    /// 資源だからである。`HaqumeiOptions` は `Copy` なので所有する値を置けない。
    pub(crate) morph_filter: Option<std::sync::Arc<MorphFilter>>,
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
        Ok(Haqumei {
            open_jtalk,
            options,
            morph_filter: None,
        })
    }

    /// 解析に使う辞書とオプションを現在の設定に同期します。
    fn prepare_analysis(&mut self) -> Result<(), HaqumeiError> {
        self.open_jtalk.ensure_dictionary_is_latest()?;
        self.open_jtalk.resolve_kanji_variants = self.options.resolve_kanji_variants;
        self.open_jtalk.defer_unvoicing = true;
        self.open_jtalk.split_prefix_accent_phrase = self.options.split_prefix_accent_phrase;
        Ok(())
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

    /// 辞書と、0 個以上のユーザー辞書のパスから [Haqumei] を生成します。
    pub fn from_paths<P: AsRef<Path>, Q: AsRef<Path>>(
        dict_dir: P,
        user_dicts: &[Q],
        options: HaqumeiOptions,
    ) -> Result<Self, HaqumeiError> {
        Self::from_open_jtalk(OpenJTalk::from_paths(dict_dir, user_dicts)?, options)
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
            self.prepare_analysis()?;
            return Ok(Vec::new());
        }

        let features = self.run_frontend(text)?;

        if features.is_empty() {
            return Ok(Vec::new());
        }

        let mut phonemes = self.open_jtalk.extract_phonemes(&features)?;
        postprocess::apply_allophones(&mut phonemes, &self.options);

        Ok(phonemes)
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
            self.prepare_analysis()?;
            return Ok(Vec::new());
        }

        let detailed_mapping = self.g2p_mapping(text)?;

        let mut phonemes = Vec::new();
        for map in detailed_mapping {
            phonemes.extend(map.phonemes);
        }
        postprocess::apply_allophones(&mut phonemes, &self.options);

        Ok(phonemes)
    }

    /// 入力テキストをカタカナに変換します。
    ///
    /// pyopenjtalk と同様に、記号や未知語などの文字は、元の表記が使用されます。
    pub fn g2k(&mut self, text: &str) -> Result<String, HaqumeiError> {
        if text.is_empty() {
            self.prepare_analysis()?;
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
    pub fn g2k_per_word(&mut self, text: &str) -> Result<Vec<String>, HaqumeiError> {
        if text.is_empty() {
            self.prepare_analysis()?;
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
            self.prepare_analysis()?;
            return Ok(Vec::new());
        }

        let features = self.run_frontend(text)?;

        if features.is_empty() {
            return Ok(Vec::new());
        }

        // 区間を捨てるので `njd_spans` は空で渡す
        let seeds =
            self.open_jtalk
                .g2p_seed_inner(&features, &[], self.options.is_non_pause_symbol)?;

        let mut result: Vec<Vec<Phoneme>> = seeds.into_iter().map(|s| s.phonemes).collect();
        postprocess::apply_allophones(result.iter_mut().flat_map(|p| p.iter_mut()), &self.options);

        Ok(result)
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
    /// //     char_span: 0..2,
    /// // },
    /// // WordPhonemeMap {
    /// //     word: "麺",
    /// //     phonemes: ["m", "e", "N"],
    /// //     is_unknown: false,
    /// //     is_ignored: false,
    /// //     char_span: 2..3,
    /// // },
    /// // WordPhonemeMap {
    /// //     word: "\u{3000}",
    /// //     phonemes: ["sp"],
    /// //     is_unknown: false,
    /// //     is_ignored: true,
    /// //     char_span: 3..4,
    /// // },
    /// // WordPhonemeMap {
    /// //     word: "お冷",
    /// //     phonemes: ["o", "h", "i", "y", "a"],
    /// //     is_unknown: false,
    /// //     is_ignored: false,
    /// //     char_span: 4..6,
    /// // },
    /// // WordPhonemeMap {
    /// //     word: "を",
    /// //     phonemes: ["o"],
    /// //     is_unknown: false,
    /// //     is_ignored: false,
    /// //     char_span: 6..7,
    /// // },
    /// // WordPhonemeMap {
    /// //     word: "頼ん",
    /// //     phonemes: ["t", "a", "n", "o", "N"],
    /// //     is_unknown: false,
    /// //     is_ignored: false,
    /// //     char_span: 7..9,
    /// // },
    /// // WordPhonemeMap {
    /// //     word: "だ",
    /// //     phonemes: ["d", "a"],
    /// //     is_unknown: false,
    /// //     is_ignored: false,
    /// //     char_span: 9..10,
    /// // }]
    /// // ```
    pub fn g2p_mapping(&mut self, text: &str) -> Result<Vec<WordPhonemeMap>, HaqumeiError> {
        if text.is_empty() {
            self.prepare_analysis()?;
            return Ok(Vec::new());
        }

        let (njd_features, morphs) = self.run_frontend_detailed(text)?;

        if njd_features.is_empty() {
            return Ok(Vec::new());
        }

        let njd_spans = njd_char_spans(&njd_features, &morphs);
        let mapping = self.open_jtalk.g2p_seed_inner(
            &njd_features,
            &njd_spans,
            self.options.is_non_pause_symbol,
        )?;

        let mut mapping = self.open_jtalk.make_phoneme_mapping(morphs, mapping)?;
        postprocess::apply_allophones(
            mapping.iter_mut().flat_map(|m| m.phonemes.iter_mut()),
            &self.options,
        );

        Ok(mapping)
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
    /// //   char_span: 0..2,
    /// // }
    /// // ```
    pub fn g2p_mapping_detailed(
        &mut self,
        text: &str,
    ) -> Result<Vec<WordPhonemeDetail>, HaqumeiError> {
        if text.is_empty() {
            self.prepare_analysis()?;
            return Ok(Vec::new());
        }

        // normalize_unicode_if_needed, revert_pron_to_read はここで実行される
        let (njd_features, morphs) = self.run_frontend_detailed(text)?;

        let njd_spans = njd_char_spans(&njd_features, &morphs);
        let mapping = self.open_jtalk.g2p_mapping_inner(
            &njd_features,
            &njd_spans,
            self.options.is_non_pause_symbol,
        )?;

        let mut mapping = self.open_jtalk.make_phoneme_mapping(morphs, mapping)?;

        postprocess::apply_allophones(
            mapping.iter_mut().flat_map(|m| m.phonemes.iter_mut()),
            &self.options,
        );

        Ok(mapping)
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
            self.prepare_analysis()?;
            return Ok(Vec::new());
        }

        // normalize_unicode_if_needed, revert_pron_to_read はここで実行される
        let (njd_features, morphs) = self.run_frontend_detailed(text)?;

        let njd_spans = njd_char_spans(&njd_features, &morphs);
        let mapping = self.open_jtalk.g2p_mapping_prosody_inner(
            &njd_features,
            &njd_spans,
            self.options.is_non_pause_symbol,
        )?;

        let mut mapping = self.open_jtalk.make_phoneme_mapping(morphs, mapping)?;
        postprocess::apply_allophones_to_prosody(
            mapping.iter_mut().flat_map(|m| m.phonemes.iter_mut()),
            &self.options,
        );

        Ok(mapping)
    }

    /// MeCab の出力を NJD に渡す前に書き換える手続きを設定します。
    ///
    /// 辞書だけでは選べない読みを外から決めるための口である。ラティスに立っている
    /// 候補は全部渡されるので、`feature` を候補のもので置き換えれば読みが変わる。
    pub fn set_morph_filter(
        &mut self,
        filter: impl Fn(&str, &[LatticeNode], &mut [MecabMorph]) + Send + Sync + 'static,
    ) {
        self.morph_filter = Some(std::sync::Arc::new(filter));
    }

    /// 書き換えの手続きを外します。
    pub fn clear_morph_filter(&mut self) {
        self.morph_filter = None;
    }

    /// OpenJTalk のテキスト処理フロントエンドを実行する。
    pub fn run_frontend(&mut self, text: &str) -> Result<Vec<NjdFeature>, HaqumeiError> {
        // 書き換えは MeCab の解析結果を要するので、形態素を返さない経路でも
        // detailed 側を通す。ここを分けたままにすると `g2k` や `extract_fullcontext`
        // でだけ手続きが無視され、同じ入力で API ごとに読みが変わる
        if self.morph_filter.is_some() {
            return Ok(self.run_frontend_detailed(text)?.0);
        }

        self.prepare_analysis()?;
        if text.is_empty() {
            return Ok(Vec::new());
        }

        let roman = self
            .options
            .resolve_roman_numerals
            .then(|| roman::prepare(text, self.options.normalize_unicode))
            .flatten();
        let prepared = self
            .options
            .ignore_kaomoji
            .then(|| kaomoji::prepare(text, self.options.normalize_unicode))
            .flatten();
        if roman.is_some() || prepared.is_some() {
            return Ok(self.run_frontend_prepared(text, prepared, roman)?.0);
        }

        let text = self.normalize_unicode_if_needed(text);
        let text = text.as_ref();

        // `predict_kana_english` が英単語の区切りを決めるには、空白も含む MeCab
        // 形態素の位置が要る。英字を含まないときは `OpenJTalk::run_frontend` を使い、
        // 英字を含むときだけ詳細な解析結果を残す。
        let needs_english_positions = self.options.predict_kana_english
            && text
                .chars()
                .any(|c| matches!(c, 'A'..='Z' | 'a'..='z' | 'Ａ'..='Ｚ' | 'ａ'..='ｚ'));

        if calendar::may_contain_date(text)
            || (self.options.resolve_number_identifiers && identifier::may_contain_identifier(text))
        {
            let mut morphs = self.open_jtalk.run_mecab_with_symbol_split(
                text,
                self.options.protect_user_dict_readings || needs_english_positions,
            )?;
            let normalized = self.open_jtalk.text2mecab_string(text)?;

            calendar::merge(&normalized, &mut morphs, &[]);
            identifier::merge(&normalized, &mut morphs, &[], &self.options);

            return Ok(self.finish_frontend(text, morphs, &[])?.0);
        }

        if self.options.protect_user_dict_readings
            || (self.options.protect_user_dict_accents && self.options.retreat_acc_nuc)
            || needs_english_positions
        {
            // 核の保護のために辞書由来を調べても、記号の区切りは変えない。
            let (njd_features, morphs) = self.open_jtalk.run_frontend_with_morphs(
                text,
                self.options.modify_numeral_reading,
                self.options.protect_user_dict_readings,
                self.options.protect_user_dict_readings || needs_english_positions,
            )?;
            let protected = if self.options.protect_user_dict_readings {
                protected_indices(&njd_features, &morphs)
            } else {
                HashMap::new()
            };
            return self.apply_postprocessing(text, njd_features, &protected, &morphs);
        }

        let njd_features = self
            .open_jtalk
            .run_frontend_with_numeral_reading(text, self.options.modify_numeral_reading)?;
        self.apply_postprocessing(text, njd_features, &HashMap::new(), &[])
    }

    /// OpenJTalk のテキスト処理フロントエンドを実行する。
    /// [NjdFeature] だけでなく、Mecab の解析結果の [MecabMorph] のリスト
    /// を取得することができる。
    pub fn run_frontend_detailed(
        &mut self,
        text: &str,
    ) -> Result<(Vec<NjdFeature>, Vec<MecabMorph>), HaqumeiError> {
        self.prepare_analysis()?;
        if text.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }

        let prepared = self
            .options
            .ignore_kaomoji
            .then(|| kaomoji::prepare(text, self.options.normalize_unicode))
            .flatten();
        let roman = self
            .options
            .resolve_roman_numerals
            .then(|| roman::prepare(text, self.options.normalize_unicode))
            .flatten();
        self.run_frontend_prepared(text, prepared, roman)
    }

    fn run_frontend_prepared(
        &mut self,
        input: &str,
        prepared: Option<kaomoji::Prepared>,
        roman: Option<roman::Prepared>,
    ) -> Result<(Vec<NjdFeature>, Vec<MecabMorph>), HaqumeiError> {
        let text = if let Some(prepared) = &prepared {
            std::borrow::Cow::Borrowed(prepared.unicode.as_str())
        } else if let Some(roman) = &roman {
            std::borrow::Cow::Borrowed(roman.unicode.as_str())
        } else {
            self.normalize_unicode_if_needed(input)
        };
        let mut morphs = self.open_jtalk.run_mecab_detailed(&text)?;
        let has_calendar = calendar::may_contain_date(&text);
        let has_identifier =
            self.options.resolve_number_identifiers && identifier::may_contain_identifier(&text);
        let before_filter =
            ((prepared.is_some() || roman.is_some() || has_calendar || has_identifier)
                && self.morph_filter.is_some())
            .then(|| morphs.clone());
        if let Some(filter) = self.morph_filter.clone() {
            let normalized = self.open_jtalk.text2mecab_string(&text)?;
            let nodes = self.open_jtalk.analyze_lattice(&text)?;
            filter(&normalized, &nodes, &mut morphs);
        }
        let edited = before_filter
            .as_ref()
            .map(|before| kaomoji::edited_ranges(before, &morphs))
            .unwrap_or_default();
        if let Some(prepared) = &prepared {
            let mut ranges = prepared.select(input, &morphs);
            if let Some(before) = &before_filter {
                kaomoji::protect_edits(&mut ranges, before, &morphs);
            }
            kaomoji::merge(&prepared.normalized, &mut morphs, &ranges);
        }
        if let Some(roman) = &roman {
            roman.merge(&mut morphs, &edited);
        }

        if has_calendar || has_identifier {
            let normalized = self.open_jtalk.text2mecab_string(&text)?;

            if has_calendar {
                calendar::merge(&normalized, &mut morphs, &edited);
            }

            if has_identifier {
                identifier::merge(&normalized, &mut morphs, &edited, &self.options);
            }
        }

        self.finish_frontend(&text, morphs, &edited)
    }

    fn finish_frontend(
        &mut self,
        text: &str,
        morphs: Vec<MecabMorph>,
        edited: &[std::ops::Range<usize>],
    ) -> Result<(Vec<NjdFeature>, Vec<MecabMorph>), HaqumeiError> {
        let features = self.open_jtalk.run_njd_from_morphs(
            &morphs,
            self.options.modify_numeral_reading,
            self.options.protect_user_dict_readings,
            edited,
        )?;
        let protected = if self.options.protect_user_dict_readings {
            protected_indices(&features, &morphs)
        } else {
            HashMap::new()
        };
        Ok((
            self.apply_postprocessing(text, features, &protected, &morphs)?,
            morphs,
        ))
    }

    /// MeCab の解析結果を、形態素ごとの情報として返します。
    ///
    /// [`HaqumeiOptions::normalize_unicode`] を通したテキストを
    /// [`OpenJTalk::run_mecab_detailed`] に渡します。[`MecabMorph::char_span`] と
    /// [`Haqumei::analyze_lattice`] が返す [`LatticeNode::char_span`] は、同じ文字列の
    /// 位置になります。
    pub fn run_mecab_detailed(&mut self, text: &str) -> Result<Vec<MecabMorph>, HaqumeiError> {
        self.prepare_analysis()?;
        if text.is_empty() {
            return Ok(Vec::new());
        }
        let text = self.normalize_unicode_if_needed(text);
        self.open_jtalk.run_mecab_detailed(text.as_ref())
    }

    /// MeCab のラティスを、ノードごとの経路コスト差とともに返します。
    ///
    /// [`HaqumeiOptions::normalize_unicode`] を通したテキストを
    /// [`OpenJTalk::analyze_lattice`] に渡します。[`LatticeNode::char_span`] を
    /// [`MecabMorph::char_span`] と突き合わせるなら、形態素も
    /// [`Haqumei::run_mecab_detailed`] から取ります。片方を
    /// [`OpenJTalk::analyze_lattice`] から取ると、正規化のぶん位置がずれます。
    ///
    /// ラティスから読みの候補を作るなら [`Haqumei::g2p_candidates`] があります。
    pub fn analyze_lattice(&mut self, text: &str) -> Result<Vec<LatticeNode>, HaqumeiError> {
        self.prepare_analysis()?;
        if text.is_empty() {
            return Ok(Vec::new());
        }
        let text = self.normalize_unicode_if_needed(text);
        self.open_jtalk.analyze_lattice(text.as_ref())
    }

    /// テキストから [haqumei_jlabel::Label] のリストとしてフルコンテキストラベルを抽出する。
    ///
    /// pyopenjtalk の `extract_fullcontext` に相当する文字列が
    /// 欲しい場合は、 `extract_fullcontext_string` を使用してください。
    pub fn extract_fullcontext(&mut self, text: &str) -> Result<Vec<Label>, HaqumeiError> {
        if text.is_empty() {
            self.prepare_analysis()?;
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
            self.prepare_analysis()?;
            return Ok(Vec::new());
        }

        let njd_features = self.run_frontend(text.as_ref())?;
        self.open_jtalk
            .extract_fullcontext_labels(&njd_features)
            .map(|labels| labels.into_iter().map(|l| l.to_string()).collect())
    }

    fn apply_postprocessing(
        &mut self,
        text: &str,
        mut njd_features: Vec<NjdFeature>,
        protected: &HashMap<usize, usize>,
        morphs: &[MecabMorph],
    ) -> Result<Vec<NjdFeature>, HaqumeiError> {
        let options = self.options;

        // 補正で形態素の添字が変わるため、ユーザー辞書の読みを文字位置で控える。
        // 英語の読み推定は保護対象を結合せず、英数字の結合は読みの復元後に行う。
        let saved: HashMap<usize, (String, String, i32)> = protected
            .iter()
            .filter_map(|(&start, &idx)| {
                njd_features
                    .get(idx)
                    .map(|f| (start, (f.read.clone(), f.pron.clone(), f.mora_size)))
            })
            .collect();

        if options.modify_filler_accent {
            modify_filler_accent(&mut njd_features);
        }
        if options.predict_nani {
            self.predict_nani_reading(&mut njd_features);
        }
        if options.predict_kana_english {
            predict_kana_english(&mut njd_features, morphs, protected);
            modify_english_words(text, &mut njd_features);
            suppress_english_hyphen_pause(&mut njd_features, morphs);
        }

        // 読みを確定させた後、アクセント関連の補正より前に文脈依存の読みを解決する
        // (いずれも mora_size が変わるため)
        let protected_nodes: Vec<bool> = if saved.is_empty()
            || !(options.modify_context_reading || options.modify_numeral_reading)
        {
            Vec::new()
        } else {
            njd_char_spans(&njd_features, morphs)
                .into_iter()
                .map(|span| !span.is_empty() && saved.contains_key(&span.start))
                .collect()
        };
        if options.modify_context_reading {
            modify_context_reading(&mut njd_features, &protected_nodes);
        }
        if options.modify_old_province_yomi {
            modify_old_province_yomi(&mut njd_features);
        }
        if options.modify_numeral_reading {
            modify_fraction_denominator(&mut njd_features);
            postprocess::modify_group_reading(&mut njd_features, &protected_nodes);
        }
        // 辞書に無い漢字への読みの付与は、他の補正がすべて読みを決めたあとに行う。
        // ここまでで読みが付かなかったものだけが対象になる
        if options.restore_loanword_kana {
            restore_loanword_kana(&mut njd_features);
        }
        if options.read_unknown_kanji {
            read_unknown_kanji(&mut njd_features);
        }

        // アクセントの補正は mora_size を見るので、書き戻すならその前でなければならない。
        //
        // このあとの `revert_pron_to_read` と `normalize_iu` は表記の慣習を選ぶ
        // オプションで、呼び出し側の明示的な指定なので守らない。
        if !saved.is_empty() {
            // 形態素の数が変わっていなければ添字も動いていないとは言い切れないので、再計算する。
            for (idx, span) in njd_char_spans(&njd_features, morphs)
                .into_iter()
                .enumerate()
            {
                // 位取りとして差し込まれた形態素は元の文字を持たず、区間が空になる。
                // 開始位置は隣の形態素と同じなので、除かないと隣の読みを書き込む
                if span.start == span.end {
                    continue;
                }
                if let Some((read, pron, mora_size)) = saved.get(&span.start)
                    && let Some(f) = njd_features.get_mut(idx)
                {
                    f.read = read.clone();
                    f.pron = pron.clone();
                    f.mora_size = *mora_size;
                }
            }
        }

        // 読みの復元より前に結合すると、GNU2 の 2 など、保護対象に続く読みが消える。
        if options.predict_kana_english {
            merge_english_alphanumeric_words(&mut njd_features, morphs);
        }

        // 読みを変更した語とその隣の無声化を判定する。辞書にある無声化の指定は残す。
        // 核の後退は無声化を参照するため、retreat_acc_nuc より前に判定する。
        postprocess::apply_unvoicing(&mut njd_features);
        if options.retreat_acc_nuc {
            let registered = if options.protect_user_dict_accents {
                registered_accent_nuclei(&njd_features, morphs)
            } else {
                Vec::new()
            };
            retreat_acc_nuc(&mut njd_features, &registered);
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
        g2k_batch => g2k -> String
    );

    impl_batch_method_haqumei!(
        /// 単語ごとに分割されたカタカナ変換のバッチ処理。
        g2k_per_word_batch => g2k_per_word -> Vec<String>
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
        /// 単語ごとの IPA の広い音声表記への変換を並行して行うバッチ処理。
        g2ipa_batch => g2ipa -> Vec<WordIpaMap>
    );

    impl_batch_method_haqumei!(
        /// ピッチアクセントと韻律境界を保持した IPA 変換のバッチ処理。
        g2ipa_prosody_batch => g2ipa_prosody -> Vec<WordIpaProsody>
    );

    impl_batch_method_haqumei!(
        /// 読みの候補のバッチ処理。
        g2p_candidates_batch => g2p_candidates -> Candidates<WordPhonemeMap>
    );

    impl_batch_method_haqumei!(
        /// NJD の情報を含む読みの候補のバッチ処理。
        g2p_candidates_detailed_batch => g2p_candidates_detailed -> Candidates<WordPhonemeDetail>
    );

    impl_batch_method_haqumei!(
        /// プロソディ記号付きの読みの候補のバッチ処理。
        g2p_candidates_prosody_batch => g2p_candidates_prosody -> Candidates<WordPhonemeProsody>
    );

    impl_batch_method_haqumei!(
        /// [`CandidateOptions`] を指定した読みの候補のバッチ処理。
        g2p_candidates_with_options_batch => g2p_candidates_with_options(options: CandidateOptions) -> Candidates<WordPhonemeMap>
    );

    impl_batch_method_haqumei!(
        /// [`CandidateOptions`] を指定した、NJD の情報を含む読みの候補のバッチ処理。
        g2p_candidates_detailed_with_options_batch => g2p_candidates_detailed_with_options(options: CandidateOptions) -> Candidates<WordPhonemeDetail>
    );

    impl_batch_method_haqumei!(
        /// [`CandidateOptions`] を指定した、プロソディ記号付きの読みの候補のバッチ処理。
        g2p_candidates_prosody_with_options_batch => g2p_candidates_prosody_with_options(options: CandidateOptions) -> Candidates<WordPhonemeProsody>
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
