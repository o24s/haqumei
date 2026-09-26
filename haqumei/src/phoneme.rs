//! 日本語の音素 (Phoneme) 定義と、異音 (allophone) の解決ロジック。
//!
//! 本モジュールでは、音声合成のフロントエンド処理において使用される音素の
//! 列挙型 ([`Phoneme`]) と、その音韻的特徴を判定するメソッド群を提供します。
//!
//! [`Haqumei`] からこれらのロジックを使用するには、[`HaqumeiOptions`]
//! で設定を行う必要があります。
//!
//! # 「ン」「ッ」の異音解決に関する音声学的・音韻論的背景
//!
//! 詳細は各アイテムのドキュメントコメントを参照してください。
//! - [`HaqumeiOptions::split_n_allophones`]
//! - [`HaqumeiOptions::split_n_before_r`]
//! - [`HaqumeiOptions::split_n_before_palatal_affricate`]
//! - [`HaqumeiOptions::split_q_allophones`]
//! - [`HaqumeiOptions::enable_final_glottal_stop`]
//! - [`Phoneme::resolve_n_allophone`]
//! - [`Phoneme::resolve_q_allophone`]
//! - [`Phoneme::resolve_q_final_glottal_stop`]
//!
//! ## 設計上の基本方針
//!
//! - 異音ラベルは音響モデルへ渡すための分類です。構音の連続変異や
//!   閉鎖中の部分的な無声化まで、一つのラベルで予測するものではありません。
//! - r・ch・j の前の撥音は、分類を粗く保つため既定で `Nd` にまとめています。
//!   `Nr`・`Npl` への分離も選べますが、既定の分類が調音点の同一性を保証する
//!   わけではありません。
//!   参照した研究だけでは細分化の直接的な裏付けが十分でない可能性があるため、
//!   分離は既定で無効にしています。各記述や音響モデルとの対応に必要な場合には、
//!   [`HaqumeiOptions::split_n_before_r`] と
//!   [`HaqumeiOptions::split_n_before_palatal_affricate`] で選択できます。
//! - s・y・hy などの前の撥音には、口腔閉鎖を伴う実現と鼻音化母音の両方が
//!   観測されています。後続音素だけでは選べないため、`Nn` を出力します。
//! - 発話末撥音の調音位置は連続的に変わるため、母音別の細分化を
//!   提供していません。`Nq` は発話末という環境を表すラベルです。
//!
//! ## 主な参照文献
//!
//! - Maekawa, K. (2019). A real-time MRI study of Japanese moraic nasal in
//!   utterance-final position. *Proceedings of ICPhS XIX*, Melbourne, 1987–1991.
//! - Maekawa, K. (2023). Production of the utterance-final moraic nasal
//!   in Japanese: A real-time MRI study. *Journal of the International
//!   Phonetic Association*, 53(1), 189–212.
//! - Fujimoto, M., Maekawa, K., & Funatsu, S. (2010). Laryngeal
//!   characteristics during the production of geminate consonants.
//!   *Proceedings of Interspeech 2010*, Makuhari, 925–928.
//! - Kawahara, S. (2005). Voicing and geminacy in Japanese: An acoustic and
//!   perceptual study. *UMOP* 31, 87–120.
//! - Kawahara, S. (2015). The phonetics of sokuon, or geminate obstruents.
//!   In *Handbook of Japanese Phonetics and Phonology*, 43–78.
//! - Maekawa, K. (2023). Articulatory characteristics of the Japanese /r/:
//!   A real-time MRI study.
//!   *Proceedings of ICPhS 2023*, Prague, pp.992-996, 2023.08.10
//! - Yoshinaga, T., Maekawa, K., & Iida, A. (2022). Variability in Production of
//!   Non-Sibilant Fricative `[ç]` in /hi/.
//!   *Proceedings of Interspeech 2022*, 620–624.
//! - Vance, T. J. (2008). *The Sounds of Japanese*. Cambridge University Press.
//! - Okada, H. (1999). Japanese. In *Handbook of the International Phonetic
//!   Association*. Cambridge University Press.
//!
//! ## 注意点・既知の限界
//!
//! - Fujimoto et al. (2010) は被験者1名の語中促音を調べた予備的研究です。
//!   発話末の「ッ」を声門閉鎖とする規則は、その実験からは導けません。
//! - Maekawa (2023, JIPA) の Alveolar・Palatal などは分析前に与えた分類です。
//!   r・ch を Alveolar に含めたことだけでは、群内の調音点の差を検証できません。
//! - Maekawa (2023, ICPhS) の /r/ の実験は語頭・母音間を対象としており、
//!   /Nr/ の撥音を直接調べたものではありません。
//! - Vance (2008) と Okada (1999) の記述は、異音ラベルを選ぶ際の参照先ですが、
//!   記述間の違いだけでは各実現の頻度や適用範囲を確定できません。
//!   「参照した研究から直接の裏付けを得られない」ことと、
//!   「実測によってその区別が否定された」ことは区別しています。

#[cfg(doc)]
use crate::Haqumei;
#[cfg(doc)]
use crate::HaqumeiOptions;

use haqumei_macros::phonemes;

