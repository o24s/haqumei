//! Set pronunciation based on various clues.

use crate::NJD;

use haqumei_jpreprocess_core::{
    pos::*,
    pron,
    pronunciation::{MoraEnum, Pronunciation},
};

use haqumei_jpreprocess_window::*;

pub fn njd_set_pronunciation(njd: &mut NJD) {
    for node in &mut njd.nodes {
        if node.is_silent() || node.get_pron().mora_size() != 0 {
            continue;
        }
        let mut moras = Pronunciation::parse_mora_str(node.get_string())
            .into_iter()
            .flat_map(|(_, moras)| moras)
            .filter(|mora| !matches!(mora.mora_enum, MoraEnum::Touten | MoraEnum::Xke))
            .map(|mut mora| {
                mora.is_voiced = true;
                mora
            })
            .collect::<Vec<_>>();
        if node.get_orig().is_none() {
            let orig = node.get_string().to_owned();
            node.set_orig(&orig);
        }
        if moras.is_empty() {
            moras.push(haqumei_jpreprocess_core::pronunciation::Mora {
                mora_enum: MoraEnum::Touten,
                is_voiced: true,
            });
            *node.get_pos_mut() = POS::Kigou(Kigou::Touten);
            node.get_details_mut().ctype = haqumei_jpreprocess_core::ctype::CType::None;
            node.get_details_mut().cform = haqumei_jpreprocess_core::cform::CForm::None;
        }
        let mut pron = Pronunciation::new(moras, node.get_pron().accent());
        let z_count = node
            .get_string()
            .chars()
            .filter(|c| matches!(c, 'Ｚ' | 'ｚ'))
            .count();
        pron.set_mora_size(pron.mora_size().saturating_sub(z_count));
        if pron.mora_size() != 0 {
            *node.get_pos_mut() = POS::Filler;
        }
        node.set_read(&pron.to_pure_string());
        node.set_pron(pron);
    }

    njd.remove_silent_node();

    /* chain kana sequence */
    {
        let mut head_of_kana_filler_sequence_index: Option<usize> = None;
        for i in 0..njd.nodes.len() {
            let (head_of_kana_filler_sequence, node) = {
                let (a, b) = njd.nodes.split_at_mut(i);
                let head_of_kana_filler_sequence =
                    head_of_kana_filler_sequence_index.and_then(|i| a.get_mut(i));
                let node = b.get_mut(0).unwrap();
                (head_of_kana_filler_sequence, node)
            };
            if matches!(node.get_pos(), POS::Filler) {
                if Pronunciation::is_mora_convertable(node.get_string()) {
                    if let Some(seq) = head_of_kana_filler_sequence {
                        seq.transfer_from(node);
                    } else {
                        head_of_kana_filler_sequence_index = Some(i);
                    }
                } else {
                    head_of_kana_filler_sequence_index = None;
                }
            } else {
                head_of_kana_filler_sequence_index = None;
            }
        }
    }

    njd.remove_silent_node();

    {
        let mut iter = njd.iter_quint_mut();
        while let Some(quint) = iter.next() {
            let (node, next) = match Triple::from(quint) {
                Triple::First(node, next) => (node, next),
                Triple::Full(_, node, next) => (node, next),
                _ => continue,
            };
            let auxiliary_u = next.get_pron().mora_matches(MoraEnum::U)
                && matches!(next.get_pos(), POS::Jodoushi)
                && matches!(node.get_pos(), POS::Doushi(_) | POS::Jodoushi)
                && node.get_pron().mora_size() > 0;
            let registered_long = next.get_pron().mora_matches(MoraEnum::Long)
                && next.get_pron().moras()[0].is_voiced;
            if auxiliary_u || registered_long {
                let keep_u = next.get_read() == Some("ウ")
                    && node
                        .get_pron()
                        .moras()
                        .last()
                        .is_some_and(|mora| mora.is_voiced && ends_in_a_i_e(mora.as_str()));
                if keep_u && registered_long {
                    next.get_pron_mut().moras_mut()[0].mora_enum = MoraEnum::U;
                } else if keep_u {
                    next.set_pron(pron!([U], 0));
                } else if auxiliary_u {
                    next.set_pron(pron!([Long], 0));
                }
            }
            if matches!(node.get_pos(), POS::Jodoushi) && matches!(next.get_string(), "？" | "！")
            {
                match node.get_string() {
                    "です" => node.set_pron(pron!([De, Su], 1)),
                    "ます" => node.set_pron(pron!([Ma, Su], 1)),
                    _ => (),
                }
            }
        }
    }
}

