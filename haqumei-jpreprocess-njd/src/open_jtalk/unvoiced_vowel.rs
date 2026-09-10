//! Set unvoiced flag.
//!
//! ## Rules
//!
//! 0. フィラーは無声化しない
//! 1. 助動詞の「です」と「ます」の「す」が無声化
//! 2. 動詞，助動詞，助詞の「し」は無声化しやすい
//! 3. 続けて無声化しない
//! 4. アクセント核で無声化しない
//! 5. 無声子音(`k ky s sh t ty ch ts h f hy p py`)に囲まれた「`i`」と「`u`」が無声化
//!    - 例外：`s->s`, `s->sh`, `f->f`, `f->h`, `f->hy`, `h->f`, `h->h`, `h->hy`
//! 6. 破擦音の後、摩擦音の前にある「`i`」と「`u`」は有声のままにする
//!
//! ## 参照文献
//!
//! - Maekawa, K. & Kikuchi, H. (2005). Corpus-based analysis of vowel
//!   devoicing in spontaneous Japanese: an interim report. *Voicing in
//!   Japanese*, 205-228. 規則 6 は Tables 6-7 に基づく。

use haqumei_jpreprocess_core::pronunciation::{
    Mora, MoraEnum,
    phoneme::{Consonant, Vowel},
};

use crate::NJD;
use haqumei_jpreprocess_core::pos::*;

use haqumei_jpreprocess_window::{IterQuintMut, QuadForward};

#[derive(Debug)]
struct MoraState<'a> {
    pub mora: &'a mut Mora,
    pub node_index: usize,
    pub pos: POS,
    pub is_voiced_flag: Option<bool>,
    pub midx: usize,
    pub atype: usize,
}

pub fn njd_set_unvoiced_vowel(njd: &mut NJD) {
    let mut states: Vec<MoraState> = Vec::new();

    let mut midx = 0;
    let mut acc = 0;
    for (node_index, node) in njd.nodes.iter_mut().enumerate() {
        // If not chained, reset mora index for new word.
        // Otherwise, use the same accent position.
        if matches!(node.get_chain_flag(), None | Some(false)) {
            midx = 0;
            acc = node.get_pron().accent();
        }

        let pos = node.get_pos().to_owned();
        let pron = node.get_pron_mut();

        for mora in pron.moras_mut() {
            states.push(MoraState {
                is_voiced_flag: if mora.is_voiced { None } else { Some(false) },
                mora,
                node_index,
                pos,
                midx,
                atype: acc,
            });
            midx += 1;
        }
    }

    let mut iter = IterQuintMut::new(&mut states);
    while let Some(quint) = iter.next() {
        let (state_curr, mut state_next, mut state_nextnext) = match QuadForward::from(quint) {
            QuadForward::Single(curr) => (curr, None, None),
            QuadForward::Double(curr, next) => (curr, Some(next), None),
            QuadForward::Triple(curr, next, nextnext) => (curr, Some(next), Some(nextnext)),
            QuadForward::Full(curr, next, nextnext, _) => (curr, Some(next), Some(nextnext)),
        };

        /* rule 1: look-ahead for 'masu' and 'desu' */
        if let (Some(state_next), Some(state_nextnext)) =
            (state_next.as_mut(), state_nextnext.as_mut())
        {
            let index_ok = state_curr.node_index == state_next.node_index
                && state_next.node_index != state_nextnext.node_index;
            let pos_ok = matches!(
                state_next.pos,
                POS::Doushi(_) | POS::Jodoushi | POS::Kandoushi
            );
            let mora_ok = matches!(
                (state_curr.mora.mora_enum, state_next.mora.mora_enum),
                (MoraEnum::Ma | MoraEnum::De, MoraEnum::Su)
            );
            if index_ok && pos_ok && mora_ok {
                state_next.is_voiced_flag = Some(matches!(
                    state_nextnext.mora.mora_enum,
                    MoraEnum::Question | MoraEnum::Long
                ));
            }
        }

        /* rule 2: look-ahead for shi */
        if let Some(state_next) = state_next.as_mut() {
            let is_voiced_ok = matches!(state_curr.is_voiced_flag, None | Some(true))
                && state_next.is_voiced_flag.is_none()
                && matches!(
                    state_nextnext.as_ref().and_then(|nn| nn.is_voiced_flag),
                    None | Some(true)
                );
            let pos_ok = matches!(
                state_next.pos,
                POS::Doushi(_) | POS::Jodoushi | POS::Joshi(_)
            );
            let mora_ok = matches!(state_next.mora.mora_enum, MoraEnum::Shi)
                && state_curr.node_index != state_next.node_index
                && state_nextnext
                    .as_ref()
                    .is_none_or(|nn| nn.node_index != state_next.node_index);
            if is_voiced_ok && pos_ok && mora_ok {
                state_next.is_voiced_flag = if state_next.atype == state_next.midx + 1 {
                    /* rule 4 */
                    Some(true)
                } else {
                    /* rule 5 */
                    apply_unvoice_rule(state_next.mora, state_nextnext.as_ref().map(|n| &*n.mora))
                };
                if matches!(state_next.is_voiced_flag, Some(false)) {
                    state_curr.is_voiced_flag.get_or_insert(true);
                    state_nextnext
                        .as_mut()
                        .map(|nn| nn.is_voiced_flag.get_or_insert(true));
                }
            }
        }

        /* estimate unvoice */
        if state_curr.is_voiced_flag.is_none() {
            state_curr.is_voiced_flag = if
            /* rule 0 */
            matches!(state_curr.pos, POS::Filler)  ||
                /* rule 3 */
                matches!(
                    state_next.as_ref().and_then(|n| n.is_voiced_flag),
                    Some(false)
                ) ||
                /* rule 4 */
                state_curr.atype == state_curr.midx + 1
            {
                Some(true)
            } else {
                /* rule 5 */
                apply_unvoice_rule(state_curr.mora, state_next.as_ref().map(|n| &*n.mora))
            };
        }

        if matches!(state_curr.is_voiced_flag, Some(false)) {
            state_next
                .as_mut()
                .map(|n| n.is_voiced_flag.get_or_insert(true));
        }

        state_curr.mora.is_voiced = state_curr.is_voiced_flag.unwrap_or(true);
    }
}

