#[allow(unused)]
mod data {
    include!(concat!(env!("OUT_DIR"), "/data.rs"));
}

use haqumei::{Haqumei, HaqumeiOptions};
use similar::{ChangeTag, TextDiff};
use std::borrow::Cow;
use std::error::Error;
use std::fs::File;
use std::io::{BufWriter, Write};

use data::{basic5000, rohan4600};

const IGNORE_PAU: bool = true;
const DICT_DIR: &str = "../compiled";

const BASIC_OUT: &str = "basic5000_report.txt";
const ROHAN_OUT: &str = "rohan4600_kana_report.txt";

#[allow(unused)]
#[derive(Debug, Clone, Copy)]
enum EditOp {
    Equal(usize, usize),      // expected_idx, actual_idx
    Substitute(usize, usize), // expected_idx, actual_idx
    Delete(usize, usize),     // expected_idx, actual_idx (actual_idx is the column index at that time)
    Insert(usize, usize),     // expected_idx, actual_idx
}

/// Levenshtein DP returning counts and edit ops for token sequences (tokens are &str slices)
fn compute_edit_ops(expected: &[&str], actual: &[&str]) -> (usize, usize, usize, Vec<EditOp>) {
    let m = expected.len();
    let n = actual.len();

    let mut dp = vec![vec![0usize; n + 1]; m + 1];

    for (i, row) in dp.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in dp[0].iter_mut().enumerate() {
        *cell = j;
    }

    for i in 1..=m {
        for j in 1..=n {
            let cost = if expected[i - 1] == actual[j - 1] {
                0
            } else {
                1
            };
            dp[i][j] = std::cmp::min(
                std::cmp::min(dp[i - 1][j] + 1, dp[i][j - 1] + 1),
                dp[i - 1][j - 1] + cost,
            );
        }
    }

    let mut ops_rev: Vec<EditOp> = Vec::new();
    let mut i = m;
    let mut j = n;
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && dp[i][j] == dp[i - 1][j - 1] && expected[i - 1] == actual[j - 1] {
            ops_rev.push(EditOp::Equal(i - 1, j - 1));
            i -= 1;
            j -= 1;
        } else if i > 0 && j > 0 && dp[i][j] == dp[i - 1][j - 1] + 1 {
            ops_rev.push(EditOp::Substitute(i - 1, j - 1));
            i -= 1;
            j -= 1;
        } else if i > 0 && dp[i][j] == dp[i - 1][j] + 1 {
            ops_rev.push(EditOp::Delete(i - 1, j));
            i -= 1;
        } else {
            ops_rev.push(EditOp::Insert(i, j - 1));
            j -= 1;
        }
    }

    ops_rev.reverse();

    let mut s = 0usize;
    let mut d = 0usize;
    let mut ins = 0usize;
    for op in &ops_rev {
        match op {
            EditOp::Equal(_, _) => {}
            EditOp::Substitute(_, _) => s += 1,
            EditOp::Delete(_, _) => d += 1,
            EditOp::Insert(_, _) => ins += 1,
        }
    }

    (s, d, ins, ops_rev)
}

fn write_token_diff<W: Write>(
    w: &mut W,
    expected: &[&str],
    actual: &[&str],
) -> std::io::Result<()> {
    let diff = TextDiff::from_slices(expected, actual);
    write!(w, "Diff: ")?;
    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Equal => write!(w, "{} ", change.value())?,
            ChangeTag::Delete => write!(w, "[-{}] ", change.value())?,
            ChangeTag::Insert => write!(w, "[+{}] ", change.value())?,
        }
    }
    writeln!(w)?;
    Ok(())
}