// ア・イ・エ段の後では「う」を独立した母音として残す。
// 小書きのャ・ヵ・ヶとヰ・ヱは従来の段判定に含まれない。
#[rustfmt::skip]
fn ends_in_a_i_e(mora: &str) -> bool {
    matches!(mora.chars().last(), Some(
        'ア' | 'カ' | 'サ' | 'タ' | 'ナ' | 'ハ' | 'マ' | 'ヤ' | 'ラ' | 'ワ' | 'ガ' | 'ザ'
        | 'ダ' | 'バ' | 'パ' | 'ァ'
        | 'イ' | 'キ' | 'シ' | 'チ' | 'ニ' | 'ヒ' | 'ミ' | 'リ' | 'ギ' | 'ジ' | 'ヂ' | 'ビ'
        | 'ピ' | 'ィ'
        | 'エ' | 'ケ' | 'セ' | 'テ' | 'ネ' | 'ヘ' | 'メ' | 'レ' | 'ゲ' | 'ゼ' | 'デ' | 'ベ'
        | 'ペ' | 'ェ'
    ))
}

#[cfg(test)]
mod tests {
    use crate::{NJD, pronunciation::njd_set_pronunciation};

    #[test]
    fn auxiliary_u_preserves_separate_vowels() {
        for (previous, expected) in [
            ("ア", "ウ"),
            ("イ", "ウ"),
            ("エ", "ウ"),
            ("ウ", "ー"),
            ("オ", "ー"),
            ("カ", "ウ"),
            ("キ", "ウ"),
            ("ケ", "ウ"),
            ("キャ", "ー"),
            ("クァ", "ウ"),
            ("ティ", "ウ"),
            ("シェ", "ウ"),
            ("ヰ", "ー"),
            ("ヱ", "ー"),
            ("ン", "ー"),
            ("ー", "ー"),
            ("キ’", "ー"),
        ] {
            let feature = format!("前,動詞,自立,*,*,一段,未然形,前,{previous},{previous},0/1,*,0");
            let mut njd: NJD = [
                feature.as_str(),
                "う,助動詞,*,*,*,不変化型,基本形,う,ウ,ウ,0/1,*,0",
            ]
            .into_iter()
            .collect();
            njd_set_pronunciation(&mut njd);
            assert_eq!(njd.nodes[1].get_pron().to_string(), expected, "{previous}");
        }
    }

    #[test]
    fn ordinary_u_is_not_lengthened() {
        let mut njd: NJD = [
            "前,動詞,自立,*,*,一段,未然形,前,オ,オ,0/1,*,0",
            "う,名詞,一般,*,*,*,*,う,ウ,ウ,0/1,*,0",
        ]
        .into_iter()
        .collect();
        njd_set_pronunciation(&mut njd);
        assert_eq!(njd.nodes[1].get_pron().to_string(), "ウ");
    }

    #[test]
    fn unknown_z_uses_the_registered_mora_count() {
        let mut njd: NJD = ["Ｚ,名詞,一般,*,*,*,*,*"].into_iter().collect();
        njd_set_pronunciation(&mut njd);
        let node = &njd.nodes[0];
        assert_eq!(node.get_read(), Some("ゼット"));
        assert_eq!(node.get_pron().to_string(), "ゼット");
        assert_eq!(node.get_pron().mora_size(), 2);
        assert_eq!(node.get_pron().moras().len(), 3);
    }

    #[test]
    fn unknown_small_ke_is_a_pause() {
        let mut njd: NJD = ["ヶ,名詞,一般,*,*,*,*,*"].into_iter().collect();
        njd_set_pronunciation(&mut njd);
        assert_eq!(njd.nodes[0].get_pron().to_string(), "、");
        assert_eq!(njd.nodes[0].get_pron().mora_size(), 0);
        assert_eq!(njd.nodes[0].get_pos().to_string(), "記号,読点,*,*");
    }

    #[test]
    fn barry_payne() {
        let mut njd: NJD = [
            "バリー・ペーン,名詞,*,*,*,*,*,バリー・ペーン,*,,0/0,*,-1",
            "は,名詞,*,*,*,*,*,は,*,,0/0,*,-1",
        ]
        .into_iter()
        .collect();

        njd_set_pronunciation(&mut njd);

        assert_eq!(njd.nodes.len(), 2);
        assert_eq!(njd.nodes[0].get_string(), "バリー・ペーン");
        assert_eq!(njd.nodes[0].get_pron().to_string(), "バリーペーン");
        assert_eq!(njd.nodes[1].get_pron().mora_size(), 1);
    }
}
