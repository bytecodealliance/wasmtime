use anyhow::Result;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use wasmtime::*;

#[test]
fn func_constructors() {
    let store = Store::default();
    Func::wrap(&store, || {});
    Func::wrap(&store, |_: i32| {});
    Func::wrap(&store, |_: i32, _: i64| {});
    Func::wrap(&store, |_: f32, _: f64| {});
    Func::wrap(&store, || -> i32 { 0 });
    Func::wrap(&store, || -> i64 { 0 });
    Func::wrap(&store, || -> f32 { 0.0 });
    Func::wrap(&store, || -> f64 { 0.0 });
    Func::wrap(&store, || -> Option<ExternRef> { None });
    Func::wrap(&store, || -> Option<Func> { None });

    Func::wrap(&store, || -> Result<(), Trap> { loop {} });
    Func::wrap(&store, || -> Result<i32, Trap> { loop {} });
    Func::wrap(&store, || -> Result<i64, Trap> { loop {} });
    Func::wrap(&store, || -> Result<f32, Trap> { loop {} });
    Func::wrap(&store, || -> Result<f64, Trap> { loop {} });
    Func::wrap(&store, || -> Result<Option<ExternRef>, Trap> { loop {} });
    Func::wrap(&store, || -> Result<Option<Func>, Trap> { loop {} });
}

#[test]
fn dtor_runs() {
    static HITS: AtomicUsize = AtomicUsize::new(0);

    struct A;

    impl Drop for A {
        fn drop(&mut self) {
            HITS.fetch_add(1, SeqCst);
        }
    }

    let store = Store::default();
    let a = A;
    assert_eq!(HITS.load(SeqCst), 0);
    Func::wrap(&store, move || {
        drop(&a);
    });
    drop(store);
    assert_eq!(HITS.load(SeqCst), 1);
}

