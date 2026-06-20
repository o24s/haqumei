#[cfg(test)]
mod tests;
mod utils;

use std::borrow::Cow;
use std::ops::Range;

use haqumei_kanalizer::{ConvertOptions, MaxLength};
use unicode_normalization::{IsNormalized, UnicodeNormalization as _, is_nfc_quick, is_nfkc_quick};
use vibrato_rkyv::tokenizer::worker::Worker;

use crate::{
    Haqumei, IuPronunciation, KANALIZER, KANALIZER_CACHE, NjdFeature, OpenJTalk,
    UnicodeNormalization, VIBRATO_CACHE,
    data::MULTI_READ_KANJI_LIST,
    errors::HaqumeiError,
    features::UnidicFeature,
    utils::{
        count_mora, is_kanji, is_kanji_feature, is_single_kanji_feature, is_small_kana,
        split_kana_mora,
    },
};
use utils::{TO_DAKUON, TO_SEION, TO_SEION_CHAR};

impl Haqumei {
    #[inline(always)]
    pub(crate) fn normalize_unicode_if_needed<'a>(&self, text: &'a str) -> Cow<'a, str> {
        match self.options.normalize_unicode {
            UnicodeNormalization::None => Cow::Borrowed(text),
            UnicodeNormalization::Nfc => {
                if is_nfc_quick(text.chars()) == IsNormalized::Yes {
                    Cow::Borrowed(text)
                } else {
                    Cow::Owned(text.nfc().collect::<String>())
                }
            }
            UnicodeNormalization::Nfkc => {
                if is_nfkc_quick(text.chars()) == IsNormalized::Yes {
                    Cow::Borrowed(text)
                } else {
                    Cow::Owned(text.nfkc().collect::<String>())
                }
            }
        }
    }

    pub(crate) fn revert_pron_to_read(&self, njd_features: &mut [NjdFeature]) {
        let options = &self.options;
        debug_assert!(
            options.use_read_as_pron || options.revert_long_vowels || options.revert_yotsugana
        );

        for feature in njd_features.iter_mut() {
            let should_revert_to_read = options.use_read_as_pron
                || (options.revert_long_vowels
                    && feature.pron.contains('ー')
                    && !feature.orig.contains('ー'))
                || (options.revert_yotsugana
                    && (feature.read.contains('ヅ') || feature.read.contains('ヂ')));

            if should_revert_to_read {
                feature.pron = feature.read.clone();
            }
        }
    }

    pub(crate) fn normalize_iu(&self, njd_features: &mut [NjdFeature], option: IuPronunciation) {
        for f in njd_features.iter_mut() {
            let orig = f.orig.as_str();

            if matches!(option, IuPronunciation::KanjiIu | IuPronunciation::KanjiYuu)
                && !orig.contains('言')
                && !orig.contains('云')
            {
                continue;
            }

            if f.pos == "連体詞" {
                if matches!(orig, "こういう" | "そういう" | "どういう" | "ああいう")
                {
                    replace_iu(f, 6..9, option);
                }
                continue;
            }

            if orig.starts_with("ていう") || orig.starts_with("という") {
                replace_iu(f, 3..6, option);
                continue;
            }
            if orig.starts_with("っていう") || orig.starts_with("とかいう") {
                replace_iu(f, 6..9, option);
                continue;
            }

            if orig.starts_with("あっという")
                || orig.starts_with("アッという")
                || orig.starts_with("あっと言う")
                || orig.starts_with("アッと言う")
            {
                replace_iu(f, 9..12, option);
                continue;
            }

            let is_target_pos = (f.pos == "動詞" && f.pos_group1 == "自立")
                || (f.pos == "形容詞" && f.pos_group1.ends_with("自立") /* 自立/非自立 */ && f.ctype == "形容詞・アウオ段")
                || (f.pos == "副詞" && f.pos_group1 == "一般");

            if !is_target_pos {
                continue;
            }

            let orig = f.orig.as_str();

            if f.pron == "イウ"
                || orig.starts_with("いう")
                || orig.starts_with("言う")
                || orig.starts_with("云う")
            {
                replace_iu(f, 0..3, option);
            }
            // 複合語 (e.g., 物言う) などの場合
            else if orig.contains("言う")
                && let Some(pos) = rfind_iu_sound_in_pron(f.pron.as_bytes())
            {
                replace_iu(f, pos..pos + 3, option);
            }
        }
    }
}

