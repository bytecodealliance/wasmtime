use anyhow::{Context, Result};
use std::process::Command;
use wasm_encoder::ConstExpr;
use wasmtime::{Config, Engine, Instance, Linker, Module, Store};
use wasmtime_wasi::{WasiCtxBuilder, p1};
use wasmtime_wizer::Wizer;
use wat::parse_str as wat_to_wasm;

fn run_wat(args: &[wasmtime::Val], expected: i32, wat: &str) -> Result<()> {
    let _ = env_logger::try_init();
    let wasm = wat_to_wasm(wat)?;
    wizen_and_run_wasm(args, expected, &wasm, get_wizer())
}

fn get_wizer() -> Wizer {
    Wizer::new()
}

fn store() -> Result<Store<p1::WasiP1Ctx>> {
    let mut wasi = WasiCtxBuilder::new();
    let mut config = Config::new();
    config.relaxed_simd_deterministic(true);
    let engine = Engine::new(&config)?;
    Ok(Store::new(&engine, wasi.build_p1()))
}

fn instantiate(store: &mut Store<p1::WasiP1Ctx>, module: &Module) -> Result<Instance> {
    let mut linker = Linker::new(store.engine());
    p1::add_to_linker_sync(&mut linker, |x| x)?;
    linker.define_unknown_imports_as_traps(module)?;
    linker.instantiate(store, module)
}

fn wizen_and_run_wasm(
    args: &[wasmtime::Val],
    expected: i32,
    wasm: &[u8],
    wizer: Wizer,
) -> Result<()> {
    let _ = env_logger::try_init();

    log::debug!(
        "=== PreWizened Wasm ==========================================================\n\
      {}\n\
      ===========================================================================",
        wasmprinter::print_bytes(&wasm).unwrap()
    );
    let mut store = store()?;
    let wasm = wizer.run(&mut store, &wasm, instantiate)?;
    log::debug!(
        "=== Wizened Wasm ==========================================================\n\
      {}\n\
      ===========================================================================",
        wasmprinter::print_bytes(&wasm).unwrap()
    );
    if log::log_enabled!(log::Level::Debug) {
        std::fs::write("test.wasm", &wasm).unwrap();
    }

    let module =
        wasmtime::Module::new(store.engine(), wasm).context("Wasm test case failed to compile")?;

    let mut linker = wasmtime::Linker::new(store.engine());
    let thunk = wasmtime::Func::wrap(&mut store, || {});
    linker
        .define_name(&mut store, "dummy_func", thunk)?
        .define(&mut store, "env", "f", thunk)?
        .define_name(&mut store, "f", thunk)?
        .define(&mut store, "x", "f", thunk)?;

    p1::add_to_linker_sync(&mut linker, |wasi| wasi)?;

    let instance = linker.instantiate(&mut store, &module)?;

    let run = instance
        .get_func(&mut store, "run")
        .ok_or_else(|| anyhow::anyhow!("the test Wasm module does not export a `run` function"))?;

    let mut actual = vec![wasmtime::Val::I32(0)];
    run.call(&mut store, args, &mut actual)?;
    anyhow::ensure!(actual.len() == 1, "expected one result");
    let actual = match actual[0] {
        wasmtime::Val::I32(x) => x,
        _ => anyhow::bail!("expected an i32 result"),
    };
    anyhow::ensure!(
        expected == actual,
        "expected `{expected}`, found `{actual}`",
    );

    Ok(())
}

fn fails_wizening(wat: &str) -> Result<()> {
    let _ = env_logger::try_init();

    let wasm = wat_to_wasm(wat)?;

    let mut features = wasmparser::WasmFeatures::WASM2;
    features.set(wasmparser::WasmFeatures::MULTI_MEMORY, true);

    let mut validator = wasmparser::Validator::new_with_features(features);
    validator
        .validate_all(&wasm)
        .context("initial Wasm should be valid")?;

    anyhow::ensure!(
        get_wizer().run(&mut store()?, &wasm, instantiate).is_err(),
        "Expected an error when wizening, but didn't get one"
    );
    Ok(())
}

