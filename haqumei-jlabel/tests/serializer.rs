mod fixtures;
use fixtures::fixtures;

#[test]
fn test_serialize() {
    for (expected, label) in fixtures() {
        let actual = label.to_string();
        assert_eq!(actual, expected);
    }
}
