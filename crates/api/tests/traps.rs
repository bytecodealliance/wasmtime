use std::rc::Rc;
use wasmtime::*;
use wat::parse_str;

#[test]
fn test_trap_return() -> Result<(), String> {
    struct HelloCallback;

    impl Callable for HelloCallback {
        fn call(&self, _params: &[Val], _results: &mut [Val]) -> Result<(), HostRef<Trap>> {
            Err(HostRef::new(Trap::new("test 123".into())))
        }
    }

    let engine = HostRef::new(Engine::default());
    let store = HostRef::new(Store::new(&engine));
    let binary = parse_str(
        r#"
                (module
                (func $hello (import "" "hello"))
                (func (export "run") (call $hello))
                )
            "#,
    )
    .map_err(|e| format!("failed to parse WebAssembly text source: {}", e))?;

    let module = HostRef::new(
        Module::new(&store, &binary).map_err(|e| format!("failed to compile module: {}", e))?,
    );
    let hello_type = FuncType::new(Box::new([]), Box::new([]));
    let hello_func = HostRef::new(Func::new(&store, hello_type, Rc::new(HelloCallback)));

    let imports = vec![hello_func.into()];
    let instance = Instance::new(&store, &module, imports.as_slice())
        .map_err(|e| format!("failed to instantiate module: {}", e))?;
    let run_func = instance.exports()[0]
        .func()
        .expect("expected function export");

    let e = run_func
        .borrow()
        .call(&[])
        .err()
        .expect("error calling function");

    assert_eq!(e.borrow().message(), "test 123");

    Ok(())
}
