#[inline(always)]
pub const fn is_vowel_fullwidth(c: char) -> bool {
    matches!(
        c,
        'Ａ' | 'Ｅ' | 'Ｉ' | 'Ｏ' | 'Ｕ' | 'Ｙ' | 'ａ' | 'ｅ' | 'ｉ' | 'ｏ' | 'ｕ' | 'ｙ'
    )
}

#[inline(always)]
pub const fn is_aeiou_fullwidth(c: char) -> bool {
    matches!(
        c,
        'Ａ' | 'Ｅ' | 'Ｉ' | 'Ｏ' | 'Ｕ' | 'ａ' | 'ｅ' | 'ｉ' | 'ｏ' | 'ｕ'
    )
}

#[inline(always)]
pub const fn is_consonant_fullwidth(c: char) -> bool {
    matches!(c, 'Ａ'..='Ｚ' | 'ａ'..='ｚ') && !is_vowel_fullwidth(c)
}

/// 持続可能そうな子音 (摩擦音・鼻音・流音など、母音なしで引き伸ばして発音できる子音)
    /// n は外した方がメリットが大きそうなので、加えていない
    #[rustfmt::skip]
    #[inline(always)]
    pub const fn is_continuant_fullwidth(c: char) -> bool {
        matches!(
            c,
            'Ｓ' | 'Ｚ' | 'Ｆ' | 'Ｖ' | 'Ｈ' | 'Ｍ' | 'Ｌ' | 'Ｒ' | 'Ｗ'
            | 'ｓ' | 'ｚ' | 'ｆ' | 'ｖ' | 'ｈ' | 'ｍ' | 'ｌ' | 'ｒ' | 'ｗ'
        )
    }

/// 2文字の母音が許容される二重母音かどうか
    /// 入力は全角小文字である必要があります
    #[rustfmt::skip]
    #[inline(always)]
    fn is_allowed_nucleus2(a: char, b: char) -> bool {
        matches!(
            (a, b),
            ('ａ', 'ｉ') | ('ａ', 'ｙ') | ('ａ', 'ｕ') | ('ａ', 'ｗ') | ('ｅ', 'ａ') |
            ('ｅ', 'ｅ') | ('ｅ', 'ｉ') | ('ｅ', 'ｙ') | ('ｅ', 'ｕ') | ('ｉ', 'ｅ') |
            ('ｏ', 'ａ') | ('ｏ', 'ｉ') | ('ｏ', 'ｏ') | ('ｏ', 'ｕ') | ('ｏ', 'ｗ') |
            ('ｏ', 'ｙ') | ('ｕ', 'ｅ') | ('ｕ', 'ｙ')
        )
    }

/// 2文字の子音が英語のオンセットとして許容されるかどうか
    /// 入力は全角小文字である必要があります
    #[rustfmt::skip]
    #[inline(always)]
    fn is_allowed_onset2(a: char, b: char) -> bool {
        matches!(
            (a, b),
            ('ｂ', 'ｌ') | ('ｂ', 'ｒ') | ('ｃ', 'ｌ') | ('ｃ', 'ｒ') | ('ｄ', 'ｒ') |
            ('ｆ', 'ｌ') | ('ｆ', 'ｒ') | ('ｇ', 'ｌ') | ('ｇ', 'ｒ') | ('ｐ', 'ｌ') |
            ('ｐ', 'ｒ') | ('ｓ', 'ｃ') | ('ｓ', 'ｋ') | ('ｓ', 'ｌ') | ('ｓ', 'ｍ') |
            ('ｓ', 'ｎ') | ('ｓ', 'ｐ') | ('ｓ', 'ｔ') | ('ｓ', 'ｗ') | ('ｔ', 'ｒ') |
            ('ｔ', 'ｗ') | ('ｓ', 'ｈ') | ('ｃ', 'ｈ') | ('ｔ', 'ｈ') | ('ｗ', 'ｈ') |
            ('ｐ', 'ｈ') | ('ｗ', 'ｒ') | ('ｋ', 'ｎ') | ('ｑ', 'ｕ')
        )
    }

