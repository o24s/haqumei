# haqumei-cli

A command-line interface for [haqumei](https://github.com/o24s/haqumei), a G2P (Grapheme-to-Phoneme) and text processing library for Japanese.

## Installation

```bash
cargo install haqumei-cli
```

## Usage

### REPL

Run without arguments to enter the interactive mode.
```bash
$ haqumei-cli
Enter text to process (Ctrl+C or Ctrl+D to exit):
> 今日はいい天気ですね。
ky o o w a i i t e N k i d e s U n e
```

### One-liner / Pipeline Processing

You can specify the text directly as an argument, or pass the text via standard input.

```bash
$ haqumei-cli "吾輩は猫である" --mode kana-per-word
ワガハイ ワ ネコ デ アル

$ echo "吾輩は猫である" | haqumei-cli --mode kana-per-word
ワガハイ ワ ネコ デ アル
```

### File Processing

Read from an input file and write to an output file.

```bash
haqumei-cli --input input.txt --output output.txt --mode g2p
```

### JSON Lines Output

Supports structured JSON output.

```bash
$ haqumei-cli "テスト" --mode mapping-detailed --format json
[{"word":"テスト","phonemes":["t","e","s","U","t","o"],"features":["テスト","名詞","サ変接続","*","*","*","*","テスト","テスト","テスト","1/3","C1"],"pos":"名詞","pos_group1":"サ変接続","pos_group2":"*","pos_group3":"*","ctype":"*","cform":"*","orig":"テスト","read":"テスト","pron":"テス’ト","accent_nucleus":1,"mora_count":3,"chain_rule":"C1","chain_flag":-1,"is_unknown":false,"is_ignored":false}]
```

## Modes (`--mode`)

`haqumei-cli` supports various output modes:
- `g2p` (default): Flat phoneme sequence.
- `g2p-detailed`: Detailed phoneme sequence (symbols converted to `sp`, `unk`, etc.).
- `kana`: Katakana sequence.
- `kana-per-word`: Katakana sequence separated by word.
- `per-word`: Phoneme list grouped by word.
- `pairs`: Word to phoneme mapping.
- `mapping`: Detailed mapping including unknown word status.
- `mapping-detailed`: Includes POS, pronunciation, accent nucleus, and mora count.
- `fullcontext`: Full-context labels for TTS.

## Options

Run `--help` to see all available configuration options (e.g., dictionary paths, unicode normalization, long vowel rules).
```bash
haqumei-cli --help
```