fn apply_unvoice_rule(mora_curr: &Mora, mora_next: Option<&Mora>) -> Option<bool> {
    let Some(mora_next) = mora_next else {
        return Some(true);
    };

    let (curr_consonant, curr_vowel) = mora_curr.phonemes();
    let (next_consonant, _) = mora_next.phonemes();

    if !matches!(
        curr_vowel,
        Some(Vowel::I | Vowel::U | Vowel::IUnvoiced | Vowel::UUnvoiced)
    ) {
        return None;
    }

    #[inline(always)]
    fn is_affricate(consonant: Option<Consonant>) -> bool {
        matches!(consonant, Some(Consonant::Ch | Consonant::Ts))
    }

    #[inline(always)]
    fn is_fricative(consonant: Option<Consonant>) -> bool {
        matches!(
            consonant,
            Some(Consonant::F | Consonant::H | Consonant::S | Consonant::Sh)
        )
    }

    // Maekawa, K. & Kikuchi, H. (2005) によると、
    // 破擦音から摩擦音へ続く環境の無声化率は /i/ で 33.3%、/u/ で 48.1% だった。
    if is_affricate(curr_consonant) && is_fricative(next_consonant) {
        return Some(true);
    }

    Some(match (curr_consonant, next_consonant) {
        (Some(Consonant::S), Some(Consonant::S | Consonant::Sh)) => true,
        (
            Some(Consonant::F | Consonant::H),
            Some(Consonant::F | Consonant::Fy | Consonant::H | Consonant::Hy),
        ) => true,
        (
            Some(
                Consonant::K
                | Consonant::Ky
                | Consonant::S
                | Consonant::Sh
                | Consonant::T
                | Consonant::Ty
                | Consonant::Ch
                | Consonant::Ts
                | Consonant::H
                | Consonant::F
                | Consonant::Hy
                | Consonant::P
                | Consonant::Py,
            ),
            Some(
                Consonant::K
                | Consonant::Kw
                | Consonant::Fy
                | Consonant::Ky
                | Consonant::S
                | Consonant::Sh
                | Consonant::T
                | Consonant::Ty
                | Consonant::Ch
                | Consonant::Ts
                | Consonant::H
                | Consonant::F
                | Consonant::Hy
                | Consonant::P
                | Consonant::Py,
            ),
        ) => false,
        _ => true,
    })
}

