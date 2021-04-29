use anyhow::Context;
use wat::parse_str as wat_to_wasm;
use wizer::Wizer;

fn run_wat(args: &[wasmtime::Val], expected: i32, wat: &str) -> anyhow::Result<()> {
    let _ = env_logger::try_init();
    let wasm = wat_to_wasm(wat)?;
    run_wasm(args, expected, &wasm)
}

fn run_wasm(args: &[wasmtime::Val], expected: i32, wasm: &[u8]) -> anyhow::Result<()> {
    let _ = env_logger::try_init();

    let mut wizer = Wizer::new();
    wizer.allow_wasi(true);
    wizer.wasm_multi_memory(true);
    wizer.wasm_module_linking(true);
    let wasm = wizer.run(&wasm)?;

    let mut config = wasmtime::Config::new();
    config.cache_config_load_default().unwrap();
    config.wasm_multi_memory(true);
    config.wasm_multi_value(true);
    config.wasm_module_linking(true);

    let engine = wasmtime::Engine::new(&config)?;
    let store = wasmtime::Store::new(&engine);
    let module =
        wasmtime::Module::new(store.engine(), wasm).context("Wasm test case failed to compile")?;

    let mut linker = wasmtime::Linker::new(&store);
    let ctx = wasi_cap_std_sync::WasiCtxBuilder::new().build()?;
    let wasi = wasmtime_wasi::Wasi::new(&store, ctx);
    wasi.add_to_linker(&mut linker)?;
    let instance = linker.instantiate(&module)?;

    let run = instance
        .get_func("run")
        .ok_or_else(|| anyhow::anyhow!("the test Wasm module does not export a `run` function"))?;

    let actual = run.call(args)?;
    anyhow::ensure!(actual.len() == 1, "expected one result");
    let actual = match actual[0] {
        wasmtime::Val::I32(x) => x,
        _ => anyhow::bail!("expected an i32 result"),
    };
    anyhow::ensure!(
        expected == actual,
        "expected `{}`, found `{}`",
        expected,
        actual,
    );

    Ok(())
}

#[test]
fn basic_global() -> anyhow::Result<()> {
    run_wat(
        &[],
        42,
        r#"
(module
  (global $g (mut i32) i32.const 0)
  (func (export "wizer.initialize")
    i32.const 42
    global.set $g)
  (func (export "run") (result i32)
    global.get $g))
        "#,
    )
}

#[test]
fn basic_memory() -> anyhow::Result<()> {
    run_wat(
        &[],
        42,
        r#"
(module
  (memory 1)
  (func (export "wizer.initialize")
    i32.const 0
    i32.const 42
    i32.store offset=1337)
  (func (export "run") (result i32)
    i32.const 0
    i32.load offset=1337))
        "#,
    )
}

#[test]
fn multi_memory() -> anyhow::Result<()> {
    run_wat(
        &[],
        42,
        r#"
(module
  (memory $m1 1)
  (memory $m2 1)
  (func (export "wizer.initialize")
    i32.const 0
    i32.const 41
    i32.store (memory $m1) offset=1337
    i32.const 0
    i32.const 1
    i32.store (memory $m2) offset=1337)
  (func (export "run") (result i32)
    i32.const 0
    i32.load (memory $m1) offset=1337
    i32.const 0
    i32.load (memory $m2) offset=1337
    i32.add))
"#,
    )
}

#[test]
fn reject_imported_memory() -> anyhow::Result<()> {
    assert!(run_wat(
        &[],
        42,
        r#"
(module
  (import "" "" (memory 1)))
"#,
    )
    .is_err());
    Ok(())
}

#[test]
fn reject_imported_global() -> anyhow::Result<()> {
    assert!(run_wat(
        &[],
        42,
        r#"
(module
  (import "" "" (global i32)))
"#,
    )
    .is_err());
    Ok(())
}

#[test]
fn reject_imported_table() -> anyhow::Result<()> {
    assert!(run_wat(
        &[],
        42,
        r#"
(module
  (import "" "" (table)))
"#,
    )
    .is_err());
    Ok(())
}

