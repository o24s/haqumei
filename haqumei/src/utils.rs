use std::{
    collections::HashSet,
    fs::Metadata,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use sha2::{Digest, Sha256};
use vibrato_rkyv::tokenizer::worker::Worker;

use crate::{
    Haqumei, NjdFeature, VIBRATO_CACHE,
    data::{MULTI_READ_KANJI_LIST, TO_DAKUON, TO_SEION},
    errors::HaqumeiError,
    features::UnidicFeature,
    open_jtalk::OpenJTalk,
};

pub(crate) fn collect_dict_files(dir: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut paths = Vec::new();

    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry?;
        let path = entry.path();
        if path.is_file()
            && let Some(extension) = path.extension()
            && (extension == "dic" || extension == "bin")
        {
            paths.push(path.to_path_buf());
        }
    }

    paths.sort();

    Ok(paths)
}

#[inline(always)]
pub(crate) fn compute_metadata_key(meta: &Metadata) -> [u8; 32] {
    let mut hasher = Sha256::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        hasher.update(meta.dev().to_le_bytes());
        hasher.update(meta.ino().to_le_bytes());
        hasher.update(meta.size().to_le_bytes());
        hasher.update(meta.mtime().to_le_bytes());
        hasher.update(meta.mtime_nsec().to_le_bytes());
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        hasher.update(meta.file_size().to_le_bytes());
        hasher.update(meta.last_write_time().to_le_bytes());
        hasher.update(meta.creation_time().to_le_bytes());
        hasher.update(meta.file_attributes().to_le_bytes());
    }

    #[cfg(not(any(unix, windows)))]
    {
        use std::time::SystemTime;

        fn update_system_time(time: Result<SystemTime, std::io::Error>, hasher: &mut Sha256) {
            match time.and_then(|t| {
                t.duration_since(SystemTime::UNIX_EPOCH)
                    .map_err(|_| std::io::Error::from(std::io::ErrorKind::Other))
            }) {
                Ok(duration) => {
                    hasher.update(duration.as_secs().to_le_bytes());
                    hasher.update(duration.subsec_nanos().to_le_bytes());
                }
                Err(_) => {
                    hasher.update([0u8; 12]);
                }
            }
        }

        let file_type = meta.file_type();
        let type_byte: u8 = if file_type.is_file() {
            0x01
        } else if file_type.is_dir() {
            0x02
        } else if file_type.is_symlink() {
            0x03
        } else {
            0x00
        };
        hasher.update([type_byte]);

        let readonly_byte: u8 = if meta.permissions().readonly() {
            0x01
        } else {
            0x00
        };
        hasher.update([readonly_byte]);

        hasher.update(meta.len().to_le_bytes());

        update_system_time(meta.modified(), &mut hasher);

        update_system_time(meta.created(), &mut hasher);
    }

    hasher.finalize().into()
}

/// フィラーが acc > mora_size のときに、平版型 (acc = 0) にし、
/// その直後の形態素が名詞だったとき、
/// その前のフィラーに結合しない (chain_flag = 0) ようにする
pub fn modify_filler_accent(njd_features: &mut [NjdFeature]) {
    let mut is_after_filler = false;

    for features in njd_features.iter_mut() {
        if features.pos == "フィラー" {
            if features.acc > features.mora_size {
                features.acc = 0;
            }
            is_after_filler = true;
        } else if is_after_filler {
            if features.pos == "名詞" {
                features.chain_flag = 0;
            }
            is_after_filler = false;
        }
    }
}

impl Haqumei {
    pub(crate) fn predict_nani_reading(&mut self, njd_features: &mut [NjdFeature]) {
        for i in 0..njd_features.len() {
            if njd_features[i].orig == "何" {
                let next_node_feature = njd_features.get(i + 1);
                let is_read_nan = self.predict_is_nan(next_node_feature);
                let yomi = if is_read_nan { "ナン" } else { "ナニ" };

                njd_features[i].pron = yomi.to_string();
                njd_features[i].read = yomi.to_string();
            }
        }
    }

