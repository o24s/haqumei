use haqumei_macros::phonemes;

phonemes! {
    UnvoicedA = "A",
    UnvoicedE = "E",
    UnvoicedI = "I",
    UnvoicedO = "O",
    UnvoicedU = "U",

    // 撥音「ン」とその異音

    /// Moraic nasal (ん): デフォルト (鼻音化母音 / 未解決)
    Nn   = "N",

    // Nn は"ん"を表す `HaqumeiOptions::split_n_*` オプション無効化時のデフォルトで、
    // これは pyopenjtalk(-plus) の "ん" と同様の表現です。
    //
    // `HaqumeiOptions::split_n_allophones` のオプション有効時の未解決状態、
    // すなわち口腔閉鎖を伴わない鼻音化母音としての発音を表すフォールバック先でもあります。
    // オプションによって、`resolve_n_allophone` で後続音素の環境に応じて
    // 以下の専用ラベルへ解決されることがあります。

    /// 両唇鼻音 [m]: p, b, m の前
    Nm   = "Nm",
    /// 軟口蓋鼻音 [ŋ]: k, g の前
    Ng   = "Ng",
    /// 歯茎鼻音 [n]: t, d, ts, n, z の前
    Nd   = "Nd",
    /// 口蓋垂鼻音 [ɴ] (語末専用): 発話境界の前
    Nq   = "Nq",

    // 学術的な根拠が弱い専用ラベル

    /// 硬口蓋鼻音 [ɲ]: ch, j の前
    Npl  = "Npl",
    /// 後部歯茎鼻音 [n̠]: r の前
    Nr   = "Nr",

    A    = "a",
    B    = "b",
    By   = "by",
    Ch   = "ch",

    // 促音「ッ」とその異音

    /// 促音(ッ): デフォルト・未解決事件
    Cl   = "cl",

    // Cl は "ッ" を表す `split_n_*` オプション無効化時のデフォルトで、
    // これは pyopenjtalk(-plus) の "ッ" と同様の表現です。
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
    /// 摩擦の継続 (無声/有声を問わない): s, sh, f, h, v の前
    ClS  = "cls",
    /// 有声閉鎖 (声帯振動を伴う): b, d, g, z, j の前
    ClV  = "clv",

    /// 声門閉鎖 [ʔ] (語末・感嘆のみ): 発話境界の前
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
    /// `Nn` (未解決・鼻音化母音) に加え、`split_n_allophones` 等のオプションに
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
        self.is_unvoiced_consonant() || self.is_voiced_consonant()
    }

    /// 無声子音であるか判定します (促音 cl 系を含みません)
    ///
    /// `ClS` (摩擦の継続) は、独立した子音としての構えを持たない促音の異音
    /// であるため `is_sokuon` 側ではなくこちらに含めています。
    /// (摩擦が無声で継続する点で、通常の無声摩擦音と音響的に同じ性質のため)
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
                | Self::ClS
        )
    }

    /// 有声子音であるか判定します (撥音 Nn 系を含みません)
    ///
    /// `ClV` (有声閉鎖) は、独立した子音としての構えを持たない促音の異音で
    /// あるため `is_sokuon` ではなくこれに含めています。
    /// (閉鎖中も声帯が振動し続ける点で、通常の有声子音と似た性質のため)
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

    /// 閉鎖区間 (促音の無声閉鎖・声門閉鎖、ポーズ、スペース) であるかを判定します。
    ///
    /// 促音の異音のうち `ClP`/`ClT`/`ClK`/`ClQ` は声帯振動も摩擦音響も伴わない
    /// 無音区間であるため、ここに分類します。一方 `ClS` (摩擦継続) と
    /// `ClV` (有声閉鎖) は声帯振動を伴うため無音ではなく、それぞれ
    /// `is_unvoiced_consonant` / `is_voiced_consonant` 側で扱います。
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
    /// `ClQ` が表すのは「直前母音の末尾に生じる連続的な音質変化
    /// ではなく、離散的な無音区間で声門破裂 [ʔ] です。
    /// 高速度ビデオによる声門観測 (Fujimoto, Maekawa, and Funatsu, 2010) では、
    /// 後続に阻害音がある通常の語中促音の発音中に声門の緊縮は見られなかった
    /// ことが報告されています。したがって「ッ」という同一の表記であっても、
    /// 語中 (口腔閉鎖) と語末 (声門閉鎖) では全く異なる発声器官のジェスチャー
    /// が用いられており、これを明示的に分けることには強い音声学的妥当性が
    /// あります。
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
    /// ## `v` の分類について
    ///
    /// 唯一の有声摩擦音である `v` は `ClS` (摩擦継続) に分類しています。
    /// 促音が「無音区間になるか、音響エネルギーが継続するか」を決めるのは
    /// 声帯振動の有無ではなく、閉鎖か摩擦かで判断するべきだとしているからです。
    /// (`ClV` という名称が示す「閉鎖」を、そもそも
    /// 口腔閉鎖を持たない `v` に当てるのは整合しない)
    /// なお `z` は同じ有声阻害音ですが、促音直後では破擦音 [dz] として実現
    /// される傾向が強い (例: グッズ [guddzu]) ため、閉鎖を持つ阻害音として
    /// `ClV` に残しています。
    ///
    /// "ッヴ" 自体は実例をほぼ確認できないほど稀な組み合わせであり、
    /// 一貫性を優先したものであって実証的な裏付けはありません。
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
    /// - 両唇音 (p, b, m) の前: `Nm` [m]
    /// - 軟口蓋音 (k, g) の前: `Ng` [ŋ]
    /// - 歯茎音 (t, d, ts, n, z) の前: `Nd` [n]
    /// - 発話境界 (後続音素なし、pau, sp) の前: `Nq` [ɴ]
    /// - 流音 r の前: デフォルトで `Nd` に統合 (`split_n_before_r` で `Nr` に分離可能)
    /// - 破擦音 ch, j の前: デフォルトで `Nd` に統合
    ///   (`split_n_before_palatal_affricate` で `Npl` に分離可能)
    /// - 母音・無声化母音・半母音 (y, w) ・無声摩擦音 (s, sh, h, f) ・有声摩擦音
    ///   (v) の前: 解決せず `Nn` のまま
    ///   (多数派が口腔閉鎖を伴わない「鼻音化母音」になるため)
    ///
    /// なお `s`/`h`/`j`/`w` の前は、実測上は少数のトークンが実際の鼻音閉鎖を
    /// 伴うことが報告されていますが (Maekawa 2021)、多数派は鼻音化母音である
    /// ため、本メソッドでは一律「解決せず `Nn` のまま」をデフォルト挙動として
    /// います。
    ///
    /// ## 語末 `Nq` の細分化を提供しない理由
    ///
    /// 語末の `Nq` を直前母音の前後性によって [ŋ]/[ɴ] に離散的に二値分岐させる
    /// オプションは、意図的に提供していません。Maekawa (2021, JIPA 53(1):
    /// 189-212) のリアルタイムMRI観測により、この変動は実際には連続的かつ
    /// 話者間変動が極めて大きい現象であることが示されており、離散的な二値
    /// ルールで近似すると、実態にない判断をデータへ埋め込むことになるためです。
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
    /// 両唇音 (p, b, m) の前 -> 両唇鼻音 [m] に同化
    Bilabial,

    /// 軟口蓋音 (k, g) の前 -> 軟口蓋鼻音 [ŋ] に同化
    Velar,

    /// 歯茎音 (t, d, ts, n, z) の前 -> 歯茎鼻音 [n] に同化
    ///
    /// 「Nd のデフォルト確定環境」です。rtMRI実測 (Maekawa 2021) で明確に
    /// 分離されたクラスタであり、比較的信頼性は高いと判断しています。
    Alveolar,

    /// 破擦音 ch, j の前 -> 伝統的には硬口蓋鼻音 [ɲ] と記述されることが多い環境
    ///
    /// 直接的な調音観測 (rtMRI) では Alveolar 群と未分離であるため、
    /// デフォルトでは Nd に統合します。
    PalatalAffricate,

    /// 流音 r の前 -> 伝統的には後部歯茎鼻音 [n̠] とやや異なる構えとして
    /// 記述されることがある環境
    ///
    /// PalatalAffricate と同様、rtMRI実測では Alveolar 群と未分離であるため、
    /// デフォルトでは Nd に統合します。
    Liquid,

    /// 発話境界 (後続音素なし、Pau, Sp) -> 口蓋垂鼻音 [ɴ] (語末専用) に解決
    UtteranceBoundary,

    /// 上記のいずれにも該当しない環境
    ///
    /// 母音・無声化母音・半母音 (y, w) ・無声摩擦音 (s, sh, f, h) ・有声摩擦音
    /// (v) ・促音系 (cl とその異音) ・撥音系自身・不明音 (unk) が該当します。
    /// これらの前では多くの場合口腔閉鎖を伴わない「鼻音化母音」として現れるため、
    /// 解決せず `Nn` のまま残します。
    ///
    /// # s, f の扱いについての既知の留保
    ///
    /// Maekawa (2019, ICPhS) のrtMRI計測では [s], [ɸ] (本ライブラリの `S`, `F`)
    /// も閉鎖を伴うクラスタに分類されていますが、この分析は閉鎖を持つトークン
    /// に限定した上での閉鎖位置の分析であり (186サンプル中25サンプルが鼻音化
    /// 母音として解析対象から除外されている)、s/fの前で閉鎖がそもそもどの程度
    /// の頻度で生じるかは論文から分かりません。
    /// しかし、r/chの分離オプションとは異なり、これを分岐させると
    /// 「閉鎖が起きない方が多い可能性がある環境」に閉鎖を強制することになるため、
    /// オプションとしても提供していません。
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
    fn test_phoneme_exhaustiveness() {
        let all_phonemes = Phoneme::ALL;

        for p in all_phonemes {
            // Sp, Pau を除くすべての音素は「有声音」「無声音」「閉鎖区間」「無音・特殊記号」のいずれか1つに必ず属するべき
            let is_voiced = p.is_voiced();
            let is_unvoiced = p.is_unvoiced();
            let is_silent = p.is_silent();
            let is_special = p.is_special();

            let true_count = [is_voiced, is_unvoiced, is_silent, is_special]
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
