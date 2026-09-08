use haqumei_jpreprocess_dictionary::mecab::{Analysis, Worker};

use crate::{errors::HaqumeiError, open_jtalk::model::MecabModel};

#[derive(Debug)]
pub(crate) struct Mecab {
    worker: Worker,
}

impl Mecab {
    pub(crate) fn from_model(model: &MecabModel) -> Result<Self, HaqumeiError> {
        let model = model
            .model
            .clone()
            .ok_or(HaqumeiError::GlobalDictionaryNotInitialized)?;
        let worker = model
            .worker()
            .map_err(|e| HaqumeiError::MecabError(e.to_string()))?;
        Ok(Self { worker })
    }

    pub(crate) fn analyze(&mut self, text: &str) -> Result<Analysis, HaqumeiError> {
        self.worker
            .analyze(text)
            .map_err(|e| HaqumeiError::MecabError(e.to_string()))
    }

    pub(crate) fn analyze_lattice(&mut self, text: &str) -> Result<Analysis, HaqumeiError> {
        self.worker
            .analyze_lattice(text)
            .map_err(|error| HaqumeiError::MecabError(error.to_string()))
    }
}
