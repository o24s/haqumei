use rustc_hash::FxHashMap;
use std::ops::Range;

#[allow(deprecated)]
use crate::WordPhonemePair;
use crate::errors::HaqumeiError;
use crate::ffi;
use crate::phoneme::Phoneme;
use crate::prosody::{PitchAccent, ProsodicPhoneme};
use crate::utils::has_odori_chars;
use crate::word_phoneme::WordPhonemeProsody;
use crate::{MecabMorph, OpenJTalk};
use crate::{NjdFeature, WordPhonemeDetail, WordPhonemeMap};

pub(crate) trait WordPhonemeEntry {
    fn phonemes_mut(&mut self) -> &mut Vec<Phoneme>;
    fn phonemes(&self) -> &[Phoneme];

    /// 他の要素が空音素としてマージされる際に、テキストや付随情報を自身に結合する
    fn merge_from(&mut self, other: &mut Self);
}

#[allow(deprecated)]
impl WordPhonemeEntry for WordPhonemePair {
    fn phonemes_mut(&mut self) -> &mut Vec<Phoneme> {
        &mut self.phonemes
    }
    fn phonemes(&self) -> &[Phoneme] {
        &self.phonemes
    }

    fn merge_from(&mut self, other: &mut Self) {
        let text_to_merge = std::mem::take(&mut other.word);
        self.word.push_str(&text_to_merge);
    }
}

impl WordPhonemeEntry for WordPhonemeDetail {
    fn phonemes_mut(&mut self) -> &mut Vec<Phoneme> {
        &mut self.phonemes
    }
    fn phonemes(&self) -> &[Phoneme] {
        &self.phonemes
    }

    fn merge_from(&mut self, other: &mut Self) {
        debug_assert!(
            other.phonemes.is_empty(),
            "phonemes should be empty when merging"
        );

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

        // 吸収した語のぶんだけ区間を伸ばす。`make_phoneme_mapping` は区間を入れ直す
        // ので、ここで伸ばした値が残るのは `assign_and_merge_phonemes` の結果を
        // そのまま読む経路だけである
        if other.char_span.end > self.char_span.end {
            self.char_span.end = other.char_span.end;
        }
    }
}

impl WordPhonemeProsody {
    /// この関数によってマージされるとき、phonemes は空のケースである。
    /// そのため、phonemes のマージを考える必要はない。
    pub(crate) fn merge_from(&mut self, other: &mut Self) {
        debug_assert!(
            other.phonemes.is_empty(),
            "phonemes should be empty when merging"
        );

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

        // 吸収した語のぶんだけ区間を伸ばす。`make_phoneme_mapping` は区間を入れ直す
        // ので、ここで伸ばした値が残るのは `assign_and_merge_phonemes` の結果を
        // そのまま読む経路だけである
        if other.char_span.end > self.char_span.end {
            self.char_span.end = other.char_span.end;
        }
    }
}

/// `assign_and_merge_phonemes` に音素を入れてもらうあいだだけ使う、
/// [`WordPhonemeMap`] を組み立てる前の形。
///
/// [`WordPhonemePair`] を使っていたが、[`WordPhonemeMap::char_span`] に入れる区間を
/// 持ち回れない。廃止予定の公開型にフィールドを足す代わりに、内部の型を分けてある。
pub(crate) struct WordPhonemeSeed {
    pub(crate) word: String,
    pub(crate) phonemes: Vec<Phoneme>,
    pub(crate) char_span: Range<usize>,
}

impl WordPhonemeEntry for WordPhonemeSeed {
    fn phonemes_mut(&mut self) -> &mut Vec<Phoneme> {
        &mut self.phonemes
    }
    fn phonemes(&self) -> &[Phoneme] {
        &self.phonemes
    }

    fn merge_from(&mut self, other: &mut Self) {
        let text_to_merge = std::mem::take(&mut other.word);
        self.word.push_str(&text_to_merge);
        if other.char_span.end > self.char_span.end {
            self.char_span.end = other.char_span.end;
        }
    }
}

impl IntoPhonemeMapItem for WordPhonemeSeed {
    type Output = WordPhonemeMap;

    #[inline]
    fn word(&self) -> &str {
        &self.word
    }

    #[inline]
    fn new_ignored(morph: &MecabMorph) -> Self::Output {
        WordPhonemeMap {
            word: morph.surface.clone(),
            phonemes: vec![Phoneme::Sp],
            is_unknown: morph.is_unknown,
            is_ignored: true,
            char_span: morph.char_span.clone(),
        }
    }

    #[inline]
    fn into_unmatched_remainder(self) -> Self::Output {
        let is_ignored = self.phonemes.is_empty();
        WordPhonemeMap {
            word: self.word,
            phonemes: self.phonemes,
            is_unknown: false,
            is_ignored,
            char_span: self.char_span,
        }
    }

    #[inline]
    fn into_exact_match(self, morph: &MecabMorph) -> Self::Output {
        // JPCommonが音素を割り当てなかったとき is_ignored にする
        let is_ignored = self.phonemes.is_empty();
        let mut phonemes = self.phonemes;

        if morph.is_unknown && (phonemes.is_empty() || phonemes == [Phoneme::Pau]) {
            phonemes = vec![Phoneme::Unk];
        }

        WordPhonemeMap {
            word: self.word,
            phonemes,
            is_unknown: morph.is_unknown,
            is_ignored,
            char_span: self.char_span,
        }
    }

    #[inline]
    fn into_prefix_match(self, is_unknown_word: bool) -> Self::Output {
        let mut phonemes = self.phonemes;
        let is_ignored = phonemes.is_empty();

        if is_unknown_word && (phonemes.is_empty() || phonemes == [Phoneme::Pau]) {
            phonemes = vec![Phoneme::Unk];
        }

        WordPhonemeMap {
            word: self.word,
            phonemes,
            is_unknown: is_unknown_word,
            is_ignored,
            char_span: self.char_span,
        }
    }

