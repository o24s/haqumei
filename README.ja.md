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
  - [G2P オプションで出力を変更する](#g2p-オプションで出力を変更する)
- [プロソディ機能 (`g2p_prosody` / `g2p_mapping_prosody`)](#プロソディ機能-g2p_prosody--g2p_mapping_prosody)
  - [`g2p_prosody_with_options` の仕様](#g2p_prosody_with_options-の仕様)
  - [`g2p_mapping_prosody` の仕様](#g2p_mapping_prosody-の仕様)
- [精度](#精度)
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

- Word-Phoneme Mapping APIs: 従来は直接取得が難しかった、単語 ($\approx$ 表層形・辞書エントリ) と音素のマッピング情報を提供します。入力テキストに対して情報のロスが少なく、未知語情報を含む詳細な解析結果を取得可能です。 ([Advanced Features](#advanced-features))
- プロソディ情報の取得: プロソディ記号付き音素列と、入力テキストに対してロスの少ないマッピング (`g2p_prosody`, `g2p_mapping_prosody`) を得ることができます。 (それらの詳細については、[ここ](#プロソディ機能-g2p_prosody--g2p_mapping_prosody) を参照してください。)
- より詳細な音素ラベル: 撥音・促音に対する条件異音 (allophone) 解決によって、専用の音素ラベルとして導入された異音の取得をいくつかの選択肢から設定できます。 (詳細は、[ここ](https://docs.rs/haqumei/latest/haqumei/phoneme/index.html) を参照してください。)
- パフォーマンス: Rustによるネイティブ実装により、高速な処理を実現しています。([ベンチマーク](#ベンチマーク))
- 精度: `haqumei-kanalizer` による英単語読み推定やその他の補正に加えて、[`pyopenjtalk-plus`](https://github.com/tsukumijima/pyopenjtalk-plus) で実装された多くの手法を取り入れ、精度が改善されています。 ([精度](#精度))
- 並行処理: `*_batch` 系のメソッドを使うことで、複数のスレッドでG2Pが行えます。
- 多様なオプション: [HaqumeiOptions](https://docs.rs/haqumei/latest/haqumei/options/struct.HaqumeiOptions.html) を用いることで、条件異音の音素ラベル導入、Unicode 正規化、読み方についての柔軟な変更が可能です。

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
未知語情報さえ必要ない場合は、`g2p_pairs` のような API もありますが、従来の `g2p` と同様に入力の損失が大きいためにあまり推奨しません。

プロソディ情報付きの単語と音素を得るには、`g2p_mapping_prosody` が有用です。
詳しくは [ここ](#g2p_mapping_prosody-の仕様) を読んでください。  
とはいえ、`g2p_mapping_prosody` がリストとして返す [`WordPhonemeProsody`](https://docs.rs/haqumei/latest/haqumei/word_phoneme/struct.WordPhonemeProsody.html) は、 `g2p_mapping_detailed` の返却する [`WordPhonemeDetail`](https://docs.rs/haqumei/latest/haqumei/word_phoneme/struct.WordPhonemeDetail.html) のスーパーセット的な実装になっている (Mecab の features を除けば) 点は留意してください。

以上より、この API で得られる情報の大きさを簡単に示すと、  
`g2p_pairs` < `g2p_mapping` < `g2p_mapping_detailed` < `g2p_mapping_prosody`  
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
  // },
  // WordPhonemeMap {
  //     word: "麺",
  //     phonemes: ["m", "e", "N"],
  //     is_unknown: false,
  //     is_ignored: false,
  // },
  // WordPhonemeMap {
  //     word: "\u{3000}",
  //     phonemes: ["sp"],
  //     is_unknown: false,
  //     is_ignored: true,
  // },
  // WordPhonemeMap {
  //     word: "お冷",
  //     phonemes: ["o", "h", "i", "y", "a"],
  //     is_unknown: false,
  //     is_ignored: false,
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
  // }]

  Ok(())
}
```

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

`haqumei-eval` クレートを用いた、JSUT corpus の Basic5000 に対するアノテーションである、jsut-label のフォーク [prj-beatrice/jsut-label](https://github.com/prj-beatrice/jsut-label) の音素エラー率(PER)と、[ROHAN](https://github.com/mmorise/rohan4600) のカタカナエラー率(Katakana Error Rate)を示します。

### jsut-label

Phoneme Error Rate (S+D+I / N_expected): **1.17%** (Substitute=2117, Delete=527, Insert=831, N=297843)

`HaqumeiOptions`:
```rust
HaqumeiOptions {
  normalize_iu: Some(IuPronunciation::Yuu),
  ..Default::default()
}
```

### ROHAN

Katakana Error Rate (S+D+I / N_expected): **1.64%** (Substitute=1689, Delete=493, Insert=288, N=150637)

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
  特に `*_batch` 系 API において、`pyopenjtalk` と比べ、1行あたりの文字数が多くなるほどスループット (chars/s) が高くなる傾向にあります (だいたい 4KB ぐらいまでは)。  
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

#### `pyopenjtalk-plus` との比較

前提として、Sudachi や ONNX モデルによる読み補正やその他の改善を取り入れた `pyopenjtalk-plus` は、  
フォーク元の [pyopenjtalk](https://github.com/r9y9/pyopenjtalk) と比べてほぼ同じスループットです。

しかし、`pyopenjtalk-plus` は、ROHAN において Haqumei より精度が少し高く、公平性を欠くためパフォーマンスの比較対象としていません。  

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

Haqumeiは [pyopenjtalk-plus](https://github.com/tsukumijima/pyopenjtalk-plus) に含まれる辞書を使用しています。

## ライセンス

`haqumei-jlabel` を除く Haqumei の Rust コードは、Apache License 2.0 の条件に基づいて配布されています。

### 同梱ソフトウェアのライセンスと由来

Haqumei は、G2P を提供するために、改変された Open JTalk の C/C++ コードを含んでいます。この同梱コードの由来とライセンスは以下の通りです。

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
- バンドルされた `haqumei-jlabel` ソースコード
  - 由来: `haqumei-jlabel` ディレクトリに含まれるコードは、
    [jpreprocess/jlabel](https://github.com/jpreprocess/jlabel) リポジトリをベースとしています。
  - ライセンス: バンドルされている `haqumei-jlabel` ソースコードは BSD 3-Clause License の下でライセンスされています。このライセンスは
    `haqumei-jlabel` ディレクトリ内のコードにのみ適用され、本プロジェクトの他の部分には適用されません。再配布に関する要件に従い、BSD 3-Clause License の全文は
    `haqumei-jlabel/LICENSE` ファイルに含められています。


## 謝辞

`haqumei` の基礎的な設計とAPIは、`pyopenjtalk` とその大幅に改善されたフォークである `pyopenjtalk-plus` に触発されています。
また、利便性や精度のために、`jlabel` や `kanalizer` をもとにした実装を行いました。

- pyopenjtalk: Copyright (c) 2018 Ryuichi Yamamoto
- pyopenjtalk-plus: Copyright (c) 2023 tsukumijima
- jlabel: Copyright (c) 2024 JPreprocess Team
- kanalizer: Copyright (c) 2025 VOICEVOX

これらの基礎となるプロジェクトの著者および貢献者の皆様に深く感謝いたします。
