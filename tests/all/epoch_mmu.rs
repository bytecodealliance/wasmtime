#![cfg(not(miri))]

use object::{LittleEndian, Object, ObjectSection, U32Bytes};
use wasmtime::{Config, Engine, Instance, Module, Store};
use wasmtime_environ::obj::ELF_WASMTIME_EPOCH_CHECKS;

/// Asserts that each epoch-check offset encoded into the binary points to the
/// byte after its corresponding dead load.
#[test]
fn epoch_check_offsets() {
    let mut config = Config::new();
    config.target("x86_64").unwrap();
    config.epoch_interruption_via_mmu(true);
    let engine = Engine::new(&config).unwrap();

    // A function with an infinite loop contains two epoch checks: one in the
    // function prologue and another at the loop backedge.
    let elf_bytes = engine
        .precompile_module(
            // If you change this wat, change it in
            // epoch-interruption-mmu-compile-loop.wat, too.
            r#"(module
             (memory 0)
             (func (loop (br 0)))
           )"#
            .as_bytes(),
        )
        .unwrap();

    let elf = object::read::elf::ElfFile64::<object::Endianness>::parse(&*elf_bytes)
        .expect("ELF should be parseable");
    let section = elf
        .section_by_name(ELF_WASMTIME_EPOCH_CHECKS)
        .expect(&format!(
            "{ELF_WASMTIME_EPOCH_CHECKS} section should be present"
        ));
    let data = section.data().unwrap();

    let (count_raw, rest) = object::from_bytes::<U32Bytes<LittleEndian>>(data).expect(
        ".wasmtime.epochchecks section should be long enough to contain a count of epoch checks",
    );
    let count = count_raw.get(LittleEndian) as usize;
    let (starts_raw, rest) = object::slice_from_bytes::<U32Bytes<LittleEndian>>(rest, count)
        .expect(".wasmtime.epochchecks section should be long enough to contain a location for each epoch check");
    let starts: Vec<u32> = starts_raw.iter().map(|b| b.get(LittleEndian)).collect();
    let (length_bits, _rest) = object::slice_from_bytes::<u8>(rest, count.div_ceil(8))
        .expect(".wasmtime.epochchecks section should be long enough to contain a length bit for each epoch check");

    // The emitted machine code is nailed down by the
    // epoch-interruption-mmu-compile-loop.wat disas test. As long as that keeps
    // passing, these values remain valid.
    assert_eq!(
        starts,
        vec![12, 15],
        "There should be 2 epoch checks (function prologue & loop backedge). The offset of the prologue's dead load should be 12, and that of the loop's backedge should be 15."
    );
    assert_eq!(
        length_bits,
        vec![0],
        "Neither check's load instruction uses R12 of RSP as its source, so all length bits should be 0."
    );
}

#[test]
fn epoch_mmu_trap_via_signal_handler() {
/// Runs a wasm function with MMU-based epoch interruption enabled and the epoch
/// ended. Make sure the function returns happily after the interruption.
    let mut config = Config::new();
    config.epoch_interruption_via_mmu(true);
    let engine = Engine::new(&config).unwrap();
    let module = Module::new(
        &engine,
        r#"(module
             (memory 0)
             (func (export "answer") (result i32)
                i32.const 42
             )
           )"#,
    )
    .unwrap();

    // Trap as soon as the first epoch check is encountered, in the function
    // prologue. Recall that MMU-based epochs don't operate based on a numeric
    // deadline but on an external entity protecting the memory page, typically
    // on a timer.
    let mut store = Store::new(&engine, ());
    store.epoch_deadline_trap(); // Allegedly the default.
    // Protect that page:
    store.end_mmu_epoch();

    let instance = Instance::new(&mut store, &module, &[]).unwrap();
    let func = instance
        .get_typed_func::<(), i32>(&mut store, "answer")
        .unwrap();

    let result = func.call(&mut store, ()).unwrap();
    assert_eq!(result, 42);
}
