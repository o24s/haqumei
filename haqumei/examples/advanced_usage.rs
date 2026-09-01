//! `basic_usage` が扱う `g2p` / `g2p_prosody` / `g2k` の先にある API を並べる。
//!
//! - 語と音素の対応 (`g2p_mapping`)
//! - 未知語と空白の見え方 (`g2p` と `g2p_detailed` の違い)
//! - 語ごとの音素 (`g2p_per_word`)
//! - 品詞とアクセント (`g2p_mapping_detailed`)
//! - 読みの候補 (`g2p_candidates`)

use haqumei::Haqumei;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut haqumei = Haqumei::new()?;

    println!("1. Word-phoneme mapping (g2p_mapping)");
    println!("   Each word carries where it came from and how it was analysed\n");

    // 全角空白を入れて、`is_ignored` の語が出るようにする
    let text = "𰻞𰻞麺　お冷を頼んだ";
    println!("Text: {text}");
    for word in haqumei.g2p_mapping(text)? {
        let status = match (word.is_unknown, word.is_ignored) {
            (true, _) => " (unknown)",
            (_, true) => " (ignored)",
            _ => "",
        };
        println!(
            "  {:?} {:?} -> {:?}{}",
            word.word, word.char_span, word.phonemes, status
        );
    }
    println!();

    println!("2. g2p vs g2p_detailed");
    println!("   g2p follows Open JTalk and drops spaces and unknown words\n");

    let text = "テスト 𰻞𰻞 です";
    println!("Text: {text}");
    println!("  g2p:          {:?}", haqumei.g2p(text)?);
    println!("  g2p_detailed: {:?}", haqumei.g2p_detailed(text)?);
    println!();

    println!("3. Phonemes grouped by word (g2p_per_word)\n");

    let text = "東京タワーに行きました";
    println!("Text: {text}");
    for (i, phonemes) in haqumei.g2p_per_word(text)?.iter().enumerate() {
        println!("  {}: {:?}", i + 1, phonemes);
    }
    println!();

    println!("4. Part of speech and accent (g2p_mapping_detailed)\n");

    let text = "薄明の空を見上げた";
    println!("Text: {text}");
    for word in haqumei.g2p_mapping_detailed(text)? {
        println!(
            "  {:?} {}-{} read={} pron={} accent={}/{}",
            word.word,
            word.pos,
            word.pos_group1,
            word.read,
            word.pron,
            word.accent_nucleus,
            word.mora_count,
        );
    }
    println!();

    println!("5. Reading candidates (g2p_candidates)");
    println!("   Leaves the branch open instead of committing to one reading\n");

    let text = "彼の話を聞いた。";
    println!("Text: {text}");
    let got = haqumei.g2p_candidates(text)?;
    for branch in &got.branches {
        let readings: Vec<String> = branch
            .alternatives
            .iter()
            .map(|a| {
                let unit = if a.nodes.len() == 1 { "node" } else { "nodes" };
                format!("{} ({} {unit})", a.pron(), a.nodes.len())
            })
            .collect();
        println!(
            "  branch {:?} {:?}: {}",
            branch.char_span,
            branch.surface,
            readings.join(" / ")
        );
    }
    for candidate in &got.candidates {
        let phonemes: Vec<&str> = candidate
            .words
            .iter()
            .flat_map(|w| w.phonemes.iter())
            .map(|p| p.as_str())
            .collect();
        println!("  delta={:<5} {}", candidate.delta, phonemes.join(" "));
    }

    Ok(())
}
