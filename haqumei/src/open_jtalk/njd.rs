use rustc_hash::FxHashMap;

use crate::utils::{is_katakana_word, split_kana_mora};
use crate::{errors::HaqumeiError, features::NjdFeature};

mod counter;
mod number;
pub(super) use number::{is_numeric_identifier_surface, is_numeric_surface};

/// pyopenjtalk-plus の独自結合ルールなどを適用する
pub(crate) fn apply_plus_rules(features: &mut [haqumei_jpreprocess_njd::NJDNode]) {
    use haqumei_jpreprocess_core::{
        accent_rule::ChainRules,
        cform::CForm,
        ctype::{CType, SaIrregular},
    };
    if features.len() < 2 {
        return;
    }

    for i in 0..features.len() - 1 {
        let (head, tail) = features.split_at_mut(i + 1);

        let njd = &mut head[i];
        let next_njd = &mut tail[0];

        let (is_sahen_prefix, is_verb, is_adjective) = plus_pos_flags(njd.get_details());
        let (_, next_is_verb, _) = plus_pos_flags(next_njd.get_details());

        // サ変動詞(スル)の前にサ変接続や名詞が来た場合は、一つのアクセント句に纏める
        if is_sahen_prefix && matches!(next_njd.get_ctype(), CType::SaIrregular(SaIrregular::Alone))
        {
            next_njd.set_chain_flag(true);
        }

        // ご遠慮、ご配慮のような接頭語がつく場合に、その後に続く単語の結合則を変更する
        let is_honorific_prefix = matches!(njd.get_string(), "お" | "御" | "ご");
        if is_honorific_prefix && njd.get_chain_rule().to_original_string() == "P1" {
            if next_njd.get_pron().accent() == 0
                || next_njd.get_pron().accent() == next_njd.get_pron().mora_size()
            {
                next_njd.get_details_mut().chain_rule = ChainRules::new("C4");
                next_njd.get_pron_mut().set_accent(0);
            } else {
                next_njd.get_details_mut().chain_rule = ChainRules::new("C1");
            }
        }

        // 動詞(自立)が連続する場合(e.g., 推し量る, 刺し貫く)、後ろの動詞のアクセント核が採用される
        if is_verb && next_is_verb {
            if next_njd.get_pron().accent() != 0 {
                next_njd.get_details_mut().chain_rule = ChainRules::new("C1");
            } else {
                next_njd.get_details_mut().chain_rule = ChainRules::new("C4");
            }
        }

        // 連用形のアクセント核の登録を修正する
        let is_renyoukei = matches!(
            njd.get_cform(),
            CForm::Renyou
                | CForm::RenyouConjunctionTa
                | CForm::RenyouConjunctionGozai
                | CForm::RenyouConjunctionTe
        );
        if is_renyoukei
            && njd.get_pron().accent() == njd.get_pron().mora_size()
            && njd.get_pron().mora_size() > 1
        {
            let accent = njd.get_pron().accent() - 1;
            njd.get_pron_mut().set_accent(accent);
        }

        // 「らる、られる」＋「た」の組み合わせで「た」の助動詞/F2@0を上書きしてアクセントを下げないようにする
        let is_rareru_form = matches!(
            njd.get_orig().unwrap_or("*"),
            "れる" | "られる" | "せる" | "させる" | "ちゃう"
        );
        if is_rareru_form && next_njd.get_string() == "た" {
            next_njd.get_details_mut().chain_rule = ChainRules::new("F2@1");
        }

        // 形容詞＋「なる、する」を一つのアクセント句に纏める
        if is_adjective && matches!(next_njd.get_orig().unwrap_or("*"), "なる" | "する") {
            next_njd.set_chain_flag(true);
        }
    }
}