phonemes! {
    UnvoicedA = "A",
    UnvoicedE = "E",
    UnvoicedI = "I",
    UnvoicedO = "O",
    UnvoicedU = "U",

    // 撥音「ン」とその異音

    /// Moraic nasal (ン): デフォルト・未解決
    Nn   = "N",

    // Nn は "ン" を表す `HaqumeiOptions::split_n_*` オプション無効化時のデフォルトで、
    // これは pyopenjtalk(-plus) の "ン" と同様の表現です。
    //
    // `HaqumeiOptions::split_n_allophones` が有効でも、実現を選べない環境では
    // Nn を出力する。口腔閉鎖を伴う鼻音と鼻音化母音の両方を含みうる。
    // オプションによって、`resolve_n_allophone` で後続音素の環境に応じて
    // 以下の専用ラベルへ解決されることがあります。

    /// 両唇鼻音 `[m]`: p, b, m の前
    Nm   = "Nm",
    /// 軟口蓋鼻音 `[ŋ]`: k, g の前
    Ng   = "Ng",
    /// 歯茎鼻音 `[n]`: t, d, ts, n, z の前
    Nd   = "Nd",

    /// 発話末撥音 (Utterance-final moraic nasal): 発話境界の前
    ///
    /// 口蓋垂鼻音 `[ɴ]` と表記されることがありますが、これは慣習的な代表記号で
    /// あり、固定された調音点を主張するものではありません。実際の調音位置は
    /// 先行母音に応じて変動し、観測範囲は歯茎〜口蓋垂に及びます。
    /// (Maekawa, 2023, JIPA 53(1): 189-212)
    /// 理由は以下を参照してください。
    ///
    /// ## 発話末 `Nq` の細分化を提供しない理由
    ///
    /// 発話末の `Nq` を直前母音の前後性によって `[ŋ]`/`[ɴ]` に離散的に二値分岐させる
    /// オプションは、意図的に提供していません。Maekawa (2023, JIPA 53(1):
    /// 189-212) のリアルタイムMRI観測により、この変動は実際には連続的な
    /// 調音位置の変動であり、統計的にも前舌/後舌の2分法ではなく
    /// 「{i} / {e, u} / {a, o}」という3水準のグルーピングに近いことが
    /// 示されています。離散的な二値ルールで近似すると、実態にない判断を
    /// データへ埋め込むことになるためです。
    Nq   = "Nq",

    // 直接的な裏付けが十分でない可能性のある細分化
    //
    // r 前の撥音について、Vance (2008, p.97) は尖端歯茎 [n̺] と記述している。
    // t/d/n 前の舌面による閉鎖との差であり、調音点を後方にずらす [n̠] とは異なる。
    // Okada (1999) の /r/ は後部歯茎音として記述されている。先行する撥音も
    // 後部歯茎鼻音とみなす場合の表記が [n̠] であり、Vance の [n̺] とは異なる。
    // Nr は r/ry 前の撥音を区別するラベルで、特定の閉鎖位置を定義に含めていない。
    // ch・j 前については、Vance (2008, pp.96–97) に歯茎硬口蓋鼻音の記述がある。
    // Npl は、狭い表記の細部を省いた代表記号 [ɲ] に対応させている。
    //
    // Maekawa (2023, JIPA, §3.1・§4.1) は r と ch を Alveolar 群に分類している。
    // ただし、後続子音に基づいて分析前に与えた分類であり、r/ch と他の歯茎音の
    // 群内差を検定したものではない。群への所属だけで細分化を支持・否定はできない。
    // j [dʑ] は当該分類に記載がないため、ch と同じ規則を適用するのは実装上の類推。
    //
    // Maekawa (2023, ICPhS) の /r/ は男性話者7名の167例すべてが tap であり、
    // 後部歯茎の flap とする記述とは一致しなかった。ただし語頭・母音間の /r/ を
    // 調べた研究であり、/Nr/ の撥音の直接観測ではない。
    //
    // 参照した研究だけでは細分化の裏付けが十分でない可能性があるため、
    // 既定の Nd は粗い分類として採用し、Nr・Npl への分離はオプションにしている。
    // Nd への統合も、すべての環境で調音点が同じだという実測結果を表すものではない。

    /// 硬口蓋鼻音 `[ɲ]`: 分離オプションを有効にした場合の ch, j の前
    ///
    /// 歯茎硬口蓋鼻音を区別する記述との対応に使います。
    /// 参照した rtMRI 研究だけでは、この区別の直接的な裏付けは十分ではありません。
    /// j の前にも ch と同じラベルを返す処理は、音韻的な対応関係からの類推を含みます。
    Npl  = "Npl",

    /// r/ry の前の撥音を区別するラベル。
    ///
    /// [`HaqumeiOptions::split_n_before_r`] による分離時に出力します。
    /// 後部歯茎鼻音と解釈する場合の表記は `[n̠]` ですが、r 前の撥音が常に
    /// 後部歯茎で閉鎖するという直接的な裏付けは、参照した研究からは得られません。
    /// 尖端歯茎 `[n̺]` は舌先を使うことを表し、閉鎖位置を後方へずらす `[n̠]` とは異なります。
    Nr   = "Nr",


    A    = "a",
    B    = "b",
    By   = "by",
    Ch   = "ch",

    // 促音「ッ」とその異音

    /// 促音(ッ): デフォルト・未解決
    Cl   = "cl",

    // Cl は pyopenjtalk(-plus) と同じ "ッ" の表現。
    // `split_q_allophones` と `enable_final_glottal_stop` で解決しない場合に出力する。
    //
    // `split_q_allophones`, `enable_final_glottal_stop` オプションによって、
    // それぞれ `resolve_q_allophone` / `resolve_q_final_glottal_stop` を通して
    // 後続音素の環境に応じて以下の専用ラベルへ解決されることがあります。

    /// 無声・両唇閉鎖: p の前
    ClP  = "clp",
    /// 無声・歯茎(硬口蓋)閉鎖: t, ts, ch の前
    ClT  = "clt",
    /// 無声・軟口蓋閉鎖: k の前
    ClK  = "clk",
    /// 摩擦・接近の継続 (無声/有声を問わない): s, sh, f, h, v の前
    ClS  = "cls",
    /// 音韻的に有声の閉鎖・破擦: b, d, g, z, j の前。部分的な無声化を含みます。
    ClV  = "clv",

    /// 発話末の促音を声門閉鎖 `[ʔ]` として表す慣習的なラベル
    ClQ  = "clq",

    D    = "d",
    Dy   = "dy",
    E    = "e",
    F    = "f",
    Fy   = "fy",
    G    = "g",
    Gw   = "gw",
    Gy   = "gy",
    H    = "h",
    Hy   = "hy",
    I    = "i",
    J    = "j",
    K    = "k",
    Kw   = "kw",
    Ky   = "ky",
    M    = "m",
    My   = "my",
    N    = "n",
    Ny   = "ny",
    O    = "o",
    P    = "p",
    Py   = "py",
    R    = "r",
    Ry   = "ry",
    S    = "s",
    Sh   = "sh",
    T    = "t",
    Ts   = "ts",
    Ty   = "ty",
    U    = "u",
    V    = "v",
    W    = "w",
    Y    = "y",
    Z    = "z",
    Sp   = "sp",
    Pau  = "pau",
    Unk  = "unk",
}