#[inline(always)]
fn replace_iu(njd_feature: &mut NjdFeature, range: Range<usize>, option: IuPronunciation) {
    let bytes = unsafe { njd_feature.pron.as_mut_vec() };

    if range.end > bytes.len() {
        return;
    }

    debug_assert_eq!(range.end - range.start, 3);

    match option {
        IuPronunciation::Iu | IuPronunciation::KanjiIu => {
            bytes[range].copy_from_slice("イ".as_bytes());
        }
        IuPronunciation::Yuu | IuPronunciation::KanjiYuu => {
            bytes[range].copy_from_slice("ユ".as_bytes());
        }
    }
}

#[inline(always)]
const fn rfind_iu_sound_in_pron(bytes: &[u8]) -> Option<usize> {
    if bytes.len() < 6 {
        return None;
    }

    let mut i = bytes.len() - 6;
    loop {
        let b1 = bytes[i];
        let b2 = bytes[i + 1];
        let b3 = bytes[i + 2];

        // 「イ」([227, 130, 164]) または 「ユ」([227, 131, 166])
        let is_i_or_yu = (b1 == 227) && ((b2 == 130 && b3 == 164) || (b2 == 131 && b3 == 166));

        if is_i_or_yu {
            let n1 = bytes[i + 3];
            let n2 = bytes[i + 4];
            let n3 = bytes[i + 5];

            // 次の文字が「ウ, ッ, エ, オ, ー」のいずれかであるか
            let is_target_next = (n1 == 227)
                && (
                    (n2 == 130 && n3 == 166) || // ウ
                (n2 == 131 && n3 == 131) || // ッ
                (n2 == 130 && n3 == 168) || // エ
                (n2 == 130 && n3 == 170) || // オ
                (n2 == 131 && n3 == 188)
                    // ー
                );

            if is_target_next {
                return Some(i);
            }
        }

        if i == 0 {
            break;
        }
        i -= 1;
    }
    None
}

/// フィラーが acc > mora_size のときに、平版型 (acc = 0) にし、
/// その直後の形態素が名詞だったとき、
/// その前のフィラーに結合しない (chain_flag = 0) ようにする
pub(crate) fn modify_filler_accent(njd_features: &mut [NjdFeature]) {
    let mut is_after_filler = false;

    for features in njd_features.iter_mut() {
        if features.pos == "フィラー" {
            if features.acc > features.mora_size {
                features.acc = 0;
            }
            is_after_filler = true;
        } else if is_after_filler {
            if features.pos == "名詞" {
                features.chain_flag = 0;
            }
            is_after_filler = false;
        }
    }
}

pub(crate) fn vibrato_analysis(worker: &mut Worker, text: &str) -> Vec<UnidicFeature> {
    VIBRATO_CACHE.get_with(text.to_string(), || {
        worker.reset_sentence(text);
        worker.tokenize();

        worker
            .token_iter()
            .map(|token| {
                let token = token.to_buf();
                let mut ranges = Vec::with_capacity(29);
                let mut start = 0;
                for part in token.feature.split(',') {
                    let end = start + part.len();
                    ranges.push(start..end);
                    start = end + 1;
                }

                UnidicFeature {
                    surface: token.surface,
                    feature: token.feature,
                    range_char: token.range_char,
                    range_byte: token.range_byte,
                    lex_type: token.lex_type,
                    word_id: token.word_id,
                    left_id: token.left_id,
                    right_id: token.right_id,
                    word_cost: token.word_cost,
                    total_cost: token.total_cost,
                    feature_ranges: ranges,
                }
            })
            .collect()
    })
}

pub(crate) mod english {
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
}

/// 1字アルファベット: 処理しない
/// 2字アルファベット: 子音 + A E I O U Y または A I O U + 子音 または A I O U + A I O U
///                 のときだけ処理、それ以外はアクロニムとみなしておく
/// 3字アルファベット: 発音可能かどうかをもとに、処理するかどうかを決定する
/// n字アルファベット: A E I O U Y を含まない文字のとき、処理しない
#[rustfmt::skip]
#[inline(always)]
fn should_use_kanalizer(chars: &[char]) -> bool {
    if chars.len() == 1 {
        return false;
    } else if chars.len() == 2 {
        return (
            english::is_consonant_fullwidth(chars[0])
                && english::is_vowel_fullwidth(chars[1])
        ) || (
            english::is_aeiou_fullwidth(chars[0])
            && english::is_consonant_fullwidth(chars[1])
        ) || (
            english::is_aeiou_fullwidth(chars[0]) && english::is_aeiou_fullwidth(chars[1])
        );
    } else if chars.len() == 3 {
        let has_vowel = chars.iter().any(|&c| english::is_vowel_fullwidth(c));

        if !has_vowel {
            return (chars[1] == chars[2]) && english::is_continuant_fullwidth(chars[1]);
        }

        return english::is_pronounceable([chars[0], chars[1], chars[2]]);
    }

    chars.iter().any(|c| english::is_vowel_fullwidth(*c))
}