    #[inline]
    fn into_mismatch(self) -> Self::Output {
        let is_ignored = self.phonemes.is_empty();
        WordPhonemeMap {
            word: self.word,
            phonemes: self.phonemes,
            is_unknown: false,
            is_ignored,
            char_span: self.char_span,
        }
    }
}

pub(crate) trait IntoPhonemeMapItem: Sized {
    type Output;

    fn word(&self) -> &str;

    /// is_ignored な形態素用の出力を生成
    fn new_ignored(morph: &MecabMorph) -> Self::Output;

    /// morphs が尽きた場合の処理
    fn into_unmatched_remainder(self) -> Self::Output;

    /// 完全一致の場合の処理
    fn into_exact_match(self, morph: &MecabMorph) -> Self::Output;

    /// 先頭一致（結合）の場合の処理
    fn into_prefix_match(self, is_unknown_word: bool) -> Self::Output;

    /// 不一致の場合の処理
    fn into_mismatch(self) -> Self::Output;
}

impl IntoPhonemeMapItem for WordPhonemeDetail {
    type Output = WordPhonemeDetail;

    #[inline]
    fn word(&self) -> &str {
        &self.word
    }

    #[inline]
    fn new_ignored(morph: &MecabMorph) -> Self::Output {
        let surface = morph.surface.clone();
        WordPhonemeDetail {
            word: surface.clone(),
            phonemes: vec![Phoneme::Sp],
            features: Vec::new(),
            pos: "記号".to_string(),
            pos_group1: "空白".to_string(),
            pos_group2: "*".to_string(),
            pos_group3: "*".to_string(),
            ctype: "*".to_string(),
            cform: "*".to_string(),
            orig: surface.clone(),
            read: surface.clone(),
            pron: surface,
            accent_nucleus: 0,
            mora_count: 0,
            chain_rule: "*".to_string(),
            chain_flag: -1,
            is_unknown: morph.is_unknown,
            is_ignored: true,
            char_span: morph.char_span.clone(),
        }
    }

    #[inline]
    fn into_unmatched_remainder(mut self) -> Self::Output {
        self.is_ignored = self.phonemes.is_empty();
        self
    }

    #[inline]
    fn into_exact_match(mut self, morph: &MecabMorph) -> Self::Output {
        if morph.is_unknown && (self.phonemes.is_empty() || self.phonemes == [Phoneme::Pau]) {
            self.phonemes = vec![Phoneme::Unk];
        }
        self.is_unknown = morph.is_unknown;

        // JPCommonが音素を割り当てなかったとき is_ignored にする
        self.is_ignored = self.phonemes.is_empty();
        self.features = morph.feature.split(',').map(|s| s.to_string()).collect();
        self
    }

    #[inline]
    fn into_prefix_match(mut self, is_unknown_word: bool) -> Self::Output {
        if is_unknown_word && (self.phonemes.is_empty() || self.phonemes == [Phoneme::Pau]) {
            self.phonemes = vec![Phoneme::Unk];
        }
        self.is_unknown = is_unknown_word;
        self.is_ignored = self.phonemes.is_empty();
        self.features = Vec::new();
        self
    }

    #[inline]
    fn into_mismatch(mut self) -> Self::Output {
        self.is_unknown = false;
        self.is_ignored = self.phonemes.is_empty();
        self.features = Vec::new();
        self
    }
}

impl IntoPhonemeMapItem for WordPhonemeProsody {
    type Output = WordPhonemeProsody;

    #[inline]
    fn word(&self) -> &str {
        &self.word
    }

    #[inline]
    fn new_ignored(morph: &MecabMorph) -> Self::Output {
        let surface = morph.surface.clone();
        WordPhonemeProsody {
            word: surface.clone(),
            phonemes: vec![ProsodicPhoneme::sp()],
            pos: "記号".to_string(),
            pos_group1: "空白".to_string(),
            pos_group2: "*".to_string(),
            pos_group3: "*".to_string(),
            ctype: "*".to_string(),
            cform: "*".to_string(),
            orig: surface.clone(),
            read: surface.clone(),
            pron: surface,
            accent_nucleus: 0,
            mora_count: 0,
            chain_rule: "*".to_string(),
            chain_flag: -1,
            is_unknown: morph.is_unknown,
            is_ignored: true,
            char_span: morph.char_span.clone(),
        }
    }

    #[inline]
    fn into_unmatched_remainder(mut self) -> Self::Output {
        self.is_ignored = self.phonemes.is_empty();
        self
    }

    #[inline]
    fn into_exact_match(mut self, morph: &MecabMorph) -> Self::Output {
        if morph.is_unknown
            && (self.phonemes.is_empty() || self.phonemes == [ProsodicPhoneme::pau()])
        {
            self.phonemes = vec![ProsodicPhoneme::unk()];
        }
        self.is_unknown = morph.is_unknown;

        // JPCommonが音素を割り当てなかったとき is_ignored にする
        self.is_ignored = self.phonemes.is_empty();
        self
    }

    #[inline]
    fn into_prefix_match(mut self, is_unknown_word: bool) -> Self::Output {
        if is_unknown_word
            && (self.phonemes.is_empty() || self.phonemes == [ProsodicPhoneme::pau()])
        {
            self.phonemes = vec![ProsodicPhoneme::unk()];
        }
        self.is_unknown = is_unknown_word;
        self.is_ignored = self.phonemes.is_empty();
        self
    }