impl Phoneme {
    /// 指定された異音解決のフラグ状況において、出力されうるすべての音素の集合を返します。
    pub fn possible_phonemes(
        split_n_allophones: bool,
        split_n_before_r: bool,
        split_n_before_palatal_affricate: bool,
        split_q_allophones: bool,
        enable_final_glottal_stop: bool,
    ) -> &'static [Phoneme] {
        let mut idx = 0;
        if split_n_allophones {
            idx |= 1;
        }
        if split_n_before_r {
            idx |= 2;
        }
        if split_n_before_palatal_affricate {
            idx |= 4;
        }
        if split_q_allophones {
            idx |= 8;
        }
        if enable_final_glottal_stop {
            idx |= 16;
        }

        let list = &POSSIBLE_PHONEMES_TABLE[idx];
        &list.data[..list.len]
    }

    /// 無声音 (無声化母音、および無声子音) であるか判定します
    pub const fn is_unvoiced(&self) -> bool {
        self.is_unvoiced_vowel() || self.is_unvoiced_consonant()
    }

    /// 有声音 (有声母音、有声子音、撥音とその異音) であるか判定します
    ///
    /// なお、ポーズや不明な音は含みません。
    pub const fn is_voiced(&self) -> bool {
        self.is_voiced_vowel() || self.is_voiced_consonant() || self.is_moraic_nasal()
    }

    /// 声帯振動の有無 (有声・無声) がラベル単体では不定であるか判定します
    ///
    /// `ClS` は無声摩擦音の前にも有声摩擦音 v の前にも使うため、
    /// ラベル単体からは有声・無声を区別できません。
    pub const fn is_voicing_underspecified(&self) -> bool {
        self.is_continuant_sokuon()
    }

    /// 母音 (有声・無声両方) であるか判定します
    pub const fn is_vowel(&self) -> bool {
        self.is_voiced_vowel() || self.is_unvoiced_vowel()
    }

    /// 有声母音であるか判定します
    pub const fn is_voiced_vowel(&self) -> bool {
        matches!(self, Self::A | Self::E | Self::I | Self::O | Self::U)
    }

    /// 無声化母音であるか判定します
    pub const fn is_unvoiced_vowel(&self) -> bool {
        matches!(
            self,
            Self::UnvoicedA | Self::UnvoicedE | Self::UnvoicedI | Self::UnvoicedO | Self::UnvoicedU
        )
    }

    /// 撥音「ン」、またはその異音 (allophone) のいずれかであるかを判定します。
    ///
    /// `Nn` (未解決) に加え、`split_n_allophones` 等のオプションに
    /// よって解決され得る `Nm`/`Ng`/`Nd`/`Nq`/`Npl`/`Nr` をすべて含みます。
    pub const fn is_moraic_nasal(&self) -> bool {
        matches!(
            self,
            Self::Nn | Self::Nm | Self::Ng | Self::Nd | Self::Nq | Self::Npl | Self::Nr
        )
    }

    /// 促音、またはその異音 (allophone) のいずれかであるかを判定します。
    ///
    /// `Cl` (デフォルト・未解決) に加え、`split_q_allophones` / `enable_final_glottal_stop`
    /// によって解決され得る `ClP`/`ClT`/`ClK`/`ClS`/`ClV`/`ClQ` をすべて含みます。
    pub const fn is_sokuon(&self) -> bool {
        matches!(
            self,
            Self::Cl | Self::ClP | Self::ClT | Self::ClK | Self::ClS | Self::ClV | Self::ClQ
        )
    }

    /// 子音 (有声・無声両方) であるか判定します
    pub const fn is_consonant(&self) -> bool {
        self.is_unvoiced_consonant() || self.is_voiced_consonant() || self.is_continuant_sokuon()
    }

    /// 無声子音であるか判定します (促音 cl 系を含みません)
    pub const fn is_unvoiced_consonant(&self) -> bool {
        matches!(
            self,
            Self::K
                | Self::Ky
                | Self::Kw
                | Self::S
                | Self::Sh
                | Self::T
                | Self::Ts
                | Self::Ty
                | Self::Ch
                | Self::P
                | Self::Py
                | Self::F
                | Self::Fy
                | Self::H
                | Self::Hy
        )
    }

    /// 有声子音であるか判定します (撥音 Nn 系を含みません)
    ///
    /// `ClV` は音韻的な有声性で分類し、`is_sokuon` にも含めます。
    /// 閉鎖全体での声帯振動を保証するものではありません。
    // Kawahara (2005, pp.93–95) の被験者3名では、有声促音の声帯振動は
    // 閉鎖区間の約30–40%にとどまる。音韻的な有声性と実際の声帯振動は区別する。
    pub const fn is_voiced_consonant(&self) -> bool {
        matches!(
            self,
            Self::G
                | Self::Gy
                | Self::Gw
                | Self::Z
                | Self::J
                | Self::D
                | Self::Dy
                | Self::B
                | Self::By
                | Self::M
                | Self::My
                | Self::N
                | Self::Ny
                | Self::R
                | Self::Ry
                | Self::W
                | Self::Y
                | Self::V
                | Self::ClV
        )
    }

    /// 促音の異音のうち、摩擦・接近の継続によって発音されるものか判定します
    ///
    /// 声帯振動の有無 (無声/有声) は後続音素によって変わるため、
    /// is_voiced / is_unvoiced のどちらにも属しません。
    pub const fn is_continuant_sokuon(&self) -> bool {
        matches!(self, Self::ClS)
    }

    /// 閉鎖または休止として扱うラベルか判定します。
    ///
    /// 未解決の `Cl`、無声閉鎖の `ClP`/`ClT`/`ClK`、声門閉鎖の `ClQ`、
    /// `Pau`、`Sp` が該当します。録音の無音区間を判定するメソッドではありません。
    /// `ClS` は摩擦の継続として `is_continuant_sokuon` に、`ClV` は
    /// 音韻的な有声性によって `is_voiced_consonant` に分類します。
    pub const fn is_silent(&self) -> bool {
        matches!(
            self,
            Self::Cl | Self::ClP | Self::ClT | Self::ClK | Self::ClQ | Self::Pau | Self::Sp
        )
    }

    /// 無音 (ポーズ、スペース) であるか判定します
    pub const fn is_rest(&self) -> bool {
        matches!(self, Self::Sp | Self::Pau)
    }

    /// 特殊記号 (ポーズ、不明な音) であるか判定します
    pub const fn is_special(&self) -> bool {
        self.is_rest() || matches!(self, Self::Unk)
    }

    /// 語末・感嘆表現における、後続子音を伴わない促音を、声門閉鎖音
    /// `ClQ` に解決します。
    ///
    /// `self` が `Cl` 以外の場合、または `enable_final_glottal_stop == false` の場合、
    /// または `next` が発話境界 (後続音素なし、`Pau`, `Sp`) でない場合は、
    /// 何もせず常に `self` をそのまま返します。
    ///
    /// `ClQ` は発話末の促音を `[ʔ]` で表す規則であり、実際の声門閉鎖の
    /// 有無や長さを保証しません。
    ///
    /// ## 語中促音の実測と、発話末への適用の限界
    ///
    /// Fujimoto, Maekawa & Funatsu (2010) は、男性話者1名の語中促音を
    /// 高速度ビデオと光電声門図で調べ、声門の明確な緊縮を観測しませんでした。
    /// 閉鎖音・破擦音では声門開大の中断や直前母音の振動停止の差がありましたが、
    /// 著者らは話者を増やした追加検証が必要だとしています。
    ///
    /// 同研究は発話末の「ッ」を調べていないため、語中の観測から
    /// 発話末の声門閉鎖を実証したことにはなりません。
    /// 発話末を専用ラベルとして音響モデルへ渡す用途のために `ClQ` を設けており、
    /// 一律に `[ʔ]` とすることの直接的な根拠は同研究だけでは得られません。
    pub fn resolve_q_final_glottal_stop(
        self,
        next: Option<Phoneme>,
        enable_final_glottal_stop: bool,
    ) -> Phoneme {
        if self != Phoneme::Cl || !enable_final_glottal_stop {
            return self;
        }

        match q_environment(next) {
            QEnvironment::UtteranceBoundary => Phoneme::ClQ,
            QEnvironment::VoicelessBilabialStop
            | QEnvironment::VoicelessAlveolarStopOrAffricate
            | QEnvironment::VoicelessVelarStop
            | QEnvironment::VoicelessOrUnmarkedContinuant
            | QEnvironment::VoicedStopOrAffricate
            | QEnvironment::Unresolved => self,
        }
    }

    /// 語中における促音の異音 (allophone) を解決します
    /// (語末・感嘆表現の声門閉鎖は対象としません。
    /// `resolve_q_final_glottal_stop` を使用してください)。
    ///
    /// `self` が `Cl` 以外の場合、または `split_q_allophones == false` の場合は
    /// 常に `self` をそのまま返します。`next` が発話境界の場合も、
    /// このメソッドでは解決しません (語末の処理を意図的に分離しているため)。
    ///
    /// ## 解決規則
    ///
    /// - 無声破裂・破擦音 (p / t,ts,ch / k) の前: `ClP` / `ClT` / `ClK`
    /// - 摩擦が継続する音 (s, sh, f, h, および有声摩擦音 v) の前: `ClS`
    /// - 有声閉鎖・破擦音 (b, d, g, z, j) の前: `ClV`
    ///
    /// `v` の前も摩擦の継続として `ClS` を返します。
    ///
    /// ## `v` と `z` の分類について
    ///
    /// 有声摩擦音 `v` は口腔閉鎖を持たないため、`ClS` (摩擦の継続) に分類します。
    /// 「ッヴ」自体の構音を検証した規則ではなく、分類の一貫性を優先した扱いです。
    /// 参照した文献による直接の実証的な裏付けはありません。
    ///
    /// `z` の促音には `[oddzɯ]` のような破擦音の記述があるため、
    /// 閉鎖を持つ阻害音として `ClV` を採用しています。
    /// Kawahara (2015, p.54, 注13) は破擦化を可能性として述べており、
    /// すべての発音に必須の実現だとはしていません。
    pub fn resolve_q_allophone(self, next: Option<Phoneme>, split_q_allophones: bool) -> Phoneme {
        if self != Phoneme::Cl || !split_q_allophones {
            return self;
        }

        match q_environment(next) {
            QEnvironment::VoicelessBilabialStop => Phoneme::ClP,
            QEnvironment::VoicelessAlveolarStopOrAffricate => Phoneme::ClT,
            QEnvironment::VoicelessVelarStop => Phoneme::ClK,
            QEnvironment::VoicelessOrUnmarkedContinuant => Phoneme::ClS,
            QEnvironment::VoicedStopOrAffricate => Phoneme::ClV,
            QEnvironment::UtteranceBoundary | QEnvironment::Unresolved => Phoneme::Cl,
        }
    }

    /// 撥音「ン」の異音 (allophone) を解決します。
    ///
    /// `self` が `Nn` 以外の場合、または `split_n_allophones == false` の場合は
    /// 常に `self` をそのまま返します。
    ///
    /// ## 解決規則
    ///
    /// - 両唇音 (p, b, m) の前: `Nm` `[m]`
    /// - 軟口蓋音 (k, g) の前: `Ng` `[ŋ]`
    /// - 歯茎音 (t, d, ts, n, z) の前: `Nd` `[n]`
    /// - 発話境界 (後続音素なし、pau, sp) の前: `Nq` `[ɴ]`
    /// - 流音 r の前: デフォルトで `Nd` に統合 (`split_n_before_r` で `Nr` に分離可能)
    /// - 破擦音 ch, j の前: デフォルトで `Nd` に統合
    ///   (`split_n_before_palatal_affricate` で `Npl` に分離可能)
    /// - 母音・無声化母音・半母音 (y, w) ・無声摩擦音 (s, sh, h, f) ・有声摩擦音
    ///   (v) の前: 解決せず `Nn` のまま
    ///   (`Nn` は鼻音化母音に限定したラベルではありません)
    ///
    /// 発話末は単一の [`Phoneme::Nq`] で表し、直前母音による細分化は行いません。
    pub fn resolve_n_allophone(
        self,
        next: Option<Phoneme>,
        split_n_allophones: bool,
        split_n_before_r: bool,
        split_n_before_palatal_affricate: bool,
    ) -> Phoneme {
        if self != Phoneme::Nn || !split_n_allophones {
            return self;
        }

        match n_environment(next) {
            NEnvironment::Bilabial => Phoneme::Nm,
            NEnvironment::Velar => Phoneme::Ng,
            NEnvironment::Alveolar => Phoneme::Nd,
            NEnvironment::Liquid => {
                if split_n_before_r {
                    Phoneme::Nr
                } else {
                    Phoneme::Nd
                }
            }
            NEnvironment::PalatalAffricate => {
                if split_n_before_palatal_affricate {
                    Phoneme::Npl
                } else {
                    Phoneme::Nd
                }
            }
            NEnvironment::UtteranceBoundary => Phoneme::Nq,
            NEnvironment::Unresolved => Phoneme::Nn,
        }
    }
}

