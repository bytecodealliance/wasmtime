use super::*;
use wasmtime::*;

#[test]
fn engine_without_compiler_cannot_compile() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    let engine = Engine::new(&config)?;
    match Module::new(&engine, r#"(module (func (export "f") nop))"#) {
        Ok(_) => panic!("should not compile without a compiler"),
        Err(err) => err.assert_contains("Engine was not configured with a compiler"),
    }
    Ok(())
}
