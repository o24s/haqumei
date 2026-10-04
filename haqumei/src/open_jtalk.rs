pub mod dictionary;
mod lattice;
pub(crate) mod mapping;
mod mecab;
mod model;
pub(crate) mod morph;
pub(crate) mod njd;
pub(crate) mod reading_protection;

#[cfg(test)]
mod label_tests;
#[cfg(test)]
mod tests;

use crate::cursor::CharCursor;
use crate::errors::HaqumeiError;
#[cfg(not(feature = "embed-dictionary"))]
use crate::open_jtalk::model::MecabModel;
use crate::phoneme::Phoneme;
use crate::utils::{default_is_non_pause_symbol, get_known_symbol_feature};
use crate::word_phoneme::WordPhonemeProsody;
use crate::{NjdFeature, WordPhonemeDetail, WordPhonemeMap};
use crate::{PitchAccent, ProsodyFormat};

use arc_swap::ArcSwap;
use haqumei_jlabel::Label;
use mecab::Mecab;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use std::path::Path;
use std::sync::{Arc, LazyLock};

pub use dictionary::build_mecab_dictionary;
pub use dictionary::{Dictionary, MecabDictIndexCompiler};
pub use lattice::LatticeNode;
pub use mapping::njd_char_spans;
pub use morph::{MecabMorph, NO_DICTIONARY_INDEX};

#[cfg(doc)]
use crate::Haqumei;

/// `Haqumei`, `OpenJTalk` が利用するグローバル辞書。
/// [update_global_dictionary], [unset_user_dictionary] を使って変更してください。
pub static GLOBAL_MECAB_DICTIONARY: LazyLock<ArcSwap<Dictionary>> = LazyLock::new(|| {
    #[cfg(feature = "embed-dictionary")]
    {
        let default_dict = Dictionary::from_embedded()
            .expect("Failed to load embedded dictionary. This should not happen.");
        ArcSwap::from(Arc::new(default_dict))
    }
    #[cfg(not(feature = "embed-dictionary"))]
    {
        let dummy_model = MecabModel::new_uninitialized();
        let dummy_dict = Dictionary {
            model: Arc::new(dummy_model),
            dict_dir: std::path::PathBuf::new(),
        };
        ArcSwap::from(Arc::new(dummy_dict))
    }
});

/// `Haqumei`, `OpenJTalk` から使用されるグローバル辞書を更新します (設定します)。
///
/// この関数を呼び出した後、新たに `g2p_*` や `extract_fullcontext` などを呼び出す際には、この辞書が使用されるようになります。
/// 既存のインスタンスについては、次のメソッド呼び出し時に新しい辞書に更新されます。
pub fn update_global_dictionary(new_dict: Dictionary) {
    GLOBAL_MECAB_DICTIONARY.store(Arc::new(new_dict));
}

/// `Haqumei`, `OpenJTalk` から使用されるグローバル辞書のユーザー辞書を外します。
pub fn unset_user_dictionary() -> Result<(), HaqumeiError> {
    GLOBAL_MECAB_DICTIONARY.store(Arc::new(Dictionary::from_path(
        &GLOBAL_MECAB_DICTIONARY.load_full().dict_dir,
        None,
    )?));
    Ok(())
}

/// Open JTalk 互換の形態素解析・NJD・ラベル生成を行う Rust 実装。
///
/// [`OpenJTalk::new`] で作ったインスタンスは、読み込み済みの辞書を共有します。
///
/// 辞書の更新には `update_global_dictionary`,
/// グローバル辞書のユーザー辞書の解除には `unset_user_dictionary`
/// を使用してください。
#[derive(Debug)]
pub struct OpenJTalk {
    pub(crate) mecab: Mecab,
    pub(crate) dict: Option<Arc<Dictionary>>,
    /// グローバル辞書の更新に追従するかどうか。
    ///
    /// [OpenJTalk::new] で作った場合はグローバル辞書を使っているので追従する。
    /// `from_dictionary` や `from_path` で辞書を明示した場合は、
    /// [update_global_dictionary] で勝手に差し替えられては困るので追従しない。
    pub(crate) follows_global: bool,
}

