use anyhow::Result;
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use wasmtime::*;
use wasmtime_wasi::sync::WasiCtxBuilder;

#[test]
#[should_panic = "cannot use `func_new_async` without enabling async support"]
fn async_required() {
    let engine = Engine::default();
    let mut linker = Linker::<()>::new(&engine);
    drop(linker.func_new_async(
        "",
        "",
        FuncType::new(None, None),
        move |_caller, _params, _results| Box::new(async { Ok(()) }),
    ));
}

#[test]
fn wrap_func() -> Result<()> {
    let engine = Engine::default();
    let mut linker = Linker::<()>::new(&engine);
    linker.allow_shadowing(true);

    linker.func_wrap("", "", || {})?;
    linker.func_wrap("m", "f", |_: i32| {})?;
    linker.func_wrap("m", "f2", |_: i32, _: i64| {})?;
    linker.func_wrap("m2", "f", |_: f32, _: f64| {})?;
    linker.func_wrap("m2", "f2", || -> i32 { 0 })?;
    linker.func_wrap("", "", || -> i64 { 0 })?;
    linker.func_wrap("m", "f", || -> f32 { 0.0 })?;
    linker.func_wrap("m2", "f", || -> f64 { 0.0 })?;
    linker.func_wrap("m3", "", || -> Option<ExternRef> { None })?;
    linker.func_wrap("m3", "f", || -> Option<Func> { None })?;

    linker.func_wrap("", "f1", || -> Result<(), Trap> { loop {} })?;
    linker.func_wrap("", "f2", || -> Result<i32, Trap> { loop {} })?;
    linker.func_wrap("", "f3", || -> Result<i64, Trap> { loop {} })?;
    linker.func_wrap("", "f4", || -> Result<f32, Trap> { loop {} })?;
    linker.func_wrap("", "f5", || -> Result<f64, Trap> { loop {} })?;
    linker.func_wrap("", "f6", || -> Result<Option<ExternRef>, Trap> { loop {} })?;
    linker.func_wrap("", "f7", || -> Result<Option<Func>, Trap> { loop {} })?;
    Ok(())
}

#[test]
fn drop_func() -> Result<()> {
    static HITS: AtomicUsize = AtomicUsize::new(0);
    struct A;

    impl Drop for A {
        fn drop(&mut self) {
            HITS.fetch_add(1, SeqCst);
        }
    }

    let engine = Engine::default();
    let mut linker = Linker::<()>::new(&engine);
    linker.allow_shadowing(true);

    let a = A;
    linker.func_wrap("", "", move || {
        drop(&a);
    })?;

    assert_eq!(HITS.load(SeqCst), 0);

    // Define the function again to ensure redefined functions are dropped

    let a = A;
    linker.func_wrap("", "", move || {
        drop(&a);
    })?;

    assert_eq!(HITS.load(SeqCst), 1);

    drop(linker);

    assert_eq!(HITS.load(SeqCst), 2);

    Ok(())
}