    #[inline]
    fn into_mismatch(mut self) -> Self::Output {
        self.is_unknown = false;
        self.is_ignored = self.phonemes.is_empty();
        self
    }
}

#[inline(always)]
pub(super) fn consume_odori_morphs(
    morphs: &[MecabMorph],
    morph_idx: usize,
    map_word: &str,
) -> usize {
    let mut consumed = 1;
    if let Some(ahead) = morphs.get(morph_idx + 1)
        && !ahead.is_ignored
        && map_word.ends_with(&ahead.surface)
    {
        consumed += 1;
    }
    consumed
}

#[rustfmt::skip]
#[inline(always)]
pub(super) fn consume_mismatched_morphs<'a>(
    morphs: &[MecabMorph],
    morph_idx: usize,
    current_map_word: &str,
    remaining_words: impl Iterator<Item = &'a str>,
) -> usize {
    let current_morph = &morphs[morph_idx];

    // 数字関連の Mismatch かどうかを判定
    let is_digit_mismatch = matches!(
        current_morph.surface.as_str(),
        "０" | "１" | "２" | "３" | "４" | "５" | "６" | "７" | "８" | "９" |
        "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"
    );

    if !is_digit_mismatch {
        return 1;
    }

    // 連続する数字 morph の数を数える
    let mut digit_morphs_count = 0;
    for m in &morphs[morph_idx..] {
        if m.is_ignored {
            continue;
        }
        if matches!(
            m.surface.as_str(),
            "０" | "１" | "２" | "３" | "４" | "５" | "６" | "７" | "８" | "９" |
            "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"
        ) {
            digit_morphs_count += 1;
        } else {
            break;
        }
    }

    // 連続する漢数字 mapping の数を数える (現在の単語も含める)
    let mut digit_maps_count: i32 = 0;
    if matches!(
        current_map_word,
        "一" | "二" | "三" | "四" | "五" | "六" | "七" | "八" |"九" |
        "十" | "百" | "千" | "万" | "億" | "兆" | "〇" | "零"
    ) {
        digit_maps_count += 1;
    }
    
    for w in remaining_words {
        if matches!(
            w,
            "一" | "二" | "三" | "四" | "五" | "六" | "七" | "八" | "九" |
            "十" | "百" | "千" | "万" | "億" | "兆" | "〇" | "零"
        ) {
            digit_maps_count += 1;
        } else {
            break;
        }
    }

    let target_remaining_morphs = digit_maps_count.saturating_sub(1);

    if digit_morphs_count <= target_remaining_morphs {
        return 0; // 挿入ノードとして morph を消費しない
    }

    let needed_non_ignored = digit_morphs_count - target_remaining_morphs;
    let mut consumed = 0;
    let mut counted_non_ignored = 0;

    while let Some(m) = morphs.get(morph_idx + consumed) {
        if !m.is_ignored {
            if !matches!(
                m.surface.as_str(),
                "０" | "１" | "２" | "３" | "４" | "５" | "６" | "７" | "８" | "９" |
                "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"
            ) {
                break;
            }
            counted_non_ignored += 1;
        }
        consumed += 1;

        if counted_non_ignored >= needed_non_ignored {
            break;
        }
    }

    consumed
}

/// 突き合わせのために、算用数字と漢数字を同じ文字へ寄せる。
///
/// `njd_set_digit` は `５` を `五` に直すので、そのままでは表層形が一致しない。
/// 位置を突き合わせるときだけ同じものとして扱う。
fn number_key(c: char) -> Option<char> {
    Some(match c {
        '０' | '0' | '〇' | '○' | '零' => '〇',
        '１' | '1' | '一' | '壱' => '一',
        '２' | '2' | '二' | '弐' => '二',
        '３' | '3' | '三' | '参' => '三',
        '４' | '4' | '四' => '四',
        '５' | '5' | '五' => '五',
        '６' | '6' | '六' => '六',
        '７' | '7' | '七' => '七',
        '８' | '8' | '八' => '八',
        '９' | '9' | '九' => '九',
        '十' | '百' | '千' | '万' | '億' | '兆' => c,
        _ => return None,
    })
}

/// 表層形が数字だけでできているか。
fn is_number_surface(surface: &str) -> bool {
    !surface.is_empty() && surface.chars().all(|c| number_key(c).is_some())
}

/// 数詞の並びを、突き合わせ用の文字列に直す。
fn number_keys(surface: &str) -> Vec<char> {
    surface.chars().filter_map(number_key).collect()
}

/// 編集距離の表が入力の長さの二乗で膨らまないための上限。通常の数値表記を
/// 十分に上回る大きさにしてある。
const MAX_NUMBER_BLOCK: usize = 128;

