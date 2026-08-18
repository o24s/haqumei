//! 数詞まわりの読みの補正。
//!
//! いずれも隣接する形態素の並びで読みが決まるが、`context_reading` の決定リストが
//! 扱う「直前・直後の 1 形態素を見て固定の読みを与える」形には収まらない。
//! 2 つ先まで見る必要があったり、既存の読みを条件付きで書き換えたりするため、
//! ここに個別の処理として置いている。

use crate::NjdFeature;
use crate::utils::count_mora;

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

/// 2 つ以上続く「〇」を伏字として `マル` と読む。
///
/// NJD は「〇」を数詞として扱うので、既定では `ゼロ` になる。しかし 2 つ以上
/// 続く「〇」は数値ではなく伏字なので `マル` が正しい (〇〇株式会社 =
/// マルマルカブシキガイシャ)。
///
/// 負の対照: 単独の「〇円」は数詞のままにして、従来の読みを保つ。
pub(crate) fn modify_placeholder_maru(njd_features: &mut [NjdFeature]) {
    const MARU: &str = "マル";

    for i in 0..njd_features.len().saturating_sub(1) {
        if njd_features[i].string != "〇" || njd_features[i + 1].string != "〇" {
            continue;
        }
        for feature in &mut njd_features[i..=i + 1] {
            // 数詞のままだと後段の数値処理に巻き込まれる
            feature.pos_group1 = "一般".to_string();
            feature.read = MARU.to_string();
            feature.pron = MARU.to_string();
            feature.acc = 1;
            feature.mora_size = count_mora(MARU) as i32;
        }
    }
}