/// 自動的に `read`, `pron` がアルファベット読みになってしまう英語のカタカナ読みを推定する。
///
/// "IT" は辞書に拾われ、pos == "名詞" になる一方で、"it" は "i" と "t" と分かれた
/// pos_group1 == "アルファベット" の `NjdFeature` になってしまう。
/// そこで、連続するアルファベットを結合し、条件を満たすものを `Kanalizer` に通している。
pub(crate) fn predict_kana_english(njd_features: &mut Vec<NjdFeature>) {
    let mut i = 0;
    while i < njd_features.len() {
        let is_filler = njd_features[i].pos == "フィラー";
        let is_alphabet = njd_features[i].pos_group1 == "アルファベット";

        if !is_filler && !is_alphabet {
            i += 1;
            continue;
        }

        if njd_features[i]
            .string
            .chars()
            .any(|c| !matches!(c, 'Ａ'..='Ｚ' | 'ａ'..='ｚ'))
        {
            i += 1;
            continue;
        }

        let mut end = i + 1;

        // "アルファベット" (バラバラの文字) の場合のみ、後続のアルファベットをマージする。
        // "フィラー" (未知英単語) の場合は既に1つの単語としてまとまっているのでマージしない。
        if is_alphabet {
            while end < njd_features.len() && njd_features[end].pos_group1 == "アルファベット"
            {
                end += 1;
            }
            if end - i == 1 {
                i += 1;
                continue;
            }
        }

        if end - i > 1 {
            let mut string = String::new();
            let mut orig = String::new();
            let mut read = String::new();
            let mut pron = String::new();
            let mut mora_size = 0;

            for f in &njd_features[i..end] {
                string.push_str(&f.string);
                orig.push_str(&f.orig);
                read.push_str(&f.read);
                pron.push_str(&f.pron);
                mora_size += f.mora_size;
            }

            njd_features[i].string = string;
            njd_features[i].orig = orig;
            njd_features[i].read = read;
            njd_features[i].pron = pron;
            njd_features[i].mora_size = mora_size;

            njd_features.drain(i + 1..end);
        }

        let f = &mut njd_features[i];

        if let Some(kana) = KANALIZER_CACHE.get(&f.string) {
            f.read = kana.clone();
            f.pron = kana;
            f.acc = 0;

            i += 1;
            continue;
        }

        let chars: Vec<char> = f.string.chars().collect();

        if should_use_kanalizer(&chars) {
            let mut kanalizer = KANALIZER.lock().unwrap();

            let options = ConvertOptions {
                max_length: MaxLength::Fixed(
                    std::num::NonZeroUsize::new(f.string.len() * 2).unwrap(),
                ),
                error_on_incomplete: false,
                ..Default::default()
            };

            if let Ok(kana) =
                kanalizer.convert_with_options(&english::fullwidth_to_halfwidth(chars), &options)
            {
                KANALIZER_CACHE.insert(f.string.clone(), kana.clone());

                f.read = kana.clone();
                f.mora_size = count_mora(&kana) as i32;
                f.pron = kana;
                f.acc = 0;
            }
        }

        i += 1;
    }
}

impl Haqumei {
    pub(crate) fn predict_nani_reading(&mut self, njd_features: &mut [NjdFeature]) {
        for i in 0..njd_features.len() {
            if njd_features[i].orig == "何" {
                let next_node_feature = njd_features.get(i + 1);
                let is_read_nan = self.predict_is_nan(next_node_feature);
                let yomi = if is_read_nan { "ナン" } else { "ナニ" };

                njd_features[i].pron = yomi.to_string();
                njd_features[i].read = yomi.to_string();
            }
        }
    }

