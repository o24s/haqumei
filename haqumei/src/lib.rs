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
mod utils;

use std::{
    borrow::Cow, path::Path, sync::{Arc, LazyLock, Mutex}
};

use moka::sync::Cache;

pub use features::NjdFeature;
pub use open_jtalk::{
    MecabDictIndexCompiler, OpenJTalk, unset_user_dictionary, update_global_dictionary,
};

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use unicode_normalization::UnicodeNormalization;
use vibrato_rkyv::dictionary::PresetDictionaryKind;

use crate::{
    errors::HaqumeiError,
    features::UnidicFeature,
    nani_predict::NaniPredictor,
    open_jtalk::{Dictionary, GLOBAL_MECAB_DICTIONARY, MecabMorph},
    utils::{
        modify_acc_after_chaining, modify_filler_accent, process_odori_features, retreat_acc_nuc,
        vibrato_analysis,
    },
};

static VIBRATO_CACHE: LazyLock<Cache<String, Vec<UnidicFeature>>> =
    LazyLock::new(|| Cache::new(1000));
static NANI_PREDICTOR_CACHE: LazyLock<Cache<NjdFeature, bool>> = LazyLock::new(|| Cache::new(1000));
static NANI_PREDICTOR: LazyLock<Mutex<NaniPredictor>> = LazyLock::new(|| {
    Mutex::new(NaniPredictor::new().expect("Failed to initialize NaniPredictor models"))
});

/// Open JTalk をバインディングしたG2Pエンジン。
///
/// [`pyopenjtalk-plus`](https://github.com/tsukumijima/pyopenjtalk-plus) の辞書を使用しています。
///
/// [Haqumei::with_options], [HaqumeiOptions] を使うことで、出力をカスタマイズできます。
pub struct Haqumei {
    open_jtalk: OpenJTalk,
    tokenizer: Option<vibrato_rkyv::Tokenizer>,
    options: HaqumeiOptions,
}

#[derive(Debug, Clone, Copy)]
pub struct HaqumeiOptions {
    /// 入力テキストを Unicode NFC (Normalization Form C) に正規化する。
    /// 「か + 濁点」などの結合文字を 1文字の「が」に統合します。
    ///
    /// デフォルトで無効になっています。
    pub normalize_unicode: bool,

    /// - フィラーが acc > mora_size のときに、平版型 (acc = 0) にする
    /// - フィラー直後の形態素が名詞だったとき、その前のフィラーに結合しない (chain_flag = 0) ようにする
    ///
    /// デフォルトで有効になっています。
    pub modify_filler_accent: bool,

    /// Nani Predictor を使って、「何」 の読みを修正する。
    ///
    /// デフォルトで無効になっています。
    pub predict_nani: bool,

    /// Unidic を使って、漢字の読みを修正する。
    ///
    /// デフォルトで無効になっています。
    pub modify_kanji_yomi: bool,

    /// 長母音、重母音、撥音がアクセント核に来た場合に、
    /// ひとつ前のモーラにアクセント核がズレるルールを適用する。
    ///
    /// デフォルトで有効になっています。
    pub retreat_acc_nuc: bool,

    /// 品詞「特殊・マス」の直前に接続する動詞にアクセント核がある場合、アクセント核を「ま」に移動させる。
    ///
    ///   書きます → か\[きま\]す, 参ります → ま\[いりま\]す
    ///   書いております → \[か\]いております
    ///
    /// デフォルトで有効になっています。
    pub modify_acc_after_chaining: bool,

    /// 踊り字 (e.g., 々, ヽ, ヾ) の展開を有効にする。
    ///
    /// デフォルトで有効になっています。
    pub process_odoriji: bool,
}

