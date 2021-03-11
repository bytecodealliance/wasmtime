use anyhow::Result;
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use wasi_cap_std_sync::WasiCtxBuilder;
use wasmtime::*;
use wasmtime_wasi::Wasi;

#[test]
fn async_required() {
    let mut config = Config::default();
    config.define_host_func_async(
        "",
        "",
        FuncType::new(None, None),
        move |_caller, _params, _results| Box::new(async { Ok(()) }),
    );

    assert_eq!(
        Engine::new(&config)
            .map_err(|e| e.to_string())
            .err()
            .unwrap(),
        "an async host function cannot be defined without async support enabled in the config"
    );
}

#[test]
fn wrap_func() {
    let mut config = Config::default();

    config.wrap_host_func("", "", || {});
    config.wrap_host_func("m", "f", |_: i32| {});
    config.wrap_host_func("m", "f2", |_: i32, _: i64| {});
    config.wrap_host_func("m2", "f", |_: f32, _: f64| {});
    config.wrap_host_func("m2", "f2", || -> i32 { 0 });
    config.wrap_host_func("", "", || -> i64 { 0 });
    config.wrap_host_func("m", "f", || -> f32 { 0.0 });
    config.wrap_host_func("m2", "f", || -> f64 { 0.0 });
    config.wrap_host_func("m3", "", || -> Option<ExternRef> { None });
    config.wrap_host_func("m3", "f", || -> Option<Func> { None });

    config.wrap_host_func("", "f1", || -> Result<(), Trap> { loop {} });
    config.wrap_host_func("", "f2", || -> Result<i32, Trap> { loop {} });
    config.wrap_host_func("", "f3", || -> Result<i64, Trap> { loop {} });
    config.wrap_host_func("", "f4", || -> Result<f32, Trap> { loop {} });
    config.wrap_host_func("", "f5", || -> Result<f64, Trap> { loop {} });
    config.wrap_host_func("", "f6", || -> Result<Option<ExternRef>, Trap> { loop {} });
    config.wrap_host_func("", "f7", || -> Result<Option<Func>, Trap> { loop {} });
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

    let mut config = Config::default();

    let a = A;
    config.wrap_host_func("", "", move || {
        drop(&a);
    });

    assert_eq!(HITS.load(SeqCst), 0);

    // Define the function again to ensure redefined functions are dropped

    let a = A;
    config.wrap_host_func("", "", move || {
        drop(&a);
    });

    assert_eq!(HITS.load(SeqCst), 1);

    drop(config);

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

    let mut config = Config::default();

    let a = A;
    config.wrap_host_func("", "", move || drop(&a));

    assert_eq!(HITS.load(SeqCst), 0);

    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, &wat::parse_str(r#"(import "" "" (func))"#)?)?;

    let store = Store::new(&engine);
    let instance = Instance::new(
        &store,
        &module,
        &[store
            .get_host_func("", "")
            .expect("function should be defined")
            .into()],
    )?;

    drop((instance, store));

    assert_eq!(HITS.load(SeqCst), 0);

    let store = Store::new(&engine);
    let instance = Instance::new(
        &store,
        &module,
        &[store
            .get_host_func("", "")
            .expect("function should be defined")
            .into()],
    )?;

    drop((instance, store, engine, module));

    assert_eq!(HITS.load(SeqCst), 0);

    drop(config);

    assert_eq!(HITS.load(SeqCst), 1);

    Ok(())
}

#[test]
fn signatures_match() -> Result<()> {
    let mut config = Config::default();

    config.wrap_host_func("", "f1", || {});
    config.wrap_host_func("", "f2", || -> i32 { loop {} });
    config.wrap_host_func("", "f3", || -> i64 { loop {} });
    config.wrap_host_func("", "f4", || -> f32 { loop {} });
    config.wrap_host_func("", "f5", || -> f64 { loop {} });
    config.wrap_host_func(
        "",
        "f6",
        |_: f32, _: f64, _: i32, _: i64, _: i32, _: Option<ExternRef>, _: Option<Func>| -> f64 {
            loop {}
        },
    );

    let engine = Engine::new(&config)?;
    let store = Store::new(&engine);

    let f = store
        .get_host_func("", "f1")
        .expect("func should be defined");

    assert_eq!(f.ty().params().collect::<Vec<_>>(), &[]);
    assert_eq!(f.param_arity(), 0);
    assert_eq!(f.ty().results().collect::<Vec<_>>(), &[]);
    assert_eq!(f.result_arity(), 0);

    let f = store
        .get_host_func("", "f2")
        .expect("func should be defined");

    assert_eq!(f.ty().params().collect::<Vec<_>>(), &[]);
    assert_eq!(f.ty().results().collect::<Vec<_>>(), &[ValType::I32]);

    let f = store
        .get_host_func("", "f3")
        .expect("func should be defined");

    assert_eq!(f.ty().params().collect::<Vec<_>>(), &[]);
    assert_eq!(f.ty().results().collect::<Vec<_>>(), &[ValType::I64]);

    let f = store
        .get_host_func("", "f4")
        .expect("func should be defined");

    assert_eq!(f.ty().params().collect::<Vec<_>>(), &[]);
    assert_eq!(f.ty().results().collect::<Vec<_>>(), &[ValType::F32]);

    let f = store
        .get_host_func("", "f5")
        .expect("func should be defined");

    assert_eq!(f.ty().params().collect::<Vec<_>>(), &[]);
    assert_eq!(f.ty().results().collect::<Vec<_>>(), &[ValType::F64]);

    let f = store
        .get_host_func("", "f6")
        .expect("func should be defined");

    assert_eq!(
        f.ty().params().collect::<Vec<_>>(),
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
    assert_eq!(f.ty().results().collect::<Vec<_>>(), &[ValType::F64]);

    Ok(())
}

#[test]
// Note: Cranelift only supports refrerence types (used in the wasm in this
// test) on x64.
#[cfg(target_arch = "x86_64")]
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

    let mut config = Config::new();
    config.wasm_reference_types(true);

    config.wrap_host_func("", "f1", || {
        assert_eq!(HITS.fetch_add(1, SeqCst), 0);
    });

    config.wrap_host_func("", "f2", |x: i32| -> i32 {
        assert_eq!(x, 0);
        assert_eq!(HITS.fetch_add(1, SeqCst), 1);
        1
    });

    config.wrap_host_func("", "f3", |x: i32, y: i64| {
        assert_eq!(x, 2);
        assert_eq!(y, 3);
        assert_eq!(HITS.fetch_add(1, SeqCst), 2);
    });

    config.wrap_host_func(
        "",
        "f4",
        |a: i32, b: i64, c: i32, d: f32, e: f64, f: Option<ExternRef>, g: Option<Func>| {
            assert_eq!(a, 100);
            assert_eq!(b, 200);
            assert_eq!(c, 300);
            assert_eq!(d, 400.0);
            assert_eq!(e, 500.0);
            assert_eq!(
                f.as_ref().unwrap().data().downcast_ref::<String>().unwrap(),
                "hello"
            );
            assert_eq!(g.as_ref().unwrap().call(&[]).unwrap()[0].unwrap_i32(), 42);
            assert_eq!(HITS.fetch_add(1, SeqCst), 3);
        },
    );

    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, &wasm)?;

    let store = Store::new(&engine);
    let instance = Instance::new(
        &store,
        &module,
        &[
            store
                .get_host_func("", "f1")
                .expect("should be defined")
                .into(),
            store
                .get_host_func("", "f2")
                .expect("should be defined")
                .into(),
            store
                .get_host_func("", "f3")
                .expect("should be defined")
                .into(),
            store
                .get_host_func("", "f4")
                .expect("should be defined")
                .into(),
        ],
    )?;

    let run = instance.get_func("run").unwrap();
    run.call(&[
        Val::ExternRef(Some(ExternRef::new("hello".to_string()))),
        Val::FuncRef(Some(Func::wrap(&store, || -> i32 { 42 }))),
    ])?;

    assert_eq!(HITS.load(SeqCst), 4);

    Ok(())
}