/// `resolve_n_allophone` で使用する後続音素による環境分類。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NEnvironment {
    /// 両唇音 (p, b, m) の前 -> 両唇鼻音 `[m]` に同化
    Bilabial,

    /// 軟口蓋音 (k, g) の前 -> 軟口蓋鼻音 `[ŋ]` に同化
    Velar,

    /// 歯茎音 (t, d, ts, n, z) の前 -> 歯茎鼻音 `[n]` に同化
    ///
    /// Maekawa (2023) は Alveolar と他の調音部位の群間差を検討しています。
    /// ただし、Alveolar 群内の個々の後続音で調音点が等しいことを意味しません。
    Alveolar,

    /// 破擦音 ch, j の前。既定では Nd、分離オプション有効時には Npl。
    ///
    /// Maekawa (2023) は ch を Alveolar に含めますが、群内差の検定はしていません。
    /// j の扱いは ch との音韻的な対応に基づく類推であり、直接の観測による
    /// 裏付けは十分でない可能性があります。
    PalatalAffricate,

    /// 流音 r の前。既定では Nd、分離オプション有効時には Nr。
    ///
    /// r 前を常に後部歯茎鼻音とすることは、参照した rtMRI 研究だけでは
    /// 裏付けられません。Nd への統合も、群内差がないという実測結果ではありません。
    Liquid,

    /// 発話境界 (後続音素なし、Pau, Sp) -> Utterance-final moraic nasal として解決
    UtteranceBoundary,

    /// 上記のいずれにも該当しない環境
    ///
    /// 母音・無声化母音・半母音 (y, w) ・無声摩擦音 (s, sh, f, h) ・有声摩擦音
    /// (v) ・促音系 (cl とその異音) ・撥音系自身・不明音 (unk) が該当します。
    /// 後続音素だけで実現を選べない環境も含むため、`Nn` を出力します。
    ///
    /// # 摩擦音・半母音の前についての留保
    ///
    /// Maekawa (2019, ICPhS) は、186例中25例の鼻音化母音を閉鎖位置の
    /// 分析から除外しています。`[s]`・`[ɸ]` の前の閉鎖位置も分析していますが、
    /// 閉鎖を持つ例に限定した結果から、閉鎖自体の出現頻度は決められません。
    ///
    /// Maekawa (2023, JIPA, Table 1, pp.194–195) の閉鎖を持つ語中 /N/ は、
    /// `[s]` の前で35/40例、`[j]` の前で25/33例、`[ç]` の前で6/11例でした。
    /// 対象語と話者を限定した件数ですが、鼻音化母音が大多数とはいえません。
    /// 同じ後続音でも閉鎖の有無が変わるため、一つの実現に固定していません。
    /// `Nn` は、鼻音化母音が多数派だという意味ではありません。
    ///
    /// # `[ç]` の構音変異と撥音への推測
    ///
    /// Yoshinaga, Maekawa & Iida (2022) は、被験者1名が持続発音した
    /// `[ç]` の声道形状を3回のMRIスキャンで比較し、変異を報告しています。
    /// 後続する `[ç]` の構音変異が撥音にも影響する可能性は考えられますが、
    /// 同研究は /Nç/ を測定していません。撥音の閉鎖位置の変異を直接示した
    /// 根拠ではなく、検証を要する推測として区別する必要があります。
    Unresolved,
}

