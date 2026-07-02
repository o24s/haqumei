use haqumei::Haqumei;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut haqumei = Haqumei::new()?;

    let text = "こんにちは、世界！";

    // 音素リストに変換
    let phonemes = haqumei.g2p(text)?;
    println!("G2P: {:?}", phonemes);
    assert_eq!(
        phonemes,
        [
            "k", "o", "N", "n", "i", "ch", "i", "w", "a", "pau", "s", "e", "k", "a", "i"
        ]
    );

    // プロソディ記号付きの音素リストを得る
    let phones = haqumei.g2p_prosody(text)?.join(" ");
    println!("Prosody: {}", phones);
    assert_eq!(phones, "^ k o [ N n i ch i w a _ s e ] k a i ! $");

    // カタカナ読みに変換
    let kana = haqumei.g2k(text)?;
    println!("Katakana: {}", kana);
    assert_eq!(kana, "コンニチワ、セカイ！");

    // 異音解決を有効にする
    haqumei.options.use_allophones = true;

    let text = "執筆";

    // プロソディ情報付きの Word-Phoneme 対応を得る
    let mapping = haqumei.g2p_mapping_prosody(text)?;
    let shippitsu = &mapping[0];
    assert_eq!(shippitsu.word, "執筆");
    assert_eq!(shippitsu.pos, "名詞");
    assert_eq!(shippitsu.accent_nucleus, 0); // 平板型

    println!("Mapping (執筆): {:?}", shippitsu.phonemes);
    // [Phoneme {
    //     phoneme: Sh,
    //     pitch: Some(Low)
    // },
    // Phoneme {
    //     phoneme: I,
    //     pitch: Some(Low)
    // },
    // Phoneme {
    //     phoneme: ClP,
    //     pitch: Some(High)
    // },
    // Phoneme {
    //     phoneme: P,
    //     pitch: Some(High)
    // },
    // Phoneme {
    //     phoneme: UnvoicedI,
    //     pitch: Some(High)
    // }, ...]

    Ok(())
}
