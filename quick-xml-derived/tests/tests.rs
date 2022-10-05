use macrotest;

#[test]
pub fn pass() {
    macrotest::expand("tests/expand/*.rs");
}