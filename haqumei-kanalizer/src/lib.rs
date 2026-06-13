pub mod constants;
pub mod error;
pub mod options;
mod utils;

pub use error::{KanalizerError, Result};
pub use options::{ConvertOptions, MaxLength, Strategy, StrategyTopK, StrategyTopP};

use ndarray::{Array2, Array3, ArrayView3};
use ort::{session::Session, value::TensorRef};
use rand::RngExt as _;

use crate::constants::KANAS;

const ENCODER_MODEL_BYTES: &[u8] = include_bytes!("../model/kanalizer_encoder.onnx");
const DECODER_MODEL_BYTES: &[u8] = include_bytes!("../model/kanalizer_decoder_step.onnx");

pub struct Kanalizer {
    encoder: Session,
    decoder: Session,
}

impl Kanalizer {
    pub fn new() -> Result<Self> {
        let encoder = Session::builder()?
            .with_intra_threads(1)?
            .with_inter_threads(1)?
            .commit_from_memory(ENCODER_MODEL_BYTES)?;

        let decoder = Session::builder()?
            .with_intra_threads(1)?
            .with_inter_threads(1)?
            .commit_from_memory(DECODER_MODEL_BYTES)?;

        Ok(Self::new_inner(encoder, decoder))
    }

    pub fn from_paths(enc_path: &str, dec_path: &str) -> Result<Self> {
        let encoder = Session::builder()?
            .with_intra_threads(1)?
            .with_inter_threads(1)?
            .commit_from_file(enc_path)?;

        let decoder = Session::builder()?
            .with_intra_threads(1)?
            .with_inter_threads(1)?
            .commit_from_file(dec_path)?;

        Ok(Self::new_inner(encoder, decoder))
    }

    fn new_inner(encoder: Session, decoder: Session) -> Self {
        Self { encoder, decoder }
    }

    pub fn convert(&mut self, input: &str) -> Result<String> {
        self.convert_with_options(input, &ConvertOptions::default())
    }

    pub fn convert_with_options(
        &mut self,
        input: &str,
        options: &ConvertOptions,
    ) -> Result<String> {
        if input.is_empty() {
            return Err(KanalizerError::EmptyInput);
        }

        let mut source = Vec::with_capacity(input.len() + 2);
        source.push(constants::SOS_IDX as i64);

        for c in input.chars() {
            if let Some(idx) = utils::get_ascii_index(c) {
                source.push(idx);
            } else if options.error_on_invalid_input {
                return Err(KanalizerError::InvalidCharacter(c));
            }
        }
        source.push(constants::EOS_IDX as i64);

        let src_len = source.len();
        let src_array = Array2::from_shape_vec((1, src_len), source)?;

        let tensor_ref = TensorRef::from_array_view(src_array.view())
            .map_err(|e| KanalizerError::ExtractionError(e.to_string()))?;
        let enc_inputs = ort::inputs!["src" => tensor_ref];

        let enc_out = {
            let enc_outputs = self.encoder.run(enc_inputs)?;
            let enc_tuple = enc_outputs["enc_out"]
                .try_extract_tensor::<f32>()
                .map_err(|e| KanalizerError::ExtractionError(e.to_string()))?;

            let seq_len = enc_tuple.1.len() / constants::DIM;

            ArrayView3::from_shape((1, seq_len, constants::DIM), enc_tuple.1)?.to_owned()
        };

        let decoding_max_length = match options.max_length {
            MaxLength::Auto => src_len + 2,
            MaxLength::Fixed(len) => len.get(),
        };

        let mut result = vec![constants::SOS_IDX as i64];
        let mut h1 = Array3::<f32>::zeros((1, 1, constants::DIM));
        let mut h2 = Array3::<f32>::zeros((1, 1, constants::DIM));
        let mut dec_input = Array2::<i64>::zeros((1, 1));

        let mut finished = false;

        for i in 0..decoding_max_length {
            dec_input[[0, 0]] = *result.last().unwrap();

            let dec_input_t = TensorRef::from_array_view(dec_input.view())
                .map_err(|e| KanalizerError::ExtractionError(e.to_string()))?;
            let enc_out_t = TensorRef::from_array_view(enc_out.view())
                .map_err(|e| KanalizerError::ExtractionError(e.to_string()))?;
            let h1_t = TensorRef::from_array_view(h1.view())
                .map_err(|e| KanalizerError::ExtractionError(e.to_string()))?;
            let h2_t = TensorRef::from_array_view(h2.view())
                .map_err(|e| KanalizerError::ExtractionError(e.to_string()))?;

            let dec_inputs = ort::inputs![
                "dec_input" => dec_input_t,
                "enc_out" => enc_out_t,
                "h1" => h1_t,
                "h2" => h2_t
            ];

            let dec_outputs = self.decoder.run(dec_inputs)?;

            let h1_tuple = dec_outputs["h1_new"]
                .try_extract_tensor::<f32>()
                .map_err(|e| KanalizerError::ExtractionError(e.to_string()))?;
            h1.assign(&ArrayView3::from_shape((1, 1, constants::DIM), h1_tuple.1)?);

            let h2_tuple = dec_outputs["h2_new"]
                .try_extract_tensor::<f32>()
                .map_err(|e| KanalizerError::ExtractionError(e.to_string()))?;
            h2.assign(&ArrayView3::from_shape((1, 1, constants::DIM), h2_tuple.1)?);

            let logits_tuple = dec_outputs["logits"]
                .try_extract_tensor::<f32>()
                .map_err(|e| KanalizerError::ExtractionError(e.to_string()))?;
            let logits_slice = logits_tuple.1;

            let next_token = Self::decode(logits_slice, &options.strategy, i == 0);

            result.push(next_token);
            if next_token == constants::EOS_IDX as i64 {
                finished = true;
                break;
            }
        }

        if !finished && options.error_on_incomplete {
            return Err(KanalizerError::IncompleteConversion);
        }

        let mut output_str = String::new();
        for &token in result.iter().skip(1) {
            if token == constants::EOS_IDX as i64 {
                break;
            }
            if let Some(kana) = KANAS.get(token as usize).copied() {
                output_str.push_str(kana);
            }
        }

        Ok(output_str)
    }

