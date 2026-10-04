# CJKVI 由来の字体対応データ

`variants.rs` は、未知の異体字を含む語を辞書の表記で再解析するための対応表である。

- 出典: [CJKVI 異体字データベース](https://kanji-database.sourceforge.net/variants/variants.html)
- 版: [cjkvi/cjkvi-variants の e4f1da248c9737a243f9930b5dc497cef5d5ae16](https://github.com/cjkvi/cjkvi-variants/tree/e4f1da248c9737a243f9930b5dc497cef5d5ae16)
- ライセンス: MIT。漢字データベースプロジェクトの[ライセンス表示](https://kanji-database.sourceforge.net/)では、異体字データを MIT の対象としている。全文は [LICENSE](LICENSE) に収めている。
- 著作権表示: 同じ版の `cjkvi-variants.txt` にある `Copyright (c) 2014 CJKVI Database` を保持している。

常用漢字・人名用漢字は正字へ、表外漢字は簡易慣用字体へ、残りは戸籍統一文字の
正字へ対応させている。対応先が複数ある字と循環は除き、1 字から 1 字への対応に
限定している。関連字・借用字・中国語の簡体字の表、異体字セレクタは扱っていない。
`崗→岡` と `巛→川` は、地名のコウや部首名のセンを、辞書にある別の固有名詞の
オカ・カワへ変えてしまうため、自動置換から除いている。
抽出規則と入力ファイルの SHA-256 は `scripts/build_cjkvi_variants.py` にある。

MIT ライセンスはこのディレクトリのデータに適用される。
`haqumei/dictionary` の辞書データには、その `COPYING` が適用される。