fn plus_pos_flags(
    details: &haqumei_jpreprocess_core::word_details::WordDetails,
) -> (bool, bool, bool) {
    use haqumei_jpreprocess_core::pos::{Joshi, Meishi, POS};
    if let Some((pos, original)) = &details.pos_original
        && *pos == details.pos
    {
        let mut fields = original.split(',');
        let major = fields.next().unwrap_or("*");
        let minor = fields.next().unwrap_or("*");
        return (
            matches!(minor, "サ変接続" | "格助詞" | "接続助詞")
                || (major == "名詞" && minor == "一般")
                || major == "副詞",
            major == "動詞",
            major == "形容詞",
        );
    }
    (
        matches!(
            details.pos,
            POS::Meishi(Meishi::SahenSetsuzoku | Meishi::General)
                | POS::Joshi(Joshi::KakuJoshi(_) | Joshi::SetsuzokuJoshi)
                | POS::Fukushi(_)
        ),
        matches!(details.pos, POS::Doushi(_)),
        matches!(details.pos, POS::Keiyoushi(_)),
    )
}

fn modify_lake_chain_rules(nodes: &mut [haqumei_jpreprocess_njd::NJDNode]) {
    use haqumei_jpreprocess_core::{accent_rule::ChainRules, pos::POS};

    for i in 1..nodes.len() {
        if nodes[i].get_string() != "湖" || nodes[i].get_pron().to_pure_string() != "ミズウミ"
        {
            continue;
        }
        // 「湖」をコと読む湖名が 4 モーラ以上なら、湖の直前に核を置く。
        // ミズウミのモーラ数で結合すると、読みを短くした後も核が湖の中に残る。
        let mut moras = 1;
        for node in nodes[..i].iter().rev() {
            if !matches!(node.get_pos(), POS::Meishi(_)) {
                break;
            }
            moras += node.get_pron().mora_size();
            if node.get_chain_flag() == Some(false) {
                break;
            }
        }
        if moras >= 4 {
            nodes[i].get_details_mut().chain_rule = ChainRules::new("C3");
        }
    }
}

/// 未知語が `njd_set_pronunciation` でフィラーに変更されたのを、MeCab の品詞に戻す。
///
/// Open JTalk は「読みを持たない語が仮名として読めたらフィラーにする」という
/// 処理を持つ。(`njd_set_pronunciation.c`)
/// 言い淀み (「えーと」「あのー」) を想定した規則だが、未知のカタカナ語もすべて
/// この扱いになってしまう。
///
/// ```text
/// MeCab  クルツ,名詞,固有名詞,組織,*,*,*,*   <- unk.def の品詞
/// NJD    pos=フィラー-* acc=0              <- 上書きされる
/// ```
///
/// フィラーは慣例として平板なのでアクセントが 0 になり、しかも品詞が変わるため
/// アクセント句の作られ方まで変わる。MeCab は正しい品詞を持っているので戻す。
///
/// 本物の言い淀みは辞書 (`fillers.csv`) に載っていて読みを持つため、この処理の
/// 対象にならない。ここで見るのは列数が短い未知語の feature だけである。
///
/// さらに表層形がカタカナの語に限る。英字の未知語がフィラーになることに
/// [`crate::postprocess::modify_english_words`] が依存しており、英字の語には
/// Kanalizer が別の経路で読みを与えるため、触ってはいけない。
///
/// # アクセント
///
/// 品詞を戻しただけでは核が 0 (平板) のままなので、外来語のアクセント規則に
/// 従って核を後ろから 3 モーラ目に置く。特殊拍 (長音・撥音・促音・小書き) には
/// 核が立たないので、その場合は 1 つ前へずらす。
pub(crate) fn restore_unknown_word_pos(
    features: &mut [haqumei_jpreprocess_njd::NJDNode],
    mecab_features: &[&str],
) {
    /// 既知語の feature は 12 列以上、未知語は読みを持たないので短い
    const KNOWN_FIELD_COUNT: usize = 12;

    let mut unknown: FxHashMap<&str, [&str; 4]> = FxHashMap::default();
    for feature in mecab_features {
        let mut fields = ["*"; KNOWN_FIELD_COUNT];
        let mut count = 0;
        for (slot, value) in fields.iter_mut().zip(feature.split(',')) {
            *slot = value;
            count += 1;
        }
        if !(5..KNOWN_FIELD_COUNT).contains(&count) {
            continue;
        }
        unknown.insert(
            fields[0],
            [
                fields[1],
                *fields.get(2).unwrap_or(&"*"),
                *fields.get(3).unwrap_or(&"*"),
                *fields.get(4).unwrap_or(&"*"),
            ],
        );
    }
    if unknown.is_empty() {
        return;
    }

    for feature in features.iter_mut() {
        if !matches!(
            feature.get_pos(),
            haqumei_jpreprocess_core::pos::POS::Filler
        ) {
            continue;
        }
        if !is_katakana_word(feature.get_string()) {
            continue;
        }
        let Some(pos) = unknown.get(feature.get_string()) else {
            continue;
        };
        use haqumei_jpreprocess_core::pos::POS;
        let parsed = POS::from_strs(pos[0], pos[1], pos[2], pos[3]).unwrap_or(POS::Others);
        let original = pos.join(",");
        let accent = loanword_accent(&feature.get_pron().to_string());
        let details = feature.get_details_mut();
        details.pos = parsed;
        details.pos_original = (original != parsed.to_string()).then_some((parsed, original));
        details.pron.set_accent(accent as usize);
    }
}