    pub(crate) fn modify_kanji_yomi(&mut self, text: &str, njd_features: &mut [NjdFeature]) {
        let tokens: Vec<UnidicFeature> = if let Some(rx) = self.rx.take() {
            rx.recv().unwrap_or_default()
        } else {
            VIBRATO_CACHE.get(text).unwrap_or_else(|| {
                let mut worker = self.tokenizer.as_ref().unwrap().new_worker();
                vibrato_analysis(&mut worker, text)
            })
        };

        if tokens.is_empty() {
            return;
        }

        let mut unidic_iter = tokens.into_iter().peekable();
        let mut current_char_pos = 0;
        for njd_feature in njd_features {
            let node_string = &njd_feature.string;
            let node_orig = &njd_feature.orig;
            let node_char_len = node_string.chars().count();

            while let Some(candidate) = unidic_iter.peek() {
                if candidate.range_char.end <= current_char_pos {
                    unidic_iter.next();
                } else {
                    break;
                }
            }

            let mut pron_to_set: Option<String> = None;
            let mut read_to_set: Option<String> = None;

            if MULTI_READ_KANJI_LIST.contains(node_orig.as_str())
                && let Some(candidate) = unidic_iter.peek()
                && candidate.range_char.start == current_char_pos
                && candidate.surface == *node_orig
            {
                let correct_yomi_token = unidic_iter.next().unwrap();

                let reading = correct_yomi_token.pron();
                pron_to_set = Some(reading.to_string());
                read_to_set = Some(reading.to_string());
            }
            if let Some(pron) = pron_to_set {
                njd_feature.pron = pron;
            }
            if let Some(read) = read_to_set {
                njd_feature.read = read;
            }

            current_char_pos += node_char_len;
        }
    }
}

pub(crate) fn modify_english_words(text: &str, njd_features: &mut [NjdFeature]) {
    if njd_features.len() < 2 {
        return;
    }

    #[inline(always)]
    fn get_target_word(s: &str) -> Option<&'static str> {
        let mut chars = s.chars();
        match chars.next()? {
            'a' | 'A' | 'ａ' | 'Ａ' => {
                if chars.next().is_none() {
                    return Some("a");
                }
            }
            'h' | 'H' | 'ｈ' | 'Ｈ' => {
                if let Some('e' | 'E' | 'ｅ' | 'Ｅ') = chars.next()
                    && chars.next().is_none()
                {
                    return Some("he");
                }
            }
            's' | 'S' | 'ｓ' | 'Ｓ' => {
                if let Some('h' | 'H' | 'ｈ' | 'Ｈ') = chars.next()
                    && let Some('e' | 'E' | 'ｅ' | 'Ｅ') = chars.next()
                    && chars.next().is_none()
                {
                    return Some("she");
                }
            }
            _ => {}
        }
        None
    }

    #[inline(always)]
    fn is_all_alphabet(s: &str) -> bool {
        !s.is_empty()
            && s.chars()
                .all(|c| matches!(c, 'Ａ'..='Ｚ' | 'ａ'..='ｚ' | 'A'..='Z' | 'a'..='z'))
    }

    let mut text_lower_cache: Option<String> = None;

    for i in 0..njd_features.len() {
        let curr_orig = &njd_features[i].orig;

        // a / he / she
        if let Some(word) = get_target_word(curr_orig)
            && i + 1 < njd_features.len()
        {
            let next_orig = &njd_features[i + 1].orig;

            if is_all_alphabet(next_orig) {
                let next_half = english::to_halfwidth_lower_string(next_orig);
                let text_lower = text_lower_cache
                    .get_or_insert_with(|| english::to_halfwidth_lower_string(text));

                let pattern_space = format!("{} {}", word, next_half);
                let pattern_full_space = format!("{}　{}", word, next_half);

                if text_lower.contains(&pattern_space) || text_lower.contains(&pattern_full_space) {
                    let curr_mut = &mut njd_features[i];

                    match word {
                        "a" => {
                            curr_mut.read = "ア".to_string();
                            curr_mut.pron = "ア".to_string();
                            curr_mut.mora_size = 1;
                            curr_mut.acc = 0; // 平板型
                        }
                        "he" => {
                            curr_mut.read = "ヒー".to_string();
                            curr_mut.pron = "ヒー".to_string();
                            curr_mut.mora_size = 2;
                            curr_mut.acc = 1; // 頭高型
                        }
                        "she" => {
                            curr_mut.read = "シー".to_string();
                            curr_mut.pron = "シー".to_string();
                            curr_mut.mora_size = 2;
                            curr_mut.acc = 1; // 頭高型
                        }
                        _ => unreachable!(),
                    }
                }
            }
        }

        if i > 0 && i < njd_features.len() - 1 {
            let curr = &njd_features[i];

            let is_two = curr.pos_group1 == "数"
                && curr.pos == "名詞"
                && (curr.orig == "二" || curr.orig == "２" || curr.orig == "2");

            if is_two {
                let prev_orig = &njd_features[i - 1].orig;
                let next_orig = &njd_features[i + 1].orig;

                if is_all_alphabet(prev_orig) && is_all_alphabet(next_orig) {
                    let text_lower = text_lower_cache
                        .get_or_insert_with(|| english::to_halfwidth_lower_string(text));

                    let prev_half = english::to_halfwidth_lower_string(prev_orig);
                    let next_half = english::to_halfwidth_lower_string(next_orig);

                    let pattern = format!("{}2{}", prev_half, next_half);

                    if text_lower.contains(&pattern) {
                        let is_single_both =
                            prev_orig.chars().count() == 1 && next_orig.chars().count() == 1;

                        let curr_mut = &mut njd_features[i];

                        curr_mut.string = "２".to_string();
                        curr_mut.orig = "２".to_string();

                        if is_single_both {
                            curr_mut.read = "ツー".to_string();
                            curr_mut.pron = "ツー".to_string();
                        } else {
                            curr_mut.read = "トゥー".to_string();
                            curr_mut.pron = "トゥー".to_string();
                        }

                        curr_mut.mora_size = 2;
                        curr_mut.acc = 1; // 頭高型
                    }
                }
            }
        }
    }
}