fn n_environment(next: Option<Phoneme>) -> NEnvironment {
    use Phoneme::*;
    match next {
        None => NEnvironment::UtteranceBoundary,
        Some(Pau) | Some(Sp) => NEnvironment::UtteranceBoundary,

        Some(P) | Some(Py) | Some(B) | Some(By) | Some(M) | Some(My) => NEnvironment::Bilabial,

        Some(K) | Some(Ky) | Some(Kw) | Some(G) | Some(Gy) | Some(Gw) => NEnvironment::Velar,

        Some(T) | Some(Ty) | Some(Ts) | Some(D) | Some(Dy) | Some(N) | Some(Ny) | Some(Z) => {
            NEnvironment::Alveolar
        }

        Some(Ch) | Some(J) => NEnvironment::PalatalAffricate,

        Some(R) | Some(Ry) => NEnvironment::Liquid,

        Some(A) | Some(E) | Some(I) | Some(O) | Some(U) | Some(UnvoicedA) | Some(UnvoicedE)
        | Some(UnvoicedI) | Some(UnvoicedO) | Some(UnvoicedU) | Some(Y) | Some(W) | Some(S)
        | Some(Sh) | Some(F) | Some(Fy) | Some(H) | Some(Hy) | Some(V) | Some(Cl) | Some(ClP)
        | Some(ClT) | Some(ClK) | Some(ClS) | Some(ClV) | Some(ClQ) | Some(Nn) | Some(Nm)
        | Some(Ng) | Some(Nd) | Some(Nq) | Some(Npl) | Some(Nr) | Some(Unk) => {
            NEnvironment::Unresolved
        }
    }
}

