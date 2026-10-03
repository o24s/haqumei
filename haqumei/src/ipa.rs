//! IPA の広い音声表記への変換。
//!
//! [`Haqumei::g2ipa`] は、日本語テキストから単語ごとの音声表記を返します。
//! 各要素の文字列は [`IpaToken::as_str`] で取得できます。
//! [`Haqumei::g2ipa_prosody`] はピッチアクセントと韻律境界も返します。
//!
//! 出力は規則で決められる範囲の broad phonetic transcription であり、
//! 音声を観測した narrow transcription ではない。とくに発話末撥音の `[ɴ]` と
//! 日本語 `/w/` の `[β̞]` は、連続変異を一つの記号へ畳んだ広表記上の約束である。
//! 既存の異音解決オプションは音響モデルへ渡す音素ラベルを選ぶための設定なので、
//! IPA 出力には影響しない。閉鎖の有無を選べない撥音などは、後続音によって分類し、
//! [`IpaToken::Special`] で専用ラベルを返す。専用ラベルは `{N:s}` のように
//! 波括弧で囲み、IPA 記号と区別する。閉鎖位置や閉鎖の有無までは指定しない。
//! r・ch・j の前の撥音を `[n]`、発話末促音を `[ʔ]` とするのも、広表記上の約束である。
//!
//! # 主な参照文献
//!
//! - Maekawa, K. (2023). Production of the utterance-final moraic nasal in
//!   Japanese: A real-time MRI study. *Journal of the International Phonetic
//!   Association*, 53(1), 189–212.
//! - Maekawa, K. (2020). Remarks on Japanese /w/. *ICU Working Papers in
//!   Linguistics*, 10, 45–52.
//! - Maekawa, K. (2023). Articulatory characteristics of the Japanese /r/:
//!   A real-time MRI study. *Proceedings of ICPhS 2023*, 992–996.
//! - Fujimoto, M., Maekawa, K., & Funatsu, S. (2010). Laryngeal characteristics
//!   during the production of geminate consonants. *Proceedings of Interspeech
//!   2010*, 925–928.
//! - Kawahara, S. (2005). Voicing and geminacy in Japanese: An acoustic and
//!   perceptual study. *UMOP*, 31, 87–120.
//! - Kawahara, S. (2015). The phonetics of sokuon, or geminate obstruents.
//!   In *Handbook of Japanese Phonetics and Phonology*, 43–78.
//! - Tanner, J., Sonderegger, M., & Torreira, F. (2019). Durational evidence that
//!   Tokyo Japanese vowel devoicing is not gradient reduction. *Frontiers in
//!   Psychology*, 10, 821.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use std::ops::Range;

use haqumei_macros::ipa_phones;

use crate::{
    Haqumei, Phoneme, PitchAccent, ProsodicPhoneme, WordPhonemeProsody, errors::HaqumeiError,
};

ipa_phones! {
    A = "a",
    LongA = "aː",
    NasalizedA = "ã",
    UnvoicedA = "ḁ",
    B = "b",
    By = "bʲ",
    LongB = "bː",
    LongBy = "bʲː",
    Ch = "tɕ",
    LongCh = "tːɕ",
    D = "d",
    Dy = "dʲ",
    LongD = "dː",
    LongDy = "dʲː",
    Dz = "dz",
    LongDz = "dːz",
    E = "e",
    LongE = "eː",
    NasalizedE = "ẽ",
    UnvoicedE = "e̥",
    F = "ɸ",
    Fy = "ɸʲ",
    LongF = "ɸː",
    LongFy = "ɸʲː",
    G = "ɡ",
    Gw = "ɡʷ",
    Gy = "ɡʲ",
    LongG = "ɡː",
    LongGw = "ɡʷː",
    LongGy = "ɡʲː",
    H = "h",
    Hy = "ç",
    LongH = "hː",
    LongHy = "çː",
    I = "i",
    LongI = "iː",
    NasalizedI = "ĩ",
    UnvoicedI = "i̥",
    J = "dʑ",
    LongJ = "dːʑ",
    K = "k",
    Kw = "kʷ",
    Ky = "kʲ",
    LongK = "kː",
    LongKw = "kʷː",
    LongKy = "kʲː",
    M = "m",
    My = "mʲ",
    N = "n",
    Ng = "ŋ",
    Nq = "ɴ",
    Ny = "nʲ",
    O = "o",
    LongO = "oː",
    NasalizedO = "õ",
    UnvoicedO = "o̥",
    P = "p",
    Py = "pʲ",
    LongP = "pː",
    LongPy = "pʲː",
    R = "ɾ",
    Ry = "ɾʲ",
    S = "s",
    LongS = "sː",
    Sh = "ɕ",
    LongSh = "ɕː",
    T = "t",
    LongT = "tː",
    Ts = "ts",
    LongTs = "tːs",
    Ty = "tʲ",
    LongTy = "tʲː",
    U = "ɯ",
    LongU = "ɯː",
    NasalizedU = "ɯ̃",
    UnvoicedU = "ɯ̥",
    V = "v",
    LongV = "vː",
    W = "β̞",
    Y = "j",
    Z = "z",
    GlottalStop = "ʔ",
}

/// 一つの IPA phone に固定せず、音素の種類や後続音によって分類する専用ラベル。
///
/// [`Self::as_str`] は `{N:s}` などの表記を返します。標準 IPA の記号ではありません。
/// `NBeforeS` などの後続音による分類は、閉鎖位置や閉鎖の有無を指定しません。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum SpecialPhone {
    /// s の前の撥音。`{N:s}`。
    #[cfg_attr(feature = "serde", serde(rename = "{N:s}"))]
    NBeforeS,
    /// sh `[ɕ]` の前の撥音。`{N:sh}`。
    #[cfg_attr(feature = "serde", serde(rename = "{N:sh}"))]
    NBeforeSh,
    /// y `[j]` の前の撥音。`{N:y}`。
    #[cfg_attr(feature = "serde", serde(rename = "{N:y}"))]
    NBeforeY,
    /// hy `[ç]` の前の撥音。「ヒ」の h も含みます。`{N:hy}`。
    #[cfg_attr(feature = "serde", serde(rename = "{N:hy}"))]
    NBeforeHy,
    /// fy `[ɸʲ]` の前の撥音。`{N:fy}`。
    #[cfg_attr(feature = "serde", serde(rename = "{N:fy}"))]
    NBeforeFy,
    /// v の前の撥音。`{N:v}`。
    #[cfg_attr(feature = "serde", serde(rename = "{N:v}"))]
    NBeforeV,
    /// 促音の前の撥音。促音の後まで調音点の同化を適用しません。`{N:cl}`。
    #[cfg_attr(feature = "serde", serde(rename = "{N:cl}"))]
    NBeforeSokuon,
    /// 撥音の前の撥音。`{N:N}`。
    #[cfg_attr(feature = "serde", serde(rename = "{N:N}"))]
    NBeforeN,
    /// 母音・w・h・f の前で、直前の有声母音を参照できない撥音。`{N:vowel}`。
    ///
    /// 鼻音化母音による広表記を採用する環境ですが、母音の音質は指定しません。
    #[cfg_attr(feature = "serde", serde(rename = "{N:vowel}"))]
    NasalizedMora,
    /// 後続音による分類を行わない撥音。未知音・空白の直前など。`{N}`。
    #[cfg_attr(feature = "serde", serde(rename = "{N}"))]
    MoraicNasal,
    /// 長子音や発話末の声門閉鎖へ変換しない促音。`{Q}`。
    #[cfg_attr(feature = "serde", serde(rename = "{Q}"))]
    Sokuon,
}

