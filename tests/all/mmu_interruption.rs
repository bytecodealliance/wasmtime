#![cfg(not(miri))]

use object::{LittleEndian, Object, ObjectSection, U32};
use wasmtime::{Config, Engine, Result};
use wasmtime_environ::obj::ELF_WASMTIME_MMU_INTERRUPT_CHECKS;
use wasmtime_test_macros::wasmtime_test;

// Asserts that each MMU-interrupt-check offset encoded into the binary points
// to the byte after its corresponding dead load.
#[wasmtime_test(strategies(only(CraneliftNative)))]
fn mmu_interrupt_check_offsets(config: &mut Config) -> Result<()> {
    config.mmu_interruption(true);
    config.target("x86_64").unwrap();
    let engine = Engine::new(config).unwrap();

    // A function with an infinite loop contains two MMU-interrupt checks: one
    // in the function prologue and another at the loop backedge.
    let elf_bytes = engine
        .precompile_module(
            // If you change this wat, change it in
            // mmu-interruption-compile-loop.wat, too.
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
        .section_by_name(ELF_WASMTIME_MMU_INTERRUPT_CHECKS)
        .expect(&format!(
            "{ELF_WASMTIME_MMU_INTERRUPT_CHECKS} section should be present"
        ));
    let data = section.data().unwrap();

    let (count_raw, rest) = object::from_bytes::<U32<LittleEndian>>(data).expect(
        ".wasmtime.mmu_interrupt_checks section should be long enough to contain a count of MMU-interrupt checks",
    );
    let count = count_raw.get(LittleEndian) as usize;
    let (starts_raw, rest) = object::slice_from_bytes::<U32<LittleEndian>>(rest, count)
        .expect(".wasmtime.mmu_interrupt_checks section should be long enough to contain a location for each MMU-interrupt check");
    let starts: Vec<u32> = starts_raw.iter().map(|b| b.get(LittleEndian)).collect();
    let (length_bits, _rest) = object::slice_from_bytes::<u8>(rest, count.div_ceil(8))
        .expect(".wasmtime.mmu_interrupt_checks section should be long enough to contain a length bit for each MMU-interrupt check");

    // The emitted machine code is nailed down by the
    // mmu-interruption-compile-loop.wat disas test. As long as that keeps
    // passing, these values remain valid.
    assert_eq!(
        starts,
        vec![12, 15],
        "There should be 2 MMU-interrupt checks (function prologue & loop backedge). The offset of the prologue's dead load should be 12, and that of the loop's backedge should be 15."
    );
    assert_eq!(
        length_bits,
        vec![0],
        "Neither check's load instruction uses R12 or RSP as its source, so all length bits should be 0."
    );
    Ok(())
}
