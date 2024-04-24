use anyhow::bail;
use std::sync::atomic::Ordering;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst};
use std::sync::Arc;
use wasmtime::*;

#[test]
#[cfg_attr(miri, ignore)]
fn call_wasm_to_wasm() -> Result<()> {
    let wasm = wat::parse_str(
        r#"
          (module
            (func (result i32 i32 i32)
              i32.const 1
              i32.const 2
              i32.const 3
            )
            (func (export "run") (result i32 i32 i32)
                call 0
            )
          )
        "#,
    )?;
    let mut store = Store::<()>::default();
    let module = Module::new(store.engine(), &wasm)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let func = instance
        .get_typed_func::<(), (i32, i32, i32)>(&mut store, "run")
        .unwrap();
    let results = func.call(&mut store, ())?;
    assert_eq!(results, (1, 2, 3));
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn call_wasm_to_native() -> Result<()> {
    let wasm = wat::parse_str(
        r#"
          (module
            (import "" "" (func (result i32 i32 i32)))
            (func (export "run") (result i32 i32 i32)
                call 0
            )
          )
        "#,
    )?;
    let mut store = Store::<()>::default();
    let module = Module::new(store.engine(), &wasm)?;
    let import_func = Func::wrap(&mut store, || (1_i32, 2_i32, 3_i32));
    let instance = Instance::new(&mut store, &module, &[import_func.into()])?;
    let func = instance
        .get_typed_func::<(), (i32, i32, i32)>(&mut store, "run")
        .unwrap();
    let results = func.call(&mut store, ())?;
    assert_eq!(results, (1, 2, 3));
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn call_wasm_to_array() -> Result<()> {
    let wasm = wat::parse_str(
        r#"
          (module
            (import "" "" (func (result i32 i32 i32)))
            (func (export "run") (result i32 i32 i32)
                call 0
            )
          )
        "#,
    )?;
    let mut store = Store::<()>::default();
    let module = Module::new(store.engine(), &wasm)?;
    let func_ty = FuncType::new(
        store.engine(),
        vec![],
        vec![ValType::I32, ValType::I32, ValType::I32],
    );
    let import_func = Func::new(&mut store, func_ty, |_, _params, results| {
        results[0] = Val::I32(1);
        results[1] = Val::I32(2);
        results[2] = Val::I32(3);
        Ok(())
    });
    let instance = Instance::new(&mut store, &module, &[import_func.into()])?;
    let func = instance
        .get_typed_func::<(), (i32, i32, i32)>(&mut store, "run")
        .unwrap();
    let results = func.call(&mut store, ())?;
    assert_eq!(results, (1, 2, 3));
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn call_native_to_wasm() -> Result<()> {
    let wasm = wat::parse_str(
        r#"
          (module
            (func (export "run") (result i32 i32 i32)
                i32.const 42
                i32.const 420
                i32.const 4200
            )
          )
        "#,
    )?;
    let mut store = Store::<()>::default();
    let module = Module::new(store.engine(), &wasm)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let func = instance
        .get_typed_func::<(), (i32, i32, i32)>(&mut store, "run")
        .unwrap();
    let results = func.call(&mut store, ())?;
    assert_eq!(results, (42, 420, 4200));
    Ok(())
}

#[test]
fn call_native_to_native() -> Result<()> {
    let mut store = Store::<()>::default();

    let func = Func::wrap(&mut store, |a: i32, b: i32, c: i32| -> (i32, i32, i32) {
        (b, c, a)
    });
    let func = func.typed::<(i32, i32, i32), (i32, i32, i32)>(&store)?;
    let results = func.call(&mut store, (1, 2, 3))?;
    assert_eq!(results, (2, 3, 1));
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn call_native_to_array() -> Result<()> {
    let mut store = Store::<()>::default();

    let func_ty = FuncType::new(
        store.engine(),
        [ValType::I32, ValType::I32, ValType::I32],
        [ValType::I32, ValType::I32, ValType::I32],
    );
    let func = Func::new(&mut store, func_ty, |_caller, params, results| {
        results[0] = params[2].clone();
        results[1] = params[0].clone();
        results[2] = params[1].clone();
        Ok(())
    });
    let func = func.typed::<(i32, i32, i32), (i32, i32, i32)>(&store)?;
    let results = func.call(&mut store, (1, 2, 3))?;
    assert_eq!(results, (3, 1, 2));
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn call_array_to_wasm() -> Result<()> {
    let wasm = wat::parse_str(
        r#"
          (module
            (func (export "run") (param i32 i32 i32) (result i32 i32 i32)
              local.get 1
              local.get 2
              local.get 0
            )
          )
        "#,
    )?;
    let mut store = Store::<()>::default();
    let module = Module::new(store.engine(), &wasm)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let func = instance.get_func(&mut store, "run").unwrap();
    let mut results = [Val::I32(0), Val::I32(0), Val::I32(0)];
    func.call(
        &mut store,
        &[Val::I32(10), Val::I32(20), Val::I32(30)],
        &mut results,
    )?;
    assert_eq!(results[0].i32(), Some(20));
    assert_eq!(results[1].i32(), Some(30));
    assert_eq!(results[2].i32(), Some(10));
    Ok(())
}

#[test]
fn call_array_to_native() -> Result<()> {
    let mut store = Store::<()>::default();
    let func = Func::wrap(&mut store, |a: i32, b: i32, c: i32| -> (i32, i32, i32) {
        (a * 10, b * 10, c * 10)
    });
    let mut results = [Val::I32(0), Val::I32(0), Val::I32(0)];
    func.call(
        &mut store,
        &[Val::I32(10), Val::I32(20), Val::I32(30)],
        &mut results,
    )?;
    assert_eq!(results[0].i32(), Some(100));
    assert_eq!(results[1].i32(), Some(200));
    assert_eq!(results[2].i32(), Some(300));
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn call_array_to_array() -> Result<()> {
    let mut store = Store::<()>::default();
    let func_ty = FuncType::new(
        store.engine(),
        [ValType::I32, ValType::I32, ValType::I32],
        [ValType::I32, ValType::I32, ValType::I32],
    );
    let func = Func::new(&mut store, func_ty, |_caller, params, results| {
        results[0] = params[2].clone();
        results[1] = params[0].clone();
        results[2] = params[1].clone();
        Ok(())
    });
    let mut results = [Val::I32(0), Val::I32(0), Val::I32(0)];
    func.call(
        &mut store,
        &[Val::I32(10), Val::I32(20), Val::I32(30)],
        &mut results,
    )?;
    assert_eq!(results[0].i32(), Some(30));
    assert_eq!(results[1].i32(), Some(10));
    assert_eq!(results[2].i32(), Some(20));
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn call_indirect_native_from_wasm_import_global() -> Result<()> {
    let wasm = wat::parse_str(
        r#"
          (module
            (import "" "" (global funcref))
            (table 1 1 funcref)
            (func (export "run") (result i32 i32 i32)
                i32.const 0
                global.get 0
                table.set
                i32.const 0
                call_indirect (result i32 i32 i32)
            )
          )
        "#,
    )?;
    let mut store = Store::<()>::default();
    let module = Module::new(store.engine(), &wasm)?;
    let func = Func::wrap(&mut store, || -> (i32, i32, i32) { (10, 20, 30) });
    let global = Global::new(
        &mut store,
        GlobalType::new(ValType::FUNCREF, Mutability::Const),
        Val::FuncRef(Some(func)),
    )?;
    let instance = Instance::new(&mut store, &module, &[global.into()])?;
    let func = instance.get_typed_func::<(), (i32, i32, i32)>(&mut store, "run")?;
    let results = func.call(&mut store, ())?;
    assert_eq!(results, (10, 20, 30));
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn call_indirect_native_from_wasm_import_table() -> Result<()> {
    let wasm = wat::parse_str(
        r#"
          (module
            (import "" "" (table 1 1 funcref))
            (func (export "run") (result i32 i32 i32)
                i32.const 0
                call_indirect (result i32 i32 i32)
            )
          )
        "#,
    )?;
    let mut store = Store::<()>::default();
    let module = Module::new(store.engine(), &wasm)?;
    let func = Func::wrap(&mut store, || -> (i32, i32, i32) { (10, 20, 30) });
    let table = Table::new(
        &mut store,
        TableType::new(RefType::FUNCREF, 1, Some(1)),
        Ref::Func(Some(func)),
    )?;
    let instance = Instance::new(&mut store, &module, &[table.into()])?;
    let func = instance.get_typed_func::<(), (i32, i32, i32)>(&mut store, "run")?;
    let results = func.call(&mut store, ())?;
    assert_eq!(results, (10, 20, 30));
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn call_indirect_native_from_wasm_import_func_returns_funcref() -> Result<()> {
    let wasm = wat::parse_str(
        r#"
          (module
            (import "" "" (func (result funcref)))
            (table 1 1 funcref)
            (func (export "run") (result i32 i32 i32)
                i32.const 0
                call 0
                table.set
                i32.const 0
                call_indirect (result i32 i32 i32)
            )
          )
        "#,
    )?;
    let mut store = Store::<()>::default();
    let module = Module::new(store.engine(), &wasm)?;
    let func = Func::wrap(&mut store, || -> (i32, i32, i32) { (10, 20, 30) });
    let get_func = Func::wrap(&mut store, move || -> Option<Func> { Some(func) });
    let instance = Instance::new(&mut store, &module, &[get_func.into()])?;
    let func = instance.get_typed_func::<(), (i32, i32, i32)>(&mut store, "run")?;
    let results = func.call(&mut store, ())?;
    assert_eq!(results, (10, 20, 30));
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn call_indirect_native_from_exported_table() -> Result<()> {
    let wasm = wat::parse_str(
        r#"
          (module
            (table (export "table") 1 1 funcref)
            (func (export "run") (result i32 i32 i32)
                i32.const 0
                call_indirect (result i32 i32 i32)
            )
          )
        "#,
    )?;
    let mut store = Store::<()>::default();
    let module = Module::new(store.engine(), &wasm)?;
    let func = Func::wrap(&mut store, || -> (i32, i32, i32) { (10, 20, 30) });
    let instance = Instance::new(&mut store, &module, &[])?;
    let table = instance.get_table(&mut store, "table").unwrap();
    table.set(&mut store, 0, func.into())?;
    let run = instance.get_typed_func::<(), (i32, i32, i32)>(&mut store, "run")?;
    let results = run.call(&mut store, ())?;
    assert_eq!(results, (10, 20, 30));
    Ok(())
}

// wasm exports global, host puts native-call funcref in global, wasm calls funcref
#[test]
#[cfg_attr(miri, ignore)]
fn call_indirect_native_from_exported_global() -> Result<()> {
    let wasm = wat::parse_str(
        r#"
          (module
            (global (export "global") (mut funcref) (ref.null func))
            (table 1 1 funcref)
            (func (export "run") (result i32 i32 i32)
                i32.const 0
                global.get 0
                table.set
                i32.const 0
                call_indirect (result i32 i32 i32)
            )
          )
        "#,
    )?;
    let mut store = Store::<()>::default();
    let module = Module::new(store.engine(), &wasm)?;
    let func = Func::wrap(&mut store, || -> (i32, i32, i32) { (10, 20, 30) });
    let instance = Instance::new(&mut store, &module, &[])?;
    let global = instance.get_global(&mut store, "global").unwrap();
    global.set(&mut store, func.into())?;
    let run = instance.get_typed_func::<(), (i32, i32, i32)>(&mut store, "run")?;
    let results = run.call(&mut store, ())?;
    assert_eq!(results, (10, 20, 30));
    Ok(())
}

#[test]
fn func_constructors() {
    let mut store = Store::<()>::default();
    Func::wrap(&mut store, || {});
    Func::wrap(&mut store, |_: i32| {});
    Func::wrap(&mut store, |_: i32, _: i64| {});
    Func::wrap(&mut store, |_: f32, _: f64| {});
    Func::wrap(&mut store, || -> i32 { 0 });
    Func::wrap(&mut store, || -> i64 { 0 });
    Func::wrap(&mut store, || -> f32 { 0.0 });
    Func::wrap(&mut store, || -> f64 { 0.0 });
    Func::wrap(&mut store, || -> Rooted<ExternRef> { loop {} });
    Func::wrap(&mut store, || -> Option<Rooted<ExternRef>> { None });
    Func::wrap(&mut store, || -> ManuallyRooted<ExternRef> { loop {} });
    Func::wrap(&mut store, || -> Option<ManuallyRooted<ExternRef>> { None });
    Func::wrap(&mut store, || -> Rooted<AnyRef> { loop {} });
    Func::wrap(&mut store, || -> Option<Rooted<AnyRef>> { None });
    Func::wrap(&mut store, || -> ManuallyRooted<AnyRef> { loop {} });
    Func::wrap(&mut store, || -> Option<ManuallyRooted<AnyRef>> { None });
    Func::wrap(&mut store, || -> I31 { loop {} });
    Func::wrap(&mut store, || -> Option<I31> { None });
    Func::wrap(&mut store, || -> Func { loop {} });
    Func::wrap(&mut store, || -> Option<Func> { None });
    Func::wrap(&mut store, || -> NoFunc { loop {} });
    Func::wrap(&mut store, || -> Option<NoFunc> { None });

    Func::wrap(&mut store, || -> Result<()> { loop {} });
    Func::wrap(&mut store, || -> Result<i32> { loop {} });
    Func::wrap(&mut store, || -> Result<i64> { loop {} });
    Func::wrap(&mut store, || -> Result<f32> { loop {} });
    Func::wrap(&mut store, || -> Result<f64> { loop {} });
    Func::wrap(&mut store, || -> Result<Rooted<ExternRef>> { loop {} });
    Func::wrap(&mut store, || -> Result<Option<Rooted<ExternRef>>> {
        loop {}
    });
    Func::wrap(&mut store, || -> Result<ManuallyRooted<ExternRef>> {
        loop {}
    });
    Func::wrap(
        &mut store,
        || -> Result<Option<ManuallyRooted<ExternRef>>> { loop {} },
    );
    Func::wrap(&mut store, || -> Result<Rooted<AnyRef>> { loop {} });
    Func::wrap(&mut store, || -> Result<Option<Rooted<AnyRef>>> { loop {} });
    Func::wrap(&mut store, || -> Result<ManuallyRooted<AnyRef>> { loop {} });
    Func::wrap(&mut store, || -> Result<Option<ManuallyRooted<AnyRef>>> {
        loop {}
    });
    Func::wrap(&mut store, || -> Result<I31> { loop {} });
    Func::wrap(&mut store, || -> Result<Option<I31>> { loop {} });
    Func::wrap(&mut store, || -> Result<Func> { loop {} });
    Func::wrap(&mut store, || -> Result<Option<Func>> { loop {} });
    Func::wrap(&mut store, || -> Result<NoFunc> { loop {} });
    Func::wrap(&mut store, || -> Result<Option<NoFunc>> { loop {} });
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

    let mut store = Store::<()>::default();
    let a = A;
    assert_eq!(HITS.load(SeqCst), 0);
    Func::wrap(&mut store, move || {
        let _ = &a;
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

    let mut store = Store::<()>::default();
    let a = A;
    let func = Func::wrap(&mut store, move || {
        let _ = &a;
    });

    assert_eq!(HITS.load(SeqCst), 0);
    let wasm = wat::parse_str(r#"(import "" "" (func))"#)?;
    let module = Module::new(store.engine(), &wasm)?;
    let _instance = Instance::new(&mut store, &module, &[func.into()])?;
    assert_eq!(HITS.load(SeqCst), 0);
    drop(store);
    assert_eq!(HITS.load(SeqCst), 1);
    Ok(())
}

#[test]
fn signatures_match() {
    let mut store = Store::<()>::default();

    let f = Func::wrap(&mut store, || {});
    assert_eq!(f.ty(&store).params().len(), 0);
    assert_eq!(f.ty(&store).results().len(), 0);

    let f = Func::wrap(&mut store, || -> i32 { loop {} });
    assert_eq!(f.ty(&store).params().len(), 0);
    assert_eq!(f.ty(&store).results().len(), 1);
    assert!(f.ty(&store).results().nth(0).unwrap().is_i32());

    let f = Func::wrap(&mut store, || -> i64 { loop {} });
    assert_eq!(f.ty(&store).params().len(), 0);
    assert_eq!(f.ty(&store).results().len(), 1);
    assert!(f.ty(&store).results().nth(0).unwrap().is_i64());

    let f = Func::wrap(&mut store, || -> f32 { loop {} });
    assert_eq!(f.ty(&store).params().len(), 0);
    assert_eq!(f.ty(&store).results().len(), 1);
    assert!(f.ty(&store).results().nth(0).unwrap().is_f32());

    let f = Func::wrap(&mut store, || -> f64 { loop {} });
    assert_eq!(f.ty(&store).params().len(), 0);
    assert_eq!(f.ty(&store).results().len(), 1);
    assert!(f.ty(&store).results().nth(0).unwrap().is_f64());

    let f = Func::wrap(
        &mut store,
        |_: f32,
         _: f64,
         _: i32,
         _: i64,
         _: i32,
         _: Option<Rooted<ExternRef>>,
         _: Option<ManuallyRooted<ExternRef>>,
         _: Option<Rooted<AnyRef>>,
         _: Option<ManuallyRooted<AnyRef>>,
         _: Option<Func>|
         -> f64 { loop {} },
    );

    assert_eq!(f.ty(&store).params().len(), 10);
    assert!(f.ty(&store).params().nth(0).unwrap().is_f32());
    assert!(f.ty(&store).params().nth(1).unwrap().is_f64());
    assert!(f.ty(&store).params().nth(2).unwrap().is_i32());
    assert!(f.ty(&store).params().nth(3).unwrap().is_i64());
    assert!(f.ty(&store).params().nth(4).unwrap().is_i32());
    assert!(f.ty(&store).params().nth(5).unwrap().is_externref());
    assert!(f.ty(&store).params().nth(6).unwrap().is_externref());
    assert!(f.ty(&store).params().nth(7).unwrap().is_anyref());
    assert!(f.ty(&store).params().nth(8).unwrap().is_anyref());
    assert!(f.ty(&store).params().nth(9).unwrap().is_funcref());

    assert_eq!(f.ty(&store).results().len(), 1);
    assert!(f.ty(&store).results().nth(0).unwrap().is_f64());
}

#[test]
#[cfg_attr(miri, ignore)]
fn import_works() -> Result<()> {
    let _ = env_logger::try_init();

    static HITS: AtomicUsize = AtomicUsize::new(0);

    let wasm = wat::parse_str(
        r#"
            (import "" "" (func))
            (import "" "" (func (param i32) (result i32)))
            (import "" "" (func (param i32) (param i64)))
            (import "" "" (func (param i32 i64 i32 f32 f64 externref externref funcref anyref anyref i31ref)))

            (func (export "run") (param externref externref funcref)
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
                local.get 2
                (ref.i31 (i32.const 36))
                (ref.i31 (i32.const 42))
                (ref.i31 (i32.const 0x1234))
                call 3
            )
        "#,
    )?;

    let mut config = Config::new();
    config.wasm_reference_types(true);
    config.wasm_function_references(true);
    config.wasm_gc(true);

    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());
    let module = Module::new(&engine, &wasm)?;

    let imports = [
        Func::wrap(&mut store, || {
            assert_eq!(HITS.fetch_add(1, SeqCst), 0);
        })
        .into(),
        Func::wrap(&mut store, |x: i32| -> i32 {
            assert_eq!(x, 0);
            assert_eq!(HITS.fetch_add(1, SeqCst), 1);
            1
        })
        .into(),
        Func::wrap(&mut store, |x: i32, y: i64| {
            assert_eq!(x, 2);
            assert_eq!(y, 3);
            assert_eq!(HITS.fetch_add(1, SeqCst), 2);
        })
        .into(),
        Func::wrap(
            &mut store,
            |mut caller: Caller<'_, ()>,
             a: i32,
             b: i64,
             c: i32,
             d: f32,
             e: f64,
             f: Option<Rooted<ExternRef>>,
             g: Option<ManuallyRooted<ExternRef>>,
             h: Option<Func>,
             i: Option<Rooted<AnyRef>>,
             j: Option<ManuallyRooted<AnyRef>>,
             k: Option<I31>|
             -> Result<()> {
                assert_eq!(a, 100);
                assert_eq!(b, 200);
                assert_eq!(c, 300);
                assert_eq!(d, 400.0);
                assert_eq!(e, 500.0);
                assert_eq!(
                    f.as_ref()
                        .unwrap()
                        .data(&caller)?
                        .downcast_ref::<String>()
                        .unwrap(),
                    "hello"
                );
                assert_eq!(
                    g.as_ref()
                        .unwrap()
                        .data(&caller)?
                        .downcast_ref::<String>()
                        .unwrap(),
                    "goodbye"
                );
                assert_eq!(
                    i.unwrap().as_i31(&caller).unwrap().unwrap(),
                    I31::wrapping_u32(36)
                );
                assert_eq!(
                    j.unwrap().as_i31(&caller).unwrap().unwrap(),
                    I31::wrapping_u32(42)
                );
                assert_eq!(k, Some(I31::wrapping_u32(0x1234)));
                let mut results = [Val::I32(0)];
                h.as_ref()
                    .unwrap()
                    .call(&mut caller, &[], &mut results)
                    .unwrap();
                assert_eq!(results[0].unwrap_i32(), 42);
                assert_eq!(HITS.fetch_add(1, SeqCst), 3);
                Ok(())
            },
        )
        .into(),
    ];

    let instance = Instance::new(&mut store, &module, &imports)?;
    let run = instance.get_func(&mut store, "run").unwrap();
    let hello = Val::ExternRef(Some(ExternRef::new(&mut store, "hello".to_string())?));
    let goodbye = Val::ExternRef(Some(ExternRef::new(&mut store, "goodbye".to_string())?));
    let funcref = Val::FuncRef(Some(Func::wrap(&mut store, || -> i32 { 42 })));
    run.call(&mut store, &[hello, goodbye, funcref], &mut [])?;

    assert_eq!(HITS.load(SeqCst), 4);
    Ok(())
}

#[test]
fn trap_smoke() -> Result<()> {
    let mut store = Store::<()>::default();
    let f = Func::wrap(&mut store, || -> Result<()> { bail!("test") });
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
    let mut store = Store::<()>::default();
    let module = Module::new(store.engine(), &wasm)?;
    let import = Func::wrap(&mut store, || -> Result<()> { bail!("foo") });
    let trap = Instance::new(&mut store, &module, &[import.into()]).unwrap_err();
    assert!(trap.to_string().contains("foo"));
    Ok(())
}

#[test]
fn get_from_wrapper() {
    let mut store = Store::<()>::default();
    let f = Func::wrap(&mut store, || {});
    assert!(f.typed::<(), ()>(&store).is_ok());
    assert!(f.typed::<(), i32>(&store).is_err());
    assert!(f.typed::<(), ()>(&store).is_ok());
    assert!(f.typed::<i32, ()>(&store).is_err());
    assert!(f.typed::<i32, i32>(&store).is_err());
    assert!(f.typed::<(i32, i32), ()>(&store).is_err());
    assert!(f.typed::<(i32, i32), i32>(&store).is_err());

    let f = Func::wrap(&mut store, || -> i32 { loop {} });
    assert!(f.typed::<(), i32>(&store).is_ok());
    let f = Func::wrap(&mut store, || -> f32 { loop {} });
    assert!(f.typed::<(), f32>(&store).is_ok());
    let f = Func::wrap(&mut store, || -> f64 { loop {} });
    assert!(f.typed::<(), f64>(&store).is_ok());
    let f = Func::wrap(&mut store, || -> Rooted<ExternRef> { loop {} });
    assert!(f.typed::<(), Rooted<ExternRef>>(&store).is_ok());
    let f = Func::wrap(&mut store, || -> Option<Rooted<ExternRef>> { loop {} });
    assert!(f.typed::<(), Option<Rooted<ExternRef>>>(&store).is_ok());
    let f = Func::wrap(&mut store, || -> ManuallyRooted<ExternRef> { loop {} });
    assert!(f.typed::<(), ManuallyRooted<ExternRef>>(&store).is_ok());
    let f = Func::wrap(&mut store, || -> Option<ManuallyRooted<ExternRef>> {
        loop {}
    });
    assert!(f
        .typed::<(), Option<ManuallyRooted<ExternRef>>>(&store)
        .is_ok());
    let f = Func::wrap(&mut store, || -> Rooted<AnyRef> { loop {} });
    assert!(f.typed::<(), Rooted<AnyRef>>(&store).is_ok());
    let f = Func::wrap(&mut store, || -> Option<Rooted<AnyRef>> { loop {} });
    assert!(f.typed::<(), Option<Rooted<AnyRef>>>(&store).is_ok());
    let f = Func::wrap(&mut store, || -> ManuallyRooted<AnyRef> { loop {} });
    assert!(f.typed::<(), ManuallyRooted<AnyRef>>(&store).is_ok());
    let f = Func::wrap(&mut store, || -> Option<ManuallyRooted<AnyRef>> { loop {} });
    assert!(f
        .typed::<(), Option<ManuallyRooted<AnyRef>>>(&store)
        .is_ok());
    let f = Func::wrap(&mut store, || -> I31 { loop {} });
    assert!(f.typed::<(), I31>(&store).is_ok());
    let f = Func::wrap(&mut store, || -> Option<I31> { loop {} });
    assert!(f.typed::<(), Option<I31>>(&store).is_ok());
    let f = Func::wrap(&mut store, || -> Func { loop {} });
    assert!(f.typed::<(), Func>(&store).is_ok());
    let f = Func::wrap(&mut store, || -> Option<Func> { loop {} });
    assert!(f.typed::<(), Option<Func>>(&store).is_ok());

    let f = Func::wrap(&mut store, |_: i32| {});
    assert!(f.typed::<i32, ()>(&store).is_ok());
    assert!(f.typed::<i64, ()>(&store).is_err());
    assert!(f.typed::<f32, ()>(&store).is_err());
    assert!(f.typed::<f64, ()>(&store).is_err());
    let f = Func::wrap(&mut store, |_: i64| {});
    assert!(f.typed::<i64, ()>(&store).is_ok());
    let f = Func::wrap(&mut store, |_: f32| {});
    assert!(f.typed::<f32, ()>(&store).is_ok());
    let f = Func::wrap(&mut store, |_: f64| {});
    assert!(f.typed::<f64, ()>(&store).is_ok());
    let f = Func::wrap(&mut store, |_: Rooted<ExternRef>| {});
    assert!(f.typed::<Rooted<ExternRef>, ()>(&store).is_ok());
    let f = Func::wrap(&mut store, |_: Option<Rooted<ExternRef>>| {});
    assert!(f.typed::<Option<Rooted<ExternRef>>, ()>(&store).is_ok());
    let f = Func::wrap(&mut store, |_: ManuallyRooted<ExternRef>| {});
    assert!(f.typed::<ManuallyRooted<ExternRef>, ()>(&store).is_ok());
    let f = Func::wrap(&mut store, |_: Option<ManuallyRooted<ExternRef>>| {});
    assert!(f
        .typed::<Option<ManuallyRooted<ExternRef>>, ()>(&store)
        .is_ok());
    let f = Func::wrap(&mut store, |_: Rooted<AnyRef>| {});
    assert!(f.typed::<Rooted<AnyRef>, ()>(&store).is_ok());
    let f = Func::wrap(&mut store, |_: Option<Rooted<AnyRef>>| {});
    assert!(f.typed::<Option<Rooted<AnyRef>>, ()>(&store).is_ok());
    let f = Func::wrap(&mut store, |_: ManuallyRooted<AnyRef>| {});
    assert!(f.typed::<ManuallyRooted<AnyRef>, ()>(&store).is_ok());
    let f = Func::wrap(&mut store, |_: Option<ManuallyRooted<AnyRef>>| {});
    assert!(f
        .typed::<Option<ManuallyRooted<AnyRef>>, ()>(&store)
        .is_ok());
    let f = Func::wrap(&mut store, |_: I31| {});
    assert!(f.typed::<I31, ()>(&store).is_ok());
    let f = Func::wrap(&mut store, |_: Option<I31>| {});
    assert!(f.typed::<Option<I31>, ()>(&store).is_ok());
    let f = Func::wrap(&mut store, |_: Func| {});
    assert!(f.typed::<Func, ()>(&store).is_ok());
    let f = Func::wrap(&mut store, |_: Option<Func>| {});
    assert!(f.typed::<Option<Func>, ()>(&store).is_ok());
}

#[test]
#[cfg_attr(miri, ignore)]
fn get_from_signature() {
    let mut store = Store::<()>::default();
    let ty = FuncType::new(store.engine(), None, None);
    let f = Func::new(&mut store, ty, |_, _, _| panic!());
    assert!(f.typed::<(), ()>(&store).is_ok());
    assert!(f.typed::<(), i32>(&store).is_err());
    assert!(f.typed::<i32, ()>(&store).is_err());

    let ty = FuncType::new(store.engine(), Some(ValType::I32), Some(ValType::F64));
    let f = Func::new(&mut store, ty, |_, _, _| panic!());
    assert!(f.typed::<(), ()>(&store).is_err());
    assert!(f.typed::<(), i32>(&store).is_err());
    assert!(f.typed::<i32, ()>(&store).is_err());
    assert!(f.typed::<i32, f64>(&store).is_ok());
}

#[test]
#[cfg_attr(miri, ignore)]
fn get_from_module() -> anyhow::Result<()> {
    let mut store = Store::<()>::default();
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
    let instance = Instance::new(&mut store, &module, &[])?;
    let f0 = instance.get_func(&mut store, "f0").unwrap();
    assert!(f0.typed::<(), ()>(&store).is_ok());
    assert!(f0.typed::<(), i32>(&store).is_err());
    let f1 = instance.get_func(&mut store, "f1").unwrap();
    assert!(f1.typed::<(), ()>(&store).is_err());
    assert!(f1.typed::<i32, ()>(&store).is_ok());
    assert!(f1.typed::<i32, f32>(&store).is_err());
    let f2 = instance.get_func(&mut store, "f2").unwrap();
    assert!(f2.typed::<(), ()>(&store).is_err());
    assert!(f2.typed::<(), i32>(&store).is_ok());
    assert!(f2.typed::<i32, ()>(&store).is_err());
    assert!(f2.typed::<i32, f32>(&store).is_err());
    Ok(())
}

#[test]
fn call_wrapped_func() -> Result<()> {
    let mut store = Store::<()>::default();
    let f = Func::wrap(&mut store, |a: i32, b: i64, c: f32, d: f64| {
        assert_eq!(a, 1);
        assert_eq!(b, 2);
        assert_eq!(c, 3.0);
        assert_eq!(d, 4.0);
    });
    f.call(
        &mut store,
        &[Val::I32(1), Val::I64(2), 3.0f32.into(), 4.0f64.into()],
        &mut [],
    )?;
    f.typed::<(i32, i64, f32, f64), ()>(&store)?
        .call(&mut store, (1, 2, 3.0, 4.0))?;

    let mut results = [Val::I32(0)];
    let f = Func::wrap(&mut store, || 1i32);
    f.call(&mut store, &[], &mut results)?;
    assert_eq!(results[0].unwrap_i32(), 1);
    assert_eq!(f.typed::<(), i32>(&store)?.call(&mut store, ())?, 1);

    let f = Func::wrap(&mut store, || 2i64);
    f.call(&mut store, &[], &mut results)?;
    assert_eq!(results[0].unwrap_i64(), 2);
    assert_eq!(f.typed::<(), i64>(&store)?.call(&mut store, ())?, 2);

    let f = Func::wrap(&mut store, || 3.0f32);
    f.call(&mut store, &[], &mut results)?;
    assert_eq!(results[0].unwrap_f32(), 3.0);
    assert_eq!(f.typed::<(), f32>(&store)?.call(&mut store, ())?, 3.0);

    let f = Func::wrap(&mut store, || 4.0f64);
    f.call(&mut store, &[], &mut results)?;
    assert_eq!(results[0].unwrap_f64(), 4.0);
    assert_eq!(f.typed::<(), f64>(&store)?.call(&mut store, ())?, 4.0);
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn caller_memory() -> anyhow::Result<()> {
    let mut store = Store::<()>::default();
    let f = Func::wrap(&mut store, |mut c: Caller<'_, ()>| {
        assert!(c.get_export("x").is_none());
        assert!(c.get_export("y").is_none());
        assert!(c.get_export("z").is_none());
    });
    f.call(&mut store, &[], &mut [])?;

    let f = Func::wrap(&mut store, |mut c: Caller<'_, ()>| {
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
    Instance::new(&mut store, &module, &[f.into()])?;

    let f = Func::wrap(&mut store, |mut c: Caller<'_, ()>| {
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
    Instance::new(&mut store, &module, &[f.into()])?;

    let f = Func::wrap(&mut store, |mut c: Caller<'_, ()>| {
        assert!(c.get_export("m").is_some());
        assert!(c.get_export("f").is_some());
        assert!(c.get_export("g").is_some());
        assert!(c.get_export("t").is_some());
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
    Instance::new(&mut store, &module, &[f.into()])?;
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn func_write_nothing() -> anyhow::Result<()> {
    let mut store = Store::<()>::default();
    let ty = FuncType::new(store.engine(), None, Some(ValType::I32));
    let f = Func::new(&mut store, ty, |_, _, _| Ok(()));
    let err = f.call(&mut store, &[], &mut [Val::I32(0)]).unwrap_err();
    assert!(err
        .to_string()
        .contains("function attempted to return an incompatible value"));
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn return_cross_store_value() -> anyhow::Result<()> {
    let _ = env_logger::try_init();

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

    let mut store1 = Store::new(&engine, ());
    let mut store2 = Store::new(&engine, ());

    let store2_func = Func::wrap(&mut store2, || {});
    let return_cross_store_func = Func::wrap(&mut store1, move || Some(store2_func.clone()));

    let instance = Instance::new(&mut store1, &module, &[return_cross_store_func.into()])?;

    let run = instance.get_func(&mut store1, "run").unwrap();
    let result = run.call(&mut store1, &[], &mut [Val::I32(0)]);
    assert!(result.is_err());
    assert!(format!("{:?}", result.unwrap_err()).contains("cross-`Store`"));

    Ok(())
}

#[test]
fn pass_cross_store_arg() -> anyhow::Result<()> {
    let mut config = Config::new();
    config.wasm_reference_types(true);
    let engine = Engine::new(&config)?;

    let mut store1 = Store::new(&engine, ());
    let mut store2 = Store::new(&engine, ());

    let store1_func = Func::wrap(&mut store1, |_: Option<Func>| {});
    let store2_func = Func::wrap(&mut store2, || {});

    // Using regular `.call` fails with cross-Store arguments.
    assert!(store1_func
        .call(
            &mut store1,
            &[Val::FuncRef(Some(store2_func.clone()))],
            &mut []
        )
        .is_err());

    // And using `.get` followed by a function call also fails with cross-Store
    // arguments.
    let f = store1_func.typed::<Option<Func>, ()>(&store1)?;
    let result = f.call(&mut store1, Some(store2_func));
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cross-`Store`"));

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn externref_signature_no_reference_types() -> anyhow::Result<()> {
    let mut config = Config::new();
    config.wasm_reference_types(false);
    let mut store = Store::new(&Engine::new(&config)?, ());
    Func::wrap(&mut store, |_: Option<Func>| {});
    let func_ty = FuncType::new(
        store.engine(),
        [ValType::FUNCREF, ValType::EXTERNREF].iter().cloned(),
        [ValType::FUNCREF, ValType::EXTERNREF].iter().cloned(),
    );
    Func::new(&mut store, func_ty, |_, _, _| Ok(()));
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn trampolines_always_valid() -> anyhow::Result<()> {
    // Compile two modules up front
    let mut store = Store::<()>::default();
    let module1 = Module::new(store.engine(), "(module (import \"\" \"\" (func)))")?;
    let module2 = Module::new(store.engine(), "(module (func (export \"\")))")?;
    // Start instantiating the first module, but this will fail.
    // Historically this registered the module's trampolines with `Store`
    // before the failure, but then after the failure the `Store` didn't
    // hold onto the trampoline.
    drop(Instance::new(&mut store, &module1, &[]));
    drop(module1);

    // Then instantiate another module which has the same function type (no
    // parameters or results) which tries to use the trampoline defined in
    // the previous module. Then we extract the function and, after we drop the
    // module's reference, we call the func.
    let i = Instance::new(&mut store, &module2, &[])?;
    let func = i.get_func(&mut store, "").unwrap();
    drop(module2);

    // ... and no segfaults! right? right? ...
    func.call(&mut store, &[], &mut [])?;
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn typed_multiple_results() -> anyhow::Result<()> {
    let mut store = Store::<()>::default();
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
    let instance = Instance::new(&mut store, &module, &[])?;
    let f0 = instance.get_func(&mut store, "f0").unwrap();
    assert!(f0.typed::<(), ()>(&store).is_err());
    assert!(f0.typed::<(), (i32, f32)>(&store).is_err());
    assert!(f0.typed::<(), i32>(&store).is_err());
    assert_eq!(
        f0.typed::<(), (i32, i64)>(&store)?.call(&mut store, ())?,
        (0, 1)
    );

    let f1 = instance.get_func(&mut store, "f1").unwrap();
    assert_eq!(
        f1.typed::<(i32, i32, i32), (f32, f64)>(&store)?
            .call(&mut store, (1, 2, 3))?,
        (2., 3.)
    );
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn trap_doesnt_leak() -> anyhow::Result<()> {
    #[derive(Default)]
    struct Canary(Arc<AtomicBool>);

    impl Drop for Canary {
        fn drop(&mut self) {
            self.0.store(true, SeqCst);
        }
    }

    let mut store = Store::<()>::default();

    // test that `Func::wrap` is correct
    let canary1 = Canary::default();
    let dtor1_run = canary1.0.clone();
    let f1 = Func::wrap(&mut store, move || -> Result<()> {
        let _ = &canary1;
        bail!("")
    });
    assert!(f1.typed::<(), ()>(&store)?.call(&mut store, ()).is_err());
    assert!(f1.call(&mut store, &[], &mut []).is_err());

    // test that `Func::new` is correct
    let canary2 = Canary::default();
    let dtor2_run = canary2.0.clone();
    let func_ty = FuncType::new(store.engine(), None, None);
    let f2 = Func::new(&mut store, func_ty, move |_, _, _| {
        let _ = &canary2;
        bail!("")
    });
    assert!(f2.typed::<(), ()>(&store)?.call(&mut store, ()).is_err());
    assert!(f2.call(&mut store, &[], &mut []).is_err());

    // drop everything and ensure dtors are run
    drop(store);
    assert!(dtor1_run.load(SeqCst));
    assert!(dtor2_run.load(SeqCst));
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn wrap_multiple_results() -> anyhow::Result<()> {
    fn test<T>(store: &mut Store<()>, t: T) -> anyhow::Result<()>
    where
        T: WasmRet
            + WasmResults
            + PartialEq
            + Copy
            + std::fmt::Debug
            + EqualToValues
            + 'static
            + Send
            + Sync,
    {
        let f = Func::wrap(&mut *store, move || t);
        let mut results = vec![Val::I32(0); f.ty(&store).results().len()];
        assert_eq!(f.typed::<(), T>(&store)?.call(&mut *store, ())?, t);
        f.call(&mut *store, &[], &mut results)?;
        assert!(t.eq_values(&results));

        let module = Module::new(store.engine(), &T::gen_wasm())?;
        let instance = Instance::new(&mut *store, &module, &[f.into()])?;
        let f = instance.get_func(&mut *store, "foo").unwrap();

        assert_eq!(f.typed::<(), T>(&store)?.call(&mut *store, ())?, t);
        f.call(&mut *store, &[], &mut results)?;
        assert!(t.eq_values(&results));
        Ok(())
    }

    let mut store = Store::default();
    // 0 element
    test(&mut store, ())?;

    // 1 element
    test(&mut store, (1i32,))?;
    test(&mut store, (2u32,))?;
    test(&mut store, (3i64,))?;
    test(&mut store, (4u64,))?;
    test(&mut store, (5.0f32,))?;
    test(&mut store, (6.0f64,))?;

    // 2 element ...
    test(&mut store, (7i32, 8i32))?;
    test(&mut store, (7i32, 8i64))?;
    test(&mut store, (7i32, 8f32))?;
    test(&mut store, (7i32, 8f64))?;

    test(&mut store, (7i64, 8i32))?;
    test(&mut store, (7i64, 8i64))?;
    test(&mut store, (7i64, 8f32))?;
    test(&mut store, (7i64, 8f64))?;

    test(&mut store, (7f32, 8i32))?;
    test(&mut store, (7f32, 8i64))?;
    test(&mut store, (7f32, 8f32))?;
    test(&mut store, (7f32, 8f64))?;

    test(&mut store, (7f64, 8i32))?;
    test(&mut store, (7f64, 8i64))?;
    test(&mut store, (7f64, 8f32))?;
    test(&mut store, (7f64, 8f64))?;

    // and beyond...
    test(&mut store, (1i32, 2i32, 3i32))?;
    test(&mut store, (1i32, 2f32, 3i32))?;
    test(&mut store, (1f64, 2f32, 3i32))?;
    test(&mut store, (1f64, 2i64, 3i32))?;
    test(&mut store, (1f32, 2f32, 3i64, 4f64))?;
    test(&mut store, (1f64, 2i64, 3i32, 4i64, 5f32))?;
    test(&mut store, (1i32, 2f64, 3i64, 4f64, 5f64, 6f32))?;
    test(&mut store, (1i64, 2i32, 3i64, 4f32, 5f32, 6i32, 7u64))?;
    test(&mut store, (1u32, 2f32, 3u64, 4f64, 5i32, 6f32, 7u64, 8u32))?;
    test(
        &mut store,
        (1f32, 2f64, 3f32, 4i32, 5u32, 6i64, 7f32, 8i32, 9u64),
    )?;
    return Ok(());

    trait EqualToValues {
        fn eq_values(&self, values: &[Val]) -> bool;
        fn gen_wasm() -> String;
    }

    macro_rules! equal_tuples {
        ($($cnt:tt ($($a:ident),*))*) => ($(
            #[allow(non_snake_case)]
            impl<$($a: EqualToValue,)*> EqualToValues for ($($a,)*) {
                fn eq_values(&self, values: &[Val]) -> bool {
                    let ($($a,)*) = self;
                    let mut _values = values.iter();
                    _values.len() == $cnt &&
                        $($a.eq_value(_values.next().unwrap()) &&)*
                        true
                }

                fn gen_wasm() -> String {
                    let mut wasm = String::new();
                    wasm.push_str("(module ");
                    wasm.push_str("(type $t (func (result ");
                    $(
                        wasm.push_str($a::wasm_ty());
                        wasm.push_str(" ");
                    )*
                    wasm.push_str(")))");

                    wasm.push_str("(import \"\" \"\" (func $host (type $t)))");
                    wasm.push_str("(func (export \"foo\") (type $t)");
                    wasm.push_str("call $host");
                    wasm.push_str(")");
                    wasm.push_str(")");

                    wasm
                }
            }
        )*)
    }

    equal_tuples! {
        0 ()
        1 (A1)
        2 (A1, A2)
        3 (A1, A2, A3)
        4 (A1, A2, A3, A4)
        5 (A1, A2, A3, A4, A5)
        6 (A1, A2, A3, A4, A5, A6)
        7 (A1, A2, A3, A4, A5, A6, A7)
        8 (A1, A2, A3, A4, A5, A6, A7, A8)
        9 (A1, A2, A3, A4, A5, A6, A7, A8, A9)
    }

    trait EqualToValue {
        fn eq_value(&self, value: &Val) -> bool;
        fn wasm_ty() -> &'static str;
    }

    macro_rules! equal_values {
        ($a:ident $($ty:ident $wasm:tt $variant:ident $e:expr,)*) => ($(
            impl EqualToValue for $ty {
                fn eq_value(&self, val: &Val) -> bool {
                    if let Val::$variant($a) = *val {
                        return *self == $e;
                    }
                    false
                }

                fn wasm_ty() -> &'static str {
                    $wasm
                }
            }
        )*)
    }

    equal_values! {
        a
        i32 "i32" I32 a,
        u32 "i32" I32 a as u32,
        i64 "i64" I64 a,
        u64 "i64" I64 a as u64,
        f32 "f32" F32 f32::from_bits(a),
        f64 "f64" F64 f64::from_bits(a),
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn trampoline_for_declared_elem() -> anyhow::Result<()> {
    let engine = Engine::default();

    let module = Module::new(
        &engine,
        r#"
            (module
                (elem declare func $f)
                (func $f)
                (func (export "g") (result funcref)
                  (ref.func $f)
                )
            )
        "#,
    )?;

    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;

    let g = instance.get_typed_func::<(), Option<Func>>(&mut store, "g")?;

    let func = g.call(&mut store, ())?;
    func.unwrap().call(&mut store, &[], &mut [])?;
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn wasm_ty_roundtrip() -> Result<(), anyhow::Error> {
    let mut store = Store::<()>::default();
    let debug = Func::wrap(
        &mut store,
        |a: i32, b: u32, c: f32, d: i64, e: u64, f: f64| {
            assert_eq!(a, -1);
            assert_eq!(b, 1);
            assert_eq!(c, 2.0);
            assert_eq!(d, -3);
            assert_eq!(e, 3);
            assert_eq!(f, 4.0);
        },
    );
    let module = Module::new(
        store.engine(),
        r#"
             (module
                 (import "" "" (func $debug (param i32 i32 f32 i64 i64 f64)))
                 (func (export "foo") (param i32 i32 f32 i64 i64 f64)
                    (if (i32.ne (local.get 0) (i32.const -1))
                        (then unreachable)
                    )
                    (if (i32.ne (local.get 1) (i32.const 1))
                        (then unreachable)
                    )
                    (if (f32.ne (local.get 2) (f32.const 2))
                        (then unreachable)
                    )
                    (if (i64.ne (local.get 3) (i64.const -3))
                        (then unreachable)
                    )
                    (if (i64.ne (local.get 4) (i64.const 3))
                        (then unreachable)
                    )
                    (if (f64.ne (local.get 5) (f64.const 4))
                        (then unreachable)
                    )
                    local.get 0
                    local.get 1
                    local.get 2
                    local.get 3
                    local.get 4
                    local.get 5
                    call $debug
                )
            )
         "#,
    )?;
    let instance = Instance::new(&mut store, &module, &[debug.into()])?;
    let foo = instance.get_typed_func::<(i32, u32, f32, i64, u64, f64), ()>(&mut store, "foo")?;
    foo.call(&mut store, (-1, 1, 2.0, -3, 3, 4.0))?;
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn typed_funcs_count_params_correctly_in_error_messages() -> anyhow::Result<()> {
    let mut store = Store::<()>::default();
    let module = Module::new(
        store.engine(),
        r#"
            (module
                (func (export "f") (param i32 i32))
            )

        "#,
    )?;
    let instance = Instance::new(&mut store, &module, &[])?;

    // Too few parameters.
    match instance.get_typed_func::<(), ()>(&mut store, "f") {
        Ok(_) => panic!("should be wrong signature"),
        Err(e) => {
            let msg = format!("{:?}", e);
            assert!(dbg!(msg).contains("expected 0 types, found 2"))
        }
    }
    match instance.get_typed_func::<(i32,), ()>(&mut store, "f") {
        Ok(_) => panic!("should be wrong signature"),
        Err(e) => {
            let msg = format!("{:?}", e);
            assert!(dbg!(msg).contains("expected 1 types, found 2"))
        }
    }

    // Too many parameters.
    match instance.get_typed_func::<(i32, i32, i32), ()>(&mut store, "f") {
        Ok(_) => panic!("should be wrong signature"),
        Err(e) => {
            let msg = format!("{:?}", e);
            assert!(dbg!(msg).contains("expected 3 types, found 2"))
        }
    }

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn calls_with_funcref_and_externref() -> anyhow::Result<()> {
    let mut store = Store::<()>::default();
    let module = Module::new(
        store.engine(),
        r#"
            (module
                (import "" "witness" (func $witness (param funcref externref)))
                (func (export "f") (param funcref externref) (result externref funcref)
                    local.get 0
                    local.get 1
                    call $witness
                    local.get 1
                    local.get 0
                )
            )

        "#,
    )?;
    let mut linker = Linker::new(store.engine());
    linker.func_wrap(
        "",
        "witness",
        |mut caller: Caller<'_, ()>, func: Option<Func>, externref: Option<Rooted<ExternRef>>| {
            if func.is_some() {
                assert_my_funcref(&mut caller, func.as_ref())?;
            }
            if externref.is_some() {
                assert_my_externref(&caller, externref);
            }
            Ok(())
        },
    )?;
    let instance = linker.instantiate(&mut store, &module)?;

    let typed = instance
        .get_typed_func::<(Option<Func>, Option<Rooted<ExternRef>>), (Option<Rooted<ExternRef>>, Option<Func>)>(
            &mut store, "f",
        )?;
    let untyped = typed.func();

    let my_funcref = Func::wrap(&mut store, || 100u32);
    let my_externref = ExternRef::new(&mut store, 99u32)?;
    let mut results = [Val::I32(0), Val::I32(0)];

    fn assert_my_funcref(mut store: impl AsContextMut, func: Option<&Func>) -> Result<()> {
        let mut store = store.as_context_mut();
        let func = func.unwrap();
        assert_eq!(func.typed::<(), u32>(&store)?.call(&mut store, ())?, 100);
        Ok(())
    }
    fn assert_my_externref(store: impl AsContext, externref: Option<Rooted<ExternRef>>) {
        assert_eq!(
            externref.unwrap().data(&store).unwrap().downcast_ref(),
            Some(&99u32)
        );
    }

    // funcref=null, externref=null
    let (a, b) = typed.call(&mut store, (None, None))?;
    assert!(a.is_none());
    assert!(b.is_none());
    untyped.call(
        &mut store,
        &[Val::FuncRef(None), Val::ExternRef(None)],
        &mut results,
    )?;
    assert!(results[0].unwrap_externref().is_none());
    assert!(results[1].unwrap_funcref().is_none());

    // funcref=Some, externref=null
    let (a, b) = typed.call(&mut store, (Some(my_funcref), None))?;
    assert!(a.is_none());
    assert_my_funcref(&mut store, b.as_ref())?;
    untyped.call(
        &mut store,
        &[Val::FuncRef(Some(my_funcref)), Val::ExternRef(None)],
        &mut results,
    )?;
    assert!(results[0].unwrap_externref().is_none());
    assert_my_funcref(&mut store, results[1].unwrap_funcref())?;

    // funcref=null, externref=Some
    let (a, b) = typed.call(&mut store, (None, Some(my_externref.clone())))?;
    assert_my_externref(&store, a);
    assert!(b.is_none());
    untyped.call(
        &mut store,
        &[
            Val::FuncRef(None),
            Val::ExternRef(Some(my_externref.clone())),
        ],
        &mut results,
    )?;
    assert_my_externref(&store, results[0].unwrap_externref().copied());
    assert!(results[1].unwrap_funcref().is_none());

    // funcref=Some, externref=Some
    let (a, b) = typed.call(&mut store, (Some(my_funcref), Some(my_externref.clone())))?;
    assert_my_externref(&store, a);
    assert_my_funcref(&mut store, b.as_ref())?;
    untyped.call(
        &mut store,
        &[
            Val::FuncRef(Some(my_funcref)),
            Val::ExternRef(Some(my_externref.clone())),
        ],
        &mut results,
    )?;
    assert_my_externref(&store, results[0].unwrap_externref().copied());
    assert_my_funcref(&mut store, results[1].unwrap_funcref())?;

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn typed_concrete_param() -> anyhow::Result<()> {
    let mut config = Config::new();
    config.wasm_function_references(true);
    let engine = Engine::new(&config)?;
    let module = Module::new(
        &engine,
        r#"
            (module
                (type $t (func))
                (func (export "f") (param (ref null $t)))
            )
        "#,
    )?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;

    let nop = Func::new(
        &mut store,
        FuncType::new(&engine, None, None),
        |_caller, _params, _results| Ok(()),
    );

    let f = instance.get_func(&mut store, "f").unwrap();

    // Can type with a subtype, which should avoid all dynamic type checks after
    // successful construction.
    let a = f.typed::<Option<NoFunc>, ()>(&store)?;
    a.call(&mut store, None)?;
    // NB: Cannot call with Some(_) as `NoFunc` is uninhabited.

    // Can call `typed` with a supertype, falling back to dynamic type checks on
    // each call.
    let a = f.typed::<Option<Func>, ()>(&store)?;
    a.call(&mut store, None)?;
    a.call(&mut store, Some(nop.clone()))?;
    let e = a.call(&mut store, Some(f.clone())).expect_err(
        "should return an error because while we did pass an instance of \
         `Option<Func>`, it was not an instance of `(ref null $t)`",
    );
    let e = format!("{e:?}");
    assert!(e.contains("argument type mismatch for reference to concrete type"));
    assert!(e.contains(
        "type mismatch: expected (type (func)), \
         found (type (func (param (ref null (concrete VMSharedTypeIndex(0))))))"
    ));

    // And dynamic checks also work with a non-nullable super type.
    let a = f.typed::<Func, ()>(&store)?;
    a.call(&mut store, nop.clone())?;
    let e = a.call(&mut store, f.clone()).expect_err(
        "should return an error because while we did pass an instance of \
         `Func`, it was not an instance of `(ref null $t)`",
    );
    let e = format!("{e:?}");
    assert!(e.contains("argument type mismatch for reference to concrete type"));
    assert!(e.contains(
        "type mismatch: expected (type (func)), \
         found (type (func (param (ref null (concrete VMSharedTypeIndex(0))))))"
    ));

    // Calling `typed` with a type that is not a supertype nor a subtype fails
    // the initial type check.
    let e = f
        .typed::<Option<Rooted<ExternRef>>, ()>(&store)
        .err()
        .unwrap();
    let e = format!("{e:?}");
    assert!(e.contains("type mismatch with parameters"));
    assert!(e.contains("type mismatch: expected func, found extern"));

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn typed_concrete_result() -> anyhow::Result<()> {
    let mut config = Config::new();
    config.wasm_function_references(true);
    let engine = Engine::new(&config)?;
    let module = Module::new(
        &engine,
        r#"
            (module
                (type $t (func))
                (func $nop)
                (elem declare func $nop)
                (func (export "f") (result (ref $t))
                    ref.func $nop
                )
            )
        "#,
    )?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;

    let f = instance.get_func(&mut store, "f").unwrap();

    // Can type `f` with a supertype of the declared result type, and we get the
    // expected return value.
    let a = f.typed::<(), Func>(&store)?;
    let g = a.call(&mut store, ())?;
    g.typed::<(), ()>(&store)?.call(&mut store, ())?;

    // Also works with a nullable supertype.
    let a = f.typed::<(), Option<Func>>(&store)?;
    let g = a.call(&mut store, ())?;
    g.unwrap().typed::<(), ()>(&store)?.call(&mut store, ())?;

    // But we can't claim that `f` returns a particular subtype of its actual
    // return type.
    let e = f.typed::<(), NoFunc>(&store).err().unwrap();
    let e = format!("{e:?}");
    assert!(e.contains("type mismatch with results"));
    assert!(e.contains(
        "type mismatch: expected (ref nofunc), found (ref (concrete VMSharedTypeIndex(0)))"
    ));

    // Nor some unrelated type that it is neither a subtype or supertype of.
    let e = f.typed::<(), Rooted<ExternRef>>(&store).err().unwrap();
    let e = format!("{e:?}");
    assert!(e.contains("type mismatch with results"));
    assert!(e.contains(
        "type mismatch: expected (ref extern), found (ref (concrete VMSharedTypeIndex(0)))"
    ));

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn wrap_subtype_param() -> anyhow::Result<()> {
    let mut store = Store::<()>::default();
    let f = Func::wrap(&mut store, |_caller: Caller<'_, ()>, _: Option<Func>| {
        // No-op.
    });

    // Precise type.
    let a = f.typed::<Option<Func>, ()>(&store)?;
    a.call(&mut store, None)?;
    a.call(&mut store, Some(f.clone()))?;

    // Subtype via heap type.
    let a = f.typed::<Option<NoFunc>, ()>(&store)?;
    a.call(&mut store, None)?;

    // Subtype via non-null.
    let a = f.typed::<Func, ()>(&store)?;
    a.call(&mut store, f.clone())?;

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn wrap_supertype_result() -> anyhow::Result<()> {
    let mut store = Store::<()>::default();
    let f = Func::wrap(&mut store, |_caller: Caller<'_, ()>| -> NoFunc {
        unreachable!()
    });

    // Precise type.
    let _ = f.typed::<(), NoFunc>(&store)?;

    // Supertype via heap type.
    let _ = f.typed::<(), Func>(&store)?;

    // Supertype via nullability.
    let _ = f.typed::<(), Option<NoFunc>>(&store)?;

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn call_wasm_passing_subtype_func_param() -> anyhow::Result<()> {
    let mut config = Config::new();
    config.wasm_function_references(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    let module = Module::new(
        &engine,
        r#"
            (module
                (type $ty (func (result funcref)))
                (func (export "f") (param (ref null $ty)) (result funcref)
                    ;; Return null if the funcref is null.
                    ref.null func
                    local.get 0
                    ref.is_null
                    br_if 0
                    drop

                    ;; Otherwise, call it.
                    local.get 0
                    call_ref $ty
                )
            )
        "#,
    )?;

    let instance = Instance::new(&mut store, &module, &[])?;
    let f = instance.get_func(&mut store, "f").unwrap();

    let g_ty = FuncType::new(&engine, None, Some(ValType::I32));
    let g = Func::new(&mut store, g_ty.clone(), |_caller, _params, results| {
        results[0] = Val::I32(0x1234_5678);
        Ok(())
    });

    // h's type is a subtype of the Wasm-defined `$ty`:
    //
    //     (func (result (ref null g_ty))) <: (func (result funcref))
    let h_ty = FuncType::new(
        &engine,
        None,
        Some(ValType::Ref(RefType::new(
            true,
            HeapType::ConcreteFunc(g_ty),
        ))),
    );
    let h = Func::new(&mut store, h_ty, move |_caller, _params, results| {
        results[0] = Val::FuncRef(Some(g.clone()));
        Ok(())
    });

    // Array call, passing in a subtype of the expected parameter.

    let mut results = vec![Val::I32(0)];
    f.call(&mut store, &[Val::null_func_ref()], &mut results)?;
    assert!(results[0].unwrap_func_ref().is_none());

    f.call(&mut store, &[h.clone().into()], &mut results)?;
    let g = results[0].clone();
    let g = g.unwrap_func_ref().unwrap();
    g.call(&mut store, &[], &mut results)?;
    assert_eq!(results[0].unwrap_i32(), 0x1234_5678);

    // Native call, passing in a subtype of the expected parameter.

    let f = f.typed::<Option<Func>, Option<Func>>(&store)?;
    let r = f.call(&mut store, None)?;
    assert!(r.is_none());

    let g = f.call(&mut store, Some(h))?;
    let g = g.unwrap().typed::<(), u32>(&mut store)?;
    let x = g.call(&mut store, ())?;
    assert_eq!(x, 0x1234_5678);

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn call_wasm_getting_subtype_func_return() -> anyhow::Result<()> {
    let mut config = Config::new();
    config.wasm_gc(true);
    config.wasm_function_references(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    let module = Module::new(
        &engine,
        r#"
            (module
                (type $ty (func (result funcref)))

                (func $a (result i32)
                    i32.const 0x12345678
                )

                (func $b (result funcref)
                    ref.func $a
                )

                (elem declare func $a $b)

                ;; Returns a `(ref null nofunc)` if called with `0`, otherwise
                ;; returns `(ref null $ty)`, both of which are subtypes of
                ;; `funcref`.
                (func (export "f") (param i32) (result funcref)
                    block
                        local.get 0
                        br_if 0
                        ref.null nofunc
                        return
                    end
                    ref.func $b
                )
            )
        "#,
    )?;

    let instance = Instance::new(&mut store, &module, &[])?;
    let f = instance.get_func(&mut store, "f").unwrap();

    // Array call, receiving a subtype of the expected result.

    let mut results = vec![Val::I32(0)];
    f.call(&mut store, &[Val::I32(0)], &mut results)?;
    assert!(results[0].unwrap_func_ref().is_none());

    f.call(&mut store, &[Val::I32(1)], &mut results)?;
    let b = results[0].clone();
    let b = b.unwrap_func_ref().unwrap();
    b.call(&mut store, &[], &mut results)?;
    let a = results[0].clone();
    let a = a.unwrap_func_ref().unwrap();
    a.call(&mut store, &[], &mut results)?;
    assert_eq!(results[0].unwrap_i32(), 0x1234_5678);

    // Native call, receiving a subtype of the expected result.

    let f = f.typed::<u32, Option<Func>>(&store)?;
    let r = f.call(&mut store, 0)?;
    assert!(r.is_none());

    let b = f.call(&mut store, 1)?;
    let b = b.unwrap().typed::<(), Option<Func>>(&store)?;
    let a = b.call(&mut store, ())?;
    let a = a.unwrap().typed::<(), u32>(&store)?;
    let x = a.call(&mut store, ())?;
    assert_eq!(x, 0x1234_5678);

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
fn typed_v128() -> anyhow::Result<()> {
    let mut store = Store::<()>::default();
    let module = Module::new(
        store.engine(),
        r#"
            (module
                (func (export "a") (param v128) (result v128)
                    local.get 0)
                (func (export "b") (param v128 v128) (result v128 v128)
                    local.get 0
                    local.get 1)
                (func (export "c") (param v128 v128 v128 v128 v128 v128 v128 v128) (result v128)
                    local.get 0
                    local.get 1
                    local.get 2
                    local.get 3
                    local.get 4
                    local.get 5
                    local.get 6
                    local.get 7
                    i64x2.add
                    i64x2.add
                    i64x2.add
                    i64x2.add
                    i64x2.add
                    i64x2.add
                    i64x2.add)
            )

        "#,
    )?;
    let instance = Instance::new(&mut store, &module, &[])?;

    let a = instance.get_typed_func::<V128, V128>(&mut store, "a")?;
    assert_eq!(a.call(&mut store, V128::from(1))?, V128::from(1));
    assert_eq!(a.call(&mut store, V128::from(2))?, V128::from(2));

    let b = instance.get_typed_func::<(V128, V128), (V128, V128)>(&mut store, "b")?;
    assert_eq!(
        b.call(&mut store, (V128::from(1), V128::from(2)))?,
        (V128::from(1), V128::from(2))
    );

    let c = instance.get_typed_func::<(V128, V128, V128, V128, V128, V128, V128, V128), V128>(
        &mut store, "c",
    )?;
    assert_eq!(
        c.call(
            &mut store,
            (
                V128::from(1),
                V128::from(2),
                V128::from(3),
                V128::from(4),
                V128::from(5),
                V128::from(6),
                V128::from(7),
                V128::from(8),
            )
        )?,
        V128::from(1 + 2 + 3 + 4 + 5 + 6 + 7 + 8),
    );

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
fn typed_v128_imports() -> anyhow::Result<()> {
    let mut store = Store::<()>::default();
    let module = Module::new(
        store.engine(),
        r#"
            (module
                (import "" "a" (func $a (param v128) (result i32)))
                (import "" "b" (func $b (param i32) (result v128)))
                (import "" "c" (func $c (param v128 f64 v128) (result v128 i32 v128)))
                (import "" "d" (func $d (param v128 v128 v128 v128 v128 v128 v128 v128) (result i32)))

                (func (export "a") (param v128) (result i32)
                    local.get 0
                    call $a)

                (func (export "b") (param i32) (result v128)
                    local.get 0
                    call $b)

                (func (export "c") (param v128 f64 v128) (result v128 i32 v128)
                    local.get 0
                    local.get 1
                    local.get 2
                    call $c)

                (func (export "d") (param v128 v128 v128 v128 v128 v128 v128 v128) (result i32)
                    local.get 0
                    local.get 1
                    local.get 2
                    local.get 3
                    local.get 4
                    local.get 5
                    local.get 6
                    local.get 7
                    call $d)
            )

        "#,
    )?;

    let mut l = Linker::new(store.engine());
    l.func_wrap("", "a", |x: V128| {
        let x = x.as_u128();
        (x >> 0) as u32 + (x >> 32) as u32 + (x >> 64) as u32 + (x >> 96) as u32
    })?;
    l.func_wrap("", "b", |x: u32| {
        let x = u128::from(x);
        V128::from(x | ((x + 1) << 32) | ((x + 2) << 64) | ((x + 3) << 96))
    })?;
    l.func_wrap("", "c", |x: V128, y: f64, z: V128| (x, y as i32, z))?;
    l.func_wrap(
        "",
        "d",
        |a0: V128, a1: V128, a2: V128, a3: V128, a4: V128, a5: V128, a6: V128, a7: V128| {
            let tmp = a0.as_u128()
                + a1.as_u128()
                + a2.as_u128()
                + a3.as_u128()
                + a4.as_u128()
                + a5.as_u128()
                + a6.as_u128()
                + a7.as_u128();
            (tmp >> 0) as u32 + (tmp >> 32) as u32 + (tmp >> 64) as u32 + (tmp >> 96) as u32
        },
    )?;

    let i = l.instantiate(&mut store, &module)?;
    let a = i.get_typed_func::<V128, i32>(&mut store, "a")?;
    let b = i.get_typed_func::<i32, V128>(&mut store, "b")?;
    let c = i.get_typed_func::<(V128, f64, V128), (V128, i32, V128)>(&mut store, "c")?;
    let d =
        i.get_typed_func::<(V128, V128, V128, V128, V128, V128, V128, V128), i32>(&mut store, "d")?;

    assert_eq!(
        a.call(&mut store, 0x00000004_00000003_00000002_00000001.into())?,
        1 + 2 + 3 + 4
    );
    assert_eq!(
        b.call(&mut store, 0x10)?,
        V128::from(0x00000013_00000012_00000011_00000010),
    );
    assert_eq!(
        c.call(&mut store, (1.into(), 2., 3.into()))?,
        (V128::from(1), 2, V128::from(3)),
    );
    assert_eq!(
        d.call(
            &mut store,
            (
                1.into(),
                2.into(),
                3.into(),
                4.into(),
                5.into(),
                6.into(),
                7.into(),
                8.into(),
            )
        )?,
        1 + 2 + 3 + 4 + 5 + 6 + 7 + 8
    );

    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn wrap_and_typed_i31ref() -> Result<()> {
    let mut config = Config::new();
    config.wasm_function_references(true);
    config.wasm_gc(true);

    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    static HITS: AtomicUsize = AtomicUsize::new(0);
    let mut linker = Linker::new(&engine);
    linker.func_wrap("env", "i31ref", |x: Option<I31>| -> Option<I31> {
        assert_eq!(HITS.fetch_add(1, Ordering::SeqCst), 0);
        x
    })?;
    linker.func_wrap("env", "ref-i31", |x: I31| -> I31 {
        assert_eq!(HITS.fetch_add(1, Ordering::SeqCst), 1);
        x
    })?;

    let module = Module::new(
        &engine,
        r#"
            (module
                (import "env" "i31ref" (func (param i31ref) (result i31ref)))
                (import "env" "ref-i31" (func (param (ref i31)) (result (ref i31))))

                (func (export "i31ref") (param i31ref) (result i31ref)
                    local.get 0
                    call 0
                )

                (func (export "ref-i31") (param (ref i31)) (result (ref i31))
                    local.get 0
                    call 1
                )
            )
        "#,
    )?;

    let instance = linker.instantiate(&mut store, &module)?;

    let i31ref = instance.get_typed_func::<Option<I31>, Option<I31>>(&mut store, "i31ref")?;
    let x = i31ref.call(&mut store, Some(I31::wrapping_u32(42)))?;
    assert_eq!(x, Some(I31::wrapping_u32(42)));

    let ref_i31 = instance.get_typed_func::<I31, I31>(&mut store, "ref-i31")?;
    let x = ref_i31.call(&mut store, I31::wrapping_u32(0x1234))?;
    assert_eq!(x, I31::wrapping_u32(0x1234));

    assert_eq!(HITS.load(Ordering::SeqCst), 2);
    Ok(())
}

#[test]
fn call_func_with_funcref_both_typed_and_untyped() -> Result<()> {
    let mut config = Config::new();
    config.wasm_function_references(true);
    config.wasm_gc(true);

    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    let f1 = Func::wrap(&mut store, |_: Option<Func>| {});
    let f2 = Func::wrap(&mut store, || {});

    f1.typed::<Func, ()>(&mut store)?.call(&mut store, f2)?;
    f1.call(&mut store, &[Val::FuncRef(Some(f2))], &mut [])?;
    Ok(())
}
