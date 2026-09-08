use haqumei_jpreprocess_dictionary::mecab::{Analysis, Model};

use crate::{errors::HaqumeiError, open_jtalk::model::MecabModel};

#[derive(Debug)]
pub(crate) struct Mecab {
    model: Model,
}

impl Mecab {
    pub(crate) fn from_model(model: &MecabModel) -> Result<Self, HaqumeiError> {
        let model = model
            .model
            .clone()
            .ok_or(HaqumeiError::GlobalDictionaryNotInitialized)?;
        Ok(Self { model })
    }

    pub(crate) fn analyze(&self, text: &str) -> Result<Analysis, HaqumeiError> {
        self.model
            .analyze(text)
            .map_err(|error| HaqumeiError::MecabError(error.to_string()))
    }
}
