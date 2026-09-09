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
const DICTIONARY_URL: &str =
    "https://github.com/o24s/haqumei/releases/download/dictionary-20260909/dictionary.tar.zst";
#[cfg(feature = "download-dictionary")]
const COMPRESSED_DICTIONARY_HASH: &str =
    "5802f29334476f4e467fd0630e5a5047b7e26215aeb91a1d97b6ca1f13145702";
const DICTIONARY_NAME: &str = "dictionary.tar.zst";
const RUNTIME_DICTIONARY_FILES: [&str; 3] = ["char.bin", "matrix.bin", "system.bin"];

#[cfg(feature = "download-dictionary")]
static CACHE_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    let cache_dir = dirs::cache_dir().unwrap().join("haqumei");
    fs::create_dir_all(&cache_dir).unwrap();
    cache_dir
});

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-env-changed=HAQUMEI_DICT_SRC");
    println!("cargo:rerun-if-env-changed=DOCS_RS");
    println!("cargo:rerun-if-env-changed=HAQUMEI_DICT_ARCHIVE");
    println!("cargo:rerun-if-env-changed=HAQUMEI_DICT_RELEASE_NONCE");
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
    if let Some(path) = env::var_os("HAQUMEI_DICT_ARCHIVE") {
        if env::var_os("CARGO_FEATURE_EMBED_DICTIONARY").is_none() {
            return Err("HAQUMEI_DICT_ARCHIVE requires the embed-dictionary feature".into());
        }
        let path = PathBuf::from(path).canonicalize()?;
        println!("cargo:rerun-if-changed={}", path.display());
        let prepared = out_dir.join("supplied-dictionary");
        if prepared.exists() {
            fs::remove_dir_all(&prepared)?;
        }
        fs::create_dir_all(&prepared)?;
        tar::Archive::new(zstd::Decoder::new(File::open(&path)?)?).unpack(&prepared)?;
        let hash = validate_archive_dictionary(&prepared)?;
        println!("cargo:rustc-env=HAQUMEI_EMBED_DICT_PATH={}", path.display());
        println!("cargo:rustc-env=HAQUMEI_DICT_HASH={hash}");
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

        let prepared = out_dir.join("prepared-dictionary");
        if prepared.exists() {
            fs::remove_dir_all(&prepared)?;
        }
        fs::create_dir_all(&prepared)?;
        tar::Archive::new(zstd::Decoder::new(File::open(&compressed_dict_path)?)?)
            .unpack(&prepared)?;
        let hash = validate_archive_dictionary(&prepared)?;
        println!(
            "cargo:rustc-env=HAQUMEI_EMBED_DICT_PATH={}",
            compressed_dict_path.display()
        );
        println!("cargo:rustc-env=HAQUMEI_DICT_HASH={hash}");
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
        write_dictionary_archive(&compiled, &archive_path, 19)?;
        let hash = validate_dictionary(&compiled)?;
        println!(
            "cargo:rustc-env=HAQUMEI_EMBED_DICT_PATH={}",
            archive_path.display()
        );
        println!("cargo:rustc-env=HAQUMEI_DICT_HASH={hash}");
    }
    Ok(())
}

fn hash_dictionary(dir: &Path) -> Result<String, Box<dyn Error>> {
    let mut hash = Sha256::new();
    for name in RUNTIME_DICTIONARY_FILES {
        let path = dir.join(name);
        if !path.is_file() {
            return Err(format!("dictionary archive must contain {name}").into());
        }
        hash.update(fs::read(&path)?);
    }
    Ok(hex::encode(hash.finalize()))
}

fn validate_dictionary(dir: &Path) -> Result<String, Box<dyn Error>> {
    let hash = hash_dictionary(dir)?;
    let model = haqumei_jpreprocess_dictionary::mecab::Model::open(dir, &[])?;
    model.worker()?.analyze("あ")?;
    Ok(hash)
}

fn validate_archive_dictionary(dir: &Path) -> Result<String, Box<dyn Error>> {
    let mut names = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            return Err("dictionary archive must contain exactly three files at its root".into());
        }
        names.push(
            entry
                .file_name()
                .into_string()
                .map_err(|_| "dictionary archive contains a non-UTF-8 file name")?,
        );
    }
    names.sort();
    let expected = RUNTIME_DICTIONARY_FILES.map(str::to_owned);
    if names != expected {
        return Err(format!(
            "dictionary archive must contain exactly {} at its root",
            RUNTIME_DICTIONARY_FILES.join(", ")
        )
        .into());
    }
    validate_dictionary(dir)
}

fn write_dictionary_archive(
    source: &Path,
    output: &Path,
    level: i32,
) -> Result<(), Box<dyn Error>> {
    let encoder = zstd::Encoder::new(File::create(output)?, level)?;
    let mut archive = tar::Builder::new(encoder);
    for name in RUNTIME_DICTIONARY_FILES {
        let mut file = File::open(source.join(name))?;
        let mut header = tar::Header::new_gnu();
        header.set_size(file.metadata()?.len());
        header.set_mode(0o644);
        header.set_uid(0);
        header.set_gid(0);
        header.set_mtime(0);
        header.set_cksum();
        archive.append_data(&mut header, name, &mut file)?;
    }
    archive.into_inner()?.finish()?;
    Ok(())
}
