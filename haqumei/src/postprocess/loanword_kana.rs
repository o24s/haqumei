//! 辞書が別の仮名に置き換えている外来語表記の仮名を、表層形から復元する。
//!
//! [`crate::Phoneme`] と `haqumei_jpreprocess_core::pronunciation` は `ヴィ` `テュ` `クィ` など 39 種を
//! すべて表現でき、Open JTalk の読み変換表も小書きのカナを保っている。区別を失って
//! いるのは辞書のエントリだけで、`unidic-csj` は 20 種を一貫してバ行などの
//! 一般的な仮名に置き換えている。
//!
//! ```text
//! ヴィクトリーヌ  pron=ビクトリーヌ  -> ヴィクトリーヌ
//! アイシュヴァルヤ pron=アイシュバルヤ -> アイシュヴァルヤ
//! ```
//!
//! # 表層形から機械的に作り直さない
//!
//! 置き換えた形のほうが日本語として定着している語が多くある。
//!
//! ```text
//! ホンデュラス -> ホンジュラス, バースディ -> バースデイ
//! キウイ
//! ```
//!
//! そこで復元表は、辞書が一貫して置き換えている仮名だけに限る。`デュ` は表に
//! 無いので `ホンジュラス` は対象にならない。さらに表層形と発音を先頭から
//! 突き合わせ、全体が対応したときだけ書き換える。発音の長音には、表層形を
//! [`read_to_pron`] で長音表記にした形も照合する。どちらの形でも対応しなければ
//! 書き換えない。(`エヌ・エイチ・ヴィ・…` のように区切り記号が落ちている語など)

use crate::NjdFeature;
use crate::utils::{count_mora, is_katakana_word, read_to_pron};

/// 辞書が一貫して置き換えている仮名と、置き換え後の形の対。
///
/// 音韻論的な単位ではなく仮名の並びとして持つ。左は 1 つずつが 1 モーラだが、
/// 右は `イェ -> イエ` `シィ -> シー` のように 2 モーラになるものがあり、
/// モーラでも音節でも対の両側を揃えて数えられない。
///
/// 長いものから順に並べる (`ヴャ` を `ヴ` より先に見る必要がある)。
const RESTORE: &[(&str, &str)] = &[
    ("ヴャ", "ビャ"),
    ("ヴュ", "ビュ"),
    ("ヴョ", "ビョ"),
    ("ヴァ", "バ"),
    ("ヴィ", "ビ"),
    ("ヴェ", "ベ"),
    ("ヴォ", "ボ"),
    ("ヴ", "ブ"),
    ("スィ", "シ"),
    ("ズィ", "ジ"),
    ("テュ", "チュ"),
    ("デュ", "ヂュ"),
    ("イェ", "イエ"),
    ("シィ", "シー"),
    ("リェ", "リエ"),
    ("ニェ", "ニエ"),
    ("ヒェ", "ヒエ"),
    ("ミェ", "ミエ"),
    ("ビェ", "ビエ"),
    ("ピェ", "ピエ"),
    ("キェ", "ケ"),
    ("ギェ", "ゲ"),
    ("グゥ", "グウ"),
    ("クゥ", "クウ"),
];

struct RestoredSegment {
    start: i32,
    old_mora: i32,
    new_mora: i32,
}

/// 表層形と発音を先頭から突き合わせ、復元した発音とモーラ数の変わった区間を返す。
///
/// 全体が対応しなければ `None`。
///
/// pron には NAIST-jdic の無声化記号 `’` が入りうる (`ビク’トリーヌ`)。
/// 表層形には無いので、突き合わせでは読み飛ばしてそのまま持ち越す。
#[inline]
fn restore(surface: &str, pron: &str) -> Option<(String, Vec<RestoredSegment>)> {
    /// NAIST-jdic の無声化記号
    const DEVOICED: char = '\u{2019}';

    let mut out = String::with_capacity(surface.len());
    let (mut s, mut p) = (surface, pron);
    let mut changed = false;
    let mut segments = Vec::new();

    while !s.is_empty() {
        if p.starts_with(DEVOICED) {
            out.push(DEVOICED);
            p = &p[DEVOICED.len_utf8()..];
            continue;
        }
        if let Some((rare, collapsed)) = RESTORE
            .iter()
            .find(|(rare, collapsed)| s.starts_with(*rare) && p.starts_with(*collapsed))
        {
            let old_mora = count_mora(collapsed) as i32;
            let new_mora = count_mora(rare) as i32;
            if old_mora != new_mora {
                let start = count_mora(&pron[..pron.len() - p.len()]) as i32;
                segments.push(RestoredSegment {
                    start,
                    old_mora,
                    new_mora,
                });
            }
            out.push_str(rare);
            s = &s[rare.len()..];
            p = &p[collapsed.len()..];
            changed = true;
            continue;
        }
        let c = s.chars().next()?;
        if !p.starts_with(c) {
            return None;
        }
        out.push(c);
        s = &s[c.len_utf8()..];
        p = &p[c.len_utf8()..];
    }

    // 末尾に残った無声化記号も持ち越す
    if p.starts_with(DEVOICED) {
        out.push(DEVOICED);
        p = &p[DEVOICED.len_utf8()..];
    }
    (p.is_empty() && changed).then_some((out, segments))
}

/// 辞書が別の仮名に置き換えている外来語表記の仮名を、表層形から復元する。
pub(crate) fn restore_loanword_kana(njd_features: &mut [NjdFeature]) {
    for i in 0..njd_features.len() {
        let feature = &mut njd_features[i];
        if !is_katakana_word(&feature.string) {
            continue;
        }
        // read (正書法) と pron (発音) は別の欄なので別々に見る
        if let Some((read, _)) = restore(&feature.string, &feature.read) {
            feature.read = read;
        }
        // 「ウェイヴ／ウェーブ」は長音だけが表層形と対応しない。
        // 正書法を発音表記へ変えてからも照合し、長音を保った「ウェーヴ」に戻す。
        let restored = restore(&feature.string, &feature.pron)
            .or_else(|| restore(&read_to_pron(&feature.string), &feature.pron));
        if let Some((pron, segments)) = restored {
            // イェ -> イエ のようにモーラ数が変わる対があるので数え直す
            feature.mora_size = count_mora(&pron) as i32;
            feature.pron = pron;

            // 結合後の核は句先頭にある。単語の全長の差だけでは、復元区間より前の
            // 核まで動くため、短くなった区間ごとに元のモーラ位置を対応させる。
            let mut head = i;
            while head > 0 && njd_features[head].chain_flag == 1 {
                head -= 1;
            }
            let before: i32 = njd_features[head..i].iter().map(|f| f.mora_size).sum();
            let acc = njd_features[head].acc;
            let shift: i32 = segments
                .iter()
                .map(|segment| {
                    let offset = acc - before - segment.start;
                    if offset > segment.old_mora {
                        segment.old_mora - segment.new_mora
                    } else if offset > 0 {
                        offset - offset.min(segment.new_mora)
                    } else {
                        0
                    }
                })
                .sum();
            njd_features[head].acc -= shift;
        }
    }
}