/// 数詞の並びについて、NJD の形態素それぞれが消費する MeCab 形態素を決める。
///
/// `njd_set_digit` は位取りの文字を差し込む一方 (`２０` -> `二 十`)、ゼロや
/// 助数詞との結合では入力を吸収する。差し込みと吸収があるので、1 つずつ順に
/// 対応させると以降がずれる。並び全体で編集距離が最小になる経路を求め、
/// 差し込まれた形態素には入力を割り当てず、吸収された入力は直前の形態素に
/// まとめる。
///
/// 実装は `pyopenjtalk-plus` の `_align_number_block` に倣った。
fn align_number_block(source: &[Vec<char>], target: &[Vec<char>]) -> Vec<Vec<usize>> {
    let (n, m) = (source.len(), target.len());
    let mut assignments: Vec<Vec<usize>> = vec![Vec::new(); m];

    // 極端に長い数詞では表を作らず、入力順に 1 対 1 で消費する。
    // NJD 側が少ない場合の余りは最後の形態素へ集約し、入力を取りこぼさない
    if n > MAX_NUMBER_BLOCK || m > MAX_NUMBER_BLOCK {
        for (i, assignment) in (0..n).zip(0..) {
            let _ = assignment;
            assignments[i.min(m.saturating_sub(1))].push(i);
        }
        return assignments;
    }

    #[derive(Clone, Copy, PartialEq)]
    enum Action {
        None,
        Align,
        Delete,
        Insert,
    }

    let mut cost = vec![vec![0usize; m + 1]; n + 1];
    let mut action = vec![vec![Action::None; m + 1]; n + 1];
    for i in 1..=n {
        cost[i][0] = i;
        action[i][0] = Action::Delete;
    }
    for j in 1..=m {
        cost[0][j] = j;
        action[0][j] = Action::Insert;
    }
    for i in 1..=n {
        for j in 1..=m {
            let substitution = usize::from(source[i - 1] != target[j - 1]);
            let candidates = [
                (cost[i - 1][j - 1] + substitution, Action::Align),
                (cost[i - 1][j] + 1, Action::Delete),
                (cost[i][j - 1] + 1, Action::Insert),
            ];
            let (best, best_action) = candidates
                .into_iter()
                .min_by_key(|(c, a)| {
                    (
                        *c,
                        match a {
                            Action::Align => 0,
                            Action::Delete => 1,
                            _ => 2,
                        },
                    )
                })
                .unwrap();
            cost[i][j] = best;
            action[i][j] = best_action;
        }
    }

    // 経路を逆にたどり、入力順へ戻す
    let mut steps: Vec<(Action, Option<usize>, Option<usize>)> = Vec::new();
    let (mut i, mut j) = (n, m);
    while i > 0 || j > 0 {
        match action[i][j] {
            Action::Align => {
                steps.push((Action::Align, Some(i - 1), Some(j - 1)));
                i -= 1;
                j -= 1;
            }
            Action::Delete => {
                steps.push((Action::Delete, Some(i - 1), None));
                i -= 1;
            }
            _ => {
                steps.push((Action::Insert, None, Some(j - 1)));
                j -= 1;
            }
        }
    }
    steps.reverse();

    // 吸収された入力 (delete) は直前の形態素へまとめる
    let mut pending: Vec<usize> = Vec::new();
    let mut previous: Option<usize> = None;
    for (act, src, tgt) in steps {
        match act {
            Action::Align => {
                let tgt = tgt.unwrap();
                assignments[tgt].append(&mut pending);
                assignments[tgt].push(src.unwrap());
                previous = Some(tgt);
            }
            Action::Delete => match previous {
                Some(prev) => assignments[prev].push(src.unwrap()),
                None => pending.push(src.unwrap()),
            },
            _ => {}
        }
    }
    if let Some(prev) = previous {
        assignments[prev].extend(pending);
    } else if let Some(first) = assignments.first_mut() {
        first.extend(pending);
    }

    assignments
}

/// NJD の形態素列に、解析対象の文字列における位置 (文字単位、半開区間) を与える。
///
/// [`MecabMorph::char_span`] は MeCab のノードがバッファのどこを指すかで決まるが、
/// NJD は数字を正規化するので (「１４７３」-> 千 四 百 七 十 三)、NJD の形態素に
/// そのまま対応するバイト位置は無い。そこで MeCab の形態素列と突き合わせ、
/// 各 NJD 形態素が元の文字列のどこにあたるかを求める。
///
/// 突き合わせは 4 通りになる。表層形がそのまま一致する場合、NJD が複数の形態素を
/// 1 語にまとめた場合、数詞の並び、踊り字の展開である。数詞の並びは
/// `align_number_block` が MeCab の形態素と NJD の形態素を対応付け、踊り字は
/// `consume_odori_morphs` が消費する形態素の数を数える。
///
/// 対応が取れなかった形態素には空の区間 (`n..n`) を与える。位取りとして
/// 差し込まれた形態素 (`２０` の `十`) がこれにあたる。
pub fn njd_char_spans(features: &[NjdFeature], morphs: &[MecabMorph]) -> Vec<Range<usize>> {
    let mut spans: Vec<Range<usize>> = vec![0..0; features.len()];
    let mut morph_idx = 0;
    let mut idx = 0;

    while idx < features.len() {
        // NJD の形態素列に現れない morph (空白など) を先に進める
        while morphs.get(morph_idx).is_some_and(|m| m.is_ignored) {
            morph_idx += 1;
        }

        let word = features[idx].string.as_str();
        let Some(morph) = morphs.get(morph_idx) else {
            let end = morphs.last().map_or(0, |m| m.char_span.end);
            spans[idx] = end..end;
            idx += 1;
            continue;
        };

        // 数詞の並びは、まとめて突き合わせる
        if is_number_surface(word) && is_number_surface(&morph.surface) {
            let feature_end = (idx..features.len())
                .find(|&i| !is_number_surface(&features[i].string))
                .unwrap_or(features.len());
            let mut morph_end = morph_idx;
            while morphs
                .get(morph_end)
                .is_some_and(|m| m.is_ignored || is_number_surface(&m.surface))
            {
                morph_end += 1;
            }
            let source_indices: Vec<usize> = (morph_idx..morph_end)
                .filter(|&i| !morphs[i].is_ignored)
                .collect();
            let source: Vec<Vec<char>> = source_indices
                .iter()
                .map(|&i| number_keys(&morphs[i].surface))
                .collect();
            let target: Vec<Vec<char>> = features[idx..feature_end]
                .iter()
                .map(|f| number_keys(&f.string))
                .collect();

            // 位取りとして差し込まれた形態素は元の文字を持たないので、直前の
            // 形態素の終端に空の区間として置く。並びの先頭に置くと、そこから
            // 始まる形態素と開始位置が同じになる
            let mut at = morph.char_span.start;
            for (offset, assigned) in align_number_block(&source, &target).into_iter().enumerate() {
                spans[idx + offset] = match (
                    assigned
                        .first()
                        .map(|&i| morphs[source_indices[i]].char_span.start),
                    assigned
                        .last()
                        .map(|&i| morphs[source_indices[i]].char_span.end),
                ) {
                    (Some(start), Some(end)) => {
                        at = end;
                        start..end
                    }
                    _ => at..at,
                };
            }
            idx = feature_end;
            morph_idx = morph_end;
            continue;
        }

        let start = morph.char_span.start;
        let consumed = if word == morph.surface {
            1
        } else if word.starts_with(&morph.surface) {
            // NJD が複数の morph を 1 語にまとめた
            let mut matched = 0;
            let mut consumed = 0;
            while let Some(m) = morphs.get(morph_idx + consumed) {
                if m.is_ignored {
                    consumed += 1;
                    continue;
                }
                if !word[matched..].starts_with(&m.surface) {
                    break;
                }
                matched += m.surface.len();
                consumed += 1;
                if matched == word.len() {
                    break;
                }
            }
            consumed
        } else if has_odori_chars(&morph.surface) {
            consume_odori_morphs(morphs, morph_idx, word)
        } else {
            1
        };

        let end = morphs[morph_idx..morph_idx + consumed]
            .iter()
            .map(|m| m.char_span.end)
            .max()
            .unwrap_or(start);
        spans[idx] = start..end;
        morph_idx += consumed;
        idx += 1;
    }

    spans
}