#[test]
fn trap_smoke() -> Result<()> {
    let mut config = Config::default();
    config.wrap_host_func("", "", || -> Result<(), Trap> { Err(Trap::new("test")) });

    let engine = Engine::new(&config)?;
    let store = Store::new(&engine);

    let f = store.get_host_func("", "").expect("should be defined");

    let err = f.call(&[]).unwrap_err().downcast::<Trap>()?;

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

    let mut config = Config::default();
    config.wrap_host_func("", "", || -> Result<(), Trap> { Err(Trap::new("foo")) });

    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, &wasm)?;
    let store = Store::new(&engine);

    let trap = Instance::new(
        &store,
        &module,
        &[store.get_host_func("", "").expect("defined").into()],
    )
    .err()
    .unwrap()
    .downcast::<Trap>()?;

    assert!(trap.to_string().contains("foo"));

    Ok(())
}

#[test]
fn new_from_signature() -> Result<()> {
    let mut config = Config::default();

    let ty = FuncType::new(None, None);
    config.define_host_func("", "f1", ty, |_, _, _| panic!());

    let ty = FuncType::new(Some(ValType::I32), Some(ValType::F64));
    config.define_host_func("", "f2", ty, |_, _, _| panic!());

    let engine = Engine::new(&config)?;
    let store = Store::new(&engine);

    let f = store.get_host_func("", "f1").expect("func defined");
    assert!(f.get0::<()>().is_ok());
    assert!(f.get0::<i32>().is_err());
    assert!(f.get1::<i32, ()>().is_err());

    let f = store.get_host_func("", "f2").expect("func defined");
    assert!(f.get0::<()>().is_err());
    assert!(f.get0::<i32>().is_err());
    assert!(f.get1::<i32, ()>().is_err());
    assert!(f.get1::<i32, f64>().is_ok());

    Ok(())
}

