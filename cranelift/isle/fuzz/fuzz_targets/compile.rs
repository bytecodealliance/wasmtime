#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|s: &str| {
    let _ = env_logger::try_init();

    let lexer = cranelift_isle::lexer::Lexer::from_str(s, "fuzz-input.isle");
    log::debug!("lexer = {:?}", lexer);
    let lexer = match lexer {
        Ok(l) => l,
        Err(_) => return,
    };

    let defs = cranelift_isle::parser::parse(lexer);
    log::debug!("defs = {:?}", defs);
    let defs = match defs {
        Ok(d) => d,
        Err(_) => return,
    };

    let code = cranelift_isle::compile::compile(&defs);
    log::debug!("code = {:?}", code);
    let code = match code {
        Ok(c) => c,
        Err(_) => return,
    };

    // TODO: check that the generated code is valid Rust. This will require
    // stubbing out extern types, extractors, and constructors.
    drop(code);
});
