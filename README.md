<div align="center">
  <h1>Haqumei</h1>
  <p>
    Haqumei is a Japanese Grapheme-to-Phoneme (G2P) library implemented in Rust.
  </p>
  <p>
    English | <a href="https://github.com/stellanomia/haqumei/blob/main/README.ja.md">日本語</a>
  </p>
  <p>
    <a href="LICENSE"><img src="https://img.shields.io/badge/License-Apache--2.0-blue.svg" alt="License: Apache-2.0"></a>
  </p>
</div>

## Install

### Rust

```bash
cargo add haqumei --git "https://github.com/stellanomia/haqumei.git"
```

### Python

```bash
pip install "git+https://github.com/stellanomia/haqumei.git#subdirectory=haqumei-python"
```

## Features

- **Performance:** Achieves fast G2P through a native Rust implementation and by incorporating several improvements from [`pyopenjtalk-plus`](https://github.com/tsukumijima/pyopenjtalk-plus).
- **Output Formats:** Provides results in various formats, including a simple phoneme list (`g2p`), a detailed list with unknown word information (`g2p_detailed`), and a list split by words (`g2p_per_word`).
- **Diverse Analysis Information:** Capable of retrieving detailed mapping information linking morphological analysis results with phonemes (`g2p_mapping`, `g2p_mapping_detailed`) and full-context labels (`extract_fullcontext`), similar to `pyopenjtalk`.

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

## Advanced Usage

### Getting Phoneme Mapping with Original Words

Haqumei implements `g2p_mapping`, enabling accurate mapping between phonemes and their corresponding words, which was difficult to achieve with the original Open JTalk.
This was implemented by traversing the `JPCommon` structure and tracking the pointers of each phoneme to its respective word.

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

### Detailed G2P Output

In Open JTalk (and pyopenjtalk), unknown words are treated as pauses (`、`), and Haqumei's `g2p` function follows this behavior.
By using functions with the `_detailed` suffix, you can detect ignored unknown words and spaces as `unk` and `sp` respectively.

Note that `sp` represents `"記号,空白"` (symbol, space) output by Mecab that would originally be ignored in `pyopenjtalk`, not the spaces you input directly. Therefore, symbols that Mecab itself ignores (e.g., `\t`, `\n`) are not included in `sp`.

- **Known words:** Regular phoneme sequence (pauses like `、` become `pau`).
- **Unknown words:** `unk`
- **Spaces, etc.:** `sp` (Space)

Using `g2p_mapping_detailed`, you can obtain the mapping of phonemes to original words, along with information on whether a word is unknown (`is_unknown`) and whether it would be ignored in the original pipeline (`is_ignored`).

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