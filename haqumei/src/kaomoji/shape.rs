use std::ops::Range;

const EYES: &str = "・･ﾟ゜°•●○◕╹>＞<＜≧≦∩＾^;；'＇‘’｀´`￣-˘ಠ◉б≖⌒ゝゞ〃ﾉノ/／＼\\˙◔◠◡☉⊙꒪ↀ⩌òóŏÒÓìíˊˋ˃˂❛◍º＝=╯╰◞◟☋☌☍✹☆★へつ↑↓◜◝՞Φ¯‾ᴖ⁰Ծ✖✘l*+ーಥ눈╮╭ಲ൭ˇᵕᴗ";
const MOUTHS: &str = "ω∀▽∇Ддεз﹏ヮ◡ㅅ益皿_＿‿ーｰ-～〜~︿ᵕᴗᗜ⊖△◇○〇口ρڡڼਊة㉦⌓³₃⊱ᯅ⤚。oｏOＯㅂ‸︶⌣⁀Δ∧ᆺ▿﹃꒳Θワx0.ȏᗨへエළ╯ᴥｪェꞈˬ";
const ARMS: &str =
    "ヾヽゞゝﾉノつっ⊂⊃∩╰╯╭╮┌┐└┘┗┛┏┓」｢｣⌒✌☝☛☚٩۶งว੭ʅʃᕦᕤ＼／ιυщوΣ∑mopqdbcvεσρぅ彡卍ฅ∪լ∠_「ｼﾞ";
const ORNAMENTS: &str = " 　*＊｡。.,，､、:：♡♥❤✧✿❀๑〃＠@ｰー̀́͜ิﾞ゛゙ˇ▔▰灬˶⸝︶⌣|＃#∗❁〄☞☜̵ุ̣̥̑⁾ั∞⋈✗ㆀ†ूु∽͟͞~";

fn fold(c: char) -> char {
    if ('！'..='～').contains(&c) {
        char::from_u32(c as u32 - 0xfee0).unwrap()
    } else {
        match c {
            '−' => '-',
            '　' => ' ',
            _ => c,
        }
    }
}
fn member(set: &str, c: char) -> bool {
    set.contains(c)
}
fn mark(c: char) -> bool {
    matches!(c, 'ヾ' | 'ヽ' | 'ゞ' | 'ゝ')
}
fn arm(c: char) -> bool {
    member(ARMS, c) || member(ARMS, fold(c)) || matches!(c, '/' | '\\')
}
fn eye(c: char) -> bool {
    member(EYES, c)
}
fn letter(c: char) -> bool {
    let c = fold(c);
    c.is_ascii_alphanumeric()
        || c == '_'
        || ('\u{0370}'..='\u{052f}').contains(&c) && c.is_alphabetic()
}
fn allowed(c: char) -> bool {
    eye(c)
        || arm(c)
        || member(ORNAMENTS, c)
        || ('\u{300}'..='\u{36f}').contains(&c)
        || ('\u{1dc0}'..='\u{1dff}').contains(&c)
        || matches!(c, '🎀' | '💢' | '💦' | '\u{200b}' | 'ლ' | 'ᐢ' | 'ˇ' | 'P')
        || ('\u{fe00}'..='\u{fe0f}').contains(&c)
}

fn covering_pair(a: char, b: char) -> bool {
    matches!(
        (a, b),
        ('⊃', '⊂')
            | ('つ', '⊂')
            | ('っ', 'c')
            | ('p', 'q')
            | ('q', 'p')
            | ('∩', '∩')
            | ('ฅ', 'ฅ')
            | ('ﾉ' | 'ノ' | '/', 'ﾉ' | 'ノ' | 'ヾ' | '\\')
    )
}

fn strong_mouth(c: char) -> bool {
    "ω∀▽∇Ддεз﹏ヮᴥｪェᵕ꒳".contains(c)
}

fn covering_body(t: &[char]) -> bool {
    let mut f = ['\0'; 48];
    let mut n = 0;
    for &c in t {
        if !matches!(
            c,
            ' ' | '*' | '｡' | '´' | '`' | '｀' | '〃' | 'ᐢ' | '′' | '‵' | '๑'
        ) {
            f[n] = c;
            n += 1;
        }
    }
    if n == 2 && covering_pair(f[0], f[1]) && !f[0].is_ascii_alphabetic() {
        return true;
    }
    if n > 3
        && !f[0].is_ascii_alphabetic()
        && covering_pair(f[0], f[n - 1])
        && body(&f[1..n - 1], false)
    {
        return true;
    }
    n == 3
        && covering_pair(f[0], f[2])
        && (strong_mouth(f[1]) || !f[0].is_ascii_alphabetic() && "-_ー".contains(f[1]))
}

