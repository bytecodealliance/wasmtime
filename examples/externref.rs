//! Small example of how to use `externref`s.

// You can execute this example with `cargo run --example externref`

use anyhow::Result;
use wasmtime::*;

fn main() -> Result<()> {
    println!("Initializing...");
    let mut config = Config::new();
    config.wasm_reference_types(true);
    let engine = Engine::new(&config)?;
    let store = Store::new(&engine);

    println!("Compiling module...");
    let module = Module::from_file(&engine, "examples/externref.wat")?;

    println!("Instantiating module...");
    let imports = [];
    let instance = Instance::new(&store, &module, &imports)?;

    println!("Creating new `externref`...");
    let externref = ExternRef::new("Hello, World!");
    assert!(externref.data().is::<&'static str>());
    assert_eq!(
        *externref.data().downcast_ref::<&'static str>().unwrap(),
        "Hello, World!"
    );

    println!("Touching `externref` table...");
    let table = instance.get_table("table").unwrap();
    table.set(3, Some(externref.clone()).into())?;
    let elem = table.get(3).unwrap().unwrap_externref().unwrap();
    assert!(elem.ptr_eq(&externref));

    println!("Touching `externref` global...");
    let global = instance.get_global("global").unwrap();
    global.set(Some(externref.clone()).into())?;
    let global_val = global.get().unwrap_externref().unwrap();
    assert!(global_val.ptr_eq(&externref));

    println!("Calling `externref` func...");
    let func = instance.get_func("func").unwrap();
    let func = func.get1::<Option<ExternRef>, Option<ExternRef>>()?;
    let ret = func(Some(externref.clone()))?;
    assert!(ret.is_some());
    assert!(ret.unwrap().ptr_eq(&externref));

    println!("GCing within the store...");
    store.gc();

    println!("Done.");
    Ok(())
}