#[test]
fn call_wrapped_func() -> Result<()> {
    let mut config = Config::default();

    config.wrap_host_func("", "f1", |a: i32, b: i64, c: f32, d: f64| {
        assert_eq!(a, 1);
        assert_eq!(b, 2);
        assert_eq!(c, 3.0);
        assert_eq!(d, 4.0);
    });

    config.wrap_host_func("", "f2", || 1i32);

    config.wrap_host_func("", "f3", || 2i64);

    config.wrap_host_func("", "f4", || 3.0f32);

    config.wrap_host_func("", "f5", || 4.0f64);

    let engine = Engine::new(&config)?;
    let store = Store::new(&engine);

    let f = store.get_host_func("", "f1").expect("func defined");
    f.call(&[Val::I32(1), Val::I64(2), 3.0f32.into(), 4.0f64.into()])?;
    f.get4::<i32, i64, f32, f64, ()>()?(1, 2, 3.0, 4.0)?;

    let f = store.get_host_func("", "f2").expect("func defined");
    let results = f.call(&[])?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].unwrap_i32(), 1);
    assert_eq!(f.get0::<i32>()?()?, 1);

    let f = store.get_host_func("", "f3").expect("func defined");
    let results = f.call(&[])?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].unwrap_i64(), 2);
    assert_eq!(f.get0::<i64>()?()?, 2);

    let f = store.get_host_func("", "f4").expect("func defined");
    let results = f.call(&[])?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].unwrap_f32(), 3.0);
    assert_eq!(f.get0::<f32>()?()?, 3.0);

    let f = store.get_host_func("", "f5").expect("func defined");
    let results = f.call(&[])?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].unwrap_f64(), 4.0);
    assert_eq!(f.get0::<f64>()?()?, 4.0);

    Ok(())
}