/// 外来語のアクセント核の位置を「後ろから 3 モーラ目」で求める。
///
/// 3 モーラ以下の語は頭高になる。核の来る位置が特殊拍のときは、そこに核が
/// 立てないので 1 つ前へずらす。
///
/// 小書き仮名は [`crate::utils::split_kana_mora`] が直前の仮名と 1 モーラに
/// まとめるので、ここには単独で現れない。
fn loanword_accent(pron: &str) -> i32 {
    let moras = split_kana_mora(pron);
    if moras.len() <= 3 {
        return 1;
    }
    let mut index = moras.len() - 3;
    while index > 0 && is_special_mora(moras[index]) {
        index -= 1;
    }
    index as i32 + 1
}

/// 核が立てない特殊拍か。
///
/// 長音・撥音・促音の 3 つ。
/// 二重母音の副音 (`アイ` の `イ`) を特殊拍に数える立場もあるが、裏付けを取れていない。
fn is_special_mora(mora: &str) -> bool {
    matches!(mora, "ー" | "ン" | "ッ")
}

/// MeCab の特徴量から、発音・数詞・アクセントを順に求める。
pub(crate) fn run_frontend(
    raw: &[&str],
    modify_numeral_reading: bool,
    protected_raw: &[bool],
    apply_unvoicing: bool,
    split_prefixes: bool,
) -> Result<Vec<NjdFeature>, HaqumeiError> {
    use haqumei_jpreprocess_core::word_entry::WordEntry;
    use haqumei_jpreprocess_njd::{
        NJD, NJDNode, accent_phrase, accent_type, digit, digit_sequence, pronunciation,
        unvoiced_vowel,
    };

    let mut nodes = Vec::with_capacity(raw.len());
    let mut has_roman = false;
    let mut has_calendar = false;
    let mut protected_nodes = Vec::new();

    for (i, feature) in raw.iter().enumerate() {
        let mut fields = ["*"; 13];
        for (field, value) in fields.iter_mut().zip(feature.split(',')) {
            *field = value;
        }

        let is_protected = protected_raw.get(i).copied().unwrap_or(false);

        if fields[1..3] == ["名詞", "数"]
            && let Ok(value) = fields[7].parse::<u16>()
            && match fields[3] {
                "暦年" => (1..=9999).contains(&value),
                "暦元年" => value == 1,
                "暦月" => (1..=12).contains(&value),
                "暦日" => (1..=31).contains(&value),
                _ => false,
            }
        {
            has_calendar = true;
            let source = u32::try_from(i + 1)
                .ok()
                .and_then(std::num::NonZeroU32::new)
                .ok_or_else(|| {
                    HaqumeiError::MecabError("Too many morphemes for calendar expansion".into())
                })?;

            number::expand_calendar(value, fields[3], source, &mut nodes);
            protected_nodes.resize(nodes.len(), false);
            continue;
        }

        if fields[1..4] == ["名詞", "数", "ローマ数字"]
            && let Ok(value @ 1..=3999) = fields[7].parse::<u16>()
        {
            has_roman = true;
            let source = u32::try_from(i + 1)
                .ok()
                .and_then(std::num::NonZeroU32::new)
                .ok_or_else(|| {
                    HaqumeiError::MecabError(
                        "Too many morphemes for Roman numeral expansion".into(),
                    )
                })?;
            number::expand_roman(value, source, &mut nodes);
            protected_nodes.resize(nodes.len(), false);
            continue;
        }
        if !is_protected
            && number::expand_unknown_digits(
                &fields,
                &mut nodes,
                raw.get(i + 1).and_then(|f| f.split(',').next()),
            )
        {
            protected_nodes.resize(nodes.len(), false);
            continue;
        }
        let entry = WordEntry::load(&fields[1..13])
            .map_err(|error| HaqumeiError::MecabError(format!("NJD: {feature}: {error}")))?;
        match entry {
            WordEntry::Single(details) => {
                let mut node = NJDNode::from_details(fields[0].to_owned(), details);
                if fields[1] == "その他" && fields[2] == "顔文字" {
                    node.silence();
                }
                nodes.push(node)
            }
            entry @ WordEntry::Multiple(_) => nodes.extend(NJDNode::load(fields[0], &entry)),
        }
        protected_nodes.resize(nodes.len(), is_protected);
    }
    number::retain_number_spaces(&mut nodes, &mut protected_nodes);
    if has_roman {
        for i in 1..nodes.len() {
            if protected_nodes[i] && nodes[i - 1].roman_source().is_some() {
                nodes[i].protect_counter_reading();
            }
        }
    }
    number::restore_numeric_zeros(&mut nodes, &protected_nodes);
    if modify_numeral_reading {
        modify_placeholder_maru(&mut nodes, &protected_nodes);
    }
    counter::restore_counter_features(&mut nodes, &protected_nodes);
    number::mark_identifiers(&mut nodes, &protected_nodes);
    let mut njd = NJD { nodes };
    pronunciation::njd_set_pronunciation(&mut njd);
    for node in &mut njd.nodes {
        // 補正中に発音を書き換えても、公開特徴量の範囲に収めたモーラ数を維持する。
        let pron = node.get_pron_mut();
        pron.set_mora_size(pron.mora_size().min(i32::MAX as usize));
        pron.set_accent(pron.accent().min(i32::MAX as usize));
    }
    restore_unknown_word_pos(&mut njd.nodes, raw);
    apply_plus_rules(&mut njd.nodes);
    modify_lake_chain_rules(&mut njd.nodes);
    digit_sequence::njd_digit_sequence(&mut njd);
    digit::njd_set_digit(&mut njd);
    number::remove_number_spaces(&mut njd.nodes);
    // 無読の顔を句頭にすると、後続語の核が発音のない要素に記録される。
    // 数詞の処理後、アクセント計算の間だけ顔を除き、元の位置に戻す。
    let mut silent = Vec::new();
    if njd.nodes.iter().any(NJDNode::is_silent) {
        njd.nodes = std::mem::take(&mut njd.nodes)
            .into_iter()
            .enumerate()
            .filter_map(|(i, node)| {
                if node.is_silent() {
                    silent.push((i, node));
                    None
                } else {
                    Some(node)
                }
            })
            .collect();
    }
    accent_phrase::njd_set_accent_phrase(&mut njd);
    if split_prefixes {
        split_prefix_accent_phrase(&mut njd.nodes);
    }
    accent_type::njd_set_accent_type(&mut njd);
    if apply_unvoicing {
        unvoiced_vowel::njd_set_unvoiced_vowel(&mut njd);
    }
    if !silent.is_empty() {
        let total = silent.len() + njd.nodes.len();
        let mut spoken = std::mem::take(&mut njd.nodes).into_iter();
        let mut silent = silent.into_iter().peekable();
        njd.nodes = (0..total)
            .map(|i| {
                if silent.peek().is_some_and(|(at, _)| *at == i) {
                    silent.next().unwrap().1
                } else {
                    spoken.next().unwrap()
                }
            })
            .collect();
    }
    let mut features = rust_njd_to_features(&njd);
    if has_roman {
        let mut previous = None;
        for (feature, node) in features.iter_mut().zip(&njd.nodes) {
            if let Some(source) = node.roman_source() {
                // 日付や人数の縮約で取り込んだ接尾辞は、元のローマ数字の後ろに残す。
                let suffix = feature.string.trim_start_matches([
                    '〇', '零', '一', '二', '三', '四', '五', '六', '七', '八', '九', '十', '百',
                    '千',
                ]);
                let surface = if previous == Some(source) {
                    ""
                } else {
                    raw[source.get() as usize - 1].split(',').next().unwrap()
                };
                feature.string = format!("{surface}{suffix}");
            }
            previous = node.roman_source();
        }
    }

    if has_calendar {
        let mut previous = None;

        for (feature, node) in features.iter_mut().zip(&njd.nodes) {
            if let Some(source) = node.calendar_source() {
                // 先頭ゼロと空白を含む表層形を1回だけ戻し、展開した数詞の音素を同じ区間へ集める。
                feature.string = if previous == Some(source) {
                    String::new()
                } else {
                    raw[source.get() as usize - 1]
                        .split(',')
                        .next()
                        .unwrap()
                        .to_owned()
                };
            }

            previous = node.calendar_source();
        }
    }

    Ok(features)
}

