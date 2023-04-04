#[cfg(target_arch = "x86_64")]
use anyhow::Result;
#[cfg(target_arch = "x86_64")]
use wasmtime::*;

#[test]
// For now, winch is only supported on x86_64 when running through wasmtime.
#[cfg(target_arch = "x86_64")]
fn compiles_with_winch() -> Result<()> {
    let mut c = Config::new();

    c.strategy(Strategy::Winch);

    let engine = Engine::new(&c)?;

    // Winch only supports a very basic function signature for now while it's being developed.
    let test_mod = r#"
    (module
      (func $test (result i32)
        (i32.const 42)
      )
      (export "test" (func $test))
    )
    "#;

    let mut store = Store::new(&engine, ());

    let module = Module::new(&engine, test_mod)?;

    let instance = Instance::new(&mut store, &module, &[])?;

    let f = instance
        .get_func(&mut store, "test")
        .ok_or(anyhow::anyhow!("test function not found"))?;

    let mut returns = vec![Val::null(); 1];

    // Winch doesn't support calling typed functions at the moment.
    f.call(&mut store, &[], &mut returns)?;

    assert_eq!(returns.len(), 1);
    assert_eq!(returns[0].unwrap_i32(), 42);

    Ok(())
}

#[test]
#[cfg(target_arch = "x86_64")]
fn compiles_with_winch_stack_arguments() -> Result<()> {
    let mut c = Config::new();

    c.strategy(Strategy::Winch);

    let engine = Engine::new(&c)?;

    // Winch only supports a very basic function signature for now while it's being developed.
    let test_mod = r#"
    (module
      (func $sum10 (param $arg_1 i32) (param $arg_2 i32) (param $arg_3 i32) (param $arg_4 i32) (param $arg_5 i32) (param $arg_6 i32) (param $arg_7 i32) (param $arg_8 i32) (param $arg_9 i32) (param $arg_10 i32) (result i32)
        local.get $arg_1
        local.get $arg_2
        i32.add
        local.get $arg_3
        i32.add
        local.get $arg_4
        i32.add
        local.get $arg_5
        i32.add
        local.get $arg_6
        i32.add
        local.get $arg_7
        i32.add
        local.get $arg_8
        i32.add
        local.get $arg_9
        i32.add
        local.get $arg_10
        i32.add)
      (export "sum10" (func $sum10))
    )
    "#;

    let mut store = Store::new(&engine, ());

    let module = Module::new(&engine, test_mod)?;

    let instance = Instance::new(&mut store, &module, &[])?;

    let f = instance
        .get_func(&mut store, "sum10")
        .ok_or(anyhow::anyhow!("sum10 function not found"))?;

    let mut returns = vec![Val::null(); 1];

    // create a new Val array with ten 1s
    let args = vec![Val::I32(1); 10];

    // Winch doesn't support calling typed functions at the moment.
    f.call(&mut store, &args, &mut returns)?;

    assert_eq!(returns.len(), 1);
    assert_eq!(returns[0].unwrap_i32(), 10);

    Ok(())
}