impl OpenJTalk {
    /// 現在のグローバルな辞書を使って、`OpenJTalk` インスタンスを作成します。
    ///
    /// `embed-dictionary` feature が有効である場合、バイナリ埋め込みされた辞書を自動で使用します。
    ///
    /// グローバル辞書は [`update_global_dictionary`] で更新できます。
    pub fn new() -> Result<Self, HaqumeiError> {
        let initial_dict = GLOBAL_MECAB_DICTIONARY.load_full();

        if !initial_dict.model.is_initialized() {
            return Err(HaqumeiError::GlobalDictionaryNotInitialized);
        }

        let mecab = Mecab::from_model(&initial_dict.model)?;

        Ok(Self {
            mecab,
            dict: Some(initial_dict),
            follows_global: true,
        })
    }

    pub(crate) fn ensure_dictionary_is_latest(&mut self) -> Result<(), HaqumeiError> {
        if !self.follows_global {
            return Ok(());
        }

        let latest_dict = GLOBAL_MECAB_DICTIONARY.load();

        if let Some(active_dict) = &self.dict
            && !Arc::ptr_eq(active_dict, &*latest_dict)
        {
            log::info!("OpenJTalk instance detected a dictionary update. Re-initializing Mecab.");
            let new_mecab = Mecab::from_model(&latest_dict.model)?;

            self.dict = Some(latest_dict.clone());
            self.mecab = new_mecab;
        }
        Ok(())
    }

    /// [Dictionary] から [OpenJTalk] を作成します。
    pub fn from_dictionary(dict: Dictionary) -> Result<Self, HaqumeiError> {
        let mecab = Mecab::from_model(&dict.model)?;

        Ok(Self {
            mecab,
            dict: Some(Arc::new(dict)),
            follows_global: false,
        })
    }

    /// `Arc` でラップされた [Dictionary] からインスタンスを作成します。
    pub fn from_shared_dictionary(dict: Arc<Dictionary>) -> Result<Self, HaqumeiError> {
        let mecab = Mecab::from_model(&dict.model)?;

        Ok(Self {
            mecab,
            dict: Some(dict),
            follows_global: false,
        })
    }

    /// 指定された辞書の存在するパスから、[OpenJTalk] を生成します。
    pub fn from_path<P: AsRef<Path>>(dict_dir: P) -> Result<Self, HaqumeiError> {
        Self::from_path_inner(dict_dir, None::<P>)
    }

    /// 指定された辞書及びユーザー辞書のパスから、[OpenJTalk] を生成します。
    pub fn from_path_with_userdict<P: AsRef<Path>, Q: AsRef<Path>>(
        dict_dir: P,
        user_dict: Q,
    ) -> Result<Self, HaqumeiError> {
        Self::from_path_inner(dict_dir, Some(user_dict))
    }

    /// 指定された辞書と、0 個以上のユーザー辞書から [OpenJTalk] を生成します。
    pub fn from_paths<P: AsRef<Path>, Q: AsRef<Path>>(
        dict_dir: P,
        user_dicts: &[Q],
    ) -> Result<Self, HaqumeiError> {
        Self::from_dictionary(Dictionary::from_paths(dict_dir.as_ref(), user_dicts)?)
    }

    fn from_path_inner<P: AsRef<Path>, Q: AsRef<Path>>(
        dict_dir: P,
        user_dict: Option<Q>,
    ) -> Result<Self, HaqumeiError> {
        // [Dictionary] を経由することで、パスの解決とプラットフォーム差の吸収を
        // 一箇所に寄せている。また [Dictionary] を保持しておくことで、`*_batch` の
        // 各ワーカーがグローバル辞書ではなくこの辞書からインスタンスを作れる。
        // `user_dict` は所有権を持つ型 (tempfile の `TempPath` など) でも渡せる。
        // `map` で消費すると drop が走ってファイルが消えることがあるため、借用する。
        let dict = Dictionary::from_path(
            dict_dir.as_ref().to_path_buf(),
            user_dict.as_ref().map(|p| p.as_ref().to_path_buf()),
        )?;

        Self::from_dictionary(dict)
    }

    /// OpenJTalk のテキスト処理フロントエンドを実行する。
    pub fn run_frontend(&mut self, text: &str) -> Result<Vec<NjdFeature>, HaqumeiError> {
        self.run_frontend_with_numeral_reading(text, false)
    }