#[test]
fn basic_global() -> Result<()> {
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
fn basic_memory() -> Result<()> {
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
fn multi_memory() -> Result<()> {
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
       i32.store $m1 offset=1337
       i32.const 0
       i32.const 1
       i32.store $m2 offset=1337)
 (func (export "run") (result i32)
       i32.const 0
       i32.load $m1 offset=1337
       i32.const 0
       i32.load $m2 offset=1337
       i32.add))
"#,
    )
}
#[test]
fn reject_imported_memory() -> Result<()> {
    fails_wizening(
        r#"
            (module
              (import "" "" (memory 1)))
        "#,
    )
}

#[test]
fn reject_imported_global() -> Result<()> {
    fails_wizening(
        r#"
            (module
              (import "" "" (global i32)))
        "#,
    )
}

#[test]
fn reject_imported_table() -> Result<()> {
    fails_wizening(
        r#"
            (module
              (import "" "" (table 0 externref)))
        "#,
    )
}

#[test]
fn reject_table_copy() -> Result<()> {
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
    assert!(
        err.to_string()
            .contains("unsupported `table.copy` instruction")
    );

    Ok(())
}

#[test]
fn reject_table_get_set() -> Result<()> {
    let wat = r#"
(module
  (table 3 funcref)

  (func $f (result i32) (i32.const 0))
  (func $g (result i32) (i32.const 0))
  (func $h (result i32) (i32.const 0))

  (func (export "main")
    i32.const 0
    i32.const 1
    table.get
    table.set)

  (elem (i32.const 0) $f $g $h)
)
"#;

    let _ = env_logger::try_init();
    let wizer = Wizer::new();
    let wasm = wat_to_wasm(wat)?;
    let result = wizen_and_run_wasm(&[], 42, &wasm, wizer);

    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(
        err.to_string()
            .contains("unsupported `table.get` instruction"),
        "bad error: {err}",
    );

    Ok(())
}

#[test]
fn reject_table_get_set_with_reference_types_enabled() -> Result<()> {
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
          table.get
          table.set)

        (elem (i32.const 0) $f $g $h)
      )"#,
    );
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(
        err.to_string()
            .contains("unsupported `table.get` instruction"),
    );

    Ok(())
}

#[test]
fn reject_table_grow_with_reference_types_enabled() -> anyhow::Result<()> {
    let wat = r#"
      (module
        (elem declare func $f)
        (func $f)
        (table 0 funcref)
        (func (export "_initialize") (result i32)
            ref.func $f
            i32.const 1
            table.grow
        )
      )"#;

    let _ = env_logger::try_init();
    let wizer = Wizer::new();
    let wasm = wat_to_wasm(wat)?;
    let result = wizen_and_run_wasm(&[], 42, &wasm, wizer);

    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(
        err.to_string()
            .contains("unsupported `ref.func` instruction")
    );

    Ok(())
}

#[test]
fn indirect_call_with_reference_types() -> anyhow::Result<()> {
    let wat = r#"
      (module
        (type $sig (func (result i32)))
        (table 0 funcref)
        (table $table1 1 funcref)
        (elem (table $table1) (i32.const 0) func $f)
        (func $f (type $sig)
          i32.const 42
        )
        (func (export "wizer.initialize"))
        (func (export "run") (result i32)
          i32.const 0
          call_indirect $table1 (type $sig)
        )
      )"#;

    let _ = env_logger::try_init();
    let wizer = Wizer::new();
    let wasm = wat_to_wasm(wat)?;
    wizen_and_run_wasm(&[], 42, &wasm, wizer)
}

#[test]
fn reject_table_init() -> Result<()> {
    let result = run_wat(
        &[],
        42,
        r#"
(module
  (table 3 funcref)

  (func $f (result i32) (i32.const 0))
  (func $g (result i32) (i32.const 0))
  (func $h (result i32) (i32.const 0))

  (elem $elem func $f $g $h)

  (func (export "main")
    i32.const 0
    i32.const 0
    i32.const 3
    table.init $elem)
)
"#,
    );
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(
        err.to_string()
            .contains("unsupported `table.init` instruction")
    );

    Ok(())
}