    pub(crate) fn modify_kanji_yomi(&mut self, text: &str, njd_features: &mut [NjdFeature]) {
        let tokens: Vec<UnidicFeature> = VIBRATO_CACHE
            .get(text)
            .unwrap_or({
                let mut worker = self.tokenizer.as_ref().unwrap().new_worker();
                vibrato_analysis(&mut worker, text)
            })
            .into_iter()
            .filter(|t| MULTI_READ_KANJI_LIST.contains(t.surface.as_str()))
            .collect();

        if tokens.is_empty() {
            return;
        }

        let mut unidic_iter = tokens.into_iter().peekable();
        let mut current_char_pos = 0;
        for njd_feature in njd_features {
            let node_string = &njd_feature.string;
            let node_orig = &njd_feature.orig;
            let node_char_len = node_string.chars().count();

            while let Some(candidate) = unidic_iter.peek() {
                if candidate.range_char.end <= current_char_pos {
                    unidic_iter.next();
                } else {
                    break;
                }
            }

            let mut pron_to_set: Option<String> = None;
            let mut read_to_set: Option<String> = None;

            if MULTI_READ_KANJI_LIST.contains(node_orig.as_str())
                && let Some(candidate) = unidic_iter.peek()
                && candidate.range_char.start == current_char_pos
                && candidate.surface == *node_orig
            {
                let correct_yomi_token = unidic_iter.next().unwrap();

                let reading = correct_yomi_token.pron();
                pron_to_set = Some(reading.to_string());
                read_to_set = Some(reading.to_string());
            }
            if let Some(pron) = pron_to_set {
                njd_feature.pron = pron;
            }
            if let Some(read) = read_to_set {
                njd_feature.read = read;
            }

            current_char_pos += node_char_len;
        }
    }
}

pub(crate) fn vibrato_analysis(worker: &mut Worker, text: &str) -> Vec<UnidicFeature> {
    VIBRATO_CACHE.get_with(text.to_string(), || {
        worker.reset_sentence(text);
        worker.tokenize();

        worker
            .token_iter()
            .map(|token| {
                let token = token.to_buf();
                let mut ranges = Vec::with_capacity(29);
                let mut start = 0;
                for part in token.feature.split(',') {
                    let end = start + part.len();
                    ranges.push(start..end);
                    start = end + 1;
                }

                UnidicFeature {
                    surface: token.surface,
                    feature: token.feature,
                    range_char: token.range_char,
                    range_byte: token.range_byte,
                    lex_type: token.lex_type,
                    word_id: token.word_id,
                    left_id: token.left_id,
                    right_id: token.right_id,
                    word_cost: token.word_cost,
                    total_cost: token.total_cost,
                    feature_ranges: ranges,
                }
            })
            .collect()
    })
}

/// 長母音、重母音、撥音がアクセント核に来た場合にひとつ前のモーラにアクセント核がズレるルールを適用します。
pub(crate) fn retreat_acc_nuc(njd_features: &mut [NjdFeature]) {
    if njd_features.is_empty() {
        return;
    }

    const INAPPROPRIATE_FOR_NUCLEAR_CHARS: &[char] = &['ー', 'ッ', 'ン'];

    let mut head_index = 0;
    let mut acc = 0;

    for i in 0..njd_features.len() {
        // アクセント境界直後の node (chain_flag 0 or -1) にアクセント核の位置の情報が入っている
        if njd_features[i].chain_flag == 0 || njd_features[i].chain_flag == -1 {
            head_index = i;
            acc = njd_features[head_index].acc;
        }

        const YOUON_CHARS: &[char] = &['ャ', 'ュ', 'ョ', 'ァ', 'ィ', 'ゥ', 'ェ', 'ォ'];
        let pron_without_youon: String = njd_features[i]
            .pron
            .chars()
            .filter(|c| !YOUON_CHARS.contains(c))
            .collect();

        let pron_ref = if pron_without_youon.is_empty() {
            &njd_features[i].pron
        } else {
            &pron_without_youon
        };

        if acc > 0 {
            if acc <= njd_features[i].mora_size {
                if pron_ref
                    .chars()
                    .nth((acc - 1) as usize)
                    .or(pron_ref.chars().next())
                    .is_some_and(|nuc_pron| INAPPROPRIATE_FOR_NUCLEAR_CHARS.contains(&nuc_pron))
                {
                    njd_features[head_index].acc = njd_features[head_index].acc.saturating_sub(1);
                }

                acc = -1;
            } else {
                acc -= njd_features[i].mora_size;
            }
        }
    }
}