    pub(crate) fn run_frontend_with_numeral_reading(
        &mut self,
        text: &str,
        modify_numeral_reading: bool,
    ) -> Result<Vec<NjdFeature>, HaqumeiError> {
        self.ensure_dictionary_is_latest()?;

        if text.is_empty() {
            return Ok(Vec::new());
        }

        let mecab_features = self.run_mecab(text)?;
        self.run_njd_from_mecab_with_numeral_reading(&mecab_features, modify_numeral_reading)
    }

    /// OpenJTalk のテキスト処理フロントエンドを実行する。
    /// [NjdFeature] だけでなく、Mecab の解析結果の [MecabMorph] のリスト
    /// を取得することができる。
    pub fn run_frontend_detailed(
        &mut self,
        text: &str,
    ) -> Result<(Vec<NjdFeature>, Vec<MecabMorph>), HaqumeiError> {
        self.run_frontend_detailed_with_numeral_reading(text, false, false)
    }

    pub(crate) fn run_frontend_detailed_with_numeral_reading(
        &mut self,
        text: &str,
        modify_numeral_reading: bool,
        protect_user_dict_readings: bool,
    ) -> Result<(Vec<NjdFeature>, Vec<MecabMorph>), HaqumeiError> {
        self.run_frontend_with_morphs(
            text,
            modify_numeral_reading,
            protect_user_dict_readings,
            true,
        )
    }

    pub(crate) fn run_frontend_with_morphs(
        &mut self,
        text: &str,
        modify_numeral_reading: bool,
        protect_user_dict_readings: bool,
        split_symbols: bool,
    ) -> Result<(Vec<NjdFeature>, Vec<MecabMorph>), HaqumeiError> {
        self.ensure_dictionary_is_latest()?;

        if text.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }

        let mecab_morphs = self.run_mecab_with_symbol_split(text, split_symbols)?;
        Ok((
            self.run_njd_from_morphs(
                &mecab_morphs,
                modify_numeral_reading,
                protect_user_dict_readings,
            )?,
            mecab_morphs,
        ))
    }

    /// テキストから [haqumei_jlabel::Label] のリストとしてフルコンテキストラベルを抽出する。
    ///
    /// pyopenjtalk の `extract_fullcontext` に相当する文字列が
    /// 欲しい場合は、 `extract_fullcontext_string` を使用してください。
    pub fn extract_fullcontext(&mut self, text: &str) -> Result<Vec<Label>, HaqumeiError> {
        if text.is_empty() {
            self.ensure_dictionary_is_latest()?;
            return Ok(Vec::new());
        }

        let njd_features = self.run_frontend(text.as_ref())?;
        self.extract_fullcontext_labels(&njd_features)
    }

    /// テキストから jpcommon が出力するフルコンテキストラベルを抽出する。
    /// pyopenjtalk の `extract_fullcontext` に相当します。
    ///
    /// 構造化された [haqumei_jlabel::Label] が欲しい場合は、 `extract_fullcontext` を使用してください。
    pub fn extract_fullcontext_string(&mut self, text: &str) -> Result<Vec<String>, HaqumeiError> {
        if text.is_empty() {
            self.ensure_dictionary_is_latest()?;
            return Ok(Vec::new());
        }

        let njd_features = self.run_frontend(text.as_ref())?;
        self.extract_fullcontext_labels(&njd_features)
            .map(|labels| labels.into_iter().map(|l| l.to_string()).collect())
    }

    /// 入力テキストを音素列 (フラットなリスト) に変換します。
    ///
    /// pyopenjtalk と同様の出力を得るためには、`.join(" ")` をチェーンしてください。
    ///
    /// # Examples
    /// ```rust
    /// use haqumei::OpenJTalk;
    ///
    /// let mut open_jtalk = OpenJTalk::new().unwrap();
    /// // Ok(["k", "o", "N", "n", "i", "ch", "i", "w", "a"])
    /// println!("{:?}", open_jtalk.g2p("こんにちは"));
    /// ```
    pub fn g2p(&mut self, text: &str) -> Result<Vec<Phoneme>, HaqumeiError> {
        self.ensure_dictionary_is_latest()?;

        if text.is_empty() {
            return Ok(Vec::new());
        }

        let mecab_features = self.run_mecab(text.as_ref())?;
        let njd_features = self.run_njd_from_mecab(&mecab_features)?;

        if njd_features.is_empty() {
            return Ok(Vec::new());
        }

        self.extract_phonemes(&njd_features)
    }

    /// より詳細な G2P 変換。
    ///
    /// - 既知語: 通常の音素列 (読点などは `pau`)
    /// - 未知語: `unk`
    /// - 空白等: `sp` (Space)
    ///
    /// pyopenjtalk のような音素文字列を得るためには、`.join(" ")` をチェーンしてください。
    ///
    /// # Examples
    /// ```rust
    /// use haqumei::OpenJTalk;
    ///
    /// let mut open_jtalk = OpenJTalk::new().unwrap();
    /// // Ok(["k", "o", "N", "n", "i", "ch", "i", "w", "a", "sp", "unk", "m", "e", "N"])
    /// println!("{:?}", open_jtalk.g2p_detailed("こんにちは 𰻞𰻞麺"));
    /// ```
    pub fn g2p_detailed(&mut self, text: &str) -> Result<Vec<Phoneme>, HaqumeiError> {
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
    pub fn g2k(&mut self, text: &str) -> Result<String, HaqumeiError> {
        self.ensure_dictionary_is_latest()?;

        if text.is_empty() {
            return Ok(String::new());
        }

        let mecab_features = self.run_mecab(text.as_ref())?;
        let njd_features = self.run_njd_from_mecab(&mecab_features)?;

        if njd_features.is_empty() {
            return Ok(String::new());
        }

        let kana_string: String = njd_features
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

    /// 入力テキストを単語（形態素）ごとのカタカナリストに変換します。
    pub fn g2k_per_word(&mut self, text: &str) -> Result<Vec<String>, HaqumeiError> {
        if text.is_empty() {
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
    /// 音素ごとにピッチ情報が欲しい場合は、[OpenJTalk::g2p_prosody_with_options] を使用してください。
    ///
    /// 出力には通常の音素に加えて、以下の制御記号が含まれます：
    ///
    /// | 記号 | 意味 | 出現位置 |
    /// | :--- | :--- | :--- |
    /// | `^` | 発話の開始 (BOS) | 文頭 |
    /// | `$` | 発話の終結 (EOS) | 文末 |
    /// | `?` | 疑問文の終結 (？) | 文中 |
    /// | `!` | 感嘆の終結 (！, 独自拡張) | 文中 |
    /// | `_` | ポーズ・読点 (、) | 文中 |
    /// | `#` | アクセント句境界 | 文中 |
    /// | `[` | ピッチ上昇 (句頭) | 句の開始付近 |
    /// | `]` | ピッチ下降 (アクセント核) | 核モーラの直後 |
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
        let njd_features = self.run_frontend(text)?;

        let labels = self.extract_fullcontext_labels(&njd_features)?;
        if labels.is_empty() {
            return Ok(Vec::new());
        }

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

    /// 単語（形態素）単位に分割された音素リストを返します。
    ///
    /// # Returns
    ///
    /// 単語ごとの音素リストのベクタ。
    ///
    /// (e.g., [["k", "o", "N", "n", "i", "ch", "i", "w", "a"], ["pau"], ["s", "e", "k", "a", "i"]])
    pub fn g2p_per_word(&mut self, text: &str) -> Result<Vec<Vec<Phoneme>>, HaqumeiError> {
        self.ensure_dictionary_is_latest()?;

        if text.is_empty() {
            return Ok(Vec::new());
        }

        let mecab_features = self.run_mecab(text)?;
        let njd_features = self.run_njd_from_mecab(&mecab_features)?;

        if njd_features.is_empty() {
            return Ok(Vec::new());
        }

        // 区間を捨てるので `njd_spans` は空で渡す
        let seeds = self.g2p_seed_inner(&njd_features, &[], default_is_non_pause_symbol)?;

        Ok(seeds.into_iter().map(|s| s.phonemes).collect())
    }

    /// 入力テキストの形態素ごとの音素マッピングを返します。
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
    /// use haqumei::OpenJTalk;
    ///
    /// let mut open_jtalk = OpenJTalk::new().unwrap();
    /// let mapping = open_jtalk.g2p_mapping("𰻞𰻞麺 お冷を頼んだ").unwrap();
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
        self.ensure_dictionary_is_latest()?;

        if text.is_empty() {
            return Ok(Vec::new());
        }

        let morphs = self.run_mecab_detailed(text)?;

        // 本来の Open JTalk パイプラインと同じ状態にして渡す
        let njd_features = self.run_njd_from_mecab(
            morphs
                .iter()
                .filter(|m| !m.is_ignored)
                .map(|morph| morph.feature.as_str()),
        )?;

        if njd_features.is_empty() {
            return Ok(Vec::new());
        }

        let njd_spans = crate::njd_char_spans(&njd_features, &morphs);
        let seeds = self.g2p_seed_inner(&njd_features, &njd_spans, default_is_non_pause_symbol)?;

        self.make_phoneme_mapping(morphs, seeds)
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
    /// use haqumei::OpenJTalk;
    ///
    /// let mut open_jtalk = OpenJTalk::new().unwrap();
    /// let mapping = open_jtalk.g2p_mapping_detailed("薄明").unwrap();
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
            return Ok(Vec::new());
        }
        let (njd_features, morphs) = self.run_frontend_detailed(text)?;

        let njd_spans = crate::njd_char_spans(&njd_features, &morphs);
        let mapping =
            self.g2p_mapping_inner(&njd_features, &njd_spans, default_is_non_pause_symbol)?;

        self.make_phoneme_mapping(morphs, mapping)
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
            return Ok(Vec::new());
        }
        let (njd_features, morphs) = self.run_frontend_detailed(text)?;

        let njd_spans = crate::njd_char_spans(&njd_features, &morphs);
        let mapping =
            self.g2p_mapping_prosody_inner(&njd_features, &njd_spans, default_is_non_pause_symbol)?;

        self.make_phoneme_mapping(morphs, mapping)
    }

    /// MeCab解析を実行し、feature のリストを返します。
    pub fn run_mecab(&mut self, text: &str) -> Result<Vec<String>, HaqumeiError> {
        self.ensure_dictionary_is_latest()?;

        let normalized = self.text2mecab_string(text)?;
        let analysis = self.mecab.analyze(&normalized)?;
        let mut features = Vec::with_capacity(analysis.best_path.len());
        for &index in &analysis.best_path {
            let node = &analysis.nodes[index];
            if node.feature.contains("記号,空白") {
                continue;
            }
            let surface = &normalized[node.byte_span.clone()];
            // 未知記号と一語にまとめられた「！」「？」は、NJDで読点に変わる。
            // 詳細解析と同じ記号の素性に分け、疑問・感嘆の区別を保つ。
            if node.is_unknown
                && surface.contains(['！', '？'])
                && surface.chars().all(|c| !c.is_alphanumeric())
                && surface.chars().count() > 1
            {
                for ch in surface.chars() {
                    let symbol = ch.to_string();
                    let known = get_known_symbol_feature(&symbol);
                    if known.is_some() && ch.is_whitespace() {
                        continue;
                    }
                    features.push(format!("{},{}", symbol, known.unwrap_or(&node.feature)));
                }
            } else {
                features.push(format!("{},{}", surface, node.feature));
            }
        }
        Ok(features)
    }

    /// MeCab解析を実行し、詳細な形態素情報を返します。
    ///
    /// 空白や記号など、通常 OpenJTalk で無視されるトークンも含め、
    /// 全ての解析結果を返します。
    /// `text2mecab` を通した文字列を返します。
    ///
    /// [`MecabMorph::char_span`] と [`LatticeNode::char_span`] が指すのはこの文字列で、
    /// 入力とは文字数が変わることがあります。`text2mecab` は制御文字と範囲外の文字を
    /// 出力せず、半角カナと濁点の並び (`ｶﾞ`) を 1 文字にまとめ、ASCII を全角にします。
    ///
    /// 出力をもう一度通しても変わりません。変換表の右辺はどれも左辺に現れないので、
    /// [`OpenJTalk::run_mecab_detailed`] に渡し直しても同じ文字列になります。
    pub fn text2mecab_string(&self, text: &str) -> Result<String, HaqumeiError> {
        let normalized = haqumei_jpreprocess::normalize_text_for_open_jtalk(text);
        Ok(normalized)
    }

    pub fn run_mecab_detailed(&mut self, text: &str) -> Result<Vec<MecabMorph>, HaqumeiError> {
        self.run_mecab_with_symbol_split(text, true)
    }

    fn run_mecab_with_symbol_split(
        &mut self,
        text: &str,
        split_symbols: bool,
    ) -> Result<Vec<MecabMorph>, HaqumeiError> {
        self.ensure_dictionary_is_latest()?;
        let normalized = self.text2mecab_string(text)?;
        let analysis = self.mecab.analyze(&normalized)?;
        let mut cursor = CharCursor::new(normalized.as_bytes());
        let mut results = Vec::new();
        for &index in &analysis.best_path {
            let node = &analysis.nodes[index];
            let surface = &normalized[node.byte_span.clone()];
            let char_start = cursor.char_at(node.byte_span.start);
            let char_end = cursor.char_at(node.byte_span.end);
            let is_ignored = node.feature.contains("記号,空白");
            let base = MecabMorph {
                surface: surface.to_owned(),
                feature: format!("{},{}", surface, node.feature),
                left_id: node.left_id,
                right_id: node.right_id,
                pos_id: node.pos_id,
                word_cost: node.word_cost,
                char_span: char_start..char_end,
                dictionary_index: node.dictionary_index,
                is_unknown: node.is_unknown,
                is_ignored,
            };
            if node.is_unknown
                && (split_symbols || surface.contains(['！', '？']))
                && surface.chars().all(|c| !c.is_alphanumeric())
                && surface.chars().count() > 1
            {
                for (offset, ch) in surface.chars().enumerate() {
                    let surface = ch.to_string();
                    let known = get_known_symbol_feature(&surface);
                    results.push(MecabMorph {
                        feature: format!("{},{}", surface, known.unwrap_or(&node.feature)),
                        surface,
                        char_span: char_start + offset..char_start + offset + 1,
                        dictionary_index: if known.is_some() {
                            0
                        } else {
                            node.dictionary_index
                        },
                        is_unknown: known.is_none(),
                        is_ignored: if known.is_some() {
                            ch.is_whitespace()
                        } else {
                            is_ignored
                        },
                        ..base.clone()
                    });
                }
            } else {
                results.push(base);
            }
        }
        Ok(results)
    }

    /// MeCab の feature 文字列の列を `mecab2njd` に渡し、[`NjdFeature`] の列を
    /// 返します。
    ///
    /// [`OpenJTalk::run_frontend`] から MeCab の解析を除いたものです。
    ///
    /// 渡す文字列は [`MecabMorph::feature`] と同じ形でなければなりません。表層形が
    /// 先頭に付いていないと `mecab2njd` が列を 1 つずらして読みます。[`LatticeNode`]
    /// から組むときは `format!("{},{}", node.surface, node.feature)` にします。
    ///
    /// [`Haqumei`] が持つ読みの補正は動きません。
    ///
    /// [`Haqumei`]: crate::Haqumei
    pub fn run_njd_from_mecab<'a, I>(
        &mut self,
        mecab_features: I,
    ) -> Result<Vec<NjdFeature>, HaqumeiError>
    where
        I: IntoIterator,
        I::Item: AsRef<str> + 'a,
    {
        self.run_njd_from_mecab_with_numeral_reading(mecab_features, false)
    }

    pub(crate) fn run_njd_from_mecab_with_numeral_reading<'a, I>(
        &mut self,
        mecab_features: I,
        modify_numeral_reading: bool,
    ) -> Result<Vec<NjdFeature>, HaqumeiError>
    where
        I: IntoIterator,
        I::Item: AsRef<str> + 'a,
    {
        let raw: Vec<_> = mecab_features.into_iter().collect();
        let borrowed: Vec<&str> = raw.iter().map(AsRef::as_ref).collect();
        njd::run_frontend(&borrowed, modify_numeral_reading, &[])
    }

    pub(crate) fn run_njd_from_morphs(
        &mut self,
        morphs: &[MecabMorph],
        modify_numeral_reading: bool,
        protect_user_dict_readings: bool,
    ) -> Result<Vec<NjdFeature>, HaqumeiError> {
        let (raw, protected): (Vec<_>, Vec<_>) = morphs
            .iter()
            .filter(|m| !m.is_ignored)
            .map(|m| {
                (
                    m.feature.as_str(),
                    protect_user_dict_readings && m.is_from_user_dictionary(),
                )
            })
            .unzip();
        njd::run_frontend(&raw, modify_numeral_reading, &protected)
    }

    /// NJD の特徴からフルコンテキストラベル文字列を生成します。
    pub fn make_label(&mut self, features: &[NjdFeature]) -> Result<Vec<String>, HaqumeiError> {
        Ok(self
            .extract_fullcontext_labels(features)?
            .into_iter()
            .map(|label| label.to_string())
            .collect())
    }

    pub(crate) fn extract_fullcontext_labels(
        &mut self,
        features: &[NjdFeature],
    ) -> Result<Vec<Label>, HaqumeiError> {
        njd::extract_fullcontext_labels(features)
    }

    /// NJD の特徴から、発話両端の無音を除いた音素列を返します。
    pub fn extract_phonemes(
        &mut self,
        features: &[NjdFeature],
    ) -> Result<Vec<Phoneme>, HaqumeiError> {
        njd::extract_phonemes(features)
    }

    impl_batch_method_openjtalk!(
        /// 複数のテキストに対して `run_frontend` を実行します。
        run_frontend_batch => run_frontend -> Vec<NjdFeature>
    );

    impl_batch_method_openjtalk!(
        /// 複数のテキストに対して `run_frontend_detailed` を実行します。
        run_frontend_detailed_batch => run_frontend_detailed -> (Vec<NjdFeature>, Vec<MecabMorph>)
    );

    impl_batch_method_openjtalk!(
        /// 複数のテキストに対して `g2p` を実行します。
        g2p_batch => g2p -> Vec<Phoneme>
    );

    impl_batch_method_openjtalk!(
        /// すべてのトークンを保持する詳細な G2P 変換のバッチ処理。
        ///
        /// - 既知語: 通常の音素列 (読点などは `pau`)
        /// - 未知語: `unk`
        /// - 空白等: `sp` (Space)
        g2p_detailed_batch => g2p_detailed -> Vec<Phoneme>
    );

    impl_batch_method_openjtalk!(
        /// カタカナ変換のバッチ処理。
        g2k_batch => g2k -> String
    );

    impl_batch_method_openjtalk!(
        /// 単語ごとに分割されたカタカナ変換のバッチ処理。
        g2k_per_word_batch => g2k_per_word -> Vec<String>
    );

    impl_batch_method_openjtalk!(
        /// 入力テキストのリストから、プロソディ記号付き音素リストを抽出するバッチ処理。
        g2p_prosody_batch => g2p_prosody -> Vec<String>
    );

    impl_batch_method_openjtalk!(
        /// 入力テキストのリストから、プロソディ記号付き音素リストを抽出するバッチ処理。
        g2p_prosody_with_options_batch => g2p_prosody_with_options(format: ProsodyFormat) -> Vec<String>
    );

    impl_batch_method_openjtalk!(
        /// 単語ごとに分割された音素リストのバッチ処理。
        g2p_per_word_batch => g2p_per_word -> Vec<Vec<Phoneme>>
    );

    impl_batch_method_openjtalk!(
        /// 形態素ごとの未知語を含めたより詳細な音素マッピングのバッチ処理。
        ///
        /// MeCab による形態素解析の結果と 1:1 に対応するマッピング情報を生成します。
        ///
        /// - 既知語: 通常の音素列 (読点などは `pau`)
        /// - 未知語: `unk`
        /// - 空白等: `sp` (Space)
        g2p_mapping_batch => g2p_mapping -> Vec<WordPhonemeMap>
    );

    impl_batch_method_openjtalk!(
        /// 形態素ごとの未知語や NJD の情報を含めたより詳細な音素マッピングのバッチ処理。
        ///
        /// MeCab による形態素解析の結果と 1:1 に対応するマッピング情報を生成します。
        ///
        /// - 既知語: 通常の音素列 (読点などは `pau`)
        /// - 未知語: `unk`
        /// - 空白等: `sp` (Space)
        g2p_mapping_detailed_batch => g2p_mapping_detailed -> Vec<WordPhonemeDetail>
    );

    impl_batch_method_openjtalk!(
        /// プロソディ記号付き音素マッピングのバッチ処理。
        g2p_mapping_prosody_batch => g2p_mapping_prosody -> Vec<WordPhonemeProsody>
    );

    impl_batch_method_openjtalk!(
        /// haqumei_jlabel::Label を返すフルコンテキストラベル抽出のバッチ処理。
        extract_fullcontext_batch => extract_fullcontext -> Vec<Label>
    );

    impl_batch_method_openjtalk!(
        /// フルコンテキストラベル抽出のバッチ処理。
        extract_fullcontext_string_batch => extract_fullcontext_string -> Vec<String>
    );
}
