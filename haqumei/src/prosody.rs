#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{Phoneme, word_phoneme::WordPhonemeProsody};

/// 音素ごとのピッチアクセント (高低) を表す enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PitchAccent {
    Low,
    High,
}

/// プロソディ情報つきの音素を表す enum
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ProsodicPhoneme {
    Phoneme {
        phoneme: Phoneme,
        pitch: Option<PitchAccent>,
    },
    /// アクセント句境界 (`#`)
    AccentPhraseBoundary,
    /// 通常のポーズ・読点 (`_`)
    Pause,
    /// 疑問の終結・ポーズ (`?`)
    Interrogative,
    /// 感嘆の終結・ポーズ (`!`)
    Exclamatory,
}

impl ProsodicPhoneme {
    pub(crate) fn pau() -> Self {
        Self::Phoneme {
            phoneme: Phoneme::Pau,
            pitch: None,
        }
    }

    pub(crate) fn unk() -> Self {
        Self::Phoneme {
            phoneme: Phoneme::Unk,
            pitch: None,
        }
    }

    pub(crate) fn sp() -> Self {
        Self::Phoneme {
            phoneme: Phoneme::Sp,
            pitch: None,
        }
    }
}

/// `g2p_prosody_with_options` のような関数で出力形式を指定する enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ProsodyFormat {
    /// tdmelodic 風 (`a [ o ] i #`) の記法
    #[default]
    Default,
    /// `L_a`, `H_o` のようなプレフィックス表現
    Prefix,
    /// `a:0`, `o:1` のような数値サフィックス表現
    Numeric,
}

impl WordPhonemeProsody {
    /// 単語のプロソディ情報を、指定されたフォーマットで文字列配列に変換する。
    ///
    /// # Arguments
    /// - `format` - 出力するプロソディ表現のフォーマット
    /// - `prev_pitch` - 直前の音素のピッチ状態。`Default` における `[` `]` 挿入の判定に使用する。
    ///   イテレーション間で状態を保持するため可変参照を渡す。
    pub fn to_formatted_strings(
        &self,
        format: ProsodyFormat,
        prev_pitch: &mut Option<PitchAccent>,
    ) -> Vec<String> {
        let mut result = Vec::new();

        if self.is_unknown {
            result.push("{".to_string());
        }

        for p in &self.phonemes {
            match p {
                ProsodicPhoneme::Phoneme { phoneme, pitch } => {
                    if format == ProsodyFormat::Default
                        && let Some(curr) = pitch
                    {
                        match (*prev_pitch, *curr) {
                            (Some(PitchAccent::Low), PitchAccent::High) => {
                                result.push("[".to_string());
                            }
                            (Some(PitchAccent::High), PitchAccent::Low) => {
                                result.push("]".to_string());
                            }
                            _ => {}
                        }
                        *prev_pitch = Some(*curr);
                    }

                    let phoneme_str = phoneme.as_str();
                    match format {
                        ProsodyFormat::Prefix => {
                            let prefix = match pitch {
                                Some(PitchAccent::High) => "H_",
                                Some(PitchAccent::Low) => "L_",
                                None => "",
                            };
                            result.push(format!("{}{}", prefix, phoneme_str));
                        }
                        ProsodyFormat::Numeric => {
                            let suffix = match pitch {
                                Some(PitchAccent::High) => ":1",
                                Some(PitchAccent::Low) => ":0",
                                None => "",
                            };
                            result.push(format!("{}{}", phoneme_str, suffix));
                        }
                        ProsodyFormat::Default => {
                            result.push(phoneme_str.to_string());
                        }
                    }
                }
                ProsodicPhoneme::AccentPhraseBoundary => {
                    result.push("#".to_string());
                    // アクセント句を跨いだらピッチの追跡をリセット
                    if format == ProsodyFormat::Default {
                        *prev_pitch = None;
                    }
                }
                ProsodicPhoneme::Pause => {
                    result.push("_".to_string());
                    if format == ProsodyFormat::Default {
                        *prev_pitch = None;
                    }
                }
                ProsodicPhoneme::Interrogative => {
                    result.push("?".to_string());
                    if format == ProsodyFormat::Default {
                        *prev_pitch = None;
                    }
                }
                ProsodicPhoneme::Exclamatory => {
                    result.push("!".to_string());
                    if format == ProsodyFormat::Default {
                        *prev_pitch = None;
                    }
                }
            }
        }

        if self.is_unknown {
            result.push("}".to_string());
        }

        result
    }
}
