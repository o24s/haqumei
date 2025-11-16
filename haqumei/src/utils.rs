use std::{fs::File, io::Read, path::Path};

use sha2::{Digest, Sha256};
use vibrato_rkyv::tokenizer::worker::Worker;

use crate::{Haqumei, NjdFeature, VIBRATO_CACHE, data::MULTI_READ_KANJI_LIST, errors::HaqumeiError, features::UnidicFeature};

pub fn calculate_compiled_dir_hash(dir: &Path) -> Result<String, HaqumeiError> {
    let mut hasher = Sha256::new();
    let mut paths = Vec::new();

    for entry in walkdir::WalkDir::new(dir).sort_by_file_name() {
        let entry = entry?;
        let path = entry.path();
        if path.is_file()
            && let Some(extension) = path.extension()
            && (extension == "dic" || extension == "bin") {
                paths.push(path.to_path_buf());
            }
    }

    paths.sort();

    for path in paths {
        let relative_path = path.strip_prefix(dir)?;
        hasher.update(relative_path.to_string_lossy().as_bytes());

        let mut file = File::open(&path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        hasher.update(&buffer);
    }

    Ok(hex::encode(hasher.finalize()))
}


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
    pub(crate) fn modify_kanji_yomi(
        &mut self,
        text: &str,
        njd_features: &mut [NjdFeature],
    ) {
        let tokens: Vec<UnidicFeature> = VIBRATO_CACHE.get(text).unwrap_or( {
            let mut worker = self.tokenizer.new_worker();
            vibrato_analysis(&mut worker, text)
        }).into_iter()
            .filter(|t| MULTI_READ_KANJI_LIST.contains(t.surface.as_str()))
            .collect();

        if tokens.is_empty() {
            return;
        }

        let mut unidic_iter = tokens.into_iter().peekable();
        let mut current_char_pos = 0;
        for i in 0..njd_features.len() {
            let node_string = &njd_features[i].string;
            let node_orig = &njd_features[i].orig;
            let node_char_len = node_string.chars().count();
            let next_node_feature = njd_features.get(i + 1);

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
                && candidate.range_char.start == current_char_pos && candidate.surface == *node_orig {
                    let correct_yomi_token = unidic_iter.next().unwrap();

                    if false { // *node_orig == "何"
                        let is_read_nan = self.predict_is_nan(next_node_feature);
                        let yomi = if is_read_nan { "ナン" } else { "ナニ" };
                        pron_to_set = Some(yomi.to_string());
                        read_to_set = Some(yomi.to_string());
                    } else {
                        let reading = correct_yomi_token.pron();
                        pron_to_set = Some(reading.to_string());
                        read_to_set = Some(reading.to_string());
                    }
                }
            if let Some(pron) = pron_to_set {
                njd_features[i].pron = pron;
            }
            if let Some(read) = read_to_set {
                njd_features[i].read = read;
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
                    .is_some_and(|nuc_pron| INAPPROPRIATE_FOR_NUCLEAR_CHARS.contains(&nuc_pron)) {
                        njd_features[head_index].acc = njd_features[head_index].acc.saturating_sub(1);
                    }

                acc = -1;
            } else {
                acc -= njd_features[i].mora_size;
            }
        }
    }
}
