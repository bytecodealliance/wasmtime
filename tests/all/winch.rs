use wasmtime::*;

const MODULE: &'static str = r#"
    (module
      (import "" "" (func $add (param i32 i32) (result i32)))
      (func $test (result i32)
        (i32.const 42)
      )

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

      (func $call_add (param i32 i32) (result i32)
        (local.get 0)
        (local.get 1)
        (call $add))

      (export "42" (func $test))
      (export "sum10" (func $sum10))
      (export "call_add" (func $call_add))
    )
    "#;

fn add_fn(store: impl AsContextMut) -> Func {
    Func::wrap(store, |a: i32, b: i32| a + b)
}

#[test]
#[cfg_attr(miri, ignore)]
fn array_to_wasm() {
    let mut c = Config::new();
    c.strategy(Strategy::Winch);
    let engine = Engine::new(&c).unwrap();
    let mut store = Store::new(&engine, ());
    let module = Module::new(&engine, MODULE).unwrap();

    let add_fn = add_fn(store.as_context_mut());
    let instance = Instance::new(&mut store, &module, &[add_fn.into()]).unwrap();

    let constant = instance
        .get_func(&mut store, "42")
        .ok_or(anyhow::anyhow!("test function not found"))
        .unwrap();
    let mut returns = vec![Val::null(); 1];
    constant.call(&mut store, &[], &mut returns).unwrap();

    assert_eq!(returns.len(), 1);
    assert_eq!(returns[0].unwrap_i32(), 42);

    let sum = instance
        .get_func(&mut store, "sum10")
        .ok_or(anyhow::anyhow!("sum10 function not found"))
        .unwrap();
    let mut returns = vec![Val::null(); 1];
    let args = vec![Val::I32(1); 10];
    sum.call(&mut store, &args, &mut returns).unwrap();

    assert_eq!(returns.len(), 1);
    assert_eq!(returns[0].unwrap_i32(), 10);
}

#[test]
#[cfg_attr(miri, ignore)]
fn native_to_wasm() {
    let mut c = Config::new();
    c.strategy(Strategy::Winch);
    let engine = Engine::new(&c).unwrap();
    let mut store = Store::new(&engine, ());
    let module = Module::new(&engine, MODULE).unwrap();

    let add_fn = add_fn(store.as_context_mut());
    let instance = Instance::new(&mut store, &module, &[add_fn.into()]).unwrap();

    let f = instance
        .get_typed_func::<(i32, i32, i32, i32, i32, i32, i32, i32, i32, i32), i32>(
            &mut store, "sum10",
        )
        .unwrap();

    let args = (1, 1, 1, 1, 1, 1, 1, 1, 1, 1);
    let result = f.call(&mut store, args).unwrap();

    assert_eq!(result, 10);
}

#[test]
#[cfg_attr(miri, ignore)]
fn wasm_to_native() {
    let mut c = Config::new();
    c.strategy(Strategy::Winch);
    let engine = Engine::new(&c).unwrap();
    let mut store = Store::new(&engine, ());
    let module = Module::new(&engine, MODULE).unwrap();

    let add_fn = add_fn(store.as_context_mut());
    let instance = Instance::new(&mut store, &module, &[add_fn.into()]).unwrap();

    let f = instance
        .get_typed_func::<(i32, i32), i32>(&mut store, "call_add")
        .unwrap();

    let args = (41, 1);
    let result = f.call(&mut store, args).unwrap();

    assert_eq!(result, 42);
}
