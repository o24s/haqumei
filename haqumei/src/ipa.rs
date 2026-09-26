//! IPA の広い音声表記への変換。
//!
//! 促音と後続子音のように、複数の音素から一つの phone が作られることがある。
//! そのため、変換は [`Phoneme`] ごとの置換ではなく、
//! [`Haqumei::g2p_mapping_prosody`] が返した列全体に対して行う。
//!
//! 出力は規則で決められる範囲の broad phonetic transcription であり、
//! 音声を観測した narrow transcription ではない。とくに発話末撥音の `[ɴ]` と
//! 日本語 `/w/` の `[β̞]` は、連続変異を一つの記号へ畳んだ広表記上の約束である。
//! 既存の異音解決オプションは音響モデルへ渡す音素ラベルを選ぶための設定なので、
//! IPA の判断には使わない。撥音と促音は未解決の形へ戻し、本モジュールの文脈規則で
//! 一意に変換する。
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
    LongCh = "tɕː",
    D = "d",
    Dy = "dʲ",
    LongD = "dː",
    LongDy = "dʲː",
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
    LongJ = "dʑː",
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
    LongTs = "tsː",
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
    LongZ = "zː",
    GlottalStop = "ʔ",
}

/// IPA 変換後の一要素。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum IpaToken {
    /// 一つの IPA phone。
    Phone(IpaPhone),
    /// 入力が [`Phoneme::Unk`] だった位置。
    Unknown,
    /// 文脈が足りず、IPA phone を一つに決められなかった音素。
    Unresolved(Phoneme),
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
    /// 入力テキストを単語ごとの IPA の広い音声表記へ変換する。
    ///
    /// 返り値は phone、未知音、未解決音素を区別する。促音と後続子音は一つの
    /// 長子音へまとめられるため、単純な `Phoneme` ごとの置換ではない。
    /// [`HaqumeiOptions`](crate::HaqumeiOptions) の異音解決オプションには影響されない。
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
                let current = canonicalize_allophone(phoneme);

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
                            TranscribedToken::Ipa(IpaToken::Unresolved(current.phoneme))
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
                    transcribe_unresolved_n(input, i, current, &mut output);
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
                    phoneme: canonicalize_allophone(phoneme),
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

fn transcribe_unresolved_n(
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
            IpaToken::Unresolved(Phoneme::Nn)
        }
        None if matches!(context, FollowingContext::UtteranceFinal) => {
            IpaToken::Phone(IpaPhone::Nq)
        }
        _ => IpaToken::Unresolved(Phoneme::Nn),
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

fn uses_nasalized_vowel(next: Phoneme) -> bool {
    next.is_vowel()
        || matches!(
            next,
            Phoneme::Y
                | Phoneme::W
                | Phoneme::S
                | Phoneme::Sh
                | Phoneme::F
                | Phoneme::Fy
                | Phoneme::H
                | Phoneme::Hy
                | Phoneme::V
        )
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
        Phoneme::Sp | Phoneme::Pau => IpaToken::Unresolved(phoneme.phoneme),
        phoneme => match fixed_ipa(phoneme) {
            Some(symbol) => IpaToken::Phone(symbol),
            None => IpaToken::Unresolved(phoneme),
        },
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
        Phoneme::Z => IpaPhone::LongZ,
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

    fn unresolved(phoneme: Phoneme) -> TranscribedToken {
        TranscribedToken::Ipa(IpaToken::Unresolved(phoneme))
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
        assert_eq!(tokens(&[Phoneme::ClQ], false), [unresolved(Phoneme::Cl)]);
        assert_eq!(tokens(&[Phoneme::Cl], true), [phone(IpaPhone::GlottalStop)]);
        assert_eq!(tokens(&[Phoneme::Cl], false), [unresolved(Phoneme::Cl)]);
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
    fn unresolved_n_uses_both_sides() {
        assert_eq!(
            tokens(&[Phoneme::I, Phoneme::Nn, Phoneme::S], true),
            [
                phone(IpaPhone::I),
                phone(IpaPhone::NasalizedI),
                phone(IpaPhone::S)
            ]
        );
        assert_eq!(
            tokens(&[Phoneme::Nn, Phoneme::P], true),
            [phone(IpaPhone::M), phone(IpaPhone::P)]
        );
        assert_eq!(tokens(&[Phoneme::Nn], true), [phone(IpaPhone::Nq)]);
        assert_eq!(tokens(&[Phoneme::Nn], false), [unresolved(Phoneme::Nn)]);
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
    fn serde_keeps_typed_ipa_phone() {
        let value = IpaToken::Phone(IpaPhone::NasalizedU);

        let json = serde_json::to_string(&value).unwrap();
        let restored: IpaToken = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, value);

        let restored: IpaPhone = serde_json::from_value(serde_json::json!("ɯ̃")).unwrap();
        assert_eq!(restored, IpaPhone::NasalizedU);
    }
}