#[test]
fn reject_elem_drop() -> Result<()> {
    let result = run_wat(
        &[],
        42,
        r#"
(module
  (table 3 funcref)

  (func $f (result i32) (i32.const 0))
  (func $g (result i32) (i32.const 0))
  (func $h (result i32) (i32.const 0))

  (elem $elem func $f $g $h)

  (func (export "main")
    elem.drop $elem)
)
"#,
    );
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(
        err.to_string()
            .contains("unsupported `elem.drop` instruction")
    );

    Ok(())
}

#[test]
fn reject_data_drop() -> Result<()> {
    let result = run_wat(
        &[],
        42,
        r#"
(module
  (memory 1)
  (data $data "hello, wizer!")

  (func (export "main")
    data.drop $data)
)
"#,
    );
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(
        err.to_string()
            .contains("unsupported `data.drop` instruction")
    );

    Ok(())
}

#[test]
fn rust_regex() -> Result<()> {
    let status = Command::new("cargo")
        .args(&["build", "--target=wasm32-wasip1", "-q"])
        .current_dir("./tests/regex-test")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env_remove("RUSTFLAGS")
        .status()
        .expect("failed to build regex test case");
    assert!(status.success());
    wizen_and_run_wasm(
        &[wasmtime::Val::I32(13)],
        42,
        &std::fs::read("../../target/wasm32-wasip1/debug/regex_test.wasm")
            .expect("failed to read regex test case"),
        get_wizer(),
    )
}

#[test]
fn data_segment_at_end_of_memory() -> Result<()> {
    // Test that we properly synthesize data segments for data at the end of
    // memory.
    run_wat(
        &[],
        42,
        r#"
(module
  (memory 1)
  (func (export "wizer.initialize")
    ;; Store 42 to the last byte in memory.
    i32.const 0
    i32.const 42
    i32.store8 offset=65535
  )
  (func (export "run") (result i32)
    i32.const 0
    i32.load8_u offset=65535
  )
)
"#,
    )
}

#[test]
fn too_many_data_segments_for_engines() -> Result<()> {
    run_wat(
        &[],
        42,
        r#"
(module
  ;; Enough memory to create more segments than engines will allow:
  ;;
  ;;     // The maximum number of segments that engines will allow a module to
  ;;     // have.
  ;;     let max_segments = 100_000;
  ;;
  ;;     // The minimum gap that Wizer won't automatically merge two data
  ;;     // segments (see `MIN_ACTIVE_SEGMENT_OVERHEAD`).
  ;;     let wizer_min_gap = 6;
  ;;
  ;;     // Wasm page size.
  ;;     let wasm_page_size = 65_536;
  ;;
  ;;     let num_pages = round_up(max_segments * wizer_min_gap / wasm_page_size);
  (memory 10)

  (func (export "wizer.initialize")
    (local i32)
    loop
      (i32.ge_u (local.get 0) (i32.const 655360)) ;; 10 * wasm_page_size
      if
        return
      end

      (i32.store8 (local.get 0) (i32.const 42))
      (local.set 0 (i32.add (local.get 0) (i32.const 6)))
      br 0
    end
  )
  (func (export "run") (result i32)
    i32.const 0
    i32.load8_u
  )
)
"#,
    )
}

#[test]
fn rename_functions() -> Result<()> {
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
    wizer.func_rename("func_a", "func_b");
    wizer.func_rename("func_b", "func_c");
    let wasm = wizer.run(&mut store()?, &wasm, instantiate)?;
    let wat = wasmprinter::print_bytes(&wasm)?;

    let expected_wat = r#"
(module
  (type (;0;) (func))
  (type (;1;) (func (result i32)))
  (export "func_a" (func 2))
  (export "func_b" (func 3))
  (func (;0;) (type 0))
  (func (;1;) (type 1) (result i32)
    i32.const 1
  )
  (func (;2;) (type 1) (result i32)
    i32.const 2
  )
  (func (;3;) (type 1) (result i32)
    i32.const 3
  )
)
  "#;

    assert_eq!(wat.trim(), expected_wat.trim());
    Ok(())
}

