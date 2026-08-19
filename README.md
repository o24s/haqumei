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

## Table of Contents

- [Features](#features)
- [Install](#install)
  - [Rust](#rust)
  - [Python](#python)
    - [Supported Platforms](#supported-platforms)
- [Command-Line Tool](#command-line-tool)
- [Usage](#usage)
  - [Rust](#rust-1)
  - [Python](#python-1)
- [Advanced Features](#advanced-features)
  - [Word-Phoneme Mapping APIs](#word-phoneme-mapping-apis)
  - [Modifying Output with G2P Options](#modifying-output-with-g2p-options)
- [Prosody Features (`g2p_prosody` / `g2p_mapping_prosody`)](#prosody-features-g2p_prosody--g2p_mapping_prosody)
  - [Specification of `g2p_prosody_with_options`](#specification-of-g2p_prosody_with_options)
  - [Specification of `g2p_mapping_prosody`](#specification-of-g2p_mapping_prosody)
- [Accuracy](#accuracy)
  - [Reproducing](#reproducing)
  - [jsut-label](#jsut-label)
  - [ROHAN](#rohan)
- [Benchmark](#benchmark)
  - [Performance Notes](#performance-notes)
- [Building with a Custom Embedded Dictionary](#building-with-a-custom-embedded-dictionary)
  - [Change the Cargo Features](#change-the-cargo-features)
  - [Prepare the Dictionary Source and Set the Environment Variable](#prepare-the-dictionary-source-and-set-the-environment-variable)
- [Dictionary](#dictionary)
- [License](#license)
  - [Licenses and Origins of Bundled Software](#licenses-and-origins-of-bundled-software)
- [Acknowledgements](#acknowledgements)

## Features

| | |
| :--- | :--- |
| **Word-Phoneme Mapping APIs** | Provides mapping information between words ($\approx$ surface forms / dictionary entries) and phonemes, which was previously difficult to obtain directly. Enables retrieval of detailed analysis results with minimal loss of information from the input text, including unknown-word information. (See [Advanced Features](#advanced-features)) |
| **Prosody Information Retrieval** | Provides phoneme sequences annotated with prosodic symbols, along with a word-to-phoneme mapping carrying structured prosody information (`g2p_prosody`, `g2p_mapping_prosody`). (For more details, see [Prosody Features](#prosody-features-g2p_prosody--g2p_mapping_prosody).) |
| **More Detailed Phoneme Labels** | Through allophone resolution for moraic nasals (撥音) and geminate consonants (促音), you can choose from several options for the allophones introduced as dedicated phoneme labels. (See [here](https://docs.rs/haqumei/latest/haqumei/phoneme/index.html) for details.) |
| **Performance** | Enables fast processing through a native Rust implementation. (See [Benchmark](#benchmark)) |
| **Accuracy** | Successive dictionary and logic improvements reach 0.87% PER on [jsut-label](https://github.com/prj-beatrice/jsut-label) and 0.81% CER on [ROHAN](https://github.com/mmorise/rohan4600). Further changes build on the dictionary and the accuracy techniques of [`pyopenjtalk-plus`](https://github.com/tsukumijima/pyopenjtalk-plus). (See [Accuracy](#accuracy)) |
| **Unknown Word Fallbacks** | Reading estimation for English words that would otherwise be unknown via `haqumei-kanalizer`, an on'yomi fallback for kanji that match no dictionary entry, and accent correction for words written entirely in katakana. |
| **Concurrency** | Enables concurrent G2P processing across multiple threads using the `*_batch` methods. |
| **Diverse Options** | Using [HaqumeiOptions](https://docs.rs/haqumei/latest/haqumei/options/struct.HaqumeiOptions.html), you can flexibly customize allophone phoneme label introduction, Unicode normalization, and reading behavior. |

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
pip install haqumei
```

#### Supported Platforms

Pre-built wheels are available for the following platforms:

| OS | Architecture |
|---|---|
| **Linux** | `x86_64`, `aarch64` |
| **macOS** | `aarch64` (e.g., Apple Silicon M1/M2/M3)|
| **Windows** | `x86_64` |


Pre-built wheels bundle the embedded dictionary and require no network access during installation.

If a wheel is unavailable for your platform, installation falls back to building from source, which requires a Rust toolchain. In that case, the dictionary is downloaded and embedded during the build (same as the Rust crate build process).

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
  let kana = haqumei.g2k(text)?;
  assert_eq!(kana, "コンニチワ、セカイ！");

  // Enable allophone resolution
  haqumei.options.use_allophones = true;

  let text = "執筆";

  // Get Word-Phoneme mapping with prosody information
  let mapping = haqumei.g2p_mapping_prosody(text)?;
  let shippitsu = &mapping[0];
  assert_eq!(shippitsu.word, "執筆");
  assert_eq!(shippitsu.pos, "名詞");
  assert_eq!(shippitsu.accent_nucleus, 0); // Heiban (flat) type

  println!("{:?}", shippitsu.phonemes);
  // Output:
  // [Phoneme {
  //     phoneme: Sh,
  //     pitch: Some(Low)
  // },
  // Phoneme {
  //     phoneme: I,
  //     pitch: Some(Low)
  // },
  // Phoneme {
  //     phoneme: ClP, // Allophone of the geminate consonant /cl/ (Phoneme::Cl): voiceless bilabial stop
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
> We do not remove pitch information from devoiced vowels or from contextual allophones introduced as dedicated phoneme labels, even in cases where no vocal-cord vibration (and thus no pitch) would be expected.
> As a G2P library, we believe it is better not to arbitrarily discard information, and to leave the decision of whether to drop pitch up to the user. (We shouldn't foreclose the option of keeping the pitch while converting back to a voiced vowel.)
>
> Please refer to the [documentation](https://docs.rs/haqumei/latest/haqumei/phoneme/index.html) for options other than `use_allophones` and more detailed information.

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
kana = haqumei.g2k(text)
print(f"Katakana reading: {kana}")
# -> Katakana reading: コンニチワ、セカイ！
```

## Advanced Features

### Word-Phoneme Mapping APIs

In Open JTalk (`pyopenjtalk`), unknown words are treated as `pau` (pauses), and Haqumei's standard `g2p` function follows this behavior.  
However, by using G2P functions whose names contain `mapping`, `detailed`, or `prosody`, you can detect unknown words and spaces themselves as `unk` and `sp` respectively.

> [!WARNING]
> Note that `sp` does not refer to raw space characters in the input, but rather the `"記号,空白"` (symbol, space) part-of-speech output by Mecab, which is normally ignored in `pyopenjtalk`. In particular, symbols that Mecab itself ignores (e.g., `\t`, `\n`) are not included in `sp`.
> This is why we describe the Word-Phoneme Mapping APIs as having "minimal loss relative to the input text": an exact match with the input text is not guaranteed. (Open JTalk also converts Latin characters to full-width.)
>
> A note on the phrase "mapping words ($\approx$ surface forms / dictionary entries) to phonemes":
> To begin with, there is no single, universally agreed-upon definition of a "word" in Japanese. In the context of Japanese morphological analysis, a dictionary's surface form is generally [treated](https://clrd.ninjal.ac.jp/unidic/glossary.html#morphological_analysis) as a "word," with grammatical function identified by analyzing the input string.
> During various stages of processing, Open JTalk merges `NjdFeature` entries carrying surface form, grammar, and accent information, and the HTS-format full-context label (which Haqumei [extends](https://github.com/o24s/haqumei/tree/main/haqumei-jlabel)) represents this abstractly as a [Word](https://docs.rs/haqumei-jlabel/latest/haqumei_jlabel/struct.Word.html).
> To represent substrings of the input text, using "surface form" is clearly inaccurate given the merging involved. Yet, we still needed a term for this split-but-processing-friendly unit, hence our deliberate use of the intentionally loose term "Word".

- **Known words**: Regular phoneme sequence (punctuation marks become `pau`).
- **Unknown words**: `unk`
- **Spaces, etc.**: `sp` (Space)

Using `g2p_mapping`, you can obtain the phoneme-to-word mapping along with flags indicating whether a word is unknown (`is_unknown`) and whether it would normally be ignored in the original pipeline (`is_ignored`).
In addition, using `g2p_mapping_detailed` allows you to retrieve not only the mapping but also part-of-speech information and accent details.
Additionally, an API such as `g2p_pairs` is available for cases where unknown-word information is not needed. However, like the traditional `g2p`, it loses a significant amount of input information and is not particularly recommended.

To obtain words and phonemes together with prosody information, `g2p_mapping_prosody` is useful.
See [here](#specification-of-g2p_mapping_prosody) for details.
That said, keep in mind that [`WordPhonemeProsody`](https://docs.rs/haqumei/latest/haqumei/word_phoneme/struct.WordPhonemeProsody.html), the list type returned by `g2p_mapping_prosody`, is essentially a superset of [`WordPhonemeDetail`](https://docs.rs/haqumei/latest/haqumei/word_phoneme/struct.WordPhonemeDetail.html) (returned by `g2p_mapping_detailed`), aside from Mecab's `features`.

In short, the amount of information provided by these APIs can be roughly ordered as:
`g2p_pairs` < `g2p_mapping` < `g2p_mapping_detailed` < `g2p_mapping_prosody`


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
| `?` | End of interrogative (？) | Sentence-medial |
| `!` | End of exclamation (Custom extension) | Sentence-medial |
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
  assert_eq!(phones.join(" "), "^ a [ o ] i # s o ] r a _ h i [ r o g a r u _ $");

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
| `word` | Word, a substring of the input text | `"空"` |
| `phonemes` | List consisting of phonemes, pitch information, and prosodic symbols (see below) | `[ProsodicPhoneme::Exclamatory]` |
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
| `Phoneme` | [Phoneme](https://docs.rs/haqumei/latest/haqumei/phoneme/enum.Phoneme.html) and its pitch (`High` / `Low`) | `a`, `a:0`, `H_a`, etc. |
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

Measured with [japanese-g2p-benchmark](https://github.com/o24s/japanese-g2p-benchmark).

The figures are the phoneme error rate (PER) on [prj-beatrice/jsut-label](https://github.com/prj-beatrice/jsut-label), a fork of `jsut-label` that annotates the basic5000 subset of the JSUT corpus, and the katakana error rate (KER) on [ROHAN](https://github.com/mmorise/rohan4600).

| G2P | jsut-label (PER) | ROHAN (KER) |
| :--- | ---: | ---: |
| pyopenjtalk 0.4.1 | 1.31% | 5.02% * |
| pyopenjtalk-plus 0.4.1.post9 | 1.09% | 1.60% |
| **haqumei 0.9.0** | **0.87%** | **0.81%** |

\* vanilla `pyopenjtalk` has no way to write long vowels and yotsugana in their
original spelling, so its output cannot be brought to ROHAN's notation. Most of that
gap is notation rather than misreading.

For each G2P, the options are chosen from the ones it offers to match the annotation convention of the corpus.  
Every combination is listed under "All option combinations" below.

<details>
<summary>All option combinations</summary>

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

`-` means it was not measured. The options that restore the original spelling
(`revert_long_vowels` / `revert_yotsugana`) only mean something for ROHAN, which
avoids the long vowel mark, so jsut-label is not run on the rows that enable them.

</details>

### Reproducing

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

## Benchmark

Here are the comparison results between `pyopenjtalk` (Baseline) and `haqumei` using approximately 318,000 characters of Japanese text.

Input data: [I Am a Cat (吾輩は猫である)](https://www.aozora.gr.jp/cards/000148/files/789_14547.html) 318,407 chars / 8,451 lines (Average 37 chars/line) (Ruby characters have been removed)

| Execution Mode | Execution Time (Mean) | Throughput | Speedup |
| :--- | :--- | :--- | :--- |
| **pyopenjtalk** (Baseline) | 2.358 s | 135k chars/s | 1.00x |
| **haqumei** (Default) | 1.303 s | 244k chars/s | **1.81x** |
| **haqumei** (`g2p_batch`, Default) | 0.098 s | 3.24M chars/s | 24.04x |

The detailed benchmark code can be found in [`haqumei-bench/pyopenjtalk`](https://github.com/o24s/haqumei/tree/main/haqumei-bench/pyopenjtalk).

Additionally, Rust-layer benchmarks for Haqumei using [`Criterion.rs`](https://crates.io/crates/criterion) can be run via `cargo bench` in the `haqumei-bench` crate. The comparison benchmark with `pyopenjtalk-plus` is located in [`haqumei-bench/pyopenjtalk-plus`](https://github.com/o24s/haqumei/tree/main/haqumei-bench/pyopenjtalk-plus).

### Performance Notes

- **Throughput Variation by Input Structure**:  
  Especially in the `*_batch` APIs, throughput (chars/s) tends to increase as the number of characters per line grows (up to approximately 4KB), compared with pyopenjtalk. This efficiency stems from an implementation that directly extracts labels from Open JTalk's internal structures, combined with minimal FFI overhead. When processing large volumes of text, it is most efficient to pass content in substantial chunks rather than splitting it into excessively short lines.
- **"Default" in the table**:  
  The configuration using `Haqumei::new` as is.

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

Haqumei uses a modified form of the dictionary included in [pyopenjtalk-plus](https://github.com/tsukumijima/pyopenjtalk-plus).

## License

Haqumei, excluding `haqumei-jlabel` and `haqumei-kanalizer`, is distributed under the terms of the Apache License 2.0.

Haqumei's logic includes an implementation ported from [tsukumijima/pyopenjtalk-plus](https://github.com/tsukumijima/pyopenjtalk-plus), and it likewise bundles the file stating the license of [r9y9/pyopenjtalk](https://github.com/r9y9/pyopenjtalk) that is placed in `tsukumijima/pyopenjtalk-plus`. Bundling it does not state the license of the code newly added on top of `tsukumijima/pyopenjtalk-plus`'s upstream.

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

- Bundled Kanji Reading Fallback Data
  - Origin: The data in `haqumei/data/unihan` is generated from the `kJapanese` field of the
    [Unihan Database](https://www.unicode.org/charts/unihan.html). It provides a per-character
    reading fallback, used to keep kanji missing from the dictionary from appearing verbatim in the
    kana output (see `HaqumeiOptions::read_unknown_kanji`).
  - License: UNICODE LICENSE V3. This license applies only to the data located in
    `haqumei/data/unihan`, and does not apply to the rest of this project. In accordance with
    redistribution requirements, the full text is included in `haqumei/data/unihan/LICENSE`.

- Bundled model for predicting the reading of 「何」
  - Origin: the ONNX models in `haqumei/yomi_model` are a conversion, by
    [tsukumijima/pyopenjtalk-plus](https://github.com/tsukumijima/pyopenjtalk-plus),
    of the 「何」 reading-prediction logic implemented in
    [n5-suzuki/pyopenjtalk](https://github.com/n5-suzuki/pyopenjtalk).
    They are embedded into the binary with `include_bytes!`.
  - License: the root of n5-suzuki/pyopenjtalk carries the MIT license notice of
    [r9y9/pyopenjtalk](https://github.com/r9y9/pyopenjtalk), the repository it was
    forked from. Meanwhile, no license statement covering the logic newly added on
    top of `pyopenjtalk`, or the converted models, could be found. The
    `LICENSE-pyopenjtalk` bundled with `haqumei` does not state the license of this
    model.

- Bundled `haqumei-jlabel` Source Code
  - Origin: The code contained in the `haqumei-jlabel` directory is based on the
    [jpreprocess/jlabel](https://github.com/jpreprocess/jlabel) repository.
  - License: The bundled `haqumei-jlabel` source code is licensed under the BSD 3-Clause License. This license applies
    only to the code located in `haqumei-jlabel`, and does not apply to the rest of this project. In accordance
    with redistribution requirements, the full text of the BSD 3-Clause License is included in
    `haqumei-jlabel/LICENSE`.

- Bundled `haqumei-kanalizer` Crate
  - Origin: The ONNX models bundled in `haqumei-kanalizer` are based on [VOICEVOX/kanalizer](https://github.com/VOICEVOX/kanalizer), with model weights from [VOICEVOX/kanalizer-model](https://huggingface.co/VOICEVOX/kanalizer-model) (converted via [o24s/kanalizer-onnx](https://github.com/o24s/kanalizer-onnx)).
  - License: The entire `haqumei-kanalizer` crate (both the Rust code and the bundled model weights) is licensed under the MIT License.

## Acknowledgements

The fundamental design and API of `haqumei` are inspired by `pyopenjtalk` and its highly improved fork, `pyopenjtalk-plus`.
In addition, some implementations are based on `jlabel` and `kanalizer` to improve usability and accuracy.

- pyopenjtalk: Copyright (c) 2018 Ryuichi Yamamoto
- pyopenjtalk-plus: Copyright (c) 2023 tsukumijima
- jlabel: Copyright (c) 2024 JPreprocess Team
- kanalizer: Copyright (c) 2025 VOICEVOX

We are deeply grateful to the authors and contributors of these foundational projects.