impl OpenJTalk {
    /// [`WordPhonemeMap`] を組むための種を作る。
    ///
    /// `njd_spans` は [`njd_char_spans`] が返したもので、`njd_features` と長さが
    /// 揃っていなければならない。
    pub(crate) fn g2p_seed_inner(
        &mut self,
        njd_features: &[NjdFeature],
        njd_spans: &[Range<usize>],
        is_non_pause_symbol: fn(&str) -> bool,
    ) -> Result<Vec<WordPhonemeSeed>, HaqumeiError> {
        let mut mapping: Vec<WordPhonemeSeed> = njd_features
            .iter()
            .enumerate()
            .map(|(i, f)| WordPhonemeSeed {
                word: f.string.clone(),
                phonemes: Vec::new(),
                char_span: njd_spans.get(i).cloned().unwrap_or(0..0),
            })
            .collect();

        self.assign_and_merge_phonemes(njd_features, &mut mapping, is_non_pause_symbol)?;
        Ok(mapping)
    }

    #[allow(deprecated)]
    pub(crate) fn g2p_pairs_inner(
        &mut self,
        njd_features: &[NjdFeature],
        is_non_pause_symbol: fn(&str) -> bool,
    ) -> Result<Vec<WordPhonemePair>, HaqumeiError> {
        let mut mapping: Vec<WordPhonemePair> = njd_features
            .iter()
            .map(|f| WordPhonemePair {
                word: f.string.clone(),
                phonemes: Vec::new(),
            })
            .collect();

        self.assign_and_merge_phonemes(njd_features, &mut mapping, is_non_pause_symbol)?;
        Ok(mapping)
    }

    pub(crate) fn g2p_mapping_inner(
        &mut self,
        njd_features: &[NjdFeature],
        njd_spans: &[Range<usize>],
        is_non_pause_symbol: fn(&str) -> bool,
    ) -> Result<Vec<WordPhonemeDetail>, HaqumeiError> {
        let mut mapping: Vec<WordPhonemeDetail> = njd_features
            .iter()
            .enumerate()
            .map(|(i, f)| WordPhonemeDetail {
                word: f.string.clone(),
                phonemes: Vec::new(),
                features: Vec::new(),
                pos: f.pos.clone(),
                pos_group1: f.pos_group1.clone(),
                pos_group2: f.pos_group2.clone(),
                pos_group3: f.pos_group3.clone(),
                ctype: f.ctype.clone(),
                cform: f.cform.clone(),
                orig: f.orig.clone(),
                read: f.read.clone(),
                pron: f.pron.clone(),
                accent_nucleus: f.acc,
                mora_count: f.mora_size,
                chain_rule: f.chain_rule.clone(),
                chain_flag: f.chain_flag,
                is_unknown: false,
                is_ignored: false,
                char_span: njd_spans.get(i).cloned().unwrap_or(0..0),
            })
            .collect();

        self.assign_and_merge_phonemes(njd_features, &mut mapping, is_non_pause_symbol)?;
        Ok(mapping)
    }

    pub(crate) fn g2p_mapping_prosody_inner(
        &mut self,
        njd_features: &[NjdFeature],
        njd_spans: &[Range<usize>],
        is_non_pause_symbol: fn(&str) -> bool,
    ) -> Result<Vec<WordPhonemeProsody>, HaqumeiError> {
        let mut mapping: Vec<WordPhonemeProsody> = njd_features
            .iter()
            .enumerate()
            .map(|(i, f)| WordPhonemeProsody {
                word: f.string.clone(),
                phonemes: Vec::new(),
                pos: f.pos.clone(),
                pos_group1: f.pos_group1.clone(),
                pos_group2: f.pos_group2.clone(),
                pos_group3: f.pos_group3.clone(),
                ctype: f.ctype.clone(),
                cform: f.cform.clone(),
                orig: f.orig.clone(),
                read: f.read.clone(),
                pron: f.pron.clone(),
                accent_nucleus: f.acc,
                mora_count: f.mora_size,
                chain_rule: f.chain_rule.clone(),
                chain_flag: f.chain_flag,
                is_unknown: false,
                is_ignored: false,
                char_span: njd_spans.get(i).cloned().unwrap_or(0..0),
            })
            .collect();

        self.assign_and_merge_prosodic_phonemes(njd_features, &mut mapping, is_non_pause_symbol)?;
        Ok(mapping)
    }

