#![cfg(feature = "tokenizer")]

use haqumei_jpreprocess::{JPreprocess, SystemDictionaryConfig};
use haqumei_jpreprocess_core::token::Tokenizer;
use haqumei_jpreprocess_dictionary::mecab_compile::{BuildOptions, build_system, build_user};
use std::path::Path;

#[test]
fn system_and_user_dictionaries_share_the_haqumei_backend() {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data");
    let temporary = tempfile::tempdir().unwrap();
    let system = temporary.path().join("system");
    let user = temporary.path().join("user.dic");
    build_system(
        &data.join("min-dict-src"),
        &system,
        &BuildOptions::default(),
    )
    .unwrap();
    build_user(
        &data.join("min-dict-src"),
        &[data.join("user/src.csv")],
        &user,
    )
    .unwrap();
    for users in [vec![], vec![user]] {
        let model = SystemDictionaryConfig::File(system.clone())
            .load_with_user_dictionaries(&users)
            .unwrap();
        let engine = JPreprocess::from_tokenizer(model.clone());
        for text in [
            "日本語文を解析し、音声合成エンジンに渡せる形式に変換します．",
            "日本",
            "キログラム",
            "生麦生米生卵",
        ] {
            let normalized = haqumei_jpreprocess::normalize_text_for_naist_jdic(text);
            let njd = haqumei_jpreprocess::NJD::from_tokens(model.tokenize(&normalized).unwrap())
                .unwrap();
            assert_eq!(engine.text_to_njd(text).unwrap(), njd);
            let labels = engine.extract_fullcontext(text).unwrap();
            if text.starts_with("日本語文") {
                assert_eq!(
                    labels[2].to_string(),
                    concat!(
                        "sil^n-i+h=o/A:-3+1+7/B:xx-xx_xx/C:02_xx+xx/D:02+xx_xx",
                        "/E:xx_xx!xx_xx-xx/F:7_4#0_0@1_3|1_12/G:4_4%0_0_1",
                        "/H:xx_xx/I:3-12@1+2&1-8|1+41/J:5_29/K:2+8-41"
                    )
                );
            }
            if text.starts_with("日本語文") || !users.is_empty() {
                assert!(labels.len() > 2, "{text}");
                assert_eq!(labels[0].phoneme.c.as_deref(), Some("sil"));
            }
        }
        if !users.is_empty() {
            assert!(
                model
                    .analyze("生麦生米生卵")
                    .unwrap()
                    .nodes
                    .iter()
                    .any(|node| node.dictionary_index == 1)
            );
        }
    }
}

#[test]
fn tokenizer_is_send_and_sync() {
    fn check<T: Send + Sync>() {}
    check::<JPreprocess<haqumei_jpreprocess_dictionary::mecab::Model>>();
}