#[test]
fn func_return_nothing() -> Result<()> {
    let mut config = Config::default();
    let ty = FuncType::new(None, Some(ValType::I32));

    config.define_host_func("", "", ty, |_, _, _| Ok(()));

    let engine = Engine::new(&config)?;
    let store = Store::new(&engine);
    let f = store.get_host_func("", "").expect("func defined");
    let err = f.call(&[]).unwrap_err().downcast::<Trap>()?;
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

    let mut config = Config::default();
    let a = A;
    config.wrap_host_func("", "", move |x: i32, y: i32| {
        drop(&a);
        x + y
    });

    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, &wasm)?;
    let store = Store::new(&engine);
    let instance = Instance::new(&store, &module, &[])?;

    let results = instance
        .get_func("call")
        .unwrap()
        .call(&[store.get_host_func("", "").expect("func defined").into()])?;

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].unwrap_i32(), 7);

    {
        let f = results[1].unwrap_funcref().unwrap();
        let results = f.call(&[1.into(), 2.into()])?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].unwrap_i32(), 3);
    }

    assert_eq!(HITS.load(SeqCst), 0);

    drop((results, instance, store, module, engine));

    assert_eq!(HITS.load(SeqCst), 0);

    drop(config);

    assert_eq!(HITS.load(SeqCst), 1);

    Ok(())
}

#[test]
fn store_with_context() -> Result<()> {
    struct Ctx {
        called: std::cell::Cell<bool>,
    }

    let mut config = Config::default();

    config.wrap_host_func("", "", |caller: Caller| {
        let ctx = caller
            .store()
            .get::<Ctx>()
            .expect("store should have context");
        ctx.called.set(true);
    });

    let engine = Engine::new(&config)?;
    let store = Store::new(&engine);
    assert!(store.get::<Ctx>().is_none());
    assert!(store
        .set(Ctx {
            called: std::cell::Cell::new(false)
        })
        .is_ok());
    assert!(store
        .set(Ctx {
            called: std::cell::Cell::new(false)
        })
        .is_err());
    assert!(!store.get::<Ctx>().unwrap().called.get());

    let f = store.get_host_func("", "").expect("func defined");
    f.call(&[])?;

    assert!(store.get::<Ctx>().unwrap().called.get());

    Ok(())
}

#[test]
fn wasi_imports_missing_context() -> Result<()> {
    let mut config = Config::default();
    Wasi::add_to_config(&mut config);

    let wasm = wat::parse_str(
        r#"
        (import "wasi_snapshot_preview1" "proc_exit" (func $__wasi_proc_exit (param i32)))
        (memory (export "memory") 0)
        (func (export "_start")
            (call $__wasi_proc_exit (i32.const 123))
        )
        "#,
    )?;

    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, wasm)?;
    let store = Store::new(&engine);
    let linker = Linker::new(&store);
    let instance = linker.instantiate(&module)?;

    let start = instance.get_func("_start").unwrap().get0::<()>()?;

    let trap = start().unwrap_err();

    assert!(trap.to_string().contains("context is missing in the store"));
    assert!(trap.i32_exit_status().is_none());

    Ok(())
}

#[test]
fn wasi_imports() -> Result<()> {
    let mut config = Config::default();
    Wasi::add_to_config(&mut config);

    let wasm = wat::parse_str(
        r#"
        (import "wasi_snapshot_preview1" "proc_exit" (func $__wasi_proc_exit (param i32)))
        (memory (export "memory") 0)
        (func (export "_start")
            (call $__wasi_proc_exit (i32.const 123))
        )
        "#,
    )?;

    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, wasm)?;
    let store = Store::new(&engine);
    assert!(Wasi::set_context(&store, WasiCtxBuilder::new().build()?).is_ok());
    let linker = Linker::new(&store);
    let instance = linker.instantiate(&module)?;

    let start = instance.get_func("_start").unwrap().get0::<()>()?;

    let trap = start().unwrap_err();
    assert_eq!(trap.i32_exit_status(), Some(123));

    Ok(())
}