fn body(input: &[char], outside_arm: bool) -> bool {
    if !(2..=48).contains(&input.len()) {
        return false;
    }
    let start = input
        .iter()
        .position(|c| !c.is_whitespace())
        .unwrap_or(input.len());
    let end = input
        .iter()
        .rposition(|c| !c.is_whitespace())
        .map_or(start, |i| i + 1);
    let t = &input[start..end];
    if covering_body(t) {
        return true;
    }
    if t.iter().all(|&c| member("-ー= ", c)) {
        return false;
    }
    if t.contains(&'三') {
        return t.iter().filter(|&&c| c == '三').count() <= 2
            && t.split(|&c| c == '三').all(|s| body(s, outside_arm));
    }
    if matches!(t, [a, m, b] if member("TtOo0xXQq3",*a) && a.eq_ignore_ascii_case(b)
        && (*m == '_' || member("TtOoXxQq",*a) && member("ω﹏",*m)
            || member("TtQq",*a) && *m == '.'))
    {
        return true;
    }
    if matches!(t, ['>', '<']) || outside_arm && matches!(t, ['・' | '･', '・' | '･']) {
        return true;
    }
    {
        let mut f = ['\0'; 48];
        let mut n = 0;
        for &c in t {
            if !matches!(c, ' ' | '*' | '｡') {
                f[n] = c;
                n += 1;
            }
        }
        let features = if n >= 4 && f[0] == 'o' && f[n - 1] == 'o' {
            &f[1..n - 1]
        } else {
            &f[..n]
        };
        let mut features = features.iter().copied();
        if let (Some(a), Some(b), None) = (features.next(), features.next(), features.next())
            && a == b
            && ("^˘ˇ".contains(a) || outside_arm && "･・".contains(a))
        {
            return true;
        }
    }
    if let Some(i) = t.windows(2).position(|s| s == ['ε', ':'])
        && t[..i]
            .iter()
            .chain(&t[i + 2..])
            .all(|&c| arm(c) || c == ' ')
    {
        return true;
    }
    let sideways = t.starts_with(&[':', '3'])
        || t.starts_with(&[':', 'D'])
        || t.starts_with(&[':', '0'])
        || t.starts_with(&['ε', ':'])
        || t.starts_with(&['\'', '､', '3'])
        || t.starts_with(&['\'', '、', '3']);
    let used = if t.starts_with(&['\'', '､', '3']) || t.starts_with(&['\'', '、', '3']) {
        3
    } else {
        2
    };
    if sideways && t[used..].iter().all(|&c| arm(c) || c == ' ') {
        return true;
    }
    if t.contains(&'(') || t.contains(&')') {
        let mut flattened = ['\0'; 48];
        let mut n = 0;
        for &c in t {
            if c != '(' && c != ')' {
                flattened[n] = c;
                n += 1;
            }
        }
        return body(&flattened[..n], outside_arm);
    }
    if outside_arm && t == ['_', ' ', '_'] {
        return true;
    }
    let mut clean = ['\0'; 48];
    let mut len = 0;
    let has_ascii = input.iter().any(|c| c.is_ascii_alphanumeric());
    if has_ascii {
        if t.starts_with(&[':', '3'])
            && t[2..].iter().all(|&c| arm(c) || member("ｼﾞ ", c))
            && t[2..].iter().any(|&c| mark(c))
        {
            return true;
        }
        let hands = t
            .iter()
            .all(|&c| allowed(c) || member(MOUTHS, c) || member("opcqbdl", c));
        let o_mouth = t.contains(&'o')
            && t.iter().all(|&c| allowed(c) || c == 'o')
            && t.iter().any(|&c| member("^＾", c));
        if !hands && !o_mouth {
            return false;
        }
        let other_mouth = input.iter().any(|&c| c != 'o' && member(MOUTHS, c));
        for &c in input {
            if hands && (member("pcqbdP", c) || c == 'o' && other_mouth) {
                continue;
            }
            clean[len] = c;
            len += 1;
        }
    } else {
        clean[..input.len()].copy_from_slice(input);
        len = input.len();
    }
    let s = &clean[..len];
    if s.iter().any(|&c| member("≡Ξ三", c)) {
        return s.iter().filter(|&&c| member("≡Ξ三", c)).count() <= 2
            && s.split(|&c| member("≡Ξ三", c))
                .all(|part| body(part, outside_arm));
    }
    let mut suffix_allowed = [false; 49];
    let mut suffix_eye = [false; 49];
    suffix_allowed[len] = true;
    for i in (0..len).rev() {
        suffix_allowed[i] = allowed(s[i]) && suffix_allowed[i + 1];
        suffix_eye[i] = eye(s[i]) || suffix_eye[i + 1];
    }
    let mut prefix_allowed = true;
    let mut prefix_eye = false;
    let one_eye = outside_arm || s.iter().any(|&c| member("ヾヽゞゝﾉノつっ⊂⊃", c));
    for (i, &c) in s.iter().enumerate() {
        if member(MOUTHS, c)
            && prefix_allowed
            && suffix_allowed[i + 1]
            && (prefix_eye && suffix_eye[i + 1]
                || one_eye && member("ω∀▽∇Ддεзヮ益皿◡", c) && (prefix_eye || suffix_eye[i + 1]))
        {
            return true;
        }
        prefix_allowed &= allowed(c);
        prefix_eye |= eye(c);
    }
    outside_arm && matches!(t, ['・', '・', '。'] | ['・', '・'] | ['。', '。'])
}

