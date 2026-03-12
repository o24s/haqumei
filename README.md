<div align="center">
  <h1>Haqumei 🌅</h1>
  <p>
    Haqumei is a Japanese Grapheme-to-Phoneme (G2P) library implemented in Rust.
  </p>
  <p>
    English | <a href="https://github.com/stellanomia/haqumei/blob/main/README.ja.md">日本語</a>
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

## Features

- **Phoneme <-> Word mapping:** Provides phoneme-to-word alignment by linking morphological analysis results with phonemes (`g2p_mapping`, `g2p_mapping_detailed`).  
  This capability is not available in Open JTalk or pyopenjtalk(-plus). (See [Advanced Features](#advanced-features))
- **Performance:** Achieves fast G2P through a native Rust implementation and by incorporating several improvements from [`pyopenjtalk-plus`](https://github.com/tsukumijima/pyopenjtalk-plus). (See [Benchmark](#benchmark))
- **Output Formats:** Provides results in various formats, including a simple phoneme sequence (`g2p`), a detailed list including unknown word information (`g2p_detailed`), and a list split by words (`g2p_per_word`).
- **Concurrency:** Enables concurrent G2P processing across multiple threads using the `*_batch` methods.

Code examples can be found in [haqumei/examples](https://github.com/stellanomia/haqumei/tree/main/haqumei/examples).

## Install

### Rust

```bash
cargo add haqumei --git "https://github.com/stellanomia/haqumei.git"
```

### Python

```bash
pip install "git+https://github.com/stellanomia/haqumei.git#subdirectory=haqumei-python"
```

## Usage

### Rust

```rust
use haqumei::Haqumei;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut haqumei = Haqumei::new()?;

  let text = "日本語のテキストを音素に変換します。";

  // Convert to a list of phonemes
  let phonemes = haqumei.g2p(text)?;
  println!("Phoneme list: {:?}", phonemes);

  // Convert to a space-separated string like pyopenjtalk
  let phoneme_str = phonemes.join(" ");
  println!("Space-separated phonemes: {}", phoneme_str);

  // Convert to Katakana reading
  let kana = haqumei.g2p_kana(text)?;
  println!("Katakana reading: {}", kana);

  Ok(())
}
```

### Python

```python
from haqumei import Haqumei

# Initialize Haqumei (the dictionary will be automatically set up)
haqumei = Haqumei()

text = "日本語のテキストを音素に変換します。"

# Convert to a phoneme list
phonemes = haqumei.g2p(text)
print(f"Phoneme list: {phonemes}")
# -> Phoneme list: ['n', 'i', 'h', 'o', 'N', 'g', 'o', 'n', 'o', 't', 'e', 'k', 'i', 's', 'U', 't', 'o', 'o', 'o', 'N', 's', 'o', 'n', 'i', 'h', 'e', 'N', 'k', 'a', 'N', 'sh', 'i', 'm', 'a', 's', 'U']

# Convert to a space-separated string like pyopenjtalk
phoneme_str = " ".join(phonemes)
print(f"Space-separated phonemes: {phoneme_str}")
# -> Space-separated phonemes: n i h o N g o n o t e k i s U t o o o N s o n i h e N k a N sh i m a s U

# Convert to Katakana reading
kana = haqumei.g2p_kana(text)
print(f"Katakana reading: {kana}")
# -> Katakana reading: ニホンゴノテキストヲオンソニヘンカンシマス
```

## Advanced Features

### Getting Phoneme Mapping with the Original Word String

Haqumei implements `g2p_mapping` to obtain the correspondence between phonemes and their original words.  
This is achieved by traversing the `JPCommon` structure and tracking the pointers to the words to which each phoneme belongs.

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
  // }, ... ]
}
```

### Detailed G2P Output

In Open JTalk (`pyopenjtalk`), unknown words are treated as `pau` (pauses), and Haqumei's standard `g2p` function follows this behavior.  
However, by using the `g2p_**_detailed` functions, you can detect otherwise ignored unknown words and spaces as `unk` and `sp` respectively.  

Please note that `sp` does not refer to raw space characters in the input, but rather the `"記号,空白"` (symbol, space) part-of-speech output by Mecab, which is normally ignored in `pyopenjtalk`. Therefore, symbols that Mecab itself ignores (e.g., `\t`, `\n`) are not included in `sp`.  

- **Known words**: Regular phoneme sequence (punctuation marks become `pau`).
- **Unknown words**: `unk`
- **Spaces, etc.**: `sp` (Space)

Using `g2p_mapping_detailed`, you can obtain the phoneme-to-word mapping along with flags indicating whether a word is unknown (`is_unknown`) and whether it would normally be ignored in the original pipeline (`is_ignored`).

```rust
use haqumei::Haqumei;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut haqumei = Haqumei::new()?;

  println!("{:?}", haqumei.g2p_detailed("こんにちは 𰻞𰻞麺")?);
  // ["k", "o", "N", "n", "i", "ch", "i", "w", "a", "sp", "unk", "m", "e", "N"]

  println!("{:?}", haqumei.g2p_mapping_detailed("𰻞𰻞麺 お冷を頼んだ")?);
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

### Modifying Output with G2P Options

You can customize the behavior of `Haqumei` by using `Haqumei::with_options`.
For details on the default behavior and available options, please refer to [HaqumeiOptions](https://stellanomia.github.io/haqumei/haqumei/struct.HaqumeiOptions.html).

In the following example, `normalize_unicode` (which is disabled by default) is enabled to apply Unicode NFC normalization to the input text.

```rust
use haqumei::{Haqumei, HaqumeiOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut haqumei = Haqumei::with_options(HaqumeiOptions {
    normalize_unicode: true,
    ..Default::default()
  })?;

  let text = &[
    "\u{304B}\u{3099}", // か + ゙ (が)
    "\u{306F}\u{309A}", // は + ゚ (ぱ)
    "\u{30B3}\u{3099}", // コ + ゙ (ゴ)
  ];

  println!("{:?}", haqumei.g2p_detailed_batch(text)?);
  // Output: [["g", "a"], ["p", "a"], ["g", "o"]]
}
```

## Benchmark

Here are the comparison results between `pyopenjtalk` (Baseline) and `haqumei` using approximately 318,000 characters of Japanese text.

Input data: [I Am a Cat (吾輩は猫である)](https://www.aozora.gr.jp/cards/000148/files/789_14547.html) 318,407 chars / 8,451 lines (Average 37 chars/line) (Ruby characters have been removed)

| Execution Mode | Execution Time (Mean) | Throughput | Speedup |
| :--- | :--- | :--- | :--- |
| **pyopenjtalk** (Baseline) | 2.358 s | 135k chars/s | 1.00x |
| **haqumei** (Default) | 1.303 s | 244k chars/s | **1.81x** |
| **haqumei** (`g2p_batch`, Default) | 0.098 s | 3.24M chars/s | 24.04x |
| **haqumei** (Heavy) | 2.101 s | 151k chars/s | 1.12x |
| **haqumei** (`g2p_batch`, Heavy) | 2.208 s | 144k chars/s | 1.07x |

The detailed benchmark code can be found in [`haqumei-bench/pyopenjtalk`](https://github.com/stellanomia/haqumei/tree/main/haqumei-bench/pyopenjtalk).

Additionally, Rust-layer benchmarks for Haqumei using [`Criterion.rs`](https://crates.io/crates/criterion) can be run via `cargo bench` in the `haqumei-bench` crate. The comparison benchmark with `pyopenjtalk-plus` is located in [`haqumei-bench/pyopenjtalk-plus`](https://github.com/stellanomia/haqumei/tree/main/haqumei-bench/pyopenjtalk-plus).

### Performance Notes

- **Throughput Variation by Input Structure**:  
  Compared to `pyopenjtalk`, throughput (chars/s) improves as the average number of characters per line increases. This is due to reduced FFI call overhead and the efficient direct extraction of labels from Open JTalk's internal structures.  
  When processing large amounts of text, it's most efficient to pass the content in batches at reasonable lengths rather than breaking it into excessively fine-grained lines.
- **Difference Between Default and Heavy**:  
  In the table, "Default" represents the configuration using `Haqumei::new` as is, while "Heavy" shows the results when `predict_nani` and `modify_kanji_yomi` are enabled in [HaqumeiOptions](https://stellanomia.github.io/haqumei/haqumei/struct.HaqumeiOptions.html).

### Considerations on Heavy Configuration Performance

#### Scaling Limitations of `*_batch` Methods

When the `modify_kanji_yomi` option is enabled, [vibrato-rkyv](https://github.com/stellanomia/vibrato-rkyv) is run concurrently with Mecab to correct readings using Unidic. Because of this, the `*_batch` methods are designed to process sequentially within the same instance.  
Generating a Unidic tokenizer per thread is inefficient in terms of memory and initialization costs. Parallelizing this would require implementing more advanced multithreading. However, since the significant accuracy improvement from `modify_kanji_yomi` is not clearly demonstrated (as the base `pyopenjtalk-plus` dictionary is already of high quality) and considering the implementation costs, multithreading for the Heavy configuration is currently not implemented.

#### Overhead of the `predict_nani` Feature

`predict_nani` uses ONNX. Since creating an ONNX session per OS thread is a bit wild, a `Mutex` is used to share the session. (While ONNX sessions themselves are thread-safe, the Rust binding `ort` requires exclusive reference for `Session::run` as discussed [here](https://github.com/pykeio/ort/issues/402#issuecomment-2949993914)).  
Regarding concerns about this becoming a bottleneck: the Nani Predictor itself is extremely lightweight. Also, unless the input contains an unusually massive amount of "何 (nani)", it practically does not affect performance. Furthermore, a concurrency-resilient caching mechanism is in place, providing some tolerance against extreme inputs.  
In fact, even in the benchmark using "I Am a Cat" (which contains nearly 800 instances of "何"), the difference in execution time between the default setting and the one with `predict_nani` enabled was negligible, confirming it is not a practical bottleneck.

#### Comparison with `pyopenjtalk-plus`

It is known that `pyopenjtalk-plus`, which incorporates reading corrections via Sudachi and the Nani Predictor, is tens to hundreds of times slower than the original [pyopenjtalk](https://github.com/r9y9/pyopenjtalk) (see [voicevox_engine#1486](https://github.com/VOICEVOX/voicevox_engine/issues/1486)). Thus, we believe the current execution speed is quite reasonable.  
Haqumei runs about 50 times faster than `pyopenjtalk-plus` under a similar Heavy configuration. However, since `pyopenjtalk-plus` achieves slightly higher accuracy using models like [ROHAN4600](https://github.com/mmorise/rohan4600), it is not treated merely as a target for speed comparison. If significant accuracy improvements from Unidic corrections are confirmed in the future, we might pursue more aggressive optimizations or adopt Sudachi similarly to `pyopenjtalk-plus`.

## Dictionary

Haqumei uses the dictionary included in [pyopenjtalk-plus](https://github.com/tsukumijima/pyopenjtalk-plus).

## License

The Rust code of `haqumei` is distributed under the terms of the Apache License 2.0. See the `LICENSE` file in the repository root for details.

### Licenses and Origins of Bundled Software

`haqumei` includes C/C++ source code from a modified version of Open JTalk to provide its Grapheme-to-Phoneme (G2P) functionality. The origins and licenses of this bundled code are as follows:

- Bundled Open JTalk Source Code
  - Origin: The code contained in the `vendor/open_jtalk` directory is based on the
    [tsukumijima/open_jtalk](https://github.com/tsukumijima/open_jtalk) repository, which integrates
    improvements from various community forks (including those from the VOICEVOX project) into an enhanced
    version of Open JTalk.
  - License: The bundled Open JTalk source code is licensed under the Modified BSD License. This license applies
    only to the code located in `vendor/open_jtalk`, and does not apply to the rest of this project. In accordance
    with redistribution requirements, the full text of the Modified BSD License is included in
    `vendor/open_jtalk/src/COPYING`.

## Acknowledgements

The overall design and API of `haqumei` are inspired by `pyopenjtalk` and its highly improved fork, `pyopenjtalk-plus`.

- pyopenjtalk: Copyright (c) 2018 Ryuichi Yamamoto (MIT License)
- pyopenjtalk-plus: Copyright (c) 2023 tsukumijima (MIT License)

We are deeply grateful to the authors and contributors of these foundational projects.
