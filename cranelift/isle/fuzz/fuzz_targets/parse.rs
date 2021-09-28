#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|s: &str| {
    let _ = env_logger::try_init();

    let lexer = isle::lexer::Lexer::from_str(s, "fuzz-input.isle");
    log::debug!("lexer = {:?}", lexer);

    if let Ok(lexer) = lexer {
        let mut parser = isle::parser::Parser::new(lexer);

        let defs = parser.parse_defs();
        log::debug!("defs = {:?}", defs);
    }
});
