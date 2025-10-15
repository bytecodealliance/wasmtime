use anyhow::Result;
use wasmtime::{Instance, Linker, Module};
use wasmtime_wizer::Wizer;
use wat::parse_str as wat_to_wasm;

const PRELOAD1: &'static str = r#"
(module
 (func (export "f") (param i32) (result i32)
  local.get 0
  i32.const 1
  i32.add))
  "#;

const PRELOAD2: &'static str = r#"
(module
 (func (export "f") (param i32) (result i32)
  local.get 0
  i32.const 2
  i32.add))
  "#;

fn run_with_preloads(args: &[wasmtime::Val], wat: &str) -> Result<wasmtime::Val> {
    let wasm = wat_to_wasm(wat)?;
    let engine = wasmtime::Engine::default();
    let mut store = wasmtime::Store::new(&engine, ());
    let mod1 = Module::new(store.engine(), PRELOAD1)?;
    let mod2 = Module::new(store.engine(), PRELOAD2)?;

    let processed = Wizer::new().run(&mut store, &wasm, |store, module| {
        let i1 = Instance::new(&mut *store, &mod1, &[])?;
        let i2 = Instance::new(&mut *store, &mod2, &[])?;
        let mut linker = Linker::new(store.engine());
        linker.instance(&mut *store, "mod1", i1)?;
        linker.instance(&mut *store, "mod2", i2)?;
        linker.instantiate(store, module)
    })?;

    let testmod = wasmtime::Module::new(&engine, &processed[..])?;

    let mod1_inst = wasmtime::Instance::new(&mut store, &mod1, &[])?;
    let mod2_inst = wasmtime::Instance::new(&mut store, &mod2, &[])?;
    let mut linker = wasmtime::Linker::new(&engine);
    linker.instance(&mut store, "mod1", mod1_inst)?;
    linker.instance(&mut store, "mod2", mod2_inst)?;

    let inst = linker.instantiate(&mut store, &testmod)?;
    let run = inst
        .get_func(&mut store, "run")
        .ok_or_else(|| anyhow::anyhow!("no `run` function on test module"))?;
    let mut returned = vec![wasmtime::Val::I32(0)];
    run.call(&mut store, args, &mut returned)?;
    Ok(returned[0])
}

#[test]
fn test_preloads() {
    const WAT: &'static str = r#"
    (module
     (import "mod1" "f" (func $mod1f (param i32) (result i32)))
     (import "mod2" "f" (func $mod2f (param i32) (result i32)))
     (global $g1 (mut i32) (i32.const 0))
     (global $g2 (mut i32) (i32.const 0))
     (func (export "wizer.initialize")
      i32.const 100
      call $mod1f
      global.set $g1
      i32.const 100
      call $mod2f
      global.set $g2)
     (func (export "run") (param i32 i32) (result i32)
      local.get 0
      call $mod1f
      local.get 1
      call $mod2f
      i32.add
      global.get $g1
      global.get $g2
      i32.add
      i32.add))
    "#;

    let result =
        run_with_preloads(&[wasmtime::Val::I32(200), wasmtime::Val::I32(201)], WAT).unwrap();
    assert!(matches!(result, wasmtime::Val::I32(607)));
}
