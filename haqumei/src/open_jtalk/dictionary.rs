use std::{
    ffi::{CString, NulError},
    fs, io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use libc::{c_char, c_int};
use thiserror::Error;

use crate::{errors::HaqumeiError, ffi, open_jtalk::model::MecabModel};

static DICT_EXTRACT_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone)]
pub struct Dictionary {
    pub(crate) model: Arc<MecabModel>,
    pub(crate) dict_dir: PathBuf,
}

impl Dictionary {
    pub fn from_path<P: AsRef<Path>>(
        dict_dir: P,
        user_dict: Option<P>,
    ) -> Result<Self, HaqumeiError> {
        let path_to_string = |p: &Path| -> Result<String, HaqumeiError> {
            p.to_str().map(|s| s.to_string()).ok_or_else(|| {
                HaqumeiError::InvalidDictionaryPath(p.to_string_lossy().into_owned())
            })
        };

        let dict_dir_str = path_to_string(dict_dir.as_ref())?;
        let user_dict_ref = user_dict.as_ref().map(|p| p.as_ref());
        let user_dict_str = user_dict_ref.map(path_to_string).transpose()?;

        let model = MecabModel::new(&dict_dir_str, user_dict_str.as_deref())?;
        Ok(Self {
            model: Arc::new(model),
            dict_dir: dict_dir.as_ref().to_path_buf(),
        })
    }

    #[cfg(feature = "embed-dictionary")]
    pub fn from_embedded() -> Result<Self, HaqumeiError> {
        use crate::utils::collect_dict_files;
        use crate::utils::compute_metadata_key;
        use fs4::fs_std::FileExt;

        use sha2::{Digest, Sha256};
        use std::{fs::File, io::Read};

        const DICTIONARY_BYTES: &[u8] =
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/dictionary.tar.zst"));
        const EXPECTED_DICT_HASH: &str = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/compiled_dictionary.sha256"
        ));

        let cache_dir = dirs::cache_dir()
            .ok_or(HaqumeiError::CacheDirectoryNotFound)?
            .join("haqumei");
        let dict_path = cache_dir.join("dict");

        let _thread_guard = DICT_EXTRACT_LOCK.lock().expect("Poisoned");

        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir)?;
        }

        let lock_file_path = cache_dir.join(".lock");

        let lock_file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&lock_file_path)?;

        lock_file
            .lock_exclusive()
            .map_err(|e| HaqumeiError::CacheIo {
                path: lock_file_path.clone(),
                source: e,
            })?;

        let hash_files_full = |paths: &Vec<PathBuf>| -> Result<_, HaqumeiError> {
            let mut file_hasher = Sha256::new();

            for path in paths {
                let mut file = File::open(path)?;
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer)?;
                file_hasher.update(&buffer);
            }

            Ok(file_hasher.finalize())
        };

        let mut needs_unpack = true;

        if dict_path.exists() {
            let paths = collect_dict_files(&dict_path)?;

            let mut metadata_hasher = Sha256::new();

            for path in &paths {
                metadata_hasher.update(compute_metadata_key(&fs::metadata(path)?));
            }

            let metadata_hash = hex::encode(metadata_hasher.finalize());

            let meta_cache_dir = cache_dir.join(".cache");
            let metadata_hash_path = meta_cache_dir.join(format!("{metadata_hash}.sha256"));

            if metadata_hash_path.exists() {
                return Self::from_path(dict_path, None);
            }

            let full_hash = hex::encode(hash_files_full(&paths)?);

            if full_hash == EXPECTED_DICT_HASH.trim() {
                needs_unpack = false;
                if !meta_cache_dir.exists() {
                    fs::create_dir_all(&meta_cache_dir)?;
                }

                if let Ok(entries) = fs::read_dir(&meta_cache_dir) {
                    for entry in entries.flatten() {
                        if let Ok(file_type) = entry.file_type()
                            && file_type.is_file()
                        {
                            let _ = fs::remove_file(entry.path());
                        }
                    }
                }

                File::create(metadata_hash_path)?;
            } else {
                fs::remove_dir_all(&dict_path).map_err(|source| HaqumeiError::CacheIo {
                    path: dict_path.clone(),
                    source,
                })?;
            }
        }

        if needs_unpack {
            use std::fs;

            fs::create_dir_all(&dict_path).map_err(|source| HaqumeiError::CacheIo {
                path: dict_path.clone(),
                source,
            })?;

            let decoder = zstd::Decoder::new(DICTIONARY_BYTES)?;
            let mut archive = tar::Archive::new(decoder);
            archive.unpack(&dict_path)?;

            let paths = collect_dict_files(&dict_path)?;

            let actual_hash = hex::encode(hash_files_full(&paths)?);

            if actual_hash != EXPECTED_DICT_HASH.trim() {
                return Err(HaqumeiError::DictionaryVerification {
                    path: dict_path,
                    expected: EXPECTED_DICT_HASH.to_string(),
                    actual: actual_hash,
                });
            }
        }

        Self::from_path(dict_path, None)
    }
}

