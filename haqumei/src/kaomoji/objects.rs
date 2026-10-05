use std::ops::Range;

#[derive(Clone, Debug)]
pub struct Proposal {
    pub extent: Range<usize>,
    pub left: Option<&'static str>,
    pub right: Option<&'static str>,
}

fn bar(c: char) -> bool {
    "-－−ーｰ―─━═╌┄".contains(c)
}
fn space(c: char) -> bool {
    matches!(c, ' ' | '　')
}
fn round(c: char) -> bool {
    "○●◎〇◯￮".contains(c)
}
fn steam(c: char) -> bool {
    "~～〜”\"".contains(c)
}
fn hand(c: char) -> bool {
    "つっ⊂⊃ﾉノヾヽoｏcｃ_＿/／＼\\".contains(c)
}
fn decoration(c: char) -> bool {
    "ﾟ゜｡。.*＊･・:：'’✧".contains(c)
}

// 続く文章の先頭が用途を変える例がある。○○○円や☆5個を持ち物と断定しない。
fn occupied(c: char) -> bool {
    c.is_numeric()
        || "円件個点番号年月日時分秒割％%式項口人台本桁".contains(c)
        || c.is_ascii_alphanumeric()
        || ('Ａ'..='Ｚ').contains(&c)
        || ('ａ'..='ｚ').contains(&c)
}

fn object(chars: &[char]) -> Option<(usize, &'static str)> {
    let mut i = 0;
    while i < 2 && chars.get(i).is_some_and(|&c| space(c)) {
        i += 1;
    }
    let start = i;
    while i - start < 12 && chars.get(i).is_some_and(|&c| bar(c)) {
        i += 1;
    }
    let bars = i - start;
    let symbol_start = i;
    let kind;
    if bars > 0 && chars.get(i).is_some_and(|&c| round(c)) {
        while chars.get(i).is_some_and(|&c| round(c)) && i - symbol_start < 6 {
            i += 1;
        }
        if i - symbol_start < 2 || chars.get(i).is_some_and(|&c| round(c)) {
            return None;
        }
        kind = "skewer";
        if chars.get(i).is_some_and(|&c| bar(c)) {
            i += 1;
        }
    } else if bars > 0 && chars.get(i).is_some_and(|&c| "☆★✿❀".contains(c)) {
        while chars.get(i).is_some_and(|&c| "☆★✿❀".contains(c)) && i - symbol_start < 8 {
            i += 1;
        }
        if chars.get(i).is_some_and(|&c| "☆★✿❀".contains(c)) {
            return None;
        }
        kind = "wand";
        let at = i;
        while chars.get(i).is_some_and(|&c| decoration(c)) && i - at < 8 {
            i += 1;
        }
    } else if bars > 0 && chars.get(i) == Some(&'>') {
        i += 1;
        if !chars.get(i).is_some_and(|&c| "ﾟ゜".contains(c)) {
            return None;
        }
        i += 1;
        let at = i;
        while chars.get(i) == Some(&')') && i - at < 8 {
            i += 1;
        }
        if i == at || !chars.get(i).is_some_and(|&c| "≫>".contains(c)) {
            return None;
        }
        i += 1;
        if chars.get(i) != Some(&'彡') {
            return None;
        }
        i += 1;
        let end = i;
        let mut last = i;
        while i - end < 8 && chars.get(i).is_some_and(|&c| c == '~' || space(c)) {
            if chars[i] == '~' {
                last = i + 1;
            }
            i += 1;
        }
        i = last;
        kind = "fishing";
    } else if bars > 0 && chars.get(i..i + 4) == Some(&['[', '二', '二', ']']) {
        i += 4;
        kind = "boxed_food";
    } else if bars > 0 && chars.get(i..i + 2) == Some(&['{', '}']) {
        i += 2;
        let mut count = 1;
        while chars.get(i..i + 3) == Some(&['@', '{', '}']) && count < 4 {
            i += 3;
            count += 1;
        }
        if count < 2 {
            return None;
        }
        if chars.get(i).is_some_and(|&c| bar(c)) {
            i += 1;
        }
        kind = "skewered_food";
    } else if bars == 0 && chars.get(i) == Some(&'︻') {
        i += 1;
        if chars.get(i) == Some(&'┻') {
            i += 1;
        }
        if !chars.get(i).is_some_and(|&c| "┳デ".contains(c)) {
            return None;
        }
        i += 1;
        let at = i;
        while chars.get(i).is_some_and(|&c| "═━".contains(c)) && i - at < 8 {
            i += 1;
        }
        if i == at || chars.get(i) != Some(&'一') {
            return None;
        }
        i += 1;
        kind = "drawn_weapon";
    } else if bars == 0
        && chars.get(i..i + 2).is_some_and(|s| {
            "￭■□".contains(s[0]) && "PDCＰＤＣ".contains(s[1])
                || "PDCＰＤＣ".contains(s[0]) && "￭■□".contains(s[1])
        })
    {
        i += 2;
        let at = i;
        while chars.get(i).is_some_and(|&c| steam(c)) && i - at < 4 {
            i += 1;
        }
        kind = "mug";
    } else if bars == 0 && chars.get(i).is_some_and(|&c| "旦且🍡☕🍵".contains(c)) {
        let cup = chars[i];
        i += 1;
        if chars
            .get(i)
            .is_some_and(|&c| matches!(c, '\u{fe0e}' | '\u{fe0f}'))
        {
            i += 1;
        }
        let at = i;
        while chars.get(i).is_some_and(|&c| steam(c)) && i - at < 4 {
            i += 1;
        }
        // 漢字の旦・且は文頭にも現れる。手に加えて湯気がある場合に限定する。
        if "旦且".contains(cup) && i == at {
            return None;
        }
        kind = "cup_or_food";
    } else {
        return None;
    }
    if kind != "cup_or_food"
        && chars
            .get(i)
            .is_some_and(|&c| matches!(c, '\u{fe0e}' | '\u{fe0f}'))
    {
        i += 1;
    }
    // 絵文字や結合文字列を途中まで無読化しない。未対応の結合は候補全体を残す。
    if chars.get(i).is_some_and(|&c| {
        c == '\u{200d}'
            || ('\u{0300}'..='\u{036f}').contains(&c)
            || ('\u{1ab0}'..='\u{1aff}').contains(&c)
            || ('\u{1dc0}'..='\u{1dff}').contains(&c)
            || ('\u{20d0}'..='\u{20ff}').contains(&c)
            || ('\u{fe00}'..='\u{fe0f}').contains(&c)
            || ('\u{fe20}'..='\u{fe2f}').contains(&c)
            || ('\u{1f3fb}'..='\u{1f3ff}').contains(&c)
    }) {
        return None;
    }
    let next = chars[i..].iter().copied().find(|&c| !space(c));
    if next.is_some_and(occupied) {
        return None;
    }
    if kind == "drawn_weapon" && chars.get(i).is_some_and(|c| c.is_alphanumeric()) {
        return None;
    }
    if chars.get(i).is_some_and(|&c| ".．=＝:/／".contains(c))
        && chars.get(i + 1).is_some_and(|&c| occupied(c))
    {
        return None;
    }
    Some((i, kind))
}