#[test]
fn dtor_delayed() -> Result<()> {
    static HITS: AtomicUsize = AtomicUsize::new(0);

    struct A;

    impl Drop for A {
        fn drop(&mut self) {
            HITS.fetch_add(1, SeqCst);
        }
    }

    let store = Store::default();
    let a = A;
    let func = Func::wrap(&store, move || drop(&a));

    assert_eq!(HITS.load(SeqCst), 0);
    let wasm = wat::parse_str(r#"(import "" "" (func))"#)?;
    let module = Module::new(store.engine(), &wasm)?;
    let instance = Instance::new(&store, &module, &[func.into()])?;
    assert_eq!(HITS.load(SeqCst), 0);
    drop((instance, module, store));
    assert_eq!(HITS.load(SeqCst), 1);
    Ok(())
}

#[test]
fn signatures_match() {
    let store = Store::default();

    let f = Func::wrap(&store, || {});
    assert_eq!(f.ty().params().collect::<Vec<_>>(), &[]);
    assert_eq!(f.param_arity(), 0);
    assert_eq!(f.ty().results().collect::<Vec<_>>(), &[]);
    assert_eq!(f.result_arity(), 0);

    let f = Func::wrap(&store, || -> i32 { loop {} });
    assert_eq!(f.ty().params().collect::<Vec<_>>(), &[]);
    assert_eq!(f.ty().results().collect::<Vec<_>>(), &[ValType::I32]);

    let f = Func::wrap(&store, || -> i64 { loop {} });
    assert_eq!(f.ty().params().collect::<Vec<_>>(), &[]);
    assert_eq!(f.ty().results().collect::<Vec<_>>(), &[ValType::I64]);

    let f = Func::wrap(&store, || -> f32 { loop {} });
    assert_eq!(f.ty().params().collect::<Vec<_>>(), &[]);
    assert_eq!(f.ty().results().collect::<Vec<_>>(), &[ValType::F32]);

    let f = Func::wrap(&store, || -> f64 { loop {} });
    assert_eq!(f.ty().params().collect::<Vec<_>>(), &[]);
    assert_eq!(f.ty().results().collect::<Vec<_>>(), &[ValType::F64]);

    let f = Func::wrap(
        &store,
        |_: f32, _: f64, _: i32, _: i64, _: i32, _: Option<ExternRef>, _: Option<Func>| -> f64 {
            loop {}
        },
    );
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
}

#[test]
fn import_works() -> Result<()> {
    static HITS: AtomicUsize = AtomicUsize::new(0);

    let wasm = wat::parse_str(
        r#"
            (import "" "" (func))
            (import "" "" (func (param i32) (result i32)))
            (import "" "" (func (param i32) (param i64)))
            (import "" "" (func (param i32 i64 i32 f32 f64 externref funcref)))

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
    let engine = Engine::new(&config)?;
    let store = Store::new(&engine);
    let module = Module::new(&engine, &wasm)?;
    let instance = Instance::new(
        &store,
        &module,
        &[
            Func::wrap(&store, || {
                assert_eq!(HITS.fetch_add(1, SeqCst), 0);
            })
            .into(),
            Func::wrap(&store, |x: i32| -> i32 {
                assert_eq!(x, 0);
                assert_eq!(HITS.fetch_add(1, SeqCst), 1);
                1
            })
            .into(),
            Func::wrap(&store, |x: i32, y: i64| {
                assert_eq!(x, 2);
                assert_eq!(y, 3);
                assert_eq!(HITS.fetch_add(1, SeqCst), 2);
            })
            .into(),
            Func::wrap(
                &store,
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
            )
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
    let store = Store::default();
    let f = Func::wrap(&store, || -> Result<(), Trap> { Err(Trap::new("test")) });
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
    let store = Store::default();
    let module = Module::new(store.engine(), &wasm)?;
    let trap = Instance::new(
        &store,
        &module,
        &[Func::wrap(&store, || -> Result<(), Trap> { Err(Trap::new("foo")) }).into()],
    )
    .err()
    .unwrap()
    .downcast::<Trap>()?;
    assert!(trap.to_string().contains("foo"));
    Ok(())
}

#[test]
fn get_from_wrapper() {
    let store = Store::default();
    let f = Func::wrap(&store, || {});
    assert!(f.typed::<(), ()>().is_ok());
    assert!(f.typed::<(), i32>().is_err());
    assert!(f.typed::<(), ()>().is_ok());
    assert!(f.typed::<i32, ()>().is_err());
    assert!(f.typed::<i32, i32>().is_err());
    assert!(f.typed::<(i32, i32), ()>().is_err());
    assert!(f.typed::<(i32, i32), i32>().is_err());

    let f = Func::wrap(&store, || -> i32 { loop {} });
    assert!(f.typed::<(), i32>().is_ok());
    let f = Func::wrap(&store, || -> f32 { loop {} });
    assert!(f.typed::<(), f32>().is_ok());
    let f = Func::wrap(&store, || -> f64 { loop {} });
    assert!(f.typed::<(), f64>().is_ok());
    let f = Func::wrap(&store, || -> Option<ExternRef> { loop {} });
    assert!(f.typed::<(), Option<ExternRef>>().is_ok());
    let f = Func::wrap(&store, || -> Option<Func> { loop {} });
    assert!(f.typed::<(), Option<Func>>().is_ok());

    let f = Func::wrap(&store, |_: i32| {});
    assert!(f.typed::<i32, ()>().is_ok());
    assert!(f.typed::<i64, ()>().is_err());
    assert!(f.typed::<f32, ()>().is_err());
    assert!(f.typed::<f64, ()>().is_err());
    let f = Func::wrap(&store, |_: i64| {});
    assert!(f.typed::<i64, ()>().is_ok());
    let f = Func::wrap(&store, |_: f32| {});
    assert!(f.typed::<f32, ()>().is_ok());
    let f = Func::wrap(&store, |_: f64| {});
    assert!(f.typed::<f64, ()>().is_ok());
    let f = Func::wrap(&store, |_: Option<ExternRef>| {});
    assert!(f.typed::<Option<ExternRef>, ()>().is_ok());
    let f = Func::wrap(&store, |_: Option<Func>| {});
    assert!(f.typed::<Option<Func>, ()>().is_ok());
}

#[test]
fn get_from_signature() {
    let store = Store::default();
    let ty = FuncType::new(None, None);
    let f = Func::new(&store, ty, |_, _, _| panic!());
    assert!(f.typed::<(), ()>().is_ok());
    assert!(f.typed::<(), i32>().is_err());
    assert!(f.typed::<i32, ()>().is_err());

    let ty = FuncType::new(Some(ValType::I32), Some(ValType::F64));
    let f = Func::new(&store, ty, |_, _, _| panic!());
    assert!(f.typed::<(), ()>().is_err());
    assert!(f.typed::<(), i32>().is_err());
    assert!(f.typed::<i32, ()>().is_err());
    assert!(f.typed::<i32, f64>().is_ok());
}

#[test]
fn get_from_module() -> anyhow::Result<()> {
    let store = Store::default();
    let module = Module::new(
        store.engine(),
        r#"
            (module
                (func (export "f0"))
                (func (export "f1") (param i32))
                (func (export "f2") (result i32)
                    i32.const 0)
            )

        "#,
    )?;
    let instance = Instance::new(&store, &module, &[])?;
    let f0 = instance.get_func("f0").unwrap();
    assert!(f0.typed::<(), ()>().is_ok());
    assert!(f0.typed::<(), i32>().is_err());
    let f1 = instance.get_func("f1").unwrap();
    assert!(f1.typed::<(), ()>().is_err());
    assert!(f1.typed::<i32, ()>().is_ok());
    assert!(f1.typed::<i32, f32>().is_err());
    let f2 = instance.get_func("f2").unwrap();
    assert!(f2.typed::<(), ()>().is_err());
    assert!(f2.typed::<(), i32>().is_ok());
    assert!(f2.typed::<i32, ()>().is_err());
    assert!(f2.typed::<i32, f32>().is_err());
    Ok(())
}

#[test]
fn call_wrapped_func() -> Result<()> {
    let store = Store::default();
    let f = Func::wrap(&store, |a: i32, b: i64, c: f32, d: f64| {
        assert_eq!(a, 1);
        assert_eq!(b, 2);
        assert_eq!(c, 3.0);
        assert_eq!(d, 4.0);
    });
    f.call(&[Val::I32(1), Val::I64(2), 3.0f32.into(), 4.0f64.into()])?;
    f.typed::<(i32, i64, f32, f64), ()>()?
        .call((1, 2, 3.0, 4.0))?;

    let f = Func::wrap(&store, || 1i32);
    let results = f.call(&[])?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].unwrap_i32(), 1);
    assert_eq!(f.typed::<(), i32>()?.call(())?, 1);

    let f = Func::wrap(&store, || 2i64);
    let results = f.call(&[])?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].unwrap_i64(), 2);
    assert_eq!(f.typed::<(), i64>()?.call(())?, 2);

    let f = Func::wrap(&store, || 3.0f32);
    let results = f.call(&[])?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].unwrap_f32(), 3.0);
    assert_eq!(f.typed::<(), f32>()?.call(())?, 3.0);

    let f = Func::wrap(&store, || 4.0f64);
    let results = f.call(&[])?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].unwrap_f64(), 4.0);
    assert_eq!(f.typed::<(), f64>()?.call(())?, 4.0);
    Ok(())
}

