use super::*;

#[test]
fn test_modify_acc_after_chaining_mut() {
    let mut features = [
        NjdFeature {
            string: "参り".to_string(),
            pos: "動詞".to_string(),
            pos_group1: "自立".to_string(),
            pos_group2: "*".to_string(),
            pos_group3: "*".to_string(),
            ctype: "五段・ラ行".to_string(),
            cform: "連用形".to_string(),
            orig: "参る".to_string(),
            read: "マイリ".to_string(),
            pron: "マイリ".to_string(),
            acc: 1,
            mora_size: 3,
            chain_rule: "*".to_string(),
            chain_flag: -1,
        },
        NjdFeature {
            string: "ます".to_string(),
            pos: "助動詞".to_string(),
            pos_group1: "*".to_string(),
            pos_group2: "*".to_string(),
            pos_group3: "*".to_string(),
            ctype: "特殊・マス".to_string(),
            cform: "基本形".to_string(),
            orig: "ます".to_string(),
            read: "マス".to_string(),
            pron: "マス’".to_string(),
            acc: 1,
            mora_size: 2,
            chain_rule: "動詞%F2@1/助詞%F2@1".to_string(),
            chain_flag: 1,
        },
    ];

    modify_acc_after_chaining(&mut features);

    let 参り = features.first().unwrap();
    assert_eq!(参り.acc, 4);
}

fn to_fullwidth(s: &str) -> Vec<char> {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' => char::from_u32(c as u32 + 0xFEE0).unwrap(),
            _ => c,
        })
        .collect()
}

#[test]
fn test_should_use_kanalizer_1char() {
    // 1文字の単語は false
    let words = ["A", "I", "a", "x", "Z"];
    for w in words {
        let chars = to_fullwidth(w);
        assert!(!should_use_kanalizer(&chars), "Failed on 1-char: {}", w);
    }
}

#[test]
fn test_should_use_kanalizer_2chars() {
    // true になるべき一般的な2文字英単語 (V+C, C+V, V+V)
    let words_true = [
        "it", "is", "he", "we", "go", "to", "do", "no", "my", "by", "on", "in", "am", "an", "UI",
    ];
    for w in words_true {
        let chars = to_fullwidth(w);
        assert!(should_use_kanalizer(&chars), "Failed on 2-char word: {}", w);
    }

    // false になるべきアクロニムや発音しない2文字
    let words_false = ["PC", "PR", "CD", "DJ", "TV", "VR", "XR", "HP", "JS"];
    for w in words_false {
        let chars = to_fullwidth(w);
        assert!(
            !should_use_kanalizer(&chars),
            "Failed on 2-char acronym: {}",
            w
        );
    }
}

#[rustfmt::skip]
#[test]
fn test_should_use_kanalizer_3chars() {
    // 発音可能な3文字英単語
    let words_true = [
        // CVC: 常に許容される
        "cat", "dog", "pen", "mac", "bug", "run", "how", "new",
        // CCV: th, sh, pr, wh などは許容される組み合わせ
        "the", "she", "pro", "who", "why",
        // CVV: ou, ee, oo, ay などは許容される組み合わせ
        "you", "see", "too", "day", "way",
        // VCC: nd, ct, sk, ff などは許容される組み合わせ
        "and", "act", "ask", "add", "off", "ill",
        // VCV: 常に許容される
        "use", "are", "one", "ice", "age",
        // 特殊: 母音なしだが発音可能 (continuantの連続)
        "hmm", "shh",
    ];
    for w in words_true {
        let chars = to_fullwidth(w);
        assert!(should_use_kanalizer(&chars), "Failed on 3-char word: {}", w);
    }

    // 発音不能なアクロニムの3文字
    let words_false = [
        "USB", "FBI", "CPU", "GPU", "SQL", "AWS", "KGB", "BBC", "CNN", "npm",
    ];
    for w in words_false {
        let chars = to_fullwidth(w);
        assert!(
            !should_use_kanalizer(&chars),
            "Failed on 3-char acronym: {}",
            w
        );
    }
}

#[test]
fn test_should_use_kanalizer_n_chars() {
    // 4文字以上の一般的な英単語
    let words_true = [
        "This", "that", "apple", "hello", "world", "good", "morning", "GitHub", "Rust",
    ];
    for w in words_true {
        let chars = to_fullwidth(w);
        assert!(should_use_kanalizer(&chars), "Failed on n-char word: {}", w);
    }

    // 4文字以上のアクロニム
    let words_false = ["HTML", "HTTP", "HTTPS", "SMTP", "JDBC"];
    for w in words_false {
        let chars = to_fullwidth(w);
        assert!(
            !should_use_kanalizer(&chars),
            "Failed on n-char acronym: {}",
            w
        );
    }
}

#[test]
fn test_realistic_sentences() {
    let sentence1 = vec![
        ("This", true),
        ("is", true),
        ("a", false), // ただし `modify_english_words` で補正される
        ("pen", true),
    ];

    let sentence2 = vec![
        ("I", false),
        ("use", true),
        ("a", false),
        ("Mac", true), // CVC
        ("PC", false),
    ];

    let sentence3 = vec![
        ("The", true), // CCV
        ("USB", false),
        ("is", true),
        ("broken", true),
    ];

    for (word, expected) in sentence1.into_iter().chain(sentence2).chain(sentence3) {
        let chars = to_fullwidth(word);
        assert_eq!(
            should_use_kanalizer(&chars),
            expected,
            "Failed in sentence context: '{}'",
            word
        );
    }
}

