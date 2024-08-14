#![no_main]

use std::sync::Arc;

use cranelift_isle::files::Files;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|src: &str| {
    let _ = env_logger::try_init();

    let lexer = cranelift_isle::lexer::Lexer::new(0, src);
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

    let files = Arc::new(Files::from_names_and_contents([(
        "fuzz-input.isle".to_string(),
        src.to_string(),
    )]));

    let code = cranelift_isle::compile::compile(files, &defs, &Default::default());
    log::debug!("code = {:?}", code);
    let code = match code {
        Ok(c) => c,
        Err(_) => return,
    };

    // TODO: check that the generated code is valid Rust. This will require
    // stubbing out extern types, extractors, and constructors.
    drop(code);
});
