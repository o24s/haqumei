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

### プロソディ記号の出力

`--mode prosody` を指定すると、アクセント句境界やピッチの上下を含めた音素列を出力します。

```bash
$ haqumei-cli "青い空が、好きだ！" --mode prosody
^ a [ o ] i # s o ] r a g a _ s U [ k i ] d a ! $

# プレフィックス形式 (--prosody-format prefix)
$ haqumei-cli "青い空" --mode prosody --prosody-format prefix
^ L_a H_o L_i # H_s H_o L_r L_a $
```

### JSON Lines 形式での出力

`--format json` を指定することで、構造化された JSON 形式での出力に対応しています。

```bash
$ haqumei-cli "テスト" --mode mapping-detailed --format json
[{"word":"テスト","phonemes":["t","e","s","U","t","o"],"features":["テスト","名詞","サ変接続","*","*","*","*","テスト","テスト","テスト","1/3","C1"],"pos":"名詞","pos_group1":"サ変接続","pos_group2":"*","pos_group3":"*","ctype":"*","cform":"*","orig":"テスト","read":"テスト","pron":"テス’ト","accent_nucleus":1,"mora_count":3,"chain_rule":"C1","chain_flag":-1,"is_unknown":false,"is_ignored":false}]
```

## オプション

### 出力モード (`--mode` / `-m`)

`haqumei-cli` は様々な出力モードをサポートしています。

- `g2p` (デフォルト): 音素列 (フラット)
- `prosody`: プロソディ記号付き音素列
- `g2p-detailed`: 詳細な音素列 (記号等を `sp` や `unk` に変換)
- `kana`: カタカナ
- `kana-per-word`: 単語 (形態素) ごとに分割されたカタカナ
- `per-word`: 単語ごとの音素リスト
- `pairs`: 形態素ごとの音素マッピング (`word: phonemes`) (廃止予定、`mapping` を使う)
- `mapping`: 未知語情報などを含めた形態素マッピング
- `mapping-detailed`: 品詞、発音、アクセント核、モーラ数などを含めたさらに詳細なマッピング
- `mapping-prosody`: 形態素ごとの詳細なプロソディ情報を含めたマッピング
- `candidates`: 読みの候補。分岐点を `# 開始..終了<TAB>表層形<TAB>発音(コスト差) / ...` で並べ、続けて候補を 1 行ずつ `コスト差<TAB>音素` で出す
- `fullcontext`: 構造化された拡張 HTSフルコンテキストラベル
- `fullcontext-string`: 拡張 HTSフルコンテキストラベル文字列

### 出力フォーマット (`--format` / `-f`)

- `text` (デフォルト): 人間が読みやすいテキスト形式
- `json`: JSON 形式

### プロソディ記号オプション (`--prosody-format`)

`--mode` が `prosody` または `mapping-prosody` の場合に有効です。

- `default` (デフォルト): `[` (ピッチ上昇) や `]` (ピッチ下降) を用いた形式
- `prefix`: 音素のプレフィックスとして `H_` (高) や `L_` (低) を付与
- `numeric`: 音素のサフィックスとして `:1` (高) や `:0` (低) を付与

### 言語処理・辞書オプション

`haqumei` のテキスト処理の挙動や、ルールの有効/無効を細かく制御できます。

- `--dict-dir <DIR>`: カスタム辞書のディレクトリパス
- `--user-dict <FILE>`: ユーザー辞書 (.csv) のパス
- `--normalize-unicode <none|nfc|nfkc>`: Unicode正規化の方法を指定
- `--normalize-iu <iu|yuu|kanji-iu|kanji-yuu>`: 「言う」の発音正規化方式を指定
- `--use-read-as-pron`: 読み (`read`) を発音 (`pron`) の代わりに使用し、長音の自動変換などを無効化する
- `--revert-long-vowels`: 自動的に長音化された発音を、元のテキストに忠実な読みに復元する
- `--revert-yotsugana`: 四つ仮名 (ヅ・ヂ) を元のテキスト通りの表記に復元する

**ルールの無効化フラグ** (デフォルトはすべて有効です)
- `--no-modify-filler-accent`: フィラーのアクセント修正を無効にする
- `--no-predict-nani`: Nani Predictor による「何」の読み修正を無効にする
- `--no-retreat-acc-nuc`: アクセント核を1つ前のモーラにずらすルールを無効にする
- `--no-modify-acc-after-chaining`: 品詞「特殊・マス」前のアクセント移動を無効にする
- `--no-process-odoriji`: 踊り字 (々, ヽ, ヾ) の展開を無効にする

その他の詳細については `--help` で確認できます。
```bash
haqumei-cli --help
```