/// `resolve_q_allophone` / `resolve_q_final_glottal_stop` に使用する、
/// 後続音素による環境分類。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QEnvironment {
    /// 無声両唇閉鎖 (p) の前 -> ClP に解決
    VoicelessBilabialStop,

    /// 無声歯茎(硬口蓋)閉鎖・破擦 (t, ts, ch) の前 -> ClT に解決
    VoicelessAlveolarStopOrAffricate,

    /// 無声軟口蓋閉鎖 (k) の前 -> ClK に解決
    VoicelessVelarStop,

    /// 摩擦の継続 (s, sh, f, h、および有声摩擦音 v) の前 -> ClS に解決
    ///
    /// 促音の実現が「無音区間か、摩擦継続か」を決定するのは声帯振動の有無
    /// ではなく構え (閉鎖か摩擦か) であるという原則に従い、有声/無声を問わず
    /// 摩擦音はここに分類しています。
    VoicelessOrUnmarkedContinuant,

    /// 有声閉鎖・破擦 (b, d, g, z, j) の前 -> ClV に解決
    VoicedStopOrAffricate,

    /// 発話境界 (後続音素なし、Pau, Sp) -> ClQ (声門閉鎖) に解決
    ///
    /// `resolve_q_allophone` 単体では関与しません
    /// (`resolve_q_final_glottal_stop` が個別に処理します)。
    UtteranceBoundary,

    /// 上記のいずれにも該当しない環境
    ///
    /// 母音・無声化母音・半母音 (y, w) ・鼻音 (m, n, ny, my, および撥音系) ・
    /// 流音 (r, ry) ・促音系自身・不明音 (unk) が該当します。標準的な日本語の
    /// 音韻論では、促音は阻害音 (破裂音・破擦音・摩擦音) の前にしか出現しない
    /// ため、`Cl` のまま残しておく。
    Unresolved,
}

fn q_environment(next: Option<Phoneme>) -> QEnvironment {
    use Phoneme::*;
    match next {
        None => QEnvironment::UtteranceBoundary,
        Some(Pau) | Some(Sp) => QEnvironment::UtteranceBoundary,

        Some(P) | Some(Py) => QEnvironment::VoicelessBilabialStop,

        Some(T) | Some(Ty) | Some(Ts) | Some(Ch) => QEnvironment::VoicelessAlveolarStopOrAffricate,

        Some(K) | Some(Ky) | Some(Kw) => QEnvironment::VoicelessVelarStop,

        Some(S) | Some(Sh) | Some(F) | Some(Fy) | Some(H) | Some(Hy) | Some(V) => {
            QEnvironment::VoicelessOrUnmarkedContinuant
        }

        Some(B) | Some(By) | Some(D) | Some(Dy) | Some(G) | Some(Gy) | Some(Gw) | Some(Z)
        | Some(J) => QEnvironment::VoicedStopOrAffricate,

        Some(A) | Some(E) | Some(I) | Some(O) | Some(U) | Some(UnvoicedA) | Some(UnvoicedE)
        | Some(UnvoicedI) | Some(UnvoicedO) | Some(UnvoicedU) | Some(Y) | Some(W) | Some(M)
        | Some(My) | Some(N) | Some(Ny) | Some(R) | Some(Ry) | Some(Cl) | Some(ClP) | Some(ClT)
        | Some(ClK) | Some(ClS) | Some(ClV) | Some(ClQ) | Some(Nn) | Some(Nm) | Some(Ng)
        | Some(Nd) | Some(Nq) | Some(Npl) | Some(Nr) | Some(Unk) => QEnvironment::Unresolved,
    }
}

#[derive(Debug, Clone, Copy)]
struct PhonemeList {
    len: usize,
    data: [Phoneme; Phoneme::ALL.len()],
}

impl PhonemeList {
    /// 特定のフラグの組み合わせに対する音素リストを計算して構築する const 関数
    const fn new(
        split_n_allophones: bool,
        split_n_before_r: bool,
        split_n_before_palatal_affricate: bool,
        split_q_allophones: bool,
        enable_final_glottal_stop: bool,
    ) -> Self {
        let mut data = [Phoneme::Unk; Phoneme::ALL.len()];
        let mut len = 0;
        let mut i = 0;

        while i < Phoneme::ALL.len() {
            let p = Phoneme::ALL[i];

            let include = match p {
                // 撥音の異音
                Phoneme::Nm | Phoneme::Ng | Phoneme::Nd | Phoneme::Nq => split_n_allophones,
                Phoneme::Nr => split_n_allophones && split_n_before_r,
                Phoneme::Npl => split_n_allophones && split_n_before_palatal_affricate,

                // 促音の異音
                Phoneme::ClP | Phoneme::ClT | Phoneme::ClK | Phoneme::ClS | Phoneme::ClV => {
                    split_q_allophones
                }

                // 語末・感嘆の声門閉鎖
                Phoneme::ClQ => enable_final_glottal_stop,

                // 基本音素は常に含まれる
                _ => true,
            };

            if include {
                data[len] = p;
                len += 1;
            }
            i += 1;
        }

        Self { len, data }
    }
}