    #[inline(always)]
    fn decode(logits: &[f32], strategy: &Strategy, is_first: bool) -> i64 {
        match strategy {
            Strategy::Greedy => Self::greedy(logits, is_first) as i64,
            Strategy::TopK(StrategyTopK { k }) => Self::top_k(logits, *k, is_first) as i64,
            Strategy::TopP(StrategyTopP { top_p, temperature }) => {
                Self::top_p(logits, *top_p, *temperature, is_first) as i64
            }
        }
    }

    #[inline(always)]
    fn greedy(step_dec: &[f32], is_first: bool) -> usize {
        let mut max = f32::NEG_INFINITY;
        let mut argmax = 0;

        for (i, &v) in step_dec.iter().enumerate() {
            if is_first && i == constants::EOS_IDX {
                continue;
            }

            if v > max {
                max = v;
                argmax = i;
            }
        }

        argmax
    }

    #[inline(always)]
    fn top_k(step_dec: &[f32], k: usize, is_first: bool) -> usize {
        let mut rng = rand::rng();

        let mut indices: Vec<usize> = (0..step_dec.len())
            .filter(|&i| !(is_first && i == constants::EOS_IDX))
            .collect();

        indices.sort_unstable_by(|&i, &j| step_dec[j].partial_cmp(&step_dec[i]).unwrap());

        indices.truncate(k.max(1));

        let max_logit = indices
            .iter()
            .map(|&i| step_dec[i])
            .fold(f32::NEG_INFINITY, f32::max);

        let weights: Vec<f32> = indices
            .iter()
            .map(|&i| (step_dec[i] - max_logit).exp())
            .collect();

        let total: f32 = weights.iter().sum();

        let mut r = rng.random::<f32>() * total;

        for (idx, w) in indices.iter().zip(weights.iter()) {
            r -= *w;
            if r <= 0.0 {
                return *idx;
            }
        }

        *indices.last().unwrap()
    }

    #[inline(always)]
    fn top_p(step_dec: &[f32], top_p: f32, temperature: f32, is_first: bool) -> usize {
        let mut rng = rand::rng();

        let max_logit = step_dec.iter().copied().fold(f32::NEG_INFINITY, f32::max);

        let exp_logits: Vec<f32> = step_dec
            .iter()
            .enumerate()
            .map(|(i, &x)| {
                if is_first && i == constants::EOS_IDX {
                    0.0
                } else {
                    ((x - max_logit) / temperature).exp()
                }
            })
            .collect();

        let sum: f32 = exp_logits.iter().sum();

        let probs: Vec<f32> = exp_logits.iter().map(|x| x / sum).collect();

        let mut sorted: Vec<(usize, f32)> = probs.iter().copied().enumerate().collect();

        sorted.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let mut cutoff = 0;
        let mut cumulative = 0.0;

        while cutoff < sorted.len() && cumulative < top_p {
            cumulative += sorted[cutoff].1;
            cutoff += 1;
        }

        cutoff = cutoff.max(1);

        let candidates = &sorted[..cutoff];

        let total: f32 = candidates.iter().map(|(_, p)| *p).sum();

        let mut r = rng.random::<f32>() * total;

        for (idx, p) in candidates {
            r -= *p;
            if r <= 0.0 {
                return *idx;
            }
        }

        candidates.last().unwrap().0
    }
}
