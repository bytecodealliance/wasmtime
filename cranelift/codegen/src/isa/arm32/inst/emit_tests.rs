//! Binary-encoding tests for the arm32 (Thumb-2) backend.

// use crate::ir::types::*;
use crate::isa::arm32;
use crate::isa::arm32::inst::*;
use crate::machinst::MachBuffer;
use crate::settings;
use alloc::vec::Vec;

#[test]
fn test_arm32_binemit() {
    // (instruction, expected little-endian hex bytes as uppercase string)
    let mut insns = Vec::<(Inst, &str)>::new();

    // ---- Group A: moves & immediates ----
    // Ret: BX LR = 0x4770, bytes in memory [0x70, 0x47] -> "7047"
    insns.push((Inst::Ret {}, "7047"));

    // ---- Pseudo-instructions (zero bytes) ----
    // Rets: pseudo-instruction, emits no bytes
    // insns.push((Inst::Rets { rets: Vec::new() }, ""));

    // Args: pseudo-instruction, emits no bytes
    // insns.push((Inst::Args { args: Vec::new() }, ""));

    let flags = settings::Flags::new(settings::builder());
    let isa_flags = arm32::settings::Flags::new(&flags, &arm32::settings::builder());
    let emit_info = EmitInfo::new(flags, isa_flags);

    for (insn, expected) in insns {
        let mut buffer = MachBuffer::new();
        insn.emit(&mut buffer, &emit_info, &mut Default::default());
        let buffer = buffer.finish(&Default::default(), &mut Default::default());
        let actual = buffer.stringify_code_bytes();
        assert_eq!(expected, actual, "bad encoding for {insn:?}");
    }
}
