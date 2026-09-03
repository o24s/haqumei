<div align="center">
  <h1>Haqumei 🌅</h1>
  <p>
    Haqumeiは、Rustで実装された日本語の Grapheme-to-Phoneme (G2P) ライブラリです。
  </p>
  <p>
    <a href="https://github.com/o24s/haqumei/">English</a> | 日本語
  </p>
  <p>
    <a href="https://crates.io/crates/haqumei">
      <img src="https://img.shields.io/crates/v/haqumei.svg" alt="Crates.io">
    </a>
    <a href="https://docs.rs/haqumei">
      <img src="https://docs.rs/haqumei/badge.svg" alt="docs.rs">
    </a>
    <a href="LICENSE">
      <img src="https://img.shields.io/badge/License-Apache--2.0-blue.svg" alt="License: Apache-2.0">
    </a>
  </p>
  <p>
    <a href="https://pypi.org/project/haqumei/">
      <img src="https://img.shields.io/pypi/v/haqumei.svg" alt="PyPI version">
    </a>
    <a href="https://pypi.org/project/haqumei/">
      <img src="https://img.shields.io/badge/python-%3E%3D%203.9-blue" alt="Python version">
    </a>
    <a href="https://github.com/o24s/haqumei/actions/workflows/rust.yml">
      <img src="https://github.com/o24s/haqumei/actions/workflows/rust.yml/badge.svg" alt="Push CI">
    </a>
    <a href="https://github.com/o24s/haqumei/actions/workflows/pypi.yml">
      <img src="https://github.com/o24s/haqumei/actions/workflows/pypi.yml/badge.svg" alt="PyPI CI">
    </a>
  </p>
</div>

## 目次

