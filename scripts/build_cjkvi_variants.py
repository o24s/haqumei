#!/usr/bin/env python3
"""CJKVI の日本語字体表から、未知字の再解析に使う対応表を生成する。

入力は cjkvi/cjkvi-variants の COMMIT に固定する。常用漢字・人名用漢字は正字へ、
表外漢字は簡易慣用字体へ、残りは戸籍統一文字の正字へ対応させる。
先に選んだ正字を後の表で逆向きに変換しない。対応先が複数ある字と循環は除く。
異体字セレクタを伴う列、関連字、借用字、簡体字の表は使っていない。

    python3 scripts/build_cjkvi_variants.py --source-dir /path/to/cjkvi-variants
"""

import argparse
import hashlib
from collections import defaultdict
from pathlib import Path

COMMIT = "e4f1da248c9737a243f9930b5dc497cef5d5ae16"
SOURCES = {
    "joyo-variants.txt": "da0666d333c330c07efa2184cdc596bb0de245f65b775c0c805b2b11e6b13c4b",
    "jinmei-variants.txt": "a4dbe76597ca64e6eb2e2b3ec41bab037cb9f443151d02d97a8edfbd9befc6bb",
    "hyogai-variants.txt": "1fb13dfcd96ca212096621cdef5bd27f270ebf8f1fc5834425033be248ce6b3e",
    "koseki-variants.txt": "c33703056001b7b3a01ff860695017ef9cdad0407ee68217fffe538ad68ca032",
}

# 字体の対応があっても、日本語の辞書読みへ置き換えられない組がある。
# 崗のコウは岡を含む地名でオカに、部首名の巛のセンは川を含む人名でカワになる。
EXCLUDED_PAIRS = {("崗", "岡"), ("巛", "川")}


def build(source_dir):
    mapping = {}
    preferred = set()
    for filename, digest in SOURCES.items():
        data = (source_dir / filename).read_bytes()
        if hashlib.sha256(data).hexdigest() != digest:
            raise ValueError(f"{filename}: {COMMIT} の SHA-256 と一致しません")
        pairs = defaultdict(set)
        for line in data.decode("utf-8").splitlines():
            fields = line.split(",")
            if len(fields) != 3:
                continue
            proper, relation, variant = fields
            if len(proper) != 1 or len(variant) != 1 or not relation.endswith("/variant"):
                continue
            if filename == "hyogai-variants.txt":
                proper, variant = variant, proper
            if (variant, proper) in EXCLUDED_PAIRS:
                continue
            pairs[variant].add(proper)
        accepted = {
            variant: next(iter(targets))
            for variant, targets in pairs.items()
            if len(targets) == 1 and variant not in mapping and variant not in preferred
        }
        mapping.update(accepted)
        preferred.update(accepted.values())

    resolved = {}
    for variant in mapping:
        seen = {variant}
        proper = mapping[variant]
        while proper in mapping and proper not in seen:
            seen.add(proper)
            proper = mapping[proper]
        if proper not in seen:
            resolved[variant] = proper
    return resolved


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-dir", type=Path, required=True)
    parser.add_argument("--out", type=Path, default=Path("haqumei/data/cjkvi/variants.rs"))
    args = parser.parse_args()
    mapping = build(args.source_dir)
    lines = [
        "// scripts/build_cjkvi_variants.py による生成物。手で編集しない。",
        f"// CJKVI {COMMIT}; MIT License (同じディレクトリの LICENSE)。",
        "phf::phf_map! {",
        *(f"    '{variant}' => '{proper}'," for variant, proper in sorted(mapping.items())),
        "}",
    ]
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"{len(mapping)} 字: {args.out}")


if __name__ == "__main__":
    main()
