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

#[cfg(feature = "unidic-yomi")]
#[test]
fn test_split_unidic_feature_respects_quotes() {
    fn fields(feature: &str) -> Vec<&str> {
        split_unidic_feature(feature)
            .into_iter()
            .map(|r| &feature[r])
            .collect()
    }

    // 引用符を含まない通常のケース
    assert_eq!(
        fields("名詞,普通名詞,一般,*"),
        ["名詞", "普通名詞", "一般", "*"]
    );

    // fConType の値がカンマを含み引用符で囲まれている場合、
    // 1フィールドとして扱い、以降のフィールド番号がずれないこと
    let feature = "名詞,普通名詞,助数詞可能,*,*,*,トオリ,通り,通,ドーリ,通,ドーリ,和,\
                   ト濁,濁音形,*,*,*,\"B1WB2WB3WB4WBjS,B1WB2WB8SjS\",体,ドオリ,ドオリ,\
                   ドオリ,トオリ,3,C2,*,7202643210551808,26203";
    let f = fields(feature);
    assert_eq!(f.len(), 29);
    assert_eq!(f[9], "ドーリ", "pron");
    assert_eq!(
        f[18], "B1WB2WB3WB4WBjS,B1WB2WB8SjS",
        "fConType は囲みの引用符を含まない"
    );
    assert_eq!(f[19], "体", "type");
    assert_eq!(f[20], "ドオリ", "kana");
    assert_eq!(f[24], "3", "aType");
    assert_eq!(f[25], "C2", "aConType");
    assert_eq!(f[28], "26203", "lemma_id");

    // aType 側がカンマを含むケース (例: 形容詞「多い」の "1,2")
    let feature = "形容詞,一般,*,*,形容詞,終止形-一般,オオイ,多い,多い,オーイ,多い,オーイ,\
                   和,*,*,*,*,*,*,相,オオイ,オオイ,オオイ,オオイ,\"1,2\",C1,*,\
                   1241082407035563,4623";
    let f = fields(feature);
    assert_eq!(f.len(), 29);
    assert_eq!(f[20], "オオイ", "kana");
    assert_eq!(f[24], "1,2", "aType");
    assert_eq!(f[25], "C1", "aConType");
}