fn modify_placeholder_maru(nodes: &mut [haqumei_jpreprocess_njd::NJDNode], protected: &[bool]) {
    use haqumei_jpreprocess_core::{
        pos::{Meishi, POS},
        pronunciation::Pronunciation,
    };

    let mut i = 0;
    while i < nodes.len() {
        let start = i;
        if nodes[i].get_string() != "〇" {
            i += 1;
            continue;
        }
        while i < nodes.len() && nodes[i].get_string() == "〇" {
            i += 1;
        }
        // 「二〇〇〇年」「一〇〇周年」は数値として展開するため、先行する数詞が
        // ある連続した〇を伏字にしない。
        if i - start < 2
            || (nodes[start].get_pos().is_kazu() && nodes[start].get_read() == Some("ゼロ"))
            || (start > 0 && nodes[start - 1].get_details().pos == POS::Meishi(Meishi::Kazu))
        {
            continue;
        }
        for (node, &is_protected) in nodes[start..i].iter_mut().zip(&protected[start..i]) {
            if is_protected {
                continue;
            }
            // 記号のままではカナ出力に表層形が残り、句も結合されない。
            // 数詞処理より前に名詞へ変え、句の核はアクセント結合で決める。
            node.get_details_mut().pos = POS::Meishi(Meishi::General);
            node.get_details_mut().pos_original = None;
            node.set_read("マル");
            node.set_pron(Pronunciation::parse("マル", 0).unwrap());
        }
    }
}

