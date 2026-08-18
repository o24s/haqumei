//! 辞書に無い漢字にフォールバックの読みを与える。
//!
//! Open JTalk の NJD は、読みを決められなかった語を「記号-読点」に格下げして
//! 読みを `、` にする。`g2k` は pyopenjtalk と同じく記号については表層形を
//! 出すので、辞書に無い漢字がカナ列にそのまま混入する。
//!
//! ```text
//! 騸馬   -> 騸馬        (「センバ」ではなく漢字がそのまま出る)
//! 嗅神経  -> 嗅シンケー
//! ```
//!
//! 辞書が単独で読める漢字は約 3,100 字しかないため、それ以外はすべてこうなる。
//!
//! 入力の段階で異体字を新字体に直すような正規化は、辞書が知っている語を壊してしまう。
//! 「醫學部」は辞書に複合語として載っていて正しく `イガクブ` と読めるのに、
//! `醫` だけを `医` に直すと `医學部` という未知語になり `イマナブブ` に崩れる。
//!
//! ここでは NJD が読みを諦めた印 (記号-読点 かつ読みが `、`) が付いた語だけを
//! 対象にする。 (既に壊れている出力しか触らない)
//!
//! # 限界
//!
//! 1 字 1 読みの表なので、文脈による読み分けも連濁も熟字訓も扱えない。
//! `悪魔憑き` は `アクマヒョウキ` になり、正しい `アクマツキ` にはならない。

use crate::NjdFeature;
use crate::utils::{count_mora, is_kanji, read_to_pron};

/// 漢字から読みを引く表。出典とライセンスは `haqumei/data/unihan/README.md`。
static READINGS: phf::Map<char, &'static str> = include!("../../data/unihan/readings.rs");

/// NJD が読みを諦めた語に、1 文字ずつ読みを与える。
pub(crate) fn read_unknown_kanji(njd_features: &mut [NjdFeature]) {
    for feature in njd_features.iter_mut() {
        // NJD が諦めた印。本物の句読点は string が `、` なので漢字を含まず、
        // 下の判定で弾かれる
        if feature.pos != "記号" || feature.pos_group1 != "読点" {
            continue;
        }
        if !feature.string.chars().any(is_kanji) {
            continue;
        }

        // 1 文字でも読みを引けなければ、中途半端な結果を作らずに諦める
        let mut reading = String::with_capacity(feature.string.len());
        for c in feature.string.chars() {
            match READINGS.get(&c).copied() {
                Some(r) => reading.push_str(r),
                None if !is_kanji(c) => reading.push(c),
                None => {
                    reading.clear();
                    break;
                }
            }
        }
        if reading.is_empty() {
            continue;
        }

        // READINGS は正書法の読みなので、発音は長音を `ー` に直してから入れる
        feature.pron = read_to_pron(&reading);
        feature.read = reading;
        // 記号のままだとポーズとして扱われるので、名詞に戻す
        feature.pos = "名詞".to_string();
        feature.pos_group1 = "一般".to_string();
        feature.pos_group2 = "*".to_string();
        feature.pos_group3 = "*".to_string();
        feature.mora_size = count_mora(&feature.pron) as i32;
        feature.acc = 0;
        feature.chain_flag = -1;
    }
}
