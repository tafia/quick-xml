use macrotest;
#[test]
pub fn pass() {
    macrotest::expand("tests/expand/*.rs");
}

#[test]
fn try_build_tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/try_build_tests/example.rs");
}