use haqumei_macros::phonemes;

phonemes! {
    UnvoicedA = "A",
    UnvoicedE = "E",
    UnvoicedI = "I",
    UnvoicedO = "O",
    UnvoicedU = "U",
    Nn        = "N", // Moraic nasal (ん)
    A         = "a",
    B         = "b",
    By        = "by",
    Ch        = "ch",
    Cl        = "cl",
    D         = "d",
    Dy        = "dy",
    E         = "e",
    F         = "f",
    Fy        = "fy",
    G         = "g",
    Gw        = "gw",
    Gy        = "gy",
    H         = "h",
    Hy        = "hy",
    I         = "i",
    J         = "j",
    K         = "k",
    Kw        = "kw",
    Ky        = "ky",
    M         = "m",
    My        = "my",
    N         = "n",  // Standard nasal consonant
    Ny        = "ny",
    O         = "o",
    P         = "p",
    Py        = "py",
    R         = "r",
    Ry        = "ry",
    S         = "s",
    Sh        = "sh",
    T         = "t",
    Ts        = "ts",
    Ty        = "ty",
    U         = "u",
    V         = "v",
    W         = "w",
    Y         = "y",
    Z         = "z",
    Sp        = "sp",
    Pau       = "pau",
    Unk       = "unk",
}

impl Phoneme {
    /// 無声音 (無声化母音、および無声子音) であるか判定します
    pub const fn is_unvoiced(&self) -> bool {
        self.is_unvoiced_vowel() || self.is_unvoiced_consonant()
    }

    /// 有声音 (有声母音、および有声子音、撥音) であるか判定します
    ///
    /// なお、ポーズや不明な音は含みません。
    pub const fn is_voiced(&self) -> bool {
        self.is_voiced_vowel() || self.is_voiced_consonant() || matches!(self, Self::Nn)
    }

    /// 母音 (有声・無声両方) であるか判定します
    pub const fn is_vowel(&self) -> bool {
        self.is_voiced_vowel() || self.is_unvoiced_vowel()
    }

    /// 有声母音であるか判定します
    pub const fn is_voiced_vowel(&self) -> bool {
        matches!(self, Self::A | Self::E | Self::I | Self::O | Self::U)
    }

    /// 無声化母音であるか判定します
    pub const fn is_unvoiced_vowel(&self) -> bool {
        matches!(
            self,
            Self::UnvoicedA | Self::UnvoicedE | Self::UnvoicedI | Self::UnvoicedO | Self::UnvoicedU
        )
    }

    /// 子音 (有声・無声両方) であるか判定します
    pub const fn is_consonant(&self) -> bool {
        self.is_unvoiced_consonant() || self.is_voiced_consonant()
    }

    /// 無声子音であるか判定します (促音 cl を含みません)
    pub const fn is_unvoiced_consonant(&self) -> bool {
        matches!(
            self,
            Self::K
                | Self::Ky
                | Self::S
                | Self::Sh
                | Self::T
                | Self::Ts
                | Self::Ty
                | Self::Ch
                | Self::P
                | Self::Py
                | Self::F
                | Self::Fy
                | Self::H
                | Self::Hy
                | Self::Kw
        )
    }

    /// 有声子音であるか判定します (撥音 Nn は含みません)
    pub const fn is_voiced_consonant(&self) -> bool {
        matches!(
            self,
            Self::G
                | Self::Gy
                | Self::Gw
                | Self::Z
                | Self::J
                | Self::D
                | Self::Dy
                | Self::B
                | Self::By
                | Self::M
                | Self::My
                | Self::N
                | Self::Ny
                | Self::R
                | Self::Ry
                | Self::W
                | Self::Y
                | Self::V
        )
    }

    /// 閉鎖区間 (促音、ポーズ、スペース) であるかを判定します
    pub const fn is_silent(&self) -> bool {
        matches!(self, Self::Cl | Self::Pau | Self::Sp)
    }

    /// 無音 (ポーズ、スペース) であるか判定します
    pub const fn is_rest(&self) -> bool {
        matches!(self, Self::Sp | Self::Pau)
    }

    /// 特殊記号 (ポーズ、不明な音) であるか判定します
    pub const fn is_special(&self) -> bool {
        self.is_rest() || matches!(self, Self::Unk)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phoneme_exhaustiveness() {
        let all_phonemes = Phoneme::ALL;

        for p in all_phonemes {
            // Sp, Pau を除くすべての音素は「有声音」「無声音」「閉鎖区間」「無音・特殊記号」のいずれか1つに必ず属するべき
            let is_voiced = p.is_voiced();
            let is_unvoiced = p.is_unvoiced();
            let is_silent = p.is_silent();
            let is_special = p.is_special();

            let true_count = [is_voiced, is_unvoiced, is_silent, is_special]
                .iter()
                .filter(|&&x| x)
                .count();

            if !matches!(p, Phoneme::Sp | Phoneme::Pau) {
                assert_eq!(
                    true_count, 1,
                    "{:?} の分類が正しくありません (voiced: {}, unvoiced: {}, silent: {}, special: {})",
                    p, is_voiced, is_unvoiced, is_silent, is_special
                );
            } else {
                assert_eq!(
                    true_count, 2,
                    "{:?} の分類が正しくありません (voiced: {}, unvoiced: {}, silent: {}, special: {})",
                    p, is_voiced, is_unvoiced, is_silent, is_special
                );
            }
        }
    }
}
