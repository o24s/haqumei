use haqumei_jlabel::Label;

/// `haqumei_jlabel::Label` のスライスから、tdmelodic, ESPnet2 like なプロソディ記号付き音素列を抽出します。
///
/// 出力には通常の音素に加えて、以下の制御記号が含まれます：
///
/// | 記号 | 意味 | 出現位置 |
/// | :--- | :--- | :--- |
/// | `^` | 発話の開始 (BOS) | 文頭 |
/// | `$` | 発話の終結 (EOS) | 文末 |
/// | `?` | 疑問文の終結 (？) | 文末・文中 |
/// | `!` | 感嘆の終結 (独自拡張) | 文末・文中 |
/// | `!?` | 感嘆疑問の終結 (独自拡張) | 文末・文中 |
/// | `_` | ポーズ・読点 (、) | 文中 |
/// | `#` | アクセント句境界 | 文中 |
/// | `[` | ピッチ上昇 (句頭) | 句の開始付近 |
/// | `]` | ピッチ下降 (アクセント核) | 核モーラの直後 |
///
/// 記号 `[` および `]` は、tdmelodic 等で一般的なアクセント記法に基づいています。
/// "Prosodic Features Control by Symbols as Input of Sequence-to-Sequence Acoustic Modeling for Neural TTS"
/// (Kurihara et al., 2021) のアルゴリズムにおける `^` および `!` に相当します。
///
/// 日本語のアクセントについて: [tdmelodic 利用マニュアル/予備知識](https://tdmelodic.readthedocs.io/ja/latest/pages/introduction.html)
///
/// # Arguments
/// - `labels` - 抽出元のフルコンテキストラベル構造体のスライス
/// - `drop_unvoiced_vowels` - 無声母音 (A, E, I, O, U) を有声母音として扱うかどうか
///
/// # Examples
///
/// ```rust
/// # use haqumei::{prosody::extract_prosody_from_labels, Haqumei};
/// # use haqumei_jlabel::Label;
/// # let mut haqumei = Haqumei::new().unwrap();
/// let labels = haqumei.extract_fullcontext("青い空、広がる。").unwrap();
/// let phones = extract_prosody_from_labels(&labels, false);
/// ```
pub fn extract_prosody_from_labels(labels: &[Label], drop_unvoiced_vowels: bool) -> Vec<String> {
    let num_labels = labels.len();
    if num_labels == 0 {
        return Vec::new();
    }

    let mut phones = Vec::with_capacity(num_labels * 2);

    for (n, label) in labels.iter().enumerate() {
        let p3 = label.phoneme.c.as_deref().unwrap_or("");

        if p3 == "sil" {
            if n == 0 {
                phones.push("^".to_string());
            } else if n == num_labels - 1 {
                let (is_inter, is_excl) = label
                    .accent_phrase_prev
                    .as_ref()
                    .map(|a| (a.is_interrogative, a.is_exclamatory))
                    .unwrap_or((false, false));

                match (is_inter, is_excl) {
                    (true, true) => phones.push("!?".to_string()),
                    (true, false) => phones.push("?".to_string()),
                    (false, true) => phones.push("!".to_string()),
                    (false, false) => phones.push("$".to_string()),
                }
            }
            continue;
        } else if p3 == "pau" {
            let (is_inter, is_excl) = label
                .accent_phrase_prev
                .as_ref()
                .map(|a| (a.is_interrogative, a.is_exclamatory))
                .unwrap_or((false, false));

            match (is_inter, is_excl) {
                (true, true) => phones.push("!?".to_string()),
                (true, false) => phones.push("?".to_string()),
                (false, true) => phones.push("!".to_string()),
                (false, false) => phones.push("_".to_string()),
            }
            continue;
        }

        let p3_str = if drop_unvoiced_vowels {
            match p3 {
                "A" => "a",
                "E" => "e",
                "I" => "i",
                "O" => "o",
                "U" => "u",
                _ => p3,
            }
        } else {
            p3
        };
        phones.push(p3_str.to_string());

        let (a1, a2, a3) = match &label.mora {
            Some(m) => (
                m.relative_accent_position as i32,
                m.position_forward as i32,
                m.position_backward as i32,
            ),
            None => (-50, -50, -50),
        };
        let f1 = label
            .accent_phrase_curr
            .as_ref()
            .map(|a| a.mora_count as i32)
            .unwrap_or(-50);
        let a2_next = if n + 1 < num_labels {
            labels[n + 1]
                .mora
                .as_ref()
                .map(|m| m.position_forward as i32)
                .unwrap_or(-50)
        } else {
            -50
        };

        if a3 == 1
            && a2_next == 1
            && matches!(
                p3,
                "a" | "e" | "i" | "o" | "u" | "A" | "E" | "I" | "O" | "U" | "N" | "cl"
            )
        {
            phones.push("#".to_string());
        } else if a1 == 0 && a2_next == a2 + 1 && a2 != f1 {
            phones.push("]".to_string());
        } else if a2 == 1 && a2_next == 2 {
            phones.push("[".to_string());
        }
    }
    phones
}