/// 品詞「特殊・マス」は直前に接続する動詞にアクセント核がある場合、アクセント核を「ま」に移動させる法則がある
///   書きます → か[きま]す, 参ります → ま[いりま]す
///   書いております → [か]いております
pub fn modify_acc_after_chaining(njd_features: &mut [NjdFeature]) {
    if njd_features.is_empty() {
        return;
    }

    const SUFFIXES_TO_MODIFY_ACC: &[&str] = &["れる", "られる", "すぎる", "せる", "させる"];

    let mut head_index = 0;
    let mut acc = 0;

    // アクセント核を含むノードを過ぎたかどうか
    let mut is_after_nuc = false;
    // アクセント句の先頭からのモーラ数
    let mut phase_len = 0;

    for i in 0..njd_features.len() {
        // アクセント境界直後の node (chain_flag 0 or -1) にアクセント核の位置の情報が入っている
        if njd_features[i].chain_flag == 0 || njd_features[i].chain_flag == -1 {
            is_after_nuc = false;
            head_index = i;
            acc = njd_features[head_index].acc;
            phase_len = 0;
        }

        // acc = 0 の場合は「特殊・マス」は存在しないと考えてよい
        if acc == 0 {
            continue;
        }

        let mora_size = njd_features[i].mora_size;
        if is_after_nuc {
            let njd = &njd_features[i];

            if njd.ctype == "特殊・マス" {
                njd_features[head_index].acc = if njd.cform != "未然形" {
                    phase_len + 1
                } else {
                    phase_len + 2
                };
            } else if njd.ctype == "特殊・ナイ" {
                njd_features[head_index].acc = phase_len;
            } else if SUFFIXES_TO_MODIFY_ACC.contains(&njd.orig.as_str()) {
                njd_features[head_index].acc = phase_len + njd.acc;
            } else {
                is_after_nuc = false;
                acc = 0;
            }

            phase_len += mora_size;
        } else {
            phase_len += mora_size;
            if acc <= mora_size {
                is_after_nuc = true;
            } else {
                acc -= mora_size;
            }
        }
    }
}