impl SpecialPhone {
    /// 定義済みの専用ラベルを列挙順に並べたスライス。
    pub const ALL: &'static [Self] = &[
        Self::NBeforeS,
        Self::NBeforeSh,
        Self::NBeforeY,
        Self::NBeforeHy,
        Self::NBeforeFy,
        Self::NBeforeV,
        Self::NBeforeSokuon,
        Self::NBeforeN,
        Self::NasalizedMora,
        Self::MoraicNasal,
        Self::Sokuon,
    ];

    /// IPA 記号と区別できる、波括弧付きのラベルを返します。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NBeforeS => "{N:s}",
            Self::NBeforeSh => "{N:sh}",
            Self::NBeforeY => "{N:y}",
            Self::NBeforeHy => "{N:hy}",
            Self::NBeforeFy => "{N:fy}",
            Self::NBeforeV => "{N:v}",
            Self::NBeforeSokuon => "{N:cl}",
            Self::NBeforeN => "{N:N}",
            Self::NasalizedMora => "{N:vowel}",
            Self::MoraicNasal => "{N}",
            Self::Sokuon => "{Q}",
        }
    }
}

impl std::fmt::Display for SpecialPhone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// IPA 記号・専用ラベル・未知音のいずれかを表す、変換後の一要素。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum IpaToken {
    /// 一つの IPA phone。
    Phone(IpaPhone),
    /// 音素の種類や後続音による専用ラベル。
    Special(SpecialPhone),
    /// 入力が [`Phoneme::Unk`] だった位置。
    Unknown,
}

impl IpaToken {
    /// IPA 記号、波括弧付きの専用ラベル、または未知音の `{unk}` を返します。
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Phone(phone) => phone.as_str(),
            Self::Special(label) => label.as_str(),
            Self::Unknown => "{unk}",
        }
    }
}

impl std::fmt::Display for IpaToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// IPA phone と分けて保持する韻律境界。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum IpaBoundary {
    /// アクセント句境界。休止ではないため、撥音と促音の後続音素探索は遮らない。
    AccentPhrase,
    /// 通常のポーズ。
    Pause,
    /// 疑問の終結。
    Interrogative,
    /// 感嘆の終結。
    Exclamatory,
}

/// ピッチアクセントと韻律境界を保持した IPA の一要素。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum ProsodicIpa {
    /// IPA token と、token を構成する入力音素・境界の韻律情報。
    Token {
        token: IpaToken,
        prosody: Vec<IpaTokenProsody>,
    },
    /// 音素列を区切る韻律境界。
    Boundary(IpaBoundary),
}

/// 一つの IPA token を構成する入力要素の韻律情報。
///
/// [`IpaTokenProsody::Pitch`] は、token の生成時に消費した各音素の順に並ぶ。
/// 複数音素から一つの phone を作る途中に境界があれば、同じ列の
/// [`IpaTokenProsody::Boundary`] が元の位置を保持する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum IpaTokenProsody {
    /// 変換元の一音素に付いたピッチアクセント。
    Pitch(Option<PitchAccent>),
    /// 複数音素から一つの phone を作る途中にあった韻律境界。
    Boundary(IpaBoundary),
}

/// 単語と IPA token の対応。
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct WordIpaMap {
    /// 単語の表層形。
    pub word: String,
    /// 単語に割り当てられた IPA token。
    pub tokens: Vec<IpaToken>,
    /// MeCab が未知語と判定したかどうか。
    pub is_unknown: bool,
    /// Open JTalk が無視した単語または空白かどうか。
    pub is_ignored: bool,
    /// 正規化後の解析対象文字列における半開区間。
    ///
    /// 詳しい規則は [`crate::WordPhonemeMap::char_span`] と同じ。
    pub char_span: Range<usize>,
}

