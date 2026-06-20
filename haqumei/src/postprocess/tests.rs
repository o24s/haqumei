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