fn wrapped_hands(text: &str) -> Option<usize> {
    let mut it = text.char_indices();
    let (_, opening) = it.next()?;
    if !matches!(opening, '(' | '（' | 'ʕ') {
        return None;
    }
    if opening == 'ʕ' {
        let mut t = ['\0'; 48];
        let mut n = 0;
        for (i, c) in it.take(48) {
            if c == 'ʔ' {
                return covering_body(&t[..n]).then_some(i + c.len_utf8());
            }
            if !matches!(fold(c), '(' | ')') {
                t[n] = fold(c);
                n += 1;
            }
        }
        return None;
    }
    let (_, left) = it.next()?;
    if !"ﾉノ/／".contains(left) || fold(it.next()?.1) != ')' {
        return None;
    }
    let mut middle = ['\0'; 16];
    for (n, (_, c)) in it.by_ref().take(17).enumerate() {
        if fold(c) == '(' {
            let (_, right) = it.next()?;
            let (at, close) = it.next()?;
            return (covering_pair(fold(left), fold(right))
                && fold(close) == ')'
                && (n == 1 && strong_mouth(middle[0]) || body(&middle[..n], true)))
            .then_some(at + close.len_utf8());
        }
        if n == 16 || fold(c) == ')' {
            return None;
        }
        middle[n] = fold(c);
    }
    None
}

fn social_body(t: &[char], missing_left: bool, missing_right: bool) -> bool {
    if t.is_empty() || t.len() > 20 {
        return false;
    }
    // 英数字を含む顔は、目や輪郭を省略せず単独でも成立する場合だけ接続する。
    if t.iter().any(|c| c.is_ascii_alphanumeric()) {
        return !missing_left && !missing_right && body(t, false);
    }
    if body(t, true) {
        return true;
    }
    let Some(a) = t.iter().position(|c| !" *｡".contains(*c)) else {
        return false;
    };
    let b = t.iter().rposition(|c| !" *｡".contains(*c)).unwrap() + 1;
    let s = &t[a..b];
    let one_side =
        |s: &[char]| !s.is_empty() && s.iter().all(|&c| allowed(c)) && s.iter().any(|&c| eye(c));
    missing_left && strong_mouth(s[0]) && one_side(&s[1..])
        || missing_right && strong_mouth(s[s.len() - 1]) && one_side(&s[..s.len() - 1])
}

fn overlapping_faces(text: &str) -> Option<usize> {
    if !text.starts_with(['(', '（']) {
        return None;
    }
    let mut t = ['\0'; 44];
    let mut first = None;
    let mut second_start = 0;
    for (n, (i, c)) in text.char_indices().skip(1).take(44).enumerate() {
        let c_fold = fold(c);
        if matches!(c_fold, '(' | ')') {
            if let Some(at) = first {
                if t[at] == ')' && n == at + 1 && c_fold == ')' {
                    second_start = n + 1;
                } else {
                    return (c_fold == ')'
                        && social_body(&t[..at], false, t[at] == '(')
                        && social_body(&t[second_start..n], t[at] == ')', false))
                    .then_some(i + c.len_utf8());
                }
            } else {
                // 左の顔が空なら重なった顔にはならず、残りの文字を調べる必要もない。
                if n == 0 || n > 20 {
                    return None;
                }
                first = Some(n);
                second_start = n + 1;
            }
        } else if !(allowed(c_fold) || member(MOUTHS, c_fold)) {
            return None;
        }
        t[n] = c_fold;
    }
    None
}

