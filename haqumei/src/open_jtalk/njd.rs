use rustc_hash::FxHashMap;

use crate::utils::{is_katakana_word, split_kana_mora};
use crate::{
    errors::HaqumeiError,
    features::NjdFeature,
    utils::{Dan, dan},
};

/// pyopenjtalk-plus の独自結合ルールなどを適用する
pub(crate) fn apply_plus_rules(features: &mut [NjdFeature]) {
    if features.len() < 2 {
        return;
    }

    for i in 0..features.len() - 1 {
        let (head, tail) = features.split_at_mut(i + 1);

        let njd = &mut head[i];
        let next_njd = &mut tail[0];

        // njd_set_pronunciation は、動詞または助動詞の後に助動詞「う」が続く場合、
        // その「う」の発音を長音（ー）に置き換えてしまう。
        // 前方の単語が ア段, イ段, エ段 で終わるとき、長音の置き換えを取り消す。
        if next_njd.pron == "ー"
            && next_njd.read == "ウ"
            && let Some(last) = njd.pron.chars().last()
            && let Some(dan) = dan(last)
            && matches!(dan, Dan::ア段 | Dan::イ段 | Dan::エ段)
        {
            next_njd.pron = "ウ".to_string();
        }

        // サ変動詞(スル)の前にサ変接続や名詞が来た場合は、一つのアクセント句に纏める
        let is_sahen_prefix = matches!(njd.pos_group1.as_str(), "サ変接続" | "格助詞" | "接続助詞")
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
pub(crate) fn restore_unknown_word_pos(features: &mut [NjdFeature], mecab_features: &[&str]) {
    /// 既知語の feature は 12 列以上、未知語は読みを持たないので短い
    const KNOWN_FIELD_COUNT: usize = 12;

    let mut unknown: FxHashMap<&str, [&str; 4]> = FxHashMap::default();
    for feature in mecab_features {
        let fields: Vec<&str> = feature.split(',').collect();
        if fields.len() >= KNOWN_FIELD_COUNT || fields.len() < 5 {
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
        if feature.pos != "フィラー" {
            continue;
        }
        if !is_katakana_word(&feature.string) {
            continue;
        }
        let Some(pos) = unknown.get(feature.string.as_str()) else {
            continue;
        };
        feature.pos = pos[0].to_string();
        feature.pos_group1 = pos[1].to_string();
        feature.pos_group2 = pos[2].to_string();
        feature.pos_group3 = pos[3].to_string();
        feature.acc = loanword_accent(&feature.pron);
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
pub(crate) fn run_frontend(raw: &[String]) -> Result<Vec<NjdFeature>, HaqumeiError> {
    use haqumei_jpreprocess_core::word_entry::WordEntry;
    use haqumei_jpreprocess_njd::{
        NJD, NJDNode, accent_phrase, accent_type, digit, digit_sequence, pronunciation,
        unvoiced_vowel,
    };

    let mut nodes = Vec::new();
    for feature in raw {
        std::ffi::CString::new(feature.as_str())?;
        let mut fields: Vec<&str> = feature.split(',').collect();
        fields.resize(13, "*");
        let entry = WordEntry::load(&fields[1..13])
            .map_err(|error| HaqumeiError::MecabError(format!("NJD: {feature}: {error}")))?;
        nodes.extend(NJDNode::load(fields[0], &entry));
    }
    let mut njd = NJD { nodes };
    pronunciation::njd_set_pronunciation(&mut njd);
    let mut features = rust_njd_to_features(&njd);
    let raw_refs: Vec<&str> = raw.iter().map(String::as_str).collect();
    restore_unknown_word_pos(&mut features, &raw_refs);
    apply_plus_rules(&mut features);
    njd = features_to_njd(&features)?;
    digit_sequence::njd_digit_sequence(&mut njd);
    digit::njd_set_digit(&mut njd);
    accent_phrase::njd_set_accent_phrase(&mut njd);
    accent_type::njd_set_accent_type(&mut njd);
    unvoiced_vowel::njd_set_unvoiced_vowel(&mut njd);
    Ok(rust_njd_to_features(&njd))
}

/// 公開特徴量から、補正後の値を持つ Rust の NJD を作る。
pub(crate) fn features_to_njd(
    features: &[NjdFeature],
) -> Result<haqumei_jpreprocess_njd::NJD, HaqumeiError> {
    use haqumei_jpreprocess_core::{
        accent_rule::ChainRules,
        cform::CForm,
        ctype::CType,
        pos::POS,
        pronunciation::{MoraEnum, Pronunciation},
        word_details::WordDetails,
    };
    use haqumei_jpreprocess_njd::{NJD, NJDNode};
    use std::str::FromStr;

    let mut nodes = Vec::with_capacity(features.len());
    for feature in features {
        for value in [
            &feature.string,
            &feature.pos,
            &feature.pos_group1,
            &feature.pos_group2,
            &feature.pos_group3,
            &feature.ctype,
            &feature.cform,
            &feature.orig,
            &feature.read,
            &feature.pron,
            &feature.chain_rule,
        ] {
            std::ffi::CString::new(value.as_str())?;
        }
        let convert_error = |error: Box<dyn std::fmt::Display>| {
            HaqumeiError::MecabError(format!("NJD: {}: {error}", feature.string))
        };
        // JPCommon は解釈できない発音の直前までを音素化するため、同じ接頭辞を渡す。
        let mut pron = Pronunciation::parse(&feature.pron, feature.acc.max(0) as usize)
            .unwrap_or_else(|_| {
                let moras = Pronunciation::parse_mora_str(&feature.pron)
                    .into_iter()
                    .next()
                    .filter(|(range, _)| range.start == 0)
                    .map(|(_, moras)| {
                        moras
                            .into_iter()
                            .take_while(|mora| mora.mora_enum != MoraEnum::Touten)
                            .collect()
                    })
                    .unwrap_or_default();
                Pronunciation::new(moras, feature.acc.max(0) as usize)
            });
        if pron.is_touten() && feature.pron != "、" {
            pron = Pronunciation::new(Vec::new(), feature.acc.max(0) as usize);
        }
        pron.set_mora_size(feature.mora_size.max(0) as usize);
        fn nonempty(value: &str) -> &str {
            if value.is_empty() { "*" } else { value }
        }
        let pos = POS::from_strs(
            nonempty(&feature.pos),
            nonempty(&feature.pos_group1),
            nonempty(&feature.pos_group2),
            nonempty(&feature.pos_group3),
        )
        .map_err(|error| convert_error(Box::new(error)))?;
        let original_pos = format!(
            "{},{},{},{}",
            feature.pos, feature.pos_group1, feature.pos_group2, feature.pos_group3
        );
        let details = WordDetails {
            pos,
            pos_original: (original_pos != pos.to_string()).then_some((pos, original_pos)),
            ctype: CType::from_str(nonempty(&feature.ctype))
                .map_err(|error| convert_error(Box::new(error)))?,
            cform: CForm::from_str(nonempty(&feature.cform))
                .map_err(|error| convert_error(Box::new(error)))?,
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
        nodes.push(NJDNode::from_details(feature.string.clone(), details));
    }
    Ok(NJD { nodes })
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