/// 長母音、重母音、撥音がアクセント核に来た場合にひとつ前のモーラにアクセント核がズレるルールを適用します。
pub(crate) fn retreat_acc_nuc(njd_features: &mut [NjdFeature]) {
    if njd_features.is_empty() {
        return;
    }

    const INAPPROPRIATE_FOR_NUCLEAR_CHARS: &[char] = &['ー', 'ッ', 'ン'];

    let mut head_index = 0;
    let mut acc = 0;

    for i in 0..njd_features.len() {
        // アクセント境界直後の node (chain_flag 0 or -1) にアクセント核の位置の情報が入っている
        if njd_features[i].chain_flag == 0 || njd_features[i].chain_flag == -1 {
            head_index = i;
            acc = njd_features[head_index].acc;
        }

        const YOUON_CHARS: &[char] = &['ャ', 'ュ', 'ョ', 'ァ', 'ィ', 'ゥ', 'ェ', 'ォ'];
        let pron_without_youon: String = njd_features[i]
            .pron
            .chars()
            .filter(|c| !YOUON_CHARS.contains(c))
            .collect();

        let pron_ref = if pron_without_youon.is_empty() {
            &njd_features[i].pron
        } else {
            &pron_without_youon
        };

        if acc > 0 {
            if acc <= njd_features[i].mora_size {
                if pron_ref
                    .chars()
                    .nth((acc - 1) as usize)
                    .or(pron_ref.chars().next())
                    .is_some_and(|nuc_pron| INAPPROPRIATE_FOR_NUCLEAR_CHARS.contains(&nuc_pron))
                {
                    njd_features[head_index].acc = njd_features[head_index].acc.saturating_sub(1);
                }

                acc = -1;
            } else {
                acc -= njd_features[i].mora_size;
            }
        }
    }
}

/// 品詞「特殊・マス」は直前に接続する動詞にアクセント核がある場合、アクセント核を「ま」に移動させる法則がある
///   書きます → か[きま]す, 参ります → ま[いりま]す
///   書いております → [か]いております
pub(crate) fn modify_acc_after_chaining(njd_features: &mut [NjdFeature]) {
    if njd_features.is_empty() {
        return;
    }

    const SUFFIXES_TO_MODIFY_ACC: &[&str] = &["れる", "られる", "すぎる", "せる", "させる"];

    let mut head_index = 0;
    let mut acc = 0;

    // アクセント核を含むノードを過ぎたかどうか
    let mut is_after_nuc = false;
    // アクセント句の先頭からのモーラ数
    let mut phase_len = 0;

    for i in 0..njd_features.len() {
        // アクセント境界直後の node (chain_flag 0 or -1) にアクセント核の位置の情報が入っている
        if njd_features[i].chain_flag == 0 || njd_features[i].chain_flag == -1 {
            is_after_nuc = false;
            head_index = i;
            acc = njd_features[head_index].acc;
            phase_len = 0;
        }

        // acc = 0 の場合は「特殊・マス」は存在しないと考えてよい
        if acc == 0 {
            continue;
        }

        let mora_size = njd_features[i].mora_size;
        if is_after_nuc {
            let njd = &njd_features[i];

            if njd.ctype == "特殊・マス" {
                njd_features[head_index].acc = if njd.cform != "未然形" {
                    phase_len + 1
                } else {
                    phase_len + 2
                };
            } else if njd.ctype == "特殊・ナイ" {
                njd_features[head_index].acc = phase_len;
            } else if SUFFIXES_TO_MODIFY_ACC.contains(&njd.orig.as_str()) {
                njd_features[head_index].acc = phase_len + njd.acc;
            } else {
                is_after_nuc = false;
                acc = 0;
            }

            phase_len += mora_size;
        } else {
            phase_len += mora_size;
            if acc <= mora_size {
                is_after_nuc = true;
            } else {
                acc -= mora_size;
            }
        }
    }
}

