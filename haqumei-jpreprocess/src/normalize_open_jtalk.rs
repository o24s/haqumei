use phf::{Map, phf_map};

/// Open JTalk と同じ変換表で正規化します。ASCII の制御文字は出力しません。
pub fn normalize_text_for_open_jtalk(input: &str) -> String {
    normalize_text_for_open_jtalk_with_mapping(input, |_, _| {})
}

/// 正規化し、変換ごとに元のバイト区間と出力の文字区間を通知します。
///
/// 除去される文字も空の出力区間として通知します。
pub fn normalize_text_for_open_jtalk_with_mapping(
    input: &str,
    mut mapped: impl FnMut(std::ops::Range<usize>, std::ops::Range<usize>),
) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.char_indices().peekable();
    let mut count = 0;
    while let Some((start, current)) = chars.next() {
        let before = count;
        if let Some(&(next, ch)) = chars.peek()
            && let Some(replacement) = COMPOSED.get(&input[start..next + ch.len_utf8()])
        {
            result.push_str(replacement);
            chars.next();
            count += replacement.chars().count();
            mapped(start..next + ch.len_utf8(), before..count);
            continue;
        }
        if let Some(replacement) = SINGLE.get(&current) {
            result.push_str(replacement);
            count += replacement.chars().count();
        } else if !current.is_ascii_control() {
            result.push(current);
            count += 1;
        }
        mapped(start..start + current.len_utf8(), before..count);
    }
    result
}

const SINGLE: Map<char, &str> = phf_map! {
    ' ' => "　",
    '!' => "！",
    '"' => "”",
    '#' => "＃",
    '$' => "＄",
    '%' => "％",
    '&' => "＆",
    '\'' => "’",
    '(' => "（",
    ')' => "）",
    '*' => "＊",
    '+' => "＋",
    ',' => "，",
    '-' => "−",
    '.' => "．",
    '/' => "／",
    '0' => "０",
    '1' => "１",
    '2' => "２",
    '3' => "３",
    '4' => "４",
    '5' => "５",
    '6' => "６",
    '7' => "７",
    '8' => "８",
    '9' => "９",
    ':' => "：",
    ';' => "；",
    '<' => "＜",
    '=' => "＝",
    '>' => "＞",
    '?' => "？",
    '@' => "＠",
    'A' => "Ａ",
    'B' => "Ｂ",
    'C' => "Ｃ",
    'D' => "Ｄ",
    'E' => "Ｅ",
    'F' => "Ｆ",
    'G' => "Ｇ",
    'H' => "Ｈ",
    'I' => "Ｉ",
    'J' => "Ｊ",
    'K' => "Ｋ",
    'L' => "Ｌ",
    'M' => "Ｍ",
    'N' => "Ｎ",
    'O' => "Ｏ",
    'P' => "Ｐ",
    'Q' => "Ｑ",
    'R' => "Ｒ",
    'S' => "Ｓ",
    'T' => "Ｔ",
    'U' => "Ｕ",
    'V' => "Ｖ",
    'W' => "Ｗ",
    'X' => "Ｘ",
    'Y' => "Ｙ",
    'Z' => "Ｚ",
    '[' => "［",
    '\\' => "￥",
    ']' => "］",
    '^' => "＾",
    '_' => "＿",
    '`' => "‘",
    'a' => "ａ",
    'b' => "ｂ",
    'c' => "ｃ",
    'd' => "ｄ",
    'e' => "ｅ",
    'f' => "ｆ",
    'g' => "ｇ",
    'h' => "ｈ",
    'i' => "ｉ",
    'j' => "ｊ",
    'k' => "ｋ",
    'l' => "ｌ",
    'm' => "ｍ",
    'n' => "ｎ",
    'o' => "ｏ",
    'p' => "ｐ",
    'q' => "ｑ",
    'r' => "ｒ",
    's' => "ｓ",
    't' => "ｔ",
    'u' => "ｕ",
    'v' => "ｖ",
    'w' => "ｗ",
    'x' => "ｘ",
    'y' => "ｙ",
    'z' => "ｚ",
    '{' => "｛",
    '|' => "｜",
    '}' => "｝",
    '~' => "〜",
    '｡' => "。",
    '｢' => "「",
    '｣' => "」",
    '､' => "、",
    '･' => "・",
    'ｦ' => "ヲ",
    'ｧ' => "ァ",
    'ｨ' => "ィ",
    'ｩ' => "ゥ",
    'ｪ' => "ェ",
    'ｫ' => "ォ",
    'ｬ' => "ャ",
    'ｭ' => "ュ",
    'ｮ' => "ョ",
    'ｯ' => "ッ",
    'ｰ' => "ー",
    'ｱ' => "ア",
    'ｲ' => "イ",
    'ｳ' => "ウ",
    'ｴ' => "エ",
    'ｵ' => "オ",
    'ｶ' => "カ",
    'ｷ' => "キ",
    'ｸ' => "ク",
    'ｹ' => "ケ",
    'ｺ' => "コ",
    'ｻ' => "サ",
    'ｼ' => "シ",
    'ｽ' => "ス",
    'ｾ' => "セ",
    'ｿ' => "ソ",
    'ﾀ' => "タ",
    'ﾁ' => "チ",
    'ﾂ' => "ツ",
    'ﾃ' => "テ",
    'ﾄ' => "ト",
    'ﾅ' => "ナ",
    'ﾆ' => "ニ",
    'ﾇ' => "ヌ",
    'ﾈ' => "ネ",
    'ﾉ' => "ノ",
    'ﾊ' => "ハ",
    'ﾋ' => "ヒ",
    'ﾌ' => "フ",
    'ﾍ' => "ヘ",
    'ﾎ' => "ホ",
    'ﾏ' => "マ",
    'ﾐ' => "ミ",
    'ﾑ' => "ム",
    'ﾒ' => "メ",
    'ﾓ' => "モ",
    'ﾔ' => "ヤ",
    'ﾕ' => "ユ",
    'ﾖ' => "ヨ",
    'ﾗ' => "ラ",
    'ﾘ' => "リ",
    'ﾙ' => "ル",
    'ﾚ' => "レ",
    'ﾛ' => "ロ",
    'ﾜ' => "ワ",
    'ﾝ' => "ン",
    'ﾞ' => "",
    'ﾟ' => "",
};

