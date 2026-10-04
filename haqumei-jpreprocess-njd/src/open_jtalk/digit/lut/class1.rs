use super::*;
use haqumei_jpreprocess_core::pron;
use phf::{phf_map, phf_set};

pub const CONVERSION_TABLE: [(Keys, DigitLUT); 12] = [
    (NUMERATIVE_CLASS1B, CONV_TABLE1B),
    (NUMERATIVE_CLASS1C1, CONV_TABLE1C1),
    (NUMERATIVE_CLASS1C2, CONV_TABLE1C2),
    (NUMERATIVE_CLASS1D, CONV_TABLE1D),
    (NUMERATIVE_CLASS1E, CONV_TABLE1E),
    (NUMERATIVE_CLASS1F, CONV_TABLE1F),
    (NUMERATIVE_CLASS1G, CONV_TABLE1G),
    (NUMERATIVE_CLASS1H, CONV_TABLE1H),
    (NUMERATIVE_CLASS1I, CONV_TABLE1I),
    (NUMERATIVE_CLASS1J, CONV_TABLE1J),
    (NUMERATIVE_CLASS1K, CONV_TABLE1K),
    (NUMERATIVE_CLASS1L, CONV_TABLE1L),
];

const NUMERATIVE_CLASS1B: Keys = phf_set! {
    "年", "円", "年間", "年生", "年代", "年度", "年版", "年余",
    "年来", "えん",
};
pub const CONV_TABLE1B: DigitLUT = phf_map! {
    "四" => pron!([Yo], 0),
};

const NUMERATIVE_CLASS1C1: Keys = phf_set! {
    "人", "人月", "人前", "人組",
};
const CONV_TABLE1C1: DigitLUT = phf_map! {
    "四" => pron!([Yo], 0),
    "七" => pron!([Shi, Chi], 1),
};

const NUMERATIVE_CLASS1C2: Keys = phf_set! {
    "時", "時間", "時限", "時半",
};
const CONV_TABLE1C2: DigitLUT = phf_map! {
    "四" => pron!([Yo], 0),
    "七" => pron!([Shi, Chi], 1),
    "九" => pron!([Ku], 0),
};

const NUMERATIVE_CLASS1D: Keys = phf_set! {
    "日", "日間",
};
const CONV_TABLE1D: DigitLUT = phf_map! {
   /* "四", "ヨッ", "1", "2", *//* modified */
    "七" => pron!([Shi, Chi], 1),
    "九" => pron!([Ku], 0),
};

const NUMERATIVE_CLASS1E: Keys = phf_set! {
    "月",
};
pub const CONV_TABLE1E: DigitLUT = phf_map! {
   "四" => pron!([Shi], 0),
   "七" => pron!([Shi, Chi], 1),
   "九" => pron!([Ku], 0),
};

const NUMERATIVE_CLASS1F: Keys = phf_set! {};
const CONV_TABLE1F: DigitLUT = phf_map! {
    "六" => pron!([Ro, Xtsu], 1),
    "八" => pron!([Ha, Xtsu], 1),
    "十" => pron!([Ju, Xtsu], 1),
    "百" => pron!([Hya, Xtsu], 1),
};

const NUMERATIVE_CLASS1G: Keys = phf_set! {
    "個", "階", "分", "発", "本", "鉢", "口", "箱",
    "か月", "か国", "か所", "か条", "か村", "か年", "カ月", "カ国",
    "カ寺", "カ所", "カ条", "カ村", "カ店", "カ年", "ケ月", "ケ国",
    "ケ所", "ケ条", "ケ村", "ケ年", "ヵ月", "ヵ国", "ヵ所", "ヵ条",
    "ヵ村", "ヵ年", "ヶ月", "ヶ国", "ヶ所", "ヶ条", "ヶ村", "ヶ年",
    "個月", "個口", "個国", "個条", "個年", "箇月", "箇国", "箇所",
    "箇条", "箇年", "かけ", "くだり", "けた", "価", "画", "回",
    "回忌", "回生", "回戦", "回線", "回分", "角", "冠", "巻",
    "貫", "貫目", "間", "基", "期", "期生", "機", "季",
    "騎", "脚", "級", "橋", "局", "曲", "極", "重ね",
    "金", "句", "躯", "計", "桁", "ケタ", "校", "港",
    "項", "組", "件", "軒", "言", "湖", "光年", "石",
    "ぴき", "ぺん", "派", "敗", "杯", "拍", "泊", "版",
    "犯", "班", "匹", "疋", "筆", "俵", "票", "品",
    "分間", "片", "篇", "編", "辺", "遍", "歩", "報",
    "方", "法", "本立て", "頭身",
};
const CONV_TABLE1G: DigitLUT = phf_map! {
   "一" => pron!([I, Xtsu], 1),
   "六" => pron!([Ro, Xtsu], 1),
   "八" => pron!([Ha, Xtsu], 1),
   "十" => pron!([Ju, Xtsu], 1),
   "百" => pron!([Hya, Xtsu], 1),
};