#[inline(always)]
fn set_to_noun(feat: &mut NjdFeature) {
    feat.pos = "名詞".to_string();
    feat.pos_group1 = "一般".to_string();
    feat.pos_group2 = "*".to_string();
    feat.pos_group3 = "*".to_string();
    feat.ctype = "*".to_string();
    feat.cform = "*".to_string();
}

// 文字列を静音化し、末尾の「々」に対応する繰り返し単位を検出する
fn detect_odori_unit(read: &str) -> Option<usize> {
    let seion_read: String = read
        .chars()
        .map(|ch| {
            if is_dakuon(ch) {
                TO_SEION_CHAR.get(&ch).copied().unwrap_or(ch)
            } else {
                ch
            }
        })
        .collect();
    let moras = split_kana_mora(&seion_read);
    let n = moras.len();
    if n < 2 {
        return None;
    }

    // 後ろ半分が前半分と一致する最小の単位を探す
    for len in 1..=(n / 2) {
        let first_half = &moras[n - len * 2..n - len];
        let second_half = &moras[n - len..n];
        if first_half == second_half {
            return Some(len);
        }
    }
    None
}

#[rustfmt::skip]
pub(crate) fn is_dakuon(c: char) -> bool {
    matches!(
        c,
        'が' | 'ぎ' | 'ぐ' | 'げ' | 'ご'
        | 'ざ' | 'じ' | 'ず' | 'ぜ' | 'ぞ'
        | 'だ' | 'ぢ' | 'づ' | 'で' | 'ど'
        | 'ば' | 'び' | 'ぶ' | 'べ' | 'ぼ'
        | 'ガ' | 'ギ' | 'グ' | 'ゲ' | 'ゴ'
        | 'ザ' | 'ジ' | 'ズ' | 'ゼ' | 'ゾ'
        | 'ダ' | 'ヂ' | 'ヅ' | 'デ' | 'ド'
        | 'バ' | 'ビ' | 'ブ' | 'ベ' | 'ボ'
        | 'ヴ'
    )
}

