//! 未知の異体字を含む語を、CJKVI の対応表で既知語として再解析する。

use std::ops::Range;

use haqumei_jpreprocess_dictionary::mecab::{Analysis, Node, Worker};

static VARIANTS: phf::Map<char, char> = include!("../../data/cjkvi/variants.rs");

pub(super) fn resolve(
    worker: &mut Worker,
    text: &str,
    original: Analysis,
    lattice: bool,
) -> std::io::Result<Analysis> {
    let mut replacements: Vec<_> = original
        .best_path
        .iter()
        .map(|&i| &original.nodes[i])
        .filter(|node| node.is_unknown)
        .flat_map(|node| {
            text[node.byte_span.clone()]
                .char_indices()
                .filter(|&(_, ch)| crate::utils::is_kanji(ch))
                .filter_map(move |(offset, ch)| {
                    VARIANTS
                        .get(&ch)
                        .map(|&proper| (node.byte_span.start + offset, proper))
                })
        })
        .collect();
    if replacements.is_empty() {
        return Ok(original);
    }

    // 「𠮷野家と齒性」では、単漢字にしかならない齒の置換だけを取り消す。
    // 再試行は 1 回までとし、文の長さに比例して解析回数が増えないようにする。
    for _ in 0..2 {
        let candidate = reanalyze(worker, text, &replacements, lattice)?;
        let recovered: Vec<_> = candidate
            .best_path
            .iter()
            .map(|&i| &candidate.nodes[i])
            .filter(|node| {
                let first = replacements.partition_point(|&(at, _)| at < node.byte_span.start);
                replacements
                    .get(first)
                    .is_some_and(|&(at, _)| at < node.byte_span.end)
                    && is_recovered_word(text, &original, node)
            })
            .map(|node| node.byte_span.clone())
            .collect();
        let retained: Vec<_> = replacements
            .iter()
            .copied()
            .filter(|&(at, _)| overlaps(&recovered, &(at..at + 1)))
            .collect();
        if retained.len() == replacements.len() {
            return Ok(
                if preserves_other_words(&original, &candidate, &recovered) {
                    candidate
                } else {
                    original
                },
            );
        }
        if retained.is_empty() {
            break;
        }
        replacements = retained;
    }
    Ok(original)
}

fn reanalyze(
    worker: &mut Worker,
    text: &str,
    replacements: &[(usize, char)],
    lattice: bool,
) -> std::io::Result<Analysis> {
    let mut normalized = String::with_capacity(text.len());
    let mut offsets = Vec::with_capacity(text.len() + 1);
    let mut changes = replacements.iter().peekable();
    for (offset, ch) in text.char_indices() {
        let ch = if let Some(&&(at, proper)) = changes.peek()
            && offset == at
        {
            changes.next();
            proper
        } else {
            ch
        };
        normalized.push(ch);
        offsets.resize(normalized.len(), offset);
    }
    offsets.push(text.len());

    let mut candidate = if lattice {
        worker.analyze_lattice(&normalized)?
    } else {
        worker.analyze(&normalized)?
    };
    for node in &mut candidate.nodes {
        let before = node.byte_span.clone();
        node.byte_span = offsets[before.start]..offsets[before.end];
        let surface = &text[node.byte_span.clone()];
        restore_compound_surface(node, &normalized[before], surface);
    }
    Ok(candidate)
}

fn same_entry(a: &Node, b: &Node) -> bool {
    a.byte_span == b.byte_span
        && a.feature == b.feature
        && a.left_id == b.left_id
        && a.right_id == b.right_id
        && a.pos_id == b.pos_id
        && a.word_cost == b.word_cost
        && a.dictionary_index == b.dictionary_index
        && a.is_unknown == b.is_unknown
}

fn overlaps(ranges: &[Range<usize>], span: &Range<usize>) -> bool {
    let index = ranges.partition_point(|range| range.end <= span.start);
    ranges
        .get(index)
        .is_some_and(|range| range.start < span.end)
}

