//! An example of how to configure capturing core dumps when the guest Wasm
//! traps that can then be passed to external tools for post-mortem analysis.

// You can execute this example with `cargo run --example coredump`.

use wasmtime::*;

fn main() -> Result<()> {
    println!("Configure core dumps to be captured on trap.");
    let mut config = Config::new();
    config.coredump_on_trap(true);
    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    println!("Define a Wasm module that will mutate local state and then trap.");
    let module = Module::new(
        store.engine(),
        r#"
            (module $trapper
                (memory 10)
                (global $g (mut i32) (i32.const 0))

                (func (export "run")
                    call $a
                )

                (func $a
                    i32.const 0x1234
                    i64.const 42
                    i64.store
                    call $b
                )

                (func $b
                    i32.const 36
                    global.set $g
                    call $c
                )

                (func $c
                    unreachable
                )
            )
        "#,
    )?;

    println!("Instantiate the module.");
    let instance = Instance::new(&mut store, &module, &[])?;

    println!("Invoke its 'run' function.");
    let run = instance
        .get_func(&mut store, "run")
        .expect("should have 'run' export");
    let args = &[];
    let results = &mut [];
    let ok = run.call(&mut store, args, results);

    println!("Calling that function trapped.");
    assert!(ok.is_err());
    let err = ok.unwrap_err();
    assert!(err.is::<Trap>());

    println!("Extract the captured core dump.");
    let dump = err
        .downcast_ref::<WasmCoreDump>()
        .expect("should have an attached core dump, since we configured core dumps on");

    println!(
        "Number of memories in the core dump: {}",
        dump.memories().len()
    );
    for (i, mem) in dump.memories().iter().enumerate() {
        if let Some(addr) = mem.data(&store).iter().position(|byte| *byte != 0) {
            let val = mem.data(&store)[addr];
            println!("  First nonzero byte for memory {i}: {val} @ {addr:#x}");
        } else {
            println!("  Memory {i} is all zeroes.");
        }
    }

    println!(
        "Number of globals in the core dump: {}",
        dump.globals().len()
    );
    for (i, global) in dump.globals().iter().enumerate() {
        let val = global.get(&mut store);
        println!("  Global {i} = {val:?}");
    }

    println!("Serialize the core dump and write it to ./example.coredump");
    let serialized = dump.serialize(&mut store, "trapper.wasm");
    std::fs::write("./example.coredump", serialized)?;

    Ok(())
}
