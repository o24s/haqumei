//! MeCab 互換辞書の解析結果を NJD の単語情報に変換します。

use crate::mecab::Model;
use haqumei_jpreprocess_core::{token::Tokenizer, word_entry::WordEntry, JPreprocessResult};

impl Tokenizer for Model {
    fn tokenize<'a>(
        &'a self,
        text: &'a str,
    ) -> JPreprocessResult<Vec<impl 'a + haqumei_jpreprocess_core::token::Token>> {
        let analysis = self.worker()?.analyze(text)?;
        analysis
            .best_path
            .iter()
            .map(|&index| {
                let node = &analysis.nodes[index];
                let mut details = node.feature.split(',').collect::<Vec<_>>();
                details.resize(12, "*");
                Ok((
                    text[node.byte_span.clone()].to_owned(),
                    WordEntry::load(&details)?,
                ))
            })
            .collect()
    }
}
