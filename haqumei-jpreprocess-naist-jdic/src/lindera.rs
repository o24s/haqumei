use lindera_dictionary::{
    dictionary::{
        character_definition::CharacterDefinition, connection_cost_matrix::ConnectionCostMatrix,
        metadata::Metadata, prefix_dictionary::PrefixDictionary,
        unknown_dictionary::UnknownDictionary, Dictionary,
    },
    LinderaResult,
};

const METADATA_DATA: &[u8] = include_bytes!(concat!(env!("JPREPROCESS_WORKDIR"), "/metadata.json"));

const CHAR_DEFINITION_DATA: &[u8] =
    include_bytes!(concat!(env!("JPREPROCESS_WORKDIR"), "/char_def.bin"));
const CONNECTION_DATA: &[u8] = include_bytes!(concat!(env!("JPREPROCESS_WORKDIR"), "/matrix.mtx"));
const IPADIC_DATA: &[u8] = include_bytes!(concat!(env!("JPREPROCESS_WORKDIR"), "/dict.trie"));
const IPADIC_VALS_IDX: &[u8] =
    include_bytes!(concat!(env!("JPREPROCESS_WORKDIR"), "/dict.valsidx"));
const IPADIC_VALS: &[u8] = include_bytes!(concat!(env!("JPREPROCESS_WORKDIR"), "/dict.vals"));
const UNKNOWN_DATA: &[u8] = include_bytes!(concat!(env!("JPREPROCESS_WORKDIR"), "/unk.bin"));
const WORDS_IDX_DATA: &[u8] =
    include_bytes!(concat!(env!("JPREPROCESS_WORKDIR"), "/dict.wordsidx"));
const WORDS_DATA: &[u8] = include_bytes!(concat!(env!("JPREPROCESS_WORKDIR"), "/dict.words"));

pub fn load() -> LinderaResult<Dictionary> {
    Ok(Dictionary {
        metadata: Metadata::load(METADATA_DATA)?.into(),
        prefix_dictionary: PrefixDictionary::load(
            IPADIC_DATA,
            IPADIC_VALS_IDX,
            IPADIC_VALS,
            WORDS_IDX_DATA,
            WORDS_DATA,
        )?
        .into(),
        connection_cost_matrix: ConnectionCostMatrix::load(CONNECTION_DATA)?.into(),
        character_definition: CharacterDefinition::load(CHAR_DEFINITION_DATA)?.into(),
        unknown_dictionary: UnknownDictionary::load(UNKNOWN_DATA)?.into(),
    })
}
