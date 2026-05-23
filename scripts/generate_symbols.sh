#!/usr/bin/env bash

DICT_DIR="${1:-haqumei/dictionary}"

if [ ! -d "$DICT_DIR" ]; then
    echo "Error: Directory '$DICT_DIR' not found." >&2
    exit 1
fi

rg -I -N ",記号," "$DICT_DIR" | awk -F',' '
{
    # $1: 表層形, $4: 単語コスト
    surface = $1
    cost = $4 + 0

    feature = $5
    for (i = 6; i <= NF; i++) {
        feature = feature "," $i
    }

    if (!(surface in min_cost) || cost < min_cost[surface]) {
        min_cost[surface] = cost
        best_feature[surface] = feature
    }
}
END {
    for (s in min_cost) {
        esc_s = s
        gsub(/\\/, "\\\\", esc_s)
        gsub(/"/, "\\\"", esc_s)

        esc_feat = best_feature[s]
        gsub(/\\/, "\\\\", esc_feat)
        gsub(/"/, "\\\"", esc_feat)

        printf "        \"%s\" => Some(\"%s\"),\n", esc_s, esc_feat
    }
}' | sort | awk '
BEGIN {
    print "pub(crate) fn get_known_symbol_feature(c: &str) -> Option<&'\''static str> {"
    print "    match c {"
}
{ print $0 }
END {
    print "        _ => None,"
    print "    }"
    print "}"
}'