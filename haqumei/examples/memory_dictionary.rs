//! 同じ辞書のファイル入力とメモリ入力を比較し、G2P の結果を JSON で返す。

use std::{fs, path::PathBuf};

use haqumei::{Haqumei, HaqumeiOptions, open_jtalk::Dictionary};
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let directory = PathBuf::from(args.next().ok_or("辞書ディレクトリを指定してください")?);
    let cases = PathBuf::from(args.next().ok_or("入力 JSON を指定してください")?);
    let users = args.map(PathBuf::from).collect::<Vec<_>>();
    let system = fs::read(directory.join("system.bin"))?;
    let characters = fs::read(directory.join("char.bin"))?;
    let matrix = fs::read(directory.join("matrix.bin"))?;
    let user_bytes = users.iter().map(fs::read).collect::<Result<Vec<_>, _>>()?;
    let slices = user_bytes.iter().map(Vec::as_slice).collect::<Vec<_>>();
    let dictionary = Dictionary::from_bytes(&system, &characters, &matrix, &slices)?;
    let options = HaqumeiOptions {
        predict_nani: false,
        predict_kana_english: false,
        ..Default::default()
    };
    let mut memory = Haqumei::from_dictionary(dictionary, options)?;
    let mut files = Haqumei::from_paths(&directory, &users, options)?;
    let cases: Vec<String> = serde_json::from_slice(&fs::read(cases)?)?;
    let mut results = Vec::new();
    for text in cases {
        let actual = json!({
            "text": text,
            "kana": memory.g2k(&text)?,
            "phonemes": memory.g2p(&text)?.iter().map(ToString::to_string).collect::<Vec<_>>(),
            "prosody": memory.g2p_prosody(&text)?,
            "mapping": memory.g2p_mapping_detailed(&text)?,
            "labels": memory.extract_fullcontext_string(&text)?,
        });
        assert_eq!(actual["kana"], files.g2k(&text)?, "{text:?}");
        assert_eq!(
            actual["mapping"],
            json!(files.g2p_mapping_detailed(&text)?),
            "{text:?}"
        );
        assert_eq!(
            actual["labels"],
            json!(files.extract_fullcontext_string(&text)?),
            "{text:?}"
        );
        results.push(actual);
    }
    println!("{}", serde_json::to_string(&results)?);
    Ok(())
}
