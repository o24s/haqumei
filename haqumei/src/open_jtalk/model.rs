use std::ffi::{CString, c_char};
use std::path::{Path, PathBuf};

use crate::{errors::HaqumeiError, ffi};

#[derive(Debug)]
pub(crate) struct MecabModel {
    pub(crate) ptr: *mut ffi::mecab_model_t,
}

/// 辞書のパスを、Mecab の C API に渡せる [CString] に変換する。
///
/// ここはプラットフォーム差を吸収する場所で、次の 2 点を扱う。
///
/// - 非 UTF-8 のパス: Unix ではパスは任意のバイト列なので、`to_str()` で
///   文字列に変換すると失敗しうる。[OsStr] のバイト列をそのまま渡す。
/// - Windows の拡張長パス: [Path::canonicalize] は `\\?\C:\...` 形式を返すが、
///   Mecab の C 側は素の `fopen` でこれを扱えない可能性があるため、接頭辞を外す。
fn to_ffi_path(path: &Path) -> Result<CString, HaqumeiError> {
    let invalid = || HaqumeiError::InvalidDictionaryPath(path.to_string_lossy().into_owned());

    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;

        CString::new(path.as_os_str().as_bytes()).map_err(|_| invalid())
    }

    #[cfg(not(unix))]
    {
        let s = path.to_str().ok_or_else(invalid)?;
        let s = s.strip_prefix(r"\\?\").unwrap_or(s);

        CString::new(s).map_err(|_| invalid())
    }
}

impl MecabModel {
    /// システム辞書のディレクトリと、0 個以上のユーザー辞書から Mecab のモデルを作る。
    ///
    /// パスは呼び出し側で絶対パスに解決済みで、存在も確かめてあることを期待する。
    /// ([`crate::open_jtalk::Dictionary::from_paths`] を参照)
    ///
    /// Mecab は `-u` にカンマ区切りで複数のユーザー辞書を取る。連結を
    /// `to_ffi_path` のあと、つまり Windows の `\\?\` を落としたあとに行う。
    /// 先に連結して 1 つのパスとして扱うと、2 つ目以降の接頭辞が残ってしまう。
    pub fn new(dict_dir: &Path, user_dicts: &[PathBuf]) -> Result<Self, HaqumeiError> {
        let arg0 = CString::new("mecab").unwrap();
        let arg_d = CString::new("-d").unwrap();
        let c_dict_dir = to_ffi_path(dict_dir)?;
        let arg_u = CString::new("-u").unwrap();
        // 空のパスを渡すと Mecab 側が読み込みに失敗するため、除外する
        let mut parts = Vec::with_capacity(user_dicts.len());
        for path in user_dicts.iter().filter(|p| !p.as_os_str().is_empty()) {
            parts.push(to_ffi_path(path)?);
        }
        let c_user_dict = if parts.is_empty() {
            None
        } else {
            let joined: Vec<u8> = parts
                .iter()
                .map(|c| c.as_bytes())
                .collect::<Vec<_>>()
                .join(&b',');
            Some(CString::new(joined).map_err(|e| {
                HaqumeiError::InvalidDictionaryPath(format!("ユーザー辞書のパスが不正です: {e}"))
            })?)
        };

        let mut argv: Vec<*mut c_char> = vec![
            arg0.as_ptr() as *mut _,
            arg_d.as_ptr() as *mut _,
            c_dict_dir.as_ptr() as *mut _,
        ];

        if let Some(user_dict) = &c_user_dict {
            argv.push(arg_u.as_ptr() as *mut _);
            argv.push(user_dict.as_ptr() as *mut _);
        }

        // SAFETY: argv の各要素は、この呼び出しが返るまで生存する CString を指す。
        let model_ptr = unsafe { ffi::mecab_model_new(argv.len() as i32, argv.as_mut_ptr()) };

        if model_ptr.is_null() {
            Err(HaqumeiError::MecabLoadError)
        } else {
            Ok(Self { ptr: model_ptr })
        }
    }

    #[allow(unused)]
    pub(crate) fn new_uninitialized() -> Self {
        Self {
            ptr: std::ptr::null_mut(),
        }
    }

    pub(crate) fn is_initialized(&self) -> bool {
        !self.ptr.is_null()
    }
}

impl Drop for MecabModel {
    fn drop(&mut self) {
        unsafe {
            ffi::mecab_model_destroy(self.ptr);
        }
    }
}
