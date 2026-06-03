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

### Prosody Output

By specifying `--mode prosody`, it outputs a phoneme sequence that includes accent phrase boundaries and pitch (high/low) markers.

```bash
$ haqumei-cli "青い空が、好きだ！" --mode prosody
^ a [ o ] i # s o ] r a g a _ s U [ k i ] d a ! $

# Prefix format (--prosody-format prefix)
$ haqumei-cli "青い空" --mode prosody --prosody-format prefix
^ L_a H_o L_i # H_s H_o L_r L_a $
```

### JSON Lines Output

Supports structured JSON output by specifying `--format json`.

```bash
$ haqumei-cli "テスト" --mode mapping-detailed --format json
[{"word":"テスト","phonemes":["t","e","s","U","t","o"],"features":["テスト","名詞","サ変接続","*","*","*","*","テスト","テスト","テスト","1/3","C1"],"pos":"名詞","pos_group1":"サ変接続","pos_group2":"*","pos_group3":"*","ctype":"*","cform":"*","orig":"テスト","read":"テスト","pron":"テス’ト","accent_nucleus":1,"mora_count":3,"chain_rule":"C1","chain_flag":-1,"is_unknown":false,"is_ignored":false}]
```

## Options

### Output Modes (`--mode` / `-m`)

`haqumei-cli` supports various output modes:

- `g2p` (default): Flat phoneme sequence.
- `prosody`: Phoneme sequence with prosodic symbols (accents, pitch, boundaries).
- `g2p-detailed`: Detailed phoneme sequence (symbols converted to `sp`, `unk`, etc.).
- `kana`: Katakana sequence.
- `kana-per-word`: Katakana sequence separated by word (morpheme).
- `per-word`: Phoneme list grouped by word.
- `pairs`: Word-to-phoneme mapping (`word: phonemes`).
- `mapping`: Detailed morpheme mapping including unknown word status.
- `mapping-detailed`: Further detailed mapping including POS, pronunciation, accent nucleus, and mora count.
- `mapping-prosody`: Mapping including detailed prosody information per morpheme.
- `fullcontext`: Structured extended HTS full-context labels.
- `fullcontext-string`: Extended HTS full-context label strings.

### Output Formats (`--format` / `-f`)

- `text` (default): Human-readable text format.
- `json`: JSON format.

### Prosody Formats (`--prosody-format`)

Valid when `--mode` is `prosody` or `mapping-prosody`.

- `default` (default): Uses `[` (pitch up) and `]` (pitch down) symbols.
- `prefix`: Adds `H_` (high) or `L_` (low) as a prefix to each phoneme.
- `numeric`: Adds `:1` (high) or `:0` (low) as a suffix to each phoneme.

### Text Processing & Dictionary Options

You can finely control the behavior of `haqumei`'s text processing and toggle specific rules.

- `--dict-dir <DIR>`: Path to a custom dictionary directory.
- `--user-dict <FILE>`: Path to a user dictionary (.csv).
- `--use-unidic-yomi`: Use Unidic to correct Kanji readings (downloads the dictionary on the first run).
- `--normalize-unicode <none|nfc|nfkc>`: Specify the Unicode normalization method.
- `--normalize-iu <iu|yuu|kanji-iu|kanji-yuu>`: Specify the pronunciation normalization for the verb "言う" (iu / yuu).
- `--use-read-as-pron`: Use reading (`read`) instead of pronunciation (`pron`), disabling automatic long vowel conversions.
- `--revert-long-vowels`: Revert automatically lengthened pronunciations back to readings faithful to the original text.
- `--revert-yotsugana`: Revert Yotsugana (ヅ/ヂ) to their original text representation.

**Rule Disabling Flags** (all enabled by default):
- `--no-modify-filler-accent`: Disable accent correction for fillers.
- `--no-predict-nani`: Disable Nani Predictor reading corrections for "何".
- `--no-retreat-acc-nuc`: Disable the rule that shifts the accent nucleus one mora back under certain conditions.
- `--no-modify-acc-after-chaining`: Disable accent shifting before the POS "特殊・マス".
- `--no-process-odoriji`: Disable expansion of iteration marks (々, ヽ, ヾ).

For more details, please check `--help`:
```bash
haqumei-cli --help
```
