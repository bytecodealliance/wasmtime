use std::fs;

use cranelift_isle_veri_test_macros::file_tests;

#[file_tests(path = "tests/data", ext = "aslt")]
fn parse(test_file: &str) {
    let src = fs::read_to_string(test_file).unwrap();
    cranelift_isle_veri_aslp::parser::parse(&src).unwrap();
}
