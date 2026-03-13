use haqumei::{Haqumei, HaqumeiOptions};
use similar::{ChangeTag, TextDiff};
use std::borrow::Cow;
use std::error::Error;

#[allow(unused)]
mod basic5000;
use basic5000::*;

const IGNORE_PAU: bool = true;

#[allow(unused)]
#[derive(Debug, Clone, Copy)]
enum EditOp {
    Equal(usize, usize),      // expected_idx, actual_idx
    Substitute(usize, usize),// expected_idx, actual_idx
    Delete(usize, usize),    // expected_idx, actual_idx (actual_idx is the column index at that time)
    Insert(usize, usize),    // expected_idx, actual_idx
}

fn compute_edit_ops(expected: &[&str], actual: &[&str]) -> (usize, usize, usize, Vec<EditOp>) {
    let m = expected.len();
    let n = actual.len();

    // dp distances
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for (i, row) in dp.iter_mut().enumerate().take(m + 1) {
        row[0] = i;
    }

    for (j, cell) in dp[0].iter_mut().enumerate().take(n + 1) {
        *cell = j;
    }

    for i in 1..=m {
        for j in 1..=n {
            let cost = if expected[i - 1] == actual[j - 1] { 0 } else { 1 };
            dp[i][j] = std::cmp::min(
                std::cmp::min(dp[i - 1][j] + 1, dp[i][j - 1] + 1),
                dp[i - 1][j - 1] + cost,
            );
        }
    }

    // backtrace to get ops (reverse)
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
            // delete from expected (expected char removed)
            // note: j is current column index after this op
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

fn main() -> Result<(), Box<dyn Error>> {
    let mut haqumei = Haqumei::with_options(HaqumeiOptions {
        predict_nani: true,
        modify_kanji_yomi: true,
        ..Default::default()
    })?;

    let results = haqumei.g2p_mapping_detailed_batch(TEXTS)?;
    let mut sentence_errors = 0usize;

    // global counters for PER
    let mut total_s = 0usize;
    let mut total_d = 0usize;
    let mut total_i = 0usize;
    let mut total_phonemes = 0usize;

    for (i, sentence_details) in results.iter().enumerate() {
        let expected_raw: &[&str] = PHONEMES[i];

        // Build actual phoneme list (Cow to avoid unnecessary allocations)
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

        // Build expected filtered (borrowing from PHONEMES)
        let expected_filtered: Vec<&str> = if IGNORE_PAU {
            expected_raw.iter().copied().filter(|&p| p != "pau").collect()
        } else {
            expected_raw.to_vec()
        };

        let actual_refs: Vec<&str> = actual_phonemes.iter().map(|c| c.as_ref()).collect();

        let (s_count, d_count, i_count, ops) = compute_edit_ops(&expected_filtered, &actual_refs);

        total_s += s_count;
        total_d += d_count;
        total_i += i_count;
        total_phonemes += expected_filtered.len();

        // If exact match (no edits), skip detailed printing
        if s_count == 0 && d_count == 0 && i_count == 0 {
            continue;
        }
        sentence_errors += 1;

        let mut error_flags = vec![false; sentence_details.len()];

        // For each edit op, decide which word index(s) to mark.
        // We don't have gold per-word mapping for expected, so we map edits to nearest actual word indexes.
        // Rules (best-effort):
        //  - Insert: maps to actual_to_word_idx[actual_idx]
        //  - Substitute: map to actual_to_word_idx[actual_idx]
        //  - Delete: expected present but missing in actual. Use actual_idx (column index at that time).
        for op in &ops {
            match *op {
                EditOp::Equal(_, _) => {}
                EditOp::Substitute(_exp_idx, actual_idx) | EditOp::Insert(_exp_idx, actual_idx) => {
                    if actual_idx < actual_to_word_idx.len() {
                        error_flags[actual_to_word_idx[actual_idx]] = true;
                    } else if !actual_to_word_idx.is_empty() {
                        // fallback to last
                        let last = actual_to_word_idx.len() - 1;
                        error_flags[actual_to_word_idx[last]] = true;
                    } else {
                        // no actual phonemes at all: mark first word (best effort)
                        if !error_flags.is_empty() {
                            error_flags[0] = true;
                        }
                    }
                }
                EditOp::Delete(_expected_idx, actual_col_idx) => {
                    // actual_col_idx is the column (j) when delete occurred.
                    // Prefer to map to actual_to_word_idx[actual_col_idx], fallback to previous or 0.
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

        println!("==================================================");
        println!("[ID: BASIC5000_{:04}]", i + 1);
        println!("Text: {}", TEXTS[i]);
        println!("--------------------------------------------------");

        let diff = TextDiff::from_slices(&expected_filtered, &actual_refs);

        print!("Diff: ");
        for change in diff.iter_all_changes() {
            match change.tag() {
                ChangeTag::Equal => {
                    print!("{} ", change.value());
                }
                ChangeTag::Delete => {
                    print!("[-{}] ", change.value());
                }
                ChangeTag::Insert => {
                    print!("[+{}] ", change.value());
                }
            }
        }
        println!("\n");

        println!("Failed Words Analysis:");
        let mut indices: Vec<usize> = error_flags
            .iter()
            .enumerate()
            .filter_map(|(idx, &flag)| if flag { Some(idx) } else { None })
            .collect();
        indices.sort_unstable();

        for word_idx in indices {
            let Some(detail) = sentence_details.get(word_idx) else {
                println!("  - Error: word index {} out of range.", word_idx);
                continue;
            };
            let ignored_mark = if detail.is_ignored { " (Ignored/Space)" } else { "" };
            let unk_mark = if detail.is_unknown { " [UNK]" } else { "" };

            println!(
                "  - Word: 「{}」{}{}",
                detail.word.replace("\n", "\\n"),
                unk_mark,
                ignored_mark
            );
            println!("    Generated: {:?}", detail.phonemes);

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
            println!("    Context:   {} -> [ {} ] -> {}", prev_word, detail.word, next_word);
            println!();
        }

        let n_expected = expected_filtered.len();
        let per = if n_expected > 0 {
            100.0 * (s_count + d_count + i_count) as f64 / n_expected as f64
        } else {
            0.0
        };
        println!(
            "Sentence stats: S={} D={} I={}  N_expected={}  PER={:.2}%",
            s_count, d_count, i_count, n_expected, per
        );
        println!("--------------------------------------------------\n");
    }

    let total_sentences = TEXTS.len();
    let exact_match = total_sentences - sentence_errors;
    let accuracy = 100.0 * (exact_match as f64) / (total_sentences as f64);

    let overall_per = if total_phonemes > 0 {
        100.0 * (total_s + total_d + total_i) as f64 / (total_phonemes as f64)
    } else {
        0.0
    };

    println!("==================================================");
    println!("Total Sentences tested: {}", total_sentences);
    println!("Sentences with errors : {}", sentence_errors);
    println!("Ignore Pau Mode       : {}", IGNORE_PAU);
    println!("Accuracy (Exact match): {:.2}%", accuracy);
    println!(
        "Overall PER (S+D+I / N_expected): {:.2}%  (S={} D={} I={} N={})",
        overall_per, total_s, total_d, total_i, total_phonemes
    );

    Ok(())
}