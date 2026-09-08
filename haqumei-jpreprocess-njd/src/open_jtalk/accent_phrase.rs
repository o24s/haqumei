use crate::{NJD, NJDNode};
use haqumei_jpreprocess_core::pos::*;

use haqumei_jpreprocess_window::*;

pub fn njd_set_accent_phrase(njd: &mut NJD) {
    if njd.nodes.is_empty() {
        return;
    }
    let mut iter = njd.iter_quint_mut();
    while let Some(quint) = iter.next() {
        let (prev, node) = match Double::from(quint) {
            Double::Full(p, c) => (p, c),
            _ => continue,
        };
        if node.get_chain_flag().is_none() {
            let chain: bool = chain_flag(prev, node);
            node.set_chain_flag(chain);
        }
    }
}

fn chain_flag(prev: &NJDNode, node: &NJDNode) -> bool {
    let prev_pos = prev.get_pos();
    let curr_pos = node.get_pos();
    match (prev_pos, curr_pos) {
        (POS::Kandoushi, _) | (_, POS::Kandoushi) => false,
        (
            POS::Meishi(Meishi::Hijiritsu(_) | Meishi::Daimeishi(_)),
            POS::Meishi(Meishi::General | Meishi::SahenSetsuzoku),
        ) => false,
        (_, POS::Meishi(Meishi::KoyuMeishi(_))) => false,
        (
            POS::Jodoushi | POS::Rentaishi | POS::Joshi(_) | POS::Keiyoushi(_) | POS::Doushi(_),
            POS::Meishi(Meishi::Hijiritsu(_)),
        ) => true,
        /* Rule 18 */
        (
            _,
            POS::Keiyoushi(Keiyoushi::Setsubi)
            | POS::Doushi(Doushi::Setsubi)
            | POS::Meishi(Meishi::Setsubi(_)),
        ) => true,
        /* Rule 15 */
        (_, POS::Settoushi(_)) => false,
        /* Rule 14 */
        (POS::Kigou(_), _) => false,
        (_, POS::Kigou(_)) => false,
        /* Rule 13 */
        (POS::Doushi(_), POS::Doushi(Doushi::Hijiritsu)) if prev.is_renyou() => true,
        (
            POS::Meishi(Meishi::SahenSetsuzoku | Meishi::KeiyoudoushiGokan) | POS::Joshi(_),
            POS::Doushi(Doushi::Hijiritsu),
        ) => true,
        /* Rule 12 */
        (
            POS::Meishi(_),
            POS::Doushi(_) | POS::Keiyoushi(_) | POS::Meishi(Meishi::KeiyoudoushiGokan),
        ) => false,
        /* Rule 11 */
        (POS::Doushi(_), POS::Keiyoushi(Keiyoushi::Hijiritsu)) if prev.is_renyou() => true,
        (POS::Keiyoushi(_), POS::Keiyoushi(Keiyoushi::Hijiritsu)) if prev.is_renyou() => true,
        (POS::Joshi(Joshi::SetsuzokuJoshi), POS::Keiyoushi(Keiyoushi::Hijiritsu))
            if matches!(prev.get_string(), "て" | "で") =>
        {
            true
        }
        /* Rule 10 */
        (
            POS::Keiyoushi(Keiyoushi::Setsubi)
            | POS::Doushi(Doushi::Setsubi)
            | POS::Meishi(Meishi::Setsubi(_)),
            POS::Meishi(_),
        ) => false,
        /* Rule 08 */
        (POS::Jodoushi | POS::Joshi(_), POS::Jodoushi | POS::Joshi(_)) => true,
        /* Rule 09 */
        (POS::Jodoushi | POS::Joshi(_), _) => false,
        /* Rule 08 */
        (_, POS::Jodoushi | POS::Joshi(_)) => true,
        /* Rule 07 */
        (POS::Meishi(Meishi::FukushiKanou), _) => false,
        (_, POS::Meishi(Meishi::FukushiKanou)) => false,
        /* Rule 06 */
        (POS::Fukushi(_) | POS::Setsuzokushi | POS::Rentaishi, _) => false,
        (_, POS::Fukushi(_) | POS::Setsuzokushi | POS::Rentaishi) => false,
        /* Rule 05 */
        (POS::Doushi(_), POS::Keiyoushi(_) | POS::Meishi(_)) => false,
        /* Rule 03 */
        (POS::Keiyoushi(_), POS::Meishi(_) | POS::Doushi(_) | POS::Keiyoushi(_)) => false,
        /* Rule 02 */
        (POS::Meishi(_), POS::Meishi(_)) => true,
        /* Rule 01 */
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(pos: &str, chain: i32) -> NJDNode {
        NJDNode::new_single(&format!("語,{pos},*,*,語,ゴ,ゴ,1/1,*,{chain}"))
    }

    #[test]
    fn fork_accent_phrase_rules() {
        for (left, right, expected) in [
            ("名詞,形容動詞語幹,*,*", "名詞,一般,*,*", true),
            ("名詞,サ変接続,*,*", "動詞,非自立,*,*", true),
            ("助詞,接続助詞,*,*", "動詞,非自立,*,*", true),
            ("助詞,格助詞,一般,*", "名詞,非自立,一般,*", true),
            ("名詞,一般,*,*", "名詞,固有名詞,一般,*", false),
            ("名詞,代名詞,一般,*", "名詞,一般,*,*", false),
            ("感動詞,*,*,*", "名詞,接尾,一般,*", false),
            ("名詞,固有名詞,人名,姓", "名詞,一般,*,*", true),
        ] {
            let mut njd = NJD {
                nodes: vec![node(left, -1), node(right, -1)],
            };
            njd_set_accent_phrase(&mut njd);
            assert_eq!(
                njd.nodes[1].get_chain_flag(),
                Some(expected),
                "{left} -> {right}"
            );
        }
    }

    #[test]
    fn explicit_chain_flag_is_preserved() {
        let mut njd = NJD {
            nodes: vec![node("感動詞,*,*,*", -1), node("名詞,一般,*,*", 1)],
        };
        njd_set_accent_phrase(&mut njd);
        assert_eq!(njd.nodes[1].get_chain_flag(), Some(true));
    }
}
