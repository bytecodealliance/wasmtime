use wasmtime::component::{Component, Instance, Linker, Val};
use wasmtime::{Config, Engine, Result, Store, ToWasmtimeResult as _, bail, error::Context as _};
use wasmtime_wizer::Wizer;

fn fail_wizening(msg: &str, wasm: &[u8]) -> Result<()> {
    let _ = env_logger::try_init();

    let wasm = wat::parse_bytes(wasm)?;
    log::debug!(
        "testing wizening failure for wasm:\n{}",
        wasmprinter::print_bytes(&wasm).to_wasmtime_result()?
    );
    match Wizer::new().instrument_component(&wasm) {
        Ok(_) => bail!("expected wizening to fail"),
        Err(e) => {
            let err = format!("{e}");
            if !err.contains(msg) {
                bail!("unexpected error: {err}");
            }
            Ok(())
        }
    }
}

#[test]
fn unsupported_constructs() -> Result<()> {
    fail_wizening(
        "does not currently support component imports",
        br#"(component
            (import "x" (component))
        )"#,
    )?;

    fail_wizening(
        "nested components with modules not currently supported",
        br#"(component
            (component (core module))
        )"#,
    )?;
    fail_wizening(
        "nested components with modules not currently supported",
        br#"(component
            (component)
            (component (core module))
        )"#,
    )?;
    fail_wizening(
        "wizer does not currently support module imports",
        br#"(component
            (component (import "x" (core module)))
        )"#,
    )?;
    fail_wizening(
        "wizer does not currently support module aliases",
        br#"(component
            (core module $a)
            (component
                (core instance (instantiate $a))
            )
        )"#,
    )?;
    fail_wizening(
        "wizer does not currently support component aliases",
        br#"(component
            (component $a)
            (component
                (instance (instantiate $a))
            )
        )"#,
    )?;

    fail_wizening(
        "does not currently support component aliases",
        br#"(component
            (import "x" (instance $i (export "x" (component))))
            (alias export $i "x" (component $c))
        )"#,
    )?;

    fail_wizening(
        "does not currently support module imports",
        br#"(component
            (import "x" (core module))
        )"#,
    )?;

    fail_wizening(
        "does not currently support module exports",
        br#"(component
            (core module $x)
            (export "x" (core module $x))
        )"#,
    )?;

    fail_wizening(
        "does not currently support module aliases",
        br#"(component
            (import "x" (instance $i (export "x" (core module))))
            (alias export $i "x" (core module $c))
        )"#,
    )?;
    fail_wizening(
        "does not currently support component start functions",
        br#"(component
            (import "f" (func $f))
            (start $f)
        )"#,
    )?;

    fail_wizening(
        "modules may be instantiated at most once",
        br#"(component
            (core module $a)
            (core instance $a1 (instantiate $a))
            (core instance $a2 (instantiate $a))
        )"#,
    )?;

    Ok(())
}

fn store() -> Result<Store<()>> {
    let mut config = Config::new();
    config.async_support(true);
    let engine = Engine::new(&config)?;
    Ok(Store::new(&engine, ()))
}

async fn instantiate(store: &mut Store<()>, component: &Component) -> Result<Instance> {
    let mut linker = Linker::new(store.engine());
    linker.define_unknown_imports_as_traps(component)?;
    linker.instantiate_async(store, component).await
}

async fn wizen(wat: &str) -> Result<Vec<u8>> {
    let _ = env_logger::try_init();
    let wasm = wat::parse_str(wat)?;

    log::debug!(
        "=== PreWizened Wasm ==========================================================\n\
      {}\n\
      ===========================================================================",
        wasmprinter::print_bytes(&wasm).unwrap()
    );
    let mut store = store()?;
    let wasm = Wizer::new()
        .run_component(&mut store, &wasm, instantiate)
        .await?;
    log::debug!(
        "=== Wizened Wasm ==========================================================\n\
      {}\n\
      ===========================================================================",
        wasmprinter::print_bytes(&wasm).unwrap()
    );
    if log::log_enabled!(log::Level::Debug) {
        std::fs::write("test.wasm", &wasm).unwrap();
    }

    Ok(wasm)
}

