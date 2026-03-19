<div align="center">
  <h1>Haqumei 🌅</h1>
  <p>
    Haqumeiは、Rustで実装された日本語の Grapheme-to-Phoneme (G2P) ライブラリです。
  </p>
  <p>
    <a href="https://github.com/stellanomia/haqumei/">English</a> | 日本語
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
</div>

## 特徴 (Features)

- Phoneme <-> Word mapping: Open JTalk (`pyopenjtalk`) に実装されていない、形態素解析の結果と音素をマッピングした詳細情報 (`g2p_pairs`, `g2p_mapping`, `g2p_mapping_detailed`) が取得可能です。 ([Advanced Features](#advanced-features))
- パフォーマンス: Rustによるネイティブ実装と、[`pyopenjtalk-plus`](https://github.com/tsukumijima/pyopenjtalk-plus) で実装されたいくつかの改善を取り入れ、高速なG2Pを実現しています。([ベンチマーク](#ベンチマーク))
- 出力形式: 単純な音素列 (`g2p`) に加え、未知語情報を含む詳細なリスト (`g2p_detailed`)、単語ごとの分割リスト (`g2p_per_word`) など、多様な形式で結果を取得できます。
- 並行処理: `*_batch` 系のメソッドを使うことで、複数のスレッドでG2Pが行えます。

コード例は [haqumei/examples](https://github.com/stellanomia/haqumei/tree/main/haqumei/examples) にあります。

## インストール

### Rust

```bash
cargo add haqumei"
```

### Python

```bash
pip install "git+https://github.com/stellanomia/haqumei.git#subdirectory=haqumei-python"
```

## 使い方 (Usage)

### Rust

```rust
use haqumei::Haqumei;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut haqumei = Haqumei::new()?;

  let text = "日本語のテキストを音素に変換します。";

  // 音素リストに変換
  let phonemes = haqumei.g2p(text)?;
  println!("音素リスト: {:?}", phonemes);

  // pyopenjtalk のようなスペース区切り文字列に変換
  let phoneme_str = phonemes.join(" ");
  println!("スペース区切りの音素: {}", phoneme_str);

  // カタカナ読みに変換
  let kana = haqumei.g2p_kana(text)?;
  println!("カタカナ読み: {}", kana);

  Ok(())
}
```

### Python

```python
from haqumei import Haqumei

# Haqumeiを初期化 (辞書は自動でセットアップされます)
haqumei = Haqumei()

text = "日本語のテキストを音素に変換します。"

# 音素列に変換
phonemes = haqumei.g2p(text)
print(f"音素列: {phonemes}")
# -> 音素列: ['n', 'i', 'h', 'o', 'N', 'g', 'o', 'n', 'o', 't', 'e', 'k', 'i', 's', 'U', 't', 'o', 'o', 'o', 'N', 's', 'o', 'n', 'i', 'h', 'e', 'N', 'k', 'a', 'N', 'sh', 'i', 'm', 'a', 's', 'U']

# pyopenjtalk風のスペース区切り文字列に変換
phoneme_str = " ".join(phonemes)
print(f"スペース区切りの音素: {phoneme_str}")
# -> スペース区切りの音素: n i h o N g o n o t e k i s U t o o o N s o n i h e N k a N sh i m a s U

# カタカナ読みに変換
kana = haqumei.g2p_kana(text)
print(f"カタカナ読み: {kana}")
# -> カタカナ読み: ニホンゴノテキストヲオンソニヘンカンシマス
```

## Advanced Features

### 元の単語文字列との音素マッピングを得る

音素から元の単語の対応を得る `g2p_pairs` が実装されています。  
`JPCommon` の構造体を走査し、各音素の属する単語のポインタを追うことによって実現しています。

```rust
use haqumei::Haqumei;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut haqumei = Haqumei::new()?;

  println!("{:?}", haqumei.g2p_pairs("𰻞𰻞麺＆お冷を頼んだ")?);
  // [WordPhonemePair {
  //     word: "𰻞𰻞",
  //     phonemes: ["pau"]
  // }, WordPhonemePair {
  //     word: "麺",
  //     phonemes: ["m", "e", "N"]
  // }, WordPhonemePair {
  //     word: "＆",
  //     phonemes: ["a", "N", "d", "o"]
  // }, WordPhonemePair {
  //     word: "お冷",
  //     phonemes: ["o", "h", "i", "y", "a"]
  // }, ...
}
```

### 詳細な G2P 出力

Open JTalk (pyopenjtalk) では、未知語は `pau` として扱われますが、`Haqumei` の `g2p` 関数もそれに則っています。  
しかし、`g2p_**_detailed` な関数を使うことで、無視された未知語や空白そのものを `unk`, `sp` として検出可能です。  

`sp` は、入力された空白ではなく、Mecab が出力した、本来 `pyopenjtalk` で無視される`"記号,空白"`であることに注意してください。そのため、Mecab がそもそも無視する記号 (e.g., `\t`, `\n`) などは `sp` に含まれません。  

- 既知語: 通常の音素列 (読点などは `pau`)
- 未知語: `unk`
- 空白等: `sp` (Space)

`g2p_mapping` を使用すると、未知語かどうか (`is_unknown`)、本来のパイプラインで無視されるかどうか (`is_ignored`) という情報とともに、音素と元の単語のマッピングを取得可能です。

```rust
use haqumei::Haqumei;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut haqumei = Haqumei::new()?;

  println!("{:?}", haqumei.g2p_detailed("こんにちは 𰻞𰻞麺")?);
  // ["k", "o", "N", "n", "i", "ch", "i", "w", "a", "sp", "unk", "m", "e", "N"]

  println!("{:?}", haqumei.g2p_mapping("𰻞𰻞麺 お冷を頼んだ")?);
  // [WordPhonemeDetail {
  //     word: "𰻞𰻞",
  //     phonemes: ["unk"],
  //     is_unknown: true,
  //     is_ignored: false,
  // },
  // WordPhonemeDetail {
  //     word: "麺",
  //     phonemes: ["m", "e", "N"],
  //     is_unknown: false,
  //     is_ignored: false,
  // },
  // WordPhonemeDetail {
  //     word: "\u{3000}",
  //     phonemes: ["sp"],
  //     is_unknown: false,
  //     is_ignored: true,
  // },
  // WordPhonemeDetail {
  //     word: "お冷",
  //     phonemes: ["o", "h", "i", "y", "a"],
  //     is_unknown: false,
  //     is_ignored: false,
  // }, ... ]
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
| **haqumei** (Heavy) | 2.101 s | 151k chars/s | 1.12x |
| **haqumei** (`g2p_batch`, Heavy) | 2.208 s | 144k chars/s | 1.07x |

ベンチマークコードは [`haqumei-bench/pyopenjtalk`](https://github.com/stellanomia/haqumei/tree/main/haqumei-bench/pyopenjtalk) にあります。

また、[`Criterion.rs`](https://crates.io/crates/criterion) を使用した Haqumei のベンチマークは、`haqumei-bench` クレートで `cargo bench` することで実行できます。
`pyopenjtalk-plus` との比較ベンチマークは、[`haqumei-bench/pyopenjtalk-plus`](https://github.com/stellanomia/haqumei/tree/main/haqumei-bench/pyopenjtalk-plus) にあります。

### 注意点

- 入力構造によるスループットの変化:  
  本ライブラリは、`pyopenjtalk` に対しては、1行あたりの文字数が多くなるほどスループット（chars/s）が高くなる傾向にあります。  
  これは G2P処理 が Open JTalk 内部の構造体から、直接ラベルを取り出すように実装されていたり、  
  FFI のオーバーヘッドが少ないためであると考えられます。  
  大量の文章を処理する場合は、極端に細かく改行せずにある程度の長さでバッチ処理に渡すのが最も効率的です。

- Default, Heavy の違い:  
  表中のDefault は `Haqumei::new` をそのまま使用しており、  
  Heavyは [HaqumeiOptions](https://docs.rs/haqumei/latest/haqumei/struct.HaqumeiOptions.html) の `predict_nani`, `modify_kanji_yomi` を有効にした場合の計測です。

### Heavy 遅くない?

#### `*_batch` 系メソッドの実装

`modify_kanji_yomi` オプションが有効であるとき、Unidic の読み補正のために Mecab と並行して [vibrato-rkyv](https://github.com/stellanomia/vibrato-rkyv) を動かす関係で、  
`*_batch` 系メソッドでは同一のインスタンスが順次処理を行う設計になっています。  
  
Unidicのトークナイザをスレッドごとに生成するのはメモリや初期化コストの観点で非効率であり、これを並行化するにはより高度なマルチスレッディングの実装が必要となります。しかし、`modify_kanji_yomi` による明確な精度向上がそこまで自明ではないこと（ベースとなる pyopenjtalk-plus 辞書が元から高品質であるため）、および実装コストの兼ね合いから、現在はHeavy設定でのマルチスレッド対応は見送っています。

#### `predict_nani` 機能

`predict_nani` は ONNX を用いますが、セッションをOSスレッドごとに作るのは正気ではないため、`Mutex` を使用しています。(ONNX のセッションはスレッドセーフだが、そのバインディングの ort は `Session::run` を[排他参照をとるようにしている](https://github.com/pykeio/ort/issues/402#issuecomment-2949993914))  

それがボトルネックではないかという懸念に対しては、そもそも Nani Predictor 自体は軽量です。  
また、並行に処理をしている際に、入力に大量の"何"がくることでボトルネックになってしまうケースはまれですし、  
そして並行性に耐性のあるキャッシュ機構を挟んでいるため、DOS的な入力への多少の耐性はあるように思います。  

「吾輩は猫である」 (800個近くの"何"を含む) を用いたベンチマーク(`haqumei-bench`)でも、デフォルトの `Haqumei` とその `predict_nani` を有効にした比較は、平均的には非常に小さい誤差に収まったために、実際にはボトルネックではありません。  

#### `pyopenjtalk-plus` との比較

Sudachi による読み補正と Nani Predictor やその他の改善を取り入れた `pyopenjtalk-plus` は、  
フォーク元の [pyopenjtalk](https://github.com/r9y9/pyopenjtalk) と比べて数十倍～百倍遅いことが知られているので、(see [voicevox_engine#1486](https://github.com/VOICEVOX/voicevox_engine/issues/1486))  
そこまで悪い速度ではないと思っています。  
`pyopenjtalk-plus` に対しては、同様の設定(Heavy)で 50倍 ほど速いですが、  
ROHAN4600 では Haqumei より精度が少し高いので、パフォーマンスの比較対象としていません。  
(Unidic 補正で有意に精度が向上することが分かれば、より攻めた最適化をしたり、また `pyopenjtalk-plus` と同様に Sudachi を使うかもしれません。)  

## 辞書

Haqumeiは [pyopenjtalk-plus](https://github.com/tsukumijima/pyopenjtalk-plus) に含まれる辞書を使用しています。

## ライセンス

Haqumei のRustコードは、Apache License 2.0 の条件に基づいて配布されています。詳細については、リポジトリのルートにある `LICENSE` ファイルを参照してください。

### 同梱ソフトウェアのライセンスと由来

Haqumei は、書記素-音素変換（G2P）機能を提供するために、改変されたOpen JTalkのC/C++ソースコードを含んでいます。この同梱コードの由来とライセンスは以下の通りです。

- 同梱されているOpen JTalkソースコード
  - 由来: `vendor/open_jtalk` ディレクトリに含まれるコードは、[tsukumijima/open_jtalk](https://github.com/tsukumijima/open_jtalk) リポジトリに基づいています。これは、Open JTalkの拡張版に、さまざまなコミュニティフォーク（VOICEVOXプロジェクトからのものを含む）による改善を統合したものです。
  - ライセンス: 同梱されているOpen JTalkソースコードは、修正BSDライセンスの下でライセンスされています。このライセンスは `vendor/open_jtalk` にあるコードにのみ適用され、このプロジェクトの他の部分には適用されません。再配布要件に従い、修正BSDライセンスの全文は `vendor/open_jtalk/src/COPYING` に含まれています。

## 謝辞

`haqumei` の全体的な設計とAPIは、`pyopenjtalk` とその大幅に改善されたフォークである `pyopenjtalk-plus` に触発されています。

- pyopenjtalk: Copyright (c) 2018 Ryuichi Yamamoto (MIT License)
- pyopenjtalk-plus: Copyright (c) 2023 tsukumijima (MIT License)

これらの基礎となるプロジェクトの著者および貢献者の皆様に深く感謝いたします。