/// 2文字の子音が英語の Rime として許容されるかどうか
    /// 入力は全角小文字である必要があります
    #[rustfmt::skip]
    #[inline(always)]
    fn is_allowed_rime2(a: char, b: char) -> bool {
        matches!(
            (a, b),
            ('ｃ', 'ｋ') | ('ｃ', 'ｔ') | ('ｆ', 'ｔ') | ('ｌ', 'ｄ') | ('ｌ', 'ｆ') |
            ('ｌ', 'ｋ') | ('ｌ', 'ｍ') | ('ｌ', 'ｎ') | ('ｌ', 'ｐ') | ('ｌ', 'ｔ') |
            ('ｍ', 'ｐ') | ('ｎ', 'ｄ') | ('ｎ', 'ｇ') | ('ｎ', 'ｋ') | ('ｎ', 'ｔ') |
            ('ｐ', 'ｔ') | ('ｒ', 'ｂ') | ('ｒ', 'ｄ') | ('ｒ', 'ｆ') | ('ｒ', 'ｇ') |
            ('ｒ', 'ｋ') | ('ｒ', 'ｌ') | ('ｒ', 'ｍ') | ('ｒ', 'ｎ') | ('ｒ', 'ｐ') |
            ('ｒ', 'ｔ') | ('ｓ', 'ｋ') | ('ｗ', 'ｌ')
        )
    }

/// 全角英字の3文字が英語の一音節として一息で発音可能な
    /// 構成かどうかを判定する。
    #[rustfmt::skip]
    #[inline(always)]
    pub fn is_pronounceable(chars: [char; 3]) -> bool {
        let c0 = fullwidth_lower(chars[0]);
        let c1 = fullwidth_lower(chars[1]);
        let c2 = fullwidth_lower(chars[2]);

        let is_v0 = is_aeiou_fullwidth(c0);
        let is_v1 = is_vowel_fullwidth(c1);
        let is_v2 = is_vowel_fullwidth(c2);

        match (is_v0, is_v1, is_v2) {
            (false, false, false) => false, // ccc
            (true, false, false) => {
                (
                    c1 == c2 &&
                        // V_0 c_1 c_1 となったとき、 c_1 にくると嬉しそうな子音たち
                        matches!(c1,
                            'ｂ' | 'ｄ' | 'ｆ' | 'ｇ' | 'ｈ' | 'ｌ' | 'ｍ' | 'ｎ' | 'ｐ' | 'ｒ' | 'ｓ'
                       )
                )   || is_allowed_rime2(c1, c2) // Vcc
            }
            (false, true, false) => true,   // cVc: 常に許容する (e.g., cat)
            (false, false, true) => is_allowed_onset2(c0, c1), // ccV
            (true, true, false) => is_allowed_nucleus2(c0, c1), // VVc
            (false, true, true) => is_allowed_nucleus2(c1, c2), // cVV
            (true, false, true) => true,    // VcV (e.g., use, are, ate, ice, one)
            (true, true, true) => false,    // VVV: 処理しないものとしておく
        }
    }

/// 全角アルファベットの大文字を全角小文字に変換する
#[inline(always)]
const fn fullwidth_lower(c: char) -> char {
    match c {
        'Ａ'..='Ｚ' => char::from_u32(c as u32 + 0x20).unwrap(),
        _ => c,
    }
}

#[inline(always)]
pub fn fullwidth_to_halfwidth(chars: Vec<char>) -> String {
    chars
        .into_iter()
        .map(|c| match c {
            '\u{FF21}'..='\u{FF3A}' | '\u{FF41}'..='\u{FF5A}' => {
                char::from_u32(c as u32 - 0xFEE0).unwrap()
            }
            _ => c,
        })
        .collect()
}

pub fn to_halfwidth_lower_string(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'Ａ'..='Ｚ' => char::from_u32(c as u32 - 0xFEE0 + 0x20).unwrap(),
            'ａ'..='ｚ' => char::from_u32(c as u32 - 0xFEE0).unwrap(),
            'A'..='Z' => c.to_ascii_lowercase(),
            '０'..='９' => char::from_u32(c as u32 - 0xFEE0).unwrap(),
            _ => c,
        })
        .collect()
}
