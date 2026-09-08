use haqumei_jpreprocess_core::word_line::WordDetailsLine;
use lindera_dictionary::{error::LinderaErrorKind, LinderaResult};

/// A trait for encoding and decoding as dictionary entry.
pub trait DictionaryWordEncoding: Sized {
    fn identifier() -> &'static str;
    fn encode(row: WordDetailsLine) -> LinderaResult<Vec<u8>>;
}

pub struct JPreprocessDictionaryWordEncoding;
impl JPreprocessDictionaryWordEncoding {
    /// 辞書エントリを rkyv 形式で保存します。
    pub fn serialize(
        data: &haqumei_jpreprocess_core::word_entry::WordEntry,
    ) -> Result<Vec<u8>, rkyv::rancor::Error> {
        Ok(rkyv::to_bytes::<rkyv::rancor::Error>(data)?.to_vec())
    }

    /// バイト列を検査し、辞書エントリを復元します。
    pub fn deserialize(
        data: &[u8],
    ) -> Result<haqumei_jpreprocess_core::word_entry::WordEntry, rkyv::rancor::Error> {
        // 辞書内のエントリ境界はアラインメントを保証しないため、整列した領域にコピーする。
        let mut aligned = rkyv::util::AlignedVec::<16>::with_capacity(data.len());
        aligned.extend_from_slice(data);
        rkyv::from_bytes::<_, rkyv::rancor::Error>(&aligned)
    }
}
impl DictionaryWordEncoding for JPreprocessDictionaryWordEncoding {
    fn identifier() -> &'static str {
        "haqumei_jpreprocess rkyv-0.8-v1"
    }

    fn encode(row: WordDetailsLine) -> LinderaResult<Vec<u8>> {
        let data = row
            .try_into()
            .map_err(|err| LinderaErrorKind::Serialize.with_error(err))?;
        Self::serialize(&data).map_err(|err| LinderaErrorKind::Serialize.with_error(err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use haqumei_jpreprocess_core::{
        pronunciation::Pronunciation, word_details::WordDetails, word_entry::WordEntry,
    };

    #[test]
    fn roundtrip_preserves_all_word_details_at_unaligned_offsets() {
        let mut details = WordDetails::load(&[
            "名詞",
            "固有名詞",
            "一般",
            "*",
            "*",
            "*",
            "試験",
            "シケン",
            "シケン",
            "1/3",
            "名詞%C2/動詞%F1",
            "1",
        ])
        .unwrap();
        details.pos_original = Some((details.pos, "名詞,固有名詞,一般,補足".into()));
        let mut borrowed = details.clone();
        borrowed.pron = Pronunciation::default();
        for entry in [
            WordEntry::Single(details.clone()),
            WordEntry::Multiple(vec![("試験".into(), details), ("".into(), borrowed)]),
            WordEntry::default(),
        ] {
            let encoded = JPreprocessDictionaryWordEncoding::serialize(&entry).unwrap();
            for offset in 0..16 {
                let mut bytes = vec![0u8; offset];
                bytes.extend_from_slice(&encoded);
                assert_eq!(
                    JPreprocessDictionaryWordEncoding::deserialize(&bytes[offset..]).unwrap(),
                    entry
                );
            }
        }
    }

    #[test]
    fn malformed_archives_return_errors() {
        let encoded = JPreprocessDictionaryWordEncoding::serialize(&WordEntry::default()).unwrap();
        for bytes in [&[][..], &[0xff; 512][..], &encoded[..encoded.len() / 2]] {
            assert!(JPreprocessDictionaryWordEncoding::deserialize(bytes).is_err());
        }
    }
}