/// 発音の解釈に失敗した場合は、JPCommon が音素化できる接頭辞を返します。
pub(crate) fn pronunciation_from_feature(
    feature: &NjdFeature,
) -> haqumei_jpreprocess_core::pronunciation::Pronunciation {
    use haqumei_jpreprocess_core::pronunciation::{MoraEnum, Pronunciation};
    // JPCommon は解釈できない発音の直前までを音素化するため、同じ接頭辞を渡す。
    let segments = Pronunciation::parse_mora_str(&feature.pron);
    let incomplete = segments.len() > 1;
    let mut moras = segments
        .into_iter()
        .next()
        .filter(|(range, _)| range.start == 0)
        .map(|(_, moras)| moras)
        .unwrap_or_default();
    if incomplete
        && let Some(end) = moras
            .iter()
            .position(|mora| mora.mora_enum == MoraEnum::Touten)
    {
        moras.truncate(end);
    }
    let mut pron = Pronunciation::new(moras, feature.acc.max(0) as usize);
    if pron.is_touten() && feature.pron != "、" {
        pron = Pronunciation::new(Vec::new(), feature.acc.max(0) as usize);
    }
    pron.set_mora_size(feature.mora_size.max(0) as usize);
    pron
}

/// 公開特徴量から、補正後の値を持つ Rust の NJD を作る。
pub(crate) fn features_to_njd(
    features: &[NjdFeature],
) -> Result<haqumei_jpreprocess_njd::NJD, HaqumeiError> {
    use haqumei_jpreprocess_core::{
        accent_rule::ChainRules, cform::CForm, ctype::CType, pos::POS, word_details::WordDetails,
    };
    use haqumei_jpreprocess_njd::{NJD, NJDNode};
    use std::str::FromStr;

    let mut nodes = Vec::with_capacity(features.len());
    for feature in features {
        let pron = pronunciation_from_feature(feature);
        fn nonempty(value: &str) -> &str {
            if value.is_empty() { "*" } else { value }
        }
        let pos = POS::from_strs(
            nonempty(&feature.pos),
            nonempty(&feature.pos_group1),
            nonempty(&feature.pos_group2),
            nonempty(&feature.pos_group3),
        )
        .unwrap_or(POS::Others);
        let original_pos = format!(
            "{},{},{},{}",
            nonempty(&feature.pos),
            nonempty(&feature.pos_group1),
            nonempty(&feature.pos_group2),
            nonempty(&feature.pos_group3)
        );
        // C の変換表にない品詞・活用は「その他」または「*」として音素化される。
        let details = WordDetails {
            pos,
            pos_original: (original_pos != pos.to_string()).then_some((pos, original_pos)),
            ctype: CType::from_str(nonempty(&feature.ctype))
                .ok()
                .filter(|ctype| ctype.to_string() == nonempty(&feature.ctype))
                .unwrap_or(CType::None),
            cform: CForm::from_str(nonempty(&feature.cform)).unwrap_or(CForm::None),
            orig: Some(feature.orig.clone()),
            read: Some(feature.read.clone()),
            pron,
            chain_rule: ChainRules::new(&feature.chain_rule),
            chain_flag: match feature.chain_flag {
                0 => Some(false),
                1 => Some(true),
                _ => None,
            },
        };
        let mut node = NJDNode::from_details(feature.string.clone(), details);
        if feature.pos == "その他" && feature.pos_group1 == "顔文字" {
            node.silence();
        }
        nodes.push(node);
    }
    Ok(NJD { nodes })
}

