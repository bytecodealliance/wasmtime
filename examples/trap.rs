use anyhow::Result;
use wasmtime::*;

fn main() -> Result<()> {
    let mut store = Store::<()>::default();
    let wat = r#"
        (module
        (func $hello (import "" "hello"))
        (func (export "run") (call $hello))
        )
    "#;

    #[derive(Debug)]
    struct MyTrap;
    impl std::fmt::Display for MyTrap {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "my trap")
        }
    }
    impl std::error::Error for MyTrap {}

    let module = Module::new(store.engine(), wat)?;
    let hello_type = FuncType::new(None, None);
    let hello_func = Func::new(&mut store, hello_type, |_, _, _| {
        Err(Trap::from(anyhow::Error::from(MyTrap)))
    });

    let instance = Instance::new(&mut store, &module, &[hello_func.into()])?;
    let run_func = instance.get_typed_func::<(), (), _>(&mut store, "run")?;

    let e = run_func
        .call(&mut store, ())
        .err()
        .expect("error calling function");
    println!("display run_func's err: {}", e);
    println!("debug run_func's err: {:?}", e);

    let source = std::error::Error::source(&e).expect("trap has a source");
    println!("display err's source: {}", source);
    println!("debug err's source: {:?}", source);

    source
        .downcast_ref::<MyTrap>()
        .expect("source downcasts to MyTrap");

    drop(source);

    let a = anyhow::Error::from(e);
    println!("display run_func's anyhow'd err: {}", a);
    println!("debug run_func's anyhow'd err: {:?}", a);

    let source = a.source().expect("anyhow trap has a source");
    println!("display anyhow'd err's source: {}", source);
    println!("debug anyhow'd err's source: {:?}", source);

    source
        .downcast_ref::<MyTrap>()
        .expect("source downcasts to MyTrap");

    Ok(())
}
