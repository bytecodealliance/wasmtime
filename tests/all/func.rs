use anyhow::{bail, Result};
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
    let import_func = Func::new(
        &mut store,
        FuncType::new(vec![], vec![ValType::I32, ValType::I32, ValType::I32]),
        |_, _params, results| {
            results[0] = Val::I32(1);
            results[1] = Val::I32(2);
            results[2] = Val::I32(3);
            Ok(())
        },
    );
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

    let func = Func::new(
        &mut store,
        FuncType::new(
            [ValType::I32, ValType::I32, ValType::I32],
            [ValType::I32, ValType::I32, ValType::I32],
        ),
        |_caller, params, results| {
            results[0] = params[2].clone();
            results[1] = params[0].clone();
            results[2] = params[1].clone();
            Ok(())
        },
    );
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
    let func = Func::new(
        &mut store,
        FuncType::new(
            [ValType::I32, ValType::I32, ValType::I32],
            [ValType::I32, ValType::I32, ValType::I32],
        ),
        |_caller, params, results| {
            results[0] = params[2].clone();
            results[1] = params[0].clone();
            results[2] = params[1].clone();
            Ok(())
        },
    );
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
        GlobalType::new(ValType::FuncRef, Mutability::Const),
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
        TableType::new(ValType::FuncRef, 1, Some(1)),
        Val::FuncRef(Some(func)),
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
    Func::wrap(&mut store, || -> Option<ExternRef> { None });
    Func::wrap(&mut store, || -> Option<Func> { None });

    Func::wrap(&mut store, || -> Result<()> { loop {} });
    Func::wrap(&mut store, || -> Result<i32> { loop {} });
    Func::wrap(&mut store, || -> Result<i64> { loop {} });
    Func::wrap(&mut store, || -> Result<f32> { loop {} });
    Func::wrap(&mut store, || -> Result<f64> { loop {} });
    Func::wrap(&mut store, || -> Result<Option<ExternRef>> { loop {} });
    Func::wrap(&mut store, || -> Result<Option<Func>> { loop {} });
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
    assert_eq!(f.ty(&store).params().collect::<Vec<_>>(), &[]);
    assert_eq!(f.ty(&store).results().collect::<Vec<_>>(), &[]);

    let f = Func::wrap(&mut store, || -> i32 { loop {} });
    assert_eq!(f.ty(&store).params().collect::<Vec<_>>(), &[]);
    assert_eq!(f.ty(&store).results().collect::<Vec<_>>(), &[ValType::I32]);

    let f = Func::wrap(&mut store, || -> i64 { loop {} });
    assert_eq!(f.ty(&store).params().collect::<Vec<_>>(), &[]);
    assert_eq!(f.ty(&store).results().collect::<Vec<_>>(), &[ValType::I64]);

    let f = Func::wrap(&mut store, || -> f32 { loop {} });
    assert_eq!(f.ty(&store).params().collect::<Vec<_>>(), &[]);
    assert_eq!(f.ty(&store).results().collect::<Vec<_>>(), &[ValType::F32]);

    let f = Func::wrap(&mut store, || -> f64 { loop {} });
    assert_eq!(f.ty(&store).params().collect::<Vec<_>>(), &[]);
    assert_eq!(f.ty(&store).results().collect::<Vec<_>>(), &[ValType::F64]);

    let f = Func::wrap(
        &mut store,
        |_: f32, _: f64, _: i32, _: i64, _: i32, _: Option<ExternRef>, _: Option<Func>| -> f64 {
            loop {}
        },
    );
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
}

