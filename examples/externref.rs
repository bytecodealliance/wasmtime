//! Small example of how to use `externref`s.

// You can execute this example with `cargo run --example externref`

use anyhow::Result;
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
    let externref = ExternRef::new("Hello, World!");
    assert!(externref.data().is::<&'static str>());
    assert_eq!(
        *externref.data().downcast_ref::<&'static str>().unwrap(),
        "Hello, World!"
    );

    println!("Touching `externref` table...");
    let table = instance.get_table(&mut store, "table").unwrap();
    table.set(&mut store, 3, Some(externref.clone()).into())?;
    let elem = table
        .get(&mut store, 3)
        .unwrap() // assert in bounds
        .unwrap_externref() // assert it's an externref table
        .unwrap(); // assert the externref isn't null
    assert!(elem.ptr_eq(&externref));

    println!("Touching `externref` global...");
    let global = instance.get_global(&mut store, "global").unwrap();
    global.set(&mut store, Some(externref.clone()).into())?;
    let global_val = global.get(&mut store).unwrap_externref().unwrap();
    assert!(global_val.ptr_eq(&externref));

    println!("Calling `externref` func...");
    let func =
        instance.get_typed_func::<Option<ExternRef>, Option<ExternRef>, _>(&mut store, "func")?;
    let ret = func.call(&mut store, Some(externref.clone()))?;
    assert!(ret.is_some());
    assert!(ret.unwrap().ptr_eq(&externref));

    println!("GCing within the store...");
    store.gc();

    println!("Done.");
    Ok(())
}