#[derive(Debug, Error)]
pub enum DictCompilerError {
    #[error("Path contains null byte and cannot be converted to CString: {0}")]
    InvalidPath(#[from] NulError),
    #[error("Path is not valid UTF-8: {0}")]
    PathNotUtf8(PathBuf),
    #[error("mecab-dict-index failed with exit code {0}")]
    CompilerFailed(c_int),
    #[error("Failed to clean output directory '{0}': {1}")]
    CleanupFailed(PathBuf, #[source] std::io::Error),
    #[error("Failed to create output directory '{0}': {1}")]
    DirectoryCreationFailed(PathBuf, #[source] std::io::Error),
    #[error(transparent)]
    IoError(#[from] io::Error),
}

#[derive(Debug)]
pub struct MecabDictIndexCompiler {
    dict_dir: PathBuf,
    out_dir: PathBuf,
    model_in: Option<PathBuf>,
    userdic_out: Option<PathBuf>,
    build_unknown: bool,
    build_model: bool,
    build_charcategory: bool,
    build_sysdic: bool,
    build_matrix: bool,
    charset: Option<String>,
    dictionary_charset: Option<String>,
    quiet: bool,
    input_files: Vec<PathBuf>,
}

impl MecabDictIndexCompiler {
    pub fn new() -> Self {
        Self {
            dict_dir: PathBuf::from("."),
            out_dir: PathBuf::from("."),
            model_in: None,
            userdic_out: None,
            build_unknown: false,
            build_model: false,
            build_charcategory: false,
            build_sysdic: false,
            build_matrix: false,
            charset: Some("utf-8".to_string()),
            dictionary_charset: Some("utf-8".to_string()),
            quiet: false,
            input_files: Vec::with_capacity(0),
        }
    }

    /// Sets the dictionary directory. Corresponds to the `-d` or `--dicdir` option.
    pub fn dict_dir<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.dict_dir = path.as_ref().to_path_buf();
        self
    }

    /// Sets the output directory. Corresponds to the `-o` or `--outdir` option.
    pub fn out_dir<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.out_dir = path.as_ref().to_path_buf();
        self
    }

    /// Sets the model file. Corresponds to the `--model` option.
    pub fn model_in<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.model_in = Some(path.as_ref().to_path_buf());
        self
    }

    /// Sets the output file path for the user dictionary to be built. Corresponds to the `-u` or `--userdic` option.
    pub fn userdic_out<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.userdic_out = Some(path.as_ref().to_path_buf());
        self
    }

    /// Enables or disables building the unknown word dictionary. Corresponds to the `--build-unknown` flag.
    pub fn build_unknown(&mut self, build: bool) -> &mut Self {
        self.build_unknown = build;
        self
    }

    /// Enables or disables building the model file. Corresponds to the `--build-model` flag.
    pub fn build_model(&mut self, build: bool) -> &mut Self {
        self.build_model = build;
        self
    }

    /// Enables or disables building the character category maps. Corresponds to the `--build-charcategory` flag.
    pub fn build_charcategory(&mut self, build: bool) -> &mut Self {
        self.build_charcategory = build;
        self
    }

    /// Enables or disables building the system dictionary. Corresponds to the `--build-sysdic` flag.
    pub fn build_sysdic(&mut self, build: bool) -> &mut Self {
        self.build_sysdic = build;
        self
    }

    /// Enables or disables building the connection matrix. Corresponds to the `--build-matrix` flag.
    pub fn build_matrix(&mut self, build: bool) -> &mut Self {
        self.build_matrix = build;
        self
    }

    /// Sets the character set of the binary dictionary. Corresponds to the `-c`, `-t`, or `--charset` option.
    pub fn charset(&mut self, charset: &str) -> &mut Self {
        self.charset = Some(charset.to_string());
        self
    }

    /// Sets the assumed character set of the input CSVs. Corresponds to the `-f` or `--dictionary-charset` option.
    pub fn dictionary_charset(&mut self, charset: &str) -> &mut Self {
        self.dictionary_charset = Some(charset.to_string());
        self
    }

    /// Suppresses progress messages. Corresponds to the `-q` or `--quiet` flag.
    pub fn quiet(&mut self, quiet: bool) -> &mut Self {
        self.quiet = quiet;
        self
    }