pub(super) fn extract_fullcontext_labels(
    features: &[NjdFeature],
) -> Result<Vec<haqumei_jlabel::Label>, HaqumeiError> {
    let njd = features_to_njd(features)?;
    Ok(haqumei_jpreprocess_jpcommon::njdnodes_to_features(
        &njd.nodes,
    ))
}

pub(super) fn extract_phonemes(
    features: &[NjdFeature],
) -> Result<Vec<crate::Phoneme>, HaqumeiError> {
    let njd = features_to_njd(features)?;
    haqumei_jpreprocess_jpcommon::njdnodes_to_phonemes_with_sources(&njd.nodes)
        .into_iter()
        .map(|phone| phone.phoneme)
        .filter(|phone| phone != "sil")
        .map(|phone| phone.parse())
        .collect()
}

fn rust_njd_to_features(njd: &haqumei_jpreprocess_njd::NJD) -> Vec<NjdFeature> {
    njd.nodes
        .iter()
        .map(|node| {
            let details = node.get_details();
            let pos = details.pos_string();
            let mut pos = pos.split(',');
            NjdFeature {
                string: node.get_string().to_owned(),
                pos: pos.next().unwrap_or("*").to_owned(),
                pos_group1: pos.next().unwrap_or("*").to_owned(),
                pos_group2: pos.next().unwrap_or("*").to_owned(),
                pos_group3: pos.next().unwrap_or("*").to_owned(),
                ctype: details.ctype.to_string(),
                cform: details.cform.to_string(),
                orig: node.get_orig().unwrap_or("*").to_owned(),
                read: node.get_read().unwrap_or("*").to_owned(),
                pron: node.get_pron().to_string(),
                acc: node.get_pron().accent().min(i32::MAX as usize) as i32,
                mora_size: node.get_pron().mora_size().min(i32::MAX as usize) as i32,
                chain_rule: node.get_chain_rule().to_original_string(),
                chain_flag: match node.get_chain_flag() {
                    Some(true) => 1,
                    Some(false) => 0,
                    None => -1,
                },
            }
        })
        .collect()
}