impl Default for HaqumeiOptions {
    fn default() -> Self {
        Self {
            normalize_unicode: false,
            modify_filler_accent: true,
            predict_nani: false,
            modify_kanji_yomi: false,
            retreat_acc_nuc: true,
            modify_acc_after_chaining: true,
            process_odoriji: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WordPhonemeMap {
    pub word: String,
    pub phonemes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WordPhonemeDetail {
    pub word: String,
    pub phonemes: Vec<String>,

    /// MeCab が未知語 (`MECAB_UNK_NODE`) と判定したかどうか。
    pub is_unknown: bool,

    /// `OpenJTalk` のパイプラインで無視される対象かどうか。
    /// (e.g, "記号,空白", 先頭の `ー`)
    pub is_ignored: bool,
}

impl Haqumei {
    pub fn new() -> Result<Self, HaqumeiError> {
        Self::from_open_jtalk(OpenJTalk::new()?, HaqumeiOptions::default())
    }

    /// [HaqumeiOptions] を使って、出力をカスタマイズします。
    pub fn with_options(options: HaqumeiOptions) -> Result<Self, HaqumeiError> {
        Self::from_open_jtalk(OpenJTalk::new()?, options)
    }

    #[inline]
    pub fn from_open_jtalk(
        open_jtalk: OpenJTalk,
        options: HaqumeiOptions,
    ) -> Result<Self, HaqumeiError> {
        let tokenizer = if options.modify_kanji_yomi {
            let Some(cache_dir) = dirs::cache_dir().map(|dir| dir.join("haqumei")) else {
                Err(HaqumeiError::CacheDirectoryNotFound)?
            };

            let kind = PresetDictionaryKind::UnidicCwj;
            log::info!("Downloading {} dictionary...", kind.name());
            let vibrato_dict = vibrato_rkyv::Dictionary::from_preset_with_download(
                kind,
                cache_dir.join(kind.name()),
            )?;
            log::info!("Downloaded {} dictionary.", kind.name());

            Some(vibrato_rkyv::Tokenizer::new(vibrato_dict))
        } else {
            None
        };

        Ok(Haqumei {
            open_jtalk,
            tokenizer,
            options,
        })
    }

    /// [open_jtalk::Dictionary] から [Haqumei] を作ります。
    pub fn from_dictionary(dict: Dictionary, options: HaqumeiOptions) -> Result<Self, HaqumeiError> {
        Self::from_open_jtalk(OpenJTalk::from_dictionary(dict)?, options)
    }

    /// `Arc` でラップされた [Dictionary] から [Haqumei] を作ります
    pub fn from_shared_dictionary(dict: Arc<Dictionary>, options: HaqumeiOptions) -> Result<Self, HaqumeiError> {
        Self::from_open_jtalk(OpenJTalk::from_shared_dictionary(dict)?, options)
    }

    pub fn from_path<P: AsRef<Path>>(
        dict_dir: P,
        user_dict: Option<P>,
        options: HaqumeiOptions,
    ) -> Result<Self, HaqumeiError> {
        Self::from_open_jtalk(OpenJTalk::from_path(dict_dir, user_dict)?, options)
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
    pub fn g2p_detailed(&mut self, text: &str) -> Result<Vec<String>, HaqumeiError> {
        let detailed_mapping = self.g2p_mapping_detailed(text)?;

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

    /// 入力テキストを単語（形態素）ごとのカタカナリストに変換します。
    pub fn g2p_kana_per_word(&mut self, text: &str) -> Result<Vec<String>, HaqumeiError> {
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
        let mapping = self.g2p_mapping(text.as_ref())?;

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
    pub fn g2p_mapping(&mut self, text: &str) -> Result<Vec<WordPhonemeMap>, HaqumeiError> {
        let features = self.run_frontend(text)?;

        if features.is_empty() {
            return Ok(Vec::new());
        }

        self.open_jtalk.g2p_mapping_inner(&features)
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
    /// let mapping = haqumei.g2p_mapping_detailed("𰻞𰻞麺 お冷を頼んだ").unwrap();
    ///
    /// // 出力:
    /// // [WordPhonemeDetail {
    /// //     word: "𰻞𰻞",
    /// //     phonemes: [
    /// //         "unk",
    /// //     ],
    /// //     is_unknown: true,
    /// //     is_ignored: false,
    /// // },
    /// // WordPhonemeDetail {
    /// //     word: "麺",
    /// //     phonemes: [
    /// //         "m",
    /// //         "e",
    /// //         "N",
    /// //     ],
    /// //     is_unknown: false,
    /// //     is_ignored: false,
    /// // },
    /// // WordPhonemeDetail {
    /// //     word: "\u{3000}",
    /// //     phonemes: [
    /// //         "sp",
    /// //     ],
    /// //     is_unknown: false,
    /// //     is_ignored: true,
    /// // },
    /// // WordPhonemeDetail {
    /// //     word: "を",
    /// //     phonemes: [
    /// //         "o",
    /// //     ],
    /// //     is_unknown: false,
    /// //     is_ignored: false,
    /// // },
    /// // WordPhonemeDetail {
    /// //     word: "\u{3000}",
    /// //     phonemes: [
    /// //         "sp",
    /// //     ],
    /// //     is_unknown: false,
    /// //     is_ignored: true,
    /// // },
    /// // WordPhonemeDetail {
    /// //     word: "食べる",
    /// //     phonemes: [
    /// //         "t",
    /// //         "a",
    /// //         "b",
    /// //         "e",
    /// //         "r",
    /// //         "u",
    /// //     ],
    /// //     is_unknown: false,
    /// //     is_ignored: false,
    /// // }]
    /// // ```
    pub fn g2p_mapping_detailed(
        &mut self,
        text: &str,
    ) -> Result<Vec<WordPhonemeDetail>, HaqumeiError> {
        let mut text = Cow::Borrowed(text);
        if self.options.normalize_unicode {
            text = Cow::Owned(text.nfc().collect::<String>());
        }
        let text = text.as_ref();

        let mut run_mecab = || -> Result<(Vec<NjdFeature>, Vec<MecabMorph>, bool), HaqumeiError> {
            let morphs = self.open_jtalk.run_mecab_detailed(text)?;

            let valid_features_str: Vec<String> = morphs
                .iter()
                .filter(|m| !m.is_ignored)
                .map(|m| m.feature.clone())
                .collect();

            let njd_features = self.open_jtalk.run_njd_from_mecab(&valid_features_str)?;
            Ok((njd_features, morphs, valid_features_str.is_empty()))
        };

        let (njd_features, morphs) = {
            let res = if let Some(tokenizer) = &self.tokenizer {
                rayon::join(&mut run_mecab, || {
                    let mut worker = tokenizer.new_worker();
                    vibrato_analysis(&mut worker, text);
                })
                .0
            } else {
                run_mecab()
            };

            let (njd_features, morphs, is_valid_features_empty) = res?;

            if is_valid_features_empty {
                return Ok(morphs
                    .into_iter()
                    .map(|m| WordPhonemeDetail {
                        word: m.surface,
                        phonemes: vec!["sp".to_string()],
                        is_unknown: m.is_unknown,
                        is_ignored: true,
                    })
                    .collect());
            }

            (self.apply_postprocessing(text, njd_features)?, morphs)
        };

        if njd_features.is_empty() {
            return Ok(Vec::new());
        }

        let mapping = self.open_jtalk.g2p_mapping_inner(&njd_features)?;

        let mut result = Vec::with_capacity(morphs.len());
        let mut morph_idx = 0;

        for map in mapping {
            // is_ignored な Morph を先に進めておく
            while let Some(m) = morphs.get(morph_idx) {
                if m.is_ignored {
                    result.push(WordPhonemeDetail {
                        word: m.surface.clone(),
                        phonemes: vec!["sp".to_string()],
                        is_unknown: m.is_unknown,
                        is_ignored: true,
                    });
                    morph_idx += 1;
                } else {
                    break;
                }
            }

            let current_map_word = &map.word;

            if let Some(morph) = morphs.get(morph_idx) {
                if current_map_word == &morph.surface {
                    let mut phonemes = map.phonemes.clone();

                    if morph.is_unknown {
                        // 先頭の長音のような Open JTalk が破棄するもの、
                        // または pau に置き換えられた未知語は unk にしておく
                        if phonemes.is_empty() || phonemes == ["pau"] {
                            phonemes = vec!["unk".to_string()];
                        }
                    }

                    result.push(WordPhonemeDetail {
                        word: map.word.clone(),
                        phonemes,
                        is_unknown: morph.is_unknown,
                        is_ignored: map.phonemes.is_empty(),
                    });
                    morph_idx += 1;
                } else if current_map_word.starts_with(&morph.surface) {
                    let mut is_unknown_word = false;
                    let mut matched_len = 0;

                    while let Some(inner_morph) = morphs.get(morph_idx) {
                        if inner_morph.is_ignored {
                            result.push(WordPhonemeDetail {
                                word: inner_morph.surface.clone(),
                                phonemes: vec!["sp".to_string()],
                                is_unknown: inner_morph.is_unknown,
                                is_ignored: true,
                            });
                            morph_idx += 1;
                            continue;
                        }

                        let remaining = &current_map_word[matched_len..];

                        if remaining.starts_with(&inner_morph.surface) {
                            is_unknown_word |= inner_morph.is_unknown;
                            matched_len += inner_morph.surface.len();
                            morph_idx += 1;

                            if matched_len == current_map_word.len() {
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    let mut phonemes = map.phonemes.clone();

                    if is_unknown_word && (phonemes.is_empty() || phonemes == ["pau"]) {
                        phonemes = vec!["unk".to_string()];
                    }

                    result.push(WordPhonemeDetail {
                        word: map.word.clone(),
                        phonemes,
                        is_unknown: is_unknown_word,
                        is_ignored: map.phonemes.is_empty(),
                    });
                } else {
                    // 四五 という入力に対して 四十五 のようなマッピングになるケースが存在する
                    result.push(WordPhonemeDetail {
                        word: map.word.clone(),
                        phonemes: map.phonemes.clone(),
                        is_unknown: false,
                        is_ignored: map.phonemes.is_empty(),
                    });
                }
            }
        }

        // mapping 終了後、morphs の末尾に無視トークン(空白等)が残っていれば回収する
        while let Some(m) = morphs.get(morph_idx) {
            if m.is_ignored {
                result.push(WordPhonemeDetail {
                    word: m.surface.clone(),
                    phonemes: vec!["sp".to_string()],
                    is_unknown: m.is_unknown,
                    is_ignored: true,
                });
            }
            morph_idx += 1;
        }

        Ok(result)
    }

    pub fn run_frontend(&mut self, text: &str) -> Result<Vec<NjdFeature>, HaqumeiError> {
        let mut text = Cow::Borrowed(text);
        if self.options.normalize_unicode {
            text = Cow::Owned(text.nfc().collect::<String>());
        }
        let text = text.as_ref();

        let njd_features = if let Some(tokenizer) = &self.tokenizer {
            rayon::join(
                || self.open_jtalk.run_frontend(text),
                || {
                    let mut worker = tokenizer.new_worker();
                    vibrato_analysis(&mut worker, text);
                },
            )
            .0
        } else {
            self.open_jtalk.run_frontend(text)
        };

        self.apply_postprocessing(text, njd_features?)
    }

    pub fn extract_fullcontext(&mut self, text: &str) -> Result<Vec<String>, HaqumeiError> {
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
        if options.modify_kanji_yomi {
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
        /// 複数のテキストに対して `g2p` を実行します。
        g2p_batch => g2p -> Vec<String>
    );

    impl_batch_method_haqumei!(
        /// すべてのトークンを保持する詳細な G2P 変換のバッチ処理。
        ///
        /// - 既知語: 通常の音素列 (読点などは `pau`)
        /// - 未知語: `unk`
        /// - 空白等: `sp` (Space)
        g2p_detailed_batch => g2p_detailed -> Vec<String>
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
        /// 単語ごとに分割された音素リストのバッチ処理。
        g2p_per_word_batch => g2p_per_word -> Vec<Vec<String>>
    );

    impl_batch_method_haqumei!(
        /// 形態素ごとの音素マッピングのバッチ処理。
        ///
        /// MeCab による形態素解析の結果と 1:1 に対応するマッピング情報を生成します。
        ///
        /// **記号・未知語の処理**: 読点 (`、`) や未知語など、OpenJTalk が発音を生成しないトークンに対しては、
        ///   音素リストとして `["pau"]` が割り当てられます。
        g2p_mapping_batch => g2p_mapping -> Vec<WordPhonemeMap>
    );

    impl_batch_method_haqumei!(
        /// 形態素ごとの未知語を含めたより詳細な音素マッピングのバッチ処理。
        ///
        /// MeCab による形態素解析の結果と 1:1 に対応するマッピング情報を生成します。
        ///
        /// - 既知語: 通常の音素列 (読点などは `pau`)
        /// - 未知語: `unk`
        /// - 空白等: `sp` (Space)
        g2p_mapping_detailed_batch => g2p_mapping_detailed -> Vec<WordPhonemeDetail>
    );

    impl_batch_method_haqumei!(
        /// フルコンテキストラベル抽出のバッチ処理。
        extract_fullcontext_batch => extract_fullcontext -> Vec<String>
    );
}