#[test]
fn test_restore_loanword_kana() {
    fn f(surface: &str, read: &str, pron: &str) -> NjdFeature {
        NjdFeature {
            string: surface.to_string(),
            pos: "名詞".to_string(),
            pos_group1: "固有名詞".to_string(),
            pos_group2: "*".to_string(),
            pos_group3: "*".to_string(),
            ctype: "*".to_string(),
            cform: "*".to_string(),
            orig: surface.to_string(),
            read: read.to_string(),
            pron: pron.to_string(),
            acc: 0,
            mora_size: count_mora(pron) as i32,
            chain_rule: "*".to_string(),
            chain_flag: -1,
        }
    }

    // 辞書が別の仮名に置き換えているものを戻す
    for (surface, pron, want) in [
        ("ヴィクトリーヌ", "ビク’トリーヌ", "ヴィク’トリーヌ"),
        ("アイシュヴァルヤ", "アイシュバルヤ", "アイシュヴァルヤ"),
        ("テュルク", "チュルク", "テュルク"),
        ("アクスィス", "アクシス", "アクスィス"),
    ] {
        let mut v = [f(surface, pron, pron)];
        restore_loanword_kana(&mut v);
        assert_eq!(v[0].pron, want, "入力: {surface}");
    }

    // 置き換えた形のほうが定着している語は触らない。
    // ホンジュラス は デュ -> ジュ で、復元表に載せていない
    for (surface, pron) in [
        ("ホンデュラス", "ホンジュラス"),
        ("バースディ", "バースデイ"),
        ("テイスト", "テイスト"),
        ("キウイ", "キウイ"),
        // クァ 行は復元表に載せていない。ウルグアイ / グアテマラ /
        // クオリティ のように、大書きの形が日本語として定着している
        ("ウルグァイ", "ウルグアイ"),
    ] {
        let mut v = [f(surface, pron, pron)];
        restore_loanword_kana(&mut v);
        assert_eq!(v[0].pron, pron, "入力: {surface}");
    }

    // 表層形と発音が途中で食い違う語は、全体を諦める
    let mut v = [f("エヌ・エイチ・ヴィ", "エヌエイチブイ", "エヌエイチブイ")];
    restore_loanword_kana(&mut v);
    assert_eq!(v[0].pron, "エヌエイチブイ");
}

/// 「等」は直前の品詞細分類 1 で ラ / トー / ナド に分かれる。
///
/// 辞書の版によって「等」自体が `名詞-一般` にも `名詞-接尾` にもなり、
/// 「こと」が `非自立` か `一般` かも変わる。埋め込み辞書に左右されないよう、
/// 規則そのものに直接あてる。
#[test]
fn test_modify_context_reading_nado() {
    fn f(string: &str, pos: &str, pos_group1: &str, pron: &str) -> NjdFeature {
        NjdFeature {
            string: string.to_string(),
            pos: pos.to_string(),
            pos_group1: pos_group1.to_string(),
            pos_group2: "*".to_string(),
            pos_group3: "*".to_string(),
            ctype: "*".to_string(),
            cform: "*".to_string(),
            orig: string.to_string(),
            read: pron.to_string(),
            pron: pron.to_string(),
            acc: 1,
            mora_size: count_mora(pron) as i32,
            chain_rule: "*".to_string(),
            chain_flag: -1,
        }
    }

    for (pos, pos_group1, expected) in [
        // 代名詞の後は ラ
        ("名詞", "代名詞", "ラ"),
        // 自立した名詞の後は トー
        ("名詞", "一般", "トー"),
        ("名詞", "サ変接続", "トー"),
        ("名詞", "固有名詞", "トー"),
        ("名詞", "接尾", "トー"),
        // 非自立名詞は句を名詞化しているだけなので ナド のまま
        ("名詞", "非自立", "ナド"),
        // 活用語の後も ナド のまま
        ("動詞", "自立", "ナド"),
        ("助動詞", "*", "ナド"),
        ("形容詞", "自立", "ナド"),
        // 数詞は 一等 / 三等 の助数詞なので触らない
        ("名詞", "数", "ナド"),
    ] {
        let mut features = [
            f("前", pos, pos_group1, "マエ"),
            f("等", "名詞", "一般", "ナド"),
        ];
        modify_context_reading(&mut features);

        let label = format!("{pos}-{pos_group1}");
        assert_eq!(features[1].pron, expected, "直前が {label}");
        assert_eq!(features[1].read, expected, "直前が {label}");
        // ナド は 2 モーラ、ラ は 1 モーラなので数え直されていること
        assert_eq!(
            features[1].mora_size,
            count_mora(expected) as i32,
            "直前が {label}"
        );
    }

    // 文頭の「等」は直前が無いので触らない
    let mut features = [f("等", "名詞", "一般", "ナド")];
    modify_context_reading(&mut features);
    assert_eq!(features[0].pron, "ナド");

    // 辞書の版によっては「等」が 名詞-接尾 として現れる。規則の対象を
    // 名詞-一般 に限ると適用されなくなるので、接尾でも適用されること
    let mut features = [
        f("これ", "名詞", "代名詞", "コレ"),
        f("等", "名詞", "接尾", "トー"),
    ];
    modify_context_reading(&mut features);
    assert_eq!(features[1].pron, "ラ");
}