/// 踊り字（々）と一の字点（ゝ、ゞ、ヽ、ヾ）の読みを処理する後処理関数
pub(crate) fn process_odori_features(
    njd_features: &mut Vec<NjdFeature>,
    open_jtalk: &mut OpenJTalk,
) -> Result<(), HaqumeiError> {
    let mut i = 0;
    while i < njd_features.len() {
        let orig = &njd_features[i].orig;
        if is_dounojiten(orig) {
            // 踊り字「々」の処理
            let mut reanalysis_result = None;
            if i > 0 {
                let prev = &njd_features[i - 1];
                if count_dounojiten(orig) == 1 && is_kanji_feature(prev) {
                    let prev_chars: Vec<char> = prev.orig.chars().collect();
                    if prev_chars.len() > 1 {
                        let last_char = *prev_chars.last().unwrap();
                        if is_kanji(last_char) {
                            // 後続トークンのチェック
                            let next_token_opt = if i + 1 < njd_features.len() {
                                Some(&njd_features[i + 1])
                            } else {
                                None
                            };

                            // 後続が1文字の漢字なら巻き込んで再解析
                            let (target_text, consumed_next) = if let Some(next) = next_token_opt {
                                if is_single_kanji_feature(next) {
                                    (format!("{}{}", last_char, next.orig), true)
                                } else {
                                    (last_char.to_string(), false)
                                }
                            } else {
                                (last_char.to_string(), false)
                            };

                            reanalysis_result = Some((target_text, consumed_next));
                        }
                    }
                }
            }

            // 再解析実行と適用
            if let Some((text, consumed_next)) = reanalysis_result {
                let mut analyzed = open_jtalk.run_frontend(&text)?;

                if let Some(first) = analyzed.get_mut(0) {
                    first.chain_flag = 1;
                }

                let range_end = if consumed_next { i + 2 } else { i + 1 };
                let analyzed_len = analyzed.len();

                if range_end <= njd_features.len() {
                    njd_features.splice(i..range_end, analyzed);

                    if !consumed_next && analyzed_len > 0 {
                        set_to_noun(&mut njd_features[i]);
                        i += 1;
                    } else {
                        i += analyzed_len;
                    }
                    continue;
                }
            }

            // 連続踊り字の展開処理
            let start = i;
            let mut end = i;
            let mut total_odori = 0;
            while end < njd_features.len() && is_dounojiten(&njd_features[end].orig) {
                total_odori += count_dounojiten(&njd_features[end].orig);
                end += 1;
            }

            if i > 0 && njd_features[i - 1].orig.ends_with('々') {
                let prev = &njd_features[i - 1];
                let base_acc = prev.acc;

                // 清音ベースで「繰り返しの長さ」を特定
                if let Some(period) = detect_odori_unit(&prev.read) {
                    let raw_read_moras = split_kana_mora(&prev.read);
                    let raw_pron_moras = split_kana_mora(&prev.pron);

                    if raw_read_moras.len() >= period {
                        let unit_read = raw_read_moras[raw_read_moras.len() - period..].join("");
                        let unit_pron = raw_pron_moras[raw_pron_moras.len() - period..].join("");
                        let unit_mora =
                            (prev.mora_size / raw_read_moras.len() as i32) * period as i32;

                        let current_feat = &mut njd_features[i];
                        let count = count_dounojiten(&current_feat.orig);

                        current_feat.read = unit_read.repeat(count);
                        current_feat.pron = unit_pron.repeat(count);
                        current_feat.mora_size = unit_mora * count as i32;

                        current_feat.acc = base_acc;
                        current_feat.chain_flag = 1;

                        if current_feat.pos == "記号" {
                            set_to_noun(current_feat);
                        }
                        i += 1;
                        continue;
                    }
                }
            }

            // 直前の漢字トークンを収集
            let mut normal_indices = Vec::new();
            let mut j = start;
            let mut collected_chars = 0;
            let needed_chars = total_odori.min(8);

            while j > 0 {
                j -= 1;
                let target = &njd_features[j];

                if matches!(target.pos.as_str(), "記号" | "フィラー" | "感動詞") {
                    break;
                }

                if is_kanji_feature(target) {
                    normal_indices.push(j);
                    collected_chars += target.orig.chars().count();
                    if collected_chars >= needed_chars {
                        break;
                    }
                } else {
                    break;
                }
            }
            normal_indices.reverse();

            if normal_indices.is_empty() {
                i = end;
                continue;
            }

            let base_acc = njd_features[normal_indices[0]].acc;

            // 置換用データの作成
            let is_single_kanji = normal_indices.len() == 1
                && njd_features[normal_indices[0]].orig.chars().count() == 1;

            let (base_read, base_pron, base_mora_size) = if is_single_kanji {
                let f = &njd_features[normal_indices[0]];
                (f.read.clone(), f.pron.clone(), f.mora_size)
            } else {
                let mut r = String::new();
                let mut p = String::new();
                let mut m = 0;
                for &idx in &normal_indices {
                    r.push_str(&njd_features[idx].read);
                    p.push_str(&njd_features[idx].pron);
                    m += njd_features[idx].mora_size;
                }
                (r, p, m)
            };

            for mut njd_feature in njd_features.iter_mut().take(end).skip(start) {
                let current_odori = count_dounojiten(&njd_feature.orig);
                let feat = &mut njd_feature;

                if is_single_kanji {
                    feat.read = base_read.repeat(current_odori);
                    feat.pron = base_pron.repeat(current_odori);
                    feat.mora_size = base_mora_size * current_odori as i32;
                } else {
                    feat.read = base_read.clone();
                    feat.pron = base_pron.clone();
                    feat.mora_size = base_mora_size;
                }
                feat.acc = base_acc; // 直前の漢字トークンの acc を使う
                feat.chain_flag = 1;

                if feat.pos == "記号" {
                    set_to_noun(feat);
                }
            }
            i = end;
        } else if is_ichinojiten(orig) {
            // 一の字点（ゝ、ゞ、ヽ、ヾ）の処理
            if i > 0 {
                // 直前が記号でないか
                if njd_features[i - 1].pos != "記号" {
                    let mut prev_index = None;
                    let mut k = i;
                    while k > 0 {
                        k -= 1;
                        if njd_features[k].pos != "記号" && njd_features[k].mora_size > 0 {
                            prev_index = Some(k);
                            break;
                        }
                    }

                    if let Some(pidx) = prev_index {
                        let prev_read = njd_features[pidx].read.clone();
                        let prev_pron = njd_features[pidx].pron.clone();
                        let prev_mora_size = njd_features[pidx].mora_size;

                        let curr = &mut njd_features[i];
                        apply_odoriji_logic(curr, &prev_read, &prev_pron, prev_mora_size);
                    }
                }
            }
            i += 1;
        } else {
            i += 1;
        }
    }
    Ok(())
}