#[test]
fn reject_bulk_memory() -> anyhow::Result<()> {
    let result = run_wat(
        &[],
        42,
        r#"
(module
  (table 3 funcref)

  (func $f (result i32) (i32.const 0))
  (func $g (result i32) (i32.const 0))
  (func $h (result i32) (i32.const 0))

  (func (export "main")
    i32.const 0
    i32.const 1
    i32.const 1
    table.copy)

  (elem (i32.const 0) $f $g $h)
)
"#,
    );
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(err
        .to_string()
        .contains("unsupported `table.copy` instruction"));

    Ok(())
}

#[test]
fn accept_module_linking_import_memory() -> anyhow::Result<()> {
    run_wat(
        &[],
        42,
        r#"
(module
  (module $A
    (memory (export "memory") 1))
  (instance $a (instantiate $A))

  (module $B
    (import "x" (instance $x (export "memory" (memory 1)))))
  (instance $b (instantiate $B (import "x" (instance $a))))

  (func (export "wizer.initialize")
    nop)

  (func (export "run") (result i32)
    i32.const 42)
)
"#,
    )
}

#[test]
fn accept_module_linking_import_global() -> anyhow::Result<()> {
    run_wat(
        &[],
        42,
        r#"
(module
  (module $A
    (global (export "global") i32 (i32.const 1337)))
  (instance $a (instantiate $A))

  (module $B
    (import "x" (instance $x (export "global" (global i32)))))
  (instance $b (instantiate $B (import "x" (instance $a))))

  (func (export "wizer.initialize")
    nop)

  (func (export "run") (result i32)
    i32.const 42)
)
"#,
    )
}

#[test]
fn accept_module_linking_import_table() -> anyhow::Result<()> {
    run_wat(
        &[],
        42,
        r#"
(module
  (module $A
    (table (export "table") 0 funcref))
  (instance $a (instantiate $A))

  (module $B
    (import "x" (instance $x (export "table" (table 0 funcref)))))
  (instance $b (instantiate $B (import "x" (instance $a))))

  (func (export "wizer.initialize")
    nop)

  (func (export "run") (result i32)
    i32.const 42)
)
"#,
    )
}

#[test]
fn module_linking_actually_works() -> anyhow::Result<()> {
    run_wat(
        &[],
        42,
        r#"
(module
  (module $memory-module
    (memory (export "memory") 1))
  (instance $memory-instance (instantiate $memory-module))

  (module $use-memory
    (import "x" (instance $m (export "memory" (memory 1))))
    (func (export "init")
      i32.const 0
      i32.const 42
      i32.store (memory $m "memory") offset=1337))
  (instance $use-memory-instance
    (instantiate $use-memory
      (import "x" (instance $memory-instance))))

  (func (export "wizer.initialize")
    (call (func $use-memory-instance "init")))

  (func (export "run") (result i32)
    i32.const 0
    i32.load (memory $memory-instance "memory") offset=1337)
)
"#,
    )
}

#[test]
fn module_linking_nested_instantiations_1() -> anyhow::Result<()> {
    run_wat(
        &[],
        8,
        r#"
(module
  (module $A
    (import "global" (global (mut i32)))

    (module $B
      (import "global" (global (mut i32)))

        (module $C
          (import "global" (global (mut i32)))

          (func (export "f")
            i32.const 1
            global.get 0
            i32.add
            global.set 0
          )
        )

        (instance $c1 (instantiate $C (import "global" (global 0))))
        (instance $c2 (instantiate $C (import "global" (global 0))))

        (func (export "f")
          call (func $c1 "f")
          call (func $c2 "f")
       )
    )

    (instance $b1 (instantiate $B (import "global" (global 0))))
    (instance $b2 (instantiate $B (import "global" (global 0))))

    (func (export "f")
      call (func $b1 "f")
      call (func $b2 "f")
    )
  )

  (module $DefinesGlobal
    (global (export "global") (mut i32) (i32.const 0)))
  (instance $global_instance (instantiate $DefinesGlobal))

  (instance $a1 (instantiate $A (import "global" (global $global_instance "global"))))
  (instance $a2 (instantiate $A (import "global" (global $global_instance "global"))))

  (func (export "wizer.initialize")
    call (func $a1 "f")
    call (func $a2 "f"))

  (func (export "run") (result i32)
    global.get (global $global_instance "global"))
)
"#,
    )
}

