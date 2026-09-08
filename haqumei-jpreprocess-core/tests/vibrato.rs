#![cfg(feature = "vibrato")]

use haqumei_jpreprocess_core::token::{Token, Tokenizer};

#[test]
fn archived_dictionary_preserves_morphology_and_pronunciation() {
    let csv = "試験,0,0,0,名詞,サ変接続,*,*,*,*,試験,シケン,シケン,1/3,C2,-1\nです,0,0,0,助動詞,*,*,*,特殊・デス,基本形,です,デス,デス,1/2,名詞%F2@0,-1\n";
    let dictionary = vibrato::SystemDictionaryBuilder::from_readers(
        csv.as_bytes(),
        &b"1 1\n0 0 0\n"[..],
        &b"DEFAULT 0 1 0\n"[..],
        "DEFAULT,0,0,10000,名詞,一般,*,*,*,*,*,*,*,0/0,*,*\n".as_bytes(),
    )
    .unwrap();
    let mut bytes = Vec::new();
    dictionary.write(&mut bytes).unwrap();
    let tokenizer = vibrato::Tokenizer::new(vibrato::Dictionary::from_bytes(&bytes).unwrap());
    let mut tokens = Tokenizer::tokenize(&tokenizer, "試験です").unwrap();
    assert_eq!(tokens.len(), 2);
    for (token, (surface, pron)) in tokens
        .iter_mut()
        .zip([("試験", "シケン"), ("です", "デス")])
    {
        let (actual_surface, entry) = token.fetch().unwrap();
        assert_eq!(actual_surface, surface);
        assert_eq!(entry.to_str_vec(surface.into())[5], pron);
    }
}
