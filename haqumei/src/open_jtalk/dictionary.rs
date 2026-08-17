use std::{
    ffi::{CString, NulError},
    fs, io,
    path::{Path, PathBuf},
    sync::Arc,
};

use libc::{c_char, c_int};
use thiserror::Error;

use crate::{
    errors::HaqumeiError, ffi, open_jtalk::model::MecabModel, setup_cpp_redirect,
    teardown_cpp_redirect,
};

#[cfg(feature = "embed-dictionary")]
static DICT_EXTRACT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// `OpenJTalk` が使用する辞書オブジェクト
#[derive(Debug, Clone)]
pub struct Dictionary {
    pub(crate) model: Arc<MecabModel>,
    pub(crate) dict_dir: PathBuf,
}

impl Dictionary {
    /// システム辞書パス、ユーザー辞書パスから [Dictionary] を生成します。
    ///
    /// パスは絶対パスに解決してから Mecab に渡します。Mecab は辞書内のファイルを
    /// 開く際に相対パスを解釈できるとは限らず、またプロセスのカレントディレクトリが
    /// 変わると同じ [Dictionary] が別の辞書を指しかねないためです。
    /// 保持する `dict_dir` も解決後のパスになります。
    pub fn from_path<P: AsRef<Path>>(
        dict_dir: P,
        user_dict: Option<P>,
    ) -> Result<Self, HaqumeiError> {
        // 存在しない場合に Mecab のロード失敗ではなく、原因の分かるエラーを返す
        let resolve = |p: &Path| -> Result<PathBuf, HaqumeiError> {
            p.canonicalize().map_err(|_| HaqumeiError::DictionaryNotFound {
                path: p.to_path_buf(),
            })
        };

        let dict_dir = resolve(dict_dir.as_ref())?;
        let user_dict = user_dict
            .as_ref()
            .map(|p| resolve(p.as_ref()))
            .transpose()?;

        let model = MecabModel::new(&dict_dir, user_dict.as_deref())?;
        Ok(Self {
            model: Arc::new(model),
            dict_dir,
        })
    }