#[test]
fn drop_delayed() -> Result<()> {
    static HITS: AtomicUsize = AtomicUsize::new(0);

    struct A;

    impl Drop for A {
        fn drop(&mut self) {
            HITS.fetch_add(1, SeqCst);
        }
    }

    let engine = Engine::default();
    let mut linker = Linker::<()>::new(&engine);

    let a = A;
    linker.func_wrap("", "", move || drop(&a))?;

    assert_eq!(HITS.load(SeqCst), 0);

    let module = Module::new(&engine, &wat::parse_str(r#"(import "" "" (func))"#)?)?;

    let mut store = Store::new(&engine, ());
    let func = linker.get(&mut store, "", Some("")).unwrap();
    Instance::new(&mut store, &module, &[func])?;

    drop(store);

    assert_eq!(HITS.load(SeqCst), 0);

    let mut store = Store::new(&engine, ());
    let func = linker.get(&mut store, "", Some("")).unwrap();
    Instance::new(&mut store, &module, &[func])?;

    drop(store);

    assert_eq!(HITS.load(SeqCst), 0);

    drop(linker);

    assert_eq!(HITS.load(SeqCst), 1);

    Ok(())
}

#[test]
fn signatures_match() -> Result<()> {
    let engine = Engine::default();
    let mut linker = Linker::<()>::new(&engine);

    linker.func_wrap("", "f1", || {})?;
    linker.func_wrap("", "f2", || -> i32 { loop {} })?;
    linker.func_wrap("", "f3", || -> i64 { loop {} })?;
    linker.func_wrap("", "f4", || -> f32 { loop {} })?;
    linker.func_wrap("", "f5", || -> f64 { loop {} })?;
    linker.func_wrap(
        "",
        "f6",
        |_: f32, _: f64, _: i32, _: i64, _: i32, _: Option<ExternRef>, _: Option<Func>| -> f64 {
            loop {}
        },
    )?;

    let mut store = Store::new(&engine, ());

    let f = linker
        .get(&mut store, "", Some("f1"))
        .unwrap()
        .into_func()
        .unwrap();
    assert_eq!(f.ty(&store).params().collect::<Vec<_>>(), &[]);
    assert_eq!(f.ty(&store).results().collect::<Vec<_>>(), &[]);

    let f = linker
        .get(&mut store, "", Some("f2"))
        .unwrap()
        .into_func()
        .unwrap();
    assert_eq!(f.ty(&store).params().collect::<Vec<_>>(), &[]);
    assert_eq!(f.ty(&store).results().collect::<Vec<_>>(), &[ValType::I32]);

    let f = linker
        .get(&mut store, "", Some("f3"))
        .unwrap()
        .into_func()
        .unwrap();
    assert_eq!(f.ty(&store).params().collect::<Vec<_>>(), &[]);
    assert_eq!(f.ty(&store).results().collect::<Vec<_>>(), &[ValType::I64]);

    let f = linker
        .get(&mut store, "", Some("f4"))
        .unwrap()
        .into_func()
        .unwrap();
    assert_eq!(f.ty(&store).params().collect::<Vec<_>>(), &[]);
    assert_eq!(f.ty(&store).results().collect::<Vec<_>>(), &[ValType::F32]);

    let f = linker
        .get(&mut store, "", Some("f5"))
        .unwrap()
        .into_func()
        .unwrap();
    assert_eq!(f.ty(&store).params().collect::<Vec<_>>(), &[]);
    assert_eq!(f.ty(&store).results().collect::<Vec<_>>(), &[ValType::F64]);

    let f = linker
        .get(&mut store, "", Some("f6"))
        .unwrap()
        .into_func()
        .unwrap();
    assert_eq!(
        f.ty(&store).params().collect::<Vec<_>>(),
        &[
            ValType::F32,
            ValType::F64,
            ValType::I32,
            ValType::I64,
            ValType::I32,
            ValType::ExternRef,
            ValType::FuncRef,
        ]
    );
    assert_eq!(f.ty(&store).results().collect::<Vec<_>>(), &[ValType::F64]);

    Ok(())
}

#[test]
fn import_works() -> Result<()> {
    static HITS: AtomicUsize = AtomicUsize::new(0);

    let wasm = wat::parse_str(
        r#"
            (import "" "f1" (func))
            (import "" "f2" (func (param i32) (result i32)))
            (import "" "f3" (func (param i32) (param i64)))
            (import "" "f4" (func (param i32 i64 i32 f32 f64 externref funcref)))

            (func (export "run") (param externref funcref)
                call 0
                i32.const 0
                call 1
                i32.const 1
                i32.add
                i64.const 3
                call 2

                i32.const 100
                i64.const 200
                i32.const 300
                f32.const 400
                f64.const 500
                local.get 0
                local.get 1
                call 3
            )
        "#,
    )?;

    let engine = Engine::default();
    let mut linker = Linker::<()>::new(&engine);

    linker.func_wrap("", "f1", || {
        assert_eq!(HITS.fetch_add(1, SeqCst), 0);
    })?;

    linker.func_wrap("", "f2", |x: i32| -> i32 {
        assert_eq!(x, 0);
        assert_eq!(HITS.fetch_add(1, SeqCst), 1);
        1
    })?;

    linker.func_wrap("", "f3", |x: i32, y: i64| {
        assert_eq!(x, 2);
        assert_eq!(y, 3);
        assert_eq!(HITS.fetch_add(1, SeqCst), 2);
    })?;

    linker.func_wrap(
        "",
        "f4",
        |mut caller: Caller<'_, _>,
         a: i32,
         b: i64,
         c: i32,
         d: f32,
         e: f64,
         f: Option<ExternRef>,
         g: Option<Func>| {
            assert_eq!(a, 100);
            assert_eq!(b, 200);
            assert_eq!(c, 300);
            assert_eq!(d, 400.0);
            assert_eq!(e, 500.0);
            assert_eq!(
                f.as_ref().unwrap().data().downcast_ref::<String>().unwrap(),
                "hello"
            );
            assert_eq!(
                g.as_ref().unwrap().call(&mut caller, &[]).unwrap()[0].unwrap_i32(),
                42
            );
            assert_eq!(HITS.fetch_add(1, SeqCst), 3);
        },
    )?;

    let module = Module::new(&engine, &wasm)?;

    let mut store = Store::new(&engine, ());
    let instance = linker.instantiate(&mut store, &module)?;
    let run = instance.get_func(&mut store, "run").unwrap();
    let funcref = Val::FuncRef(Some(Func::wrap(&mut store, || -> i32 { 42 })));
    run.call(
        &mut store,
        &[
            Val::ExternRef(Some(ExternRef::new("hello".to_string()))),
            funcref,
        ],
    )?;

    assert_eq!(HITS.load(SeqCst), 4);

    Ok(())
}

#[test]
fn call_import_many_args() -> Result<()> {
    let wasm = wat::parse_str(
        r#"
            (import "" "host" (func (param i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)))
            (func (export "run")
                i32.const 1
                i32.const 2
                i32.const 3
                i32.const 4
                i32.const 5
                i32.const 6
                i32.const 7
                i32.const 8
                i32.const 9
                i32.const 10
                call 0
            )
        "#,
    )?;

    let engine = Engine::default();
    let mut linker = Linker::<()>::new(&engine);

    linker.func_wrap(
        "",
        "host",
        |x1: i32,
         x2: i32,
         x3: i32,
         x4: i32,
         x5: i32,
         x6: i32,
         x7: i32,
         x8: i32,
         x9: i32,
         x10: i32| {
            assert_eq!(x1, 1);
            assert_eq!(x2, 2);
            assert_eq!(x3, 3);
            assert_eq!(x4, 4);
            assert_eq!(x5, 5);
            assert_eq!(x6, 6);
            assert_eq!(x7, 7);
            assert_eq!(x8, 8);
            assert_eq!(x9, 9);
            assert_eq!(x10, 10);
        },
    )?;

    let module = Module::new(&engine, &wasm)?;
    let mut store = Store::new(&engine, ());
    let instance = linker.instantiate(&mut store, &module)?;
    let run = instance.get_func(&mut store, "run").unwrap();
    run.call(&mut store, &[])?;

    Ok(())
}

#[test]
fn call_wasm_many_args() -> Result<()> {
    let wasm = wat::parse_str(
        r#"
            (func (export "run") (param i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
                i32.const 1
                get_local 0
                i32.ne
                if
                    unreachable
                end

                i32.const 10
                get_local 9
                i32.ne
                if
                    unreachable
                end
            )

            (func (export "test")
                i32.const 1
                i32.const 2
                i32.const 3
                i32.const 4
                i32.const 5
                i32.const 6
                i32.const 7
                i32.const 8
                i32.const 9
                i32.const 10
                call 0
            )
        "#,
    )?;

    let engine = Engine::default();
    let module = Module::new(&engine, &wasm)?;

    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;

    let run = instance.get_func(&mut store, "run").unwrap();
    run.call(
        &mut store,
        &[
            1.into(),
            2.into(),
            3.into(),
            4.into(),
            5.into(),
            6.into(),
            7.into(),
            8.into(),
            9.into(),
            10.into(),
        ],
    )?;

    let typed_run = instance
        .get_typed_func::<(i32, i32, i32, i32, i32, i32, i32, i32, i32, i32), (), _>(
            &mut store, "run",
        )?;
    typed_run.call(&mut store, (1, 2, 3, 4, 5, 6, 7, 8, 9, 10))?;

    let test = instance.get_func(&mut store, "test").unwrap();
    test.call(&mut store, &[])?;

    Ok(())
}

#[test]
fn trap_smoke() -> Result<()> {
    let engine = Engine::default();
    let mut linker = Linker::<()>::new(&engine);
    linker.func_wrap("", "", || -> Result<(), Trap> { Err(Trap::new("test")) })?;

    let mut store = Store::new(&engine, ());

    let f = linker
        .get(&mut store, "", Some(""))
        .unwrap()
        .into_func()
        .unwrap();

    let err = f.call(&mut store, &[]).unwrap_err().downcast::<Trap>()?;

    assert!(err.to_string().contains("test"));
    assert!(err.i32_exit_status().is_none());

    Ok(())
}

#[test]
fn trap_import() -> Result<()> {
    let wasm = wat::parse_str(
        r#"
            (import "" "" (func))
            (start 0)
        "#,
    )?;

    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    linker.func_wrap("", "", || -> Result<(), Trap> { Err(Trap::new("foo")) })?;

    let module = Module::new(&engine, &wasm)?;
    let mut store = Store::new(&engine, ());

    let trap = linker
        .instantiate(&mut store, &module)
        .err()
        .unwrap()
        .downcast::<Trap>()?;

    assert!(trap.to_string().contains("foo"));

    Ok(())
}

#[test]
fn new_from_signature() -> Result<()> {
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);

    let ty = FuncType::new(None, None);
    linker.func_new("", "f1", ty, |_, _, _| panic!())?;

    let ty = FuncType::new(Some(ValType::I32), Some(ValType::F64));
    linker.func_new("", "f2", ty, |_, _, _| panic!())?;

    let mut store = Store::new(&engine, ());

    let f = linker
        .get(&mut store, "", Some("f1"))
        .unwrap()
        .into_func()
        .unwrap();
    assert!(f.typed::<(), (), _>(&store).is_ok());
    assert!(f.typed::<(), i32, _>(&store).is_err());
    assert!(f.typed::<i32, (), _>(&store).is_err());

    let f = linker
        .get(&mut store, "", Some("f2"))
        .unwrap()
        .into_func()
        .unwrap();
    assert!(f.typed::<(), (), _>(&store).is_err());
    assert!(f.typed::<(), i32, _>(&store).is_err());
    assert!(f.typed::<i32, (), _>(&store).is_err());
    assert!(f.typed::<i32, f64, _>(&store).is_ok());

    Ok(())
}

#[test]
fn call_wrapped_func() -> Result<()> {
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);

    linker.func_wrap("", "f1", |a: i32, b: i64, c: f32, d: f64| {
        assert_eq!(a, 1);
        assert_eq!(b, 2);
        assert_eq!(c, 3.0);
        assert_eq!(d, 4.0);
    })?;

    linker.func_wrap("", "f2", || 1i32)?;

    linker.func_wrap("", "f3", || 2i64)?;

    linker.func_wrap("", "f4", || 3.0f32)?;

    linker.func_wrap("", "f5", || 4.0f64)?;

    let mut store = Store::new(&engine, ());

    let f = linker
        .get(&mut store, "", Some("f1"))
        .unwrap()
        .into_func()
        .unwrap();
    f.call(
        &mut store,
        &[Val::I32(1), Val::I64(2), 3.0f32.into(), 4.0f64.into()],
    )?;
    f.typed::<(i32, i64, f32, f64), (), _>(&store)?
        .call(&mut store, (1, 2, 3.0, 4.0))?;

    let f = linker
        .get(&mut store, "", Some("f2"))
        .unwrap()
        .into_func()
        .unwrap();
    let results = f.call(&mut store, &[])?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].unwrap_i32(), 1);
    assert_eq!(f.typed::<(), i32, _>(&store)?.call(&mut store, ())?, 1);

    let f = linker
        .get(&mut store, "", Some("f3"))
        .unwrap()
        .into_func()
        .unwrap();
    let results = f.call(&mut store, &[])?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].unwrap_i64(), 2);
    assert_eq!(f.typed::<(), i64, _>(&store)?.call(&mut store, ())?, 2);

    let f = linker
        .get(&mut store, "", Some("f4"))
        .unwrap()
        .into_func()
        .unwrap();
    let results = f.call(&mut store, &[])?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].unwrap_f32(), 3.0);
    assert_eq!(f.typed::<(), f32, _>(&store)?.call(&mut store, ())?, 3.0);

    let f = linker
        .get(&mut store, "", Some("f5"))
        .unwrap()
        .into_func()
        .unwrap();
    let results = f.call(&mut store, &[])?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].unwrap_f64(), 4.0);
    assert_eq!(f.typed::<(), f64, _>(&store)?.call(&mut store, ())?, 4.0);

    Ok(())
}

