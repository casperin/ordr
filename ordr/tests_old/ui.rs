#[test]
fn ui_tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/01.rs");
    t.pass("tests/ui/02.rs");
    t.pass("tests/ui/03.rs");
}
