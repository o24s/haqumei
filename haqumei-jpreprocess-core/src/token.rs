use crate::{JPreprocessResult, word_entry::WordEntry};

pub trait Tokenizer {
    fn tokenize<'a>(&'a self, text: &'a str) -> JPreprocessResult<Vec<impl 'a + Token>>;
}

pub trait Token {
    fn fetch(&mut self) -> JPreprocessResult<(&str, WordEntry)>;
}

impl Token for (String, WordEntry) {
    fn fetch(&mut self) -> JPreprocessResult<(&str, WordEntry)> {
        let (string, entry) = self;
        Ok((string.as_str(), entry.to_owned()))
    }
}

/// vibrato-rkyv で解析した形態素を NJD の単語情報に変換します。
#[cfg(feature = "vibrato")]
impl Tokenizer for vibrato::tokenizer::Tokenizer {
    fn tokenize<'a>(&'a self, text: &'a str) -> JPreprocessResult<Vec<impl 'a + Token>> {
        let mut worker = self.new_worker();
        worker.reset_sentence(text);
        worker.tokenize();

        worker
            .token_iter()
            .map(|token| {
                let features = token.feature();
                let mut details = features.split(',').collect::<Vec<_>>();
                details.resize(12, "*");
                let entry = WordEntry::load(&details)?;

                Ok((token.surface().to_string(), entry))
            })
            .collect()
    }
}