/// 単語と、ピッチアクセント・韻律境界を保持した IPA の対応。
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct WordIpaProsody {
    /// 単語の表層形。
    pub word: String,
    /// 単語に割り当てられた IPA token と韻律境界。
    pub tokens: Vec<ProsodicIpa>,
    /// MeCab が未知語と判定したかどうか。
    pub is_unknown: bool,
    /// Open JTalk が無視した単語または空白かどうか。
    pub is_ignored: bool,
    /// 正規化後の解析対象文字列における半開区間。
    ///
    /// 詳しい規則は [`crate::WordPhonemeMap::char_span`] と同じ。
    pub char_span: Range<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct IpaSourcePosition {
    word_index: usize,
    item_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TranscribedToken {
    Ipa(IpaToken),
    Boundary(IpaBoundary),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LocatedIpaToken {
    token: TranscribedToken,
    source: Vec<IpaSourcePosition>,
}

#[derive(Debug, Clone, Copy)]
struct LocatedPhoneme {
    phoneme: Phoneme,
    position: IpaSourcePosition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BarrierKind {
    Structural,
    UtteranceFinal,
}

#[derive(Debug, Clone, Copy)]
struct LocatedBarrier {
    kind: BarrierKind,
    boundary: Option<IpaBoundary>,
    position: IpaSourcePosition,
}

#[derive(Debug, Clone, Copy)]
enum LocatedInput {
    Phoneme(LocatedPhoneme),
    Barrier(LocatedBarrier),
}

impl Haqumei {
    /// 日本語テキストを IPA の広い音声表記へ変換し、単語ごとに返します。
    ///
    /// [`WordIpaMap`] は単語の表層形、文字位置、音声表記を持ちます。
    /// 各要素の文字列は [`IpaToken::as_str`] で取得できます。
    /// [`HaqumeiOptions`](crate::HaqumeiOptions) の異音解決オプションは出力に影響しません。
    pub fn g2ipa(&mut self, text: &str) -> Result<Vec<WordIpaMap>, HaqumeiError> {
        let words = self.g2p_mapping_prosody(text)?;
        let tokens = transcribe(&flatten(&words));
        Ok(map_words(&words, tokens))
    }

    /// 入力テキストを単語ごとの IPA へ変換し、ピッチアクセントと韻律境界も返す。
    ///
    /// 複数音素から一つの IPA token を作る場合も、各音素の pitch と途中の境界を
    /// [`IpaTokenProsody`] の列に残す。異音解決オプションは IPA token に影響しない。
    pub fn g2ipa_prosody(&mut self, text: &str) -> Result<Vec<WordIpaProsody>, HaqumeiError> {
        let words = self.g2p_mapping_prosody(text)?;
        let tokens = transcribe(&flatten(&words));
        Ok(map_words_with_prosody(&words, tokens))
    }
}

fn map_words(words: &[WordPhonemeProsody], tokens: Vec<LocatedIpaToken>) -> Vec<WordIpaMap> {
    let mut mapping = words
        .iter()
        .map(|word| WordIpaMap {
            word: word.word.clone(),
            tokens: Vec::new(),
            is_unknown: word.is_unknown,
            is_ignored: word.is_ignored,
            char_span: word.char_span.clone(),
        })
        .collect::<Vec<_>>();

    for located in tokens {
        let TranscribedToken::Ipa(token) = located.token else {
            continue;
        };
        let source = located
            .source
            .last()
            .expect("IPA token の source は空にならない");
        mapping[source.word_index].tokens.push(token);
    }

    mapping
}

fn map_words_with_prosody(
    words: &[WordPhonemeProsody],
    tokens: Vec<LocatedIpaToken>,
) -> Vec<WordIpaProsody> {
    let mut mapping = words
        .iter()
        .map(|word| WordIpaProsody {
            word: word.word.clone(),
            tokens: Vec::new(),
            is_unknown: word.is_unknown,
            is_ignored: word.is_ignored,
            char_span: word.char_span.clone(),
        })
        .collect::<Vec<_>>();

    for located in tokens {
        let source = located
            .source
            .last()
            .expect("IPA token の source は空にならない");
        let token = match located.token {
            TranscribedToken::Ipa(token) => ProsodicIpa::Token {
                token,
                prosody: located
                    .source
                    .iter()
                    .copied()
                    .map(|source| source_prosody(words, source))
                    .collect(),
            },
            TranscribedToken::Boundary(boundary) => ProsodicIpa::Boundary(boundary),
        };
        mapping[source.word_index].tokens.push(token);
    }

    mapping
}

fn source_prosody(words: &[WordPhonemeProsody], source: IpaSourcePosition) -> IpaTokenProsody {
    match words[source.word_index].phonemes[source.item_index] {
        ProsodicPhoneme::Phoneme { pitch, .. } => IpaTokenProsody::Pitch(pitch),
        ProsodicPhoneme::AccentPhraseBoundary => {
            IpaTokenProsody::Boundary(IpaBoundary::AccentPhrase)
        }
        ProsodicPhoneme::Pause => IpaTokenProsody::Boundary(IpaBoundary::Pause),
        ProsodicPhoneme::Interrogative => IpaTokenProsody::Boundary(IpaBoundary::Interrogative),
        ProsodicPhoneme::Exclamatory => IpaTokenProsody::Boundary(IpaBoundary::Exclamatory),
    }
}

fn flatten(words: &[WordPhonemeProsody]) -> Vec<LocatedInput> {
    words
        .iter()
        .enumerate()
        .flat_map(|(word_index, word)| {
            word.phonemes
                .iter()
                .enumerate()
                .map(move |(item_index, item)| {
                    let position = IpaSourcePosition {
                        word_index,
                        item_index,
                    };

                    match item {
                        ProsodicPhoneme::Phoneme {
                            phoneme: Phoneme::Sp,
                            ..
                        } => LocatedInput::Barrier(LocatedBarrier {
                            kind: BarrierKind::Structural,
                            boundary: None,
                            position,
                        }),
                        ProsodicPhoneme::Phoneme {
                            phoneme: Phoneme::Pau,
                            ..
                        } => LocatedInput::Barrier(LocatedBarrier {
                            kind: BarrierKind::UtteranceFinal,
                            boundary: Some(IpaBoundary::Pause),
                            position,
                        }),
                        ProsodicPhoneme::Phoneme { phoneme, .. } => {
                            LocatedInput::Phoneme(LocatedPhoneme {
                                phoneme: *phoneme,
                                position,
                            })
                        }
                        ProsodicPhoneme::AccentPhraseBoundary => {
                            LocatedInput::Barrier(LocatedBarrier {
                                kind: BarrierKind::Structural,
                                boundary: Some(IpaBoundary::AccentPhrase),
                                position,
                            })
                        }
                        ProsodicPhoneme::Pause => LocatedInput::Barrier(LocatedBarrier {
                            kind: BarrierKind::UtteranceFinal,
                            boundary: Some(IpaBoundary::Pause),
                            position,
                        }),
                        ProsodicPhoneme::Interrogative => LocatedInput::Barrier(LocatedBarrier {
                            kind: BarrierKind::UtteranceFinal,
                            boundary: Some(IpaBoundary::Interrogative),
                            position,
                        }),
                        ProsodicPhoneme::Exclamatory => LocatedInput::Barrier(LocatedBarrier {
                            kind: BarrierKind::UtteranceFinal,
                            boundary: Some(IpaBoundary::Exclamatory),
                            position,
                        }),
                    }
                })
        })
        .collect()
}

#[derive(Debug, Clone, Copy)]
enum FollowingContext {
    Phoneme {
        index: usize,
        phoneme: LocatedPhoneme,
    },
    UtteranceFinal,
    Blocked,
}

fn transcribe(input: &[LocatedInput]) -> Vec<LocatedIpaToken> {
    let mut output = Vec::with_capacity(input.len());
    let mut consumed = vec![false; input.len()];

    for (i, item) in input.iter().copied().enumerate() {
        if consumed[i] {
            continue;
        }

        match item {
            LocatedInput::Barrier(barrier) => {
                if let Some(boundary) = barrier.boundary {
                    push_boundary(&mut output, boundary, barrier.position);
                }
            }
            LocatedInput::Phoneme(phoneme) => {
                let current = contextualize_consonant(input, i, canonicalize_allophone(phoneme));

                if current.phoneme.is_sokuon() {
                    let context = following_context(input, i);

                    if let FollowingContext::Phoneme {
                        index,
                        phoneme: next,
                    } = context
                        && let Some(symbol) = long_consonant(next.phoneme)
                    {
                        let source = input[i..=index]
                            .iter()
                            .filter_map(|item| match item {
                                LocatedInput::Phoneme(phoneme) => Some(phoneme.position),
                                LocatedInput::Barrier(barrier) if barrier.boundary.is_some() => {
                                    Some(barrier.position)
                                }
                                LocatedInput::Barrier(_) => None,
                            })
                            .collect();
                        output.push(LocatedIpaToken {
                            token: TranscribedToken::Ipa(IpaToken::Phone(symbol)),
                            source,
                        });
                        consumed[i + 1..=index].fill(true);
                        continue;
                    }

                    output.push(LocatedIpaToken {
                        token: if matches!(context, FollowingContext::UtteranceFinal) {
                            TranscribedToken::Ipa(IpaToken::Phone(IpaPhone::GlottalStop))
                        } else {
                            TranscribedToken::Ipa(IpaToken::Special(SpecialPhone::Sokuon))
                        },
                        source: vec![current.position],
                    });
                    continue;
                }

                if current.phoneme.is_voiced_vowel() {
                    let run_len = same_vowel_run_len(input, i);
                    if run_len == 2 {
                        let LocatedInput::Phoneme(next) = input[i + 1] else {
                            unreachable!("母音列に barrier は含まれない")
                        };
                        output.push(LocatedIpaToken {
                            token: TranscribedToken::Ipa(IpaToken::Phone(long_vowel(
                                current.phoneme,
                            ))),
                            source: vec![current.position, next.position],
                        });
                        consumed[i + 1] = true;
                        continue;
                    }
                    if run_len >= 3 {
                        for item in &input[i..i + run_len] {
                            let LocatedInput::Phoneme(vowel) = *item else {
                                unreachable!("母音列に barrier は含まれない")
                            };
                            push_single_phoneme(vowel, &mut output);
                        }
                        consumed[i + 1..i + run_len].fill(true);
                        continue;
                    }
                }

                if current.phoneme == Phoneme::Nn {
                    transcribe_moraic_nasal(input, i, current, &mut output);
                    continue;
                }

                // 撥音後の z は破擦音で表す。Maekawa (2023, Table 1) の /aNzeN/ も [dz]。
                if current.phoneme == Phoneme::Z
                    && preceding_phoneme(input, i).is_some_and(|p| p.is_moraic_nasal())
                {
                    output.push(LocatedIpaToken {
                        token: TranscribedToken::Ipa(IpaToken::Phone(IpaPhone::Dz)),
                        source: vec![current.position],
                    });
                    continue;
                }

                push_single_phoneme(current, &mut output);
            }
        }
    }

    output
}

fn following_context(input: &[LocatedInput], i: usize) -> FollowingContext {
    for (index, item) in input.iter().copied().enumerate().skip(i + 1) {
        match item {
            LocatedInput::Phoneme(phoneme) => {
                return FollowingContext::Phoneme {
                    index,
                    phoneme: contextualize_consonant(input, index, canonicalize_allophone(phoneme)),
                };
            }
            LocatedInput::Barrier(LocatedBarrier {
                kind: BarrierKind::Structural,
                boundary: Some(IpaBoundary::AccentPhrase),
                ..
            }) => {
                // アクセント句境界は休止を表さない。境界情報を変換結果に残したまま、
                // 撥音と促音の調音位置を決める後続音素の探索は続ける。
            }
            LocatedInput::Barrier(LocatedBarrier {
                kind: BarrierKind::UtteranceFinal,
                ..
            }) => return FollowingContext::UtteranceFinal,
            LocatedInput::Barrier(_) => {
                return context_after_structural_barrier(&input[index + 1..]);
            }
        }
    }

    FollowingContext::UtteranceFinal
}

fn context_after_structural_barrier(input: &[LocatedInput]) -> FollowingContext {
    for item in input {
        match item {
            LocatedInput::Phoneme(_) => return FollowingContext::Blocked,
            LocatedInput::Barrier(LocatedBarrier {
                kind: BarrierKind::UtteranceFinal,
                ..
            }) => return FollowingContext::UtteranceFinal,
            LocatedInput::Barrier(LocatedBarrier {
                kind: BarrierKind::Structural,
                ..
            }) => {}
        }
    }

    FollowingContext::UtteranceFinal
}

fn same_vowel_run_len(input: &[LocatedInput], i: usize) -> usize {
    let Some(LocatedInput::Phoneme(first)) = input.get(i) else {
        return 0;
    };

    input[i..]
        .iter()
        .map_while(|item| match item {
            LocatedInput::Phoneme(phoneme) => Some(phoneme),
            LocatedInput::Barrier(_) => None,
        })
        .take_while(|item| {
            item.phoneme == first.phoneme && item.position.word_index == first.position.word_index
        })
        .count()
}

fn transcribe_moraic_nasal(
    input: &[LocatedInput],
    i: usize,
    current: LocatedPhoneme,
    output: &mut Vec<LocatedIpaToken>,
) {
    let context = following_context(input, i);
    let next = match context {
        FollowingContext::Phoneme { phoneme, .. } => Some(phoneme.phoneme),
        FollowingContext::UtteranceFinal | FollowingContext::Blocked => None,
    };

    let token = match next {
        Some(Phoneme::P | Phoneme::Py | Phoneme::B | Phoneme::By | Phoneme::M | Phoneme::My) => {
            IpaToken::Phone(IpaPhone::M)
        }
        Some(Phoneme::K | Phoneme::Ky | Phoneme::Kw | Phoneme::G | Phoneme::Gy | Phoneme::Gw) => {
            IpaToken::Phone(IpaPhone::Ng)
        }
        Some(
            Phoneme::T
            | Phoneme::Ty
            | Phoneme::Ts
            | Phoneme::D
            | Phoneme::Dy
            | Phoneme::N
            | Phoneme::Ny
            | Phoneme::Z
            | Phoneme::R
            | Phoneme::Ry
            | Phoneme::Ch
            | Phoneme::J,
        ) => IpaToken::Phone(IpaPhone::N),
        Some(next) if uses_nasalized_vowel(next) => {
            let previous = i.checked_sub(1).and_then(|i| match input.get(i) {
                Some(LocatedInput::Phoneme(phoneme)) => Some(*phoneme),
                Some(LocatedInput::Barrier(_)) | None => None,
            });
            if let Some(previous) = previous
                && let Some(symbol) = nasalized_vowel(previous.phoneme)
            {
                output.push(LocatedIpaToken {
                    token: TranscribedToken::Ipa(IpaToken::Phone(symbol)),
                    source: vec![current.position],
                });
                return;
            }
            IpaToken::Special(SpecialPhone::NasalizedMora)
        }
        None if matches!(context, FollowingContext::UtteranceFinal) => {
            IpaToken::Phone(IpaPhone::Nq)
        }
        _ => IpaToken::Special(match next {
            Some(Phoneme::S) => SpecialPhone::NBeforeS,
            Some(Phoneme::Sh) => SpecialPhone::NBeforeSh,
            Some(Phoneme::Y) => SpecialPhone::NBeforeY,
            Some(Phoneme::Hy) => SpecialPhone::NBeforeHy,
            Some(Phoneme::Fy) => SpecialPhone::NBeforeFy,
            Some(Phoneme::V) => SpecialPhone::NBeforeV,
            Some(Phoneme::Cl) => SpecialPhone::NBeforeSokuon,
            Some(Phoneme::Nn) => SpecialPhone::NBeforeN,
            _ => SpecialPhone::MoraicNasal,
        }),
    };

    output.push(LocatedIpaToken {
        token: TranscribedToken::Ipa(token),
        source: vec![current.position],
    });
}

fn canonicalize_allophone(mut phoneme: LocatedPhoneme) -> LocatedPhoneme {
    phoneme.phoneme = match phoneme.phoneme {
        Phoneme::Nn
        | Phoneme::Nm
        | Phoneme::Ng
        | Phoneme::Nd
        | Phoneme::Nq
        | Phoneme::Npl
        | Phoneme::Nr => Phoneme::Nn,
        Phoneme::Cl
        | Phoneme::ClP
        | Phoneme::ClT
        | Phoneme::ClK
        | Phoneme::ClS
        | Phoneme::ClV
        | Phoneme::ClQ => Phoneme::Cl,
        phoneme => phoneme,
    };
    phoneme
}

fn contextualize_consonant(
    input: &[LocatedInput],
    i: usize,
    mut phoneme: LocatedPhoneme,
) -> LocatedPhoneme {
    // 「ヒ」の音素列は h i なので、[ç] への変換には後続母音も使う。
    // 撥音から後続子音を調べるときも、h と ç の環境を区別する。
    if phoneme.phoneme == Phoneme::H
        && matches!(input.get(i + 1), Some(LocatedInput::Phoneme(next))
            if matches!(next.phoneme, Phoneme::I | Phoneme::UnvoicedI))
    {
        phoneme.phoneme = Phoneme::Hy;
    }
    phoneme
}

fn preceding_phoneme(input: &[LocatedInput], i: usize) -> Option<Phoneme> {
    for item in input[..i].iter().rev() {
        match item {
            LocatedInput::Phoneme(phoneme) => return Some(phoneme.phoneme),
            LocatedInput::Barrier(LocatedBarrier {
                kind: BarrierKind::Structural,
                boundary: Some(IpaBoundary::AccentPhrase),
                ..
            }) => {}
            LocatedInput::Barrier(_) => return None,
        }
    }
    None
}

fn uses_nasalized_vowel(next: Phoneme) -> bool {
    // Maekawa (2023, Table 1) では s・j・ç の前に閉鎖と鼻音化母音の両方がある。
    // 撥音を鼻音化母音に変換するのは、後続音が母音・w・h・ɸ の場合に限る。
    next.is_vowel() || matches!(next, Phoneme::W | Phoneme::F | Phoneme::H)
}

fn nasalized_vowel(phoneme: Phoneme) -> Option<IpaPhone> {
    match phoneme {
        Phoneme::A => Some(IpaPhone::NasalizedA),
        Phoneme::I => Some(IpaPhone::NasalizedI),
        Phoneme::U => Some(IpaPhone::NasalizedU),
        Phoneme::E => Some(IpaPhone::NasalizedE),
        Phoneme::O => Some(IpaPhone::NasalizedO),
        _ => None,
    }
}

fn push_single_phoneme(phoneme: LocatedPhoneme, output: &mut Vec<LocatedIpaToken>) {
    let token = match phoneme.phoneme {
        Phoneme::Unk => IpaToken::Unknown,
        // 撥音・促音・境界は transcribe と flatten で処理済み。
        phoneme => IpaToken::Phone(fixed_ipa(phoneme).expect("通常音素には IPA 記号がある")),
    };

    output.push(LocatedIpaToken {
        token: TranscribedToken::Ipa(token),
        source: vec![phoneme.position],
    });
}

fn push_boundary(
    output: &mut Vec<LocatedIpaToken>,
    boundary: IpaBoundary,
    position: IpaSourcePosition,
) {
    if let Some(last) = output.last_mut()
        && let TranscribedToken::Boundary(previous) = &mut last.token
    {
        let same_word = last
            .source
            .last()
            .is_some_and(|source| source.word_index == position.word_index);

        if *previous == IpaBoundary::AccentPhrase && boundary != IpaBoundary::AccentPhrase {
            *previous = boundary;
            last.source.push(position);
            return;
        }

        if same_word {
            if boundary_strength(boundary) >= boundary_strength(*previous) {
                *previous = boundary;
            }
            last.source.push(position);
            return;
        }
    }

    output.push(LocatedIpaToken {
        token: TranscribedToken::Boundary(boundary),
        source: vec![position],
    });
}

fn boundary_strength(boundary: IpaBoundary) -> u8 {
    match boundary {
        IpaBoundary::AccentPhrase => 0,
        IpaBoundary::Pause => 1,
        IpaBoundary::Interrogative | IpaBoundary::Exclamatory => 2,
    }
}

fn long_consonant(phoneme: Phoneme) -> Option<IpaPhone> {
    // 破擦音の長さは閉鎖の長さとして表す。Kawahara (2015, §2.3.2)。
    // z の促音も閉鎖を含む [dːz] を採用する。同書 p.54, 注13。
    let value = match phoneme {
        Phoneme::P => IpaPhone::LongP,
        Phoneme::Py => IpaPhone::LongPy,
        Phoneme::T => IpaPhone::LongT,
        Phoneme::Ty => IpaPhone::LongTy,
        Phoneme::Ts => IpaPhone::LongTs,
        Phoneme::Ch => IpaPhone::LongCh,
        Phoneme::K => IpaPhone::LongK,
        Phoneme::Ky => IpaPhone::LongKy,
        Phoneme::Kw => IpaPhone::LongKw,
        Phoneme::S => IpaPhone::LongS,
        Phoneme::Sh => IpaPhone::LongSh,
        Phoneme::F => IpaPhone::LongF,
        Phoneme::Fy => IpaPhone::LongFy,
        Phoneme::H => IpaPhone::LongH,
        Phoneme::Hy => IpaPhone::LongHy,
        Phoneme::V => IpaPhone::LongV,
        Phoneme::B => IpaPhone::LongB,
        Phoneme::By => IpaPhone::LongBy,
        Phoneme::D => IpaPhone::LongD,
        Phoneme::Dy => IpaPhone::LongDy,
        Phoneme::G => IpaPhone::LongG,
        Phoneme::Gy => IpaPhone::LongGy,
        Phoneme::Gw => IpaPhone::LongGw,
        Phoneme::Z => IpaPhone::LongDz,
        Phoneme::J => IpaPhone::LongJ,
        _ => return None,
    };
    Some(value)
}

fn long_vowel(phoneme: Phoneme) -> IpaPhone {
    match phoneme {
        Phoneme::A => IpaPhone::LongA,
        Phoneme::I => IpaPhone::LongI,
        Phoneme::U => IpaPhone::LongU,
        Phoneme::E => IpaPhone::LongE,
        Phoneme::O => IpaPhone::LongO,
        _ => unreachable!("long_vowel is called only for voiced vowels"),
    }
}

fn fixed_ipa(phoneme: Phoneme) -> Option<IpaPhone> {
    let value = match phoneme {
        Phoneme::UnvoicedA => IpaPhone::UnvoicedA,
        Phoneme::UnvoicedE => IpaPhone::UnvoicedE,
        Phoneme::UnvoicedI => IpaPhone::UnvoicedI,
        Phoneme::UnvoicedO => IpaPhone::UnvoicedO,
        Phoneme::UnvoicedU => IpaPhone::UnvoicedU,
        Phoneme::A => IpaPhone::A,
        Phoneme::B => IpaPhone::B,
        Phoneme::By => IpaPhone::By,
        Phoneme::Ch => IpaPhone::Ch,
        Phoneme::D => IpaPhone::D,
        Phoneme::Dy => IpaPhone::Dy,
        Phoneme::E => IpaPhone::E,
        Phoneme::F => IpaPhone::F,
        Phoneme::Fy => IpaPhone::Fy,
        Phoneme::G => IpaPhone::G,
        Phoneme::Gw => IpaPhone::Gw,
        Phoneme::Gy => IpaPhone::Gy,
        Phoneme::H => IpaPhone::H,
        Phoneme::Hy => IpaPhone::Hy,
        Phoneme::I => IpaPhone::I,
        Phoneme::J => IpaPhone::J,
        Phoneme::K => IpaPhone::K,
        Phoneme::Kw => IpaPhone::Kw,
        Phoneme::Ky => IpaPhone::Ky,
        Phoneme::M => IpaPhone::M,
        Phoneme::My => IpaPhone::My,
        Phoneme::N => IpaPhone::N,
        Phoneme::Ny => IpaPhone::Ny,
        Phoneme::O => IpaPhone::O,
        Phoneme::P => IpaPhone::P,
        Phoneme::Py => IpaPhone::Py,
        Phoneme::R => IpaPhone::R,
        Phoneme::Ry => IpaPhone::Ry,
        Phoneme::S => IpaPhone::S,
        Phoneme::Sh => IpaPhone::Sh,
        Phoneme::T => IpaPhone::T,
        Phoneme::Ts => IpaPhone::Ts,
        Phoneme::Ty => IpaPhone::Ty,
        Phoneme::U => IpaPhone::U,
        Phoneme::V => IpaPhone::V,
        Phoneme::W => IpaPhone::W,
        Phoneme::Y => IpaPhone::Y,
        Phoneme::Z => IpaPhone::Z,
        Phoneme::Nn
        | Phoneme::Nm
        | Phoneme::Ng
        | Phoneme::Nd
        | Phoneme::Nq
        | Phoneme::Npl
        | Phoneme::Nr
        | Phoneme::Cl
        | Phoneme::ClP
        | Phoneme::ClT
        | Phoneme::ClK
        | Phoneme::ClS
        | Phoneme::ClV
        | Phoneme::ClQ
        | Phoneme::Sp
        | Phoneme::Pau
        | Phoneme::Unk => return None,
    };
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn located(phonemes: &[Phoneme]) -> Vec<LocatedInput> {
        phonemes
            .iter()
            .enumerate()
            .map(|(item_index, phoneme)| {
                LocatedInput::Phoneme(LocatedPhoneme {
                    phoneme: *phoneme,
                    position: IpaSourcePosition {
                        word_index: 0,
                        item_index,
                    },
                })
            })
            .collect()
    }

    fn phone(phone: IpaPhone) -> TranscribedToken {
        TranscribedToken::Ipa(IpaToken::Phone(phone))
    }

    fn special(label: SpecialPhone) -> TranscribedToken {
        TranscribedToken::Ipa(IpaToken::Special(label))
    }

    fn boundary(boundary: IpaBoundary) -> TranscribedToken {
        TranscribedToken::Boundary(boundary)
    }

    fn tokens(phonemes: &[Phoneme], ends_utterance: bool) -> Vec<TranscribedToken> {
        let mut input = located(phonemes);
        if !ends_utterance {
            input.push(LocatedInput::Barrier(LocatedBarrier {
                kind: BarrierKind::Structural,
                boundary: None,
                position: IpaSourcePosition {
                    word_index: 0,
                    item_index: phonemes.len(),
                },
            }));
            input.push(LocatedInput::Phoneme(LocatedPhoneme {
                phoneme: Phoneme::Unk,
                position: IpaSourcePosition {
                    word_index: 0,
                    item_index: phonemes.len() + 1,
                },
            }));
        }
        let mut output = transcribe(&input)
            .into_iter()
            .map(|item| item.token)
            .collect::<Vec<_>>();
        if !ends_utterance {
            assert_eq!(output.pop(), Some(TranscribedToken::Ipa(IpaToken::Unknown)));
        }
        output
    }

    #[test]
    fn sokuon_consumes_following_consonant() {
        assert_eq!(
            tokens(&[Phoneme::ClK, Phoneme::K, Phoneme::A], true),
            [phone(IpaPhone::LongK), phone(IpaPhone::A)]
        );
        assert_eq!(
            tokens(&[Phoneme::ClV, Phoneme::B, Phoneme::A], true),
            [phone(IpaPhone::LongB), phone(IpaPhone::A)]
        );
    }

    #[test]
    fn all_sokuon_labels_use_the_actual_following_consonant() {
        let cases = [
            (Phoneme::ClP, Phoneme::Py, IpaPhone::LongPy),
            (Phoneme::ClT, Phoneme::Ts, IpaPhone::LongTs),
            (Phoneme::ClT, Phoneme::Ch, IpaPhone::LongCh),
            (Phoneme::ClK, Phoneme::Kw, IpaPhone::LongKw),
            (Phoneme::ClS, Phoneme::Sh, IpaPhone::LongSh),
            (Phoneme::ClS, Phoneme::Hy, IpaPhone::LongHy),
            (Phoneme::ClV, Phoneme::Gy, IpaPhone::LongGy),
            (Phoneme::ClV, Phoneme::J, IpaPhone::LongJ),
            (Phoneme::ClV, Phoneme::Z, IpaPhone::LongDz),
        ];

        for (sokuon, consonant, expected) in cases {
            assert_eq!(tokens(&[sokuon, consonant], true), [phone(expected)]);
        }
    }

    #[test]
    fn resolved_sokuon_label_does_not_override_the_actual_context() {
        assert_eq!(
            tokens(&[Phoneme::ClP, Phoneme::K], true),
            [phone(IpaPhone::LongK)]
        );
    }

    #[test]
    fn all_sokuon_labels_need_an_utterance_boundary_for_a_glottal_stop() {
        assert_eq!(
            tokens(&[Phoneme::ClQ], false),
            [special(SpecialPhone::Sokuon)]
        );
        assert_eq!(tokens(&[Phoneme::Cl], true), [phone(IpaPhone::GlottalStop)]);
        assert_eq!(
            tokens(&[Phoneme::Cl], false),
            [special(SpecialPhone::Sokuon)]
        );
    }

    #[test]
    fn resolved_n_label_does_not_override_the_actual_context() {
        assert_eq!(
            tokens(&[Phoneme::Npl, Phoneme::J], true),
            [phone(IpaPhone::N), phone(IpaPhone::J)]
        );
        assert_eq!(
            tokens(&[Phoneme::Nr, Phoneme::R], true),
            [phone(IpaPhone::N), phone(IpaPhone::R)]
        );
    }

    #[test]
    fn moraic_nasal_uses_both_sides() {
        assert_eq!(
            tokens(&[Phoneme::I, Phoneme::Nn, Phoneme::A], true),
            [
                phone(IpaPhone::I),
                phone(IpaPhone::NasalizedI),
                phone(IpaPhone::A)
            ]
        );
        assert_eq!(
            tokens(&[Phoneme::Nn, Phoneme::P], true),
            [phone(IpaPhone::M), phone(IpaPhone::P)]
        );
        assert_eq!(tokens(&[Phoneme::Nn], true), [phone(IpaPhone::Nq)]);
        assert_eq!(
            tokens(&[Phoneme::Nn], false),
            [special(SpecialPhone::MoraicNasal)]
        );
    }

    #[test]
    fn variable_nasals_are_classified_by_following_consonant() {
        for (next, expected) in [
            (Phoneme::S, SpecialPhone::NBeforeS),
            (Phoneme::Sh, SpecialPhone::NBeforeSh),
            (Phoneme::Y, SpecialPhone::NBeforeY),
            (Phoneme::Hy, SpecialPhone::NBeforeHy),
            (Phoneme::Fy, SpecialPhone::NBeforeFy),
            (Phoneme::V, SpecialPhone::NBeforeV),
        ] {
            let result = tokens(&[Phoneme::A, Phoneme::Nn, next, Phoneme::A], true);
            assert_eq!(result[1], special(expected), "{next}");
        }
        assert_eq!(
            tokens(&[Phoneme::A, Phoneme::Nn, Phoneme::H, Phoneme::I], true),
            [
                phone(IpaPhone::A),
                special(SpecialPhone::NBeforeHy),
                phone(IpaPhone::Hy),
                phone(IpaPhone::I)
            ]
        );
        assert_eq!(
            tokens(&[Phoneme::Cl, Phoneme::H, Phoneme::UnvoicedI], true),
            [phone(IpaPhone::LongHy), phone(IpaPhone::UnvoicedI)]
        );
    }

    #[test]
    fn special_labels_preserve_mora_types_without_absorbing_neighbors() {
        assert_eq!(
            tokens(&[Phoneme::Nn, Phoneme::ClP, Phoneme::K], true),
            [special(SpecialPhone::NBeforeSokuon), phone(IpaPhone::LongK)]
        );
        assert_eq!(
            tokens(&[Phoneme::Nn, Phoneme::Nm, Phoneme::P], true),
            [
                special(SpecialPhone::NBeforeN),
                phone(IpaPhone::M),
                phone(IpaPhone::P)
            ]
        );
        assert_eq!(
            tokens(&[Phoneme::Nn, Phoneme::A], true),
            [special(SpecialPhone::NasalizedMora), phone(IpaPhone::A)]
        );
        assert_eq!(
            tokens(&[Phoneme::UnvoicedI, Phoneme::Nn, Phoneme::A], true),
            [
                phone(IpaPhone::UnvoicedI),
                special(SpecialPhone::NasalizedMora),
                phone(IpaPhone::A)
            ]
        );
        assert_eq!(
            tokens(&[Phoneme::Nn, Phoneme::Unk], true),
            [
                special(SpecialPhone::MoraicNasal),
                TranscribedToken::Ipa(IpaToken::Unknown)
            ]
        );
        assert_eq!(
            tokens(&[Phoneme::Cl, Phoneme::R, Phoneme::A], true),
            [
                special(SpecialPhone::Sokuon),
                phone(IpaPhone::R),
                phone(IpaPhone::A)
            ]
        );
    }

    #[test]
    fn special_nasal_context_crosses_accent_phrase_but_not_spaces() {
        let mut input = located(&[Phoneme::Nn, Phoneme::S]);
        let LocatedInput::Phoneme(first) = input[0] else {
            unreachable!()
        };
        input.insert(
            1,
            LocatedInput::Barrier(LocatedBarrier {
                kind: BarrierKind::Structural,
                boundary: Some(IpaBoundary::AccentPhrase),
                position: first.position,
            }),
        );
        let result = transcribe(&input);
        assert_eq!(result[0].token, special(SpecialPhone::NBeforeS));
        assert_eq!(result[0].source, vec![first.position]);
        assert_eq!(result[1].token, boundary(IpaBoundary::AccentPhrase));
        assert_eq!(result[2].token, phone(IpaPhone::S));

        let LocatedInput::Barrier(barrier) = &mut input[1] else {
            unreachable!()
        };
        barrier.boundary = None;
        let result = transcribe(&input);
        assert_eq!(result[0].token, special(SpecialPhone::MoraicNasal));
        assert_eq!(result[1].token, phone(IpaPhone::S));
    }

    #[test]
    fn special_labels_do_not_collide_with_ipa_or_unknown_input() {
        let mut labels = std::collections::HashSet::new();
        for label in SpecialPhone::ALL {
            let symbol = label.as_str();
            assert!(symbol.starts_with('{') && symbol.ends_with('}'));
            assert!(labels.insert(symbol), "{symbol}");
            assert!(!IpaPhone::SYMBOLS.contains(&symbol));
            assert!(symbol.parse::<IpaPhone>().is_err());
            assert_ne!(symbol, IpaToken::Unknown.as_str());
        }
    }

    #[test]
    fn accent_phrase_boundary_is_transparent_to_allophone_context() {
        let position = |item_index| IpaSourcePosition {
            word_index: 0,
            item_index,
        };
        let accent_phrase = LocatedInput::Barrier(LocatedBarrier {
            kind: BarrierKind::Structural,
            boundary: Some(IpaBoundary::AccentPhrase),
            position: position(1),
        });

        let n_input = [
            LocatedInput::Phoneme(LocatedPhoneme {
                phoneme: Phoneme::Nn,
                position: position(0),
            }),
            accent_phrase,
            LocatedInput::Phoneme(LocatedPhoneme {
                phoneme: Phoneme::R,
                position: position(2),
            }),
        ];
        assert_eq!(
            transcribe(&n_input)
                .into_iter()
                .map(|item| item.token)
                .collect::<Vec<_>>(),
            [
                phone(IpaPhone::N),
                boundary(IpaBoundary::AccentPhrase),
                phone(IpaPhone::R),
            ]
        );

        let cl_input = [
            LocatedInput::Phoneme(LocatedPhoneme {
                phoneme: Phoneme::Cl,
                position: position(0),
            }),
            accent_phrase,
            LocatedInput::Phoneme(LocatedPhoneme {
                phoneme: Phoneme::K,
                position: position(2),
            }),
        ];
        assert_eq!(
            transcribe(&cl_input)
                .iter()
                .map(|item| item.token.clone())
                .collect::<Vec<_>>(),
            [phone(IpaPhone::LongK)]
        );
        assert_eq!(transcribe(&cl_input)[0].source.len(), 3);

        let vowel_input = [
            LocatedInput::Phoneme(LocatedPhoneme {
                phoneme: Phoneme::O,
                position: position(0),
            }),
            accent_phrase,
            LocatedInput::Phoneme(LocatedPhoneme {
                phoneme: Phoneme::O,
                position: position(2),
            }),
        ];
        assert_eq!(
            transcribe(&vowel_input)
                .into_iter()
                .map(|item| item.token)
                .collect::<Vec<_>>(),
            [
                phone(IpaPhone::O),
                boundary(IpaBoundary::AccentPhrase),
                phone(IpaPhone::O),
            ]
        );
    }

    #[test]
    fn long_vowels_do_not_collapse_longer_runs() {
        assert_eq!(
            tokens(&[Phoneme::O, Phoneme::O], true),
            [phone(IpaPhone::LongO)]
        );
        assert_eq!(
            tokens(&[Phoneme::O, Phoneme::O, Phoneme::O], true),
            [phone(IpaPhone::O), phone(IpaPhone::O), phone(IpaPhone::O)]
        );
    }

    #[test]
    fn long_vowels_do_not_cross_word_boundaries() {
        let input = [
            LocatedInput::Phoneme(LocatedPhoneme {
                phoneme: Phoneme::O,
                position: IpaSourcePosition {
                    word_index: 0,
                    item_index: 0,
                },
            }),
            LocatedInput::Phoneme(LocatedPhoneme {
                phoneme: Phoneme::O,
                position: IpaSourcePosition {
                    word_index: 1,
                    item_index: 0,
                },
            }),
        ];

        assert_eq!(
            transcribe(&input)
                .into_iter()
                .map(|item| item.token)
                .collect::<Vec<_>>(),
            [phone(IpaPhone::O), phone(IpaPhone::O)]
        );
    }

    #[test]
    fn stronger_boundary_replaces_redundant_boundary() {
        let mut output = Vec::new();
        let position = |item_index| IpaSourcePosition {
            word_index: 0,
            item_index,
        };

        push_boundary(&mut output, IpaBoundary::AccentPhrase, position(0));
        push_boundary(&mut output, IpaBoundary::Pause, position(1));
        push_boundary(&mut output, IpaBoundary::Interrogative, position(2));

        assert_eq!(output.len(), 1);
        assert_eq!(
            output[0].token,
            TranscribedToken::Boundary(IpaBoundary::Interrogative)
        );
        assert_eq!(output[0].source.len(), 3);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_keeps_special_labels_distinct_from_ipa() {
        for label in SpecialPhone::ALL {
            let token = IpaToken::Special(*label);
            let json = serde_json::to_value(token).unwrap();
            assert_eq!(json, serde_json::json!({ "Special": label.as_str() }));
            assert_eq!(serde_json::from_value::<IpaToken>(json).unwrap(), token);
        }
        assert!(serde_json::from_value::<IpaToken>(serde_json::json!({"Phone": "{N:s}"})).is_err());
        assert!(serde_json::from_value::<IpaToken>(serde_json::json!({"Special": "n"})).is_err());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_keeps_typed_ipa_phone() {
        let value = IpaToken::Phone(IpaPhone::NasalizedU);

        let json = serde_json::to_string(&value).unwrap();
        let restored: IpaToken = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, value);

        let restored: IpaPhone = serde_json::from_value(serde_json::json!("ɯ̃")).unwrap();
        assert_eq!(restored, IpaPhone::NasalizedU);
    }
}
