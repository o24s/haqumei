//! Process pronunciation of digits.
//!
//! - 接尾辞によって読み方を変える
//!   - 例えば「一分」を「いちふん」ではなく「いっぷん」と読む．
//! - 日付を正しく読む
//!   - 例えば「1日」は「いちにち」ではなく「ついたち」，「24日」は「にじゅうよんにち」ではなく「にじゅうよっか」．

mod accent;
mod counter;
mod lut;

use crate::NJD;

use haqumei_jpreprocess_core::{pos::*, pron};
use haqumei_jpreprocess_window::*;

use self::lut::{DigitType, find_pron_conv_set, numeral};

pub fn is_period(s: &str) -> bool {
    s == "．" || s == "・"
}

pub fn njd_set_digit(njd: &mut NJD) {
    let mut decimal_points = Vec::new();
    {
        enum SkipState {
            Disabled,
            IfMeishi,
            Skipping,
        }
        let mut skip_state = SkipState::Disabled;
        let node_count = njd.nodes.len();
        let mut iter = njd.iter_quint_mut();
        for index in 0..node_count {
            let Some(quint) = iter.next() else { break };
            let (prev, node, next) = match Triple::from(quint) {
                Triple::Full(prev, node, next) => (prev, node, next),
                _ => continue,
            };
            match (&skip_state, node.get_pos()) {
                (SkipState::IfMeishi, _) => {
                    skip_state = SkipState::Skipping;
                    continue;
                }
                (SkipState::Skipping, POS::Meishi(_)) => {
                    continue;
                }
                (SkipState::Skipping, _) => {
                    skip_state = SkipState::Disabled;
                    continue;
                }
                _ => (),
            }
            if !node.get_string().is_empty()
                && !prev.get_string().is_empty()
                && is_period(node.get_string())
                && node.digit_sequence_reading() != Some(false)
                && prev.roman_source().is_none()
                && next.roman_source().is_none()
                && prev.get_pos().is_kazu()
                && next.get_pos().is_kazu()
            {
                // 中黒は数の列挙にも使うため、小数の助数詞補正には含めない。
                if node.get_string() == "．" {
                    decimal_points.push(index);
                }
                node.replace_from_csv(rule::TEN_FEATURE);
                node.set_chain_flag(true);
                match prev.get_string() {
                    rule::ZERO1 | rule::ZERO2 => {
                        prev.set_pron(pron!([Re, Long], 1));
                    }
                    rule::TWO => {
                        prev.set_pron(pron!([Ni, Long], 1));
                    }
                    rule::FIVE => {
                        prev.set_pron(pron!([Go, Long], 1));
                    }
                    rule::SIX => {
                        prev.set_pron(pron!([Ro, Ku], 1));
                    }
                    _ => (),
                }
                skip_state = SkipState::IfMeishi;
            }
        }
    }

    counter::convert_counters(njd, &decimal_points);

    {
        let mut iter = njd.iter_quint_mut();
        while let Some(quint) = iter.next() {
            let (prev, node) = match Double::from(quint) {
                Double::Full(prev, node) => (prev, node),
                _ => continue,
            };
            if !prev.get_pos().is_kazu() || node.counter_reading_protected() {
                continue;
            }
            if node.get_pos().is_kazu() && !node.get_string().is_empty() {
                if numeral::NUMERAL_LIST4.contains(prev.get_string())
                    && numeral::NUMERAL_LIST5.contains(node.get_string())
                {
                    prev.set_chain_flag(false);
                    node.set_chain_flag(true);
                } else if numeral::NUMERAL_LIST5.contains(prev.get_string())
                    && numeral::NUMERAL_LIST4.contains(node.get_string())
                {
                    node.set_chain_flag(false);
                }
            }
            if let Some(lut3_conversion) = find_pron_conv_set(
                &numeral::DIGIT_CONVERSION_TABLE,
                node.get_string(),
                prev.get_string(),
            ) {
                prev.set_pron(lut3_conversion.clone());
            }
            match find_pron_conv_set(
                &numeral::NUMERATIVE_CONVERSION_TABLE,
                node.get_string(),
                prev.get_string(),
            ) {
                Some(DigitType::Voiced) => node
                    .get_pron_mut()
                    .moras_mut()
                    .first_mut()
                    .map(|mora| mora.convert_to_voiced_sound()),
                Some(DigitType::SemiVoiced) => node
                    .get_pron_mut()
                    .moras_mut()
                    .first_mut()
                    .map(|mora| mora.convert_to_semivoiced_sound()),
                _ => None,
            };
        }
    }

    counter::convert_native(njd, &decimal_points);

    if njd.nodes.len() > 2 {
        let mut iter = njd.iter_quint_mut();
        while let Some(quint) = iter.next() {
            let (node, nx1, nx2, nx3_t) = match quint {
                Quintuple::Triple(node, nx1, nx2) => (node, nx1, nx2, None),
                Quintuple::First(node, nx1, nx2, nx3) => (node, nx1, nx2, Some(nx3)),
                Quintuple::Full(prev, node, nx1, nx2, nx3) if !prev.get_pos().is_kazu() => {
                    (node, nx1, nx2, Some(nx3))
                }
                Quintuple::ThreeLeft(prev, node, nx1, nx2) if !prev.get_pos().is_kazu() => {
                    (node, nx1, nx2, None)
                }
                _ => continue,
            };

            let mut nx3 = nx3_t;
            if node.counter_reading_protected()
                || nx1.counter_reading_protected()
                || nx2.counter_reading_protected()
                || nx3.as_ref().is_some_and(|n| n.counter_reading_protected())
            {
                continue;
            }

            enum UnsetPattern {
                None,
                Nx1Nx2,
                Nx2,
                Nx2Nx3,
            }

            let (node_s, nx1_s, unset) = match (
                node.get_string(),
                nx1.get_string(),
                nx2.get_string(),
                nx3.as_ref().map(|n| n.get_string()),
            ) {
                (rule::TEN, rule::FOUR, rule::NICHI, Some("目")) => {
                    nx3.as_mut().unwrap().get_details_mut().chain_rule =
                        haqumei_jpreprocess_core::accent_rule::ChainRules::new("F4@1");
                    (Some(rule::JUYOKKA), None, UnsetPattern::Nx1Nx2)
                }
                (rule::TEN, rule::FOUR, rule::NICHI, _) => {
                    (None, Some(rule::YOKKA), UnsetPattern::Nx2)
                }
                (rule::TEN, rule::FOUR, rule::NICHIKAN, _) => {
                    (Some(rule::JUYOKKAKAN), None, UnsetPattern::Nx1Nx2)
                }
                (rule::TWO, rule::TEN, rule::NICHI, _) => {
                    (Some(rule::HATSUKA), None, UnsetPattern::Nx1Nx2)
                }
                (rule::TWO, rule::TEN, rule::NICHIKAN, _) => {
                    (Some(rule::HATSUKAKAN), None, UnsetPattern::Nx1Nx2)
                }
                (rule::TWO, rule::TEN, rule::FOUR, Some(rule::NICHI)) => {
                    (Some(rule::NIJU), Some(rule::YOKKA), UnsetPattern::Nx2Nx3)
                }
                (rule::TWO, rule::TEN, rule::FOUR, Some(rule::NICHIKAN)) => {
                    (Some(rule::NIJU), Some(rule::YOKKAKAN), UnsetPattern::Nx2Nx3)
                }
                _ => (None, None, UnsetPattern::None),
            };
            if let Some(new_node_s) = node_s {
                node.replace_from_csv(new_node_s);
            }
            if let Some(new_node_s) = nx1_s {
                nx1.replace_from_csv(new_node_s);
            }
            match unset {
                UnsetPattern::None => (),
                UnsetPattern::Nx2 => nx2.reset(),
                UnsetPattern::Nx1Nx2 => {
                    nx1.reset();
                    nx2.reset();
                }
                UnsetPattern::Nx2Nx3 => {
                    nx2.reset();
                    nx3.as_mut().unwrap().reset();
                }
            }
        }
    }

    njd.remove_silent_node();
    accent::join_teens(njd);
}