#[test]
fn wasi_reactor() -> anyhow::Result<()> {
    run_wat(
        &[],
        42,
        r#"
            (module
              (global $g (mut i32) i32.const 0)
              (func (export "_initialize")
                i32.const 6
                global.set $g
              )
              (func (export "wizer.initialize")
                global.get $g
                i32.const 7
                i32.mul
                global.set $g)
              (func (export "run") (result i32)
                global.get $g
              )
            )
        "#,
    )
}

#[test]
fn wasi_reactor_initializer_as_init_func() -> anyhow::Result<()> {
    let wat = r#"
      (module
        (global $g (mut i32) i32.const 0)
        (func (export "_initialize")
          global.get $g
          i32.const 1
          i32.add
          global.set $g
        )
        (func (export "run") (result i32)
          global.get $g
        )
      )"#;

    let _ = env_logger::try_init();
    let mut wizer = Wizer::new();
    wizer.init_func("_initialize");
    let wasm = wat_to_wasm(wat)?;
    // we expect `_initialize` to be called _exactly_ once
    wizen_and_run_wasm(&[], 1, &wasm, wizer)
}

#[test]
fn wasi_reactor_initializer_with_keep_init() -> anyhow::Result<()> {
    let wat = r#"
      (module
        (global $g (mut i32) i32.const 0)
        (func (export "_initialize")
          i32.const 1
          global.set $g
        )
        (func (export "wizer.initialize")
          i32.const 2
          global.set $g)
        (func (export "run") (result i32)
          global.get $g
        )
      )"#;

    let _ = env_logger::try_init();
    let mut wizer = Wizer::new();
    wizer.keep_init_func(true);
    let wasm = wat_to_wasm(wat)?;
    // we expect `_initialize` to be un-exported and not called at run
    wizen_and_run_wasm(&[], 2, &wasm, wizer)
}

#[test]
fn call_undefined_import_function_during_init() -> Result<()> {
    fails_wizening(
        r#"
            (module
              (import "x" "f" (func $import))
              (func (export "wizer.initialize")
                (call $import)
              )
            )
        "#,
    )
}

#[test]
fn allow_undefined_import_function() -> Result<()> {
    run_wat(
        &[],
        42,
        r#"
            (module
              (import "x" "f" (func $import))
              (func (export "wizer.initialize"))
              (func (export "run") (result i32)
                i32.const 42
              )
            )
        "#,
    )
}

#[test]
fn accept_bulk_memory_copy() -> Result<()> {
    run_wat(
        &[],
        ('h' as i32) + ('w' as i32),
        r#"
            (module
              (memory $memory (data "hello, wizer!"))
              (func (export "wizer.initialize")
                i32.const 42 ;; dst
                i32.const 0  ;; src
                i32.const 13 ;; size
                memory.copy)
              (func (export "run") (result i32)
                i32.const 42
                i32.load8_u
                i32.const 42
                i32.load8_u offset=7
                i32.add))
        "#,
    )
}