    pub(crate) fn assign_and_merge_phonemes<T: WordPhonemeEntry>(
        &mut self,
        njd_features: &[NjdFeature],
        mapping: &mut Vec<T>,
        is_non_pause_symbol: fn(&str) -> bool,
    ) -> Result<(), HaqumeiError> {
        unsafe {
            let ptr_to_idx = self.prepare_jpcommon_label_internal(njd_features)?;
            let jp = self.jp_common.inner.as_mut();

            for (f_idx, f) in njd_features.iter().enumerate() {
                let is_pause_pron = f.pron == "、" || f.pron == "？" || f.pron == "！";

                if is_pause_pron && !is_non_pause_symbol(&f.string) {
                    mapping[f_idx].phonemes_mut().push(Phoneme::Pau);
                }
            }

            let mut p = (*jp.label).phoneme_head;
            while !p.is_null() {
                let s_ptr = (*p).phoneme;
                if !s_ptr.is_null() {
                    let s = if cfg!(debug_assertions) {
                        Phoneme::try_from_ptr(s_ptr).unwrap()
                    } else {
                        Phoneme::from(s_ptr)
                    };

                    if s != Phoneme::Pau {
                        let mora = (*p).up;
                        if !mora.is_null() {
                            let word = (*mora).up;
                            if !word.is_null()
                                && let Some(&idx) = ptr_to_idx.get(&(word as usize))
                                && let Some(target) = mapping.get_mut(idx)
                            {
                                target.phonemes_mut().push(s);
                            }
                        }
                    }
                }
                p = (*p).next;
            }

            ffi::JPCommon_refresh(jp);
            ffi::NJD_refresh(self.njd.inner.as_mut());

            // 長音によって、先行する Word のモーラとして吸収されるケースがあるため、
            // 前方の Word に結合する。
            //
            // 例:
            // "つまみ出されようとした"
            // - つまみ出さ: [ts u m a m i d a s a]
            // - れよ: [r e y o o]
            // - う: []
            // - と: [t o]
            // - し: [sh I]
            // - た: [t a]
            //
            // 音素が空になった "う" を先行する "れよ" に結合する。
            // このとき、`njd_features` の "う" の pron は長音に置き換えられている。
            let mut write_idx = 0;
            for read_idx in 0..mapping.len() {
                let mut should_merge = false;

                if read_idx > 0 && mapping[read_idx].phonemes().is_empty() {
                    let pron = &njd_features[read_idx].pron;
                    let is_absorbed_long_vowel =
                        !pron.is_empty() && pron.chars().all(|c| c == 'ー');

                    if is_absorbed_long_vowel {
                        let prev_phonemes = mapping[write_idx - 1].phonemes();
                        let prev_is_pause = prev_phonemes.len() == 1 && prev_phonemes[0] == "pau";

                        if !prev_is_pause && !prev_phonemes.is_empty() {
                            should_merge = true;
                        }
                    }
                }

                if should_merge {
                    let (left, right) = mapping.split_at_mut(read_idx);
                    left[write_idx - 1].merge_from(&mut right[0]);
                    continue;
                }

                if write_idx != read_idx {
                    mapping.swap(write_idx, read_idx);
                }
                write_idx += 1;
            }
            mapping.truncate(write_idx);

            Ok(())
        }
    }

