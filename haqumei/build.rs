#[cfg(feature = "download-dictionary")]
use digest_io::IoWrapper;
use sha2::{Digest, Sha256};
#[cfg(feature = "download-dictionary")]
use std::io::{self, Seek, SeekFrom};
#[cfg(feature = "download-dictionary")]
use std::sync::LazyLock;
use std::{
    env,
    error::Error,
    fs::{self, File},
    path::{Path, PathBuf},
};

#[cfg(feature = "download-dictionary")]
const DICTIONARY_URL: &str = "https://github.com/o24s/haqumei/releases/download/dictionary-20260829/dictionary-20260829.tar.zst";
#[cfg(feature = "download-dictionary")]
const COMPRESSED_DICTIONARY_HASH: &str =
    "c338ea8d785df4e9ab38d9a8b5e602bdda9abe349ed2fdb94880ac051d521b1d";
#[cfg(feature = "download-dictionary")]
const DICTIONARY_HASH: &str = "e02c49364287546b26a3650148323fcea8a80aaa6ca4bb35c41caa60f460ca07";
const DICTIONARY_NAME: &str = "dictionary.tar.zst";

#[cfg(feature = "download-dictionary")]
static CACHE_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    let cache_dir = dirs::cache_dir().unwrap().join("haqumei");
    fs::create_dir_all(&cache_dir).unwrap();
    cache_dir
});

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-env-changed=HAQUMEI_DICT_SRC");
    println!("cargo:rerun-if-env-changed=DOCS_RS");
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").ok_or("OUT_DIR is missing")?);
    if env::var_os("DOCS_RS").is_some() {
        println!(
            "cargo:rustc-env=HAQUMEI_EMBED_DICT_PATH={}",
            Path::new(&env::var("CARGO_MANIFEST_DIR")?)
                .join("build.rs")
                .display()
        );
        println!("cargo:rustc-env=HAQUMEI_DICT_HASH=docs");
        return Ok(());
    }
    let has_download = env::var_os("CARGO_FEATURE_DOWNLOAD_DICTIONARY").is_some();
    let has_build = env::var_os("CARGO_FEATURE_BUILD_DICTIONARY").is_some();
    if has_download == has_build {
        return Err("Enable exactly one of download-dictionary and build-dictionary".into());
    }
    #[cfg(feature = "download-dictionary")]
    let is_ci = env::var_os("CI").is_some();
    #[cfg(feature = "download-dictionary")]
    if has_download {
        let cached_dict_path = CACHE_DIR.join(DICTIONARY_NAME);
        let compressed_dict_path = out_dir.join(DICTIONARY_NAME);
        let mut need_download = true;

        if cached_dict_path.exists() {
            let mut hasher = IoWrapper(Sha256::new());
            let mut file = File::open(&cached_dict_path)?;
            io::copy(&mut file, &mut hasher)?;
            if hex::encode(hasher.0.finalize()) == COMPRESSED_DICTIONARY_HASH {
                fs::copy(&cached_dict_path, &compressed_dict_path)?;
                need_download = false;
            }
        }

        if is_ci && need_download && compressed_dict_path.exists() {
            let mut hasher = IoWrapper(Sha256::new());
            let mut file = File::open(&compressed_dict_path)?;
            io::copy(&mut file, &mut hasher)?;
            if hex::encode(hasher.0.finalize()) == COMPRESSED_DICTIONARY_HASH {
                need_download = false;
            }
        }

        if need_download {
            let mut response = reqwest::blocking::get(DICTIONARY_URL)?;
            if !response.status().is_success() {
                panic!("Failed to download the dictionary from {}", DICTIONARY_URL);
            }

            let mut temp_file = tempfile::NamedTempFile::new_in(&*CACHE_DIR)?;
            response.copy_to(&mut temp_file)?;

            temp_file.seek(SeekFrom::Start(0))?;
            let calculated_hash = {
                let mut hasher = IoWrapper(Sha256::new());
                io::copy(&mut temp_file, &mut hasher)?;
                hex::encode(hasher.0.finalize())
            };

            if calculated_hash != COMPRESSED_DICTIONARY_HASH {
                panic!("Downloaded file checksum mismatch. It may be corrupted.")
            }

            temp_file.persist(&cached_dict_path)?;
            fs::copy(&cached_dict_path, &compressed_dict_path)?;
        }

        println!(
            "cargo:rustc-env=HAQUMEI_EMBED_DICT_PATH={}",
            compressed_dict_path.display()
        );
        println!("cargo:rustc-env=HAQUMEI_DICT_HASH={}", DICTIONARY_HASH);
    }

    if has_build && env::var_os("CARGO_FEATURE_EMBED_DICTIONARY").is_some() {
        let source = env::var_os("HAQUMEI_DICT_SRC")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("dictionary"));
        println!("cargo:rerun-if-changed={}", source.display());
        let compiled = out_dir.join("dictionary_out");
        let archive_path = out_dir.join(DICTIONARY_NAME);
        if compiled.exists() {
            fs::remove_dir_all(&compiled)?;
        }
        fs::create_dir_all(&compiled)?;
        haqumei_jpreprocess_dictionary::mecab_compile::build_system(
            &source,
            &compiled,
            &Default::default(),
        )?;
        let encoder = zstd::Encoder::new(File::create(&archive_path)?, 19)?;
        let mut tar = tar::Builder::new(encoder);
        tar.append_dir_all(".", &compiled)?;
        tar.into_inner()?.finish()?;
        let hash = hash_files(&compiled, &["dic", "bin"])?;
        println!(
            "cargo:rustc-env=HAQUMEI_EMBED_DICT_PATH={}",
            archive_path.display()
        );
        println!("cargo:rustc-env=HAQUMEI_DICT_HASH={hash}");
    }
    Ok(())
}

fn hash_files(dir: &Path, extensions: &[&str]) -> Result<String, Box<dyn Error>> {
    let mut paths = Vec::new();
    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry?;
        if entry.file_type().is_file()
            && entry
                .path()
                .extension()
                .and_then(|s| s.to_str())
                .is_some_and(|ext| extensions.contains(&ext))
        {
            paths.push(entry.into_path());
        }
    }
    paths.sort();
    let mut hash = Sha256::new();
    for path in paths {
        hash.update(fs::read(path)?);
    }
    Ok(hex::encode(hash.finalize()))
}
