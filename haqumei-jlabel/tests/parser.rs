mod fixtures;
use fixtures::fixtures;
use haqumei_jlabel::Label;

#[test]
fn test_parse() {
    for (input, expected) in fixtures() {
        let actual: Label = input.parse().unwrap();
        assert_eq!(actual, expected);
    }
}