mod rule {
    pub const TEN_FEATURE: &str = "．,名詞,接尾,助数詞,*,*,*,．,テン,テン,0/2,*,-1";
    pub const ZERO1: &str = "〇";
    pub const ZERO2: &str = "０";
    pub const TWO: &str = "二";
    pub const FIVE: &str = "五";
    pub const SIX: &str = "六";

    pub const GATSU: &str = "月";
    pub const NICHI: &str = "日";
    pub const NICHIKAN: &str = "日間";

    pub const ONE: &str = "一";
    pub const TSUITACHI: &str = "一日,名詞,副詞可能,*,*,*,*,一日,ツイタチ,ツイタチ,4/4,*";

    pub const FOUR: &str = "四";
    pub const TEN: &str = "十";
    pub const JUYOKKA: &str = "十四日,名詞,副詞可能,*,*,*,*,十四日,ジュウヨッカ,ジューヨッカ,1/5,*";
    pub const JUYOKKAKAN: &str =
        "十四日間,名詞,副詞可能,*,*,*,*,十四日間,ジュウヨッカカン,ジューヨッカカン,5/7,*";
    pub const NIJU: &str = "二十,名詞,副詞可能,*,*,*,*,二十,ニジュウ,ニジュー,1/3,*";
    pub const YOKKA: &str = "四日,名詞,副詞可能,*,*,*,*,四日,ヨッカ,ヨッカ,0/3,*,0";
    pub const YOKKAKAN: &str = "四日間,名詞,副詞可能,*,*,*,*,四日間,ヨッカカン,ヨッカカン,3/5,*,0";
    pub const HATSUKA: &str = "二十日,名詞,副詞可能,*,*,*,*,二十日,ハツカ,ハツカ,0/3,*";
    pub const HATSUKAKAN: &str =
        "二十日間,名詞,副詞可能,*,*,*,*,二十日間,ハツカカン,ハツカカン,3/5,*";
}
