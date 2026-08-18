#!/usr/bin/env python3
"""Unihan から `haqumei/data/unihan/readings.rs` を生成する。

辞書に無い漢字は Open JTalk が読みを決められず、`g2k` などでは表層形のまま
カナ列に混入する (`騸馬` -> `騸馬`)。`HaqumeiOptions::read_unknown_kanji` は
このデータを使って 1 文字ずつ読みを与え、混入を防ぐ。

出典とライセンスは `haqumei/data/unihan/README.md` を参照。

## 抽出の規則

Unihan の `kJapanese` は、カタカナで書かれた読み (音読み) とひらがなで書かれた
読み (訓読み) を並べて持つ (`㐀 = キュウ おか`)。ここでは

- カタカナの読みがあれば、その最初のものを採る
- 無ければひらがなの読みの最初のものをカタカナに変換して採る

辞書に無い漢字は漢語の複合語で現れることが多いため、音読みを優先している。

## 限界

1 字につき 1 読みしか持たない。文脈による読み分けも連濁も熟字訓も扱えない
ので、これは「読めないよりはまし」にするための近似である (`悪魔憑き` は
`アクマヒョウキ` になり、正しい `アクマツキ` にはならない)。

## 使い方

    curl -sSL -o Unihan.zip https://www.unicode.org/Public/UCD/latest/ucd/Unihan.zip
    unzip -o Unihan.zip Unihan_Readings.txt
    uv run scripts/build_unihan_readings.py --input Unihan_Readings.txt

生成物は `phf::phf_map!` の本体で、`include!` して使う。実行時の初期化を
避けるためにコンパイル時に完全ハッシュを作る。
"""

import argparse
import re
from pathlib import Path

KATAKANA = re.compile(r"[ァ-ヴー]+$")
HIRAGANA = re.compile(r"[ぁ-ゖー]+$")


def to_katakana(s):
    return "".join(chr(ord(c) + 0x60) if "ぁ" <= c <= "ゖ" else c for c in s)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--input", type=Path, required=True, help="Unihan_Readings.txt")
    ap.add_argument("--out", type=Path, default=Path("haqumei/data/unihan/readings.rs"))
    args = ap.parse_args()

    entries = {}
    for line in args.input.read_text(encoding="utf-8").splitlines():
        if line.startswith("#") or "\t" not in line:
            continue
        codepoint, field, value = line.split("\t", 2)
        if field != "kJapanese":
            continue
        entries[chr(int(codepoint, 0))] = value.split()

    rows = []
    for ch, values in sorted(entries.items()):
        on = [v for v in values if KATAKANA.fullmatch(v)]
        kun = [v for v in values if HIRAGANA.fullmatch(v)]
        reading = on[0] if on else (to_katakana(kun[0]) if kun else None)
        if reading:
            rows.append((ch, reading))

    body = ",\n".join(f"    '{ch}' => \"{reading}\"" for ch, reading in rows)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(
        "// このファイルは scripts/build_unihan_readings.py が生成する。手で編集しない。\n"
        "// 出典とライセンスは同じディレクトリの README.md / LICENSE を参照。\n"
        "::phf::phf_map! {\n" + body + ",\n}\n",
        encoding="utf-8",
    )
    print(f"{len(rows):,} 字 / {args.out.stat().st_size / 1024:.0f} KB -> {args.out}")


if __name__ == "__main__":
    main()
