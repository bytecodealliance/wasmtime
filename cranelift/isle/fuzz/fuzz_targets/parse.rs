#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|s: &str| {
    let lexer = isle::lexer::Lexer::from_str(s, "fuzz-input.isle");
    let mut parser = isle::parser::Parser::new(lexer);
    let _ = parser.parse_defs();
});