const COMPOSED: Map<&str, &str> = phf_map! {
    "ｳﾞ" => "ヴ",
    "ｶﾞ" => "ガ",
    "ｷﾞ" => "ギ",
    "ｸﾞ" => "グ",
    "ｹﾞ" => "ゲ",
    "ｺﾞ" => "ゴ",
    "ｻﾞ" => "ザ",
    "ｼﾞ" => "ジ",
    "ｽﾞ" => "ズ",
    "ｾﾞ" => "ゼ",
    "ｿﾞ" => "ゾ",
    "ﾀﾞ" => "ダ",
    "ﾁﾞ" => "ヂ",
    "ﾂﾞ" => "ヅ",
    "ﾃﾞ" => "デ",
    "ﾄﾞ" => "ド",
    "ﾊﾞ" => "バ",
    "ﾋﾞ" => "ビ",
    "ﾌﾞ" => "ブ",
    "ﾍﾞ" => "ベ",
    "ﾎﾞ" => "ボ",
    "ﾊﾟ" => "パ",
    "ﾋﾟ" => "ピ",
    "ﾌﾟ" => "プ",
    "ﾍﾟ" => "ペ",
    "ﾎﾟ" => "ポ",
};

#[cfg(test)]
mod tests {
    use super::{normalize_text_for_open_jtalk, normalize_text_for_open_jtalk_with_mapping};

    #[test]
    fn mapping_tracks_composed_removed_and_unchanged_characters() {
        let mut spans = Vec::new();
        let normalized =
            normalize_text_for_open_jtalk_with_mapping("\tｶﾞAﾟ𠮷", |from, to| {
                spans.push((from, to));
            });
        assert_eq!(normalized, "ガＡ𠮷");
        assert_eq!(
            spans,
            [
                (0..1, 0..0),
                (1..7, 0..1),
                (7..8, 1..2),
                (8..11, 2..2),
                (11..15, 2..3),
            ]
        );
    }

    #[test]
    fn ascii_symbols_keep_open_jtalk_spellings() {
        assert_eq!(
            normalize_text_for_open_jtalk("AZaz09 \"'`-~\\!?"),
            "ＡＺａｚ０９　”’‘−〜￥！？"
        );
    }

    #[test]
    fn halfwidth_voicing_marks_only_compose_with_supported_kana() {
        assert_eq!(
            normalize_text_for_open_jtalk("ｳﾞｶﾞﾊﾟﾜﾞｶﾟﾞﾟ「か\u{3099}」"),
            "ヴガパワカ「か\u{3099}」"
        );
    }

    #[test]
    fn controls_are_omitted_without_changing_unicode_text() {
        assert_eq!(
            normalize_text_for_open_jtalk("\0\tA\r\n\u{7f}𠮷🙂\u{85}Ｂ"),
            "Ａ𠮷🙂\u{85}Ｂ"
        );
    }
}