// 指で頬をつつく形や手をつなぐ形は、接続の両端に顔がある場合だけ認める。
fn connected_faces(text: &str) -> Option<usize> {
    fn part(text: &str) -> Option<([char; 20], usize, char, usize)> {
        let mut out = ['\0'; 20];
        for (n, (at, c)) in text.char_indices().take(21).enumerate() {
            let folded = fold(c);
            if matches!(folded, '(' | ')' | '人') {
                return (n > 0).then_some((out, n, folded, at + c.len_utf8()));
            }
            if n == 20
                || !(allowed(folded) || member(MOUTHS, folded) || member("TtOo0xXQq3", folded))
            {
                return None;
            }
            out[n] = folded;
        }
        None
    }
    let first = text.chars().next()?;
    if !matches!(first, '(' | '（') {
        return None;
    }
    let (left, n, delimiter, consumed) = part(&text[first.len_utf8()..])?;
    if !social_body(&left[..n], false, delimiter == '(') {
        return None;
    }
    let mut at = first.len_utf8() + consumed;
    let mut delimiter = delimiter;
    let mut matched = None;
    for _ in 1..8 {
        let mut tail = &text[at..];
        let mut missing_left = delimiter == ')';
        if delimiter == ')' {
            if let Some(rest) = tail.strip_prefix('人') {
                let Some(open) = rest.chars().next() else {
                    break;
                };
                if !matches!(open, '(' | '（') {
                    break;
                }
                at += '人'.len_utf8() + open.len_utf8();
                tail = &text[at..];
                missing_left = false;
            } else if tail.starts_with(['σ', 'っ']) {
                let pointer = tail.chars().next().unwrap();
                let mut used = pointer.len_utf8();
                if let Some(motion) = tail[used..].chars().next()
                    && matches!(motion, '"' | '＂' | '”')
                {
                    used += motion.len_utf8();
                }
                let Some(close) = tail[used..].chars().next() else {
                    break;
                };
                if !matches!(close, ')' | '）') {
                    break;
                }
                at += used + close.len_utf8();
                tail = &text[at..];
            } else if tail.starts_with([')', '）']) {
                at += tail.chars().next()?.len_utf8();
                tail = &text[at..];
            }
        }
        let Some((right, n, close, consumed)) = part(tail) else {
            break;
        };
        if close != ')' || !social_body(&right[..n], missing_left, false) {
            break;
        }
        at += consumed;
        matched = Some(at);
        delimiter = ')';
    }
    matched
}

fn extra(text: &str) -> Option<usize> {
    if let Some(end) = wrapped_hands(text)
        .or_else(|| connected_faces(text))
        .or_else(|| overlapping_faces(text))
    {
        return Some(end);
    }
    for form in [">ω<", "＞ω＜", "눈_눈", "ᓀ‸ᓂ", "/ᐠ｡ꞈ｡ᐟ\\"] {
        if text.starts_with(form) {
            return Some(form.len());
        }
    }
    for head in ["(:3[", "(¦3[", "('､3[", "('、3["] {
        if let Some(tail) = text.strip_prefix(head) {
            for (count, (i, c)) in tail.char_indices().take(9).enumerate() {
                if c == ']' && (2..=8).contains(&count) {
                    return Some(head.len() + i + 1);
                }
                if !matches!(c, '_' | '＿' | '▓') {
                    break;
                }
            }
        }
    }
    None
}

