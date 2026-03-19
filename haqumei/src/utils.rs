use std::fs::Metadata;

use sha2::{Digest, Sha256};

use crate::NjdFeature;

/// カタカナをひらがなに変換する
pub fn kata2hira(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'ァ'..='ヶ' | 'ヽ'..='ヾ' => std::char::from_u32(c as u32 - 0x60).unwrap_or(c),
            _ => c,
        })
        .collect()
}

/// ひらがなをカタカナに変換する
pub fn hira2kata(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'ぁ'..='ゖ' | 'ゝ'..='ゞ' => std::char::from_u32(c as u32 + 0x60).unwrap_or(c),
            _ => c,
        })
        .collect()
}

#[rustfmt::skip]
pub fn is_dakuon(c: char) -> bool {
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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub(crate) enum Dan {
    ア段 = 0,
    イ段 = 1,
    ウ段 = 2,
    エ段 = 3,
    オ段 = 4,
}

#[inline]
pub(crate) fn dan(c: char) -> Option<Dan> {
    match c {
        // ア段
        'ア' | 'カ' | 'サ' | 'タ' | 'ナ' | 'ハ' | 'マ' | 'ヤ' | 'ラ' | 'ワ' | 'ガ' | 'ザ'
        | 'ダ' | 'バ' | 'パ' | 'ァ' => Some(Dan::ア段),

        // イ段
        'イ' | 'キ' | 'シ' | 'チ' | 'ニ' | 'ヒ' | 'ミ' | 'リ' | 'ギ' | 'ジ' | 'ヂ' | 'ビ'
        | 'ピ' | 'ィ' => Some(Dan::イ段),

        // ウ段
        'ウ' | 'ク' | 'ス' | 'ツ' | 'ヌ' | 'フ' | 'ム' | 'ユ' | 'ル' | 'グ' | 'ズ' | 'ヅ'
        | 'ブ' | 'プ' | 'ヴ' | 'ゥ' => Some(Dan::ウ段),

        // エ段
        'エ' | 'ケ' | 'セ' | 'テ' | 'ネ' | 'ヘ' | 'メ' | 'レ' | 'ゲ' | 'ゼ' | 'デ' | 'ベ'
        | 'ペ' | 'ェ' => Some(Dan::エ段),

        // オ段
        'オ' | 'コ' | 'ソ' | 'ト' | 'ノ' | 'ホ' | 'モ' | 'ヨ' | 'ロ' | 'ヲ' | 'ゴ' | 'ゾ'
        | 'ド' | 'ボ' | 'ポ' | 'ォ' => Some(Dan::オ段),

        _ => None,
    }
}

#[inline(always)]
pub(crate) fn is_kanji(c: char) -> bool {
    ('\u{4E00}'..='\u{9FFF}').contains(&c)
}

#[inline(always)]
pub(crate) fn is_kanji_feature(feaure: &NjdFeature) -> bool {
    feaure.pos != "記号" && feaure.orig.chars().any(is_kanji)
}

#[inline(always)]
pub(crate) fn is_single_kanji_feature(feaure: &NjdFeature) -> bool {
    is_kanji_feature(feaure)
        && feaure.orig.chars().count() == 1
        && is_kanji(feaure.orig.chars().next().unwrap())
}

#[inline(always)]
pub(crate) fn is_small_kana(c: char) -> bool {
    matches!(c, 'ャ' | 'ュ' | 'ョ' | 'ァ' | 'ィ' | 'ゥ' | 'ェ' | 'ォ')
}

/// 文字列をモーラ単位 (小書き文字を前の文字に結合) で分割する
#[inline(always)]
pub(crate) fn split_kana_mora(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut result = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if i + 1 < chars.len() && is_small_kana(chars[i + 1]) {
            result.push(format!("{}{}", c, chars[i + 1]));
            i += 2;
        } else {
            result.push(c.to_string());
            i += 1;
        }
    }
    result
}

/// 文字列の中に踊り字が含まれているかどうか
#[inline]
pub(crate) fn has_odori_chars(surface: &str) -> bool {
    surface
        .chars()
        .any(|c| matches!(c, '々' | 'ゝ' | 'ゞ' | 'ヽ' | 'ヾ'))
}