fn evaluate_phoneme_dataset(
    haqumei: &mut Haqumei,
    texts: &[&str],
    phonemes: &[&[&str]],
    out_path: &str,
) -> Result<(), Box<dyn Error>> {
    let f = File::create(out_path)?;
    let mut w = BufWriter::new(f);

    let mut sentence_errors = 0usize;
    let mut total_s = 0usize;
    let mut total_d = 0usize;
    let mut total_i = 0usize;
    let mut total_phonemes = 0usize;

    let results = haqumei.g2p_mapping_batch(texts)?;

    for (i, sentence_details) in results.iter().enumerate() {
        let expected_raw: &[&str] = phonemes.get(i).ok_or("missing phoneme gold")?;

        let mut actual_phonemes: Vec<Cow<str>> = Vec::new();
        let mut actual_to_word_idx: Vec<usize> = Vec::new();

        for (word_idx, detail) in sentence_details.iter().enumerate() {
            for p in &detail.phonemes {
                let p_str: &str = p.as_ref();
                if IGNORE_PAU && p_str == "pau" {
                    continue;
                } else if p_str == "N" {
                    actual_phonemes.push(Cow::Borrowed("N"));
                } else {
                    let needs_lower = p_str.bytes().any(|b: u8| b.is_ascii_uppercase());
                    if needs_lower {
                        actual_phonemes.push(Cow::Owned(p_str.to_ascii_lowercase()));
                    } else {
                        actual_phonemes.push(Cow::Borrowed(p_str));
                    }
                }
                actual_to_word_idx.push(word_idx);
            }
        }

        let expected_filtered: Vec<&str> = if IGNORE_PAU {
            expected_raw
                .iter()
                .copied()
                .filter(|&p| p != "pau")
                .collect()
        } else {
            expected_raw.to_vec()
        };

        let actual_refs: Vec<&str> = actual_phonemes.iter().map(|c| c.as_ref()).collect();

        let (s_count, d_count, i_count, ops) = compute_edit_ops(&expected_filtered, &actual_refs);

        total_s += s_count;
        total_d += d_count;
        total_i += i_count;
        total_phonemes += expected_filtered.len();

        if s_count == 0 && d_count == 0 && i_count == 0 {
            continue;
        }
        sentence_errors += 1;

        let mut error_flags = vec![false; sentence_details.len()];

        for op in &ops {
            match *op {
                EditOp::Equal(_, _) => {}
                EditOp::Substitute(_exp_idx, actual_idx) | EditOp::Insert(_exp_idx, actual_idx) => {
                    if actual_idx < actual_to_word_idx.len() {
                        error_flags[actual_to_word_idx[actual_idx]] = true;
                    } else if !actual_to_word_idx.is_empty() {
                        let last = actual_to_word_idx.len() - 1;
                        error_flags[actual_to_word_idx[last]] = true;
                    } else {
                        if !error_flags.is_empty() {
                            error_flags[0] = true;
                        }
                    }
                }
                EditOp::Delete(_expected_idx, actual_col_idx) => {
                    let mapped = if actual_col_idx < actual_to_word_idx.len() {
                        actual_to_word_idx[actual_col_idx]
                    } else if actual_col_idx > 0 && !actual_to_word_idx.is_empty() {
                        actual_to_word_idx[actual_col_idx - 1]
                    } else if !error_flags.is_empty() {
                        0
                    } else {
                        continue;
                    };
                    error_flags[mapped] = true;
                }
            }
        }

        // write detailed info
        writeln!(w, "==================================================")?;
        writeln!(w, "[ID: BASIC5000_{:04}]", i + 1)?;
        writeln!(w, "Text: {}", texts[i])?;
        writeln!(w, "--------------------------------------------------")?;
        write_token_diff(&mut w, &expected_filtered, &actual_refs)?;
        writeln!(w)?;

        writeln!(w, "Failed Words Analysis:")?;
        let mut indices: Vec<usize> = error_flags
            .iter()
            .enumerate()
            .filter_map(|(idx, &flag)| if flag { Some(idx) } else { None })
            .collect();
        indices.sort_unstable();

        for word_idx in indices {
            let Some(detail) = sentence_details.get(word_idx) else {
                writeln!(w, "  - Error: word index {} out of range.", word_idx)?;
                continue;
            };
            let ignored_mark = if detail.is_ignored {
                " (Ignored/Space)"
            } else {
                ""
            };
            let unk_mark = if detail.is_unknown { " [UNK]" } else { "" };

            writeln!(
                w,
                "  - Word: 「{}」{}{}",
                detail.word.replace("\n", "\\n"),
                unk_mark,
                ignored_mark
            )?;
            writeln!(w, "    Generated: {:?}", detail.phonemes)?;
            let prev_word = if word_idx > 0 {
                &sentence_details[word_idx - 1].word
            } else {
                "BOS"
            };
            let next_word = if word_idx + 1 < sentence_details.len() {
                &sentence_details[word_idx + 1].word
            } else {
                "EOS"
            };
            writeln!(
                w,
                "    Context:   {} -> [ {} ] -> {}",
                prev_word, detail.word, next_word
            )?;
            writeln!(w)?;
        }

        let n_expected = expected_filtered.len();
        let per = if n_expected > 0 {
            100.0 * (s_count + d_count + i_count) as f64 / n_expected as f64
        } else {
            0.0
        };
        writeln!(
            w,
            "Sentence stats: S={} D={} I={}  N_expected={}  PER={:.2}%",
            s_count, d_count, i_count, n_expected, per
        )?;
        writeln!(w, "--------------------------------------------------\n")?;
    }

    // compose header summary
    let total_sentences = texts.len();
    let exact_match = total_sentences - sentence_errors;
    let accuracy = 100.0 * (exact_match as f64) / (total_sentences as f64);
    let overall_per = if total_phonemes > 0 {
        100.0 * (total_s + total_d + total_i) as f64 / (total_phonemes as f64)
    } else {
        0.0
    };

    w.flush()?;
    let body = std::fs::read_to_string(out_path)?;
    let f = File::create(out_path)?;
    let mut w = BufWriter::new(f);

    writeln!(w, "BASIC5000 phoneme summary:")?;
    writeln!(w, "Total Sentences tested: {}", total_sentences)?;
    writeln!(w, "Sentences with errors : {}", sentence_errors)?;
    writeln!(w, "Ignore Pau Mode       : {}", IGNORE_PAU)?;
    writeln!(w, "Accuracy (Exact match): {:.2}%", accuracy)?;
    writeln!(
        w,
        "Overall PER (S+D+I / N_expected): {:.2}%  (S={} D={} I={} N={})",
        overall_per, total_s, total_d, total_i, total_phonemes
    )?;
    writeln!(w)?;
    write!(w, "{}", body)?;
    w.flush()?;

    Ok(())
}

