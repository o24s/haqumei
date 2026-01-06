
#[cfg(test)]
mod tests {
    use std::{fs, panic::{self, AssertUnwindSafe}, path::{Path, PathBuf}, sync::LazyLock};

    use haqumei::{Haqumei, ParallelJTalk};

    static MANIFEST_DIR: LazyLock<&Path> = LazyLock::new(|| Path::new(env!("CARGO_MANIFEST_DIR")));
    static WAGANEKO_PATH: LazyLock<PathBuf> = LazyLock::new(|| MANIFEST_DIR.join("../resources/waganeko.txt"));

    #[test]
    fn test_g2p_mapping_detailed_full() {
        let waganeko = fs::read_to_string(WAGANEKO_PATH.as_path()).unwrap();
        let waganeko: Vec<&str> = waganeko.lines().collect();

        let pojt = ParallelJTalk::new().unwrap();
        pojt.g2p_mapping_detailed(&waganeko).unwrap();

        let mut haqumei = Haqumei::new().unwrap();

        for text in waganeko {
            let res = panic::catch_unwind(AssertUnwindSafe(|| {
                haqumei.g2p_mapping_detailed(text).unwrap();
            }));

            if res.is_err() {
                panic!("failed for input: {:?}", text);
            }
        }
    }
}