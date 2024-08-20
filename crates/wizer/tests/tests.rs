use anyhow::{Context, Result};
use wasm_encoder::ConstExpr;
use wasmtime_wasi::{preview1, WasiCtxBuilder};
use wat::parse_str as wat_to_wasm;
use wizer::{StoreData, Wizer};

fn run_wat(args: &[wasmtime::Val], expected: i32, wat: &str) -> Result<()> {
    let _ = env_logger::try_init();
    let wasm = wat_to_wasm(wat)?;
    wizen_and_run_wasm(args, expected, &wasm, get_wizer())
}

fn get_wizer() -> Wizer {
    let mut wizer = Wizer::new();
    wizer.allow_wasi(true).unwrap();
    wizer.wasm_multi_memory(true);
    wizer.wasm_bulk_memory(true);
    wizer
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
    let wasm = wizer.run(&wasm)?;
    log::debug!(
        "=== Wizened Wasm ==========================================================\n\
      {}\n\
      ===========================================================================",
        wasmprinter::print_bytes(&wasm).unwrap()
    );
    if log::log_enabled!(log::Level::Debug) {
        std::fs::write("test.wasm", &wasm).unwrap();
    }

    let mut config = wasmtime::Config::new();
    config.cache_config_load_default().unwrap();
    config.wasm_multi_memory(true);
    config.wasm_multi_value(true);

    let engine = wasmtime::Engine::new(&config)?;
    let wasi_ctx = WasiCtxBuilder::new().build_p1();
    let mut store = wasmtime::Store::new(
        &engine,
        StoreData {
            wasi_ctx: Some(wasi_ctx),
        },
    );
    let module =
        wasmtime::Module::new(store.engine(), wasm).context("Wasm test case failed to compile")?;

    let mut linker = wasmtime::Linker::new(&engine);
    let thunk = wasmtime::Func::wrap(&mut store, || {});
    linker
        .define_name(&mut store, "dummy_func", thunk)?
        .define(&mut store, "env", "f", thunk)?
        .define_name(&mut store, "f", thunk)?
        .define(&mut store, "x", "f", thunk)?;

    preview1::add_to_linker_sync(&mut linker, |wasi| wasi.wasi_ctx.as_mut().unwrap())?;

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
        "expected `{}`, found `{}`",
        expected,
        actual,
    );

    Ok(())
}

fn fails_wizening(wat: &str) -> Result<()> {
    let _ = env_logger::try_init();

    let wasm = wat_to_wasm(wat)?;

    let mut validator = wasmparser::Validator::new_with_features(wasmparser::WasmFeatures {
        multi_memory: true,
        ..Default::default()
    });
    validator
        .validate_all(&wasm)
        .context("initial Wasm should be valid")?;

    anyhow::ensure!(
        get_wizer().run(&wasm).is_err(),
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
    assert!(err
        .to_string()
        .contains("unsupported `table.copy` instruction"));

    Ok(())
}

#[test]
fn reject_table_get_set() -> Result<()> {
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
)
"#,
    );
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(err
        .to_string()
        .contains("reference types support is not enabled"),);

    Ok(())
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
    assert!(err
        .to_string()
        .contains("unsupported `table.init` instruction"));

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
    assert!(err
        .to_string()
        .contains("unsupported `elem.drop` instruction"));

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
    assert!(err
        .to_string()
        .contains("unsupported `data.drop` instruction"));

    Ok(())
}

#[test]
fn rust_regex() -> Result<()> {
    wizen_and_run_wasm(
        &[wasmtime::Val::I32(13)],
        42,
        &include_bytes!("./regex_test.wasm")[..],
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
    wizer.allow_wasi(true).unwrap();
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
    i32.const 1
  )
  (func (;2;) (type 1) (result i32)
    i32.const 2
  )
  (func (;3;) (type 1) (result i32)
    i32.const 3
  )
  (export "func_a" (func 2))
  (export "func_b" (func 3))
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
    types.function(vec![], vec![wasm_encoder::ValType::I32]);
    types.function(vec![], vec![]);
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