    /// Adds an input file (typically a CSV) to the list of files to be processed.
    pub fn add_input_file<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.input_files.push(path.as_ref().to_path_buf());
        self
    }

    /// Executes the dictionary compilation with the configured options.
    ///
    /// This method constructs the command-line arguments based on the builder's state,
    /// calls the FFI function `mecab_dict_index`, and returns the result.
    ///
    /// # Default Behavior
    ///
    /// If `userdic_out` is not set and none of the `build_*` flags are explicitly
    /// enabled, this method will automatically enable all `build_*` flags to compile
    /// a full system dictionary. This mimics the default behavior of the
    /// `mecab-dict-index` command-line tool.
    pub fn run(&self) -> Result<(), DictCompilerError> {
        let mut c_string_args: Vec<CString> = Vec::new();

        let dict_dir = &self.dict_dir.canonicalize()?;
        let out_dir = &self.out_dir;

        fs::create_dir_all(&self.out_dir)
            .map_err(|e| DictCompilerError::DirectoryCreationFailed(out_dir.to_path_buf(), e))?;
        let out_dir = &self.out_dir.canonicalize()?;

        for entry in fs::read_dir(out_dir)
            .map_err(|e| DictCompilerError::CleanupFailed(out_dir.to_path_buf(), e))?
        {
            let entry =
                entry.map_err(|e| DictCompilerError::CleanupFailed(out_dir.to_path_buf(), e))?;
            let path = entry.path();

            if path.is_file()
                && let Some(ext) = path.extension().and_then(|s| s.to_str())
                && (ext == "dic" || ext == "bin")
            {
                fs::remove_file(&path)
                    .map_err(|e| DictCompilerError::CleanupFailed(path.clone(), e))?;
            }
        }

        c_string_args.push(CString::new("mecab-dict-index").unwrap());

        fn add_path_arg(
            c_string_args: &mut Vec<CString>,
            opt: &str,
            path: &Path,
        ) -> Result<(), DictCompilerError> {
            c_string_args.push(CString::new(opt)?);
            let path_str = path
                .to_str()
                .ok_or_else(|| DictCompilerError::PathNotUtf8(path.to_path_buf()))?;
            c_string_args.push(CString::new(path_str)?);
            Ok(())
        }

        fn add_optional_path_arg(
            c_string_args: &mut Vec<CString>,
            opt: &str,
            path: &Option<PathBuf>,
        ) -> Result<(), DictCompilerError> {
            if let Some(p) = path {
                add_path_arg(c_string_args, opt, p)?;
            }
            Ok(())
        }

        fn add_str_arg(
            c_string_args: &mut Vec<CString>,
            opt: &str,
            val: &Option<String>,
        ) -> Result<(), DictCompilerError> {
            if let Some(s) = val {
                c_string_args.push(CString::new(opt)?);
                c_string_args.push(CString::new(s.as_str())?);
            }
            Ok(())
        }

        fn add_flag_arg(
            c_string_args: &mut Vec<CString>,
            opt: &str,
            flag: bool,
        ) -> Result<(), DictCompilerError> {
            if flag {
                c_string_args.push(CString::new(opt)?);
            }
            Ok(())
        }

        let should_build_all = self.userdic_out.is_none()
            && [
                self.build_charcategory,
                self.build_matrix,
                self.build_model,
                self.build_sysdic,
                self.build_unknown,
            ]
            .iter()
            .all(|&f| !f);

        add_path_arg(&mut c_string_args, "-d", dict_dir)?;
        add_path_arg(&mut c_string_args, "-o", out_dir)?;
        add_optional_path_arg(&mut c_string_args, "-m", &self.model_in)?;
        add_optional_path_arg(&mut c_string_args, "-u", &self.userdic_out)?;
        add_flag_arg(
            &mut c_string_args,
            "--build-unknown",
            self.build_unknown || should_build_all,
        )?;
        add_flag_arg(
            &mut c_string_args,
            "--build-model",
            self.build_model || should_build_all,
        )?;
        add_flag_arg(
            &mut c_string_args,
            "--build-charcategory",
            self.build_charcategory || should_build_all,
        )?;
        add_flag_arg(
            &mut c_string_args,
            "--build-sysdic",
            self.build_sysdic || should_build_all,
        )?;
        add_flag_arg(
            &mut c_string_args,
            "--build-matrix",
            self.build_matrix || should_build_all,
        )?;
        add_str_arg(&mut c_string_args, "-c", &self.charset)?;
        add_str_arg(&mut c_string_args, "-f", &self.dictionary_charset)?;
        add_flag_arg(&mut c_string_args, "-q", self.quiet)?;

        for file in &self.input_files {
            let file_str = file
                .to_str()
                .ok_or_else(|| DictCompilerError::PathNotUtf8(file.clone()))?;
            c_string_args.push(CString::new(file_str)?);
        }

        let mut argv: Vec<*mut c_char> = c_string_args
            .iter()
            .map(|s| s.as_ptr() as *mut c_char)
            .collect();
        let argc = argv.len() as c_int;

        let result = unsafe { ffi::mecab_dict_index(argc, argv.as_mut_ptr()) };

        if result == 0 {
            Ok(())
        } else {
            Err(DictCompilerError::CompilerFailed(result))
        }
    }
}

impl Default for MecabDictIndexCompiler {
    fn default() -> Self {
        Self::new()
    }
}
