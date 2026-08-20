unsafe extern "C" {
    pub(crate) fn setup_cpp_redirect();
    pub(crate) fn teardown_cpp_redirect();
}

/// This function is intended to be called from C code via FFI.
///
/// # Safety
///
/// The caller must ensure that:
/// - `msg` is a valid pointer to a null-terminated C string.
/// - The memory pointed to by `msg` is accessible and not modified concurrently during this call.
#[unsafe(no_mangle)]
unsafe extern "C" fn haqumei_rust_print(msg: *const libc::c_char, is_stderr: libc::c_int) {
    unsafe {
        if msg.is_null() {
            return;
        }
        let c_str = std::ffi::CStr::from_ptr(msg);
        let s = c_str.to_string_lossy();
        let s = s.trim_end();

        if is_stderr != 0 {
            log::warn!("[OpenJTalk] {}", s);
        } else {
            log::info!("[OpenJTalk] {}", s);
        }
    }
}

#[cfg(test)]
mod log_redirect_tests {
    use std::sync::{Mutex, OnceLock};

    static CAPTURED: OnceLock<Mutex<Vec<String>>> = OnceLock::new();

    struct Capture;

    impl log::Log for Capture {
        fn enabled(&self, _: &log::Metadata) -> bool {
            true
        }
        fn log(&self, record: &log::Record) {
            CAPTURED
                .get_or_init(Default::default)
                .lock()
                .unwrap()
                .push(format!("{} {}", record.level(), record.args()));
        }
        fn flush(&self) {}
    }

    /// vendor 側の出力が `log` に届くこと。
    ///
    /// `redirect.h` の `#define` が `printf` / `fprintf` を `haqumei_redirect_*` に
    /// 置き換え、そこからこの関数に来る。経路が切れてもビルドもテストも通って
    /// しまい、診断だけが黙って消えるので、ここで見張る。
    #[test]
    fn test_c_output_reaches_log() {
        CAPTURED.get_or_init(Default::default);
        static LOGGER: Capture = Capture;
        let _ = log::set_logger(&LOGGER);
        log::set_max_level(log::LevelFilter::Trace);

        let out = std::ffi::CString::new("from-c 1\n").unwrap();
        let err = std::ffi::CString::new("from-c 2\n").unwrap();

        // SAFETY: どちらも終端付きの有効な C 文字列を指す。
        unsafe {
            super::haqumei_rust_print(out.as_ptr(), 0);
            super::haqumei_rust_print(err.as_ptr(), 1);
        }

        let captured = CAPTURED.get().unwrap().lock().unwrap().clone();
        assert!(
            captured.contains(&"INFO [OpenJTalk] from-c 1".to_string()),
            "stdout 側が log に届いていない: {captured:?}"
        );
        assert!(
            captured.contains(&"WARN [OpenJTalk] from-c 2".to_string()),
            "stderr 側が log に届いていない: {captured:?}"
        );
    }
}