#[cfg(test)]
mod tests {
    use haqumei_jpreprocess_core::pronunciation::{Mora, MoraEnum};

    use crate::{
        NJD,
        unvoiced_vowel::{apply_unvoice_rule, njd_set_unvoiced_vowel},
    };

    fn mora(mora_enum: MoraEnum) -> Mora {
        Mora {
            mora_enum,
            is_voiced: true,
        }
    }

    #[test]
    fn corpus_based_manner_interactions() {
        for (current, next, expected) in [
            // 破擦音から摩擦音へ続く /i/ と /u/ は有声が最頻だった。
            (MoraEnum::Chi, MoraEnum::Hi, Some(true)),
            (MoraEnum::Tsu, MoraEnum::Fu, Some(true)),
            // 摩擦音から破擦音・破裂音へ続く環境は 95% 以上が無声だった。
            (MoraEnum::Shi, MoraEnum::Chi, Some(false)),
            (MoraEnum::Fu, MoraEnum::Tsu, Some(false)),
            (MoraEnum::Hi, MoraEnum::Ki, Some(false)),
            (MoraEnum::Fu, MoraEnum::Ku, Some(false)),
        ] {
            assert_eq!(
                apply_unvoice_rule(&mora(current), Some(&mora(next))),
                expected
            );
        }
    }

    #[test]
    fn exclamation_keeps_the_desu_unvoicing_rule() {
        let mut njd: NJD = [
            "です,助動詞,*,*,*,特殊・デス,基本形,です,デス,デス,1/2,*,-1",
            "！,記号,一般,*,*,*,*,！,！,！,0/0,*,-1",
        ]
        .into_iter()
        .collect();
        njd_set_unvoiced_vowel(&mut njd);
        assert_eq!(njd.nodes[0].get_pron().to_string(), "デス’");
        assert!(njd.nodes[1].get_pron().is_exclamation());
        assert_eq!(njd.nodes[1].get_pron().mora_size(), 0);
    }

    #[test]
    fn gold() {
        let mut njd: NJD = [
            "少,接頭詞,名詞接続,*,*,*,*,少,ショウ,ショー,3/2,P2,-1",
            "し金,名詞,一般,*,*,*,*,し金,シキン,シキン,1/3,C1,1",
        ]
        .into_iter()
        .collect();

        njd_set_unvoiced_vowel(&mut njd);

        let pron = njd.nodes[1].get_pron();
        assert_eq!(
            pron.moras()[0],
            Mora {
                mora_enum: MoraEnum::Shi,
                is_voiced: true
            }
        );
    }

    #[test]
    fn interpretation() {
        let mut njd: NJD = [
            "解釈,名詞,サ変接続,*,*,*,*,解釈,カイシャク,カイシャク,1/4,C1,-1",
            "し,動詞,自立,*,*,サ変・スル,連用形,し,シ,シ,0/1,*,-1",
            "て,助詞,接続助詞,*,*,*,*,て,テ,テ,0/1,動詞%F1/形容詞%F1/名詞%F5,-1",
        ]
        .into_iter()
        .collect();

        njd_set_unvoiced_vowel(&mut njd);

        let pron = njd.nodes[1].get_pron();
        assert_eq!(
            pron.moras()[0],
            Mora {
                mora_enum: MoraEnum::Shi,
                is_voiced: false
            }
        );

        let mut njd: NJD = [
            "解釈,名詞,サ変接続,*,*,*,*,解釈,カイシャク,カイシャク,1/4,C1,-1",
            "してやれ,動詞,自立,*,*,五段・ラ行,仮定形,してやれ,シテヤレ,シテヤレ,3/4,*,-1",
            "ば,助詞,接続助詞,*,*,*,*,ば,バ,バ,0/1,動詞%F2/形容詞%F1,-1",
        ]
        .into_iter()
        .collect();

        njd_set_unvoiced_vowel(&mut njd);

        let pron = njd.nodes[1].get_pron();
        assert_eq!(
            pron.moras()[0],
            Mora {
                mora_enum: MoraEnum::Shi,
                is_voiced: true
            }
        );
    }
}
