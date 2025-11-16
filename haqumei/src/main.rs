use haqumei::{Haqumei, open_jtalk::OpenJTalk};
// use vibrato_rkyv::{Dictionary, Tokenizer, dictionary::PresetDictionaryKind, token::TokenBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let text = "棚に上げた戸棚棚の上の棚を棚にする。";
    let mut haqumei = Haqumei::new()?;
    let mut openj_talk = OpenJTalk::new()?;

    let kana = haqumei.g2p(text, true)?;
    let phonemes = haqumei.g2p(text, false)?;
    let open_kana = openj_talk.g2p(text, true)?;

    dbg!(&phonemes);
    dbg!(&kana);
    dbg!(&open_kana);

    assert_ne!(kana, open_kana);

    Ok(())
}

// fn tokenize() {
    // let text = "昨日";

    // let dict = Dictionary::from_preset_with_download(PresetDictionaryKind::BccwjUnidic, "./target/dict")?;

    // let tokenizer = Tokenizer::new(dict);

    // let mut worker = tokenizer.new_worker();

    // worker.reset_sentence(text);

    // worker.tokenize_nbest(10);

    // for i in 0..worker.num_nbest_paths() {
    //     println!("Path {}:", i + 1);
    //     let cost = worker.path_cost(i).unwrap();
    //     println!("  cost {}:", cost);

    //     worker
    //         .nbest_token_iter(i)
    //         .unwrap()
    //         .for_each(|t| {
    //             println!("{}: {}", t.surface(), t.feature());
    //         });
    // }

    // return Ok(());
// }

// fn g2p() {
    // let text_body = fs::read_to_string("./data/waganeko.txt")
    //     .expect("ファイルが見つかりません。");

    // let texts: Vec<String> = text_body
    //     .split('。')
    //     .map(str::trim)
    //     .filter(|s| !s.is_empty())
    //     .map(|s| format!("{}。", s))
    //     .collect();

    // println!("Data size: {} sentences", texts.len());
    // let num_threads = rayon::current_num_threads();
    // println!("Starting multi-thread benchmark with {} threads...", num_threads);

    // let start = Instant::now();

    // let results: Vec<String> = texts
    //     .par_iter()
    //     .map_init(
    //         || OpenJTalk::new().unwrap(),
    //         |open_jtalk, text| open_jtalk.g2p(text.as_str(), false).unwrap(),
    //     )
    //     .collect();

    // let duration = start.elapsed();
    // println!("Rust OpenJTalk (Multi-thread): {:.4?}", duration);
    // drop(results);
    // compile();
// }


fn _clean() {
    use std::fs;

    for entry in fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../dictionary")).unwrap() {
        let path = entry.unwrap().path();

        if path.is_file()
            && let Some(ext) = path.extension().and_then(|s| s.to_str())
            && (ext == "dic" || ext == "bin") {
                fs::remove_file(&path).unwrap();
            }
    }
}