- [特徴 (Features)](#特徴-features)
- [インストール](#インストール)
  - [Rust](#rust)
  - [Python](#python)
    - [対応プラットフォーム](#対応プラットフォーム)
- [CLI](#cli)
- [使い方 (Usage)](#使い方-usage)
  - [Rust](#rust-1)
  - [Python](#python-1)
- [Advanced Features](#advanced-features)
  - [Word-Phoneme Mapping APIs について](#word-phoneme-mapping-apis-について)
  - [読みの候補を得る (`g2p_candidates`)](#読みの候補を得る-g2p_candidates)
  - [G2P オプションで出力を変更する](#g2p-オプションで出力を変更する)
- [プロソディ機能 (`g2p_prosody` / `g2p_mapping_prosody`)](#プロソディ機能-g2p_prosody--g2p_mapping_prosody)
  - [`g2p_prosody_with_options` の仕様](#g2p_prosody_with_options-の仕様)
  - [`g2p_mapping_prosody` の仕様](#g2p_mapping_prosody-の仕様)
- [精度](#精度)
  - [再現方法](#再現方法)
  - [jsut-label](#jsut-label)
  - [ROHAN](#rohan)
- [ベンチマーク](#ベンチマーク)
  - [注意点](#注意点)
  - [各機能の性能特性](#各機能の性能特性)
- [カスタム辞書の埋め込みビルド](#カスタム辞書の埋め込みビルド)
  - [Cargo の Feature を変更する](#cargo-の-feature-を変更する)
  - [辞書ソースの準備と環境変数の設定](#辞書ソースの準備と環境変数の設定)
- [辞書](#辞書)
- [ライセンス](#ライセンス)
  - [同梱ソフトウェアのライセンスと由来](#同梱ソフトウェアのライセンスと由来)
- [謝辞](#謝辞)

## 特徴 (Features)

| | |
| :--- | :--- |
| **Word-Phoneme Mapping APIs** | 従来は直接取得が難しかった、単語 ($\approx$ 表層形・辞書エントリ) と音素のマッピング情報を提供します。入力テキストに対して情報のロスが少なく、未知語情報を含む詳細な解析結果を取得可能です。 ([Advanced Features](#advanced-features)) |
| **プロソディ情報の取得** | プロソディ記号付き音素列と、構造化されたプロソディー情報をもつ単語と音素列マッピング (`g2p_prosody`, `g2p_mapping_prosody`) を得ることができます。 (それらの詳細については、[ここ](#プロソディ機能-g2p_prosody--g2p_mapping_prosody) を参照してください。) |
| **より詳細な音素ラベル** | 撥音・促音に対する条件異音 (allophone) 解決によって、専用の音素ラベルとして導入された異音の取得をいくつかの選択肢から設定できます。 (詳細は、[ここ](https://docs.rs/haqumei/latest/haqumei/phoneme/index.html) を参照してください。) |
| **パフォーマンス** | Rustによるネイティブ実装によって高速な処理を実現しています。([ベンチマーク](#ベンチマーク)) |
| **精度** | 辞書とロジックの改善を重ね、[jsut-label](https://github.com/prj-beatrice/jsut-label) で PER 0.87% と、 [ROHAN](https://github.com/mmorise/rohan4600) で CER 0.81% を達成しています。 [`pyopenjtalk-plus`](https://github.com/tsukumijima/pyopenjtalk-plus) の辞書と精度改善手法に基づいてさらなる変更を加えています。 ([精度](#精度)) |
| **未知語フォールバック** | 通常は未知語となってしまう英単語の `haqumei-kanalizer` による読み推定や、辞書にマッチしなかった漢字を音読みするフォールバック、カタカナによって構成される単語のアクセント補正が実装されています。 |
| **並行処理** | `*_batch` 系のメソッドを使うことで、複数のスレッドでG2Pが行えます。 |
| **多様なオプション** | [HaqumeiOptions](https://docs.rs/haqumei/latest/haqumei/options/struct.HaqumeiOptions.html) を用いることで、条件異音の音素ラベル導入、Unicode 正規化、読み方についての柔軟な変更が可能です。 |

コード例は [haqumei/examples](https://github.com/o24s/haqumei/tree/main/haqumei/examples) にあります。

## インストール

### Rust

`haqumei` の初回ビルド時には、crates.io のファイルサイズ制限によって、辞書をダウンロードしてからバイナリに埋め込みます。
自前で用意した辞書や、ネットワークがビルド時に利用できない環境については、[ここ](#カスタム辞書の埋め込みビルド) を参照してください。

```bash
cargo add haqumei
```

### Python

```bash
pip install haqumei
```

#### 対応プラットフォーム

以下のプラットフォーム向けに、ビルド済みの wheel を提供しています：

| OS | アーキテクチャ |
|---|---|
| **Linux** | `x86_64`, `aarch64` |
| **macOS** | `aarch64` (Apple Silicon M1/M2/M3 など) |
| **Windows** | `x86_64` |

事前ビルドされた wheel には組み込み辞書が含まれており、インストール時にネットワークアクセスを必要としません。

プラットフォームに対応した wheel が利用できない場合、インストールはソースからのビルドにフォールバックします。
この場合、Rustツールチェーンが必要であり、ビルドプロセス中に辞書がダウンロードされて組み込まれます。

## CLI

ターミナルから手軽にテキスト処理を行える `haqumei-cli` も提供しています。
詳しい使い方 (パイプ処理や JSON 出力など) については、[`haqumei-cli/README.ja.md`](https://github.com/o24s/haqumei/tree/main/haqumei-cli/README.ja.md) を参照してください。

```bash
cargo install haqumei-cli
```

## 使い方 (Usage)

### Rust

```rust
use haqumei::Haqumei;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut haqumei = Haqumei::new()?;

  let text = "こんにちは、世界！";

  // 音素リストに変換
  let phonemes = haqumei.g2p(text)?;
  assert_eq!(
      phonemes,
      [
          "k", "o", "N", "n", "i", "ch", "i", "w", "a", "pau", "s", "e", "k", "a", "i"
      ]
  );

  // プロソディ記号付きの音素リストを得る
  let phones = haqumei.g2p_prosody(text)?.join(" ");
  assert_eq!(phones, "^ k o [ N n i ch i w a _ s e ] k a i ! $");

  // カタカナ読みに変換
  let kana = haqumei.g2k(text)?;
  assert_eq!(kana, "コンニチワ、セカイ！");

  // 異音解決を有効にする
  haqumei.options.use_allophones = true;

  let text = "執筆";

  // プロソディ情報付きの Word-Phoneme 対応を得る
  let mapping = haqumei.g2p_mapping_prosody(text)?;
  let shippitsu = &mapping[0];
  assert_eq!(shippitsu.word, "執筆");
  assert_eq!(shippitsu.pos, "名詞");
  assert_eq!(shippitsu.accent_nucleus, 0); // 平板型

  println!("{:?}", shippitsu.phonemes);
  // 出力:
  // [Phoneme {
  //     phoneme: Sh,
  //     pitch: Some(Low)
  // },
  // Phoneme {
  //     phoneme: I,
  //     pitch: Some(Low)
  // },
  // Phoneme {
  //     phoneme: ClP, // 促音 /cl/ (Phoneme::Cl) の異音, 無声両唇閉鎖
  //     pitch: Some(High)
  // },
  // Phoneme {
  //     phoneme: P,
  //     pitch: Some(High)
  // },
  // Phoneme {
  //     phoneme: UnvoicedI,
  //     pitch: Some(High)
  // }, ...]

  Ok(())
}
```

> [!IMPORTANT]
> 無声化母音・音素ラベルとして導入された条件異音には声帯振動を伴わない、すなわちピッチが存在しないと考えられるケースについてもピッチを削除していません。
> これは、G2Pライブラリとして情報を恣意的に減らすことを避け、ピッチを削除するかどうかの判断をユーザー側の選択に委ねるべきだと考えているためです。 (ピッチを維持したまま、有声母音に変更する手段を潰すべきではない)
>
> `use_allophones` 以外のオプションや、より詳しい情報については[ドキュメント](https://docs.rs/haqumei/latest/haqumei/phoneme/index.html)を参照してください。

### Python

```python
from haqumei import Haqumei

# Haqumeiを初期化 (辞書は自動でセットアップされます)
haqumei = Haqumei()

text = "こんにちは、世界！"

# 音素列に変換
phonemes = haqumei.g2p(text)
print(f"音素列: {phonemes}")
# -> 音素列: ["k", "o", "N", "n", "i", "ch", "i", "w", "a", "pau", "s", "e", "k", "a", "i"]

# pyopenjtalk風のスペース区切り文字列に変換
phones = " ".join(haqumei.g2p_prosody(text))
print(f"プロソディ付き音素列: {phones}")
# -> プロソディ付き音素列: ^ k o [ N n i ch i w a _ s e ] k a i ! $

# カタカナ読みに変換
kana = haqumei.g2k(text)
print(f"カタカナ読み: {kana}")
# -> カタカナ読み: コンニチワ、セカイ！
```

## Advanced Features

### Word-Phoneme Mapping APIs について

Open JTalk (pyopenjtalk) では、未知語は `pau` として扱われますが、`Haqumei` の `g2p` 関数もそれに則っています。  
しかし、`mapping`, `detailed` あるいは `prosody` の名前を含む G2P 関数を使うことで、未知語や空白そのものを `unk`, `sp` として検出可能です。  

> [!WARNING]
> `sp` は、入力された空白ではなく、Mecab が出力した、本来 `pyopenjtalk` で無視される`"記号,空白"`であることに注意してください。特に、Mecab がそもそも無視する記号 (e.g., `\t`, `\n`) などは `sp` に含まれません。  
> Word-Phoneme Mapping APIs について、"入力テキストに対してロスの少ない" と表現しているのはそのためで、入力テキストと完全な一致が保証されるわけではありません。(英字も Open JTalk によって全角化される)
>
> "単語($\approx$ 表層形・辞書エントリ)と音素をマッピングする" という表現についても補足します。
> まず、そもそも日本語に「単語」の明確な共通の定義は存在せず、日本語形態素解析の文脈においては、
> 辞書の表層形を「単語」だとみなし、入力文字列を解析することで文法機能を同定していると[されて](https://clrd.ninjal.ac.jp/unidic/glossary.html#morphological_analysis)います。
> Open JTalk は様々な処理の過程で、表層形や文法、アクセント情報を伴う `NjdFeature` のマージが発生し、
> (Haqumei では[拡張されている](https://github.com/o24s/haqumei/tree/main/haqumei-jlabel)) HTS形式のフルコンテキストラベルではこれを抽象的な [Word](https://docs.rs/haqumei-jlabel/latest/haqumei_jlabel/struct.Word.html) として扱っています。
> そのため、入力テキストの部分文字列を表現するには、マージ処理のために明らかに表層形は誤りで、とはいえ処理のしやすい分割された形式として、あえて定義が曖昧な Word という表現を用いています。

- 既知語: 通常の音素列 (読点などは `pau`)
- 未知語: `unk`
- 空白等: `sp` (Space)

`g2p_mapping` を使用すると、未知語かどうか (`is_unknown`)、本来のパイプラインで無視されるかどうか (`is_ignored`) という情報とともに、音素と元の単語のマッピングが取得できます。また、`g2p_mapping_detailed` を使うことで、マッピングに加えて品詞やアクセント情報などを取得することもできます。

プロソディ情報付きの単語と音素を得るには、`g2p_mapping_prosody` が有用です。
詳しくは [ここ](#g2p_mapping_prosody-の仕様) を読んでください。  
とはいえ、`g2p_mapping_prosody` がリストとして返す [`WordPhonemeProsody`](https://docs.rs/haqumei/latest/haqumei/word_phoneme/struct.WordPhonemeProsody.html) は、 `g2p_mapping_detailed` の返却する [`WordPhonemeDetail`](https://docs.rs/haqumei/latest/haqumei/word_phoneme/struct.WordPhonemeDetail.html) のスーパーセット的な実装になっている (Mecab の features を除けば) 点は留意してください。

以上より、この API で得られる情報の大きさを簡単に示すと、  
`g2p_mapping` < `g2p_mapping_detailed` < `g2p_mapping_prosody`  
のようになると言えます。


```rust
use haqumei::Haqumei;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut haqumei = Haqumei::new()?;

  println!("{:?}", haqumei.g2p_mapping("𰻞𰻞麺 お冷を頼んだ")?);
  // [WordPhonemeMap {
  //     word: "𰻞𰻞",
  //     phonemes: ["unk"],
  //     is_unknown: true,
  //     is_ignored: false,
  //     char_span: 0..2,
  // },
  // WordPhonemeMap {
  //     word: "麺",
  //     phonemes: ["m", "e", "N"],
  //     is_unknown: false,
  //     is_ignored: false,
  //     char_span: 2..3,
  // },
  // WordPhonemeMap {
  //     word: "\u{3000}",
  //     phonemes: ["sp"],
  //     is_unknown: false,
  //     is_ignored: true,
  //     char_span: 3..4,
  // },
  // WordPhonemeMap {
  //     word: "お冷",
  //     phonemes: ["o", "h", "i", "y", "a"],
  //     is_unknown: false,
  //     is_ignored: false,
  //     char_span: 4..6,
  // }, ... ]

  println!("{:?}", haqumei.g2p_mapping_detailed("薄明")?);
  // [WordPhonemeDetail {
  //    word: "薄明",
  //    phonemes: ["h","a","k","u","m","e","e"],
  //    features: [
  //        "薄明",
  //        "名詞",
  //        "一般",
  //        "*",
  //        "*",
  //        "*",
  //        "*",
  //        "薄明",
  //        "ハクメイ",
  //        "ハクメー",
  //        "0/4",
  //        "C2",
  //    ],
  //    pos: "名詞",
  //    pos_group1: "一般",
  //    pos_group2: "*",
  //    pos_group3: "*",
  //    ctype: "*",
  //    cform: "*",
  //    orig: "薄明",
  //    read: "ハクメイ",
  //    pron: "ハクメー",
  //    accent_nucleus: 0,
  //    mora_count: 4,
  //    chain_rule: "C2",
  //    chain_flag: -1,
  //    is_unknown: false,
  //    is_ignored: false,
  //    char_span: 0..2,
  // }]

  Ok(())
}
```

### 読みの候補を得る (`g2p_candidates`)

`g2p_mapping` は読みを 1 つに決めて返しますが、Forced Alignment のように、読みが分かれる
箇所を音響モデルに決めさせたいときは、`g2p_candidates` で読みの候補を出力できます。

候補になるのは、辞書が持っている読みだけです。同じ表層形に発音の違うエントリが複数
あるところが分岐点で、分岐点ごとに 1 つ選んで解析し直します。どの候補も形態素列を
1 つに決めてから解析するので、返る候補の中身は `g2p_mapping` の返り値と同じ形です。

`Candidates::candidates` の先頭の要素は `g2p_mapping` の出力と一致します。

分割の違う読みも候補になります。「彼の」は `彼` + `の` (カレノ) と、連体詞 `彼の`
(アノ) の 2 通りに分かれます。

```rust
use haqumei::Haqumei;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut haqumei = Haqumei::new()?;
  let got = haqumei.g2p_candidates("彼の話を聞いた。")?;

  for branch in &got.branches {
    println!(
      "{:?} {} {:?}",
      branch.char_span, branch.surface,
      branch.alternatives.iter().map(|a| (a.pron(), a.nodes.len())).collect::<Vec<_>>()
    );
  }
  // 0..2 彼の [("カレノ", 2), ("アノ", 1)]

  for cand in &got.candidates {
    println!(
      "{} {:?}",
      cand.delta,
      cand.words.iter().flat_map(|w| w.phonemes.iter()).collect::<Vec<_>>()
    );
  }
  // 0    [k, a, r, e, n, o, h, a, n, a, sh, i, o, k, i, i, t, a, pau]
  // 1529 [a, n, o, h, a, n, a, sh, i, o, k, i, i, t, a, pau]

  Ok(())
}
```

候補の `words` は `g2p_candidates` なら `WordPhonemeMap`、`g2p_candidates_detailed`
なら `WordPhonemeDetail`、`g2p_candidates_prosody` なら `WordPhonemeProsody` です。
コストの閾値と候補の数は `CandidateOptions` で変えられます。

`Candidates::candidates` を並べて FST を組むと、`max_candidates` に達して作らなかった
組み合わせが欠けることになります。`Candidates::branches` は上限に影響されないので、
すべて残したいなら `branches` から直積を組めます。候補との突き合わせには `char_span` を
使えます。

候補にならないものは以下の 4 つです。

- 辞書にエントリが 1 つしかない語: ほかの読みがありうる場合でも 1 つしか返りません
- 未知語のノード: `CandidateReading::pron` が `*` なので、既定でラティスから外しています
  (`branch_on_unknown_words`)
- 読みを決める補正が書き込む箇所: 「何」の予測、文脈読みの決定リスト、数字の
  `njd_set_digit` が、ラティスの選んだ読みを上書きします
- 音素列が同じになった候補: コスト差の小さい方だけ残ります

`０` と `何` はラティスが分岐していても候補が増えません。

値の小さい組み合わせから順に `max_candidates` 通りを組み立てるので、
`Candidates::candidates` は `Candidate::delta` の昇順に並びます。MeCab のコストは分割と
品詞を決めるための値で、読みの確からしさを測ったものではないため、FST のアークの重みには
使えません。

### G2P オプションで出力を変更する

`Haqumei::with_options` を使用することで、`Haqumei` の出力をカスタマイズできます。
デフォルトの動作やオプションの詳細については、[HaqumeiOptions](https://docs.rs/haqumei/latest/haqumei/struct.HaqumeiOptions.html) を参照してください。

このケースでは、デフォルトでは無効になっている `normalize_unicode` を有効にし、入力テキストに Unicode の NFC正規化 を適用しています。

```rust
use haqumei::{Haqumei, HaqumeiOptions, UnicodeNormalization};

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut haqumei = Haqumei::with_options(HaqumeiOptions {
    normalize_unicode: UnicodeNormalization::Nfc,
    ..Default::default()
  })?;

  let text = &[
    "\u{304B}\u{3099}", // か + ゙ (が)
    "\u{306F}\u{309A}", // は + ゚ (ぱ)
    "\u{30B3}\u{3099}", // コ + ゙ (ゴ)
  ];

  println!("{:?}", haqumei.g2p_detailed_batch(text)?);
  // 出力: [["g", "a"], ["p", "a"], ["g", "o"]]

  Ok(())
}
```

## プロソディ機能 (`g2p_prosody` / `g2p_mapping_prosody`)

### `g2p_prosody_with_options` の仕様

入力テキストを `ProsodyFormat` の設定をもとにプロソディ記号付き音素リストに変換します。
(`g2p_prosody` メソッドは、`ProsodyFormat::Default` を指定したときの動作と同じです。)

出力には、共通して以下のプロソディ記号が含まれます。

| 記号 | 意味 | 出現位置 |
| :--- | :--- | :--- |
| `^` | 発話の開始 (BOS) | 文頭 |
| `$` | 発話の終結 (EOS) | 文末 |
| `?` | 疑問文の終結 (？) | 文中 |
| `!` | 感嘆の終結 (独自拡張) | 文中 |
| `_` | ポーズ・読点 (、) | 文中 |
| `#` | アクセント句境界 | 文中 |
| `{...}` | 未知語 | 文中 |

日本語のアクセントについて: [tdmelodic 利用マニュアル/予備知識](https://tdmelodic.readthedocs.io/ja/latest/pages/introduction.html)

#### ProsodyFormat::Default

出力には上記のものに追加して、以下のプロソディ記号が含まれます。

| 記号 | 意味 | 出現位置 |
| :--- | :--- | :--- |
| `[` | ピッチ上昇 (句頭) | 句の開始付近 |
| `]` | ピッチ下降 (アクセント核) | 核モーラの直後 |

記号 `[` および `]` は、tdmelodic 等で一般的なアクセント記法に基づいています。
"Prosodic Features Control by Symbols as Input of Sequence-to-Sequence Acoustic Modeling for Neural TTS"
(Kurihara et al., 2021) のアルゴリズムにおける `^` および `!` に相当します。

#### ProsodyFormat::Prefix

ピッチ上昇/下降記号 (`[` や `]`) を使用せず、各音素のプレフィックスとしてピッチの高低を付与します。
 - `H_` : ピッチが高い (High)
 - `L_` : ピッチが低い (Low)

音素ごとにピッチが明示されます。
例: `"青い空"` -> `["^", "L_a", "H_o", "L_i", "#", "H_s", "H_o", "L_r", "L_a", "$"]`

#### ProsodyFormat::Numeric

各音素のサフィックスとして、ピッチの高低を数値で付与します。
 - `:1` : ピッチが高い (High)
 - `:0` : ピッチが低い (Low)

例: `"青い空"` -> `["^", "a:0", "o:1", "i:0", "#", "s:1", "o:1", "r:0", "a:0", "$"]`

#### 例

```rust
use haqumei::Haqumei;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut haqumei = Haqumei::new()?;

  let phones = haqumei.g2p_prosody("こんにちは、世界！")?;
  assert_eq!(phones.join(" "), "^ k o [ N n i ch i w a _ s e ] k a i ! $");

  let phones = haqumei.g2p_prosody("青い空、広がる。")?;
  assert_eq!(phones.join(" "), "^ a [ o ] i # s o ] r a _ h i [ r o g a r u _ $");

  Ok(())
}
```

### `g2p_mapping_prosody` の仕様

一方で、`g2p_mapping_prosody` は入力テキストを解析し、形態素 (単語) ごとの詳細な言語情報と、プロソディ (韻律) 記号付き音素をマッピングして取得します。

[`Haqumei::g2p_prosody`] や [`Haqumei::g2p_prosody_with_options`] がフラットな文字列リスト (`Vec<String>`) を返すのに対し、
この関数は品詞、アクセント型、読み、およびピッチ情報が付与された構造化データ (`Vec<WordPhonemeProsody>`) を返します。

音声合成のフロントエンド処理において、形態素と音素の対応関係を維持したい場合や、ピッチの高低 ([`PitchAccent`]) を個別に取得・操作したい場合、あるいは未知語のハンドリングを行いたい場合に適しています。

#### `WordPhonemeProsody` に含まれる主な情報

形態素ごとのデータとして、以下の情報が含まれます。

| フィールド | 説明 | 例 |
| :--- | :--- | :--- |
| `word` | 単語、入力の部分文字列 | `"空"` |
| `phonemes` | 音素とピッチ情報やプロソディ記号からなるリスト (後述) | `[ProsodicPhoneme::Exclamatory]` |
| `pos`, `pos_group1`~`3` | 品詞およびその細分類 | `"名詞"`, `"一般"` |
| `orig`, `read`, `pron` | 原形、読み、発音形式 | `"空"`, `"ソラ"`, `"ソラ"` |
| `accent_nucleus` | アクセント核位置 (0: 平板型, 1~: n番目のモーラ) | `1` |
| `mora_count` | モーラ数 | `2` |
| `is_unknown` | MeCabによって未知語判定されたかどうか | `false` |
| `is_ignored` | 音素が割り当てられなかったか | `false` |

#### プロソディ音素 (`ProsodicPhoneme`)

`phonemes` フィールドには、以下の要素からなるリストが格納されます。

| 列挙子 | 意味 | `g2p_prosody` 等での出力記号 |
| :--- | :--- | :--- |
| `Phoneme` | [音素](https://docs.rs/haqumei/latest/haqumei/phoneme/enum.Phoneme.html)と、そのピッチの高低 (`High` / `Low`) | `a`, `a:0`, `H_a` など |
| `AccentPhraseBoundary` | アクセント句境界 | `#` |
| `Pause` | 通常のポーズ・読点 | `_` |
| `Interrogative` | 疑問文の終結・ポーズ | `?` |
| `Exclamatory` | 感嘆の終結・ポーズ | `!` |

#### 例

```rust
use haqumei::{Haqumei, PitchAccent, ProsodicPhoneme};

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut haqumei = Haqumei::new()?;

  // テキストを形態素ごとの構造化データとして取得
  let mapping = haqumei.g2p_mapping_prosody("青い空が、好きだ！")?;

  // 「青い」の形態素情報
  let aoi = &mapping[0];
  assert_eq!(aoi.word, "青い");
  assert_eq!(aoi.pos, "形容詞");
  assert_eq!(aoi.read, "アオイ");
  assert_eq!(aoi.accent_nucleus, 2); // 中高型

  // 「青い」の音素とピッチ情報 (a: Low, o: High, i: Low)
  assert!(matches!(
      aoi.phonemes[0],
      ProsodicPhoneme::Phoneme { pitch: Some(PitchAccent::Low), .. }
  ));

  let da = mapping.last().unwrap();
  assert_eq!(da.word, "！");
  assert!(da.phonemes.contains(&ProsodicPhoneme::Exclamatory));

  Ok(())
}
```

## 精度

[japanese-g2p-benchmark](https://github.com/o24s/japanese-g2p-benchmark) による計測です。

JSUT corpus の basic5000 に対するアノテーションである、jsut-label のフォーク [prj-beatrice/jsut-label](https://github.com/prj-beatrice/jsut-label) の音素エラー率 (PER) と、[ROHAN](https://github.com/mmorise/rohan4600) のカタカナエラー率 (Katakana Error Rate, KER) を示します。

| G2P | jsut-label (PER) | ROHAN (KER) |
| :--- | ---: | ---: |
| pyopenjtalk 0.4.1 | 1.31% | 5.02% * |
| pyopenjtalk-plus 0.4.1.post9 | 1.09% | 1.60% |
| **haqumei 0.9.0** | **0.87%** | **0.81%** |

\* 素の `pyopenjtalk` には長音と四つ仮名を元の表記のまま書き出す手段が無いため、
出力を ROHAN の表記に揃えられません。この差の大半は読みの誤りではなく表記の違いです。

オプションは、それぞれの G2P がもつオプションから、そのコーパスの註釈方針に合うものを選んでいます。  
すべてのオプションによる総当たり精度は、以下の「全オプションの結果」を参照してください。

<details>
<summary>全オプションの結果</summary>

| G2P | options | jsut-label (PER) | ROHAN (KER) |
| :--- | :--- | ---: | ---: |
| pyopenjtalk | - | 1.31%\* | 5.02% |
| pyopenjtalk_plus | use_sudachi_kanji_yomi=True, use_tsqyomi=True, revert_long_vowels=True, revert_yotsugana=True | - | 1.60% |
| pyopenjtalk_plus | use_sudachi_kanji_yomi=True, use_tsqyomi=True, revert_long_vowels=False, revert_yotsugana=False | 1.10%\* | 4.63% |
| pyopenjtalk_plus | use_sudachi_kanji_yomi=False, use_tsqyomi=True, revert_long_vowels=True, revert_yotsugana=True | - | 1.60% |
| pyopenjtalk_plus | use_sudachi_kanji_yomi=False, use_tsqyomi=True, revert_long_vowels=False, revert_yotsugana=False | 1.10%\* | 4.63% |
| pyopenjtalk_plus | use_sudachi_kanji_yomi=True, use_tsqyomi=False, revert_long_vowels=True, revert_yotsugana=True | - | 1.62% |
| pyopenjtalk_plus | use_sudachi_kanji_yomi=True, use_tsqyomi=False, revert_long_vowels=False, revert_yotsugana=False | 1.09%\* | 4.65% |
| pyopenjtalk_plus | use_sudachi_kanji_yomi=False, use_tsqyomi=False, revert_long_vowels=True, revert_yotsugana=True | - | 1.64% |
| pyopenjtalk_plus | use_sudachi_kanji_yomi=False, use_tsqyomi=False, revert_long_vowels=False, revert_yotsugana=False | 1.11%\* | 4.66% |
| haqumei | normalize_iu=none, revert_long_vowels=True, revert_yotsugana=True | - | 0.81% |
| haqumei | normalize_iu=none, revert_long_vowels=False, revert_yotsugana=False | 1.00%\* | 3.82% |
| haqumei | normalize_iu=yuu, revert_long_vowels=True, revert_yotsugana=True | - | 0.87% |
| haqumei | normalize_iu=yuu, revert_long_vowels=False, revert_yotsugana=False | 0.96%\* | 3.88% |
| haqumei | normalize_iu=yuu-base, revert_long_vowels=True, revert_yotsugana=True | - | 0.84% |
| haqumei | normalize_iu=yuu-base, revert_long_vowels=False, revert_yotsugana=False | 0.87%\* | 3.85% |

`-` は測っていないことを表します。表記を戻す設定 (`revert_long_vowels` /
`revert_yotsugana`) が意味を持つのは、長音記号を使わない ROHAN だけです。
そのため、有効にした行では jsut-label を走らせていません。

</details>

### 再現方法

```bash
git clone https://github.com/o24s/japanese-g2p-benchmark
cd japanese-g2p-benchmark
uv run init.py
uv run python run_all.py --datasets phoneme,no_lvs --sources jsut-label,rohan4600
```

### jsut-label

Phoneme Error Rate (S+D+I / N_expected): **0.87%** (Substitute=1636, Delete=395, Insert=554, N=297843)

`HaqumeiOptions`:
```rust
HaqumeiOptions {
  normalize_iu: Some(IuPronunciation::YuuBase),
  ..Default::default()
}
```

### ROHAN

Katakana Error Rate (S+D+I / N_expected): **0.81%** (Substitute=824, Delete=154, Insert=246, N=150637)

`HaqumeiOptions`:
```rust
HaqumeiOptions {
  revert_long_vowels: true,
  revert_yotsugana: true,
  ..Default::default()
}
```

## ベンチマーク

約31.8万文字の日本語テキストを対象にした、`pyopenjtalk` (Baseline) と `haqumei` の比較結果です。

入力データ: [「吾輩は猫である」](https://www.aozora.gr.jp/cards/000148/files/789_14547.html) 318,407文字 / 8,451行 (平均 37文字/行) (※ ルビは消去済み)

| 実行モード | 実行時間 (Mean) | スループット | スピードアップ |
| :--- | :--- | :--- | :--- |
| **pyopenjtalk** (Baseline) | 2.358 s | 135k chars/s | 1.00x |
| **haqumei** (Default) | 1.303 s | 244k chars/s | 1.81x |
| **haqumei** (`g2p_batch`, Default) | 0.098 s | 3.24M chars/s | 24.04x |

ベンチマークコードは [`haqumei-bench/pyopenjtalk`](https://github.com/o24s/haqumei/tree/main/haqumei-bench/pyopenjtalk) にあります。

また、[`Criterion.rs`](https://crates.io/crates/criterion) を使用した Haqumei のベンチマークは、`haqumei-bench` クレートで `cargo bench` することで実行できます。
`pyopenjtalk-plus` との比較ベンチマークは、[`haqumei-bench/pyopenjtalk-plus`](https://github.com/o24s/haqumei/tree/main/haqumei-bench/pyopenjtalk-plus) にあります。

### 注意点

- 入力構造によるスループットの変化:  
  特に `*_batch` 系 API において、`pyopenjtalk` と比べ、1行あたりの文字数が多くなるほどスループット (chars/s) が高くなる傾向にあります。(だいたい 4KB ぐらいまでは)  
  これは G2P処理 が Open JTalk 内部の構造体から、直接ラベルを取り出すように実装されていたり、FFI のオーバーヘッドが少ないためであると考えられます。  
  大量の文章を処理する場合は、極端に細かく改行せずにある程度の長さでバッチ処理に渡すのが最も効率的です。

- 表中の Default:  
  `Haqumei::new` をそのまま使用した場合の計測です。

### 各機能の性能特性

#### `predict_nani` 機能

`predict_nani` は "何" の読み推定のために ONNX を用いますが、セッションをOSスレッドごとに作るのは正気ではないため、`Mutex` を使用しています。(ONNX のセッションはスレッドセーフだが、そのバインディングの ort は `Session::run` を[排他参照をとるようにしている](https://github.com/pykeio/ort/issues/402#issuecomment-2949993914))  
並行に処理をしている際に、入力に大量の"何"がくることでボトルネックになってしまうケースはまれであるため、ボトルネックとなる懸念は否定されます。  
また、このモデル自体は軽量で、並行性に耐性のあるキャッシュ機構を挟んでいるため、DOS的な入力への多少の耐性はあるといえます。  

「吾輩は猫である」 (800個近くの"何"を含む) を用いたベンチマーク(`haqumei-bench`)でも、デフォルトの `Haqumei` とその `predict_nani` を有効にした比較は、平均的には非常に小さい誤差に収まったために、実際にはボトルネックではありません。  

## カスタム辞書の埋め込みビルド

`haqumei` はデフォルトで、ビルド時に辞書をダウンロードしてバイナリに埋め込みます。
これにより、crates.io への公開と自己完結したバイナリを両立しています。

もし、自前の辞書をバイナリに埋め込んでビルドしたい場合は、以下の手順で設定を変更できます。

### Cargo の Feature を変更する

デフォルトの `download-dictionary` を無効にし、`build-dictionary` を有効にします。

```toml
[dependencies]
haqumei = { version = "x.y.z", features = ["embed-dictionary", "build-dictionary"], default-features = false }
```

### 辞書ソースの準備と環境変数の設定

ビルド時にコンパイルさせるための辞書ソース (`.csv` や `.def` が含まれたディレクトリ) を用意し、そのパスを環境変数 `HAQUMEI_DICT_SRC` に設定してビルドを実行します。

Unix 系の場合:
```bash
HAQUMEI_DICT_SRC="/path/to/your/dictionary" cargo build --release
```

Windows (PowerShell) の場合:
```powershell
& { $env:HAQUMEI_DICT_SRC="C:\path\to\your\dictionary"; cargo build --release }
```

> **Note:** 環境変数が設定されていない場合は、クレートのルートから相対パスで `dictionary` を参照します。

## 辞書

Haqumeiは [pyopenjtalk-plus](https://github.com/tsukumijima/pyopenjtalk-plus) に含まれる辞書を改変して使用しています。

## ライセンス

`haqumei-jlabel` と `haqumei-kanalizer` を除く Haqumei は、Apache License 2.0 の条件に基づいて配布されています。

`haqumei` のロジックには、[tsukumijima/pyopenjtalk-plus](https://github.com/tsukumijima/pyopenjtalk-plus) を移植実装したものを含んでおり、`tsukumijima/pyopenjtalk-plus` に配置された [r9y9/pyopenjtalk](https://github.com/r9y9/pyopenjtalk) のライセンスを表明するファイルを同様に同梱しますが、`tsukumijima/pyopenjtalk-plus` の上流に対して新たに追加されたコードのライセンスを表明するわけではありません。

### 同梱ソフトウェアのライセンスと由来

`haqumei` には、Grapheme-to-Phoneme (G2P) 機能を提供するために、Open JTalk の改変版に由来する C/C++ ソースコードおよび辞書データが含まれています。これら同梱されているコードの由来およびライセンスは以下の通りです。

- 同梱されている Open JTalk ソースコード
  - 由来: `vendor/open_jtalk` ディレクトリに含まれるコードは、[tsukumijima/open_jtalk](https://github.com/tsukumijima/open_jtalk) リポジトリに基づいています。これは、Open JTalkの拡張版に、さまざまなコミュニティフォーク (VOICEVOXプロジェクトなど) による改善を統合したものです。
  - ライセンス: 同梱されている Open JTalk ソースコードは、修正BSDライセンスの下でライセンスされています。このライセンスは `vendor/open_jtalk` にあるコードにのみ適用され、このプロジェクトの他の部分には適用されません。再配布要件に従い、修正BSDライセンスの全文は `vendor/open_jtalk/src/COPYING` に含まれています。

- バンドルされた辞書データ
  - 由来: `haqumei/dictionary` ディレクトリに含まれる辞書データは、
    [tsukumijima/pyopenjtalk-plus](https://github.com/tsukumijima/pyopenjtalk-plus) をベースとしています。これは
    [r9y9/pyopenjtalk](https://github.com/r9y9/pyopenjtalk) のフォークです。
  - ライセンス: 辞書データの著作権は `haqumei/dictionary/COPYING` ファイルに記載された内容に準拠します。


- バンドルされた漢字読みフォールバックデータ
  - 由来: `haqumei/data/unihan` のデータは、[Unihan Database](https://www.unicode.org/charts/unihan.html) の
    `kJapanese` フィールドから生成しています。辞書に無い漢字がカナ列に表層形のまま
    混入するのを防ぐための、1 文字ごとの読みのフォールバックのために使用しています。
  - ライセンス: UNICODE LICENSE V3。このライセンスは `haqumei/data/unihan` にあるデータにのみ
    適用され、このプロジェクトの他の部分には適用されません。再配布要件に従い、全文は
    `haqumei/data/unihan/LICENSE` に含まれています。

- バンドルされた「何」の読み推定モデル
  - 由来: `haqumei/yomi_model` の ONNX モデルは、
    [n5-suzuki/pyopenjtalk](https://github.com/n5-suzuki/pyopenjtalk) で実装された
    「何」の読み推定ロジックを、
    [tsukumijima/pyopenjtalk-plus](https://github.com/tsukumijima/pyopenjtalk-plus)
    において ONNX に変換されたものです。`include_bytes!` によってバイナリに埋め込まれます。
  - ライセンス: n5-suzuki/pyopenjtalk のリポジトリのルートには、フォーク元である
    [r9y9/pyopenjtalk](https://github.com/r9y9/pyopenjtalk) の MIT ライセンス表記が
    置かれています。一方で、`pyopenjtalk` から新たに追加されたロジックおよび変換されたモデルを対象とした個別の
    ライセンス表明は見つけられませんでした。`haqumei` に同梱される `LICENSE-pyopenjtalk` は、
    このモデルのライセンス表明を行うものではありません。

- バンドルされた `haqumei-jlabel` ソースコード
  - 由来: `haqumei-jlabel` ディレクトリに含まれるコードは、
    [jpreprocess/jlabel](https://github.com/jpreprocess/jlabel) リポジトリをベースとしています。
  - ライセンス: バンドルされている `haqumei-jlabel` ソースコードは BSD 3-Clause License の下でライセンスされています。このライセンスは
    `haqumei-jlabel` ディレクトリ内のコードにのみ適用され、本プロジェクトの他の部分には適用されません。再配布に関する要件に従い、BSD 3-Clause License の全文は
    `haqumei-jlabel/LICENSE` ファイルに含められています。

- バンドルされた `haqumei-kanalizer` クレート
  - 由来: `haqumei-kanalizer` に同梱されている ONNX モデルは
    [VOICEVOX/kanalizer](https://github.com/VOICEVOX/kanalizer) をベースとしており、
    重みは [VOICEVOX/kanalizer-model](https://huggingface.co/VOICEVOX/kanalizer-model)
    のものを [o24s/kanalizer-onnx](https://github.com/o24s/kanalizer-onnx) で変換しています。
  - ライセンス: `haqumei-kanalizer` クレート全体 (Rust コードと同梱の重みの両方) は
    MIT License の下でライセンスされています。

## 謝辞

`haqumei` の基礎的な設計とAPIは、`pyopenjtalk` とその大幅に改善されたフォークである `pyopenjtalk-plus` に触発されています。
また、利便性や精度のために、`jlabel` や `kanalizer` をもとにした実装を行いました。

- pyopenjtalk: Copyright (c) 2018 Ryuichi Yamamoto
- pyopenjtalk-plus: Copyright (c) 2023 tsukumijima
- jlabel: Copyright (c) 2024 JPreprocess Team
- kanalizer: Copyright (c) 2025 VOICEVOX

これらの基礎となるプロジェクトの著者および貢献者の皆様に深く感謝いたします。
