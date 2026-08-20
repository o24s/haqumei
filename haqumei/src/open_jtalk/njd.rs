use std::{
    ffi::{CStr, c_char},
    mem::MaybeUninit,
    ptr::NonNull,
};

use rustc_hash::FxHashMap;

use crate::utils::{is_katakana_word, split_kana_mora};

use crate::{
    errors::HaqumeiError,
    features::NjdFeature,
    ffi,
    utils::{Dan, dan},
};

#[derive(Debug)]
pub(crate) struct Njd {
    pub(crate) inner: NonNull<ffi::NJD>,
}

impl Njd {
    pub(crate) fn new() -> Result<Self, HaqumeiError> {
        unsafe {
            let mut njd_uninit = Box::new(MaybeUninit::<ffi::NJD>::uninit());

            ffi::NJD_initialize(njd_uninit.as_mut_ptr());

            let njd_init = njd_uninit.assume_init();

            let raw_ptr = Box::into_raw(njd_init);

            match NonNull::new(raw_ptr) {
                Some(inner) => Ok(Self { inner }),
                None => {
                    let _ = Box::from_raw(raw_ptr);
                    Err(HaqumeiError::AllocationError("Njd"))
                }
            }
        }
    }
}

impl Drop for Njd {
    fn drop(&mut self) {
        unsafe {
            ffi::NJD_clear(self.inner.as_ptr());

            let _ = Box::from_raw(self.inner.as_ptr());
        }
    }
}

fn cstr_to_string(ptr: *const c_char) -> String {
    if ptr.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned()
    }
}

pub(crate) fn njd_to_features(njd: &Njd) -> Vec<NjdFeature> {
    let mut features = Vec::new();
    let mut current_node = unsafe { (*njd.inner.as_ptr()).head };

    while !current_node.is_null() {
        let node_ref = unsafe { &*current_node };
        unsafe {
            features.push(NjdFeature {
                string: cstr_to_string(ffi::NJDNode_get_string(current_node)),
                pos: cstr_to_string(ffi::NJDNode_get_pos(current_node)),
                pos_group1: cstr_to_string(ffi::NJDNode_get_pos_group1(current_node)),
                pos_group2: cstr_to_string(ffi::NJDNode_get_pos_group2(current_node)),
                pos_group3: cstr_to_string(ffi::NJDNode_get_pos_group3(current_node)),
                ctype: cstr_to_string(ffi::NJDNode_get_ctype(current_node)),
                cform: cstr_to_string(ffi::NJDNode_get_cform(current_node)),
                orig: cstr_to_string(ffi::NJDNode_get_orig(current_node)),
                read: cstr_to_string(ffi::NJDNode_get_read(current_node)),
                pron: cstr_to_string(ffi::NJDNode_get_pron(current_node)),
                acc: ffi::NJDNode_get_acc(current_node),
                mora_size: ffi::NJDNode_get_mora_size(current_node),
                chain_rule: cstr_to_string(ffi::NJDNode_get_chain_rule(current_node)),
                chain_flag: ffi::NJDNode_get_chain_flag(current_node),
            });
        }
        current_node = node_ref.next;
    }
    features
}

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
/// 品詞を戻しただけでは核が 0 (平板) のままなので、外来語のアクセント規則
/// 「後ろから 3 モーラ目」を当てる。特殊拍 (長音・撥音・促音・小書き) には
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
