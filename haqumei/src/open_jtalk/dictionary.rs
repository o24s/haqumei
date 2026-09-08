use std::{
    ffi::NulError,
    fs, io,
    path::{Path, PathBuf},
    sync::Arc,
};

use thiserror::Error;

use crate::{errors::HaqumeiError, open_jtalk::model::MecabModel};

#[cfg(feature = "embed-dictionary")]
static DICT_EXTRACT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// `OpenJTalk` が使用する辞書オブジェクト
#[derive(Debug, Clone)]
pub struct Dictionary {
    pub(crate) model: Arc<MecabModel>,
    pub(crate) dict_dir: PathBuf,
}

/// パスを絶対パスに直す。
///
/// 見つからないときと、それ以外の理由で解決できないときを分ける。
fn resolve_path(path: &Path) -> Result<PathBuf, HaqumeiError> {
    path.canonicalize().map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => HaqumeiError::DictionaryNotFound {
            path: path.to_path_buf(),
        },
        _ => HaqumeiError::InvalidDictionaryPath(format!("{}: {e}", path.display())),
    })
}

impl Dictionary {
    /// システム辞書パス、ユーザー辞書パスから [Dictionary] を生成します。
    ///
    /// ユーザー辞書を複数使う場合は [`Dictionary::from_paths`] を使ってください。
    pub fn from_path<P: AsRef<Path>>(
        dict_dir: P,
        user_dict: Option<P>,
    ) -> Result<Self, HaqumeiError> {
        match user_dict {
            Some(user_dict) => Self::from_paths(dict_dir.as_ref(), &[user_dict]),
            None => Self::from_paths::<&Path>(dict_dir.as_ref(), &[]),
        }
    }

    /// システム辞書と、0 個以上のユーザー辞書から [Dictionary] を生成します。
    ///
    /// パスは絶対パスに解決して保持します。
    ///
    /// `dict_dir` がディレクトリでないとき、ユーザー辞書がファイルでないとき、
    /// パスが存在しないときは [`HaqumeiError`] を返します。
    pub fn from_paths<P: AsRef<Path>>(
        dict_dir: &Path,
        user_dicts: &[P],
    ) -> Result<Self, HaqumeiError> {
        let dict_dir = resolve_path(dict_dir)?;
        if !dict_dir.is_dir() {
            return Err(HaqumeiError::InvalidDictionaryPath(format!(
                "{} はディレクトリではありません。\
                 システム辞書は `sys.dic` などを含むディレクトリを指定してください",
                dict_dir.display()
            )));
        }

        let mut resolved = Vec::with_capacity(user_dicts.len());
        for user_dict in user_dicts {
            let path = resolve_path(user_dict.as_ref())?;
            if !path.is_file() {
                return Err(HaqumeiError::InvalidDictionaryPath(format!(
                    "{} はファイルではありません。\
                     ユーザー辞書はコンパイル済みの `.dic` を指定してください",
                    path.display()
                )));
            }
            resolved.push(path);
        }

        let model = MecabModel::new(&dict_dir, &resolved)?;
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
            let metadata_hash_path = meta_cache_dir.join(format!(
                "{}-{metadata_hash}.sha256",
                EXPECTED_DICT_HASH.trim()
            ));

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
    CompilerFailed(i32),
    #[error("Failed to clean output directory '{0}': {1}")]
    CleanupFailed(PathBuf, #[source] std::io::Error),
    #[error("Failed to create output directory '{0}': {1}")]
    DirectoryCreationFailed(PathBuf, #[source] std::io::Error),
    #[error(transparent)]
    IoError(#[from] io::Error),
}

/// 辞書ディレクトリをコンパイルし、同じディレクトリへ書き出します。
///
/// `path` にある `*.csv` / `*.def` から `sys.dic` などを作ります。MeCab の辞書は
/// もとの CSV とコンパイル結果を同じディレクトリに置くので、それに合わせています。
///
/// 出力先を分けたい場合や、ユーザー辞書を作る場合は
/// [`MecabDictIndexCompiler`] を直接使ってください。
pub fn build_mecab_dictionary<P: AsRef<Path>>(path: P) -> Result<(), DictCompilerError> {
    MecabDictIndexCompiler::new()
        .dict_dir(&path)
        .out_dir(&path)
        .run()
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
    /// Rust のコンパイラで MeCab 互換のバイナリ辞書を作ります。
    /// 入出力は UTF-8、EUC-JP、Shift_JIS に対応し、既定は UTF-8 です。
    /// ユーザー辞書のコストを省略する場合は、
    /// [`Self::model_in`] で指定した学習モデルから推定します。
    ///
    /// # デフォルトの挙動
    ///
    /// `userdict_out` が設定されておらず、かつ `build_*` フラグがいずれも明示的に
    /// 有効化されていない場合、このメソッドは自動的にすべての `build_*` フラグを有効にして
    /// 完全なシステム辞書をコンパイルします。
    pub fn run(&self) -> Result<(), DictCompilerError> {
        use haqumei_jpreprocess_dictionary::mecab_compile::{self, BuildOptions};
        let charset = self.charset.as_deref().unwrap_or("utf-8");
        let dictionary_charset = self.dictionary_charset.as_deref().unwrap_or("utf-8");
        if let Some(output) = &self.userdict_out {
            mecab_compile::build_user_with_model_and_charsets(
                &self.dict_dir,
                &self.input_files,
                output,
                self.model_in.as_deref(),
                dictionary_charset,
                charset,
            )?;
        } else {
            mecab_compile::build_system_with_charsets(
                &self.dict_dir,
                &self.out_dir,
                &BuildOptions {
                    unknown: self.build_unknown,
                    charcategory: self.build_charcategory,
                    sysdic: self.build_sysdic,
                    matrix: self.build_matrix,
                    model: self.build_model,
                },
                dictionary_charset,
                charset,
            )?;
        }
        Ok(())
    }
}

impl Default for MecabDictIndexCompiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod compiler_tests {
    use super::MecabDictIndexCompiler;
    use std::fs;

    #[test]
    fn compiler_forwards_input_and_output_charsets() {
        let cases: [(&str, &str, &[u8], &[u8]); 3] = [
            (
                "utf-8",
                "Shift_JIS",
                "あ,0,0,1,名詞\n".as_bytes(),
                b"\x96\xbc\x8e\x8c\0",
            ),
            (
                "EUC-JP",
                "utf-8",
                b"\xa4\xa2,0,0,1,\xcc\xbe\xbb\xec\n",
                "名詞\0".as_bytes(),
            ),
            (
                "Shift_JIS",
                "EUC-JP",
                b"\x82\xa0,0,0,1,\x96\xbc\x8e\x8c\n",
                b"\xcc\xbe\xbb\xec\0",
            ),
        ];
        for (input_charset, output_charset, csv, feature) in cases {
            let directory = tempfile::tempdir().unwrap();
            let source = directory.path().join("user.csv");
            let output = directory.path().join("user.dic");
            fs::write(directory.path().join("matrix.def"), "1 1\n0 0 0\n").unwrap();
            fs::write(&source, csv).unwrap();
            MecabDictIndexCompiler::new()
                .dict_dir(directory.path())
                .add_input_file(&source)
                .userdict_out_path(&output)
                .dictionary_charset(input_charset)
                .charset(output_charset)
                .run()
                .unwrap();
            let bytes = fs::read(output).unwrap();
            assert_eq!(
                &bytes[40..40 + output_charset.len()],
                output_charset.as_bytes()
            );
            assert!(bytes.ends_with(feature));
        }
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