#[test]
fn caller_memory() -> anyhow::Result<()> {
    let store = Store::default();
    let f = Func::wrap(&store, |c: Caller<'_>| {
        assert!(c.get_export("x").is_none());
        assert!(c.get_export("y").is_none());
        assert!(c.get_export("z").is_none());
    });
    f.call(&[])?;

    let f = Func::wrap(&store, |c: Caller<'_>| {
        assert!(c.get_export("x").is_none());
    });
    let module = Module::new(
        store.engine(),
        r#"
            (module
                (import "" "" (func $f))
                (start $f)
            )

        "#,
    )?;
    Instance::new(&store, &module, &[f.into()])?;

    let f = Func::wrap(&store, |c: Caller<'_>| {
        assert!(c.get_export("memory").is_some());
    });
    let module = Module::new(
        store.engine(),
        r#"
            (module
                (import "" "" (func $f))
                (memory (export "memory") 1)
                (start $f)
            )

        "#,
    )?;
    Instance::new(&store, &module, &[f.into()])?;

    let f = Func::wrap(&store, |c: Caller<'_>| {
        assert!(c.get_export("m").is_some());
        assert!(c.get_export("f").is_some());
        assert!(c.get_export("g").is_none());
        assert!(c.get_export("t").is_none());
    });
    let module = Module::new(
        store.engine(),
        r#"
            (module
                (import "" "" (func $f))
                (memory (export "m") 1)
                (func (export "f"))
                (global (export "g") i32 (i32.const 0))
                (table (export "t") 1 funcref)
                (start $f)
            )

        "#,
    )?;
    Instance::new(&store, &module, &[f.into()])?;
    Ok(())
}

#[test]
fn func_write_nothing() -> anyhow::Result<()> {
    let store = Store::default();
    let ty = FuncType::new(None, Some(ValType::I32));
    let f = Func::new(&store, ty, |_, _, _| Ok(()));
    let err = f.call(&[]).unwrap_err().downcast::<Trap>()?;
    assert!(err
        .to_string()
        .contains("function attempted to return an incompatible value"));
    Ok(())
}

#[test]
// Note: Cranelift only supports refrerence types (used in the wasm in this
// test) on x64.
#[cfg(target_arch = "x86_64")]
fn return_cross_store_value() -> anyhow::Result<()> {
    let wasm = wat::parse_str(
        r#"
            (import "" "" (func (result funcref)))

            (func (export "run") (result funcref)
                call 0
            )
        "#,
    )?;
    let mut config = Config::new();
    config.wasm_reference_types(true);
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, &wasm)?;

    let store1 = Store::new(&engine);
    let store2 = Store::new(&engine);

    let store2_func = Func::wrap(&store2, || {});
    let return_cross_store_func = Func::wrap(&store1, move || Some(store2_func.clone()));

    let instance = Instance::new(&store1, &module, &[return_cross_store_func.into()])?;

    let run = instance.get_func("run").unwrap();
    let result = run.call(&[]);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cross-`Store`"));

    Ok(())
}