#[test]
fn func_return_nothing() -> Result<()> {
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    let ty = FuncType::new(None, Some(ValType::I32));
    linker.func_new("", "", ty, |_, _, _| Ok(()))?;

    let mut store = Store::new(&engine, ());
    let f = linker
        .get(&mut store, "", Some(""))
        .unwrap()
        .into_func()
        .unwrap();
    let err = f.call(&mut store, &[]).unwrap_err().downcast::<Trap>()?;
    assert!(err
        .to_string()
        .contains("function attempted to return an incompatible value"));
    Ok(())
}

#[test]
fn call_via_funcref() -> Result<()> {
    static HITS: AtomicUsize = AtomicUsize::new(0);

    struct A;

    impl Drop for A {
        fn drop(&mut self) {
            HITS.fetch_add(1, SeqCst);
        }
    }

    let wasm = wat::parse_str(
        r#"
            (table $t 1 funcref)
            (type $add (func (param i32 i32) (result i32)))
            (func (export "call") (param funcref) (result i32 funcref)
                (table.set $t (i32.const 0) (local.get 0))
                (call_indirect (type $add) (i32.const 3) (i32.const 4) (i32.const 0))
                (local.get 0)
            )
        "#,
    )?;

    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    let a = A;
    linker.func_wrap("", "", move |x: i32, y: i32| {
        drop(&a);
        x + y
    })?;

    let module = Module::new(&engine, &wasm)?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;

    let f = linker
        .get(&mut store, "", Some(""))
        .unwrap()
        .into_func()
        .unwrap();
    let results = instance
        .get_func(&mut store, "call")
        .unwrap()
        .call(&mut store, &[f.into()])?;

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].unwrap_i32(), 7);

    {
        let f = results[1].unwrap_funcref().unwrap();
        let results = f.call(&mut store, &[1.into(), 2.into()])?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].unwrap_i32(), 3);
    }

    assert_eq!(HITS.load(SeqCst), 0);

    drop(store);

    assert_eq!(HITS.load(SeqCst), 0);

    drop(linker);

    assert_eq!(HITS.load(SeqCst), 1);

    Ok(())
}

