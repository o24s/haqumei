# haqumei-cli

日本語向けの G2P (Grapheme-to-Phoneme) ライブラリである [haqumei](https://github.com/o24s/haqumei) のコマンドラインインターフェースです。

## インストール

```bash
cargo install haqumei-cli
```

## 使い方

### REPL

引数なしで実行すると対話モードが起動します。

```bash
$ haqumei-cli
Enter text to process (Ctrl+C or Ctrl+D to exit):
> 今日はいい天気ですね。
ky o o w a i i t e N k i d e s U n e
```

### パイプライン処理

引数で直接テキストを指定するか、標準入力経由でテキストを渡すことができます。

```bash
$ haqumei-cli "吾輩は猫である" --mode kana-per-word
ワガハイ ワ ネコ デ アル

$ echo "吾輩は猫である" | haqumei-cli --mode kana-per-word
ワガハイ ワ ネコ デ アル
```

### ファイル処理

入力ファイルから読み込み、結果を出力ファイルへ書き込みます。

```bash
haqumei-cli --input input.txt --output output.txt --mode g2p
```

### JSON Lines 形式での出力

構造化された JSON 形式での出力に対応しています。

```bash
$ haqumei-cli "テスト" --mode mapping-detailed --format json
[{"word":"テスト","phonemes":["t","e","s","U","t","o"],"features":["テスト","名詞","サ変接続","*","*","*","*","テスト","テスト","テスト","1/3","C1"],"pos":"名詞","pos_group1":"サ変接続","pos_group2":"*","pos_group3":"*","ctype":"*","cform":"*","orig":"テスト","read":"テスト","pron":"テス’ト","accent_nucleus":1,"mora_count":3,"chain_rule":"C1","chain_flag":-1,"is_unknown":false,"is_ignored":false}]
```

## モード (`--mode`)

`haqumei-cli` は様々な出力モードをサポートしています。
- `g2p` (デフォルト): 音素列（フラット）
- `g2p-detailed`: 詳細な音素列（記号等を `sp` や `unk` に変換）
- `kana`: カタカナ
- `kana-per-word`: 単語（形態素）ごとに分割されたカタカナ
- `per-word`: 単語ごとの音素リスト
- `pairs`: 形態素ごとの音素マッピング
- `mapping`: 未知語情報などを含めたマッピング
- `mapping-detailed`: 品詞、発音、アクセント核、モーラ数などを含めたさらに詳細なマッピング
- `fullcontext`: 音声合成 (TTS) 用のフルコンテキストラベル

## オプション

`--help` を実行すると、利用可能なすべての設定オプションを確認できます。
```bash
haqumei-cli --help
```
