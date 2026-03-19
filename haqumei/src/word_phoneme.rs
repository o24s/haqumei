#[derive(Debug, Clone, PartialEq)]
pub struct WordPhonemePair {
    pub word: String,
    pub phonemes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WordPhonemeMap {
    pub word: String,
    pub phonemes: Vec<String>,

    /// 元となった形態素について、MeCab が未知語 (`MECAB_UNK_NODE`) と判定したかどうか。
    ///
    /// NJDの処理によって複数の形態素が結合された場合は、その中に1つでも未知語が含まれていれば `true` となる。
    pub is_unknown: bool,

    /// `pyopenjtalk` のパイプラインで無視される対象 ("記号,空白") として空白 (`sp`) に置き換えられたか、
    /// または NJD/JPCommon の処理結果として音素が1つも割り当てられなかったかどうか。
    ///
    /// (e.g., 先頭の `ー` など、他の形態素に長音として吸収されず破棄されたケース)
    pub is_ignored: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WordPhonemeDetail {
    /// 表層形 (surface)
    pub word: String,
    pub phonemes: Vec<String>,

    /// Mecab が出力した features。
    /// 既知語は 12 列、未知語は 8 列 (read, pron, acc, chain_rule がない)
    pub features: Vec<String>,

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
    /// 発音形式
    pub pron: String,

    /// アクセント核位置 (0: 平板型, 1-n: n番目のモーラにアクセント核)
    pub accent_nucleus: i32,
    /// モーラ数
    pub mora_count: i32,
    /// アクセント結合規則 (C1-C5/F1-F5/P1-P2 等)
    pub chain_rule: String,
    /// アクセント句連結フラグ
    pub chain_flag: i32,

    /// 元となった形態素について、MeCab が未知語 (`MECAB_UNK_NODE`) と判定したかどうか。
    ///
    /// NJDの処理によって複数の形態素が結合された場合は、その中に1つでも未知語が含まれていれば `true` となる。
    pub is_unknown: bool,

    /// `pyopenjtalk` のパイプラインで無視される対象 ("記号,空白") として空白 (`sp`) に置き換えられたか、
    /// または NJD/JPCommon の処理結果として音素が1つも割り当てられなかったかどうか。
    ///
    /// (e.g., 先頭の `ー` など、他の形態素に長音として吸収されず破棄されたケース)
    pub is_ignored: bool,
}

pub(crate) trait WordPhonemeEntry {
    fn phonemes_mut(&mut self) -> &mut Vec<String>;
    fn phonemes(&self) -> &[String];

    /// 他の要素が空音素としてマージされる際に、テキストや付随情報を自身に結合する
    fn merge_from(&mut self, other: &mut Self);
}

impl WordPhonemeEntry for WordPhonemePair {
    fn phonemes_mut(&mut self) -> &mut Vec<String> {
        &mut self.phonemes
    }
    fn phonemes(&self) -> &[String] {
        &self.phonemes
    }

    fn merge_from(&mut self, other: &mut Self) {
        let text_to_merge = std::mem::take(&mut other.word);
        self.word.push_str(&text_to_merge);
    }
}

impl WordPhonemeEntry for WordPhonemeDetail {
    fn phonemes_mut(&mut self) -> &mut Vec<String> {
        &mut self.phonemes
    }
    fn phonemes(&self) -> &[String] {
        &self.phonemes
    }

    fn merge_from(&mut self, other: &mut Self) {
        let text_to_merge = std::mem::take(&mut other.word);
        self.word.push_str(&text_to_merge);

        self.mora_count += other.mora_count;

        // orig は辞書の原形を表すため、活用形の吸収では連結しないが、
        // リテラルの長音記号 ("ー") が吸収された場合は入力テキストを保持するため連結する
        if !other.orig.is_empty() && other.orig.chars().all(|c| c == 'ー') {
            let orig_to_merge = std::mem::take(&mut other.orig);
            self.orig.push_str(&orig_to_merge);
        }

        let read_to_merge = std::mem::take(&mut other.read);
        self.read.push_str(&read_to_merge);

        let pron_to_merge = std::mem::take(&mut other.pron);
        self.pron.push_str(&pron_to_merge);
    }
}