    pub(crate) fn assign_and_merge_prosodic_phonemes(
        &mut self,
        njd_features: &[NjdFeature],
        mapping: &mut Vec<WordPhonemeProsody>,
        is_non_pause_symbol: fn(&str) -> bool,
    ) -> Result<(), HaqumeiError> {
        let labels = self.extract_fullcontext_labels(njd_features)?;

        unsafe {
            let ptr_to_idx = self.prepare_jpcommon_label_internal(njd_features)?;
            let jp = self.jp_common.inner.as_mut();

            for (f_idx, f) in njd_features.iter().enumerate() {
                let is_pause_pron = f.pron == "、" || f.pron == "？" || f.pron == "！";

                if is_pause_pron && !is_non_pause_symbol(&f.string) {
                    for c in f.string.chars() {
                        let marker = match c {
                            '？' | '?' => ProsodicPhoneme::Interrogative,
                            '！' | '!' => ProsodicPhoneme::Exclamatory,
                            _ => ProsodicPhoneme::Pause,
                        };
                        mapping[f_idx].phonemes.push(marker);
                    }
                }
            }

            let check_already_has = |mapping: &[WordPhonemeProsody], target_idx: usize| -> bool {
                let end_idx = (target_idx + 3).min(mapping.len());
                mapping[target_idx..end_idx].iter().any(|m| {
                    m.phonemes.iter().any(|p| {
                        matches!(
                            p,
                            ProsodicPhoneme::Interrogative | ProsodicPhoneme::Exclamatory
                        )
                    })
                })
            };

            let mut last_target_idx: Option<usize> = None;
            let num_labels = labels.len();

            let mut p = (*jp.label).phoneme_head;
            let mut label_idx = 0;

            while label_idx < num_labels {
                let label = &labels[label_idx];
                let p3 = label.phoneme.c.as_deref().unwrap_or("");

                if p3 == "sil" {
                    if label_idx == num_labels - 1 {
                        let (is_inter, is_excl) = label
                            .accent_phrase_prev
                            .as_ref()
                            .map(|a| (a.is_interrogative, a.is_exclamatory))
                            .unwrap_or((false, false));

                        if (is_inter || is_excl)
                            && let Some(target_idx) = last_target_idx
                            && !check_already_has(mapping, target_idx)
                        {
                            if is_excl {
                                mapping[target_idx]
                                    .phonemes
                                    .push(ProsodicPhoneme::Exclamatory);
                            }
                            if is_inter {
                                mapping[target_idx]
                                    .phonemes
                                    .push(ProsodicPhoneme::Interrogative);
                            }
                        }
                    }
                    label_idx += 1;
                    continue;
                }

                if p.is_null() {
                    label_idx += 1;
                    continue;
                }

                // Word インデックスを特定
                let mut current_target_idx = None;
                let mora = (*p).up;
                if !mora.is_null() {
                    let word = (*mora).up;
                    if !word.is_null()
                        && let Some(&idx) = ptr_to_idx.get(&(word as usize))
                    {
                        current_target_idx = Some(idx);
                    }
                }

                if current_target_idx.is_some() {
                    last_target_idx = current_target_idx;
                }

                let target_idx = match current_target_idx.or(last_target_idx) {
                    Some(idx) => idx,
                    None => {
                        p = (*p).next;
                        label_idx += 1;
                        continue;
                    }
                };
                let check_already_has = check_already_has(mapping, target_idx);
                let target = mapping.get_mut(target_idx).unwrap();

                let s_ptr = (*p).phoneme;
                let s = if cfg!(debug_assertions) {
                    Phoneme::try_from_ptr(s_ptr).unwrap()
                } else {
                    Phoneme::from(s_ptr)
                };

                if s == Phoneme::Pau {
                    let (is_inter, is_excl) = label
                        .accent_phrase_prev
                        .as_ref()
                        .map(|a| (a.is_interrogative, a.is_exclamatory))
                        .unwrap_or((false, false));

                    if (is_inter || is_excl) && !check_already_has {
                        if is_excl {
                            target.phonemes.push(ProsodicPhoneme::Exclamatory);
                        }
                        if is_inter {
                            target.phonemes.push(ProsodicPhoneme::Interrogative);
                        }
                    }

                    p = (*p).next;
                    label_idx += 1;
                    continue;
                }

                // アクセント核の位置
                let f2 = label
                    .accent_phrase_curr
                    .as_ref()
                    .map(|a| a.accent_position as i32)
                    .unwrap_or(0);

                // 現在のモーラ位置
                let a2 = label
                    .mora
                    .as_ref()
                    .map(|m| m.position_forward as i32)
                    .unwrap_or(0);

                let is_high = if f2 == 0 {
                    a2 >= 2 // 平板型
                } else if f2 == 1 {
                    a2 == 1 // 頭高型
                } else {
                    a2 >= 2 && a2 <= f2 // 中高・尾高型
                };

                let pitch = if is_high {
                    PitchAccent::High
                } else {
                    PitchAccent::Low
                };

                target.phonemes.push(ProsodicPhoneme::Phoneme {
                    phoneme: s,
                    pitch: Some(pitch),
                });

                // アクセント句境界の計算
                let a3 = label
                    .mora
                    .as_ref()
                    .map(|m| m.position_backward as i32)
                    .unwrap_or(-50);
                let a2_next = if label_idx + 1 < num_labels {
                    labels[label_idx + 1]
                        .mora
                        .as_ref()
                        .map(|m| m.position_forward as i32)
                        .unwrap_or(-50)
                } else {
                    -50
                };

                if a3 == 1
                    && a2_next == 1
                    && matches!(
                        p3,
                        "a" | "e" | "i" | "o" | "u" | "A" | "E" | "I" | "O" | "U" | "N" | "cl"
                    )
                {
                    target.phonemes.push(ProsodicPhoneme::AccentPhraseBoundary);
                }

                p = (*p).next;
                label_idx += 1;
            }

            ffi::JPCommon_refresh(jp);
            ffi::NJD_refresh(self.njd.inner.as_mut());

            // 長音によって、先行する Word のモーラとして吸収されるケースがあるため、
            // 前方の Word に結合する。
            let mut write_idx = 0;
            for read_idx in 0..mapping.len() {
                let mut should_merge = false;

                if read_idx > 0 && mapping[read_idx].phonemes.is_empty() {
                    let pron = &njd_features[read_idx].pron;
                    let is_absorbed_long_vowel =
                        !pron.is_empty() && pron.chars().all(|c| c == 'ー');

                    if is_absorbed_long_vowel {
                        let prev_phonemes = &mapping[write_idx - 1].phonemes;
                        let prev_is_pause =
                            prev_phonemes.len() == 1 && prev_phonemes[0] == ProsodicPhoneme::pau();

                        if !prev_is_pause && !prev_phonemes.is_empty() {
                            should_merge = true;
                        }
                    }
                }

                if should_merge {
                    let (left, right) = mapping.split_at_mut(read_idx);
                    left[write_idx - 1].merge_from(&mut right[0]);

                    continue;
                }

                if write_idx != read_idx {
                    mapping.swap(write_idx, read_idx);
                }
                write_idx += 1;
            }
            mapping.truncate(write_idx);

            Ok(())
        }
    }

