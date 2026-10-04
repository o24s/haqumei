//! 数詞まわりの読みの補正。
//!
//! いずれも隣接する形態素の並びで読みが決まるが、`context_reading` の決定リストが
//! 扱う「直前・直後の 1 形態素を見て固定の読みを与える」形には収まらない。
//! 2 つ先まで見る必要があったり、既存の読みを条件付きで書き換えたりするため、
//! ここに個別の処理として置いている。

use crate::NjdFeature;
use crate::utils::count_mora;

/// 数量の一組・二組をヒトクミ・フタクミと読みます。
/// 「数詞 + 年」と「第」の直後では、組番号の読みを維持します。
pub(crate) fn modify_group_reading(features: &mut [NjdFeature], protected: &[bool]) {
    for i in 0..features.len() {
        let combined = features[i].string == "一組"
            && features[i].read == "イチクミ"
            && features[i].pos == "名詞"
            && features[i].pos_group1 == "一般";
        let separate = features[i].pos_group1 == "数"
            && matches!(features[i].string.as_str(), "一" | "二")
            && features.get(i + 1).is_some_and(|next| {
                next.string == "組" && next.read == "クミ" && next.pos_group2 == "助数詞"
            });
        if !(combined || separate)
            || protected.get(i).copied().unwrap_or(false)
            || (separate && protected.get(i + 1).copied().unwrap_or(false))
        {
            continue;
        }
        let previous = i.checked_sub(1).map(|p| &features[p]);
        if previous.is_some_and(|p| {
            p.pos_group1 == "数"
                || p.string == "第"
                || (matches!(p.string.as_str(), "．" | "・" | "点" | "一点")
                    && p.read.ends_with("テン"))
        }) || follows_school_year(features, i)
        {
            continue;
        }
        let (before, after) = if combined {
            ("イチクミ", "ヒトクミ")
        } else if features[i].string == "一" {
            ("イチ", "ヒト")
        } else {
            ("ニ", "フタ")
        };
        if features[i].pron != before {
            continue;
        }
        let old_moras = features[i].mora_size;
        features[i].read = after.into();
        features[i].pron = after.into();
        features[i].mora_size = count_mora(after) as i32;
        let delta = features[i].mora_size - old_moras;
        if delta != 0 {
            // 二組のニをフタにすると、C3 が置いた数詞末尾の核も 1 モーラ後ろへ動く。
            let mut head = i;
            while head > 0 && features[head].chain_flag == 1 {
                head -= 1;
            }
            let end = features[head..i].iter().map(|f| f.mora_size).sum::<i32>() + old_moras;
            if features[head].acc > 0 && features[head].acc >= end {
                features[head].acc += delta;
            }
        }
    }
}

fn follows_school_year(features: &[NjdFeature], i: usize) -> bool {
    let Some(previous) = i.checked_sub(1) else {
        return false;
    };
    let year = &features[previous];
    if year.string == "年" && year.read == "ネン" {
        return previous > 0 && features[previous - 1].pos_group1 == "数";
    }
    // 辞書が「一年」を一語として返す場合も、学年・組番号の並びを維持する。
    year.string.strip_suffix('年').is_some_and(|number| {
        !number.is_empty()
            && number.chars().all(|c| {
                matches!(c, '0'..='9' | '０'..='９' | '〇' | '零' | '一' | '二' | '三' | '四'
                | '五' | '六' | '七' | '八' | '九' | '十' | '百' | '千')
            })
    })
}

/// 分数の分母に来る「分」を `ブン` と読む。
///
/// 「分」は時間量の `フン`/`プン`、割合の `ブ`、部分の `ブン` を持つ。
/// 分母だけは「数値 + 分 + の + 数値」という並びで決まるので、そこだけを直す。
///
/// ```text
/// 三分の一   三分 (サンブ)   + の + 一   -> サンブン
/// 四分の三   四分 (ヨンプン)  + の + 三   -> ヨンブン
/// 3分の1    三 + 分 (プン)  + の + 一   -> ブン
/// ```
///
/// 負の対照: 「五分で着く」「五分五分」は後ろが「の + 数詞」でないので発火しない。
pub(crate) fn modify_fraction_denominator(njd_features: &mut [NjdFeature]) {
    for i in 0..njd_features.len() {
        // 「の + 数詞」が続くことが分母の条件
        let is_denominator = njd_features.get(i + 1).is_some_and(|n| n.string == "の")
            && njd_features
                .get(i + 2)
                .is_some_and(|n| n.pos_group1 == "数");
        if !is_denominator {
            continue;
        }

        let feature = &mut njd_features[i];
        if feature.string.ends_with('分') {
            // 「三分」「四分」のように 1 形態素になっている場合
            let new_pron = if let Some(stem) = feature
                .pron
                .strip_suffix("フン")
                .or_else(|| feature.pron.strip_suffix("プン"))
            {
                format!("{stem}ブン")
            } else if let Some(stem) = feature.pron.strip_suffix('ブ') {
                format!("{stem}ブン")
            } else {
                continue;
            };
            feature.read = new_pron.clone();
            feature.pron = new_pron;
            feature.mora_size = count_mora(&feature.pron) as i32;
        } else if feature.string == "分" && i > 0 && njd_features[i - 1].pos_group1 == "数" {
            // 算用数字が別形態素になり「分」が単独で現れる場合
            let feature = &mut njd_features[i];
            feature.read = "ブン".to_string();
            feature.pron = "ブン".to_string();
            feature.mora_size = count_mora(&feature.pron) as i32;
        }
    }
}
