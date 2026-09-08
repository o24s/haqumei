use std::path::{Path, PathBuf};

use haqumei_jpreprocess_dictionary::mecab::Model;

use crate::errors::HaqumeiError;

#[derive(Debug)]
pub(crate) struct MecabModel {
    pub(crate) model: Option<Model>,
}

impl MecabModel {
    pub(crate) fn new(dict_dir: &Path, user_dicts: &[PathBuf]) -> Result<Self, HaqumeiError> {
        let model = Model::open(dict_dir, user_dicts).map_err(|_| HaqumeiError::MecabLoadError)?;
        Ok(Self { model: Some(model) })
    }

    #[allow(unused)]
    pub(crate) fn new_uninitialized() -> Self {
        Self { model: None }
    }

    pub(crate) fn is_initialized(&self) -> bool {
        self.model.is_some()
    }
}
