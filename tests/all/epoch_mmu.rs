#![cfg(not(miri))]

use object::{Object, ObjectSection};
use wasmtime::{Config, Engine};
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
    assert!(data.len() >= 4, "section should at least contain a count");
    let count = u32::from_le_bytes(data[..4].try_into().unwrap()) as usize;
    assert_eq!(
        data.len(),
        4 + count * 4,
        "section should be the right size to hold the offsets it claims to contain"
    );
    let offsets: Vec<u32> = data[4..]
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
        .collect();

    // The emitted machine code is nailed down by the
    // epoch-interruption-mmu-compile-loop.wat disas test. As long as that keeps
    // passing, these offsets remain valid.
    assert_eq!(
        offsets,
        vec![15, 18],
        "There should be 2 epoch checks (function prologue & loop backedge). The offset after the prologue's dead load should be 15, and the one after the loop's backedge should be 18."
    );
}