    #[inline(always)]
    pub(crate) fn make_phoneme_mapping<T: IntoPhonemeMapItem>(
        &self,
        morphs: Vec<MecabMorph>,
        mapping: Vec<T>,
    ) -> Result<Vec<T::Output>, HaqumeiError> {
        let mut result = Vec::with_capacity(morphs.len());
        let mut morph_idx = 0;

        // 所有権を保持しつつ後方の要素を参照できるようにするため Option でラップする
        let mut mapping_options: Vec<Option<T>> = mapping.into_iter().map(Some).collect();

        for idx in 0..mapping_options.len() {
            // is_ignored な Morph を先に進めておく
            while let Some(m) = morphs.get(morph_idx) {
                if m.is_ignored {
                    result.push(T::new_ignored(m));
                    morph_idx += 1;
                } else {
                    break;
                }
            }

            let map = mapping_options[idx].take().unwrap();

            // morphs が尽きた場合
            if morph_idx >= morphs.len() {
                result.push(map.into_unmatched_remainder());
                continue;
            }

            let morph = &morphs[morph_idx];

            if map.word() == morph.surface {
                // 完全一致
                result.push(map.into_exact_match(morph));
                morph_idx += 1;
            } else if map.word().starts_with(&morph.surface) {
                // 先頭一致 (マージ)
                let mut is_unknown_word = false;
                let mut matched_len = 0;
                let mut pre_ignored = Vec::new();
                let mut internal_ignored = Vec::new();

                while let Some(inner_morph) = morphs.get(morph_idx) {
                    if inner_morph.is_ignored {
                        // 文字列構成が始まる「前」のスペースは単語の前に出す
                        if matched_len == 0 {
                            pre_ignored.push(T::new_ignored(inner_morph));
                        } else {
                            internal_ignored.push(T::new_ignored(inner_morph));
                        }
                        morph_idx += 1;
                        continue;
                    }

                    let remaining = &map.word()[matched_len..];

                    if remaining.starts_with(&inner_morph.surface) {
                        is_unknown_word |= inner_morph.is_unknown;
                        matched_len += inner_morph.surface.len();
                        morph_idx += 1;

                        if matched_len == map.word().len() {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                result.extend(pre_ignored);
                result.push(map.into_prefix_match(is_unknown_word));
                result.extend(internal_ignored);
            } else {
                // 不一致 (踊り字展開や数字展開など)
                let consumed_from = morph_idx;
                if has_odori_chars(&morph.surface) {
                    morph_idx += consume_odori_morphs(&morphs, morph_idx, map.word());
                } else {
                    morph_idx += consume_mismatched_morphs(
                        &morphs,
                        morph_idx,
                        map.word(),
                        mapping_options[idx + 1..]
                            .iter()
                            .flatten()
                            .map(|m| m.word()),
                    );
                }

                result.push(map.into_mismatch());

                // 数字の縮約は空白をまたぐ (「1　0」の morph は １ / 　 / ０ で、
                // NJD はこれを「十」1 つに縮約する)。消費した ignored な morph を
                // ここで出さないと、mapping から空白が消えて入力全体を復元できなくなる。
                // 縮約後は元の位置が復元できないので、まとめて直後に置く。
                for m in &morphs[consumed_from..morph_idx] {
                    if m.is_ignored {
                        result.push(T::new_ignored(m));
                    }
                }
            }
        }

        // 余った ignored morphs を回収
        while let Some(m) = morphs.get(morph_idx) {
            if m.is_ignored {
                result.push(T::new_ignored(m));
            }
            morph_idx += 1;
        }

        Ok(result)
    }

    /// 呼び出し後は必ず JPCommon_refresh / NJD_refresh を行わなければならない。
    /// NJDFeature を元に JPCommon の内部構造体 (Word/Mora/Phoneme階層) を構築する。
    /// 戻り値として、JPCommonLabelWord のポインタから、対応する NJDFeature のインデックスへのマッピングを返す。
    unsafe fn prepare_jpcommon_label_internal(
        &mut self,
        features: &[NjdFeature],
    ) -> Result<FxHashMap<usize, usize>, HaqumeiError> {
        Self::features_to_njd(features, &mut self.njd)?;
        let mut ptr_to_idx =
            FxHashMap::with_capacity_and_hasher(features.len(), rustc_hash::FxBuildHasher);

        unsafe {
            let jp = self.jp_common.inner.as_mut();
            let njd = self.njd.inner.as_mut();

            ffi::njd2jpcommon(jp, njd);

            // JPCommon_make_label(JPCommon * jpcommon) の部分的な移植
            if !jp.label.is_null() {
                ffi::JPCommonLabel_clear(jp.label);
            } else {
                let ptr = libc::calloc(1, std::mem::size_of::<ffi::JPCommonLabel>());
                if ptr.is_null() {
                    return Err(HaqumeiError::AllocationError("ffi::JPCommonLabel"));
                }
                jp.label = ptr as *mut ffi::JPCommonLabel;
            }

            ffi::JPCommonLabel_initialize(jp.label);

            let mut node = jp.head;
            let mut f_idx = 0;

            while !node.is_null() {
                let prev_word_tail = (*jp.label).word_tail;

                super::jpcommon_push_word::JPCommonLabel_push_word(
                    jp.label,
                    ffi::JPCommonNode_get_pron(node),
                    ffi::JPCommonNode_get_pos(node),
                    ffi::JPCommonNode_get_ctype(node),
                    ffi::JPCommonNode_get_cform(node),
                    ffi::JPCommonNode_get_acc(node),
                    ffi::JPCommonNode_get_chain_flag(node),
                )?;

                // 追加後の末尾のWordポインタ
                let curr_word_tail = (*jp.label).word_tail;

                // JPCommonLabel_push_word によって新しい Word が生成された場合のみマッピングを記録する。
                // (「ー」などで直前のWordに吸収された場合や、pau で Word が生成されなかった場合はスキップされる)
                if prev_word_tail != curr_word_tail && !curr_word_tail.is_null() {
                    ptr_to_idx.insert(curr_word_tail as usize, f_idx);
                }

                node = (*node).next;
                f_idx += 1;
            }
        }

        Ok(ptr_to_idx)
    }
}
