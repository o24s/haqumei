use thiserror::Error;

pub type Result<T> = std::result::Result<T, KanalizerError>;

#[derive(Debug, Error)]
pub enum KanalizerError {
    #[error("Input is empty")]
    EmptyInput,

    #[error("Invalid character found in input: '{0}'")]
    InvalidCharacter(char),

    #[error("Conversion did not finish within the maximum length")]
    IncompleteConversion,

    #[error("ONNX Runtime error: {0}")]
    OrtError(#[from] ort::Error),

    #[error("Shape error: {0}")]
    ShapeError(#[from] ndarray::ShapeError),

    #[error("Tensor extraction failed: {0}")]
    ExtractionError(String),
}
