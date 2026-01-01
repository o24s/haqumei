use std::{ffi::{CStr, c_char}, mem::MaybeUninit, ptr::NonNull};

use crate::{errors::HaqumeiError, features::NjdFeature, ffi};

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
        unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned()
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


/// pyopenjtalk-plus の独自結合ルールを適用する
pub(crate) fn apply_plus_rules(features: &mut [NjdFeature]) {
    if features.len() < 2 {
        return;
    }

    for i in 0..features.len() - 1 {
        let (head, tail) = features.split_at_mut(i + 1);

        let njd = &mut head[i];
        let next_njd = &mut tail[0];

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
        let is_renyoukei = matches!(njd.cform.as_str(), "連用形" | "連用タ接続" | "連用ゴザイ接続" | "連用テ接続");
        if is_renyoukei && njd.acc == njd.mora_size && njd.mora_size > 1 {
            njd.acc -= 1;
        }

        // 「らる、られる」＋「た」の組み合わせで「た」の助動詞/F2@0を上書きしてアクセントを下げないようにする
        let is_rareru_form = matches!(njd.orig.as_str(), "れる" | "られる" | "せる" | "させる" | "ちゃう");
        if is_rareru_form && next_njd.string == "た" {
            next_njd.chain_rule = "F2@1".to_string();
        }

        // 形容詞＋「なる、する」を一つのアクセント句に纏める
        if njd.pos == "形容詞" && matches!(next_njd.orig.as_str(), "なる" | "する") {
            next_njd.chain_flag = 1;
        }
    }
}