/// 文字列がすべて同の字点かどうか
#[inline(always)]
fn is_dounojiten(orig: &str) -> bool {
    !orig.is_empty() && orig.chars().all(|c| c == '々')
}

/// 文字列がすべて一の字点かどうか
#[inline(always)]
fn is_ichinojiten(orig: &str) -> bool {
    !orig.is_empty() && orig.chars().all(|c| matches!(c, 'ゝ' | 'ゞ' | 'ヽ' | 'ヾ'))
}

/// 文字列に含まれる同の字点の数
#[inline(always)]
fn count_dounojiten(orig: &str) -> usize {
    orig.chars().filter(|&c| c == '々').count()
}

fn apply_odoriji_logic(
    odori_feature: &mut NjdFeature,
    prev_read: &str,
    prev_pron: &str,
    prev_mora_size: i32,
) {
    let prev_read_mora = split_kana_mora(prev_read);
    let prev_pron_source = if prev_pron.contains('’') {
        Cow::Owned(prev_pron.replace('’', ""))
    } else {
        Cow::Borrowed(prev_pron)
    };
    let prev_pron_source = if prev_pron_source.is_empty() {
        prev_read
    } else {
        prev_pron_source.as_ref()
    };
    let prev_pron_mora = split_kana_mora(prev_pron_source);

    if prev_read_mora.is_empty() {
        return;
    }

    let mora_val = prev_mora_size / prev_read_mora.len() as i32;
    let target_read = prev_read_mora.last().unwrap();
    let target_pron = prev_pron_mora.last().unwrap_or(target_read);

    let mut is_forced_voiced = false;
    for c in odori_feature.orig.chars().peekable() {
        if matches!(c, 'ゞ' | 'ヾ') {
            is_forced_voiced = true;
            break;
        }
        if matches!(c, 'ゝ' | 'ヽ') {
            break;
        }
    }

    // 対象モーラが単一の仮名 grapheme か判定する。
    //
    // 一の字点 (ゝ, ゞ, ヽ, ヾ) は歴史的に「直前の仮名1文字」を
    // 繰り返す記号であり、拗音 (きゃ, しゃ 等) のような
    // 複数仮名からなるモーラに対して使われる例はほぼ存在しない。
    //
    // そのため厳密な規則を定義するのは難しく、実際のテキストでも
    // 拗音に対して踊り字が使われるケースは想定しにくい。
    //
    // 二字以上扱う [くの字点](https://ja.wikipedia.org/wiki/踊り字#〱（くの字点）) についても、
    //
    // > 濁点の付く文字を繰り返す場合は、濁点の付いていない「くの字点」を用いる場合と、濁点の付いている「くの字点」を用いる場合がある。
    //
    // とあって別に厳密にルール付けることはできないし、
    // これに文脈に合わせた推定をするロジックを書くぐらいならもっとやった方がいいことがある。
    // でも需要がありそうなのは濁音を維持して繰り返すケースっぽそう。

    let is_single_grapheme_mora = {
        let mut chars = target_read.chars();
        !chars.any(is_small_kana)
    };

    if is_forced_voiced {
        // 濁音の踊り字 (ゞ, ヾ) -> 強制的に濁音化
        odori_feature.read = TO_DAKUON
            .get(target_read)
            .copied()
            .unwrap_or(target_read)
            .to_string();
        odori_feature.pron = TO_DAKUON
            .get(target_pron)
            .copied()
            .unwrap_or(target_pron)
            .to_string();
    } else {
        // 清音の踊り字 (ゝ, ヽ)
        if is_single_grapheme_mora {
            // 対象が単一文字の場合 -> 清音化
            odori_feature.read = TO_SEION
                .get(target_read)
                .copied()
                .unwrap_or(target_read)
                .to_string();
            odori_feature.pron = TO_SEION
                .get(target_pron)
                .copied()
                .unwrap_or(target_pron)
                .to_string();
        } else {
            // 対象が拗音などの複数文字の場合 -> 濁点を維持する
            odori_feature.read = target_read.to_string();
            odori_feature.pron = target_pron.to_string();
        }
    }

    odori_feature.mora_size = mora_val;
    if odori_feature.pos == "記号" {
        set_to_noun(odori_feature);
    }
}
