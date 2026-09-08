#[cfg(feature = "naist-jdic")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use haqumei_jpreprocess_dictionary::mecab_compile::{build_system, BuildOptions};
    use std::{fs, path::PathBuf};

    #[derive(serde::Deserialize)]
    struct Source {
        url: String,
        digest: String,
    }
    #[derive(serde::Deserialize)]
    struct Config {
        src: Source,
    }

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=build.json");
    let out = PathBuf::from(std::env::var_os("OUT_DIR").ok_or("OUT_DIR is not set")?);
    let dictionary = out.join("naist-jdic");
    println!(
        "cargo:rustc-env=JPREPROCESS_WORKDIR={}",
        dictionary.display()
    );
    fs::create_dir_all(&dictionary)?;

    if std::env::var_os("DOCS_RS").is_some() {
        for name in ["sys.dic", "unk.dic", "char.bin", "matrix.bin"] {
            fs::write(dictionary.join(name), b"{}")?;
        }
        return Ok(());
    }

    let config: Config = serde_json::from_str(include_str!("build.json"))?;
    let archive_path = out.join("source.tar.gz");
    let valid = |bytes: &[u8]| format!("{:x}", md5::compute(bytes)) == config.src.digest;
    let bytes = match fs::read(&archive_path) {
        Ok(bytes) if valid(&bytes) => bytes,
        _ => {
            let bytes = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .user_agent(concat!(
                    "haqumei-jpreprocess-naist-jdic/",
                    env!("CARGO_PKG_VERSION")
                ))
                .build()?
                .get(&config.src.url)
                .send()?
                .error_for_status()?
                .bytes()?
                .to_vec();
            if !valid(&bytes) {
                return Err("source dictionary checksum mismatch".into());
            }
            fs::write(&archive_path, &bytes)?;
            bytes
        }
    };
    let source = out.join("source");
    if source.exists() {
        fs::remove_dir_all(&source)?;
    }
    fs::create_dir_all(&source)?;
    tar::Archive::new(flate2::read::GzDecoder::new(bytes.as_slice())).unpack(&source)?;
    let root = fs::read_dir(&source)?
        .next()
        .ok_or("source dictionary is empty")??
        .path();
    // 保存形式はビルダーの依存クレートでも変わるため、アーカイブだけ再利用して辞書は再構築する。
    build_system(&root, &dictionary, &BuildOptions::default())?;
    Ok(())
}

#[cfg(not(feature = "naist-jdic"))]
fn main() {}
