#![cfg(arc_try_new)]

use std::iter;
use wasmtime::{Config, Engine, FuncType, Result, ValType};
use wasmtime_fuzzing::oom::OomTest;

#[test]
fn func_type_try_new() -> Result<()> {
    let mut config = Config::new();
    config.enable_compiler(false);
    let engine = Engine::new(&config)?;

    // Run this OOM test a few times to make sure that we leave the engine's
    // type registry in a good state when failing to register new types.
    for i in 1..6 {
        OomTest::new().test(|| {
            let ty1 = FuncType::try_new(
                &engine,
                iter::repeat(ValType::ANYREF).take(i),
                iter::repeat(ValType::ANYREF).take(i),
            )?;
            assert_eq!(ty1.params().len(), i);
            assert_eq!(ty1.results().len(), i);

            let ty2 = FuncType::try_new(
                &engine,
                iter::repeat(ValType::ANYREF).take(i),
                iter::repeat(ValType::ANYREF).take(i),
            )?;
            assert_eq!(ty2.params().len(), i);
            assert_eq!(ty2.results().len(), i);

            let ty3 = FuncType::try_new(&engine, [], [])?;
            assert_eq!(ty3.params().len(), 0);
            assert_eq!(ty3.results().len(), 0);

            assert!(
                !FuncType::eq(&ty2, &ty3),
                "{ty2:?} should not be equal to {ty3:?}"
            );

            Ok(())
        })?;
    }

    Ok(())
}
