pub mod dictionary;
mod jp_common;
mod mapping;
mod mecab;
mod model;
mod njd;

#[cfg(test)]
mod tests;

use crate::errors::HaqumeiError;
use crate::ffi;
use crate::open_jtalk::{
    jp_common::JpCommon,
    model::MecabModel,
    njd::{Njd, apply_plus_rules, njd_to_features},
};
use crate::utils::default_is_non_pause_symbol;
use crate::{NjdFeature, WordPhonemeDetail, WordPhonemeMap, WordPhonemePair};

use arc_swap::ArcSwap;
use mecab::Mecab;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use std::cell::Cell;
use std::ffi::{CStr, CString, c_char};
use std::marker::PhantomData;
use std::path::Path;
use std::sync::{Arc, LazyLock};

pub use dictionary::{Dictionary, MecabDictIndexCompiler};

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

/// # Safety
///
/// これらの実装は、以下の条件を満たす。
///
/// - `Drop` 実装は純粋に `free` 相当の解放のみである。
///   - (TLSを触っておらず、スレッドセーフな `free` を通して唯一の C/C++ のヒープ上のリソースを、Rustの RAII モデルに則って安全に解放できる)
/// - C/C++ 側が Thread Local Storage に依存していない。
/// - C/C++ 側で非 atomic な参照カウントや非同期変更されうる可変なグローバル状態を持たない。
unsafe impl Send for Mecab {}
unsafe impl Send for Njd {}
unsafe impl Send for JpCommon {}

/// # Safety
///
/// `MecabModel` は `Dictionary` を通しアクセスされる共有オブジェクトとして `Send` / `Sync` を実装している。
///
/// - `Drop` 実装は純粋に `free` 相当の解放のみである。
///   - (TLSを触っておらず、スレッドセーフな `free` を通して唯一の C/C++ のヒープ上のリソースを、Rustの RAII モデルに則って安全に解放できる)
/// - C/C++ 側が Thread Local Storage に依存していない。
/// - C/C++ 側で非 atomic な参照カウントや非同期変更されうる可変なグローバル状態を持たない。
/// - これは不変オブジェクトとして、`Dictionary` を通して `Arc` で保護されるため、スレッド間で `*mut mecab_model_t` は変更されない。
unsafe impl Send for MecabModel {}
unsafe impl Sync for MecabModel {}

/// `Haqumei::new()`, `OpenJTalk::new()` で使用されるグローバル辞書を更新します (設定します)。
///
/// この関数を呼び出した後、新たに `Haqumei::new()`, `OpenJTalk::new()` を呼び出す際には、この辞書が使用されるようになります。
/// 既存のインスタンスについては、次のメソッド呼び出し時に新しい辞書に更新されます。
pub fn update_global_dictionary(new_dict: Dictionary) {
    GLOBAL_MECAB_DICTIONARY.store(Arc::new(new_dict));
}

/// `Haqumei::new()`, `OpenJTalk::new()` で使用されるグローバル辞書のユーザー辞書を外します。
pub fn unset_user_dictionary() -> Result<(), HaqumeiError> {
    GLOBAL_MECAB_DICTIONARY.store(Arc::new(Dictionary::from_path(
        &GLOBAL_MECAB_DICTIONARY.load_full().dict_dir,
        None,
    )?));
    Ok(())
}