#[cfg(test)]
mod nul_tests {
    #[test]
    fn nul_in_surface_and_original_form_is_not_a_terminator() {
        let raw = ["語\0尾,名詞,一般,*,*,*,*,原\0形,ゴ,ゴ,1/1,*,0"];
        let features = super::run_frontend(&raw, false, &[], true, false).unwrap();
        assert_eq!(features[0].string, "語\0尾");
        assert_eq!(features[0].orig, "原\0形");
        let njd = super::features_to_njd(&features).unwrap();
        assert_eq!(super::rust_njd_to_features(&njd), features);
    }
}

#[cfg(test)]
mod typed_rule_tests {
    use super::*;
    use haqumei_jpreprocess_njd::NJD;

    #[test]
    fn typed_rules_match_feature_based_rules() {
        let fixtures = [
            "お,接頭詞,名詞接続,*,*,*,*,お,オ,オ,0/1,P1,0",
            "語,名詞,一般,*,*,*,*,語,コトバ,コトバ,3/3,C1,0",
            "語,名詞,サ変接続,*,*,*,*,語,コトバ,コトバ,0/3,*,0",
            "を,助詞,格助詞,一般,*,*,*,を,ヲ,ヲ,0/1,*,0",
            "て,助詞,接続助詞,*,*,*,*,て,テ,テ,0/1,*,0",
            "すぐ,副詞,一般,*,*,*,*,すぐ,スグ,スグ,1/2,*,0",
            "し,動詞,自立,*,*,サ変・スル,連用形,する,シ,シ,0/1,*,0",
            "食べ,動詞,自立,*,*,一段,連用形,食べる,タベ,タベ,2/2,*,0",
            "られ,動詞,接尾,*,*,一段,連用形,られる,ラレ,ラレ,0/2,*,0",
            "た,助動詞,*,*,*,特殊・タ,基本形,た,タ,タ,0/1,動詞%F2@0,0",
            "高く,形容詞,自立,*,*,形容詞・アウオ段,連用テ接続,高い,タカク,タカク,2/3,*,0",
            "なり,動詞,自立,*,*,五段・ラ行,連用形,なる,ナリ,ナリ,2/2,*,0",
            "語,名詞,特殊,助動詞語幹,*,*,*,語,ゴ,ゴ,0/1,*,0",
            "語,その他,サ変接続,*,*,*,*,語,ゴ,ゴ,0/1,*,0",
        ];
        for first in fixtures {
            for second in fixtures {
                let mut njd: NJD = [first, second].into_iter().collect();
                let mut expected = rust_njd_to_features(&njd);
                reference_plus_rules(&mut expected);
                let expected = features_to_njd(&expected).unwrap();
                apply_plus_rules(&mut njd.nodes);
                assert_eq!(
                    rust_njd_to_features(&njd),
                    rust_njd_to_features(&expected),
                    "{first} / {second}"
                );
            }
        }
    }
    fn reference_plus_rules(features: &mut [NjdFeature]) {
        if features.len() < 2 {
            return;
        }

        for i in 0..features.len() - 1 {
            let (head, tail) = features.split_at_mut(i + 1);

            let njd = &mut head[i];
            let next_njd = &mut tail[0];

            // サ変動詞(スル)の前にサ変接続や名詞が来た場合は、一つのアクセント句に纏める
            let is_sahen_prefix =
                matches!(njd.pos_group1.as_str(), "サ変接続" | "格助詞" | "接続助詞")
                    || (njd.pos == "名詞" && njd.pos_group1 == "一般")
                    || njd.pos == "副詞";
            if is_sahen_prefix && next_njd.ctype == "サ変・スル" {
                next_njd.chain_flag = 1;
            }

            // ご遠慮、ご配慮のような接頭語がつく場合に、その後に続く単語の結合則を変更する
            let is_honorific_prefix = matches!(njd.string.as_str(), "お" | "御" | "ご");
            if is_honorific_prefix && njd.chain_rule == "P1" {
                if next_njd.acc == 0 || next_njd.acc == next_njd.mora_size {
                    next_njd.chain_rule = "C4".to_string();
                    next_njd.acc = 0;
                } else {
                    next_njd.chain_rule = "C1".to_string();
                }
            }

            // 動詞(自立)が連続する場合(e.g., 推し量る, 刺し貫く)、後ろの動詞のアクセント核が採用される
            if njd.pos == "動詞" && next_njd.pos == "動詞" {
                if next_njd.acc != 0 {
                    next_njd.chain_rule = "C1".to_string();
                } else {
                    next_njd.chain_rule = "C4".to_string();
                }
            }

            // 連用形のアクセント核の登録を修正する
            let is_renyoukei = matches!(
                njd.cform.as_str(),
                "連用形" | "連用タ接続" | "連用ゴザイ接続" | "連用テ接続"
            );
            if is_renyoukei && njd.acc == njd.mora_size && njd.mora_size > 1 {
                njd.acc -= 1;
            }

            // 「らる、られる」＋「た」の組み合わせで「た」の助動詞/F2@0を上書きしてアクセントを下げないようにする
            let is_rareru_form = matches!(
                njd.orig.as_str(),
                "れる" | "られる" | "せる" | "させる" | "ちゃう"
            );
            if is_rareru_form && next_njd.string == "た" {
                next_njd.chain_rule = "F2@1".to_string();
            }

            // 形容詞＋「なる、する」を一つのアクセント句に纏める
            if njd.pos == "形容詞" && matches!(next_njd.orig.as_str(), "なる" | "する") {
                next_njd.chain_flag = 1;
            }
        }
    }
}

// 核の計算後に句を分けると、P1 指定の「本商品」では「本」の 2 モーラに核 3 が残る。
// 接頭辞と後続語を別の句として計算するため、njd_set_accent_type より前に分ける。
// 後続語と融合する「新製品」などもあるため、収集データで句の独立を確認した 4 語に限る。
fn split_prefix_accent_phrase(nodes: &mut [haqumei_jpreprocess_njd::NJDNode]) {
    use haqumei_jpreprocess_core::pos::POS;
    for i in 1..nodes.len() {
        let prev = &nodes[i - 1];
        if nodes[i].get_chain_flag() == Some(true)
            && matches!(prev.get_pos(), POS::Settoushi(_))
            && matches!(prev.get_string(), "本" | "当" | "同" | "全")
        {
            nodes[i].set_chain_flag(false);
        }
    }
}
