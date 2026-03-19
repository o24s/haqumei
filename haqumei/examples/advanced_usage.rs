//! Advanced features example of Haqumei
//!
//! This example demonstrates:
//! - Getting phoneme mapping with original words
//! - Detailed G2P output (detecting unknown words and spaces)
//! - Per-word phoneme conversion

use haqumei::Haqumei;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut haqumei = Haqumei::new()?;

    println!("1. Phoneme Mapping (g2p_pairs)");
    println!("   Maps phonemes back to their original words\n");

    let text1 = "𰻞𰻞麺＆お冷を頼んだ";
    println!("Text: {}", text1);

    let mapping = haqumei.g2p_pairs(text1)?;
    println!("Mapping result:");
    for word_map in &mapping {
        println!(
            "  Word: {:?} -> Phonemes: {:?}",
            word_map.word, word_map.phonemes
        );
    }
    println!();

    println!("2. Detailed G2P (g2p_detailed)");
    println!("   Detects unknown words (unk) and spaces (sp)\n");

    let text2 = "こんにちは 𰻞𰻞麺";
    println!("Text: {}", text2);

    let detailed_phonemes = haqumei.g2p_detailed(text2)?;
    println!("Detailed phonemes: {:?}", detailed_phonemes);
    println!("Note: 'sp' = space, 'unk' = unknown word\n");

    println!("3. Mapping (g2p_mapping)");
    println!("   Shows unknown status and ignore flags\n");

    let text3 = "𰻞𰻞麺　お冷を頼んだ";
    println!("Text: {}", text3);

    let detailed_mapping = haqumei.g2p_mapping(text3)?;
    println!("Mapping:");
    for detail in &detailed_mapping {
        println!("  Word: {:?}", detail.word);
        println!("    Phonemes: {:?}", detail.phonemes);
        println!(
            "    Unknown: {}, Ignored: {}",
            detail.is_unknown, detail.is_ignored
        );
    }
    println!();

    println!("4. Per-word Phoneme Conversion (g2p_per_word)");
    println!("   Splits phonemes by word boundaries\n");

    let text4 = "東京タワーに行きました";
    println!("Text: {}", text4);

    let per_word = haqumei.g2p_per_word(text4)?;
    println!("Per-word phonemes:");
    for (i, phonemes) in per_word.iter().enumerate() {
        println!("  Word {}: {:?}", i + 1, phonemes);
    }
    println!();

    println!("5. Comparison: Normal vs Detailed");
    let text5 = "テスト 𰻞𰻞 です";
    println!("Text: {}\n", text5);

    let normal = haqumei.g2p(text5)?;
    let detailed = haqumei.g2p_detailed(text5)?;

    println!("Normal g2p:   {:?}", normal);
    println!("Detailed g2p: {:?}", detailed);
    println!("\nDetailed output preserves space (sp) and unknown word (unk) information");

    Ok(())
}