/// Open JTalk をバインディングしたG2Pエンジン。
///
/// [`pyopenjtalk-plus`](https://github.com/tsukumijima/pyopenjtalk-plus) の辞書を使用しています。
///
/// フルコンテキストラベルを経由せず、JPCommon で構築された内部ポインタを追って
/// g2p を行えるものはそのように実装されているため、他の Open JTalk バインディング実装より若干高速です。
#[derive(Debug)]
pub struct OpenJTalk {
    pub(crate) mecab: Mecab,
    pub(crate) njd: Njd,
    pub(crate) jp_common: JpCommon,
    pub(crate) dict: Option<Arc<Dictionary>>,
    _marker: PhantomData<Cell<()>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MecabMorph {
    /// 形態素の表層形。
    pub surface: String,

    /// MeCab が出力した特徴量文字列。
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

    /// `pyopenjtalk` のパイプラインで無視される対象かどうか。 ("記号,空白")
    ///
    /// ここでは、`pyopenjtalk` は Mecab の出力に対して、どのように必要のないトークンを除去していたか、
    /// ということをフラグによって明確にするものであって、JPCommon の音素割り当てと実際には関係がありません。
    pub is_ignored: bool,
}

impl OpenJTalk {
    /// 現在のグローバルな辞書を使って、`OpenJTalk` インスタンスを作成します。
    ///
    /// `embed-dictionary` feature が有効である場合、バイナリ埋め込みされた辞書を自動で使用します。
    ///
    /// グローバル辞書は `update_global_mecab_dictionary` を使っていつでも更新できます。
    pub fn new() -> Result<Self, HaqumeiError> {
        let initial_dict = GLOBAL_MECAB_DICTIONARY.load_full();

        if !initial_dict.model.is_initialized() {
            return Err(HaqumeiError::GlobalDictionaryNotInitialized);
        }

        let mecab = Mecab::from_model(&initial_dict.model)?;
        let njd = Njd::new()?;
        let jp_common = JpCommon::new()?;

        Ok(Self {
            mecab,
            njd,
            jp_common,
            dict: Some(initial_dict),
            _marker: PhantomData,
        })
    }

    pub(crate) fn ensure_dictionary_is_latest(&mut self) -> Result<(), HaqumeiError> {
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

    pub fn from_dictionary(dict: Dictionary) -> Result<Self, HaqumeiError> {
        let mecab = Mecab::from_model(&dict.model)?;
        let njd = Njd::new()?;
        let jp_common = JpCommon::new()?;

        Ok(Self {
            mecab,
            njd,
            jp_common,
            dict: Some(Arc::new(dict)),
            _marker: PhantomData,
        })
    }

    /// `Arc` でラップされた [Dictionary] からインスタンスを作成します。
    pub fn from_shared_dictionary(dict: Arc<Dictionary>) -> Result<Self, HaqumeiError> {
        let mecab = Mecab::from_model(&dict.model)?;
        let njd = Njd::new()?;
        let jp_common = JpCommon::new()?;

        Ok(Self {
            mecab,
            njd,
            jp_common,
            dict: Some(dict),
            _marker: PhantomData,
        })
    }

    pub fn from_path<P: AsRef<Path>>(dict_dir: P) -> Result<Self, HaqumeiError> {
        Self::from_path_inner(dict_dir, None::<P>)
    }

    pub fn from_path_with_userdict<P: AsRef<Path>, Q: AsRef<Path>>(
        dict_dir: P,
        user_dict: Q,
    ) -> Result<Self, HaqumeiError> {
        Self::from_path_inner(dict_dir, Some(user_dict))
    }

    fn from_path_inner<P: AsRef<Path>, Q: AsRef<Path>>(
        dict_dir: P,
        user_dict: Option<Q>,
    ) -> Result<Self, HaqumeiError> {
        let mecab = Mecab::new()?;
        let njd = Njd::new()?;
        let jp_common = JpCommon::new()?;

        let dict_dir = dict_dir.as_ref();
        let user_dict = user_dict.as_ref();

        if !dict_dir.exists() {
            return Err(HaqumeiError::DictionaryNotFound {
                path: dict_dir.to_path_buf(),
            });
        }

        if let Some(user_dict) = user_dict
            && !user_dict.as_ref().exists()
        {
            return Err(HaqumeiError::DictionaryNotFound {
                path: dict_dir.to_path_buf(),
            });
        }

        let path_to_cstring = |p: &Path| -> Result<CString, HaqumeiError> {
            let p = p.canonicalize()?;
            let path_str = p.as_os_str();

            #[cfg(unix)]
            {
                use std::os::unix::ffi::OsStrExt;

                CString::new(path_str.as_bytes()).map_err(|_| {
                    HaqumeiError::InvalidDictionaryPath(path_str.to_string_lossy().to_string())
                })
            }

            #[cfg(windows)]
            {
                let mut s = path_str.to_str().ok_or_else(|| {
                    HaqumeiError::InvalidDictionaryPath(path_str.to_string_lossy().to_string())
                })?;

                if let Some(stripped) = s.strip_prefix(r"\\?\") {
                    s = stripped;
                }

                CString::new(s).map_err(|_| HaqumeiError::InvalidDictionaryPath(s.to_string()))
            }
        };

        let c_dict_dir = path_to_cstring(dict_dir)?;

        let c_user_dict: Option<CString> = user_dict
            .as_ref()
            .map(|p| path_to_cstring(p.as_ref()))
            .transpose()?;

        let result = unsafe {
            if let Some(user_dict) = c_user_dict.as_ref().filter(|s| !s.to_bytes().is_empty()) {
                ffi::Mecab_load_with_userdic(
                    mecab.inner.as_ptr(),
                    c_dict_dir.as_ptr() as *mut c_char,
                    user_dict.as_ptr() as *mut c_char,
                )
            } else {
                ffi::Mecab_load(mecab.inner.as_ptr(), c_dict_dir.as_ptr() as *mut c_char)
            }
        };

        if result != 1 {
            return Err(HaqumeiError::MecabLoadError);
        }

        Ok(Self {
            mecab,
            njd,
            jp_common,
            dict: None,
            _marker: PhantomData,
        })
    }

    /// OpenJTalk のテキスト処理フロントエンドを実行する。
    pub fn run_frontend(&mut self, text: &str) -> Result<Vec<NjdFeature>, HaqumeiError> {
        self.ensure_dictionary_is_latest()?;

        if text.is_empty() {
            return Ok(Vec::new());
        }

        let mecab_features = self.run_mecab(text)?;
        self.run_njd_from_mecab(&mecab_features)
    }

    /// OpenJTalk のテキスト処理フロントエンドを実行する。
    /// [NjdFeature] だけでなく、Mecab の解析結果の [MecabMorph] のリスト
    /// を取得することができる。
    pub fn run_frontend_detailed(
        &mut self,
        text: &str,
    ) -> Result<(Vec<NjdFeature>, Vec<MecabMorph>), HaqumeiError> {
        self.ensure_dictionary_is_latest()?;

        if text.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }

        let mecab_morphs = self.run_mecab_detailed(text)?;
        Ok((
            self.run_njd_from_mecab(
                mecab_morphs
                    .iter()
                    .filter(|morph| !morph.is_ignored)
                    .map(|morph| &morph.feature),
            )?,
            mecab_morphs,
        ))
    }