/// 踊り字（々）と一の字点（ゝ、ゞ、ヽ、ヾ）の読みを適切に処理する後処理関数
pub fn process_odori_features(
    njd_features: &mut Vec<NjdFeature>,
    open_jtalk: &mut OpenJTalk,
) -> Result<(), HaqumeiError> {
    let mut i = 0;
    while i < njd_features.len() {
        let orig = njd_features[i].orig.clone();

        if is_dancing(&orig) {
            // 踊り字「々」の処理

            // 再解析が必要なケース
            let mut reanalysis_result = None;
            if i > 0 {
                let prev = &njd_features[i - 1];

                if count_odori(&orig) == 1 && is_kanji_token(prev) {
                    let prev_chars: Vec<char> = prev.orig.chars().collect();
                    if prev_chars.len() > 1 {
                        let last_char = *prev_chars.last().unwrap();
                        if is_kanji(last_char) {
                            // 後続トークンのチェック
                            let next_token_opt = if i + 1 < njd_features.len() {
                                Some(&njd_features[i + 1])
                            } else {
                                None
                            };

                            // 後続が1文字の漢字なら巻き込んで再解析
                            let (target_text, consumed_next) = if let Some(next) = next_token_opt {
                                if is_single_kanji_token(next) {
                                    (format!("{}{}", last_char, next.orig), true)
                                } else {
                                    (last_char.to_string(), false)
                                }
                            } else {
                                (last_char.to_string(), false)
                            };

                            reanalysis_result = Some((target_text, consumed_next));
                        }
                    }
                }
            }

            // 再解析実行と適用
            if let Some((text, consumed_next)) = reanalysis_result {
                let analyzed = open_jtalk.run_frontend(&text)?;

                let range_end = if consumed_next { i + 2 } else { i + 1 };
                let analyzed_len = analyzed.len();

                if range_end <= njd_features.len() {
                    njd_features.splice(i..range_end, analyzed);

                    if !consumed_next && analyzed_len > 0 {
                        let feat = &mut njd_features[i];
                        feat.pos = "名詞".to_string();
                        feat.pos_group1 = "一般".to_string();
                        feat.pos_group2 = "*".to_string();
                        feat.pos_group3 = "*".to_string();
                        feat.ctype = "*".to_string();
                        feat.cform = "*".to_string();
                        i += 1;
                    } else {
                        i += analyzed_len;
                    }
                    continue;
                }
            }

            // 連続踊り字の展開処理
            let start = i;
            let mut end = i;
            let mut total_odori = 0;
            while end < njd_features.len() && is_dancing(&njd_features[end].orig) {
                total_odori += count_odori(&njd_features[end].orig);
                end += 1;
            }

            // 直前の漢字トークンを収集
            let mut normal_indices = Vec::new();
            let mut j = start;
            let mut collected_chars = 0;
            let needed_chars = if total_odori >= 2 { 2 } else { 1 };

            while j > 0 {
                j -= 1;
                if is_kanji_token(&njd_features[j]) {
                    normal_indices.push(j);
                    collected_chars += njd_features[j].orig.chars().count();
                    if collected_chars >= needed_chars {
                        break;
                    }
                }
            }
            normal_indices.reverse();

            if normal_indices.is_empty() {
                i = end;
                continue;
            }

            // 置換用データの作成
            let is_single_kanji = normal_indices.len() == 1
                && njd_features[normal_indices[0]].orig.chars().count() == 1;

            let (base_read, base_pron, base_mora_size) = if is_single_kanji {
                let f = &njd_features[normal_indices[0]];
                (f.read.clone(), f.pron.clone(), f.mora_size)
            } else {
                let mut r = String::new();
                let mut p = String::new();
                let mut m = 0;
                for &idx in &normal_indices {
                    r.push_str(&njd_features[idx].read);
                    p.push_str(&njd_features[idx].pron);
                    m += njd_features[idx].mora_size;
                }
                (r, p, m)
            };

            // 踊り字トークンの書き換え
            (start..end).for_each(|k| {
                let current_odori = count_odori(&njd_features[k].orig);
                let feat = &mut njd_features[k];

                if is_single_kanji {
                    feat.read = base_read.repeat(current_odori);
                    feat.pron = base_pron.repeat(current_odori);
                    feat.mora_size = base_mora_size * current_odori as i32;
                } else {
                    feat.read = base_read.clone();
                    feat.pron = base_pron.clone();
                    feat.mora_size = base_mora_size;
                }

                if feat.pos == "記号" {
                    feat.pos = "名詞".to_string();
                    feat.pos_group1 = "一般".to_string();
                    feat.pos_group2 = "*".to_string();
                    feat.pos_group3 = "*".to_string();
                    feat.ctype = "*".to_string();
                    feat.cform = "*".to_string();
                }
            });
            i = end;
        } else if is_odoriji(&orig) {
            // 一の字点（ゝ、ゞ、ヽ、ヾ）の処理
            if i > 0 {
                // 直前が記号でないか
                if njd_features[i - 1].pos != "記号" {
                    let mut prev_index = None;
                    let mut k = i;
                    while k > 0 {
                        k -= 1;
                        if njd_features[k].pos != "記号" && njd_features[k].mora_size > 0 {
                            prev_index = Some(k);
                            break;
                        }
                    }

                    if let Some(pidx) = prev_index {
                        let prev_read = njd_features[pidx].read.clone();
                        let prev_pron = njd_features[pidx].pron.clone();
                        let prev_mora_size = njd_features[pidx].mora_size;

                        let curr = &mut njd_features[i];
                        apply_odoriji_logic(curr, &prev_read, &prev_pron, prev_mora_size);
                    }
                }
            }
            i += 1;
        } else {
            i += 1;
        }
    }

    Ok(())
}

#[inline(always)]
fn is_dancing(orig: &str) -> bool {
    !orig.is_empty() && orig.chars().all(|c| c == '々')
}

#[inline(always)]
fn is_odoriji(orig: &str) -> bool {
    !orig.is_empty() && orig.chars().all(|c| matches!(c, 'ゝ' | 'ゞ' | 'ヽ' | 'ヾ'))
}

#[inline(always)]
fn count_odori(orig: &str) -> usize {
    orig.chars().filter(|&c| c == '々').count()
}

#[inline(always)]
fn is_kanji(c: char) -> bool {
    ('\u{4E00}'..='\u{9FFF}').contains(&c)
}

#[inline(always)]
fn is_kanji_token(token: &NjdFeature) -> bool {
    if token.pos == "記号" {
        return false;
    }
    token.orig.chars().any(is_kanji)
}

#[inline(always)]
fn is_single_kanji_token(token: &NjdFeature) -> bool {
    is_kanji_token(token)
        && token.orig.chars().count() == 1
        && is_kanji(token.orig.chars().next().unwrap())
}