    #[cfg(feature = "embed-dictionary")]
    /// バイナリに埋め込まれた辞書から [Dictionary] を生成します。
    pub fn from_embedded() -> Result<Self, HaqumeiError> {
        use crate::utils::compute_metadata_key;

        use sha2::{Digest, Sha256};
        use std::{fs::File, io::Read};

        const DICTIONARY_BYTES: &[u8] = include_bytes!(env!("HAQUMEI_EMBED_DICT_PATH"));
        const EXPECTED_DICT_HASH: &str = env!("HAQUMEI_DICT_HASH");

        let cache_dir = dirs::cache_dir()
            .ok_or(HaqumeiError::CacheDirectoryNotFound)?
            .join("haqumei");
        let dict_path = cache_dir.join("decompressed");

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

        fs4::FileExt::lock(&lock_file).map_err(|e| HaqumeiError::CacheIo {
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
            // マーカーの名前に期待する辞書のハッシュを含める。
            //
            // これが無いと、別の辞書で書かれたマーカーによって内容の検証が
            // 飛ばされ、意図しない辞書がそのまま使われる。`build-dictionary` と
            // `download-dictionary` は展開先 (`decompressed`) を共有するため、
            // feature を切り替えて両方をビルドすると実際に起こる。
            let metadata_hash_path = meta_cache_dir
                .join(format!("{}-{metadata_hash}.sha256", EXPECTED_DICT_HASH.trim()));

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

/// [MecabDictIndexCompiler] が使用するエラー型。
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

/// Mecab 辞書をビルドするコンパイラ。
#[derive(Debug)]
pub struct MecabDictIndexCompiler {
    dict_dir: PathBuf,
    out_dir: PathBuf,
    model_in: Option<PathBuf>,
    userdict_out: Option<PathBuf>,
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
    /// 新しい [MecabDictIndexCompiler] を生成します。
    pub fn new() -> Self {
        Self {
            dict_dir: PathBuf::from("."),
            out_dir: PathBuf::from("."),
            model_in: None,
            userdict_out: None,
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

    /// 辞書ディレクトリを設定します。`-d` または `--dicdir` オプションに対応します。
    pub fn dict_dir<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.dict_dir = path.as_ref().to_path_buf();
        self
    }

    /// 出力ディレクトリを設定します。`-o` または `--outdir` オプションに対応します。
    pub fn out_dir<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.out_dir = path.as_ref().to_path_buf();
        self
    }

    /// モデルファイルを設定します。`--model` オプションに対応します。
    pub fn model_in<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.model_in = Some(path.as_ref().to_path_buf());
        self
    }

    /// 構築するユーザー辞書の出力ファイルパスを設定します。`-u` または `--userdic` オプションに対応します。
    pub fn userdict_out_path<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.userdict_out = Some(path.as_ref().to_path_buf());
        self
    }

    /// 未知語辞書を構築するかどうかを設定します。`--build-unknown` フラグに対応します。
    pub fn build_unknown(&mut self, build: bool) -> &mut Self {
        self.build_unknown = build;
        self
    }

    /// モデルファイルを構築するかどうかを設定します。`--build-model` フラグに対応します。
    pub fn build_model(&mut self, build: bool) -> &mut Self {
        self.build_model = build;
        self
    }

    /// 文字カテゴリマップを構築するかどうかを設定します。`--build-charcategory` フラグに対応します。
    pub fn build_charcategory(&mut self, build: bool) -> &mut Self {
        self.build_charcategory = build;
        self
    }

    /// システム辞書を構築するかどうかを設定します。`--build-sysdic` フラグに対応します。
    pub fn build_sysdic(&mut self, build: bool) -> &mut Self {
        self.build_sysdic = build;
        self
    }

    /// 接続行列 (matrix) を構築するかどうかを設定します。`--build-matrix` フラグに対応します。
    pub fn build_matrix(&mut self, build: bool) -> &mut Self {
        self.build_matrix = build;
        self
    }

    /// バイナリ辞書の文字セットを設定します。`-c`、`-t`、または `--charset` オプションに対応します。
    pub fn charset(&mut self, charset: &str) -> &mut Self {
        self.charset = Some(charset.to_string());
        self
    }

    /// 入力CSVの想定文字セットを設定します。`-f` または `--dictionary-charset` オプションに対応します。
    pub fn dictionary_charset(&mut self, charset: &str) -> &mut Self {
        self.dictionary_charset = Some(charset.to_string());
        self
    }

    /// 進捗メッセージの出力を抑制します。`-q` または `--quiet` フラグに対応します。
    pub fn quiet(&mut self, quiet: bool) -> &mut Self {
        self.quiet = quiet;
        self
    }

    /// 処理対象の入力ファイルをリストに追加します。
    pub fn add_input_file<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.input_files.push(path.as_ref().to_path_buf());
        self
    }

    /// 設定されたオプションを使用して辞書のコンパイルを実行します。
    ///
    /// このメソッドは、ビルダーの状態に基づいてコマンドライン引数を構築し、
    /// FFI関数 `mecab_dict_index` を呼び出して結果を返します。
    ///
    /// # デフォルトの挙動
    ///
    /// `userdict_out` が設定されておらず、かつ `build_*` フラグがいずれも明示的に
    /// 有効化されていない場合、このメソッドは自動的にすべての `build_*` フラグを有効にして
    /// 完全なシステム辞書をコンパイルします。
    pub fn run(&self) -> Result<(), DictCompilerError> {
        let mut c_string_args: Vec<CString> = Vec::new();
        unsafe {
            setup_cpp_redirect();
        };

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

        let should_build_all = self.userdict_out.is_none()
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
        add_optional_path_arg(&mut c_string_args, "-u", &self.userdict_out)?;
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

        unsafe {
            teardown_cpp_redirect();
        }

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

#[cfg(feature = "embed-dictionary")]
pub(crate) fn collect_dict_files(dir: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut paths = Vec::new();

    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry?;
        let path = entry.path();
        if path.is_file()
            && let Some(extension) = path.extension()
            && (extension == "dic" || extension == "bin")
        {
            paths.push(path.to_path_buf());
        }
    }

    paths.sort();

    Ok(paths)
}