fn evaluate_kana_dataset(
    haqumei: &mut Haqumei,
    texts: &[&str],
    gold_kanas: &[&str],
    out_path: &str,
) -> Result<(), Box<dyn Error>> {
    let f = File::create(out_path)?;
    let mut w = BufWriter::new(f);

    let mut sentence_errors = 0usize;
    let mut total_s = 0usize;
    let mut total_d = 0usize;
    let mut total_i = 0usize;
    let mut total_chars = 0usize;

    for (i, &text) in texts.iter().enumerate() {
        let gold = gold_kanas.get(i).ok_or("gold kana missing")?;

        let pred_per_word = haqumei.g2p_kana_per_word(text)?;
        let mut pred = String::new();
        for part in pred_per_word {
            pred.push_str(&part);
        }

        let expected_chars: Vec<String> = gold.chars().map(|c| c.to_string()).collect();
        let expected_refs: Vec<&str> = expected_chars.iter().map(|s| s.as_str()).collect();

        let actual_chars: Vec<String> = pred.chars().map(|c| c.to_string()).collect();
        let actual_refs: Vec<&str> = actual_chars.iter().map(|s| s.as_str()).collect();

        let (s_count, d_count, i_count, _ops) = compute_edit_ops(&expected_refs, &actual_refs);

        total_s += s_count;
        total_d += d_count;
        total_i += i_count;
        total_chars += expected_refs.len();

        if s_count == 0 && d_count == 0 && i_count == 0 {
            continue;
        } else {
            sentence_errors += 1;
        }

        writeln!(w, "==================================================")?;
        writeln!(w, "[ID: ROHAN4600_{:04}]", i + 1)?;
        writeln!(w, "Text    : {}", text)?;
        writeln!(w, "Gold    : {}", gold)?;
        writeln!(w, "Pred    : {}", pred)?;
        write_token_diff(&mut w, &expected_refs, &actual_refs)?;
        writeln!(
            w,
            "Sentence stats: S={} D={} I={}  N_expected={}  PER={:.2}%",
            s_count,
            d_count,
            i_count,
            expected_refs.len(),
            if !expected_refs.is_empty() {
                100.0 * (s_count + d_count + i_count) as f64 / expected_refs.len() as f64
            } else {
                0.0
            }
        )?;
        writeln!(w)?;
    }

    // summary header
    let total_sentences = texts.len();
    let exact_match = total_sentences - sentence_errors;
    let accuracy = 100.0 * (exact_match as f64) / (total_sentences as f64);
    let overall_per = if total_chars > 0 {
        100.0 * (total_s + total_d + total_i) as f64 / (total_chars as f64)
    } else {
        0.0
    };

    w.flush()?;
    let body = std::fs::read_to_string(out_path)?;
    let f = File::create(out_path)?;
    let mut w = BufWriter::new(f);

    writeln!(w, "ROHAN4600 kana summary:")?;
    writeln!(w, "Total Sentences tested: {}", total_sentences)?;
    writeln!(w, "Sentences with errors : {}", sentence_errors)?;
    writeln!(w, "Exact-match accuracy  : {:.2}%", accuracy)?;
    writeln!(
        w,
        "Overall KANA PER (S+D+I / N_chars): {:.2}%  (S={} D={} I={} N={})",
        overall_per, total_s, total_d, total_i, total_chars
    )?;
    writeln!(w)?;
    write!(w, "{}", body)?;
    w.flush()?;

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut haqumei = Haqumei::from_path(
        DICT_DIR,
        HaqumeiOptions {
            predict_nani: true,
            use_unidic_yomi: true,
            ..Default::default()
        },
    )?;

    evaluate_phoneme_dataset(
        &mut haqumei,
        basic5000::TEXTS,
        basic5000::PHONEMES,
        BASIC_OUT,
    )?;

    let mut haqumei = Haqumei::from_path(
        DICT_DIR,
        HaqumeiOptions {
            predict_nani: true,
            revert_long_vowels: true,
            revert_yotsugana: true,
            ..Default::default()
        },
    )?;

    evaluate_kana_dataset(&mut haqumei, rohan4600::TEXTS, rohan4600::KANAS, ROHAN_OUT)?;

    Ok(())
}