/// 一の字点のロジック適用（読み分割、濁点化など）
#[inline(always)]
fn apply_odoriji_logic(
    odori_feature: &mut NjdFeature,
    prev_read: &str,
    prev_pron: &str,
    prev_mora_size: i32,
) {
    // 読みと発音をモーラ単位（小書き文字考慮）で分割
    let prev_read_chars = split_kana_mora(prev_read);

    // 発音記号からアクセント境界'を除去
    let prev_pron_source = prev_pron.replace('’', "");
    let prev_pron_source = if prev_pron_source.is_empty() {
        prev_read
    } else {
        &prev_pron_source
    };
    let prev_pron_chars = split_kana_mora(prev_pron_source);

    // モーラサイズ計算 (最後の文字のものを使用するロジック)
    // Python版では全文字に等分しているが、最終的に使うのは最後の文字の値のみ
    if prev_read_chars.is_empty() {
        return;
    }

    let mora_val = prev_mora_size / prev_read_chars.len() as i32;

    let target_read = prev_read_chars.last().unwrap();
    let target_pron = prev_pron_chars.last().unwrap_or(target_read); // pronが短すぎる場合のフォールバック

    let odori_char = odori_feature.orig.chars().next().unwrap_or('ゝ');

    if ['ゝ', 'ヽ'].contains(&odori_char) {
        // 清音化
        odori_feature.read = TO_SEION
            .get(target_read)
            .unwrap_or(&target_read.as_str())
            .to_string();
        odori_feature.pron = TO_SEION
            .get(target_pron)
            .unwrap_or(&target_pron.as_str())
            .to_string();
        odori_feature.mora_size = mora_val;
    } else if ['ゞ', 'ヾ'].contains(&odori_char) {
        // 濁音化
        odori_feature.read = TO_DAKUON
            .get(target_read)
            .unwrap_or(&target_read.as_str())
            .to_string();
        odori_feature.pron = TO_DAKUON
            .get(target_pron)
            .unwrap_or(&target_pron.as_str())
            .to_string();
        odori_feature.mora_size = mora_val;
    }

    if odori_feature.pos == "記号" {
        odori_feature.pos = "名詞".to_string();
        odori_feature.pos_group1 = "一般".to_string();
        odori_feature.pos_group2 = "*".to_string();
        odori_feature.pos_group3 = "*".to_string();
        odori_feature.ctype = "*".to_string();
        odori_feature.cform = "*".to_string();
    }
}

static SMALL_KANA: LazyLock<HashSet<char>> =
    LazyLock::new(|| ['ャ', 'ュ', 'ョ', 'ァ', 'ィ', 'ゥ', 'ェ', 'ォ'].into());

/// 文字列をモーラ単位（小書き文字を前の文字に結合）で分割
#[inline(always)]
fn split_kana_mora(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut result = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if i + 1 < chars.len() && SMALL_KANA.contains(&chars[i + 1]) {
            result.push(format!("{}{}", c, chars[i + 1]));
            i += 2;
        } else {
            result.push(c.to_string());
            i += 1;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modify_acc_after_chaining_mut() {
        let mut features = [
            NjdFeature {
                string: "参り".to_string(),
                pos: "動詞".to_string(),
                pos_group1: "自立".to_string(),
                pos_group2: "*".to_string(),
                pos_group3: "*".to_string(),
                ctype: "五段・ラ行".to_string(),
                cform: "連用形".to_string(),
                orig: "参る".to_string(),
                read: "マイリ".to_string(),
                pron: "マイリ".to_string(),
                acc: 1,
                mora_size: 3,
                chain_rule: "*".to_string(),
                chain_flag: -1,
            },
            NjdFeature {
                string: "ます".to_string(),
                pos: "助動詞".to_string(),
                pos_group1: "*".to_string(),
                pos_group2: "*".to_string(),
                pos_group3: "*".to_string(),
                ctype: "特殊・マス".to_string(),
                cform: "基本形".to_string(),
                orig: "ます".to_string(),
                read: "マス".to_string(),
                pron: "マス’".to_string(),
                acc: 1,
                mora_size: 2,
                chain_rule: "動詞%F2@1/助詞%F2@1".to_string(),
                chain_flag: 1,
            },
        ];

        modify_acc_after_chaining(&mut features);

        let 参り = features.first().unwrap();
        assert_eq!(参り.acc, 4);
    }
}