#[test]
#[cfg_attr(miri, ignore)]
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
                let mut results = [Val::I32(0)];
                g.as_ref()
                    .unwrap()
                    .call(&mut caller, &[], &mut results)
                    .unwrap();
                assert_eq!(results[0].unwrap_i32(), 42);
                assert_eq!(HITS.fetch_add(1, SeqCst), 3);
            },
        )
        .into(),
    ];
    let instance = Instance::new(&mut store, &module, &imports)?;
    let run = instance.get_func(&mut store, "run").unwrap();
    let funcref = Val::FuncRef(Some(Func::wrap(&mut store, || -> i32 { 42 })));
    run.call(
        &mut store,
        &[
            Val::ExternRef(Some(ExternRef::new("hello".to_string()))),
            funcref,
        ],
        &mut [],
    )?;
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
    let f = Func::wrap(&mut store, || -> Option<ExternRef> { loop {} });
    assert!(f.typed::<(), Option<ExternRef>>(&store).is_ok());
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
    let f = Func::wrap(&mut store, |_: Option<ExternRef>| {});
    assert!(f.typed::<Option<ExternRef>, ()>(&store).is_ok());
    let f = Func::wrap(&mut store, |_: Option<Func>| {});
    assert!(f.typed::<Option<Func>, ()>(&store).is_ok());
}

#[test]
#[cfg_attr(miri, ignore)]
fn get_from_signature() {
    let mut store = Store::<()>::default();
    let ty = FuncType::new(None, None);
    let f = Func::new(&mut store, ty, |_, _, _| panic!());
    assert!(f.typed::<(), ()>(&store).is_ok());
    assert!(f.typed::<(), i32>(&store).is_err());
    assert!(f.typed::<i32, ()>(&store).is_err());

    let ty = FuncType::new(Some(ValType::I32), Some(ValType::F64));
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
    let ty = FuncType::new(None, Some(ValType::I32));
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
    Func::new(
        &mut store,
        FuncType::new(
            [ValType::FuncRef, ValType::ExternRef].iter().cloned(),
            [ValType::FuncRef, ValType::ExternRef].iter().cloned(),
        ),
        |_, _, _| Ok(()),
    );
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
    let f2 = Func::new(&mut store, FuncType::new(None, None), move |_, _, _| {
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
        |mut caller: Caller<'_, ()>, func: Option<Func>, externref: Option<ExternRef>| {
            if func.is_some() {
                assert_my_funcref(&mut caller, func.as_ref())?;
            }
            if externref.is_some() {
                assert_my_externref(externref.as_ref());
            }
            Ok(())
        },
    )?;
    let instance = linker.instantiate(&mut store, &module)?;

    let typed = instance
        .get_typed_func::<(Option<Func>, Option<ExternRef>), (Option<ExternRef>, Option<Func>)>(
            &mut store, "f",
        )?;
    let untyped = typed.func();

    let my_funcref = Func::wrap(&mut store, || 100u32);
    let my_externref = ExternRef::new(99u32);
    let mut results = [Val::I32(0), Val::I32(0)];

    fn assert_my_funcref(mut store: impl AsContextMut, func: Option<&Func>) -> Result<()> {
        let mut store = store.as_context_mut();
        let func = func.unwrap();
        assert_eq!(func.typed::<(), u32>(&store)?.call(&mut store, ())?, 100);
        Ok(())
    }
    fn assert_my_externref(externref: Option<&ExternRef>) {
        assert_eq!(externref.unwrap().data().downcast_ref(), Some(&99u32));
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
    assert_my_externref(a.as_ref());
    assert!(b.is_none());
    untyped.call(
        &mut store,
        &[
            Val::FuncRef(None),
            Val::ExternRef(Some(my_externref.clone())),
        ],
        &mut results,
    )?;
    assert_my_externref(results[0].unwrap_externref().as_ref());
    assert!(results[1].unwrap_funcref().is_none());

    // funcref=Some, externref=Some
    let (a, b) = typed.call(&mut store, (Some(my_funcref), Some(my_externref.clone())))?;
    assert_my_externref(a.as_ref());
    assert_my_funcref(&mut store, b.as_ref())?;
    untyped.call(
        &mut store,
        &[
            Val::FuncRef(Some(my_funcref)),
            Val::ExternRef(Some(my_externref.clone())),
        ],
        &mut results,
    )?;
    assert_my_externref(results[0].unwrap_externref().as_ref());
    assert_my_funcref(&mut store, results[1].unwrap_funcref())?;

    Ok(())
}
