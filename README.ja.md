<div align="center">
  <h1>Haqumei</h1>
  <p>
    Haqumeiは、Rustで実装された日本語の Grapheme-to-Phoneme (G2P) ライブラリです。
  </p>
  <p>
    <a href="https://github.com/stellanomia/haqumei/">English</a> | 日本語
  </p>
  <p>
    <a href="LICENSE"><img src="https://img.shields.io/badge/License-Apache--2.0-blue.svg" alt="License: Apache-2.0"></a>
  </p>
</div>

## インストール

### Rust

```bash
cargo add haqumei --git "https://github.com/stellanomia/haqumei.git"
```

### Python

```bash
pip install "git+https://github.com/stellanomia/haqumei.git#subdirectory=haqumei-python"
```

## 特徴 (Features)

- パフォーマンス: Rustによるネイティブ実装と、[`pyopenjtalk-plus`](https://github.com/tsukumijima/pyopenjtalk-plus) で実装されたいくつかの改善を取り入れ、高速なG2Pを実現します。
- 出力形式: 単純な音素列 (`g2p`) に加え、未知語情報を含む詳細なリスト (`g2p_detailed`)、単語ごとの分割リスト (`g2p_per_word`) など、多様な形式で結果を取得できます。
- 多様な解析情報: 形態素解析の結果と音素をマッピングした詳細情報 (`g2p_mapping`, `g2p_mapping_detailed`) や、`pyopenjtalk` と同様にフルコンテキストラベル (`extract_fullcontext`) も取得可能です。

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

## Advanced Usage

### 元の単語文字列との音素マッピングを得る

本来の Open JTalk では微妙に取得できなかった、音素から元の単語の対応を得る `g2p_mapping` が実装されています。  
これは、`JPCommon` の構造体を走査し、音素のそれぞれの属する単語のポインタを追うことによって実装されました。

```rust
use haqumei::Haqumei;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut haqumei = Haqumei::new()?;

  println!("{:?}", haqumei.g2p_mapping("𰻞𰻞麺＆お冷を頼んだ")?);
  // [WordPhonemeMap {
  //     word: "𰻞𰻞",
  //     phonemes: ["pau"]
  // }, WordPhonemeMap {
  //     word: "麺",
  //     phonemes: ["m", "e", "N"]
  // }, WordPhonemeMap {
  //     word: "＆",
  //     phonemes: ["a", "N", "d", "o"]
  // }, WordPhonemeMap {
  //     word: "お冷",
  //     phonemes: ["o", "h", "i", "y", "a"]
  // }, ...
}
```

### 詳細な G2P 出力

Open JTalk (pyopenjtalk) では、未知語は読点(`、`)として扱われ、`Haqumei`の`g2p`関数もそれに則っています。  
`g2p_**_detailed` な関数を使うことで、無視された未知語や空白そのものを`unk`, `sp` として検出可能です。  

`sp` は、入力された空白ではなく、Mecab が出力した、本来 `pyopenjtalk` で無視されていた`"記号,空白"`であることに注意してください。そのため、Mecab がそもそも無視する記号 (e.g., `\t`, `\n`) などは `sp` に含まれません。  

- 既知語: 通常の音素列 (読点などは `pau`)
- 未知語: `unk`
- 空白等: `sp` (Space)

`g2p_mapping_detailed` を使用すると、未知語かどうか (`is_unknown`)、本来のパイプラインで無視されるかどうか (`is_ignored`) という情報とともに、音素と元の単語のマッピングを取得可能です。

```rust
use haqumei::Haqumei;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut haqumei = Haqumei::new()?;

  println!("{:?}", haqumei.g2p_detailed("こんにちは 𰻞𰻞麺")?);
  // ["k", "o", "N", "n", "i", "ch", "i", "w", "a", "sp", "unk", "m", "e", "N"]

  println!("{:?}", haqumei.g2p_mapping_detailed("𰻞𰻞麺 お冷を頼んだ")?);
  // [WordPhonemeDetail {
  //     word: "𰻞𰻞",
  //     phonemes: [
  //         "unk",
  //     ],
  //     is_unknown: true,
  //     is_ignored: false,
  // },
  // WordPhonemeDetail {
  //     word: "麺",
  //     phonemes: [
  //         "m",
  //         "e",
  //         "N",
  //     ],
  //     is_unknown: false,
  //     is_ignored: false,
  // },
  // WordPhonemeDetail {
  //     word: "\u{3000}",
  //     phonemes: [
  //         "sp",
  //     ],
  //     is_unknown: false,
  //     is_ignored: true,
  // },
  // WordPhonemeDetail {
  //     word: "お冷",
  //     phonemes: [
  //         "o",
  //         "h",
  //         "i",
  //         "y",
  //         "a",
  //     ],
  //     is_unknown: false,
  //     is_ignored: false,
  // }, ...
}
```

## 辞書

Haqumeiは [pyopenjtalk-plus](https://github.com/tsukumijima/pyopenjtalk-plus) に含まれる辞書を使用しています。

## ライセンス

`haqumei` のRustコードは、Apache License 2.0 の条件に基づいて配布されています。詳細については、リポジトリのルートにある `LICENSE` ファイルを参照してください。

### 同梱ソフトウェアのライセンスと由来

`haqumei` は、書記素-音素変換（G2P）機能を提供するために、改変されたOpen JTalkのC/C++ソースコードを含んでいます。この同梱コードの由来とライセンスは以下の通りです。

- 同梱されているOpen JTalkソースコード
  - 由来: `vendor/open_jtalk` ディレクトリに含まれるコードは、[tsukumijima/open_jtalk](https://github.com/tsukumijima/open_jtalk) リポジトリに基づいています。これは、Open JTalkの拡張版に、さまざまなコミュニティフォーク（VOICEVOXプロジェクトからのものを含む）による改善を統合したものです。
  - ライセンス: 同梱されているOpen JTalkソースコードは、修正BSDライセンスの下でライセンスされています。このライセンスは `vendor/open_jtalk` にあるコードにのみ適用され、このプロジェクトの他の部分には適用されません。再配布要件に従い、修正BSDライセンスの全文は `vendor/open_jtalk/src/COPYING` に含まれています。

## 謝辞

`haqumei` の全体的な設計とAPIは、`pyopenjtalk` とその大幅に改善されたフォークである `pyopenjtalk-plus` に触発されています。

- pyopenjtalk: Copyright (c) 2018 Ryuichi Yamamoto (MIT License)
- pyopenjtalk-plus: Copyright (c) 2023 tsukumijima (MIT License)

これらの基礎となるプロジェクトの著者および貢献者の皆様に深く感謝いたします。
