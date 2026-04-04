use std::{
    env,
    error::Error,
    fs::{self, File},
    io::{self, Read, Seek, SeekFrom},
    path::Path,
};

use digest_io::IoWrapper;
use sha2::{Digest, Sha256};

fn main() -> Result<(), Box<dyn Error>> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("Failed to get MANIFEST_DIR");
    let manifest_dir = Path::new(&manifest_dir);
    let out_dir = env::var("OUT_DIR")?;
    let out_dir = Path::new(&out_dir);

    let jsut_label_path = manifest_dir.join("basic5000.yaml");
    let checksum = "1e5bf401006c434b7027f9bfe19187530d4567d866c66ee5868626f922a1b724";
    let mut need_download = true;

    if jsut_label_path.exists() {
        let mut hasher = IoWrapper(Sha256::new());
        let mut file = File::open(&jsut_label_path)?;
        io::copy(&mut file, &mut hasher)?;
        if hex::encode(hasher.0.finalize()) == checksum {
            need_download = false;
        }
    }

    let file = if need_download {
        let url = "https://raw.githubusercontent.com/prj-beatrice/jsut-label/refs/heads/master/text_kana/basic5000.yaml";
        let mut response = reqwest::blocking::get(url)?;
        if !response.status().is_success() {
            panic!("Failed to download the dictionary from {}", url);
        }

        let mut temp_file = tempfile::NamedTempFile::new_in(manifest_dir)?;
        response.copy_to(&mut temp_file)?;

        temp_file.seek(SeekFrom::Start(0))?;
        let calculated_hash = {
            let mut hasher = IoWrapper(Sha256::new());
            io::copy(&mut temp_file, &mut hasher)?;
            hex::encode(hasher.0.finalize())
        };

        if calculated_hash != checksum {
            panic!("Downloaded file checksum mismatch. It may be corrupted.")
        }

        Some(temp_file.persist(manifest_dir.join("basic5000.yaml"))?)
    } else {
        None
    };

    let data_path = out_dir.join("data.rs");

    // if data_path.exists() {
    //     return Ok(());
    // }

    let mut file = file.unwrap_or(File::open(jsut_label_path)?);

    let mut basic5000 = String::new();
    file.read_to_string(&mut basic5000)?;

    let mut texts = String::new();
    let mut kanas = String::new();
    let mut phonemes = String::new();

    for line in basic5000.lines() {
        let line = line.trim();

        if line.starts_with("text_level2") {
            let s = line.strip_prefix("text_level2:").unwrap().trim_start();

            texts.push_str("    \"");
            texts.push_str(s);
            texts.push_str("\",\n");
        } else if line.starts_with("kana_level2") {
            let s = line.strip_prefix("kana_level2:").unwrap().trim_start();

            kanas.push_str("    \"");
            kanas.push_str(s);
            kanas.push_str("\",\n");
        } else if line.starts_with("phone_level3") {
            let s = line.strip_prefix("phone_level3:").unwrap().trim_start();

            phonemes.push_str("    &[");

            for (i, p) in s.split('-').enumerate() {
                if i > 0 {
                    phonemes.push_str(", ");
                }
                phonemes.push('"');
                phonemes.push_str(p);
                phonemes.push('"');
            }

            phonemes.push_str("],\n");
        }
    }

    let mut data = String::new();

    data.push_str("pub mod basic5000 {\n");
    data.push_str("pub const TEXTS: &[&str] = &[\n");
    data.push_str(&texts);
    data.push_str("];\n\n");
    data.push_str("pub const KANAS: &[&str] = &[\n");
    data.push_str(&kanas);
    data.push_str("];\n\n");
    data.push_str("pub const PHONEMES: &[&[&str]] = &[\n");
    data.push_str(&phonemes);
    data.push_str("];\n");
    data.push_str("}\n\n");

    texts.clear();
    kanas.clear();

    let rohan_data_path = manifest_dir
        .join("../resources")
        .join("Rohan4600_transcript_utf8.txt");

    let rohan_data = fs::read_to_string(rohan_data_path)?;

    for line in rohan_data.lines() {
        let Some((_, pair)) = line.split_once(':') else {
            continue;
        };
        let Some((text, kana)) = pair.split_once(',') else {
            continue;
        };

        let mut s = String::new();
        let mut in_paren = false;
        for ch in text.chars() {
            match ch {
                '(' => in_paren = true,
                ')' => in_paren = false,
                _ if !in_paren => s.push(ch),
                _ => {}
            }
        }

        texts.push_str("    \"");
        texts.push_str(&s);
        texts.push_str("\",\n");

        kanas.push_str("    \"");
        kanas.push_str(kana);
        kanas.push_str("\",\n");
    }

    data.push_str("pub mod rohan4600 {\n");
    data.push_str("pub const TEXTS: &[&str] = &[\n");
    data.push_str(&texts);
    data.push_str("];\n\n");
    data.push_str("pub const KANAS: &[&str] = &[\n");
    data.push_str(&kanas);
    data.push_str("];\n");
    data.push_str("}\n");

    fs::write(data_path, data)?;

    Ok(())
}
