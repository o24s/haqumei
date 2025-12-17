mod ffi {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(dead_code)]
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}
mod data;
mod errors;
pub mod features;
mod nani_predict;
pub mod open_jtalk;
mod utils;

use std::{path::PathBuf, sync::LazyLock};

use moka::sync::Cache;
pub use {open_jtalk::{OpenJTalk, ParallelJTalk}, features::NjdFeature};

use vibrato_rkyv::dictionary::PresetDictionaryKind;

use crate::{
    errors::HaqumeiError,
    features::UnidicFeature,
    nani_predict::NaniPredictor,
    utils::{modify_acc_after_chaining, modify_filler_accent, process_odori_features, retreat_acc_nuc, vibrato_analysis},
};

static VIBRATO_CACHE: LazyLock<Cache<String, Vec<UnidicFeature>>> = LazyLock::new(|| Cache::new(1000));

#[allow(unused)]
pub struct Haqumei {
    open_jtalk: OpenJTalk,
    tokenizer: vibrato_rkyv::Tokenizer,
    data_dir: PathBuf,
    predictor: NaniPredictor,
}

impl Haqumei {
    pub fn new() -> Result<Self, HaqumeiError> {
        let open_jtalk = OpenJTalk::new()?;

        let Some(data_dir) = dirs::data_local_dir().map(|dir| dir.join("haqumei")) else {
            Err(HaqumeiError::DataDirectoryNotFound)?
        };

        let vibrato_dict = vibrato_rkyv::Dictionary::from_preset_with_download(
            PresetDictionaryKind::UnidicCsj,
            &data_dir,
        )?;

        let tokenizer = vibrato_rkyv::Tokenizer::new(vibrato_dict);

        Ok(Haqumei {
            open_jtalk,
            data_dir,
            tokenizer,
            predictor: NaniPredictor::new()?,
        })
    }

    pub fn g2p(
        &mut self,
        text: &str,
        kana: bool,
    ) -> Result<String, HaqumeiError> {
        let features = self.run_frontend(text)?;

        if features.is_empty() {
            return Ok(String::new());
        }

        if !kana {
            let labels = self.open_jtalk.make_label(&features)?;

            // python: `lambda s: s.split("-")[1].split("+")[0]`
            let phonemes: Vec<_> = labels
                .iter()
                .skip(1)
                .take(labels.len() - 2)
                .filter_map(|s| {
                    s.split_once('-')
                    .and_then(|(_, after_minus)| after_minus.split_once('+'))
                    .map(|(p, _)| p)
                })
                .collect();

            Ok(phonemes.join(" "))
        } else {
            let kana_string: String = features
                .iter()
                .map(|f| {
                    let p = if f.pos == "記号" { &f.string } else { &f.pron };
                    p.replace('’', "")
                })
                .collect();
            Ok(kana_string)
        }
    }

    pub fn run_frontend(
        &mut self,
        text: &str,
    ) -> Result<Vec<NjdFeature>, HaqumeiError> {
        let (njd_features, _) = rayon::join(
            || OpenJTalk::new()?.run_frontend(text),
            || {
            let mut worker = self.tokenizer.new_worker();
            vibrato_analysis(&mut worker, text);
        });
        self.apply_postprocessing(text, njd_features?)
    }

    fn apply_postprocessing(
        &mut self,
        text: &str,
        mut njd_features: Vec<NjdFeature>,
    ) -> Result<Vec<NjdFeature>, HaqumeiError> {
        modify_filler_accent(&mut njd_features);
        self.modify_kanji_yomi(text, &mut njd_features);
        retreat_acc_nuc(&mut njd_features);
        modify_acc_after_chaining(&mut njd_features);
        process_odori_features(&mut njd_features, &mut self.open_jtalk)?;
        Ok(njd_features)
    }

    pub(crate) fn predict_is_nan(&mut self, prev_node: Option<&NjdFeature>) -> bool {
        self.predictor.predict_is_nan(prev_node)
    }
}
