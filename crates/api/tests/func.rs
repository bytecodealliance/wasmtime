use anyhow::Result;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use wasmtime::{Callable, Func, FuncType, Instance, Module, Store, Trap, Val, ValType};

#[test]
fn func_constructors() {
    let store = Store::default();
    Func::wrap0(&store, || {});
    Func::wrap1(&store, |_: i32| {});
    Func::wrap2(&store, |_: i32, _: i64| {});
    Func::wrap2(&store, |_: f32, _: f64| {});
    Func::wrap0(&store, || -> i32 { 0 });
    Func::wrap0(&store, || -> i64 { 0 });
    Func::wrap0(&store, || -> f32 { 0.0 });
    Func::wrap0(&store, || -> f64 { 0.0 });

    Func::wrap0(&store, || -> Result<(), Trap> { loop {} });
    Func::wrap0(&store, || -> Result<i32, Trap> { loop {} });
    Func::wrap0(&store, || -> Result<i64, Trap> { loop {} });
    Func::wrap0(&store, || -> Result<f32, Trap> { loop {} });
    Func::wrap0(&store, || -> Result<f64, Trap> { loop {} });
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
    Func::wrap0(&store, move || {
        drop(&a);
    });
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
    let func = Func::wrap0(&store, move || drop(&a));

    assert_eq!(HITS.load(SeqCst), 0);
    let wasm = wat::parse_str(r#"(import "" "" (func))"#)?;
    let module = Module::new(&store, &wasm)?;
    let instance = Instance::new(&module, &[func.into()])?;
    assert_eq!(HITS.load(SeqCst), 0);
    drop(instance);
    assert_eq!(HITS.load(SeqCst), 1);
    Ok(())
}

#[test]
fn signatures_match() {
    let store = Store::default();

    let f = Func::wrap0(&store, || {});
    assert_eq!(f.ty().params(), &[]);
    assert_eq!(f.ty().results(), &[]);

    let f = Func::wrap0(&store, || -> i32 { loop {} });
    assert_eq!(f.ty().params(), &[]);
    assert_eq!(f.ty().results(), &[ValType::I32]);

    let f = Func::wrap0(&store, || -> i64 { loop {} });
    assert_eq!(f.ty().params(), &[]);
    assert_eq!(f.ty().results(), &[ValType::I64]);

    let f = Func::wrap0(&store, || -> f32 { loop {} });
    assert_eq!(f.ty().params(), &[]);
    assert_eq!(f.ty().results(), &[ValType::F32]);

    let f = Func::wrap0(&store, || -> f64 { loop {} });
    assert_eq!(f.ty().params(), &[]);
    assert_eq!(f.ty().results(), &[ValType::F64]);

    let f = Func::wrap5(&store, |_: f32, _: f64, _: i32, _: i64, _: i32| -> f64 {
        loop {}
    });
    assert_eq!(
        f.ty().params(),
        &[
            ValType::F32,
            ValType::F64,
            ValType::I32,
            ValType::I64,
            ValType::I32
        ]
    );
    assert_eq!(f.ty().results(), &[ValType::F64]);
}

#[test]
fn import_works() -> Result<()> {
    static HITS: AtomicUsize = AtomicUsize::new(0);

    let wasm = wat::parse_str(
        r#"
            (import "" "" (func))
            (import "" "" (func (param i32) (result i32)))
            (import "" "" (func (param i32) (param i64)))
            (import "" "" (func (param i32 i64 i32 f32 f64)))

            (func $foo
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
                call 3
            )
            (start $foo)
        "#,
    )?;
    let store = Store::default();
    let module = Module::new(&store, &wasm)?;
    Instance::new(
        &module,
        &[
            Func::wrap0(&store, || {
                assert_eq!(HITS.fetch_add(1, SeqCst), 0);
            })
            .into(),
            Func::wrap1(&store, |x: i32| -> i32 {
                assert_eq!(x, 0);
                assert_eq!(HITS.fetch_add(1, SeqCst), 1);
                1
            })
            .into(),
            Func::wrap2(&store, |x: i32, y: i64| {
                assert_eq!(x, 2);
                assert_eq!(y, 3);
                assert_eq!(HITS.fetch_add(1, SeqCst), 2);
            })
            .into(),
            Func::wrap5(&store, |a: i32, b: i64, c: i32, d: f32, e: f64| {
                assert_eq!(a, 100);
                assert_eq!(b, 200);
                assert_eq!(c, 300);
                assert_eq!(d, 400.0);
                assert_eq!(e, 500.0);
                assert_eq!(HITS.fetch_add(1, SeqCst), 3);
            })
            .into(),
        ],
    )?;
    Ok(())
}

#[test]
fn trap_smoke() {
    let store = Store::default();
    let f = Func::wrap0(&store, || -> Result<(), Trap> { Err(Trap::new("test")) });
    let err = f.call(&[]).unwrap_err();
    assert_eq!(err.message(), "test");
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
    let module = Module::new(&store, &wasm)?;
    let trap = Instance::new(
        &module,
        &[Func::wrap0(&store, || -> Result<(), Trap> { Err(Trap::new("foo")) }).into()],
    )
    .err()
    .unwrap()
    .downcast::<Trap>()?;
    assert_eq!(trap.message(), "foo");
    Ok(())
}

#[test]
fn get_from_wrapper() {
    let store = Store::default();
    let f = Func::wrap0(&store, || {});
    assert!(f.get0::<()>().is_ok());
    assert!(f.get0::<i32>().is_err());
    assert!(f.get1::<(), ()>().is_ok());
    assert!(f.get1::<i32, ()>().is_err());
    assert!(f.get1::<i32, i32>().is_err());
    assert!(f.get2::<(), (), ()>().is_ok());
    assert!(f.get2::<i32, i32, ()>().is_err());
    assert!(f.get2::<i32, i32, i32>().is_err());

    let f = Func::wrap0(&store, || -> i32 { loop {} });
    assert!(f.get0::<i32>().is_ok());
    let f = Func::wrap0(&store, || -> f32 { loop {} });
    assert!(f.get0::<f32>().is_ok());
    let f = Func::wrap0(&store, || -> f64 { loop {} });
    assert!(f.get0::<f64>().is_ok());

    let f = Func::wrap1(&store, |_: i32| {});
    assert!(f.get1::<i32, ()>().is_ok());
    assert!(f.get1::<i64, ()>().is_err());
    assert!(f.get1::<f32, ()>().is_err());
    assert!(f.get1::<f64, ()>().is_err());
    let f = Func::wrap1(&store, |_: i64| {});
    assert!(f.get1::<i64, ()>().is_ok());
    let f = Func::wrap1(&store, |_: f32| {});
    assert!(f.get1::<f32, ()>().is_ok());
    let f = Func::wrap1(&store, |_: f64| {});
    assert!(f.get1::<f64, ()>().is_ok());
}

#[test]
fn get_from_signature() {
    struct Foo;
    impl Callable for Foo {
        fn call(&self, _params: &[Val], _results: &mut [Val]) -> Result<(), Trap> {
            panic!()
        }
    }
    let store = Store::default();
    let ty = FuncType::new(Box::new([]), Box::new([]));
    let f = Func::new(&store, ty, Rc::new(Foo));
    assert!(f.get0::<()>().is_ok());
    assert!(f.get0::<i32>().is_err());
    assert!(f.get1::<i32, ()>().is_err());

    let ty = FuncType::new(Box::new([ValType::I32]), Box::new([ValType::F64]));
    let f = Func::new(&store, ty, Rc::new(Foo));
    assert!(f.get0::<()>().is_err());
    assert!(f.get0::<i32>().is_err());
    assert!(f.get1::<i32, ()>().is_err());
    assert!(f.get1::<i32, f64>().is_ok());
}

#[test]
fn get_from_module() -> anyhow::Result<()> {
    let store = Store::default();
    let module = Module::new(
        &store,
        r#"
            (module
                (func (export "f0"))
                (func (export "f1") (param i32))
                (func (export "f2") (result i32)
                    i32.const 0)
            )

        "#,
    )?;
    let instance = Instance::new(&module, &[])?;
    let f0 = instance.get_export("f0").unwrap().func().unwrap();
    assert!(f0.get0::<()>().is_ok());
    assert!(f0.get0::<i32>().is_err());
    let f1 = instance.get_export("f1").unwrap().func().unwrap();
    assert!(f1.get0::<()>().is_err());
    assert!(f1.get1::<i32, ()>().is_ok());
    assert!(f1.get1::<i32, f32>().is_err());
    let f2 = instance.get_export("f2").unwrap().func().unwrap();
    assert!(f2.get0::<()>().is_err());
    assert!(f2.get0::<i32>().is_ok());
    assert!(f2.get1::<i32, ()>().is_err());
    assert!(f2.get1::<i32, f32>().is_err());
    Ok(())
}
