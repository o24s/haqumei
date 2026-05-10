# haqumei-jlabel

これは [jlabel](https://github.com/jpreprocess/jlabel) をフォークしたクレートです。

拡張されたHTSスタイルのコンテキストラベルと、文字列間のパーサー/シリアライザー用の構造体を提供します。

`haqumei` は [korguchi](https://github.com/korguchi)氏 の拡張した[感嘆符をフルコンテキストラベルに追加する改善](https://github.com/o24s/haqumei/commit/4f41b847be617bff1adc96852694c784eb6f0476)を含む [tsukumijima](https://github.com/tsukumijima)氏 の [Open JTalk フォーク](https://github.com/tsukumijima/open_jtalk) をバインディングしています。
`haqumei` はフルコンテキストラベルを扱うため、それに合わせた構造体の拡張を行っています。

`haqumei-jlabel` における `jlabel` からの変更部分は、オリジナルと同様に BSD 3-Clause License の下で公開します。

---
以下はオリジナルの README の ## Credit セクションに対応しています。


## Credits

@cm-ayf さんがコードの大部分を書いてくださいました．
この場を借りて感謝申し上げます．

また，フルコンテキストラベルや「質問」の仕様については，
[hts_engine API](https://hts-engine.sourceforge.net)，
[NIT ATR503 M001](http://hts.sp.nitech.ac.jp/?Download#u879c944)
を参考にしています．

## License

BSD 3-Clause License