    /// テキストからフルコンテキストラベルを抽出する。
    pub fn extract_fullcontext(&mut self, text: &str) -> Result<Vec<String>, HaqumeiError> {
        let njd_features = self.run_frontend(text)?;
        self.make_label(&njd_features)
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
    pub fn g2p(&mut self, text: &str) -> Result<Vec<String>, HaqumeiError> {
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
    pub fn g2p_detailed(&mut self, text: &str) -> Result<Vec<String>, HaqumeiError> {
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
    pub fn g2p_kana_per_word(&mut self, text: &str) -> Result<Vec<String>, HaqumeiError> {
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

    /// 単語（形態素）単位に分割された音素リストを返します。
    ///
    /// # Returns
    ///
    /// 単語ごとの音素リストのベクタ。
    ///
    /// (e.g., [["k", "o", "N", "n", "i", "ch", "i", "w", "a"], ["pau"], ["s", "e", "k", "a", "i"]])
    pub fn g2p_per_word(&mut self, text: &str) -> Result<Vec<Vec<String>>, HaqumeiError> {
        let mapping = self.g2p_pairs(text)?;

        Ok(mapping.into_iter().map(|m| m.phonemes).collect())
    }

    /// 入力テキストの形態素ごとの音素マッピングを返します。
    ///
    /// MeCab による形態素解析の結果と 1:1 に対応するマッピング情報を生成します。
    ///
    /// 記号・未知語の処理: 読点 (`、`) や未知語など、OpenJTalk が発音を生成しないトークンに対しては、
    ///   音素リストとして `["pau"]` が割り当てられます。
    ///
    /// # Examples
    ///
    /// ```rust
    /// use haqumei::OpenJTalk;
    ///
    /// let mut open_jtalk = OpenJTalk::new().unwrap();
    /// let pairs = open_jtalk.g2p_pairs("𰻞𰻞麺＆お冷を頼んだ").unwrap();
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
        self.ensure_dictionary_is_latest()?;

        if text.is_empty() {
            return Ok(Vec::new());
        }

        let mecab_features = self.run_mecab(text.as_ref())?;
        let njd_features = self.run_njd_from_mecab(&mecab_features)?;

        if njd_features.is_empty() {
            return Ok(Vec::new());
        }

        self.g2p_pairs_inner(&njd_features, default_is_non_pause_symbol)
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

        let pairs = self.g2p_pairs_inner(&njd_features, default_is_non_pause_symbol)?;

        self.make_phoneme_mapping(morphs, pairs)
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

        let mapping = self.g2p_mapping_inner(&njd_features, default_is_non_pause_symbol)?;

        self.make_phoneme_mapping(morphs, mapping)
    }

    const BUFFER_SIZE: usize = 16384;

    pub fn run_mecab(&mut self, text: &str) -> Result<Vec<String>, HaqumeiError> {
        self.ensure_dictionary_is_latest()?;

        let c_text = CString::new(text)?;

        let mut buffer = vec![0u8; Self::BUFFER_SIZE];

        let result = unsafe {
            ffi::text2mecab(
                buffer.as_mut_ptr() as *mut i8,
                Self::BUFFER_SIZE,
                c_text.as_ptr(),
            )
        };

        match result {
            ffi::text2mecab_result_t_TEXT2MECAB_RESULT_SUCCESS => {}
            ffi::text2mecab_result_t_TEXT2MECAB_RESULT_RANGE_ERROR => {
                return Err(HaqumeiError::Text2MecabError(
                    "Text is too long".to_string(),
                ));
            }
            ffi::text2mecab_result_t_TEXT2MECAB_RESULT_INVALID_ARGUMENT => {
                return Err(HaqumeiError::Text2MecabError(
                    "Invalid argument for text2mecab".to_string(),
                ));
            }
            _ => {
                return Err(HaqumeiError::Text2MecabError(format!(
                    "Unknown error from text2mecab: {}",
                    result
                )));
            }
        }

        let result =
            unsafe { ffi::Mecab_analysis(self.mecab.inner.as_ptr(), buffer.as_ptr() as *const i8) };

        if result != 1 {
            return Err(HaqumeiError::MecabError(
                "Mecab_analysis failed to parse the text".to_string(),
            ));
        }

        let mut result_vec = Vec::new();
        unsafe {
            let mecab_ptr = self.mecab.inner.as_ptr();
            let lattice = (*mecab_ptr).lattice as *mut ffi::mecab_lattice_t;
            let mut node = ffi::mecab_lattice_get_bos_node(lattice);

            while !node.is_null() {
                let stat = (*node).stat;
                if stat != 2 && stat != 3 {
                    // BOS/EOS 以外
                    let feat_ptr = (*node).feature;
                    if !feat_ptr.is_null() {
                        let c_feature = CStr::from_ptr(feat_ptr);
                        let feature_str = c_feature.to_string_lossy();

                        if !feature_str.contains("記号,空白") {
                            let surface_ptr = (*node).surface;
                            let length = (*node).length as usize;
                            let surface = if !surface_ptr.is_null() && length > 0 {
                                let bytes =
                                    std::slice::from_raw_parts(surface_ptr as *const u8, length);
                                String::from_utf8_lossy(bytes)
                            } else {
                                std::borrow::Cow::Borrowed("")
                            };

                            result_vec.push(format!("{},{}", surface, feature_str));
                        }
                    }
                }
                node = (*node).next;
            }
            ffi::Mecab_refresh(mecab_ptr);
        }

        Ok(result_vec)
    }

    /// MeCab解析を実行し、詳細な形態素情報を返します。
    ///
    /// 空白や記号など、通常 OpenJTalk で無視されるトークンも含め、
    /// 全ての解析結果を返します。
    pub fn run_mecab_detailed(&mut self, text: &str) -> Result<Vec<MecabMorph>, HaqumeiError> {
        self.ensure_dictionary_is_latest()?;

        let c_text = CString::new(text)?;
        let mut buffer = vec![0u8; Self::BUFFER_SIZE];

        let result = unsafe {
            ffi::text2mecab(
                buffer.as_mut_ptr() as *mut i8,
                Self::BUFFER_SIZE,
                c_text.as_ptr(),
            )
        };

        match result {
            ffi::text2mecab_result_t_TEXT2MECAB_RESULT_SUCCESS => {}
            ffi::text2mecab_result_t_TEXT2MECAB_RESULT_RANGE_ERROR => {
                return Err(HaqumeiError::Text2MecabError(
                    "Text is too long".to_string(),
                ));
            }
            ffi::text2mecab_result_t_TEXT2MECAB_RESULT_INVALID_ARGUMENT => {
                return Err(HaqumeiError::Text2MecabError(
                    "Invalid argument for text2mecab".to_string(),
                ));
            }
            _ => {
                return Err(HaqumeiError::Text2MecabError(format!(
                    "Unknown error from text2mecab: {}",
                    result
                )));
            }
        }

        // MeCab Analysis
        let result =
            unsafe { ffi::Mecab_analysis(self.mecab.inner.as_ptr(), buffer.as_ptr() as *const i8) };

        if result != 1 {
            return Err(HaqumeiError::MecabError(
                "Mecab_analysis failed to parse the text".to_string(),
            ));
        }

        // Lattice Traversal
        let morphs = unsafe {
            let mecab_ptr = self.mecab.inner.as_ptr();
            let lattice = (*mecab_ptr).lattice as *mut ffi::mecab_lattice_t;

            let mut node = ffi::mecab_lattice_get_bos_node(lattice);

            let mut results = Vec::new();

            while !node.is_null() {
                let stat = (*node).stat; // 0=NOR, 1=UNK, 2=BOS, 3=EOS

                if stat != 2 && stat != 3 {
                    let surface_ptr = (*node).surface;
                    let length = (*node).length as usize;

                    let surface = if !surface_ptr.is_null() && length > 0 {
                        let bytes = std::slice::from_raw_parts(surface_ptr as *const u8, length);
                        String::from_utf8_lossy(bytes)
                    } else {
                        std::borrow::Cow::Borrowed("")
                    };

                    let feat_ptr = (*node).feature;
                    let raw_feature = if !feat_ptr.is_null() {
                        CStr::from_ptr(feat_ptr).to_string_lossy()
                    } else {
                        std::borrow::Cow::Borrowed("")
                    };

                    // mecab.cpp:
                    // ```cpp
                    // m->feature = (char **) calloc(m->size, sizeof(char *));
                    // int index = 0;
                    // for (const MeCab::Node* node = lattice->bos_node(); node; node = node->next) {
                    //     if(node->stat != MECAB_BOS_NODE && node->stat != MECAB_EOS_NODE) {
                    //         std::string f(node->surface, node->length);
                    //         f += ",";
                    //         f += node->feature;
                    //         m->feature[index] = strdup(f.c_str());
                    //         index++;
                    //     }
                    // }
                    // ```
                    let compatible_feature = format!("{},{}", surface, raw_feature);

                    let is_unknown = stat == 1;
                    let is_ignored = raw_feature.contains("記号,空白");

                    results.push(MecabMorph {
                        surface: surface.to_string(),
                        feature: compatible_feature,
                        left_id: (*node).lcAttr,
                        right_id: (*node).rcAttr,
                        pos_id: (*node).posid,
                        word_cost: (*node).wcost,
                        is_unknown,
                        is_ignored,
                    });
                }

                node = (*node).next;
            }
            results
        };

        unsafe {
            ffi::Mecab_refresh(self.mecab.inner.as_ptr());
        }

        Ok(morphs)
    }

    pub(crate) fn run_njd_from_mecab<'a, I>(
        &mut self,
        mecab_features: I,
    ) -> Result<Vec<NjdFeature>, HaqumeiError>
    where
        I: IntoIterator,
        I::Item: AsRef<str> + 'a,
    {
        let c_strings: Vec<CString> = mecab_features
            .into_iter()
            .map(|s| CString::new(s.as_ref()))
            .collect::<Result<Vec<_>, _>>()?;

        if c_strings.is_empty() {
            return Ok(Vec::new());
        }

        let mut c_string_pointers: Vec<*const c_char> =
            c_strings.iter().map(|cs| cs.as_ptr()).collect();

        unsafe {
            ffi::mecab2njd(
                self.njd.inner.as_mut(),
                c_string_pointers.as_mut_ptr() as *mut *mut c_char,
                c_string_pointers.len() as i32,
            );
            ffi::njd_set_pronunciation(self.njd.inner.as_mut());
        }

        let mut features = njd_to_features(&self.njd);
        apply_plus_rules(&mut features);

        Self::features_to_njd(&features, &mut self.njd)?;

        unsafe {
            ffi::njd_set_digit(self.njd.inner.as_mut());
            ffi::njd_set_accent_phrase(self.njd.inner.as_mut());
            ffi::njd_set_accent_type(self.njd.inner.as_mut());
            ffi::njd_set_unvoiced_vowel(self.njd.inner.as_mut());
            ffi::njd_set_long_vowel(self.njd.inner.as_mut());
        }

        let final_features = njd_to_features(&self.njd);
        unsafe {
            ffi::NJD_refresh(self.njd.inner.as_mut());
        }

        Ok(final_features)
    }

    pub fn make_label(&mut self, features: &[NjdFeature]) -> Result<Vec<String>, HaqumeiError> {
        Self::features_to_njd(features, &mut self.njd)?;

        let (label_size, label_feature_ptr) = unsafe {
            ffi::njd2jpcommon(self.jp_common.inner.as_mut(), self.njd.inner.as_mut());

            ffi::JPCommon_make_label(self.jp_common.inner.as_mut());

            let size = ffi::JPCommon_get_label_size(self.jp_common.inner.as_mut());
            let ptr = ffi::JPCommon_get_label_feature(self.jp_common.inner.as_mut());
            (size, ptr)
        };

        if label_feature_ptr.is_null() {
            return Ok(Vec::new());
        }

        let labels = unsafe {
            let mut result = Vec::with_capacity(label_size as usize);
            for i in 0..(label_size as isize) {
                let label_ptr = *label_feature_ptr.offset(i);
                let c_label = CStr::from_ptr(label_ptr);
                result.push(c_label.to_string_lossy().into_owned());
            }
            result
        };

        unsafe {
            ffi::JPCommon_refresh(self.jp_common.inner.as_mut());
            ffi::NJD_refresh(self.njd.inner.as_mut());
        }

        Ok(labels)
    }

    /// NjdFeature から直接フラットな音素リストを抽出する。
    pub fn extract_phonemes(
        &mut self,
        features: &[NjdFeature],
    ) -> Result<Vec<String>, HaqumeiError> {
        if features.is_empty() {
            return Ok(Vec::new());
        }

        let result = unsafe {
            Self::features_to_njd(features, &mut self.njd)?;

            let jp = self.jp_common.inner.as_mut();
            let njd = self.njd.inner.as_mut();

            ffi::njd2jpcommon(jp, njd);

            // JPCommon_make_label(JPCommon * jpcommon) の部分的な移植
            if !jp.label.is_null() {
                ffi::JPCommonLabel_clear(jp.label);
            } else {
                let ptr = libc::calloc(1, std::mem::size_of::<ffi::JPCommonLabel>());
                if ptr.is_null() {
                    return Err(HaqumeiError::AllocationError("ffi::JPCommonLabel"));
                }
                jp.label = ptr as *mut ffi::JPCommonLabel;
            }

            ffi::JPCommonLabel_initialize(jp.label);

            let mut node = jp.head;
            while !node.is_null() {
                ffi::JPCommonLabel_push_word(
                    jp.label,
                    ffi::JPCommonNode_get_pron(node),
                    ffi::JPCommonNode_get_pos(node),
                    ffi::JPCommonNode_get_ctype(node),
                    ffi::JPCommonNode_get_cform(node),
                    ffi::JPCommonNode_get_acc(node),
                    ffi::JPCommonNode_get_chain_flag(node),
                );

                node = (*node).next;
            }

            let mut result_vec = Vec::with_capacity(features.len() * 3);

            let mut p = (*jp.label).phoneme_head;
            while !p.is_null() {
                let s_ptr = (*p).phoneme;
                if !s_ptr.is_null() {
                    let s = CStr::from_ptr(s_ptr).to_string_lossy().into_owned();
                    result_vec.push(s);
                }
                p = (*p).next;
            }

            ffi::JPCommon_refresh(jp);
            ffi::NJD_refresh(self.njd.inner.as_mut());

            result_vec
        };

        Ok(result)
    }

    pub(crate) fn features_to_njd(
        features: &[NjdFeature],
        njd: &mut Njd,
    ) -> Result<(), HaqumeiError> {
        unsafe {
            ffi::NJD_clear(njd.inner.as_mut());
        }

        for feature in features {
            let c_string = CString::new(feature.string.as_str())?;
            let c_pos = CString::new(feature.pos.as_str())?;
            let c_pos_group1 = CString::new(feature.pos_group1.as_str())?;
            let c_pos_group2 = CString::new(feature.pos_group2.as_str())?;
            let c_pos_group3 = CString::new(feature.pos_group3.as_str())?;
            let c_ctype = CString::new(feature.ctype.as_str())?;
            let c_cform = CString::new(feature.cform.as_str())?;
            let c_orig = CString::new(feature.orig.as_str())?;
            let c_read = CString::new(feature.read.as_str())?;
            let c_pron = CString::new(feature.pron.as_str())?;
            let c_chain_rule = CString::new(feature.chain_rule.as_str())?;

            // SAFETY: このブロックは、`NJDNode` を構築・管理するために C の FFI とやり取りする。
            // 安全性は、`libc::calloc` を用いてメモリ確保を行い、ヌルポインタをチェックしていること、
            // C 関数が文字列のディープコピーを行うため `CString` のデータが安全に扱われていること、
            // そして確保された各ノードが正しく C 側の `NJD` 構造体に移譲されており、
            // Rust がそれを解放しないことで二重解放エラーを防いでいることによって保証されている。
            unsafe {
                let node =
                    libc::calloc(1, std::mem::size_of::<ffi::NJDNode>()) as *mut ffi::NJDNode;
                if node.is_null() {
                    return Err(HaqumeiError::AllocationError("ffi::NJDNode"));
                }

                ffi::NJDNode_initialize(node);

                ffi::NJDNode_set_string(node, c_string.as_ptr());
                ffi::NJDNode_set_pos(node, c_pos.as_ptr());
                ffi::NJDNode_set_pos_group1(node, c_pos_group1.as_ptr());
                ffi::NJDNode_set_pos_group2(node, c_pos_group2.as_ptr());
                ffi::NJDNode_set_pos_group3(node, c_pos_group3.as_ptr());
                ffi::NJDNode_set_ctype(node, c_ctype.as_ptr());
                ffi::NJDNode_set_cform(node, c_cform.as_ptr());
                ffi::NJDNode_set_orig(node, c_orig.as_ptr());
                ffi::NJDNode_set_read(node, c_read.as_ptr());
                ffi::NJDNode_set_pron(node, c_pron.as_ptr());
                ffi::NJDNode_set_acc(node, feature.acc);
                ffi::NJDNode_set_mora_size(node, feature.mora_size);
                ffi::NJDNode_set_chain_rule(node, c_chain_rule.as_ptr());
                ffi::NJDNode_set_chain_flag(node, feature.chain_flag);

                ffi::NJD_push_node(njd.inner.as_mut(), node);
            }
        }

        Ok(())
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
        g2p_batch => g2p -> Vec<String>
    );

    impl_batch_method_openjtalk!(
        /// すべてのトークンを保持する詳細な G2P 変換のバッチ処理。
        ///
        /// - 既知語: 通常の音素列 (読点などは `pau`)
        /// - 未知語: `unk`
        /// - 空白等: `sp` (Space)
        g2p_detailed_batch => g2p_detailed -> Vec<String>
    );

    impl_batch_method_openjtalk!(
        /// カタカナ変換のバッチ処理。
        g2p_kana_batch => g2p_kana -> String
    );

    impl_batch_method_openjtalk!(
        /// 単語ごとに分割されたカタカナ変換のバッチ処理。
        g2p_kana_per_word_batch => g2p_kana_per_word -> Vec<String>
    );

    impl_batch_method_openjtalk!(
        /// 単語ごとに分割された音素リストのバッチ処理。
        g2p_per_word_batch => g2p_per_word -> Vec<Vec<String>>
    );

    impl_batch_method_openjtalk!(
        /// 形態素ごとの音素マッピングのバッチ処理。
        ///
        /// MeCab による形態素解析の結果と 1:1 に対応するマッピング情報を生成します。
        ///
        /// **記号・未知語の処理**: 読点 (`、`) や未知語など、OpenJTalk が発音を生成しないトークンに対しては、
        ///   音素リストとして `["pau"]` が割り当てられます。
        g2p_pairs_batch => g2p_pairs -> Vec<WordPhonemePair>
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
        /// フルコンテキストラベル抽出のバッチ処理。
        extract_fullcontext_batch => extract_fullcontext -> Vec<String>
    );
}

pub fn build_mecab_dictionary<P: AsRef<Path>>(
    path: P,
) -> Result<(), dictionary::DictCompilerError> {
    MecabDictIndexCompiler::new()
        .dict_dir(&path)
        .out_dir(&path)
        .run()
}
