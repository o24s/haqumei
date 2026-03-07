use ort::{
    session::Session,
    value::{Tensor, TensorRef, Value},
};

use crate::NjdFeature;

const ENC_MODEL_BYTES: &[u8] = include_bytes!("../yomi_model/nani_enc.onnx");
const MODEL_BYTES: &[u8] = include_bytes!("../yomi_model/nani_model.onnx");

pub struct NaniPredictor {
    enc_session: Session,
    model_session: Session,
}

impl NaniPredictor {
    pub fn new() -> ort::Result<Self> {
        let enc_session = Session::builder()?.commit_from_memory(ENC_MODEL_BYTES)?;

        let model_session = Session::builder()?.commit_from_memory(MODEL_BYTES)?;

        Ok(Self {
            enc_session,
            model_session,
        })
    }

    pub fn predict_is_nan(&mut self, prev_node: Option<&NjdFeature>) -> bool {
        match self.run_inference(prev_node) {
            Ok(prediction) => prediction == 1,
            Err(e) => {
                log::error!("Nani prediction inference failed: {}", e);
                false
            }
        }
    }

    fn run_inference(&mut self, prev_node: Option<&NjdFeature>) -> ort::Result<i64> {
        let njd = match prev_node {
            Some(node) => node,
            None => return Ok(0),
        };

        let features: [String; 6] = [
            njd.pos.to_string(),
            njd.pos_group1.to_string(),
            njd.pos_group2.to_string(),
            njd.pron.to_string(),
            njd.ctype.to_string(),
            njd.cform.to_string(),
        ];

        let shape = [1, 6];
        let tensor = Tensor::from_string_array((shape, features.as_slice()))?;
        let input_value: Value = tensor.into();
        let enc_inputs = ort::inputs!["input" => input_value];

        let enc_outputs = self.enc_session.run(enc_inputs)?;

        let (shape, data) = enc_outputs[0].try_extract_tensor::<f32>()?;

        let shape_vec: Vec<usize> = shape.iter().map(|&d| d as usize).collect();
        let enc_tensor_ref = TensorRef::from_array_view((shape_vec, data))?;

        let model_inputs = ort::inputs!["input" => enc_tensor_ref];

        let model_outputs = self.model_session.run(model_inputs)?;
        let (_, prediction_data) = model_outputs[0].try_extract_tensor::<i64>()?;

        Ok(prediction_data.first().copied().unwrap_or(0))
    }
}
