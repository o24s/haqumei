//! 辞書が潰した稀な音節を、表層形から復元する。
//!
//! 音素層 ([`crate::Phoneme`] と `jpcommon_rule`) は `ヴィ` `テュ` `クィ` など
//! 39 種の稀な音節をすべて表現でき、Open JTalk の読み変換表もそれらを保存する。
//! 潰しているのは辞書のエントリだけで、`unidic-csj` は 20 種を一貫して
//! ba 行などへ寄せている。
//!
//! ```text
//! ヴィクトリーヌ  pron=ビクトリーヌ  -> ヴィクトリーヌ
//! アイシュヴァルヤ pron=アイシュバルヤ -> アイシュヴァルヤ
//! ```
//!
//! # 表層形から素朴に作り直さない
//!
//! 潰れた形のほうが日本語として定着している語が多くある。
//!
//! ```text
//! ホンデュラス -> ホンジュラス, バースディ -> バースデイ
//! キウイ
//! ```
//!
//! そこで復元表は、辞書が一貫して潰している音節だけに限る。`デュ` は表に
//! 無いので `ホンジュラス` は触られない。さらに表層形と発音を先頭から
//! 突き合わせ、全体が対応したときだけ書き換える。1 文字でも食い違えば
//! その語は諦める (`エヌ・エイチ・ヴィ・…` のように区切り記号が落ちている
//! 語などが該当する)。

use crate::NjdFeature;
use crate::utils::{count_mora, is_katakana_word};

/// 辞書が一貫して潰している音節と、潰れた形の対。
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

/// 表層形と発音を先頭から突き合わせ、復元した発音を返す。
///
/// 全体が対応しなければ `None`。
///
/// pron には NAIST-jdic の無声化記号 `’` が入りうる (`ビク’トリーヌ`)。
/// 表層形には無いので、突き合わせでは読み飛ばしてそのまま持ち越す。
#[inline]
fn restore(surface: &str, pron: &str) -> Option<String> {
    /// NAIST-jdic の無声化記号
    const DEVOICED: char = '\u{2019}';

    let mut out = String::with_capacity(surface.len());
    let (mut s, mut p) = (surface, pron);
    let mut changed = false;

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
    (p.is_empty() && changed).then_some(out)
}

/// 辞書が潰した稀な音節を表層形から復元する。
pub(crate) fn restore_rare_syllables(njd_features: &mut [NjdFeature]) {
    for feature in njd_features.iter_mut() {
        if !is_katakana_word(&feature.string) {
            continue;
        }
        // read (正書法) と pron (発音) は別の場なので別々に見る
        if let Some(read) = restore(&feature.string, &feature.read) {
            feature.read = read;
        }
        if let Some(pron) = restore(&feature.string, &feature.pron) {
            // イェ -> イエ のようにモーラ数が変わる対があるので数え直す
            feature.mora_size = count_mora(&pron) as i32;
            feature.pron = pron;
        }
    }
}