async fn wizen_and_run_wasm(expected: u32, wat: &str) -> Result<()> {
    let wasm = wizen(wat).await?;

    let mut store = store()?;
    let module =
        Component::new(store.engine(), wasm).context("Wasm test case failed to compile")?;

    let linker = Linker::new(store.engine());
    let instance = linker.instantiate_async(&mut store, &module).await?;

    let run = instance.get_func(&mut store, "run").ok_or_else(|| {
        wasmtime::format_err!("the test Wasm component does not export a `run` function")
    })?;

    let mut actual = [Val::U8(0)];
    run.call_async(&mut store, &[], &mut actual).await?;
    let actual = match actual[0] {
        Val::U32(x) => x,
        _ => wasmtime::bail!("expected an u32 result"),
    };
    wasmtime::ensure!(
        expected == actual,
        "expected `{expected}`, found `{actual}`",
    );

    Ok(())
}

#[tokio::test]
async fn simple() -> Result<()> {
    wizen_and_run_wasm(
        42,
        r#"(component
            (core module $m
                (func (export "init"))

                (func (export "run") (result i32)
                    i32.const 42
                )
            )
            (core instance $i (instantiate $m))
            (func (export "run") (result u32) (canon lift (core func $i "run")))
            (func (export "wizer-initialize") (canon lift (core func $i "init")))
        )"#,
    )
    .await?;

    Ok(())
}

#[tokio::test]
async fn snapshot_global_i32() -> Result<()> {
    wizen_and_run_wasm(
        1,
        r#"(component
            (core module $m
                (global $g (mut i32) i32.const 0)
                (func (export "init") (global.set $g (i32.const 1)))
                (func (export "run") (result i32) global.get $g)
            )
            (core instance $i (instantiate $m))
            (func (export "run") (result u32) (canon lift (core func $i "run")))
            (func (export "wizer-initialize") (canon lift (core func $i "init")))
        )"#,
    )
    .await?;

    Ok(())
}

#[tokio::test]
async fn snapshot_global_i64() -> Result<()> {
    wizen_and_run_wasm(
        1,
        r#"(component
            (core module $m
                (global $g (mut i64) i64.const 0)
                (func (export "init") (global.set $g (i64.const 1)))
                (func (export "run") (result i32)
                    global.get $g
                    i32.wrap_i64
                )
            )
            (core instance $i (instantiate $m))
            (func (export "run") (result u32) (canon lift (core func $i "run")))
            (func (export "wizer-initialize") (canon lift (core func $i "init")))
        )"#,
    )
    .await?;

    Ok(())
}

#[tokio::test]
async fn snapshot_global_f32() -> Result<()> {
    wizen_and_run_wasm(
        1,
        r#"(component
            (core module $m
                (global $g (mut f32) f32.const 0)
                (func (export "init") (global.set $g (f32.const 1)))
                (func (export "run") (result i32)
                    global.get $g
                    i32.trunc_f32_s)
            )
            (core instance $i (instantiate $m))
            (func (export "run") (result u32) (canon lift (core func $i "run")))
            (func (export "wizer-initialize") (canon lift (core func $i "init")))
        )"#,
    )
    .await?;

    Ok(())
}

#[tokio::test]
async fn snapshot_global_f64() -> Result<()> {
    wizen_and_run_wasm(
        1,
        r#"(component
            (core module $m
                (global $g (mut f64) f64.const 0)
                (func (export "init") (global.set $g (f64.const 1)))
                (func (export "run") (result i32)
                    global.get $g
                    i32.trunc_f64_s)
            )
            (core instance $i (instantiate $m))
            (func (export "run") (result u32) (canon lift (core func $i "run")))
            (func (export "wizer-initialize") (canon lift (core func $i "init")))
        )"#,
    )
    .await?;

    Ok(())
}