#[test]
fn accept_bulk_memory_data_count() -> Result<()> {
    let mut module = wasm_encoder::Module::new();
    let mut types = wasm_encoder::TypeSection::new();
    types.ty().func_type(&wasm_encoder::FuncType::new(
        vec![],
        vec![wasm_encoder::ValType::I32],
    ));
    types
        .ty()
        .func_type(&wasm_encoder::FuncType::new(vec![], vec![]));
    module.section(&types);

    let mut functions = wasm_encoder::FunctionSection::new();
    functions.function(0);
    functions.function(1);
    module.section(&functions);

    let mut memory = wasm_encoder::MemorySection::new();
    memory.memory(wasm_encoder::MemoryType {
        minimum: 1,
        maximum: Some(1),
        memory64: false,
        shared: false,
        page_size_log2: None,
    });
    module.section(&memory);

    let mut exports = wasm_encoder::ExportSection::new();
    exports.export("run", wasm_encoder::ExportKind::Func, 0);
    exports.export("wizer.initialize", wasm_encoder::ExportKind::Func, 1);
    module.section(&exports);

    module.section(&wasm_encoder::DataCountSection { count: 2 });

    let mut code = wasm_encoder::CodeSection::new();
    let mut func = wasm_encoder::Function::new(vec![]);
    func.instruction(&wasm_encoder::Instruction::I32Const(42));
    func.instruction(&wasm_encoder::Instruction::End);
    code.function(&func);

    let mut func = wasm_encoder::Function::new(vec![]);
    func.instruction(&wasm_encoder::Instruction::End);
    code.function(&func);

    module.section(&code);

    // We're expecting these two data segments to be merge into one, which will exercise wizer's
    // ability to output the correct data count (1 instead of 2 above).
    let mut data = wasm_encoder::DataSection::new();
    data.active(0, &ConstExpr::i32_const(0), vec![0, 1, 2, 3]);
    data.active(0, &ConstExpr::i32_const(4), vec![5, 6, 7, 8]);
    module.section(&data);

    wizen_and_run_wasm(&[], 42, &module.finish(), get_wizer()).unwrap();
    Ok(())
}

#[test]
fn accept_bulk_memory_fill() -> Result<()> {
    run_wat(
        &[],
        77 + 77,
        r#"
            (module
              (memory 1)
              (func (export "wizer.initialize")
                i32.const 42 ;; dst
                i32.const 77 ;; value
                i32.const 13 ;; size
                memory.fill)
              (func (export "run") (result i32)
                i32.const 42
                i32.load8_u
                i32.const 42
                i32.load8_u offset=7
                i32.add))
        "#,
    )
}

#[test]
fn accept_bulk_memory_init() -> Result<()> {
    run_wat(
        &[],
        ('h' as i32) + ('w' as i32),
        r#"
            (module
              (memory 1)
              (data $data "hello, wizer!")
              (func (export "wizer.initialize")
                i32.const 42 ;; dst
                i32.const 0  ;; offset
                i32.const 13 ;; size
                memory.init $data)
              (func (export "run") (result i32)
                i32.const 42
                i32.load8_u
                i32.const 42
                i32.load8_u offset=7
                i32.add))
        "#,
    )
}

#[test]
fn accept_simd128() -> Result<()> {
    run_wat(
        &[],
        49,
        r#"
            (module
              (global $g (mut v128) (v128.const i32x4 2 3 5 7))
              (func (export "wizer.initialize")
                global.get $g
                global.get $g
                i32x4.mul
                global.set $g)
              (func (export "run") (result i32)
                global.get $g
                i32x4.extract_lane 3))
        "#,
    )
}

#[test]
fn relaxed_simd_deterministic() -> Result<()> {
    let _ = env_logger::try_init();
    let wasm = wat_to_wasm(
        r#"
(module
  (global $g (mut i32) i32.const 0)
  (func (export "wizer.initialize")
    (v128.const f32x4 2796203.5 0.0 0.0 0.0)
    (v128.const f32x4 3.0 0.0 0.0 0.0)
    (v128.const f32x4 8388611.0 0.0 0.0 0.0)
    f32x4.relaxed_madd
    f32x4.extract_lane 0
    i32.reinterpret_f32
    global.set $g)
  (func (export "run") (result i32)
    global.get $g
  )
)
        "#,
    )?;
    let wizer = get_wizer();

    // We'll get 0x4b000003 if we have the deterministic `relaxed_madd`
    // semantics. We might get 0x4b000002 if we don't.
    wizen_and_run_wasm(&[], 0x4b800003, &wasm, wizer)
}
