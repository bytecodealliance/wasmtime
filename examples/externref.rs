//! Small example of how to use `externref`s.

// You can execute this example with `cargo run --example externref`

use wasmtime::*;

fn main() -> Result<()> {
    println!("Initializing...");
    let mut config = Config::new();
    config.wasm_reference_types(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    println!("Compiling module...");
    let module = Module::from_file(&engine, "examples/externref.wat")?;

    println!("Instantiating module...");
    let instance = Instance::new(&mut store, &module, &[])?;

    println!("Creating new `externref`...");
    let externref = ExternRef::new(&mut store, "Hello, World!")?;
    assert!(
        externref
            .data(&store)?
            .expect("should have host data")
            .is::<&'static str>()
    );
    assert_eq!(
        *externref
            .data(&store)?
            .expect("should have host data")
            .downcast_ref::<&'static str>()
            .unwrap(),
        "Hello, World!"
    );

    println!("Touching `externref` table...");
    let table = instance.get_table(&mut store, "table").unwrap();
    table.set(&mut store, 3, Some(externref).into())?;
    let elem = table
        .get(&mut store, 3)
        .unwrap() // assert in bounds
        .unwrap_extern() // assert it's an externref table
        .copied()
        .unwrap(); // assert the externref isn't null
    assert!(Rooted::ref_eq(&store, &elem, &externref)?);

    println!("Touching `externref` global...");
    let global = instance.get_global(&mut store, "global").unwrap();
    global.set(&mut store, Some(externref).into())?;
    let global_val = global.get(&mut store).unwrap_externref().copied().unwrap();
    assert!(Rooted::ref_eq(&store, &global_val, &externref)?);

    println!("Calling `externref` func...");
    let func = instance.get_typed_func::<Option<Rooted<ExternRef>>, Option<Rooted<ExternRef>>>(
        &mut store, "func",
    )?;
    let ret = func.call(&mut store, Some(externref))?;
    assert!(ret.is_some());
    assert!(Rooted::ref_eq(&store, &ret.unwrap(), &externref)?);

    println!("GCing within the store...");
    store.gc();

    println!("Done.");
    Ok(())
}