const NUMERATIVE_CLASS1H: Keys = phf_set! {
    "．", "・", "才", "頭", "着", "足", "尺", "坪",
    "通り", "センチ", "センチメートル", "ｃｍ", "サイクル", "サンチーム", "シーズン", "シリング",
    "シンガポールドル", "スイスフラン", "スウェーデンクローネ", "スクレ", "セット", "セント", "ソル", "ゾーン",
    "糎", "竿", "差", "差し", "歳", "歳児", "作", "冊",
    "刷", "棹", "艘", "子", "視", "式", "失", "室",
    "射", "社", "勺", "種", "首", "周", "周忌", "周年",
    "州", "週", "週間", "集", "宿", "所", "勝", "升",
    "床", "章", "色", "食", "親等", "進", "進数", "品",
    "すじ", "そう", "そろい", "筋", "数", "寸", "世", "隻",
    "席", "石", "節", "戦", "線", "選", "銭", "層",
    "相", "揃", "たび", "つかみ", "つがい", "つぶ", "つまみ", "つ折",
    "つ折り", "とき", "ところ", "とせ", "月", "手", "続き", "体",
    "対", "卓", "樽", "反", "丁", "丁目", "鳥", "通",
    "掴み", "艇", "滴", "店", "転", "点", "斗", "棟",
    "盗", "灯", "等", "等席", "等地", "等分", "答", "得",
    "噸", "粒", "種類", "歳馬", "世紀", "車種",
};
const CONV_TABLE1H: DigitLUT = phf_map! {
    "一" => pron!([I, Xtsu], 1),
    "八" => pron!([Ha, Xtsu], 1),
    "十" => pron!([Ju, Xtsu], 1),
};

const NUMERATIVE_CLASS1I: Keys = phf_set! {
    "キロ", "カロリー", "ｃａｌ", "ｋｂ", "ｋｇ", "ｋｌ", "ｋｍ", "ｋｔ",
    "ｋｗ", "ｋグラム", "ｋバイト", "ｋヘルツ", "ｋメートル", "ｋリットル", "ｋワット", "カナダドル",
    "ガロン", "キュリー", "キロカロリー", "キログラム", "キロトン", "キロバイト", "キロヘルツ", "キロメートル",
    "キロリットル", "キロワット", "キロワット時", "クラス", "クローナ", "クローネ", "グァラニ", "ケース",
    "コース", "粁", "海里", "カイリ", "浬", "気圧", "株", "切れ",
    "機種", "区画", "区間",
};
pub const CONV_TABLE1I: DigitLUT = phf_map! {
    "六" => pron!([Ro, Xtsu], 1),
    "十" => pron!([Ju, Xtsu], 1),
    "百" => pron!([Hya, Xtsu], 1),
};

const NUMERATIVE_CLASS1J: Keys = phf_set! {
    "トン", "ｔ", "タル", "テラ", "トライ", "皿", "市", "種目",
};
pub const CONV_TABLE1J: DigitLUT = phf_map! {
    "一" => pron!([I, Xtsu], 1),
    "十" => pron!([Ju, Xtsu], 1),
};

const NUMERATIVE_CLASS1K: Keys = phf_set! {
    "房", "％", "ポンド", "ｐａ", "ｐｐｍ", "パーセント", "パーミル", "パスカル",
    "パック", "パット", "ピーピーエム", "ピコ", "ページ", "頁", "ペア", "ペセタ",
    "ペソ", "ペニー", "ペニヒ", "ペンス", "ポイント", "振り", "針", "袋",
    "張り", "平米", "平方キロ", "平方キロメートル", "品目", "ＣＣ", "ｃｃ", "シーシー",
    "シート", "束", "玉", "家族", "カップ", "系統", "工程", "項目",
    "シーベルト", "世帯", "チーム", "地区", "地点", "店舗", "フィート", "ヘルツ",
    "部屋", "試合",
};
pub const CONV_TABLE1K: DigitLUT = phf_map! {
    "十" => pron!([Ju, Xtsu], 1),
};

const NUMERATIVE_CLASS1L: Keys =
    phf_set! { "課", "缶", "客", "球", "球目", "斤", "区", "戸", "波" };

pub const COUNTER_WORDS: Keys = phf_set! {
    "機種", "区画", "区間", "市", "種目", "選", "センチメートル", "家族",
    "カップ", "系統", "工程", "項目", "シーベルト", "世帯", "チーム", "地区",
    "地点", "店舗", "フィート", "ヘルツ", "場所", "役", "玉", "アンダー",
    "部屋", "試合",
};

pub const CONV_TABLE1L: DigitLUT = phf_map! {
    "一" => pron!([I, Xtsu], 1),
    "六" => pron!([Ro, Xtsu], 1),
    "十" => pron!([Ju, Xtsu], 1),
    "百" => pron!([Hya, Xtsu], 1),
};

pub const CONV_TABLE_TEN_HUNDRED: DigitLUT = phf_map! {
    "十" => pron!([Ju, Xtsu], 1),
    "百" => pron!([Hya, Xtsu], 1),
};

pub const CONV_TABLE_EIGHT_TEN: DigitLUT = phf_map! {
    "八" => pron!([Ha, Xtsu], 1),
    "十" => pron!([Ju, Xtsu], 1),
};

pub const CONV_TABLE_SEVEN: DigitLUT = phf_map! {
    "七" => pron!([Shi, Chi], 1),
};

pub const CONV_TABLE_NATIVE: DigitLUT = phf_map! {
    "三" => pron!([Mi], 1),
    "四" => pron!([Yo], 1),
    "六" => pron!([Mu], 1),
    "八" => pron!([Ya], 1),
    "十" => pron!([To], 1),
};

pub const CONV_TABLE_UNDER: DigitLUT = phf_map! {
    "一" => pron!([Wa, N], 1),
    "二" => pron!([Tsu, Long], 1),
    "三" => pron!([Su, Ri, Long], 1),
    "四" => pron!([Fo, Long], 1),
    "五" => pron!([Fa, I, Bu], 1),
    "六" => pron!([Shi, Xtsu, Ku, Su], 1),
    "七" => pron!([Se, Bu, N], 1),
    "八" => pron!([E, I, To], 1),
    "九" => pron!([Na, I, N], 1),
    "十" => pron!([Te, N], 1),
};