#[test]
fn module_linking_nested_instantiations_0() -> anyhow::Result<()> {
    run_wat(
        &[],
        42,
        r#"
(module
  (module $A
    (import "global" (global (mut i32)))

    (module $B
      (import "global" (global (mut i32)))

       (func (export "f")
         i32.const 42
         global.set 0
       )
    )

    (instance $b (instantiate $B (import "global" (global 0))))

    (func (export "f")
      call (func $b "f")
    )
  )

  (module $G
    (global (export "global") (mut i32) (i32.const 0)))

  (instance $g (instantiate $G))

  (instance $a (instantiate $A (import "global" (global $g "global"))))

  (func (export "wizer.initialize")
    call (func $a "f")
  )

  (func (export "run") (result i32)
    global.get (global $g "global")
  )
)
"#,
    )
}

// Test that we handle repeated and interleaved initial sections.
#[test]
fn multiple_initial_sections() -> anyhow::Result<()> {
    run_wat(
        &[],
        42,
        r#"
(module
  ;; Module section.
  (module $A
    (memory (export "memory") 1)
  )

  ;; Instance section.
  (instance $a (instantiate $A))

  ;; Alias section.
  (alias $a "memory" (memory $memory))

  ;; Module section.
  (module $B
    (import "memory" (memory 1))
    (func (export "init")
      i32.const 0
      i32.const 42
      i32.store offset=1337
    )
  )

  ;; Instance section.
  (instance $b (instantiate $B (import "memory" (memory $memory))))

  ;; Alias section.
  (alias $b "init" (func $b-init))

  ;; Module section.
  (module $C
    (import "memory" (memory 1))
    (func (export "run") (result i32)
      i32.const 0
      i32.load offset=1337
    )
  )

  ;; Instance section.
  (instance $c (instantiate $C (import "memory" (memory $memory))))

  ;; Alias section.
  (alias $c "run" (func $c-run))

  ;; Done with initial sections.

  (func (export "wizer.initialize")
    call $b-init
  )

  (func (export "run") (result i32)
    call $c-run
  )
)
"#,
    )
}

#[test]
fn start_sections_in_nested_modules() -> anyhow::Result<()> {
    run_wat(
        &[],
        42,
        r#"
(module
  (module $A
    (import "global" (global $g (mut i32)))
    (func $init
      i32.const 41
      global.set $g)
    (start $init)
  )

  (module $B
    (global (export "global") (mut i32) (i32.const 0))
  )

  (instance $b (instantiate $B))
  (alias $b "global" (global $g))
  (instance $a (instantiate $A (import "global" (global $g))))

  (func (export "wizer.initialize")
    global.get $g
    i32.const 1
    i32.add
    global.set $g
  )
  (func (export "run") (result i32)
    global.get $g
  )
)
"#,
    )
}

#[test]
fn rust_regex() -> anyhow::Result<()> {
    run_wasm(
        &[wasmtime::Val::I32(13)],
        42,
        &include_bytes!("./regex_test.wasm")[..],
    )
}

#[test]
fn rename_functions() -> anyhow::Result<()> {
    let wat = r#"
(module
 (func (export "wizer.initialize"))
 (func (export "func_a") (result i32)
   i32.const 1)
 (func (export "func_b") (result i32)
   i32.const 2)
 (func (export "func_c") (result i32)
   i32.const 3))
  "#;

    let wasm = wat_to_wasm(wat)?;
    let mut wizer = Wizer::new();
    wizer.allow_wasi(true);
    wizer.func_rename("func_a", "func_b");
    wizer.func_rename("func_b", "func_c");
    let wasm = wizer.run(&wasm)?;
    let wat = wasmprinter::print_bytes(&wasm)?;

    let expected_wat = r#"
(module
  (type (;0;) (func))
  (type (;1;) (func (result i32)))
  (func (;0;) (type 0))
  (func (;1;) (type 1) (result i32)
    i32.const 1)
  (func (;2;) (type 1) (result i32)
    i32.const 2)
  (func (;3;) (type 1) (result i32)
    i32.const 3)
  (export "func_a" (func 2))
  (export "func_b" (func 3)))
  "#;

    assert_eq!(wat.trim(), expected_wat.trim());
    Ok(())
}
