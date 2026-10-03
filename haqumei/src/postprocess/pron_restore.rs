/// 読みと対応する長音・四つ仮名だけを戻し、他の発音の違いと無声化記号を保つ。
pub(super) fn restore(read: &str, pron: &str, long_vowels: bool, yotsugana: bool) -> String {
    let read: Vec<_> = read.chars().collect();
    let mut chars: Vec<_> = pron.chars().filter(|&c| c != '’').collect();
    let blocks = if read.len() == chars.len() {
        vec![(0..read.len(), 0..chars.len())]
    } else {
        replacement_blocks(&read, &chars)
    };
    for (r, p) in blocks {
        for (&reading, pronunciation) in read[r].iter().zip(&mut chars[p]) {
            if (long_vowels
                && *pronunciation == 'ー'
                && matches!(
                    reading,
                    'ア' | 'イ' | 'ウ' | 'エ' | 'オ' | 'ァ' | 'ィ' | 'ゥ' | 'ェ' | 'ォ'
                ))
                || (yotsugana && matches!((reading, *pronunciation), ('ヅ', 'ズ') | ('ヂ', 'ジ')))
            {
                *pronunciation = reading;
            }
        }
    }
    let mut restored = chars.into_iter();
    pron.chars()
        .map(|c| {
            if c == '’' {
                c
            } else {
                restored.next().unwrap()
            }
        })
        .collect()
}

/// 一致する文字列を境に分け、文字数の等しい不一致区間だけを返す。
fn replacement_blocks(
    read: &[char],
    pron: &[char],
) -> Vec<(std::ops::Range<usize>, std::ops::Range<usize>)> {
    let mut pending = vec![(0..read.len(), 0..pron.len())];
    let mut blocks = Vec::new();
    while let Some((r, p)) = pending.pop() {
        if r.is_empty() || p.is_empty() {
            continue;
        }
        // 「アトリウム／アトリューム」のように文字数が違う場合は、長音の位置を
        // 推測で対応させない。最長の一致区間を先に確定してから前後を調べる。
        let mut lengths = vec![0; p.len() + 1];
        let (mut read_end, mut pron_end, mut longest) = (r.start, p.start, 0);
        for i in r.clone() {
            let mut diagonal = 0;
            for (j, k) in p.clone().enumerate() {
                let previous = lengths[j + 1];
                lengths[j + 1] = if read[i] == pron[k] { diagonal + 1 } else { 0 };
                diagonal = previous;
                if lengths[j + 1] > longest {
                    longest = lengths[j + 1];
                    read_end = i + 1;
                    pron_end = k + 1;
                }
            }
        }
        if longest == 0 {
            if r.len() == p.len() {
                blocks.push((r, p));
            }
        } else {
            pending.push((r.start..read_end - longest, p.start..pron_end - longest));
            pending.push((read_end..r.end, pron_end..p.end));
        }
    }
    blocks
}

#[cfg(test)]
mod tests {
    use super::restore;

    #[test]
    fn preserves_unrequested_pronunciation_changes() {
        for (read, pron, long, yotsu, expected) in [
            ("ホントウハ", "ホントーワ", true, false, "ホントウワ"),
            ("ヒャッヒョウ", "ヒャッピョー", true, false, "ヒャッピョウ"),
            ("キヅカズ", "キズカズ", false, true, "キヅカズ"),
            ("ヂゾウ", "ジゾー", false, true, "ヂゾー"),
            ("ヂゾウ", "ジゾー", true, false, "ジゾウ"),
            ("ヂゾウ", "ジゾー", true, true, "ヂゾウ"),
            ("シュウチ", "シューチ’", true, false, "シュウチ’"),
            ("アトリウム", "アトリューム", true, true, "アトリューム"),
            (
                "アトリウムヂゾウ",
                "アトリュームジゾー",
                true,
                true,
                "アトリュームヂゾウ",
            ),
            ("", "ー", true, true, "ー"),
        ] {
            assert_eq!(restore(read, pron, long, yotsu), expected, "{read}/{pron}");
        }
    }
}
