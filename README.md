<div align="center">
  <h1>Haqumei 🌅</h1>
  <p>
    Haqumei is a Japanese Grapheme-to-Phoneme (G2P) library implemented in Rust.
  </p>
  <p>
    English | <a href="https://github.com/o24s/haqumei/blob/main/README.ja.md">日本語</a>
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

- **Phoneme <-> Word mapping:** Provides phoneme-to-word alignment by linking morphological analysis results with phonemes (`g2p_pairs`, `g2p_mapping`, `g2p_mapping_detailed`).  
  This capability is not available in Open JTalk (`pyopenjtalk`). (See [Advanced Features](#advanced-features))
- **Prosody Information Retrieval:** Provides phoneme sequences enriched with prosodic symbols as well as lossless mapping to surface forms (`g2p_prosody`, `g2p_mapping_prosody`). (For more details, see [Prosody Features](#prosody-features-g2p_prosody--g2p_mapping_prosody).)
- **Performance:** Enables fast processing through a native Rust implementation. (See [Benchmark](#benchmark))
- **Accuracy:** Improves accuracy by incorporating many techniques implemented in [`pyopenjtalk-plus`](https://github.com/tsukumijima/pyopenjtalk-plus). (See [Accuracy](#accuracy))
- **Output Formats:** Provides results in various formats, including a simple phoneme sequence (`g2p`) and a detailed list including unknown word information (`g2p_detailed`).
- **Concurrency:** Enables concurrent G2P processing across multiple threads using the `*_batch` methods.

Examples can be found in [haqumei/examples](https://github.com/o24s/haqumei/tree/main/haqumei/examples).

## Install

### Rust

During the initial build of `haqumei`, the dictionary is downloaded and embedded into the binary due to the file size limits on crates.io.
For custom dictionaries, or for environments where network access is unavailable during the build, please refer to [here](#building-with-a-custom-embedded-dictionary).

```bash
cargo add haqumei
```

### Python

```bash
pip install "git+https://github.com/o24s/haqumei.git#subdirectory=haqumei-python"
```

## Command-Line Tool

We also provide `haqumei-cli`, a command-line interface for text processing from the terminal.
For detailed usage, including pipeline processing and JSON output, please see [`haqumei-cli/README.md`](./haqumei-cli/README.md)

```bash
cargo install haqumei-cli
```

## Usage

### Rust

```rust
use haqumei::Haqumei;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut haqumei = Haqumei::new()?;

  let text = "こんにちは、世界！";

  // Convert to phoneme list
  let phonemes = haqumei.g2p(text)?;
  assert_eq!(phonemes, ["k", "o", "N", "n", "i", "ch", "i", "w", "a", "pau", "s", "e", "k", "a", "i"]);

  // Get phoneme list with prosodic symbols
  let phones = haqumei.g2p_prosody(text)?.join(" ");
  assert_eq!(phones, "^ k o [ N n i ch i w a _ s e ] k a i ! $");

  // Convert to katakana reading
  let kana = haqumei.g2p_kana(text)?;
  assert_eq!(kana, "コンニチワ、セカイ！");

  Ok(())
}
```

### Python

```python
from haqumei import Haqumei

# Initialize Haqumei (the dictionary will be automatically set up)
haqumei = Haqumei()

text = "こんにちは、世界！"

# Convert to a phoneme list
phonemes = haqumei.g2p(text)
print(f"Phonemes: {phonemes}")
# -> Phonemes: ["k", "o", "N", "n", "i", "ch", "i", "w", "a", "pau", "s", "e", "k", "a", "i"]

# Get phoneme list with prosodic symbols
phones = " ".join(haqumei.g2p_prosody(text))
print(f"Prosody-annotated phonemes: {phones}")
# -> Prosody-annotated phonemes: ^ k o [ N n i ch i w a _ s e ] k a i ! $

# Convert to katakana reading
kana = haqumei.g2p_kana(text)
print(f"Katakana reading: {kana}")
# -> Katakana reading: コンニチワ、セカイ！
```

## Advanced Features

### Getting Phoneme Mapping with the Original Word String

Haqumei implements `g2p_pairs` to obtain the correspondence between phonemes and their original words.  
This is achieved by traversing the `JPCommon` structure and tracking the pointers to the words to which each phoneme belongs.

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
  // }, ... ]

  Ok(())
}
```

### Detailed G2P Output

In Open JTalk (`pyopenjtalk`), unknown words are treated as `pau` (pauses), and Haqumei's standard `g2p` function follows this behavior.  
However, by using the `g2p_**_detailed` functions, you can detect otherwise ignored unknown words and spaces as `unk` and `sp` respectively.  

Please note that `sp` does not refer to raw space characters in the input, but rather the `"記号,空白"` (symbol, space) part-of-speech output by Mecab, which is normally ignored in `pyopenjtalk`. Therefore, symbols that Mecab itself ignores (e.g., `\t`, `\n`) are not included in `sp`.  

- **Known words**: Regular phoneme sequence (punctuation marks become `pau`).
- **Unknown words**: `unk`
- **Spaces, etc.**: `sp` (Space)

Using `g2p_mapping`, you can obtain the phoneme-to-word mapping along with flags indicating whether a word is unknown (`is_unknown`) and whether it would normally be ignored in the original pipeline (`is_ignored`).
In addition, using `g2p_mapping_detailed` allows you to retrieve not only the mapping but also part-of-speech information and accent details.


```rust
use haqumei::Haqumei;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut haqumei = Haqumei::new()?;

  println!("{:?}", haqumei.g2p_detailed("こんにちは 𰻞𰻞麺")?);
  // ["k", "o", "N", "n", "i", "ch", "i", "w", "a", "sp", "unk", "m", "e", "N"]

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

### Modifying Output with G2P Options

You can customize the behavior of `Haqumei` by using `Haqumei::with_options`.
For details on the default behavior and available options, please refer to [HaqumeiOptions](https://docs.rs/haqumei/latest/haqumei/struct.HaqumeiOptions.html).

In the following example, `normalize_unicode` (which is disabled by default) is enabled to apply Unicode NFC normalization to the input text.

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
  // Output: [["g", "a"], ["p", "a"], ["g", "o"]]

  Ok(())
}
```

## Prosody Features (`g2p_prosody` / `g2p_mapping_prosody`)

### Specification of `g2p_prosody_with_options`

Converts the input text into a phoneme list annotated with prosodic symbols based on the `ProsodyFormat` setting.
(The `g2p_prosody` method behaves identically to specifying `ProsodyFormat::Default`.)

The output commonly includes the following prosodic symbols:

| Symbol | Meaning | Position |
| :--- | :--- | :--- |
| `^` | Beginning of utterance (BOS) | Sentence-initial |
| `$` | End of utterance (EOS) | Sentence-final |
| `?` | End of interrogative (？) | Sentence-medial / Sentence-final |
| `!` | End of exclamation (Custom extension) | Sentence-medial / Sentence-final |
| `_` | Pause / Comma (、) | Sentence-medial |
| `#` | Accent phrase boundary | Sentence-medial |
| `{...}` | Unknown word | Sentence-medial |

For more information on Japanese accents, please refer to the [tdmelodic User Manual / Preliminary Knowledge](https://tdmelodic.readthedocs.io/ja/latest/pages/introduction.html) (Japanese).

#### ProsodyFormat::Default

In addition to the above, the output includes the following prosodic symbols:

| Symbol | Meaning | Position |
| :--- | :--- | :--- |
| `[` | Pitch rise (Phrase head) | Near the beginning of a phrase |
| `]` | Pitch fall (Accent nucleus) | Right after the nuclear mora |

The symbols `[` and `]` are based on the accent notation commonly used in tdmelodic and similar tools.
They correspond to `^` and `!` in the algorithm described by Kurihara et al. (2021) in *"Prosodic Features Control by Symbols as Input of Sequence-to-Sequence Acoustic Modeling for Neural TTS"*.

#### ProsodyFormat::Prefix

Instead of using pitch rise/fall symbols (`[` and `]`), pitch high/low is attached as a prefix to each phoneme:
 - `H_` : High pitch
 - `L_` : Low pitch

The pitch is explicitly indicated for each phoneme.

Example: `"青い空"` -> `["^", "L_a", "H_o", "L_i", "#", "H_s", "H_o", "L_r", "L_a", "$"]`

#### ProsodyFormat::Numeric

Pitch high/low is attached as a suffix to each phoneme as a numeric value:
 - `:1` : High pitch
 - `:0` : Low pitch

Example: `"青い空"` -> `["^", "a:0", "o:1", "i:0", "#", "s:1", "o:1", "r:0", "a:0", "$"]`

#### Example

```rust
use haqumei::Haqumei;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut haqumei = Haqumei::new()?;

  let phones = haqumei.g2p_prosody("こんにちは、世界！")?;
  assert_eq!(phones.join(" "), "^ k o [ N n i ch i w a _ s e ] k a i ! $");

  let phones = haqumei.g2p_prosody("青い空、広がる。")?;
  assert_eq!(phones.join(" "), "^ a [ o ] i # s o ] r a _ h i [ r o g a r u $");

  Ok(())
}
```

### Specification of `g2p_mapping_prosody`

On the other hand, `g2p_mapping_prosody` analyzes the input text and retrieves an alignment between detailed linguistic information for each morpheme (word) and phonemes with prosodic symbols.

While [`Haqumei::g2p_prosody`] and [`Haqumei::g2p_prosody_with_options`] return a flat list of strings (`Vec<String>`), this function returns structured data (`Vec<WordPhonemeProsody>`) annotated with part-of-speech, accent type, reading, and pitch information.

This is suitable for speech synthesis frontend processing when you want to maintain the correspondence between morphemes and phonemes, individually retrieve and manipulate pitch high/low ([`PitchAccent`]), or handle unknown words.

#### Information included in `WordPhonemeProsody`

The following information is included as data for each morpheme:

| Field | Description | Example |
| :--- | :--- | :--- |
| `word` | Surface form of the morpheme | `"空"` |
| `pos`, `pos_group1`~`3` | Part-of-speech and its subdivisions | `"名詞"`, `"一般"` |
| `orig`, `read`, `pron` | Original form, reading, pronunciation form | `"空"`, `"ソラ"`, `"ソラ"` |
| `accent_nucleus` | Accent nucleus position (0: Heiban type, 1~: n-th mora) | `1` |
| `mora_count` | Number of moras | `2` |
| `is_unknown` | Whether it was judged as an unknown word by MeCab | `false` |
| `is_ignored` | Whether no phoneme was assigned | `false` |

#### Prosodic Phoneme (`ProsodicPhoneme`)

The `phonemes` field contains a list of the following elements:

| Variant | Meaning | Output symbol in `g2p_prosody`, etc. |
| :--- | :--- | :--- |
| `Phoneme` | The phoneme itself and its pitch (`High` / `Low`) | `a`, `a:0`, `H_a`, etc. |
| `AccentPhraseBoundary` | Accent phrase boundary | `#` |
| `Pause` | Regular pause / comma | `_` |
| `Interrogative` | End of interrogative / Pause | `?` |
| `Exclamatory` | End of exclamation / Pause | `!` |

#### Example

```rust
use haqumei::{Haqumei, PitchAccent, ProsodicPhoneme};

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut haqumei = Haqumei::new()?;

  // Retrieve text as structured data per morpheme
  let mapping = haqumei.g2p_mapping_prosody("青い空が、好きだ！")?;

  // Morpheme information for "青い"
  let aoi = &mapping[0];
  assert_eq!(aoi.word, "青い");
  assert_eq!(aoi.pos, "形容詞");
  assert_eq!(aoi.read, "アオイ");
  assert_eq!(aoi.accent_nucleus, 2); // 中高型

  // Phoneme and pitch information for "青い" (a: Low, o: High, i: Low)
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

## Accuracy

We evaluated the accuracy using the `haqumei-eval` crate. Below are the results:
- **Phoneme Error Rate (PER)** evaluated on [prj-beatrice/jsut-label](https://github.com/prj-beatrice/jsut-label), a fork of `jsut-label` providing annotations for the Basic5000 subset of the JSUT corpus.
- **Katakana Error Rate (Katakana ER)** evaluated on [ROHAN](https://github.com/mmorise/rohan4600).

### jsut-label

Phoneme Error Rate (S+D+I / N_expected): **1.24%** (Substitute=2244, Delete=572, Insert=889, N=297843)

`HaqumeiOptions`:
```rust
HaqumeiOptions {
  use_unidic_yomi: true,
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

## Benchmark

Here are the comparison results between `pyopenjtalk` (Baseline) and `haqumei` using approximately 318,000 characters of Japanese text.

Input data: [I Am a Cat (吾輩は猫である)](https://www.aozora.gr.jp/cards/000148/files/789_14547.html) 318,407 chars / 8,451 lines (Average 37 chars/line) (Ruby characters have been removed)

| Execution Mode | Execution Time (Mean) | Throughput | Speedup |
| :--- | :--- | :--- | :--- |
| **pyopenjtalk** (Baseline) | 2.358 s | 135k chars/s | 1.00x |
| **haqumei** (Default) | 1.303 s | 244k chars/s | **1.81x** |
| **haqumei** (`g2p_batch`, Default) | 0.098 s | 3.24M chars/s | 24.04x |
| **haqumei** (Heavy) | 2.101 s | 151k chars/s | 1.12x |
| **haqumei** (`g2p_batch`, Heavy) | 0.268 s | 1.18M chars/s | 8.80x |

The detailed benchmark code can be found in [`haqumei-bench/pyopenjtalk`](https://github.com/o24s/haqumei/tree/main/haqumei-bench/pyopenjtalk).

Additionally, Rust-layer benchmarks for Haqumei using [`Criterion.rs`](https://crates.io/crates/criterion) can be run via `cargo bench` in the `haqumei-bench` crate. The comparison benchmark with `pyopenjtalk-plus` is located in [`haqumei-bench/pyopenjtalk-plus`](https://github.com/o24s/haqumei/tree/main/haqumei-bench/pyopenjtalk-plus).

### Performance Notes

- **Throughput Variation by Input Structure**:  
  Especially in the `*_batch` APIs, throughput (chars/s) tends to increase as the number of characters per line grows (up to approximately 4KB), compared with pyopenjtalk. This efficiency stems from an implementation that directly extracts labels from Open JTalk's internal structures, combined with minimal FFI overhead. When processing large volumes of text, it is most efficient to pass content in substantial chunks rather than splitting it into excessively short lines.
- **Difference Between Default and Heavy**:  
  In the table, "Default" represents the configuration using `Haqumei::new` as is, while "Heavy" shows the results when `predict_nani` and `use_unidic_yomi` are enabled in [HaqumeiOptions](https://docs.rs/haqumei/latest/haqumei/struct.HaqumeiOptions.html).

## Building with a Custom Embedded Dictionary

By default, `haqumei` downloads the dictionary at build time and embeds it into the binary.
This allows the crate to be published to crates.io while still producing a self-contained binary.

If you want to build with your own dictionary embedded in the binary, you can change the configuration as follows.

### Change the Cargo Features

Disable the default `download-dictionary` feature and enable `build-dictionary`.
```toml
[dependencies]
haqumei = { version = "x.y.z", features = ["embed-dictionary", "build-dictionary"], default-features = false }
```

### Prepare the Dictionary Source and Set the Environment Variable

Prepare a dictionary source directory containing `.csv` and `.def` files to be compiled at build time, then set its path to the `HAQUMEI_DICT_SRC` environment variable before running the build.

On Unix-like systems:
```bash
HAQUMEI_DICT_SRC="/path/to/your/dictionary" cargo build --release
```

On Windows (PowerShell):
```powershell
& { $env:HAQUMEI_DICT_SRC="C:\path\to\your\dictionary"; cargo build --release }
```

> **Note:** If the environment variable is not set, the build script falls back to `dictionary`, relative to the crate root.

## Dictionary

Haqumei uses the dictionary included in [pyopenjtalk-plus](https://github.com/tsukumijima/pyopenjtalk-plus).

## License

The Rust code of Haqumei, excluding `haqumei-jlabel`, is distributed under the terms of the Apache License 2.0.

### Licenses and Origins of Bundled Software

`haqumei` includes C/C++ source code and dictionary data from modified versions of Open JTalk to provide its Grapheme-to-Phoneme (G2P) functionality. The origins and licenses of this bundled code are as follows:

- Bundled Open JTalk Source Code
  - Origin: The code contained in the `vendor/open_jtalk` directory is based on the
    [tsukumijima/open_jtalk](https://github.com/tsukumijima/open_jtalk) repository, which integrates
    improvements from various community forks (e.g., VOICEVOX project) into an enhanced
    version of Open JTalk.
  - License: The bundled Open JTalk source code is licensed under the Modified BSD License. This license applies
    only to the code located in `vendor/open_jtalk`, and does not apply to the rest of this project. In accordance
    with redistribution requirements, the full text of the Modified BSD License is included in
    `vendor/open_jtalk/src/COPYING`.

- Bundled Dictionary Data
  - Origin: The dictionary data contained in the `haqumei/dictionary` directory is based on
    [tsukumijima/pyopenjtalk-plus](https://github.com/tsukumijima/pyopenjtalk-plus), a modified fork of
    [r9y9/pyopenjtalk](https://github.com/r9y9/pyopenjtalk).
  - License: The dictionary data is covered by the license notices in `haqumei/dictionary/COPYING`.

- Bundled `haqumei-jlabel` Source Code
  - Origin: The code contained in the `haqumei-jlabel` directory is based on the
    [jpreprocess/jlabel](https://github.com/jpreprocess/jlabel) repository.
  - License: The bundled `haqumei-jlabel` source code is licensed under the BSD 3-Clause License. This license applies
    only to the code located in `haqumei-jlabel`, and does not apply to the rest of this project. In accordance
    with redistribution requirements, the full text of the BSD 3-Clause License is included in
    `haqumei-jlabel/LICENSE`.

## Acknowledgements

The overall design and API of `haqumei` are inspired by `pyopenjtalk` and its highly improved fork, `pyopenjtalk-plus`.

- pyopenjtalk: Copyright (c) 2018 Ryuichi Yamamoto
- pyopenjtalk-plus: Copyright (c) 2023 tsukumijima

We are deeply grateful to the authors and contributors of these foundational projects.
