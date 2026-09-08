use haqumei_jpreprocess_core::{pos::*, word_details::WordDetails};

pub fn pos_details_to_id(details: &WordDetails) -> Option<u8> {
    let Some((_, original)) = details
        .pos_original
        .as_ref()
        .filter(|(pos, _)| *pos == details.pos)
    else {
        return pos_to_id(&details.pos);
    };
    let mut fields = original
        .split(',')
        .map(|value| if value.is_empty() { "*" } else { value });
    if fields.clone().eq(details.pos.to_string().split(',')) {
        return pos_to_id(&details.pos);
    }
    // 型が省略する細分類にも登録値がある。登録されていない組合せは「その他」になる。
    match (
        fields.next(),
        fields.next(),
        fields.next(),
        fields.next(),
        fields.next(),
    ) {
        (Some("助動詞"), Some("接尾" | "自立" | "非自立"), Some("*"), Some("*"), None)
        | (Some("助動詞"), Some("非自立"), Some("助動詞語幹"), Some("*"), None) => {
            Some(10)
        }
        (Some("名詞"), Some("特殊"), Some("助動詞語幹"), Some("*"), None) => Some(2),
        _ => None,
    }
}

pub fn pos_to_id(pos: &POS) -> Option<u8> {
    match pos {
        // その他:xx
        POS::Others => None,
        // 感動詞:9
        POS::Kandoushi => Some(9),
        // 記号:xx
        POS::Kigou(_) => None,
        // 形状詞:19
        POS::Meishi(Meishi::KeiyoudoushiGokan) => Some(19),
        // 形容詞:1
        POS::Keiyoushi(Keiyoushi::Jiritsu | Keiyoushi::Hijiritsu) => Some(1),

        // 助詞-格助詞:13
        POS::Joshi(Joshi::KakuJoshi(_)) => Some(13),
        // 助詞-係助詞:24
        POS::Joshi(Joshi::KakariJoshi) => Some(24),
        // 助詞-終助詞:14
        POS::Joshi(Joshi::ShuJoshi) => Some(14),
        // 助詞-接続助詞:12
        POS::Joshi(Joshi::SetsuzokuJoshi) => Some(12),
        // 助詞-副助詞:11
        POS::Joshi(Joshi::FukuJoshi) => Some(11),
        // 助詞-その他:23
        POS::Joshi(_) => Some(23),

        // 助動詞:10
        POS::Jodoushi => Some(10),
        // 接続詞:8
        POS::Setsuzokushi => Some(8),

        // 接頭辞-形状詞的:16
        // 接頭辞-形容詞的:16
        // 接頭辞-動詞的:16
        // 接頭辞-名詞的:16
        // 接頭辞:16
        POS::Settoushi(_) => Some(16),

        // 接尾辞-形状詞的:15
        POS::Meishi(Meishi::Setsubi(Setsubi::KeiyoudoushiGokan)) => Some(15),
        // 接尾辞-形容詞的:15
        POS::Keiyoushi(Keiyoushi::Setsubi) => Some(15),
        // 接尾辞-動詞的:15
        POS::Doushi(Doushi::Setsubi) => Some(15),
        // 接尾辞-名詞的:15
        POS::Meishi(Meishi::Setsubi(_)) => Some(15),

        // 代名詞:4
        POS::Meishi(Meishi::Daimeishi(_)) => Some(4),

        // 動詞:20
        POS::Doushi(Doushi::Jiritsu) => Some(20),
        // 動詞-非自立:17
        POS::Doushi(Doushi::Hijiritsu) => Some(17),
        // 副詞:6
        POS::Fukushi(_) => Some(6),

        // 名詞-サ変接続:3
        POS::Meishi(Meishi::SahenSetsuzoku) => Some(3),
        // 名詞-固有名詞:18
        POS::Meishi(Meishi::KoyuMeishi(_)) => Some(18),
        // 名詞-数詞:5
        POS::Meishi(Meishi::Kazu) => Some(5),
        // 名詞-非自立:22
        POS::Meishi(Meishi::Hijiritsu(_)) => Some(22),
        POS::Meishi(Meishi::None | Meishi::Special) => None,
        // 名詞-普通名詞:2
        POS::Meishi(_) => Some(2),

        // 連体詞:7
        POS::Rentaishi => Some(7),
        // フィラーは NJD と JPCommon の間で感動詞に変換される。
        POS::Filler => Some(9),

        POS::Unknown => None,
    }
}