/// 顔の本体と、括弧外の腕を含む範囲をUTF-8のバイト位置で返します。
pub fn find_parts(text: &str, mut found: impl FnMut(Range<usize>, Range<usize>)) {
    if !text.contains([
        '(', '（', '⎛', '꒰', '₍', 'ʕ', '∪', '⎝', '|', '｜', '>', '＞', '눈', 'ᓀ', '/',
    ]) {
        return;
    }
    let mut consumed = 0;
    for (i, original) in text.char_indices() {
        if i >= consumed
            && matches!(original, '(' | '（' | 'ʕ' | '>' | '＞' | '눈' | 'ᓀ' | '/')
            && let Some(len) = extra(&text[i..])
        {
            let end = i + len;
            let mut a = i;
            let mut b = end;
            if matches!(original, '(' | '（' | 'ʕ') {
                for c in text[..i].chars().rev().take(16) {
                    if !arm(c) || matches!(c, 'ｼ' | 'ﾞ') || a - c.len_utf8() < consumed {
                        break;
                    }
                    let at = a - c.len_utf8();
                    if letter(c)
                        && (text[..at].chars().next_back().is_some_and(letter)
                            || text[a..].chars().next().is_some_and(letter))
                    {
                        break;
                    }
                    a = at;
                }
                for c in text[end..].chars().take(16) {
                    if !arm(c) {
                        break;
                    }
                    if letter(c)
                        && (text[..b].chars().next_back().is_some_and(letter)
                            || text[b + c.len_utf8()..].chars().next().is_some_and(letter))
                    {
                        break;
                    }
                    b += c.len_utf8();
                }
            }
            found(i..end, a..b);
            consumed = i + len;
            continue;
        }
        if i < consumed
            || !matches!(
                original,
                '(' | '（' | '⎛' | '꒰' | '₍' | 'ʕ' | '∪' | '⎝' | '|' | '｜'
            )
        {
            continue;
        }
        let mut a = i;
        for c in text[..i].chars().rev().take(16) {
            // 「仲良ｼ」の語尾と区別できないため、顔の前のｼ・ﾞは腕に含めない。
            if !arm(c) || matches!(c, 'ｼ' | 'ﾞ') || a - c.len_utf8() < consumed {
                break;
            }
            let at = a - c.len_utf8();
            if letter(c) {
                let prev = text[..at].chars().next_back();
                let next = text[a..].chars().next();
                if prev.is_some_and(letter) || next.is_some_and(letter) {
                    break;
                }
            }
            a -= c.len_utf8();
        }
        let mut chars = ['\0'; 48];
        let mut len = 0;
        let mut close = None;
        let mut depth = 0;
        for (offset, c) in text[i + original.len_utf8()..].char_indices().take(49) {
            let normalized = fold(c);
            if matches!(normalized, '⎛' | '꒰' | '₍' | '\n' | '\r') {
                break;
            }
            if normalized == '(' {
                depth += 1;
            }
            if matches!(normalized, ')' | '⎞' | '꒱' | '₎' | 'ʔ' | '⎠')
                || original == '∪' && normalized == '∪'
            {
                if normalized == ')' && depth > 0 {
                    depth -= 1;
                } else {
                    let matching = matches!(
                        (original, c),
                        ('(', ')')
                            | ('（', '）')
                            | ('⎛', '⎞')
                            | ('꒰', '꒱')
                            | ('₍', '₎')
                            | ('ʕ', 'ʔ')
                            | ('∪', '∪')
                            | ('⎝', '⎞' | '⎠')
                            | ('|' | '｜', ')' | '）')
                    );
                    if matching {
                        close = Some(i + original.len_utf8() + offset + c.len_utf8());
                    }
                    break;
                }
            }
            if len == 48 {
                break;
            }
            chars[len] = normalized;
            len += 1;
        }
        let Some(end) = close else {
            continue;
        };
        let mut b = end;
        let mut last = '\0';
        for c in text[end..].chars().take(16) {
            if !(arm(c) || member("ｼﾞ", c) && member("ﾉｼ", last)) {
                break;
            }
            if letter(c) {
                let prev = text[..b].chars().next_back();
                let next = text[b + c.len_utf8()..].chars().next();
                if prev.is_some_and(letter) || next.is_some_and(letter) {
                    break;
                }
            }
            b += c.len_utf8();
            last = c;
        }
        if body(
            &chars[..len],
            a != i || b != end || matches!(original, '|' | '｜'),
        ) {
            found(i..end, a..b);
            // 隣の顔も同じ腕を使う場合がある。腕の所有先はまだ決めない。
            consumed = end;
        }
    }
}

/// 辞書語と重なった腕を除いた後も、顔として成立するか確認します。
pub fn valid_core(text: &str, core: Range<usize>, has_arms: bool) -> bool {
    let mut chars = ['\0'; 48];
    let s = &text[core];
    if extra(s) == Some(s.len()) {
        return true;
    }
    let mut iter = s.chars();
    iter.next();
    iter.next_back();
    let mut len = 0;
    for c in iter {
        if len == 48 {
            return false;
        }
        chars[len] = fold(c);
        len += 1;
    }
    body(&chars[..len], has_arms || s.starts_with(['|', '｜']))
}