fn containing(analysis: &Analysis, span: &Range<usize>) -> Option<usize> {
    let index = analysis
        .best_path
        .partition_point(|&i| analysis.nodes[i].byte_span.end <= span.start);
    analysis.best_path.get(index).copied().filter(|&i| {
        let range = &analysis.nodes[i].byte_span;
        range.start <= span.start && span.end <= range.end
    })
}

fn is_recovered_word(text: &str, original: &Analysis, node: &Node) -> bool {
    // 単漢字の辞書読みへ置き換えると「齒性」のシが「歯」のハになる。
    // 2 字以上の既知語として解析できた場合に限り、語の読みを使う。
    if node.is_unknown
        || node.dictionary_index != 0
        || text[node.byte_span.clone()].chars().count() < 2
    {
        return false;
    }
    let start = original
        .best_path
        .partition_point(|&i| original.nodes[i].byte_span.end <= node.byte_span.start);
    for &i in &original.best_path[start..] {
        let previous = &original.nodes[i];
        if previous.byte_span.start >= node.byte_span.end {
            break;
        }
        // 「高 + 﨑駅」から「高﨑 + 駅」へ、未知語は区切り直せる。
        // 既知語の途中で分ける経路と、ユーザー登録語を吸収する経路は採らない。
        if !previous.is_unknown
            && (previous.dictionary_index != 0
                || previous.byte_span.start < node.byte_span.start
                || previous.byte_span.end > node.byte_span.end)
        {
            return false;
        }
    }
    // コロンで分割されるエントリでは、NJD が原形欄を表層形として使う。
    // 元の文字列を復元できなければ、文字と音素の対応が崩れる。
    if let Some(orig) = node.feature.split(',').nth(6)
        && orig.contains(':')
        && orig.replace(':', "") != text[node.byte_span.clone()]
    {
        return false;
    }
    true
}

fn preserves_other_words(
    original: &Analysis,
    candidate: &Analysis,
    recovered: &[Range<usize>],
) -> bool {
    for &i in &original.best_path {
        let old = &original.nodes[i];
        if old.is_unknown && overlaps(recovered, &old.byte_span) {
            continue;
        }
        let Some(j) = containing(candidate, &old.byte_span) else {
            return false;
        };
        let new = &candidate.nodes[j];
        if !same_entry(old, new)
            && !(old.dictionary_index == 0
                && recovered
                    .binary_search_by_key(&new.byte_span.start, |span| span.start)
                    .is_ok())
        {
            return false;
        }
    }
    for &i in &candidate.best_path {
        let new = &candidate.nodes[i];
        if recovered
            .binary_search_by_key(&new.byte_span.start, |span| span.start)
            .is_ok()
        {
            continue;
        }
        let Some(j) = containing(original, &new.byte_span) else {
            return false;
        };
        let old = &original.nodes[j];
        if !same_entry(old, new) && !(old.is_unknown && overlaps(recovered, &old.byte_span)) {
            return false;
        }
    }
    true
}