#[test]
fn store_with_context() -> Result<()> {
    struct Ctx {
        called: bool,
    }

    let engine = Engine::default();
    let mut linker = Linker::new(&engine);

    linker.func_wrap("", "", |mut caller: Caller<'_, Ctx>| {
        caller.data_mut().called = true;
    })?;

    let mut store = Store::new(&engine, Ctx { called: false });

    let f = linker
        .get(&mut store, "", Some(""))
        .unwrap()
        .into_func()
        .unwrap();
    f.call(&mut store, &[])?;

    assert!(store.data().called);

    Ok(())
}

#[test]
fn wasi_imports() -> Result<()> {
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;

    let wasm = wat::parse_str(
        r#"
        (import "wasi_snapshot_preview1" "proc_exit" (func $__wasi_proc_exit (param i32)))
        (memory (export "memory") 0)
        (func (export "_start")
            (call $__wasi_proc_exit (i32.const 123))
        )
        "#,
    )?;

    let module = Module::new(&engine, wasm)?;
    let mut store = Store::new(&engine, WasiCtxBuilder::new().build());
    let instance = linker.instantiate(&mut store, &module)?;

    let start = instance.get_typed_func::<(), (), _>(&mut store, "_start")?;
    let trap = start.call(&mut store, ()).unwrap_err();
    assert_eq!(trap.i32_exit_status(), Some(123));

    Ok(())
}
