//! Haqumei と共通の辞書コンパイラーと形態素解析器です。

pub mod mecab;
#[cfg(feature = "compiler")]
pub mod mecab_compile;
pub mod tokenizer;
