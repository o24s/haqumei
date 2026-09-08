# haqumei-jpreprocess

Japanese text preprocessor for Text-to-Speech application.

This project is a rewrite of [OpenJTalk](http://open-jtalk.sourceforge.net/) in Rust language.

## Usage

Put the following in Cargo.toml

```toml
[dependencies]
haqumei-jpreprocess = "0.1.0"
```

It may be necessary to add
[haqumei-jpreprocess-njd](https://crates.io/crates/haqumei-jpreprocess-njd/) and/or
[haqumei-jpreprocess-jpcommon](https://crates.io/crates/haqumei-jpreprocess-jpcommon/)
if you want control over how njd and jpcommon are processed.

## Example

In this example, haqumei_jpreprocess loads a UTF-8 MeCab-compatible dictionary and
preprocesses a text into jpcommon labels.

```rs
use haqumei_jpreprocess::*;

let system = SystemDictionaryConfig::File(path).load()?;
let haqumei_jpreprocess = JPreprocess::from_tokenizer(system);

let jpcommon_label = haqumei_jpreprocess
    .extract_fullcontext("日本語文を解析し、音声合成エンジンに渡せる形式に変換します．")?;
assert_eq!(
  jpcommon_label[2].to_string(),
  concat!(
      "sil^n-i+h=o",
      "/A:-3+1+7",
      "/B:xx-xx_xx",
      "/C:02_xx+xx",
      "/D:02+xx_xx",
      "/E:xx_xx!xx_xx-xx",
      "/F:7_4#0_0@1_3|1_12",
      "/G:4_4%0_0_1",
      "/H:xx_xx",
      "/I:3-12@1+2&1-8|1+41",
      "/J:5_29",
      "/K:2+8-41"
  )
);
```

User dictionaries are loaded with `SystemDictionaryConfig::load_with_user_dictionaries`. The previous Lindera-specific `with_dictionaries` and `from_config` APIs have been removed.

## Copyrights

This software includes source code from:

- [OpenJTalk](http://open-jtalk.sourceforge.net/).
  Copyright (c) 2008-2016  Nagoya Institute of Technology Department of Computer Science
- [Lindera](https://github.com/lindera-morphology/lindera).
  Copyright (c) 2019 by the project authors

## License

BSD-3-Clause

## API Reference

- [haqumei-jpreprocess](https://docs.rs/haqumei-jpreprocess)
