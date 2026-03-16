use std::{
    borrow::Cow,
    fs::Metadata,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};
use unicode_normalization::{IsNormalized, UnicodeNormalization as _, is_nfc_quick, is_nfkc_quick};
use vibrato_rkyv::tokenizer::worker::Worker;

use crate::{
    Haqumei, NjdFeature, UnicodeNormalization, VIBRATO_CACHE,
    data::{MULTI_READ_KANJI_LIST, TO_DAKUON, TO_SEION, TO_SEION_CHAR},
    errors::HaqumeiError,
    features::UnidicFeature,
    open_jtalk::OpenJTalk,
};

/// カタカナをひらがなに変換する
pub fn kata2hira(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'ァ'..='ヶ' | 'ヽ'..='ヾ' => std::char::from_u32(c as u32 - 0x60).unwrap_or(c),
            _ => c,
        })
        .collect()
}

/// ひらがなをカタカナに変換する
pub fn hira2kata(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'ぁ'..='ゖ' | 'ゝ'..='ゞ' => std::char::from_u32(c as u32 + 0x60).unwrap_or(c),
            _ => c,
        })
        .collect()
}

#[rustfmt::skip]
pub fn is_dakuon(c: char) -> bool {
    matches!(
        c,
        'が' | 'ぎ' | 'ぐ' | 'げ' | 'ご'
        | 'ざ' | 'じ' | 'ず' | 'ぜ' | 'ぞ'
        | 'だ' | 'ぢ' | 'づ' | 'で' | 'ど'
        | 'ば' | 'び' | 'ぶ' | 'べ' | 'ぼ'
        | 'ガ' | 'ギ' | 'グ' | 'ゲ' | 'ゴ'
        | 'ザ' | 'ジ' | 'ズ' | 'ゼ' | 'ゾ'
        | 'ダ' | 'ヂ' | 'ヅ' | 'デ' | 'ド'
        | 'バ' | 'ビ' | 'ブ' | 'ベ' | 'ボ'
        | 'ヴ'
    )
}

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
pub(crate) fn modify_filler_accent(njd_features: &mut [NjdFeature]) {
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
    #[inline(always)]
    pub(crate) fn normalize_unicode_if_needed<'a>(&self, text: &'a str) -> Cow<'a, str> {
        match self.options.normalize_unicode {
            UnicodeNormalization::None => Cow::Borrowed(text),
            UnicodeNormalization::Nfc => {
                if is_nfc_quick(text.chars()) == IsNormalized::Yes {
                    Cow::Borrowed(text)
                } else {
                    Cow::Owned(text.nfc().collect::<String>())
                }
            }
            UnicodeNormalization::Nfkc => {
                if is_nfkc_quick(text.chars()) == IsNormalized::Yes {
                    Cow::Borrowed(text)
                } else {
                    Cow::Owned(text.nfkc().collect::<String>())
                }
            }
        }
    }

    pub(crate) fn revert_pron_to_read(&mut self, njd_features: &mut [NjdFeature]) {
        let options = &self.options;
        debug_assert!(
            options.use_read_as_pron || options.revert_long_vowels || options.revert_yotsugana
        );

        for feature in njd_features.iter_mut() {
            let should_revert_to_read = options.use_read_as_pron
                || (options.revert_long_vowels
                    && feature.pron.contains('ー')
                    && !feature.orig.contains('ー'))
                || (options.revert_yotsugana
                    && (feature.read.contains('ヅ') || feature.read.contains('ヂ')));

            if should_revert_to_read {
                feature.pron = feature.read.clone();
            }
        }
    }

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
pub(crate) fn modify_acc_after_chaining(njd_features: &mut [NjdFeature]) {
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

// 文字列を静音化し、末尾の「々」に対応する繰り返し単位を検出する
fn detect_odori_unit(read: &str) -> Option<usize> {
    let seion_read: String = read
        .chars()
        .map(|ch| {
            if is_dakuon(ch) {
                TO_SEION_CHAR.get(&ch).copied().unwrap_or(ch)
            } else {
                ch
            }
        })
        .collect();
    let moras = split_kana_mora(&seion_read);
    let n = moras.len();
    if n < 2 {
        return None;
    }

    // 後ろ半分が前半分と一致する最小の単位を探す
    for len in 1..=(n / 2) {
        let first_half = &moras[n - len * 2..n - len];
        let second_half = &moras[n - len..n];
        if first_half == second_half {
            return Some(len);
        }
    }
    None
}

/// 踊り字（々）と一の字点（ゝ、ゞ、ヽ、ヾ）の読みを処理する後処理関数
pub(crate) fn process_odori_features(
    njd_features: &mut Vec<NjdFeature>,
    open_jtalk: &mut OpenJTalk,
) -> Result<(), HaqumeiError> {
    let mut i = 0;
    while i < njd_features.len() {
        let orig = &njd_features[i].orig;
        if is_dancing(orig) {
            // 踊り字「々」の処理
            let mut reanalysis_result = None;
            if i > 0 {
                let prev = &njd_features[i - 1];
                if count_odori(orig) == 1 && is_kanji_token(prev) {
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
                let mut analyzed = open_jtalk.run_frontend(&text)?;

                if let Some(first) = analyzed.get_mut(0) {
                    first.chain_flag = 1;
                }

                let range_end = if consumed_next { i + 2 } else { i + 1 };
                let analyzed_len = analyzed.len();

                if range_end <= njd_features.len() {
                    njd_features.splice(i..range_end, analyzed);

                    if !consumed_next && analyzed_len > 0 {
                        set_to_noun(&mut njd_features[i]);
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

            if i > 0 && njd_features[i - 1].orig.ends_with('々') {
                let prev = &njd_features[i - 1];
                let base_acc = prev.acc;

                // 清音ベースで「繰り返しの長さ」を特定
                if let Some(period) = detect_odori_unit(&prev.read) {
                    let raw_read_moras = split_kana_mora(&prev.read);
                    let raw_pron_moras = split_kana_mora(&prev.pron);

                    if raw_read_moras.len() >= period {
                        let unit_read = raw_read_moras[raw_read_moras.len() - period..].join("");
                        let unit_pron = raw_pron_moras[raw_pron_moras.len() - period..].join("");
                        let unit_mora =
                            (prev.mora_size / raw_read_moras.len() as i32) * period as i32;

                        let current_feat = &mut njd_features[i];
                        let count = count_odori(&current_feat.orig);

                        current_feat.read = unit_read.repeat(count);
                        current_feat.pron = unit_pron.repeat(count);
                        current_feat.mora_size = unit_mora * count as i32;

                        current_feat.acc = base_acc;
                        current_feat.chain_flag = 1;

                        if current_feat.pos == "記号" {
                            set_to_noun(current_feat);
                        }
                        i += 1;
                        continue;
                    }
                }
            }

            // 直前の漢字トークンを収集
            let mut normal_indices = Vec::new();
            let mut j = start;
            let mut collected_chars = 0;
            let needed_chars = total_odori.min(8);

            while j > 0 {
                j -= 1;
                let target = &njd_features[j];

                if matches!(target.pos.as_str(), "記号" | "フィラー" | "感動詞") {
                    break;
                }

                if is_kanji_token(target) {
                    normal_indices.push(j);
                    collected_chars += target.orig.chars().count();
                    if collected_chars >= needed_chars {
                        break;
                    }
                } else {
                    break;
                }
            }
            normal_indices.reverse();

            if normal_indices.is_empty() {
                i = end;
                continue;
            }

            let base_acc = njd_features[normal_indices[0]].acc;

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

            for mut njd_feature in njd_features.iter_mut().take(end).skip(start) {
                let current_odori = count_odori(&njd_feature.orig);
                let feat = &mut njd_feature;

                if is_single_kanji {
                    feat.read = base_read.repeat(current_odori);
                    feat.pron = base_pron.repeat(current_odori);
                    feat.mora_size = base_mora_size * current_odori as i32;
                } else {
                    feat.read = base_read.clone();
                    feat.pron = base_pron.clone();
                    feat.mora_size = base_mora_size;
                }
                feat.acc = base_acc; // 直前の漢字トークンの acc を使う
                feat.chain_flag = 1;

                if feat.pos == "記号" {
                    set_to_noun(feat);
                }
            }
            i = end;
        } else if is_odoriji(orig) {
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
fn set_to_noun(feat: &mut NjdFeature) {
    feat.pos = "名詞".to_string();
    feat.pos_group1 = "一般".to_string();
    feat.pos_group2 = "*".to_string();
    feat.pos_group3 = "*".to_string();
    feat.ctype = "*".to_string();
    feat.cform = "*".to_string();
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
    token.pos != "記号" && token.orig.chars().any(is_kanji)
}

#[inline(always)]
fn is_single_kanji_token(token: &NjdFeature) -> bool {
    is_kanji_token(token)
        && token.orig.chars().count() == 1
        && is_kanji(token.orig.chars().next().unwrap())
}

#[inline(always)]
fn is_small_kana(c: char) -> bool {
    matches!(c, 'ャ' | 'ュ' | 'ョ' | 'ァ' | 'ィ' | 'ゥ' | 'ェ' | 'ォ')
}

/// 文字列をモーラ単位（小書き文字を前の文字に結合）で分割
#[inline(always)]
fn split_kana_mora(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut result = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if i + 1 < chars.len() && is_small_kana(chars[i + 1]) {
            result.push(format!("{}{}", c, chars[i + 1]));
            i += 2;
        } else {
            result.push(c.to_string());
            i += 1;
        }
    }
    result
}

fn apply_odoriji_logic(
    odori_feature: &mut NjdFeature,
    prev_read: &str,
    prev_pron: &str,
    prev_mora_size: i32,
) {
    let prev_read_mora = split_kana_mora(prev_read);
    let prev_pron_source = if prev_pron.contains('’') {
        Cow::Owned(prev_pron.replace('’', ""))
    } else {
        Cow::Borrowed(prev_pron)
    };
    let prev_pron_source = if prev_pron_source.is_empty() {
        prev_read
    } else {
        prev_pron_source.as_ref()
    };
    let prev_pron_mora = split_kana_mora(prev_pron_source);

    if prev_read_mora.is_empty() {
        return;
    }

    let mora_val = prev_mora_size / prev_read_mora.len() as i32;
    let target_read = prev_read_mora.last().unwrap().clone();
    let target_pron = prev_pron_mora.last().unwrap_or(&target_read).clone();

    let mut is_forced_voiced = false;
    for c in odori_feature.orig.chars().peekable() {
        if matches!(c, 'ゞ' | 'ヾ') {
            is_forced_voiced = true;
            break;
        }
        if matches!(c, 'ゝ' | 'ヽ') {
            break;
        }
    }

    // 対象モーラが単一の仮名 grapheme か判定する。
    //
    // 一の字点 (ゝ, ゞ, ヽ, ヾ) は歴史的に「直前の仮名1文字」を
    // 繰り返す記号であり、拗音 (きゃ, しゃ 等) のような
    // 複数仮名からなるモーラに対して使われる例はほぼ存在しない。
    //
    // そのため厳密な規則を定義するのは難しく、実際のテキストでも
    // 拗音に対して踊り字が使われるケースは想定しにくい。
    //
    // 二字以上扱う [くの字点](https://ja.wikipedia.org/wiki/踊り字#〱（くの字点）) についても、
    //
    // > 濁点の付く文字を繰り返す場合は、濁点の付いていない「くの字点」を用いる場合と、濁点の付いている「くの字点」を用いる場合がある。
    //
    // とあって別に厳密にルール付けることはできないし、
    // これに文脈に合わせた推定をするロジックを書くぐらいならもっとやった方がいいことがある。
    // でも需要がありそうなのは濁音を維持して繰り返すケースっぽそう。

    let is_single_grapheme_mora = {
        let mut chars = target_read.chars();
        !chars.any(is_small_kana)
    };

    if is_forced_voiced {
        // 濁音の踊り字 (ゞ, ヾ) -> 強制的に濁音化
        odori_feature.read = TO_DAKUON
            .get(&target_read)
            .copied()
            .unwrap_or(&target_read)
            .to_string();
        odori_feature.pron = TO_DAKUON
            .get(&target_pron)
            .copied()
            .unwrap_or(&target_pron)
            .to_string();
    } else {
        // 清音の踊り字 (ゝ, ヽ)
        if is_single_grapheme_mora {
            // 対象が単一文字の場合 -> 清音化
            odori_feature.read = TO_SEION
                .get(&target_read)
                .copied()
                .unwrap_or(&target_read)
                .to_string();
            odori_feature.pron = TO_SEION
                .get(&target_pron)
                .copied()
                .unwrap_or(&target_pron)
                .to_string();
        } else {
            // 対象が拗音などの複数文字の場合 -> 濁点を維持する
            odori_feature.read = target_read.to_string();
            odori_feature.pron = target_pron.to_string();
        }
    }

    odori_feature.mora_size = mora_val;
    if odori_feature.pos == "記号" {
        set_to_noun(odori_feature);
    }
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

    #[test]
    fn test_hira2kata_basic() {
        assert_eq!(hira2kata("あいうえお"), "アイウエオ");
        assert_eq!(hira2kata("ぱぴぷぺぽ"), "パピプペポ");
        assert_eq!(hira2kata("ちゃちゅちょ"), "チャチュチョ");
        assert_eq!(hira2kata("っ"), "ッ");
    }

    #[test]
    fn test_kata2hira_basic() {
        assert_eq!(kata2hira("アイウエオ"), "あいうえお");
        assert_eq!(kata2hira("パピプペポ"), "ぱぴぷぺぽ");
        assert_eq!(kata2hira("チャチュチョ"), "ちゃちゅちょ");
        assert_eq!(kata2hira("ッ"), "っ");
    }

    #[test]
    fn test_precomposed_dakuten() {
        // 「が」(U+304C) <-> 「ガ」(U+30AC) の差は 0x60 なので正しく変換される
        assert_eq!(hira2kata("がぎぐげご"), "ガギグゲゴ");
        assert_eq!(kata2hira("ガギグゲゴ"), "がぎぐげご");
    }

    #[test]
    fn test_combining_dakuten() {
        // 結合濁点(U+3099)や結合半濁点(U+309A)は、変換範囲外のため「そのまま」残る
        let hira_combined = "か\u{3099}"; // か + 結合濁点
        let kata_combined = "カ\u{3099}"; // カ + 結合濁点

        assert_eq!(hira2kata(hira_combined), kata_combined);
        assert_eq!(kata2hira(kata_combined), hira_combined);
    }

    #[test]
    fn test_edge_cases() {
        // ひらがな: ぁ(3041) 〜 ゖ(3096)
        // カタカナ: ァ(30A1) 〜 ヶ(30F6)
        assert_eq!(hira2kata("ぁ"), "ァ");
        assert_eq!(hira2kata("ゖ"), "ヶ");
        assert_eq!(kata2hira("ァ"), "ぁ");
        assert_eq!(kata2hira("ヶ"), "ゖ");

        // 「ゔ」(U+3094) <-> 「ヴ」(U+30F4) も 0x60 差なので範囲内
        assert_eq!(hira2kata("ゔ"), "ヴ");
        assert_eq!(kata2hira("ヴ"), "ゔ");
    }

    #[test]
    fn test_non_target_characters() {
        let mixed = "あ漢123!ー A";
        assert_eq!(hira2kata(mixed), "ア漢123!ー A");
        assert_eq!(kata2hira("ア漢123!ー A"), mixed);

        assert_eq!(hira2kata("ｱｲｳ"), "ｱｲｳ");
    }

    #[test]
    fn test_choonpu() {
        assert_eq!(hira2kata("らーめん"), "ラーメン");
        assert_eq!(kata2hira("ラーメン"), "らーめん");
    }

    #[test]
    fn test_special_hira() {
        // 「ゐ」(U+3090) / 「ゑ」(U+3091)
        // 「ヰ」(U+30F0) / 「ヱ」(U+30F1)
        assert_eq!(hira2kata("ゐゑ"), "ヰヱ");
        assert_eq!(kata2hira("ヰヱ"), "ゐゑ");
    }
}