/// 持ち物を認識済みの区間で、手と棒だけからなる形態素か調べます。
pub fn hand_and_bar(surface: &str) -> bool {
    let mut chars = surface.chars();
    chars.next().is_some_and(hand) && chars.clone().next().is_some() && chars.all(bar)
}

fn side(text: &str, boundary: usize, left: bool) -> Option<(usize, &'static str)> {
    let mut chars = ['\0'; 40];
    let mut len = 0;
    if left {
        for c in text[..boundary].chars().rev().take(40) {
            chars[len] = c;
            len += 1;
        }
    } else {
        for c in text[boundary..].chars().take(40) {
            chars[len] = c;
            len += 1;
        }
    }
    let (count, kind) = object(&chars[..len])?;
    let bytes = chars[..count].iter().map(|c| c.len_utf8()).sum::<usize>();
    Some((
        if left {
            boundary - bytes
        } else {
            boundary + bytes
        },
        kind,
    ))
}

fn carrier(text: &str, core: &Range<usize>, base: &Range<usize>, left: bool) -> Option<usize> {
    let boundary = if left { base.start } else { base.end };
    let (scanned, arms) = if left {
        scan_carrier(text[..boundary].chars().rev())
    } else {
        scan_carrier(text[boundary..].chars())
    };
    let existing = if left {
        &text[base.start..core.start]
    } else {
        &text[core.end..base.end]
    };
    (arms > 0 || existing.chars().any(hand)).then_some(if left {
        boundary - scanned
    } else {
        boundary + scanned
    })
}

fn scan_carrier(chars: impl Iterator<Item = char>) -> (usize, usize) {
    let mut scanned = 0;
    let mut arms = 0;
    let mut spaces = 0;
    for c in chars.take(6) {
        if space(c) && arms == 0 && spaces < 2 {
            spaces += 1;
        } else if hand(c) && arms < 4 {
            arms += 1;
        } else {
            break;
        }
        scanned += c.len_utf8();
    }
    (scanned, arms)
}

/// 手に接続した持ち物の候補を返します。語と重なる候補の採否は形態素との照合で決めます。
pub fn extend(text: &str, core: &Range<usize>, base: &Range<usize>) -> Proposal {
    let mut result = Proposal {
        extent: base.clone(),
        left: None,
        right: None,
    };
    if let Some(boundary) = carrier(text, core, base, true)
        && let Some((at, kind)) = side(text, boundary, true)
    {
        result.extent.start = at;
        result.left = Some(kind);
    }
    if let Some(boundary) = carrier(text, core, base, false)
        && let Some((at, kind)) = side(text, boundary, false)
    {
        result.extent.end = at;
        result.right = Some(kind);
    }
    if result.right.is_none()
        && let Some((at, "drawn_weapon")) = side(text, base.end, false)
    {
        result.extent.end = at;
        result.right = Some("drawn_weapon");
    }
    result
}