fn restore_compound_surface(node: &mut Node, normalized: &str, original: &str) {
    if normalized == original {
        return;
    }
    let mut fields: Vec<_> = node.feature.split(',').collect();
    let Some(&orig) = fields.get(6) else { return };
    if !orig.contains(':') || orig.replace(':', "") != normalized {
        return;
    }
    let mut chars = original.chars();
    let restored = orig
        .split(':')
        .map(|part| {
            chars
                .by_ref()
                .take(part.chars().count())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join(":");
    fields[6] = &restored;
    node.feature = fields.join(",");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "embed-dictionary")]
    fn preserves_registered_variants_and_adjacent_user_words() {
        use crate::open_jtalk::{Dictionary, MecabDictIndexCompiler};
        use crate::{Haqumei, HaqumeiOptions};

        let dictionary = Dictionary::from_embedded().unwrap();
        let directory = tempfile::tempdir().unwrap();
        let csv = directory.path().join("user.csv");
        let user = directory.path().join("user.dic");
        for line in [
            "𠮷野家,1345,1345,-30000,名詞,一般,*,*,*,*,𠮷野家,チカノヤ,チカノヤ,0/4,C1\n",
            "野家,1345,1345,-30000,名詞,一般,*,*,*,*,野家,ノイエ,ノイエ,0/3,C1\n",
            "吉野家,1345,1345,-30000,名詞,一般,*,*,*,*,吉野家,キチノヤ,キチノヤ,0/4,C1\n",
        ] {
            std::fs::write(&csv, line).unwrap();
            MecabDictIndexCompiler::new()
                .dict_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("dictionary"))
                .add_input_file(&csv)
                .userdict_out_path(&user)
                .run()
                .unwrap();
            let mut engine = Haqumei::from_path_with_userdict(
                &dictionary.dict_dir,
                &user,
                HaqumeiOptions::default(),
            )
            .unwrap();
            engine.options.resolve_kanji_variants = false;
            let original = engine.run_frontend_detailed("𠮷野家").unwrap();
            let lattice = engine.analyze_lattice("𠮷野家").unwrap();
            engine.options.resolve_kanji_variants = true;
            assert_eq!(
                original,
                engine.run_frontend_detailed("𠮷野家").unwrap(),
                "{line}"
            );
            assert_eq!(lattice, engine.analyze_lattice("𠮷野家").unwrap(), "{line}");
        }
    }

    #[test]
    fn restores_the_surface_of_colon_separated_entries() {
        let mut node = Node {
            byte_span: 0..10,
            feature: "名詞,一般,*,*,*,*,吉野:家,ヨシノ:ヤ,ヨシノ:ヤ,0/3:1/1,C1".into(),
            left_id: 0,
            right_id: 0,
            pos_id: 0,
            word_cost: 0,
            dictionary_index: 0,
            is_unknown: false,
            cost: 0,
            delta: 0,
        };
        restore_compound_surface(&mut node, "吉野家", "𠮷野家");
        assert_eq!(node.feature.split(',').nth(6), Some("𠮷野:家"));
    }

    #[test]
    fn mapping_targets_are_not_rewritten_and_are_kanji() {
        for (&variant, &proper) in VARIANTS.entries() {
            assert!(crate::utils::is_kanji(variant));
            assert!(crate::utils::is_kanji(proper));
            assert!(!VARIANTS.contains_key(&proper), "{variant} -> {proper}");
        }
        assert_eq!(VARIANTS.get(&'𠮷'), Some(&'吉'));
        assert_eq!(VARIANTS.get(&'﨑'), Some(&'崎'));
        assert_eq!(VARIANTS.get(&'齒'), Some(&'歯'));
        assert!(!VARIANTS.contains_key(&'土'));
        assert!(!VARIANTS.contains_key(&'大'));
        assert!(!VARIANTS.contains_key(&'崗'));
        assert!(!VARIANTS.contains_key(&'巛'));
    }

    #[test]
    fn rejects_changes_to_words_outside_the_recovered_span() {
        let node = |span, feature: &str, unknown| Node {
            byte_span: span,
            feature: feature.into(),
            left_id: 0,
            right_id: 0,
            pos_id: 0,
            word_cost: 0,
            dictionary_index: if unknown { 255 } else { 0 },
            is_unknown: unknown,
            cost: 0,
            delta: 0,
        };
        let original = Analysis {
            nodes: vec![
                node(0..4, "*", true),
                node(4..10, "ノヤ", false),
                node(10..16, "キョー", false),
            ],
            best_path: vec![0, 1, 2],
            total_cost: 0,
        };
        let mut candidate = Analysis {
            nodes: vec![
                node(0..10, "ヨシノヤ", false),
                node(10..16, "キョー", false),
            ],
            best_path: vec![0, 1],
            total_cost: 0,
        };
        let recovered = [Range { start: 0, end: 10 }];
        assert!(preserves_other_words(&original, &candidate, &recovered));
        candidate.nodes[1].feature = "コンニチ".into();
        assert!(!preserves_other_words(&original, &candidate, &recovered));
        candidate.nodes[0].byte_span.end = 7;
        assert!(!is_recovered_word(
            "𠮷野家今日",
            &original,
            &candidate.nodes[0]
        ));
    }
}
