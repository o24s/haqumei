//! Basic usage example of Haqumei
//!
//! This example demonstrates the core functionality:
//! - Converting Japanese text to phonemes
//! - Converting to katakana reading
//! - Getting space-separated phoneme strings

use haqumei::Haqumei;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut haqumei = Haqumei::new()?;

    let text = "日本語のテキストを音素に変換します。";

    println!("Original text: {}\n", text);

    // Convert to a list of phonemes
    let phonemes = haqumei.g2p(text)?;
    println!("Phoneme list:");
    println!("{:?}\n", phonemes);

    // Convert to a space-separated string (like pyopenjtalk)
    let phoneme_str = phonemes.join(" ");
    println!("Space-separated phonemes:");
    println!("{}\n", phoneme_str);

    // Convert to Katakana reading
    let kana = haqumei.g2p_kana(text)?;
    println!("Katakana reading:");
    println!("{}\n", kana);

    // Additional example with different text
    println!("--- Another example ---\n");
    let text2 = "こんにちは、世界！";
    println!("Original text: {}", text2);

    let phonemes2 = haqumei.g2p(text2)?;
    println!("Phonemes: {}", phonemes2.join(" "));

    let kana2 = haqumei.g2p_kana(text2)?;
    println!("Katakana: {}", kana2);

    Ok(())
}
