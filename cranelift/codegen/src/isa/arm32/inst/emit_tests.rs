//! Encoding tests for arm32 instructions.

use super::*;

/// Emit a single instruction and return the encoded bytes.
fn encode(inst: Inst) -> Vec<u8> {
    use crate::isa::arm32::settings as arm32_settings;
    use crate::settings::{self, Configurable};

    let mut b = settings::builder();
    b.set("enable_verifier", "false").unwrap();
    let flags = settings::Flags::new(b);
    let isa_flags = arm32_settings::Flags::new(&flags, &arm32_settings::builder());
    let emit_info = EmitInfo::new(flags, isa_flags);

    let mut buffer = MachBuffer::new();
    let mut state = EmitState::default();
    inst.emit(&mut buffer, &emit_info, &mut state);
    let mut ctrl_plane = Default::default();
    let buffer = buffer.finish(&Default::default(), &mut ctrl_plane);
    buffer.data().to_vec()
}

fn u32_le(inst: Inst) -> u32 {
    let bytes = encode(inst);
    assert_eq!(bytes.len(), 4, "expected a single 4-byte instruction");
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

#[test]
fn bx_lr() {
    assert_eq!(u32_le(Inst::Ret), 0xe12f_ff1e);
}

#[test]
fn nop4() {
    assert_eq!(u32_le(Inst::Nop4), 0xe320_f000);
}

#[test]
fn mov_reg() {
    // mov r0, r1
    assert_eq!(
        u32_le(Inst::Mov {
            rd: writable_xreg(0),
            rm: xreg(1),
        }),
        0xe1a0_0001
    );
}

#[test]
fn movw_small_imm() {
    // movw r0, #42
    assert_eq!(
        u32_le(Inst::MovImm {
            rd: writable_xreg(0),
            imm: 42,
        }),
        0xe300_002a
    );
}
