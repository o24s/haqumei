#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NjdFeature {
    /// 表層形
    pub string: String,
    /// 品詞
    pub pos: String,
    /// 品詞細分類1
    pub pos_group1: String,
    /// 品詞細分類2
    pub pos_group2: String,
    /// 品詞細分類3
    pub pos_group3: String,
    /// 活用型
    pub ctype: String,
    /// 活用形
    pub cform: String,
    /// 原形
    pub orig: String,
    /// 読み
    pub read: String,
    /// 発音
    pub pron: String,
    /// アクセント核の位置
    pub acc: i32,
    /// モーラ数
    pub mora_size: i32,
    /// 連結規則
    pub chain_rule: String,
    /// 連結フラグ
    pub chain_flag: i32,
}
