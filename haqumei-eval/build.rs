use std::{
    env,
    error::Error,
    fs::{self, File},
    io::{self, Read, Seek, SeekFrom},
    path::Path,
};

use sha2::{Digest, Sha256};

fn main() -> Result<(), Box<dyn Error>> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("Failed to get MANIFEST_DIR");
    let manifest_dir = Path::new(&manifest_dir);

    let jsut_label_path = manifest_dir.join("basic5000.yaml");
    let checksum = "1e5bf401006c434b7027f9bfe19187530d4567d866c66ee5868626f922a1b724";
    let mut need_download = true;

    if jsut_label_path.exists() {
        let mut hasher = Sha256::new();
        let mut file = File::open(&jsut_label_path)?;
        io::copy(&mut file, &mut hasher)?;
        if hex::encode(hasher.finalize()) == checksum {
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
            let mut hasher = Sha256::new();
            io::copy(&mut temp_file, &mut hasher)?;
            hex::encode(hasher.finalize())
        };

        if calculated_hash != checksum {
            panic!("Downloaded file checksum mismatch. It may be corrupted.")
        }

        Some(temp_file.persist(manifest_dir.join("basic5000.yaml"))?)
    } else {
        None
    };

    let basic5000_rs_path = manifest_dir.join("src/basic5000.rs");

    if basic5000_rs_path.exists() {
        return Ok(());
    }

    let mut file = file.unwrap_or(File::open(jsut_label_path)?);

    let mut basic5000 = String::new();
    file.read_to_string(&mut basic5000)?;

    let mut texts = String::new();
    let mut kanas = String::new();
    let mut phonemes = String::new();

    for line in basic5000.lines() {
        let line = line.trim();

        if line.starts_with("text_level2") {
            texts.push_str(
                &("    \"".to_string()
                    + line.strip_prefix("text_level2:").unwrap().trim_start()
                    + "\",\n"),
            );
        } else if line.starts_with("kana_level2") {
            kanas.push_str(
                &("    \"".to_string()
                    + line.strip_prefix("kana_level2:").unwrap().trim_start()
                    + "\",\n"),
            );
        } else if line.starts_with("phone_level3") {
            phonemes.push_str(
                &("    &[\"".to_string()
                    + line
                        .strip_prefix("phone_level3:")
                        .unwrap()
                        .split('-')
                        .collect::<Vec<&str>>()
                        .join("\", \"")
                        .trim_start()
                    + "\"],\n"),
            );
        }
    }

    let mut basic5000_rs = String::new();

    basic5000_rs.push_str("pub const TEXTS: &[&str] = &[\n");
    basic5000_rs.push_str(&texts);
    basic5000_rs.push_str("];\n\n");
    basic5000_rs.push_str("pub const KANAS: &[&str] = &[\n");
    basic5000_rs.push_str(&kanas);
    basic5000_rs.push_str("];\n\n");
    basic5000_rs.push_str("pub const PHONEMES: &[&[&str]] = &[\n");
    basic5000_rs.push_str(&phonemes);
    basic5000_rs.push_str("];\n\n");

    fs::write(basic5000_rs_path, basic5000_rs)?;

    Ok(())
}
