use std::num::NonZeroUsize;

#[derive(Clone, Copy, Debug, Default)]
pub enum MaxLength {
    /// 自動で決定する
    #[default]
    Auto,
    /// 指定された固定長
    Fixed(NonZeroUsize),
}

#[derive(Clone, Debug, Default)]
pub enum Strategy {
    #[default]
    Greedy,
    TopK(StrategyTopK),
    TopP(StrategyTopP),
}

#[derive(Clone, Debug)]
pub struct StrategyTopK {
    pub k: usize,
}

impl Default for StrategyTopK {
    fn default() -> Self {
        Self { k: 3 }
    }
}

#[derive(Clone, Debug)]
pub struct StrategyTopP {
    pub top_p: f32,
    pub temperature: f32,
}

impl Default for StrategyTopP {
    fn default() -> Self {
        Self {
            top_p: 0.9,
            temperature: 1.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ConvertOptions {
    /// デコードの最大長
    pub max_length: MaxLength,
    /// デコード戦略
    pub strategy: Strategy,
    /// 入力に無効な文字が含まれている場合にエラーを返すかどうか
    /// falseの場合、無効な文字は無視されます
    pub error_on_invalid_input: bool,
    /// 変換が終了しなかった場合にエラーを返す
    pub error_on_incomplete: bool,
}

impl Default for ConvertOptions {
    fn default() -> Self {
        Self {
            max_length: MaxLength::default(),
            strategy: Strategy::default(),
            error_on_invalid_input: true,
            error_on_incomplete: true,
        }
    }
}
