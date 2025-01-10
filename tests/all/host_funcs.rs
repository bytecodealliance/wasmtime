use anyhow::bail;
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use wasmtime::*;

#[test]
#[should_panic = "cannot use `func_new_async` without enabling async support"]
fn async_required() {
    let engine = Engine::default();
    let mut linker = Linker::<()>::new(&engine);
    drop(linker.func_new_async(
        "",
        "",
        FuncType::new(&engine, None, None),
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
    linker.func_wrap("m3", "", || -> Option<Rooted<ExternRef>> { None })?;
    linker.func_wrap("m3", "f", || -> Option<Func> { None })?;

    linker.func_wrap("", "f1", || -> Result<()> {
        loop {}
    })?;
    linker.func_wrap("", "f2", || -> Result<i32> {
        loop {}
    })?;
    linker.func_wrap("", "f3", || -> Result<i64> {
        loop {}
    })?;
    linker.func_wrap("", "f4", || -> Result<f32> {
        loop {}
    })?;
    linker.func_wrap("", "f5", || -> Result<f64> {
        loop {}
    })?;
    linker.func_wrap("", "f6", || -> Result<Option<Rooted<ExternRef>>> {
        loop {}
    })?;
    linker.func_wrap("", "f7", || -> Result<Option<Func>> {
        loop {}
    })?;
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
        let _ = &a;
    })?;

    assert_eq!(HITS.load(SeqCst), 0);

    // Define the function again to ensure redefined functions are dropped

    let a = A;
    linker.func_wrap("", "", move || {
        let _ = &a;
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
    linker.func_wrap("", "", move || {
        let _ = &a;
    })?;

    assert_eq!(HITS.load(SeqCst), 0);

    let module = Module::new(&engine, &wat::parse_str(r#"(import "" "" (func))"#)?)?;

    let mut store = Store::new(&engine, ());
    let func = linker.get(&mut store, "", "").unwrap();
    Instance::new(&mut store, &module, &[func])?;

    drop(store);

    assert_eq!(HITS.load(SeqCst), 0);

    let mut store = Store::new(&engine, ());
    let func = linker.get(&mut store, "", "").unwrap();
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
    linker.func_wrap("", "f2", || -> i32 {
        loop {}
    })?;
    linker.func_wrap("", "f3", || -> i64 {
        loop {}
    })?;
    linker.func_wrap("", "f4", || -> f32 {
        loop {}
    })?;
    linker.func_wrap("", "f5", || -> f64 {
        loop {}
    })?;
    linker.func_wrap(
        "",
        "f6",
        |_: f32,
         _: f64,
         _: i32,
         _: i64,
         _: i32,
         _: Option<Rooted<ExternRef>>,
         _: Option<Func>|
         -> f64 { loop {} },
    )?;

    let mut store = Store::new(&engine, ());

    let f = linker
        .get(&mut store, "", "f1")
        .unwrap()
        .into_func()
        .unwrap();
    assert_eq!(f.ty(&store).params().len(), 0);
    assert_eq!(f.ty(&store).results().len(), 0);

    let f = linker
        .get(&mut store, "", "f2")
        .unwrap()
        .into_func()
        .unwrap();
    assert_eq!(f.ty(&store).params().len(), 0);
    assert_eq!(f.ty(&store).results().len(), 1);
    assert!(f.ty(&store).results().nth(0).unwrap().is_i32());

    let f = linker
        .get(&mut store, "", "f3")
        .unwrap()
        .into_func()
        .unwrap();
    assert_eq!(f.ty(&store).params().len(), 0);
    assert_eq!(f.ty(&store).results().len(), 1);
    assert!(f.ty(&store).results().nth(0).unwrap().is_i64());

    let f = linker
        .get(&mut store, "", "f4")
        .unwrap()
        .into_func()
        .unwrap();
    assert_eq!(f.ty(&store).params().len(), 0);
    assert_eq!(f.ty(&store).results().len(), 1);
    assert!(f.ty(&store).results().nth(0).unwrap().is_f32());

    let f = linker
        .get(&mut store, "", "f5")
        .unwrap()
        .into_func()
        .unwrap();
    assert_eq!(f.ty(&store).params().len(), 0);
    assert_eq!(f.ty(&store).results().len(), 1);
    assert!(f.ty(&store).results().nth(0).unwrap().is_f64());

    let f = linker
        .get(&mut store, "", "f6")
        .unwrap()
        .into_func()
        .unwrap();

    assert_eq!(f.ty(&store).params().len(), 7);
    assert!(f.ty(&store).params().nth(0).unwrap().is_f32());
    assert!(f.ty(&store).params().nth(1).unwrap().is_f64());
    assert!(f.ty(&store).params().nth(2).unwrap().is_i32());
    assert!(f.ty(&store).params().nth(3).unwrap().is_i64());
    assert!(f.ty(&store).params().nth(4).unwrap().is_i32());
    assert!(f.ty(&store).params().nth(5).unwrap().is_externref());
    assert!(f.ty(&store).params().nth(6).unwrap().is_funcref());

    assert_eq!(f.ty(&store).results().len(), 1);
    assert!(f.ty(&store).results().nth(0).unwrap().is_f64());

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
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
         f: Option<Rooted<ExternRef>>,
         g: Option<Func>| {
            assert_eq!(a, 100);
            assert_eq!(b, 200);
            assert_eq!(c, 300);
            assert_eq!(d, 400.0);
            assert_eq!(e, 500.0);
            assert_eq!(
                f.as_ref()
                    .unwrap()
                    .data(&caller)
                    .unwrap()
                    .unwrap()
                    .downcast_ref::<String>()
                    .unwrap(),
                "hello"
            );
            let mut results = [Val::I32(0)];
            g.as_ref()
                .unwrap()
                .call(&mut caller, &[], &mut results)
                .unwrap();
            assert_eq!(results[0].unwrap_i32(), 42);
            assert_eq!(HITS.fetch_add(1, SeqCst), 3);
        },
    )?;

    let module = Module::new(&engine, &wasm)?;

    let mut store = Store::new(&engine, ());
    let instance = linker.instantiate(&mut store, &module)?;
    let run = instance.get_func(&mut store, "run").unwrap();
    let funcref = Val::FuncRef(Some(Func::wrap(&mut store, || -> i32 { 42 })));
    let externref = Val::ExternRef(Some(ExternRef::new(&mut store, "hello".to_string())?));
    run.call(&mut store, &[externref, funcref], &mut [])?;

    assert_eq!(HITS.load(SeqCst), 4);

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
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
    run.call(&mut store, &[], &mut [])?;

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn call_wasm_many_args() -> Result<()> {
    let wasm = wat::parse_str(
        r#"
            (func (export "run") (param i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
                i32.const 1
                local.get 0
                i32.ne
                if
                    unreachable
                end

                i32.const 10
                local.get 9
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
        &mut [],
    )?;

    let typed_run = instance
        .get_typed_func::<(i32, i32, i32, i32, i32, i32, i32, i32, i32, i32), ()>(
            &mut store, "run",
        )?;
    typed_run.call(&mut store, (1, 2, 3, 4, 5, 6, 7, 8, 9, 10))?;

    let test = instance.get_func(&mut store, "test").unwrap();
    test.call(&mut store, &[], &mut [])?;

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn trap_smoke() -> Result<()> {
    let engine = Engine::default();
    let mut linker = Linker::<()>::new(&engine);
    linker.func_wrap("", "", || -> Result<()> { bail!("test") })?;

    let mut store = Store::new(&engine, ());

    let f = linker.get(&mut store, "", "").unwrap().into_func().unwrap();

    let err = f.call(&mut store, &[], &mut []).unwrap_err();

    assert!(err.to_string().contains("test"));

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn trap_import() -> Result<()> {
    let wasm = wat::parse_str(
        r#"
            (import "" "" (func))
            (start 0)
        "#,
    )?;

    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    linker.func_wrap("", "", || -> Result<()> { bail!("foo") })?;

    let module = Module::new(&engine, &wasm)?;
    let mut store = Store::new(&engine, ());

    let trap = linker.instantiate(&mut store, &module).unwrap_err();

    assert!(trap.to_string().contains("foo"));

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn new_from_signature() -> Result<()> {
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);

    let ty = FuncType::new(&engine, None, None);
    linker.func_new("", "f1", ty, |_, _, _| panic!())?;

    let ty = FuncType::new(&engine, Some(ValType::I32), Some(ValType::F64));
    linker.func_new("", "f2", ty, |_, _, _| panic!())?;

    let mut store = Store::new(&engine, ());

    let f = linker
        .get(&mut store, "", "f1")
        .unwrap()
        .into_func()
        .unwrap();
    assert!(f.typed::<(), ()>(&store).is_ok());
    assert!(f.typed::<(), i32>(&store).is_err());
    assert!(f.typed::<i32, ()>(&store).is_err());

    let f = linker
        .get(&mut store, "", "f2")
        .unwrap()
        .into_func()
        .unwrap();
    assert!(f.typed::<(), ()>(&store).is_err());
    assert!(f.typed::<(), i32>(&store).is_err());
    assert!(f.typed::<i32, ()>(&store).is_err());
    assert!(f.typed::<i32, f64>(&store).is_ok());

    Ok(())
}

#[test]
fn call_wrapped_func() -> Result<()> {
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    let mut results = [Val::I32(0)];

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
        .get(&mut store, "", "f1")
        .unwrap()
        .into_func()
        .unwrap();
    f.call(
        &mut store,
        &[Val::I32(1), Val::I64(2), 3.0f32.into(), 4.0f64.into()],
        &mut [],
    )?;
    f.typed::<(i32, i64, f32, f64), ()>(&store)?
        .call(&mut store, (1, 2, 3.0, 4.0))?;

    let f = linker
        .get(&mut store, "", "f2")
        .unwrap()
        .into_func()
        .unwrap();
    f.call(&mut store, &[], &mut results)?;
    assert_eq!(results[0].unwrap_i32(), 1);
    assert_eq!(f.typed::<(), i32>(&store)?.call(&mut store, ())?, 1);

    let f = linker
        .get(&mut store, "", "f3")
        .unwrap()
        .into_func()
        .unwrap();
    f.call(&mut store, &[], &mut results)?;
    assert_eq!(results[0].unwrap_i64(), 2);
    assert_eq!(f.typed::<(), i64>(&store)?.call(&mut store, ())?, 2);

    let f = linker
        .get(&mut store, "", "f4")
        .unwrap()
        .into_func()
        .unwrap();
    f.call(&mut store, &[], &mut results)?;
    assert_eq!(results[0].unwrap_f32(), 3.0);
    assert_eq!(f.typed::<(), f32>(&store)?.call(&mut store, ())?, 3.0);

    let f = linker
        .get(&mut store, "", "f5")
        .unwrap()
        .into_func()
        .unwrap();
    f.call(&mut store, &[], &mut results)?;
    assert_eq!(results[0].unwrap_f64(), 4.0);
    assert_eq!(f.typed::<(), f64>(&store)?.call(&mut store, ())?, 4.0);

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn func_return_nothing() -> Result<()> {
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    let ty = FuncType::new(&engine, None, Some(ValType::I32));
    linker.func_new("", "", ty, |_, _, _| Ok(()))?;

    let mut store = Store::new(&engine, ());
    let f = linker.get(&mut store, "", "").unwrap().into_func().unwrap();
    let err = f.call(&mut store, &[], &mut [Val::I32(0)]).unwrap_err();
    assert!(
        err.to_string()
            .contains("function attempted to return an incompatible value")
    );
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
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
        let _ = &a;
        x + y
    })?;

    let module = Module::new(&engine, &wasm)?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;

    let f = linker.get(&mut store, "", "").unwrap().into_func().unwrap();
    let mut results = [Val::I32(0), Val::I32(0)];
    instance
        .get_func(&mut store, "call")
        .unwrap()
        .call(&mut store, &[f.into()], &mut results)?;
    assert_eq!(results[0].unwrap_i32(), 7);

    {
        let f = results[1].unwrap_funcref().unwrap();
        let mut results = [Val::I32(0)];
        f.call(&mut store, &[1.into(), 2.into()], &mut results)?;
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

    let f = linker.get(&mut store, "", "").unwrap().into_func().unwrap();
    f.call(&mut store, &[], &mut [])?;

    assert!(store.data().called);

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn wasi_imports() -> Result<()> {
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |t| t)?;

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
    let mut store = Store::new(&engine, wasmtime_wasi::WasiCtxBuilder::new().build_p1());
    let instance = linker.instantiate(&mut store, &module)?;

    let start = instance.get_typed_func::<(), ()>(&mut store, "_start")?;
    let exit = start
        .call(&mut store, ())
        .unwrap_err()
        .downcast::<wasmtime_wasi::I32Exit>()?;
    assert_eq!(exit.0, 123);

    Ok(())
}