#[inline(always)]
pub(crate) fn compute_metadata_key(meta: &Metadata) -> [u8; 32] {
    let mut hasher = Sha256::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        hasher.update(meta.dev().to_le_bytes());
        hasher.update(meta.ino().to_le_bytes());
        hasher.update(meta.size().to_le_bytes());
        hasher.update(meta.mtime().to_le_bytes());
        hasher.update(meta.mtime_nsec().to_le_bytes());
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        hasher.update(meta.file_size().to_le_bytes());
        hasher.update(meta.last_write_time().to_le_bytes());
        hasher.update(meta.creation_time().to_le_bytes());
        hasher.update(meta.file_attributes().to_le_bytes());
    }

    #[cfg(not(any(unix, windows)))]
    {
        use std::time::SystemTime;

        fn update_system_time(time: Result<SystemTime, std::io::Error>, hasher: &mut Sha256) {
            match time.and_then(|t| {
                t.duration_since(SystemTime::UNIX_EPOCH)
                    .map_err(|_| std::io::Error::from(std::io::ErrorKind::Other))
            }) {
                Ok(duration) => {
                    hasher.update(duration.as_secs().to_le_bytes());
                    hasher.update(duration.subsec_nanos().to_le_bytes());
                }
                Err(_) => {
                    hasher.update([0u8; 12]);
                }
            }
        }

        let file_type = meta.file_type();
        let type_byte: u8 = if file_type.is_file() {
            0x01
        } else if file_type.is_dir() {
            0x02
        } else if file_type.is_symlink() {
            0x03
        } else {
            0x00
        };
        hasher.update([type_byte]);

        let readonly_byte: u8 = if meta.permissions().readonly() {
            0x01
        } else {
            0x00
        };
        hasher.update([readonly_byte]);

        hasher.update(meta.len().to_le_bytes());

        update_system_time(meta.modified(), &mut hasher);

        update_system_time(meta.created(), &mut hasher);
    }

    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hira2kata_basic() {
        assert_eq!(hira2kata("あいうえお"), "アイウエオ");
        assert_eq!(hira2kata("ぱぴぷぺぽ"), "パピプペポ");
        assert_eq!(hira2kata("ちゃちゅちょ"), "チャチュチョ");
        assert_eq!(hira2kata("っ"), "ッ");
    }

    #[test]
    fn test_kata2hira_basic() {
        assert_eq!(kata2hira("アイウエオ"), "あいうえお");
        assert_eq!(kata2hira("パピプペポ"), "ぱぴぷぺぽ");
        assert_eq!(kata2hira("チャチュチョ"), "ちゃちゅちょ");
        assert_eq!(kata2hira("ッ"), "っ");
    }

    #[test]
    fn test_precomposed_dakuten() {
        // 「が」(U+304C) <-> 「ガ」(U+30AC) の差は 0x60 なので正しく変換される
        assert_eq!(hira2kata("がぎぐげご"), "ガギグゲゴ");
        assert_eq!(kata2hira("ガギグゲゴ"), "がぎぐげご");
    }

    #[test]
    fn test_combining_dakuten() {
        // 結合濁点(U+3099)や結合半濁点(U+309A)は、変換範囲外のため「そのまま」残る
        let hira_combined = "か\u{3099}"; // か + 結合濁点
        let kata_combined = "カ\u{3099}"; // カ + 結合濁点

        assert_eq!(hira2kata(hira_combined), kata_combined);
        assert_eq!(kata2hira(kata_combined), hira_combined);
    }

    #[test]
    fn test_edge_cases() {
        // ひらがな: ぁ(3041) 〜 ゖ(3096)
        // カタカナ: ァ(30A1) 〜 ヶ(30F6)
        assert_eq!(hira2kata("ぁ"), "ァ");
        assert_eq!(hira2kata("ゖ"), "ヶ");
        assert_eq!(kata2hira("ァ"), "ぁ");
        assert_eq!(kata2hira("ヶ"), "ゖ");

        // 「ゔ」(U+3094) <-> 「ヴ」(U+30F4) も 0x60 差なので範囲内
        assert_eq!(hira2kata("ゔ"), "ヴ");
        assert_eq!(kata2hira("ヴ"), "ゔ");
    }

    #[test]
    fn test_non_target_characters() {
        let mixed = "あ漢123!ー A";
        assert_eq!(hira2kata(mixed), "ア漢123!ー A");
        assert_eq!(kata2hira("ア漢123!ー A"), mixed);

        assert_eq!(hira2kata("ｱｲｳ"), "ｱｲｳ");
    }

    #[test]
    fn test_choonpu() {
        assert_eq!(hira2kata("らーめん"), "ラーメン");
        assert_eq!(kata2hira("ラーメン"), "らーめん");
    }

    #[test]
    fn test_special_hira() {
        // 「ゐ」(U+3090) / 「ゑ」(U+3091)
        // 「ヰ」(U+30F0) / 「ヱ」(U+30F1)
        assert_eq!(hira2kata("ゐゑ"), "ヰヱ");
        assert_eq!(kata2hira("ヰヱ"), "ゐゑ");
    }
}