const POSSIBLE_PHONEMES_TABLE: [PhonemeList; 32] = {
    let mut table = [PhonemeList::new(false, false, false, false, false); 32];
    let mut idx = 0;

    while idx < 32 {
        let split_n = (idx & 1) != 0;
        let split_n_r = (idx & 2) != 0;
        let split_n_pa = (idx & 4) != 0;
        let split_q = (idx & 8) != 0;
        let final_glottal = (idx & 16) != 0;

        table[idx] = PhonemeList::new(split_n, split_n_r, split_n_pa, split_q, final_glottal);
        idx += 1;
    }
    table
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_lookup_uses_the_same_hash_as_phonemes() {
        use std::collections::HashSet;

        let owned: HashSet<_> = Phoneme::ALL.iter().copied().collect();
        let borrowed: HashSet<_> = Phoneme::ALL.iter().collect();
        for phoneme in Phoneme::ALL {
            assert!(owned.contains(phoneme.as_str()), "{phoneme}");
            assert!(borrowed.contains(phoneme.as_str()), "{phoneme}");
        }
        assert!(!owned.contains("not-a-phoneme"));
    }

    #[test]
    fn test_phoneme_exhaustiveness() {
        let all_phonemes = Phoneme::ALL;

        for p in all_phonemes {
            // Sp, Pau を除くすべての音素は「有声音」「無声音」「閉鎖区間」
            // 「無音・特殊記号」「声帯振動不定(単体では声帯振動の有無が判定できない)」
            // のいずれか1つに必ず属するべき
            let is_voiced = p.is_voiced();
            let is_unvoiced = p.is_unvoiced();
            let is_silent = p.is_silent();
            let is_special = p.is_special();
            let is_voicing_unresolved = p.is_voicing_underspecified();

            let true_count = [
                is_voiced,
                is_unvoiced,
                is_silent,
                is_special,
                is_voicing_unresolved,
            ]
            .iter()
            .filter(|&&x| x)
            .count();

            if !matches!(p, Phoneme::Sp | Phoneme::Pau) {
                assert_eq!(
                    true_count, 1,
                    "{:?} の分類が正しくありません (voiced: {}, unvoiced: {}, silent: {}, special: {})",
                    p, is_voiced, is_unvoiced, is_silent, is_special
                );
            } else {
                assert_eq!(
                    true_count, 2,
                    "{:?} の分類が正しくありません (voiced: {}, unvoiced: {}, silent: {}, special: {})",
                    p, is_voiced, is_unvoiced, is_silent, is_special
                );
            }
        }
    }

    /// `n_environment` / `q_environment` が `Phoneme::ALL` の全要素 (および
    /// `None`) に対してパニックせず分類できることを確認する。
    #[test]
    fn test_n_and_q_environment_cover_all_phonemes() {
        assert_eq!(n_environment(None), NEnvironment::UtteranceBoundary);
        assert_eq!(q_environment(None), QEnvironment::UtteranceBoundary);

        for &p in Phoneme::ALL.iter() {
            let _ = n_environment(Some(p));
            let _ = q_environment(Some(p));
        }
    }

    #[test]
    fn test_resolve_n_allophone_core_cases() {
        let n = Phoneme::Nn;

        assert_eq!(
            n.resolve_n_allophone(Some(Phoneme::M), true, false, false),
            Phoneme::Nm
        );
        assert_eq!(
            n.resolve_n_allophone(Some(Phoneme::P), true, false, false),
            Phoneme::Nm
        );
        assert_eq!(
            n.resolve_n_allophone(Some(Phoneme::B), true, false, false),
            Phoneme::Nm
        );

        assert_eq!(
            n.resolve_n_allophone(Some(Phoneme::K), true, false, false),
            Phoneme::Ng
        );
        assert_eq!(
            n.resolve_n_allophone(Some(Phoneme::G), true, false, false),
            Phoneme::Ng
        );

        assert_eq!(
            n.resolve_n_allophone(Some(Phoneme::T), true, false, false),
            Phoneme::Nd
        );
        assert_eq!(
            n.resolve_n_allophone(Some(Phoneme::Z), true, false, false),
            Phoneme::Nd
        );

        assert_eq!(n.resolve_n_allophone(None, true, false, false), Phoneme::Nq);
        assert_eq!(
            n.resolve_n_allophone(Some(Phoneme::Pau), true, false, false),
            Phoneme::Nq
        );
        assert_eq!(
            n.resolve_n_allophone(Some(Phoneme::Sp), true, false, false),
            Phoneme::Nq
        );

        assert_eq!(
            n.resolve_n_allophone(Some(Phoneme::A), true, false, false),
            Phoneme::Nn
        );
        assert_eq!(
            n.resolve_n_allophone(Some(Phoneme::Y), true, false, false),
            Phoneme::Nn
        );
        assert_eq!(
            n.resolve_n_allophone(Some(Phoneme::S), true, false, false),
            Phoneme::Nn
        );
        assert_eq!(
            n.resolve_n_allophone(Some(Phoneme::H), true, false, false),
            Phoneme::Nn
        );
        assert_eq!(
            n.resolve_n_allophone(Some(Phoneme::V), true, false, false),
            Phoneme::Nn
        );
    }

    #[test]
    fn test_resolve_n_allophone_master_switch_off() {
        let n = Phoneme::Nn;
        assert_eq!(
            n.resolve_n_allophone(Some(Phoneme::P), false, true, true),
            Phoneme::Nn
        );
        assert_eq!(n.resolve_n_allophone(None, false, true, true), Phoneme::Nn);
    }

    #[test]
    fn test_resolve_n_allophone_low_confidence_options() {
        let n = Phoneme::Nn;

        // デフォルト (false) では r も ch/j も Nd に統合される
        assert_eq!(
            n.resolve_n_allophone(Some(Phoneme::R), true, false, false),
            Phoneme::Nd
        );
        assert_eq!(
            n.resolve_n_allophone(Some(Phoneme::Ch), true, false, false),
            Phoneme::Nd
        );
        assert_eq!(
            n.resolve_n_allophone(Some(Phoneme::J), true, false, false),
            Phoneme::Nd
        );

        // 個別に有効化すると専用ラベルに分かれる
        assert_eq!(
            n.resolve_n_allophone(Some(Phoneme::R), true, true, false),
            Phoneme::Nr
        );
        assert_eq!(
            n.resolve_n_allophone(Some(Phoneme::Ry), true, true, false),
            Phoneme::Nr
        );
        assert_eq!(
            n.resolve_n_allophone(Some(Phoneme::Ch), true, false, true),
            Phoneme::Npl
        );
        assert_eq!(
            n.resolve_n_allophone(Some(Phoneme::J), true, false, true),
            Phoneme::Npl
        );

        // 各オプションは独立して動作する (片方が他方に影響しない)
        assert_eq!(
            n.resolve_n_allophone(Some(Phoneme::Ch), true, true, false),
            Phoneme::Nd
        );
        assert_eq!(
            n.resolve_n_allophone(Some(Phoneme::R), true, false, true),
            Phoneme::Nd
        );
    }

    #[test]
    fn test_resolve_n_allophone_is_noop_for_non_nn() {
        assert_eq!(
            Phoneme::T.resolve_n_allophone(Some(Phoneme::K), true, true, true),
            Phoneme::T
        );
    }

    #[test]
    fn test_resolve_q_allophone_core_cases() {
        let cl = Phoneme::Cl;

        assert_eq!(cl.resolve_q_allophone(Some(Phoneme::P), true), Phoneme::ClP);
        assert_eq!(cl.resolve_q_allophone(Some(Phoneme::T), true), Phoneme::ClT);
        assert_eq!(
            cl.resolve_q_allophone(Some(Phoneme::Ts), true),
            Phoneme::ClT
        );
        assert_eq!(
            cl.resolve_q_allophone(Some(Phoneme::Ch), true),
            Phoneme::ClT
        );
        assert_eq!(cl.resolve_q_allophone(Some(Phoneme::K), true), Phoneme::ClK);

        assert_eq!(cl.resolve_q_allophone(Some(Phoneme::S), true), Phoneme::ClS);
        assert_eq!(
            cl.resolve_q_allophone(Some(Phoneme::Sh), true),
            Phoneme::ClS
        );
        assert_eq!(cl.resolve_q_allophone(Some(Phoneme::H), true), Phoneme::ClS);

        // v は ClV ではなく ClS とする。
        // ClV は「閉鎖区間 (closure) における有声」を表す。
        // 一方、ClS は摩擦・接近など閉鎖を伴わない継続区間 (無声・有声を問わない) を表す。
        // [v] は有声摩擦音であり閉鎖区間を持たないため、ClV ではなく ClS に分類する。
        assert_eq!(cl.resolve_q_allophone(Some(Phoneme::V), true), Phoneme::ClS);

        assert_eq!(cl.resolve_q_allophone(Some(Phoneme::B), true), Phoneme::ClV);
        assert_eq!(cl.resolve_q_allophone(Some(Phoneme::G), true), Phoneme::ClV);
        // z は破擦音的に振る舞うため ClV (グッズ [guddzu] のような実例)
        assert_eq!(cl.resolve_q_allophone(Some(Phoneme::Z), true), Phoneme::ClV);
        assert_eq!(cl.resolve_q_allophone(Some(Phoneme::J), true), Phoneme::ClV);
    }

    #[test]
    fn test_resolve_q_allophone_master_switch_off() {
        assert_eq!(
            Phoneme::Cl.resolve_q_allophone(Some(Phoneme::P), false),
            Phoneme::Cl
        );
    }

    #[test]
    fn test_resolve_q_allophone_does_not_touch_utterance_boundary() {
        // 語末の処理は resolve_q_final_glottal_stop の責務
        let cl = Phoneme::Cl;
        assert_eq!(cl.resolve_q_allophone(None, true), Phoneme::Cl);
        assert_eq!(
            cl.resolve_q_allophone(Some(Phoneme::Pau), true),
            Phoneme::Cl
        );
        assert_eq!(cl.resolve_q_allophone(Some(Phoneme::Sp), true), Phoneme::Cl);
    }

    #[test]
    fn test_resolve_q_final_glottal_stop() {
        let cl = Phoneme::Cl;
        assert_eq!(cl.resolve_q_final_glottal_stop(None, true), Phoneme::ClQ);
        assert_eq!(
            cl.resolve_q_final_glottal_stop(Some(Phoneme::Pau), true),
            Phoneme::ClQ
        );
        assert_eq!(
            cl.resolve_q_final_glottal_stop(Some(Phoneme::Sp), true),
            Phoneme::ClQ
        );
        assert_eq!(cl.resolve_q_final_glottal_stop(None, false), Phoneme::Cl);
    }

    /// 回帰テスト: 後続に阻害音があるごく普通の促音 (キップ等) には、
    /// `enable_final_glottal_stop` を有効にしていても声門閉鎖 `ClQ` が割り当て
    /// られてはならない。
    /// Fujimoto, Maekawa & Funatsu (2010) の観測により、通常の語中促音
    /// には声門の緊縮が見られないことが示されている。
    #[test]
    fn test_ordinary_medial_geminates_never_produce_glottal_stop() {
        let cl = Phoneme::Cl;
        let ordinary_following_consonants = [
            Phoneme::P,
            Phoneme::T,
            Phoneme::K,
            Phoneme::S,
            Phoneme::Sh,
            Phoneme::B,
            Phoneme::D,
            Phoneme::G,
            Phoneme::Z,
            Phoneme::J,
            Phoneme::V,
        ];

        for &next in &ordinary_following_consonants {
            assert_eq!(
                cl.resolve_q_final_glottal_stop(Some(next), true),
                Phoneme::Cl,
                "{:?} の前で誤って声門閉鎖になっています",
                next
            );
        }
    }

    #[test]
    fn test_resolve_q_allophone_non_obstruent_environments_are_noop() {
        // 促音は本来、阻害音以外の前には出現しないため
        // Cl のまま残ることを確認する
        let cl = Phoneme::Cl;
        let non_obstruent_environments = [
            Phoneme::A,
            Phoneme::Y,
            Phoneme::W,
            Phoneme::M,
            Phoneme::N,
            Phoneme::R,
        ];

        for &next in &non_obstruent_environments {
            assert_eq!(cl.resolve_q_allophone(Some(next), true), Phoneme::Cl);
        }
    }

    #[test]
    fn test_possible_phonemes_exhaustively() {
        for i in 0..32 {
            let split_n = (i & 1) != 0;
            let split_n_r = (i & 2) != 0;
            let split_n_pa = (i & 4) != 0;
            let split_q = (i & 8) != 0;
            let final_glottal = (i & 16) != 0;

            let phonemes =
                Phoneme::possible_phonemes(split_n, split_n_r, split_n_pa, split_q, final_glottal);

            assert!(phonemes.contains(&Phoneme::Nn));
            assert!(phonemes.contains(&Phoneme::Cl));
            assert!(phonemes.contains(&Phoneme::A));
            assert!(phonemes.contains(&Phoneme::Sp));
            assert!(phonemes.contains(&Phoneme::Pau));
            assert!(phonemes.contains(&Phoneme::Unk));

            assert_eq!(phonemes.contains(&Phoneme::Nm), split_n);
            assert_eq!(phonemes.contains(&Phoneme::Ng), split_n);
            assert_eq!(phonemes.contains(&Phoneme::Nd), split_n);
            assert_eq!(phonemes.contains(&Phoneme::Nq), split_n);

            assert_eq!(phonemes.contains(&Phoneme::Nr), split_n && split_n_r);
            assert_eq!(phonemes.contains(&Phoneme::Npl), split_n && split_n_pa);

            assert_eq!(phonemes.contains(&Phoneme::ClP), split_q);
            assert_eq!(phonemes.contains(&Phoneme::ClT), split_q);
            assert_eq!(phonemes.contains(&Phoneme::ClK), split_q);
            assert_eq!(phonemes.contains(&Phoneme::ClS), split_q);
            assert_eq!(phonemes.contains(&Phoneme::ClV), split_q);

            assert_eq!(phonemes.contains(&Phoneme::ClQ), final_glottal);

            let mut prev_position = None;
            for &p in phonemes {
                let current_position = Phoneme::ALL.iter().position(|&x| x == p).unwrap();

                if let Some(prev) = prev_position {
                    assert!(
                        prev < current_position,
                        "音素の順序が壊れています: 直前={:?}, 現在={:?}, (フラグのインデックス={})",
                        Phoneme::ALL[prev],
                        p,
                        i
                    );
                }
                prev_position = Some(current_position);
            }
        }
    }
}