#[test]
fn v128_globals() -> Result<()> {
    fail_wizening(
        "component wizening does not support v128 globals",
        br#"(component
            (core module $a
                (global (export "x") (mut v128) (v128.const i32x4 1 2 3 4))
            )
            (core instance (instantiate $a))
        )"#,
    )
}

#[tokio::test]
async fn snapshot_memory() -> Result<()> {
    wizen_and_run_wasm(
        201,
        r#"(component
            (core module $m
                (memory 1)
                (func (export "init")
                    i32.const 200
                    i32.const 100
                    i32.store
                    i32.const 300
                    i32.const 101
                    i32.store
                )
                (func (export "run") (result i32)
                    i32.const 200
                    i32.load
                    i32.const 300
                    i32.load
                    i32.add
                )
            )
            (core instance $i (instantiate $m))
            (func (export "run") (result u32) (canon lift (core func $i "run")))
            (func (export "wizer-initialize") (canon lift (core func $i "init")))
        )"#,
    )
    .await?;

    Ok(())
}

#[tokio::test]
async fn nested_components() -> Result<()> {
    wizen_and_run_wasm(
        42,
        r#"(component
            (component $a)
            (instance (instantiate $a))
            (instance (export "hi") (instantiate $a))

            (component $b
                (type $t string)
                (import "x" (type (eq $t)))
                (component $a)
                (instance (instantiate $a))
                (instance (export "hi") (instantiate $a))
            )
            (type $x string)
            (instance (instantiate $b
                (with "x" (type $x))
            ))
            (instance (export "hi2") (instantiate $b
                (with "x" (type $x))
            ))

            (core module $m
                (func (export "init"))
                (func (export "run") (result i32) i32.const 42)
            )
            (core instance $i (instantiate $m))
            (func (export "run") (result u32) (canon lift (core func $i "run")))
            (func (export "wizer-initialize") (canon lift (core func $i "init")))
        )"#,
    )
    .await?;

    Ok(())
}

#[tokio::test]
async fn multiple_modules() -> Result<()> {
    wizen_and_run_wasm(
        100 + 101 + 200 + 201 + 7 + 112,
        r#"(component
            (core module $a
                (memory 1)
                (global $g (export "g") (mut i32) (i32.const 0))

                (func (export "init")
                    i32.const 200
                    i32.const 100
                    i32.store
                    i32.const 300
                    i32.const 101
                    i32.store
                )

                (func (export "run") (result i32)
                    i32.const 200
                    i32.load
                    i32.const 300
                    i32.load
                    i32.add
                    global.get $g
                    i32.add
                )
            )
            (core instance $a (instantiate $a))

            (core module $b
                (import "a" "g" (global $g (mut i32)))
                (import "a" "init" (func $init))
                (import "a" "run" (func $run (result i32)))
                (memory (export "mem") 1)
                (func (export "init")
                    call $init
                    i32.const 400
                    i32.const 200
                    i32.store
                    i32.const 500
                    i32.const 201
                    i32.store

                    i32.const 111
                    global.set $g
                )
                (func (export "run") (result i32)
                    i32.const 400
                    i32.load
                    i32.const 500
                    i32.load
                    i32.add
                    call $run
                    i32.add
                )
            )
            (core instance $b (instantiate $b (with "a" (instance $a))))

            (core module $c
                (import "a" "g" (global $g (mut i32)))
                (import "b" "init" (func $init))
                (import "b" "run" (func $run (result i32)))
                (import "b" "mem" (memory 1))

                (func (export "init")
                    call $init

                    i32.const 1
                    memory.grow
                    i32.const -1
                    i32.eq
                    if unreachable end

                    i32.const 65536
                    i32.const 7
                    i32.store

                    ;; overwrite a#init with a different value, make sure this
                    ;; one is snapshot
                    i32.const 112
                    global.set $g
                )
                (func (export "run") (result i32)
                    i32.const 65536
                    i32.load
                    call $run
                    i32.add
                )
            )
            (core instance $c (instantiate $c
                (with "a" (instance $a))
                (with "b" (instance $b))
            ))

            (func (export "run") (result u32) (canon lift (core func $c "run")))
            (func (export "wizer-initialize") (canon lift (core func $c "init")))
        )"#,
    )
    .await?;

    Ok(())
}

