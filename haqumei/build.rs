use std::error::Error;
use std::ffi::OsString;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;
use std::{env, fs};

use sha2::{Digest, Sha256};

#[cfg(feature = "download-dictionary")]
use std::io::{self, Seek, SeekFrom};

#[cfg(feature = "download-dictionary")]
const DICTIONARY_URL: &str = "https://github.com/stellanomia/haqumei/releases/download/v0.1.0/dictionary.tar.zst";
#[cfg(feature = "download-dictionary")]
const COMPRESSED_DICTIONARY_HASH: &str = "2250152f64158f90b6234d1945f8a4099cd6e7218def079f5c610315a859b8d0";
#[cfg(feature = "download-dictionary")]
const DICTIONARY_HASH: &str = "5dbb19b8302188ba5c1a0a2af04e0ee6be480563401dfb0c9391ba9f2d625604";
const DICTIONARY_NAME: &str = "dictionary.tar.zst";

static CACHE_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    let cache_dir = dirs::cache_dir()
        .unwrap()
        .join("haqumei");
    fs::create_dir_all(&cache_dir).unwrap();
    cache_dir
});

fn main() -> Result<(), Box<dyn Error>> {
    let is_ci = env::var_os("CI").is_some();
    let is_docs_rs = env::var_os("DOCS_RS").is_some();
    let out_dir = env::var("OUT_DIR")?;
    let out_dir = Path::new(&out_dir);

    let has_download = env::var_os("CARGO_FEATURE_DOWNLOAD_DICTIONARY").is_some();
    let has_build = env::var_os("CARGO_FEATURE_BUILD_DICTIONARY").is_some();

    if (has_download && has_build) && !(is_ci || is_docs_rs) {
        panic!(
            "The features \"download-dictionary\" and \"build-dictionary\" cannot be enabled simultaneously."
        );
    }

    if !(has_download || has_build || is_ci || is_docs_rs) {
        panic!(
            "You must enable either \"download-dictionary\" or \"build-dictionary\" to prepare the dictionary."
        );
    }

    let src_dir_str = "vendor/open_jtalk/src";
    let src_dir = Path::new(src_dir_str);
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("Failed to get MANIFEST_DIR");
    let manifest_dir = Path::new(&manifest_dir);
    let target = std::env::var("TARGET").unwrap();

    let watch_files = ["redirect.c", "redirect.h", "wrapper.h", src_dir_str];

    for path in &watch_files {
        println!("cargo:rerun-if-changed={}", path);
    }

    if target.contains("msvc") && std::env::var("LIBCLANG_PATH").is_err() {
        let error_msg = r#"
==============================================================================
ERROR: libclang.dll not found / libclang.dll が見つかりません。

[EN] LLVM is required to build haqumei on Windows.
- Install LLVM
   > winget install LLVM.LLVM
- Set `LIBCLANG_PATH` as an environment variable (e.g., C:\Program Files\LLVM\bin\libclang.dll)
- Restart your terminal

[JA] Windows で haqumei をビルドするには LLVM が必要です。

- LLVMをインストールしてください:
   > winget install LLVM.LLVM
- `LIBCLANG_PATH` を環境変数に設定してください。 (e.g., C:\Program Files\LLVM\bin\libclang.dll)
- インストール後、ターミナルを再起動してください。

Ref: https://rust-lang.github.io/rust-bindgen/requirements.html
=============================================================================="#
            .trim();

        for line in error_msg.lines() {
            println!("cargo:warning={}", line);
        }
        panic!("LIBCLANG_PATH is not set.");
    }

    let compressed_dict_path = CACHE_DIR.join(DICTIONARY_NAME);

    #[cfg(feature = "download-dictionary")]
    if has_download {
        let mut need_download = true;

        if compressed_dict_path.exists() {
            let mut hasher = Sha256::new();
            let mut file = File::open(&compressed_dict_path)?;
            io::copy(&mut file, &mut hasher)?;
            if hex::encode(hasher.finalize()) == COMPRESSED_DICTIONARY_HASH {
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
                let mut hasher = Sha256::new();
                io::copy(&mut temp_file, &mut hasher)?;
                hex::encode(hasher.finalize())
            };

            if calculated_hash != COMPRESSED_DICTIONARY_HASH {
                panic!("Downloaded file checksum mismatch. It may be corrupted.")
            }

            temp_file.persist(&compressed_dict_path)?;
        }

        println!("cargo:rustc-env=HAQUMEI_EMBED_DICT_PATH={}", &compressed_dict_path.display());
        println!("cargo:rustc-env=HAQUMEI_DICT_HASH={}", DICTIONARY_HASH);
    }

    let mut defines = vec![
        // CMake: add_definitions(...), set(...)
        ("DIC_VERSION", Some("102")),
        ("MECAB_DEFAULT_RC", Some("\"dummy\"")),
        ("MECAB_WITHOUT_SHARE_DIC", None),
        ("PACKAGE", Some("\"open_jtalk\"")),
        ("PACKAGE_VERSION", Some("\"1.11\"")),
        ("VERSION", Some("\"1.11\"")),
        ("PACKAGE_STRING", Some("\"open_jtalk 1.11\"")),
        (
            "PACKAGE_BUGREPORT",
            Some("\"https://github.com/stellanomia/haqumei\""),
        ),
        ("PACKAGE_NAME", Some("\"open_jtalk\"")),
        ("CHARSET_UTF_8", None),
        ("MECAB_CHARSET", Some("\"utf-8\"")),
        ("MECAB_UTF8_USE_ONLY", None),
        ("HAVE_CTYPE_H", Some("1")),
        ("HAVE_FCNTL_H", Some("1")),
        ("HAVE_INTTYPES_H", Some("1")),
        ("HAVE_MEMORY_H", Some("1")),
        ("HAVE_SETJMP_H", Some("1")),
        ("HAVE_STDINT_H", Some("1")),
        ("HAVE_STDLIB_H", Some("1")),
        ("HAVE_STRING_H", Some("1")),
        ("HAVE_SYS_STAT_H", Some("1")),
        ("HAVE_SYS_TYPES_H", Some("1")),
        ("HAVE_GETENV", Some("1")),
        ("HAVE_STRSTR", Some("1")),
        ("SIZEOF_CHAR", Some("1")),
        ("SIZEOF_SHORT", Some("2")),
        ("SIZEOF_INT", Some("4")),
        ("SIZEOF_LONG_LONG", Some("8")),
    ];

    if cfg!(unix) {
        defines.extend(vec![
            ("HAVE_DIRENT_H", Some("1")),
            ("HAVE_STRINGS_H", Some("1")),
            ("HAVE_SYS_MMAN_H", Some("1")),
            ("HAVE_SYS_TIMES_H", Some("1")),
            ("HAVE_UNISTD_H", Some("1")),
            ("HAVE_GETPAGESIZE", Some("1")),
            ("HAVE_MMAP", Some("1")),
            ("HAVE_OPENDIR", Some("1")),
            ("HAVE_SETJMP", Some("1")),
            ("HAVE_LIBM", Some("1")),
        ]);
    }
    if cfg!(windows) {
        defines.extend(vec![
            ("HAVE_WINDOWS_H", Some("1")),
            ("HAVE_IO_H", Some("1")),
        ]);
    }

    if cfg!(target_os = "windows") {
        defines.push(("SIZEOF_LONG", Some("4")));
    } else {
        defines.push(("SIZEOF_LONG", Some("8")));
    }

    match env::var("CARGO_CFG_TARGET_POINTER_WIDTH").unwrap().as_str() {
        "64" => defines.push(("SIZEOF_SIZE_T", Some("8"))),
        "32" => defines.push(("SIZEOF_SIZE_T", Some("4"))),
        w => panic!("Unsupported target pointer width: {}", w),
    };

    if cfg!(target_endian = "big") {
        defines.push(("WORDS_BIGENDIAN", Some("1")));
    }

    let redirect_header_path = manifest_dir.join("redirect.h");
    let redirect_flag = redirect_header_path.as_os_str();

    cc::Build::new().file("redirect.c").compile("redirect_impl");

    let mut build = cc::Build::new();
    build.cpp(true);

    let include_dirs = [
        "jpcommon",
        "mecab/src",
        "mecab2njd",
        "njd",
        "njd2jpcommon",
        "njd_set_accent_phrase",
        "njd_set_accent_type",
        "njd_set_digit",
        "njd_set_long_vowel",
        "njd_set_pronunciation",
        "njd_set_unvoiced_vowel",
        "text2mecab",
    ];
    for dir in &include_dirs {
        build.include(src_dir.join(dir));
    }

    for dir in &include_dirs {
        for ext in ["c", "cpp"] {
            let pattern = src_dir.join(dir).join(format!("*.{}", ext));
            for entry in glob::glob(pattern.to_str().unwrap()).expect("Failed to read glob pattern")
            {
                build.file(entry.unwrap());
            }
        }
    }

    for (key, value) in &defines {
        build.define(key, value.map(|v| v));
    }

    // compiler flags
    if build.get_compiler().is_like_msvc() {
        build.flag("/FI");
        build.flag(redirect_flag);

        build.define("_CRT_SECURE_NO_WARNINGS", None);
        build.define("_CRT_NONSTDC_NO_WARNINGS", None);
        build.flag("/source-charset:utf-8");
        build.flag("/execution-charset:utf-8");

        build.flag("/wd4100");
        build.flag("/wd4065");
    } else {
        build.flag("-include");
        build.flag(redirect_flag);

        build.flag("-fPIC");
        build.flag("-finput-charset=UTF-8");
        build.flag("-fexec-charset=UTF-8");
        build.flag("-Wno-narrowing");

        build.flag("-Wno-unused-parameter");
        build.flag("-Wno-write-strings");
        build.flag("-Wno-type-limits");
        build.flag("-Wno-class-memaccess");
        build.flag("-Wno-missing-field-initializers");
        build.flag("-Wno-implicit-fallthrough");
        build.flag("-Wno-restrict");
        build.flag("-Wno-sign-compare");
        build.flag("-Wno-unused-function");
        build.flag("-Wno-unused-variable");
        build.flag("-Wno-ignored-qualifiers");
        build.flag("-Wno-stringop-truncation");
    }

    if cfg!(unix) {
        println!("cargo:rustc-link-lib=m");
    }

    build.compile("openjtalk");

    let dict_indexer_path = build_dict_indexer(src_dir, out_dir, &defines, &include_dirs)?;

    // println!("cargo:warning=Generating bindings for openjtalk...");

    let mut bindgen_builder = bindgen::Builder::default()
        .header("wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .blocklist_item("FP_NAN")
        .blocklist_item("FP_INFINITE")
        .blocklist_item("FP_ZERO")
        .blocklist_item("FP_SUBNORMAL")
        .blocklist_item("FP_NORMAL")
        .allowlist_function("mecab_.*")
        .allowlist_function("Mecab_.*")
        .allowlist_function("JPCommon.*")
        .allowlist_function("NJD.*")
        .allowlist_function("njd2jpcommon")
        .allowlist_function("njd_set_.*")
        .allowlist_function("mecab2njd")
        .allowlist_function("text2mecab")
        .allowlist_type("mecab_.*")
        .allowlist_type("Mecab.*")
        .allowlist_type("JPCommon.*")
        .allowlist_type("NJD.*")
        .allowlist_var("text2mecab_.*")
        .allowlist_var("MECAB_.*")
        .clang_arg(format!("-I{}", src_dir_str));

    for dir in &include_dirs {
        bindgen_builder = bindgen_builder.clang_arg(format!("-I{}", src_dir.join(dir).display()));
    }

    for (key, value) in &defines {
        let arg = if let Some(val) = value {
            format!("-D{}={}", key, val)
        } else {
            format!("-D{}", key)
        };
        bindgen_builder = bindgen_builder.clang_arg(arg);
    }

    let bindings = bindgen_builder
        .generate()
        .expect("Unable to generate bindings");

    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Couldn't write bindings to file");

    if is_ci | is_docs_rs {
        println!("cargo:rustc-env=HAQUMEI_EMBED_DICT_PATH={}", manifest_dir.join("build.rs").display());
        println!("cargo:rustc-env=HAQUMEI_DICT_HASH=ci_dummy");
        return Ok(());
    }

    if env::var("CARGO_FEATURE_EMBED_DICTIONARY").is_err() {
        println!("'embed-dictionary' feature is not enabled. Skipping dictionary compilation.");
        return Ok(());
    }

    if !has_build {
        return Ok(());
    }

    let mut dict_src_dir = manifest_dir.join("dictionary");
    let dict_out_dir = out_dir.join("dictionary_out");
    let compressed_dict_hash_path = CACHE_DIR.join("dictionary.tar.zst.sha256");
    let dict_hash_path = CACHE_DIR.join("dictionary.sha256");
    let compiled_dict_hash_path = CACHE_DIR.join("compiled_dictionary.sha256");

    if dict_src_dir.is_file()
        && let Some(parent) = manifest_dir.parent()
    {
        dict_src_dir = parent.join("dictionary");
    }

    if !dict_src_dir.exists() {
        println!(
            "cargo:warning=dictionary({dict_src_dir:?}) not found, skipping dictionary compilation."
        );
        return Ok(());
    }

    println!("cargo:rerun-if-changed={}", dict_src_dir.display());

    if dict_hash_path.exists()
        && dict_src_dir.exists()
        && compressed_dict_path.exists()
        && compressed_dict_hash_path.exists()
        && let Ok(compressed_dict_hash) = calculate_compressed_dict_hash(&compressed_dict_path)
        && let Ok(dict_hash) = calculate_hash_for_extensions(&dict_src_dir, &["def", "csv"])
        && let Ok(saved_dict_hash_path) = fs::read_to_string(&dict_hash_path)
        && let Ok(saved_compressed_dict_hash_path) = fs::read_to_string(&compressed_dict_hash_path)
        && saved_dict_hash_path == dict_hash
        && saved_compressed_dict_hash_path == compressed_dict_hash
    {
        println!("Dictionary cache in MANIFEST_DIR is up-to-date. Skipping compilation.");
        return Ok(());
    }

    fs::create_dir_all(&dict_out_dir)?;

    run_dict_indexer(&dict_indexer_path, &dict_src_dir, &dict_out_dir)?;

    let tar_file = File::create(&compressed_dict_path)?;

    {
        let zstd_writer = zstd::Encoder::new(tar_file, 22)?.auto_finish();
        let mut tar_builder = tar::Builder::new(zstd_writer);

        tar_builder.append_dir_all(".", &dict_out_dir)?;
        tar_builder.finish()?;
    }

    let dict_hash = calculate_hash_for_extensions(&dict_src_dir, &["def", "csv"])?;
    let compressed_dict_hash = calculate_compressed_dict_hash(&compressed_dict_path)?;
    let compiled_dict_hash = calculate_hash_for_extensions(&dict_out_dir, &["dic", "bin"])?;
    fs::write(&dict_hash_path, dict_hash)?;
    fs::write(&compiled_dict_hash_path, &compiled_dict_hash)?;
    fs::write(&compressed_dict_hash_path, &compressed_dict_hash)?;
    if dict_out_dir.exists() {
        fs::remove_dir_all(dict_out_dir)?;
    }

    println!("cargo:rustc-env=HAQUMEI_EMBED_DICT_PATH={}", &compressed_dict_path.display());
    println!("cargo:rustc-env=HAQUMEI_DICT_HASH={}", &compiled_dict_hash);

    // println!("cargo:warning=Dictionary compressed to {}", compressed_dict_path.display());
    Ok(())
}

fn build_dict_indexer(
    src_dir: &Path,
    out_dir: &Path,
    defines: &[(&str, Option<&str>)],
    include_dirs: &[&str],
) -> Result<PathBuf, Box<dyn Error>> {
    let main_wrapper_src = r#"
#include "mecab.h"

int main(int argc, char **argv) {
  return mecab_dict_index(argc, argv);
}
"#;
    let main_wrapper_path = out_dir.join("main_wrapper.cpp");
    fs::write(&main_wrapper_path, main_wrapper_src)?;

    let mut build = cc::Build::new();
    build.cpp(true);
    let compiler = build.get_compiler();
    let mut command = compiler.to_command();

    let exe_name = if cfg!(target_os = "windows") {
        "mecab-dict-index.exe"
    } else {
        "mecab-dict-index"
    };
    let exe_path = out_dir.join(exe_name);

    command.arg(&main_wrapper_path);
    if compiler.is_like_msvc() {
        let mut arg = OsString::from("/Fe");
        arg.push(exe_path.as_os_str());
        command.arg(arg);
    } else {
        command.arg("-o").arg(&exe_path);
    }

    for dir in include_dirs {
        command.arg(format!("-I{}", src_dir.join(dir).display()));
    }

    for (key, value) in defines {
        let arg = if let Some(val) = value {
            format!("-D{}={}", key, val)
        } else {
            format!("-D{}", key)
        };
        command.arg(arg);
    }

    if compiler.is_like_msvc() {
        command.arg("/link");

        let mut arg = OsString::from("/LIBPATH:");
        arg.push(out_dir);
        command.arg(arg);

        command.arg("openjtalk.lib");
    } else {
        let mut arg = OsString::from("-L");
        arg.push(out_dir);
        command.arg(arg);

        command.arg("-lopenjtalk");
        if cfg!(unix) {
            command.arg("-lm");
        }
    }

    let output = command.output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to build mecab-dict-index executable.\nStatus: {}\nStdout: {}\nStderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    Ok(exe_path)
}

fn run_dict_indexer(
    indexer_path: &Path,
    dict_dir: &Path,
    out_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let dict_dir_str = dict_dir
        .to_str()
        .ok_or("Dictionary path contains invalid UTF-8")?;
    let out_dir_str = out_dir
        .to_str()
        .ok_or("Dictionary path contains invalid UTF-8")?;

    let output = Command::new(indexer_path)
        .arg("-d")
        .arg(dict_dir_str)
        .arg("-o")
        .arg(out_dir_str)
        .arg("-f")
        .arg("utf-8")
        .arg("-t")
        .arg("utf-8")
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "mecab-dict-index execution failed.\nstatus: {}\nstdout: {}\nstderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    Ok(())
}

fn calculate_compressed_dict_hash(path: &Path) -> Result<String, Box<dyn Error>> {
    let mut hasher = Sha256::new();

    let mut file = File::open(path)?;
    std::io::copy(&mut file, &mut hasher)?;

    Ok(hex::encode(hasher.finalize()))
}

fn calculate_hash_for_extensions(
    dir: &Path,
    extensions: &[&str],
) -> Result<String, Box<dyn Error>> {
    let mut hasher = Sha256::new();
    let mut paths = Vec::new();

    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry?;
        let path = entry.path();

        if path.is_file()
            && let Some(ext_str) = path.extension().and_then(|s| s.to_str())
            && extensions.contains(&ext_str)
        {
            paths.push(path.to_path_buf());
        }
    }

    paths.sort();

    for path in paths {
        let mut file = File::open(&path)?;
        std::io::copy(&mut file, &mut hasher)?;
    }

    Ok(hex::encode(hasher.finalize()))
}
