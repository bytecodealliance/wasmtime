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
    let externref = match ExternRef::new(&mut store, "Hello, World!") {
        Ok(x) => x,
        Err(e) => match e.downcast::<GcHeapOutOfMemory<&str>>() {
            Ok(oom) => {
                let (inner, oom) = oom.take_inner();
                store.gc(Some(&oom))?;
                ExternRef::new(&mut store, inner)?
            }
            Err(e) => return Err(e),
        },
    };
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
    // Accessors return GC references "frozen": cheap, but only valid until
    // the store's GC heap or execution next changes. Root them to keep them.
    let elem = table
        .get(&mut store, 3)
        .unwrap() // assert in bounds
        .to_rooted(&mut store)?
        .unwrap_extern() // assert it's an externref table
        .copied()
        .unwrap(); // assert the externref isn't null
    assert!(Rooted::ref_eq(&store, &elem, &externref)?);

    println!("Touching `externref` global...");
    let global = instance.get_global(&mut store, "global").unwrap();
    global.set(&mut store, Some(externref).into())?;
    // A frozen reference can also be used directly, as long as it is valid.
    let global_val = global.get(&store).unwrap_frozen_externref().unwrap();
    assert_eq!(global_val, externref.to_frozen(&store)?);

    println!("Calling `externref` func...");
    let func = instance.get_typed_func::<Option<Rooted<ExternRef>>, Option<Rooted<ExternRef>>>(
        &mut store, "func",
    )?;
    let ret = func.call(&mut store, Some(externref))?;
    assert!(ret.is_some());
    assert!(Rooted::ref_eq(&store, &ret.unwrap(), &externref)?);

    println!("GCing within the store...");
    store.gc(None)?;

    println!("Done.");
    Ok(())
}