#[tokio::test]
async fn export_is_removed() -> Result<()> {
    let wasm = wizen(
        r#"(component
            (core module $a
                (func (export "init"))
            )
            (core instance $a (instantiate $a))
            (func $a (canon lift (core func $a "init")))
            (export "wizer-initialize" (func $a))
        )"#,
    )
    .await?;

    let names = exports(&wasm);
    assert!(names.is_empty());

    let wasm = wizen(
        r#"(component
            (core module $a
                (func (export "init"))
            )
            (core instance $a (instantiate $a))
            (func $a (canon lift (core func $a "init")))
            (export "other" (func $a))
            (export "wizer-initialize" (func $a))
        )"#,
    )
    .await?;
    let names = exports(&wasm);
    assert_eq!(names, ["other"]);

    let wasm = wizen(
        r#"(component
            (core module $a
                (func (export "init"))
            )
            (core instance $a (instantiate $a))
            (func $a (canon lift (core func $a "init")))
            (export "other1" (func $a))
            (export "wizer-initialize" (func $a))
            (export "other2" (func $a))
        )"#,
    )
    .await?;
    let names = exports(&wasm);
    assert_eq!(names, ["other1", "other2"]);

    let wasm = wizen(
        r#"(component
            (core module $a
                (func (export "init"))
            )
            (core instance $a (instantiate $a))
            (func $a (canon lift (core func $a "init")))
            (export "other1" (func $a))
            (export "other2" (func $a))
            (export "wizer-initialize" (func $a))
        )"#,
    )
    .await?;
    let names = exports(&wasm);
    assert_eq!(names, ["other1", "other2"]);

    let wasm = wizen(
        r#"(component
            (core module $a
                (func (export "init"))
            )
            (core instance $a (instantiate $a))
            (func $a (canon lift (core func $a "init")))
            (export "wizer-initialize" (func $a))
            (export "other1" (func $a))
            (export "other2" (func $a))
        )"#,
    )
    .await?;
    let names = exports(&wasm);
    assert_eq!(names, ["other1", "other2"]);

    let wasm = wizen(
        r#"(component
            (core module $a
                (func (export "init"))
            )
            (core instance $a (instantiate $a))
            (func $a (canon lift (core func $a "init")))
            (export $x "other1" (func $a))
            (export "wizer-initialize" (func $a))
            (export "other2" (func $x))
        )"#,
    )
    .await?;
    let names = exports(&wasm);
    assert_eq!(names, ["other1", "other2"]);

    let wasm = wizen(
        r#"(component
            (import "x" (func))
            (core module $a
                (func (export "init"))
            )
            (core instance $a (instantiate $a))
            (func $a (canon lift (core func $a "init")))
            (export $x "other1" (func $a))
            (export "wizer-initialize" (func $a))
            (export "other2" (func $x))
        )"#,
    )
    .await?;
    let names = exports(&wasm);
    assert_eq!(names, ["other1", "other2"]);

    return Ok(());

    fn exports(wasm: &[u8]) -> Vec<&str> {
        wasmparser::Parser::new(0)
            .parse_all(&wasm)
            .filter_map(|r| r.ok())
            .filter_map(|payload| match payload {
                wasmparser::Payload::ComponentExportSection(s) => Some(s),
                _ => None,
            })
            .flat_map(|section| section.into_iter().map(|e| e.unwrap().name.0))
            .collect()
    }
}
