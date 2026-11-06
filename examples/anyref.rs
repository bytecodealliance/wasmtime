//! Small example of how to use `anyref`s.

// You can execute this example with `cargo run --example anyref`

use wasmtime::*;

fn main() -> Result<()> {
    println!("Initializing...");
    let mut config = Config::new();
    config.wasm_reference_types(true);
    config.wasm_function_references(true);
    config.wasm_gc(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    println!("Compiling module...");
    let module = Module::from_file(&engine, "examples/anyref.wat")?;

    println!("Instantiating module...");
    let instance = Instance::new(&mut store, &module, &[])?;

    println!("Creating new `anyref` from i31...");
    let i31 = I31::wrapping_u32(1234);
    let anyref = AnyRef::from_i31(&mut store, i31);
    assert!(anyref.is_i31(&store)?);
    assert_eq!(anyref.as_i31(&store)?, Some(i31));

    println!("Touching `anyref` table...");
    let table = instance.get_table(&mut store, "table").unwrap();
    table.set(&mut store, 3, anyref.into())?;
    // Accessors return GC references "frozen": cheap, but only valid until
    // the store's GC heap or execution next changes. Root them to keep them.
    let elem = table
        .get(&mut store, 3)
        .unwrap() // assert in bounds
        .to_rooted(&mut store)?
        .unwrap_any() // assert it's an anyref table
        .copied()
        .unwrap(); // assert the anyref isn't null
    assert!(Rooted::ref_eq(&store, &elem, &anyref)?);

    println!("Touching `anyref` global...");
    let global = instance.get_global(&mut store, "global").unwrap();
    global.set(&mut store, Some(anyref).into())?;
    // A frozen reference can also be used directly, as long as it is valid.
    let global_val = global.get(&store).unwrap_frozen_anyref().unwrap();
    assert_eq!(global_val, anyref.to_frozen(&store)?);

    println!("Passing `anyref` into func...");
    let func = instance.get_typed_func::<Option<Rooted<AnyRef>>, ()>(&mut store, "take_anyref")?;
    func.call(&mut store, Some(anyref))?;

    println!("Getting `anyref` from func...");
    let func =
        instance.get_typed_func::<(), Option<Rooted<AnyRef>>>(&mut store, "return_anyref")?;
    let ret = func.call(&mut store, ())?;
    assert!(ret.is_some());
    assert_eq!(ret.unwrap().unwrap_i31(&store)?, I31::wrapping_u32(42));

    println!("GCing within the store...");
    store.gc(None)?;

    println!("Done.");
    Ok(())
}