#[test]
// Note: Cranelift only supports refrerence types (used in the wasm in this
// test) on x64.
#[cfg(target_arch = "x86_64")]
fn pass_cross_store_arg() -> anyhow::Result<()> {
    let mut config = Config::new();
    config.wasm_reference_types(true);
    let engine = Engine::new(&config)?;

    let store1 = Store::new(&engine);
    let store2 = Store::new(&engine);

    let store1_func = Func::wrap(&store1, |_: Option<Func>| {});
    let store2_func = Func::wrap(&store2, || {});

    // Using regular `.call` fails with cross-Store arguments.
    assert!(store1_func
        .call(&[Val::FuncRef(Some(store2_func.clone()))])
        .is_err());

    // And using `.get` followed by a function call also fails with cross-Store
    // arguments.
    let f = store1_func.typed::<Option<Func>, ()>()?;
    let result = f.call(Some(store2_func));
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cross-`Store`"));

    Ok(())
}

#[test]
fn externref_signature_no_reference_types() -> anyhow::Result<()> {
    let store = Store::default();
    Func::wrap(&store, |_: Option<Func>| {});
    Func::new(
        &store,
        FuncType::new(
            [ValType::FuncRef, ValType::ExternRef].iter().cloned(),
            [ValType::FuncRef, ValType::ExternRef].iter().cloned(),
        ),
        |_, _, _| Ok(()),
    );
    Ok(())
}

#[test]
fn trampolines_always_valid() -> anyhow::Result<()> {
    let func = {
        // Compile two modules up front
        let store = Store::default();
        let module1 = Module::new(store.engine(), "(module (import \"\" \"\" (func)))")?;
        let module2 = Module::new(store.engine(), "(module (func (export \"\")))")?;
        // Start instantiating the first module, but this will fail.
        // Historically this registered the module's trampolines with `Store`
        // before the failure, but then after the failure the `Store` didn't
        // hold onto the trampoline.
        drop(Instance::new(&store, &module1, &[]));
        drop(module1);

        // Then instantiate another module which has the same function type (no
        // parameters or results) which tries to use the trampoline defined in
        // the previous module. Then we extract the function and, in another
        // scope where everything is dropped, we call the func.
        let i = Instance::new(&store, &module2, &[])?;
        i.get_func("").unwrap()
    };

    // ... and no segfaults! right? right? ...
    func.call(&[])?;
    Ok(())
}

#[test]
fn typed_multiple_results() -> anyhow::Result<()> {
    let store = Store::default();
    let module = Module::new(
        store.engine(),
        r#"
            (module
                (func (export "f0") (result i32 i64)
                    i32.const 0
                    i64.const 1)
                (func (export "f1") (param i32 i32 i32) (result f32 f64)
                    f32.const 2
                    f64.const 3)
            )

        "#,
    )?;
    let instance = Instance::new(&store, &module, &[])?;
    let f0 = instance.get_func("f0").unwrap();
    assert!(f0.typed::<(), ()>().is_err());
    assert!(f0.typed::<(), (i32, f32)>().is_err());
    assert!(f0.typed::<(), i32>().is_err());
    assert_eq!(f0.typed::<(), (i32, i64)>()?.call(())?, (0, 1));

    let f1 = instance.get_func("f1").unwrap();
    assert_eq!(
        f1.typed::<(i32, i32, i32), (f32, f64)>()?.call((1, 2, 3))?,
        (2., 3.)
    );
    Ok(())
}

#[test]
fn trap_doesnt_leak() -> anyhow::Result<()> {
    struct Canary(Rc<Cell<bool>>);

    impl Drop for Canary {
        fn drop(&mut self) {
            self.0.set(true);
        }
    }

    let store = Store::default();

    // test that `Func::wrap` is correct
    let canary1 = Canary(Rc::new(Cell::new(false)));
    let dtor1_run = canary1.0.clone();
    let f1 = Func::wrap(&store, move || -> Result<(), Trap> {
        drop(&canary1);
        Err(Trap::new(""))
    });
    assert!(f1.typed::<(), ()>()?.call(()).is_err());
    assert!(f1.call(&[]).is_err());

    // test that `Func::new` is correct
    let canary2 = Canary(Rc::new(Cell::new(false)));
    let dtor2_run = canary2.0.clone();
    let f2 = Func::new(&store, FuncType::new(None, None), move |_, _, _| {
        drop(&canary2);
        Err(Trap::new(""))
    });
    assert!(f2.typed::<(), ()>()?.call(()).is_err());
    assert!(f2.call(&[]).is_err());

    // drop everything and ensure dtors are run
    drop((store, f1, f2));
    assert!(dtor1_run.get());
    assert!(dtor2_run.get());
    Ok(())
}
