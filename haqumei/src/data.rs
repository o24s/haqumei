// 複数の読みを持つ漢字のリスト
// 多くはpyopenjtalk-plusのものを使用しています:
// https://github.com/tsukumijima/pyopenjtalk-plus/blob/ea2475413ef7b25d1fe0efee648611f9e19d83bb/pyopenjtalk/__init__.py#L55
pub(crate) const MULTI_READ_KANJI_LIST: phf::set::Set<&'static str> = phf::phf_set! {
    "風","観","方","出","時","上","下","君","手","嫌","表","対",
    "色","人","前","後","角","金","頭","筆","水","間","棚","奴",
    "降","中","入",

    "緒","通"
};
