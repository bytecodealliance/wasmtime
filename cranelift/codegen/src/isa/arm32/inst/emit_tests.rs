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

fn rot(v: u32) -> u32 {
    encode_rotated_imm(v).unwrap()
}

#[test]
fn data_movement() {
    assert_eq!(u32_le(Inst::Ret), 0xe12f_ff1e); // bx lr
    assert_eq!(u32_le(Inst::Nop4), 0xe320_f000);
    assert_eq!(
        u32_le(Inst::MovReg {
            rd: writable_xreg(0),
            rm: xreg(1),
        }),
        0xe1a0_0001
    );
    assert_eq!(
        u32_le(Inst::Movw {
            rd: writable_xreg(0),
            imm16: 42,
        }),
        0xe300_002a
    );
    assert_eq!(
        u32_le(Inst::MovImm {
            rd: writable_xreg(0),
            imm: 42,
        }),
        0xe300_002a
    );
    assert_eq!(
        u32_le(Inst::MovRotImm {
            rd: writable_xreg(0),
            imm12: rot(255),
        }),
        0xe3a0_00ff // mov r0, #255
    );
    assert_eq!(
        u32_le(Inst::MvnRotImm {
            rd: writable_xreg(0),
            imm12: rot(0),
        }),
        0xe3e0_0000 // mvn r0, #0
    );
}

#[test]
fn arithmetic() {
    assert_eq!(
        u32_le(Inst::AluRRR {
            op: ALUOp::Add,
            rd: writable_xreg(0),
            rn: xreg(1),
            rm: xreg(2),
        }),
        0xe081_0002 // add r0, r1, r2
    );
    assert_eq!(
        u32_le(Inst::AluRRImm {
            op: ALUOp::Add,
            rd: writable_xreg(0),
            rn: xreg(1),
            imm12: rot(1),
        }),
        0xe281_0001 // add r0, r1, #1
    );
    assert_eq!(
        u32_le(Inst::AluRRR {
            op: ALUOp::Sub,
            rd: writable_xreg(0),
            rn: xreg(1),
            rm: xreg(2),
        }),
        0xe041_0002 // sub r0, r1, r2
    );
    assert_eq!(
        u32_le(Inst::AluRRImm {
            op: ALUOp::Rsb,
            rd: writable_xreg(0),
            rn: xreg(1),
            imm12: rot(0),
        }),
        0xe261_0000 // rsb r0, r1, #0
    );
}

#[test]
fn logical() {
    let cases = [
        (ALUOp::And, 0xe001_0002u32), // and r0, r1, r2
        (ALUOp::Orr, 0xe181_0002),    // orr r0, r1, r2
        (ALUOp::Eor, 0xe021_0002),    // eor r0, r1, r2
        (ALUOp::Bic, 0xe1c1_0002),    // bic r0, r1, r2
    ];
    for (op, want) in cases {
        assert_eq!(
            u32_le(Inst::AluRRR {
                op,
                rd: writable_xreg(0),
                rn: xreg(1),
                rm: xreg(2),
            }),
            want,
            "{op:?}"
        );
    }
    assert_eq!(
        u32_le(Inst::MvnReg {
            rd: writable_xreg(0),
            rm: xreg(1),
        }),
        0xe1e0_0001 // mvn r0, r1
    );
}

#[test]
fn flag_setting() {
    assert_eq!(
        u32_le(Inst::AluRRRFlags {
            op: ALUOp::Add,
            rd: writable_xreg(0),
            rn: xreg(1),
            rm: xreg(2),
        }),
        0xe091_0002 // adds r0, r1, r2
    );
    assert_eq!(
        u32_le(Inst::AluRRRFlags {
            op: ALUOp::Sbc,
            rd: writable_xreg(0),
            rn: xreg(1),
            rm: xreg(2),
        }),
        0xe0d1_0002 // sbcs r0, r1, r2
    );
    assert_eq!(
        u32_le(Inst::CmpRR {
            op: CmpOp::Tst,
            rn: xreg(0),
            rm: xreg(1),
        }),
        0xe110_0001 // tst r0, r1
    );
    assert_eq!(
        u32_le(Inst::CmpRR {
            op: CmpOp::Teq,
            rn: xreg(0),
            rm: xreg(1),
        }),
        0xe130_0001 // teq r0, r1
    );
}

#[test]
fn shifts() {
    assert_eq!(
        u32_le(Inst::ShiftImm {
            op: ShiftOp::Lsl,
            rd: writable_xreg(0),
            rm: xreg(1),
            amount: 2,
        }),
        0xe1a0_0101 // lsl r0, r1, #2
    );
    assert_eq!(
        u32_le(Inst::ShiftImm {
            op: ShiftOp::Asr,
            rd: writable_xreg(0),
            rm: xreg(1),
            amount: 3,
        }),
        0xe1a0_01c1 // asr r0, r1, #3
    );
    assert_eq!(
        u32_le(Inst::ShiftReg {
            op: ShiftOp::Lsl,
            rd: writable_xreg(0),
            rm: xreg(1),
            rs: xreg(2),
        }),
        0xe1a0_0211 // lsl r0, r1, r2
    );
}

#[test]
fn multiplies() {
    assert_eq!(
        u32_le(Inst::Mul {
            rd: writable_xreg(0),
            rn: xreg(1),
            rm: xreg(2),
        }),
        0xe000_0291 // mul r0, r1, r2
    );
    assert_eq!(
        u32_le(Inst::Mla {
            rd: writable_xreg(0),
            rn: xreg(1),
            rm: xreg(2),
            ra: xreg(3),
        }),
        0xe020_3291 // mla r0, r1, r2, r3
    );
    assert_eq!(
        u32_le(Inst::Umull {
            rd_lo: writable_xreg(0),
            rd_hi: writable_xreg(1),
            rn: xreg(2),
            rm: xreg(3),
        }),
        0xe081_0392 // umull r0, r1, r2, r3
    );
    assert_eq!(
        u32_le(Inst::Smull {
            rd_lo: writable_xreg(0),
            rd_hi: writable_xreg(1),
            rn: xreg(2),
            rm: xreg(3),
        }),
        0xe0c1_0392 // smull r0, r1, r2, r3
    );
}

#[test]
fn bit_ops() {
    let bit_cases = [
        (BitOp::Clz, 0xe16f_0f11u32),
        (BitOp::Rev, 0xe6bf_0f31),
        (BitOp::Rev16, 0xe6bf_0fb1),
        (BitOp::Rbit, 0xe6ff_0f31),
    ];
    for (op, want) in bit_cases {
        assert_eq!(
            u32_le(Inst::BitRR {
                op,
                rd: writable_xreg(0),
                rm: xreg(1),
            }),
            want,
            "{op:?}"
        );
    }
    let ext_cases = [
        (ExtOp::Sxtb, 0xe6af_0071u32),
        (ExtOp::Sxth, 0xe6bf_0071),
        (ExtOp::Uxtb, 0xe6ef_0071),
        (ExtOp::Uxth, 0xe6ff_0071),
    ];
    for (op, want) in ext_cases {
        assert_eq!(
            u32_le(Inst::ExtRR {
                op,
                rd: writable_xreg(0),
                rm: xreg(1),
            }),
            want,
            "{op:?}"
        );
    }
}

#[test]
fn divides() {
    assert_eq!(
        u32_le(Inst::SDiv {
            rd: writable_xreg(0),
            rn: xreg(1),
            rm: xreg(2),
        }),
        0xe710_f211 // sdiv r0, r1, r2
    );
    assert_eq!(
        u32_le(Inst::UDiv {
            rd: writable_xreg(0),
            rn: xreg(1),
            rm: xreg(2),
        }),
        0xe730_f211 // udiv r0, r1, r2
    );
}

#[test]
fn fused_multiply() {
    assert_eq!(
        u32_le(Inst::Mls {
            rd: writable_xreg(0),
            rn: xreg(1),
            rm: xreg(2),
            ra: xreg(3),
        }),
        0xe060_3291 // mls r0, r1, r2, r3
    );
    assert_eq!(
        u32_le(Inst::Umlal {
            rd_lo: writable_xreg(0),
            rd_hi: writable_xreg(1),
            rn: xreg(2),
            rm: xreg(3),
        }),
        0xe0a1_0392 // umlal r0, r1, r2, r3
    );
    assert_eq!(
        u32_le(Inst::Smlal {
            rd_lo: writable_xreg(0),
            rd_hi: writable_xreg(1),
            rn: xreg(2),
            rm: xreg(3),
        }),
        0xe0e1_0392 // smlal r0, r1, r2, r3
    );
}

#[test]
fn conditional_select() {
    // csel ne r0, r1, r2  =>  mov r0, r2 ; movne r0, r1
    let bytes = encode(Inst::CSel {
        cond: Cond::Ne,
        rd: writable_xreg(0),
        rn: xreg(1),
        rm: xreg(2),
    });
    assert_eq!(bytes.len(), 8);
    let w0 = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let w1 = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    assert_eq!(w0, 0xe1a0_0002); // mov r0, r2
    assert_eq!(w1, 0x11a0_0001); // movne r0, r1
}

#[test]
fn bitfield() {
    assert_eq!(
        u32_le(Inst::Bfc {
            rd: writable_xreg(0),
            lsb: 4,
            width: 8,
        }),
        0xe7cb_021f // bfc r0, #4, #8
    );
    assert_eq!(
        u32_le(Inst::Bfi {
            rd: writable_xreg(0),
            rn: xreg(1),
            lsb: 4,
            width: 8,
        }),
        0xe7cb_0211 // bfi r0, r1, #4, #8
    );
    assert_eq!(
        u32_le(Inst::Bfx {
            op: BfxOp::Sbfx,
            rd: writable_xreg(0),
            rn: xreg(1),
            lsb: 4,
            width: 8,
        }),
        0xe7a7_0251 // sbfx r0, r1, #4, #8
    );
    assert_eq!(
        u32_le(Inst::Bfx {
            op: BfxOp::Ubfx,
            rd: writable_xreg(0),
            rn: xreg(1),
            lsb: 4,
            width: 8,
        }),
        0xe7e7_0251 // ubfx r0, r1, #4, #8
    );
}

#[test]
fn saturating() {
    let q_cases = [
        (QAluOp::Qadd, 0xe102_0051u32),
        (QAluOp::Qsub, 0xe122_0051),
        (QAluOp::Qdadd, 0xe142_0051),
        (QAluOp::Qdsub, 0xe162_0051),
    ];
    for (op, want) in q_cases {
        // qadd r0, r1, r2  =>  Rd=0, Rm=1, Rn=2
        assert_eq!(
            u32_le(Inst::QAlu {
                op,
                rd: writable_xreg(0),
                rm: xreg(1),
                rn: xreg(2),
            }),
            want,
            "{op:?}"
        );
    }
    assert_eq!(
        u32_le(Inst::Sat {
            op: SatOp::Ssat,
            rd: writable_xreg(0),
            sat_bits: 8,
            rm: xreg(1),
        }),
        0xe6a7_0011 // ssat r0, #8, r1
    );
    assert_eq!(
        u32_le(Inst::Sat {
            op: SatOp::Usat,
            rd: writable_xreg(0),
            sat_bits: 8,
            rm: xreg(1),
        }),
        0xe6e8_0011 // usat r0, #8, r1
    );
}

#[test]
fn misc_alu() {
    assert_eq!(
        u32_le(Inst::Sel {
            rd: writable_xreg(0),
            rn: xreg(1),
            rm: xreg(2),
        }),
        0xe681_0fb2 // sel r0, r1, r2
    );
    assert_eq!(
        u32_le(Inst::Pkh {
            op: PkhOp::Pkhbt,
            rd: writable_xreg(0),
            rn: xreg(1),
            rm: xreg(2),
        }),
        0xe681_0012 // pkhbt r0, r1, r2
    );
    assert_eq!(
        u32_le(Inst::Pkh {
            op: PkhOp::Pkhtb,
            rd: writable_xreg(0),
            rn: xreg(1),
            rm: xreg(2),
        }),
        0xe681_0052 // pkhtb r0, r1, r2
    );
    assert_eq!(
        u32_le(Inst::Rrx {
            rd: writable_xreg(0),
            rm: xreg(1),
        }),
        0xe1a0_0061 // rrx r0, r1
    );
    assert_eq!(
        u32_le(Inst::BitRR {
            op: BitOp::Revsh,
            rd: writable_xreg(0),
            rm: xreg(1),
        }),
        0xe6ff_0fb1 // revsh r0, r1
    );
    assert_eq!(
        u32_le(Inst::Udf {
            code: crate::ir::TrapCode::STACK_OVERFLOW,
        }),
        0xe7f0_00f0 // udf #0
    );
}

#[test]
fn extend_variants() {
    assert_eq!(
        u32_le(Inst::ExtRR {
            op: ExtOp::Sxtb16,
            rd: writable_xreg(0),
            rm: xreg(1),
        }),
        0xe68f_0071 // sxtb16 r0, r1
    );
    assert_eq!(
        u32_le(Inst::ExtRR {
            op: ExtOp::Uxtb16,
            rd: writable_xreg(0),
            rm: xreg(1),
        }),
        0xe6cf_0071 // uxtb16 r0, r1
    );
    let add_cases = [
        (ExtAddOp::Sxtab, 0xe6a1_0072u32),
        (ExtAddOp::Sxtah, 0xe6b1_0072),
        (ExtAddOp::Sxtab16, 0xe681_0072),
        (ExtAddOp::Uxtab, 0xe6e1_0072),
        (ExtAddOp::Uxtah, 0xe6f1_0072),
        (ExtAddOp::Uxtab16, 0xe6c1_0072),
    ];
    for (op, want) in add_cases {
        assert_eq!(
            u32_le(Inst::ExtAdd {
                op,
                rd: writable_xreg(0),
                rn: xreg(1),
                rm: xreg(2),
            }),
            want,
            "{op:?}"
        );
    }
}

#[test]
fn parallel_add_sub() {
    let par = |op| {
        u32_le(Inst::ParAlu {
            op,
            rd: writable_xreg(0),
            rn: xreg(1),
            rm: xreg(2),
        })
    };
    // Hand-verified reference encodings (`op r0, r1, r2`).
    assert_eq!(par(ParAluOp::Sadd8), 0xe611_0f92);
    assert_eq!(par(ParAluOp::Ssub8), 0xe611_0ff2);
    assert_eq!(par(ParAluOp::Uadd8), 0xe651_0f92);
    assert_eq!(par(ParAluOp::Uadd16), 0xe651_0f12);
    assert_eq!(par(ParAluOp::Uhadd16), 0xe671_0f12);

    // All 36 members must encode to distinct, well-formed words that share the
    // parallel add/sub encoding skeleton.
    let all = [
        ParAluOp::Sadd8,
        ParAluOp::Sadd16,
        ParAluOp::Ssub8,
        ParAluOp::Ssub16,
        ParAluOp::Sasx,
        ParAluOp::Ssax,
        ParAluOp::Qadd8,
        ParAluOp::Qadd16,
        ParAluOp::Qsub8,
        ParAluOp::Qsub16,
        ParAluOp::Qasx,
        ParAluOp::Qsax,
        ParAluOp::Shadd8,
        ParAluOp::Shadd16,
        ParAluOp::Shsub8,
        ParAluOp::Shsub16,
        ParAluOp::Shasx,
        ParAluOp::Shsax,
        ParAluOp::Uadd8,
        ParAluOp::Uadd16,
        ParAluOp::Usub8,
        ParAluOp::Usub16,
        ParAluOp::Uasx,
        ParAluOp::Usax,
        ParAluOp::Uqadd8,
        ParAluOp::Uqadd16,
        ParAluOp::Uqsub8,
        ParAluOp::Uqsub16,
        ParAluOp::Uqasx,
        ParAluOp::Uqsax,
        ParAluOp::Uhadd8,
        ParAluOp::Uhadd16,
        ParAluOp::Uhsub8,
        ParAluOp::Uhsub16,
        ParAluOp::Uhasx,
        ParAluOp::Uhsax,
    ];
    let mut seen = alloc::collections::BTreeSet::new();
    for op in all {
        let w = par(op);
        // Fixed skeleton: cond=AL + [27:24]=0110, bit23=0, [11:8]=1111 & bit4=1,
        // and the register fields Rn=1, Rd=0, Rm=2.
        assert_eq!(w & 0xff80_0000, 0xe600_0000, "{op:?}");
        assert_eq!(w & 0x0000_0f10, 0x0000_0f10, "{op:?}");
        assert_eq!(w & 0x000f_f00f, 0x0001_0002, "{op:?}");
        assert!(seen.insert(w), "duplicate encoding for {op:?}");
    }
    assert_eq!(seen.len(), 36);
}

#[test]
fn dsp_multiplies() {
    let m3 = |op| {
        u32_le(Inst::DspMul3 {
            op,
            rd: writable_xreg(0),
            rn: xreg(1),
            rm: xreg(2),
        })
    };
    // `op r0, r1, r2`
    assert_eq!(m3(DspMul3Op::Smulbb), 0xe160_0281);
    assert_eq!(m3(DspMul3Op::Smulbt), 0xe160_02c1);
    assert_eq!(m3(DspMul3Op::Smultb), 0xe160_02a1);
    assert_eq!(m3(DspMul3Op::Smultt), 0xe160_02e1);
    assert_eq!(m3(DspMul3Op::Smulwb), 0xe120_02a1);
    assert_eq!(m3(DspMul3Op::Smulwt), 0xe120_02e1);
    assert_eq!(m3(DspMul3Op::Smmul), 0xe750_f211);
    assert_eq!(m3(DspMul3Op::Smuad), 0xe700_f211);
    assert_eq!(m3(DspMul3Op::Smusd), 0xe700_f251);

    let m4 = |op| {
        u32_le(Inst::DspMul4 {
            op,
            rd: writable_xreg(0),
            rn: xreg(1),
            rm: xreg(2),
            ra: xreg(3),
        })
    };
    // `op r0, r1, r2, r3`
    assert_eq!(m4(DspMul4Op::Smlabb), 0xe100_3281);
    assert_eq!(m4(DspMul4Op::Smlabt), 0xe100_32c1);
    assert_eq!(m4(DspMul4Op::Smlatb), 0xe100_32a1);
    assert_eq!(m4(DspMul4Op::Smlatt), 0xe100_32e1);
    assert_eq!(m4(DspMul4Op::Smlawb), 0xe120_3281);
    assert_eq!(m4(DspMul4Op::Smlawt), 0xe120_32c1);
    assert_eq!(m4(DspMul4Op::Smmla), 0xe750_3211);
    assert_eq!(m4(DspMul4Op::Smmls), 0xe750_32d1);
    assert_eq!(m4(DspMul4Op::Smlad), 0xe700_3211);
    assert_eq!(m4(DspMul4Op::Smlsd), 0xe700_3251);

    let ml = |op| {
        u32_le(Inst::DspMulL {
            op,
            rd_lo: writable_xreg(0),
            rd_hi: writable_xreg(1),
            rn: xreg(2),
            rm: xreg(3),
        })
    };
    // `op r0, r1, r2, r3`
    assert_eq!(ml(DspMulLOp::Smlalbb), 0xe141_0382);
    assert_eq!(ml(DspMulLOp::Smlalbt), 0xe141_03c2);
    assert_eq!(ml(DspMulLOp::Smlaltb), 0xe141_03a2);
    assert_eq!(ml(DspMulLOp::Smlaltt), 0xe141_03e2);
    assert_eq!(ml(DspMulLOp::Smlald), 0xe741_0312);
    assert_eq!(ml(DspMulLOp::Smlsld), 0xe741_0352);
    assert_eq!(ml(DspMulLOp::Umaal), 0xe041_0392);
}

#[test]
fn memory_multiple_and_exclusive() {
    assert_eq!(
        u32_le(Inst::LdmStm {
            load: true,
            rn: xreg(0),
            writeback: true,
            reg_list: (1 << 1) | (1 << 2),
        }),
        0xe8b0_0006 // ldmia r0!, {r1, r2}
    );
    assert_eq!(
        u32_le(Inst::LdmStm {
            load: false,
            rn: xreg(0),
            writeback: false,
            reg_list: (1 << 1) | (1 << 2),
        }),
        0xe880_0006 // stmia r0, {r1, r2}
    );
    assert_eq!(
        u32_le(Inst::LoadEx {
            acquire: false,
            size: AtomicSize::Word,
            rt: writable_xreg(1),
            rn: xreg(0),
        }),
        0xe190_1f9f // ldrex r1, [r0]
    );
    assert_eq!(
        u32_le(Inst::LoadEx {
            acquire: true,
            size: AtomicSize::Word,
            rt: writable_xreg(1),
            rn: xreg(0),
        }),
        0xe190_1e9f // ldaex r1, [r0]
    );
    assert_eq!(
        u32_le(Inst::LoadEx {
            acquire: true,
            size: AtomicSize::Byte,
            rt: writable_xreg(1),
            rn: xreg(0),
        }),
        0xe1d0_1e9f // ldaexb r1, [r0]
    );
    assert_eq!(
        u32_le(Inst::LoadEx {
            acquire: true,
            size: AtomicSize::Half,
            rt: writable_xreg(1),
            rn: xreg(0),
        }),
        0xe1f0_1e9f // ldaexh r1, [r0]
    );
    assert_eq!(
        u32_le(Inst::StoreEx {
            acquire: false,
            size: AtomicSize::Word,
            rd: writable_xreg(0),
            rt: xreg(1),
            rn: xreg(2),
        }),
        0xe182_0f91 // strex r0, r1, [r2]
    );
    assert_eq!(
        u32_le(Inst::StoreEx {
            acquire: true,
            size: AtomicSize::Word,
            rd: writable_xreg(0),
            rt: xreg(1),
            rn: xreg(2),
        }),
        0xe182_0e91 // stlex r0, r1, [r2]
    );
    assert_eq!(
        u32_le(Inst::StoreEx {
            acquire: true,
            size: AtomicSize::Byte,
            rd: writable_xreg(0),
            rt: xreg(1),
            rn: xreg(2),
        }),
        0xe1c2_0e91 // stlexb r0, r1, [r2]
    );
    assert_eq!(
        u32_le(Inst::StoreEx {
            acquire: true,
            size: AtomicSize::Half,
            rd: writable_xreg(0),
            rt: xreg(1),
            rn: xreg(2),
        }),
        0xe1e2_0e91 // stlexh r0, r1, [r2]
    );
    assert_eq!(
        u32_le(Inst::LoadAcq {
            size: AtomicSize::Word,
            rt: writable_xreg(1),
            rn: xreg(0),
        }),
        0xe190_1c9f // lda r1, [r0]
    );
    assert_eq!(
        u32_le(Inst::LoadAcq {
            size: AtomicSize::Byte,
            rt: writable_xreg(1),
            rn: xreg(0),
        }),
        0xe1d0_1c9f // ldab r1, [r0]
    );
    assert_eq!(
        u32_le(Inst::LoadAcq {
            size: AtomicSize::Half,
            rt: writable_xreg(1),
            rn: xreg(0),
        }),
        0xe1f0_1c9f // ldah r1, [r0]
    );
    assert_eq!(
        u32_le(Inst::StoreRel {
            size: AtomicSize::Word,
            rt: xreg(1),
            rn: xreg(0),
        }),
        0xe180_fc91 // stl r1, [r0]
    );
    assert_eq!(
        u32_le(Inst::StoreRel {
            size: AtomicSize::Byte,
            rt: xreg(1),
            rn: xreg(0),
        }),
        0xe1c0_fc91 // stlb r1, [r0]
    );
    assert_eq!(
        u32_le(Inst::StoreRel {
            size: AtomicSize::Half,
            rt: xreg(1),
            rn: xreg(0),
        }),
        0xe1e0_fc91 // stlh r1, [r0]
    );
}

#[test]
fn barriers() {
    assert_eq!(u32_le(Inst::Barrier { op: BarrierOp::Dmb }), 0xf57f_f05f);
    assert_eq!(u32_le(Inst::Barrier { op: BarrierOp::Dsb }), 0xf57f_f04f);
    assert_eq!(u32_le(Inst::Barrier { op: BarrierOp::Isb }), 0xf57f_f06f);
    assert_eq!(
        u32_le(Inst::Barrier {
            op: BarrierOp::Clrex
        }),
        0xf57f_f01f
    );
}

#[test]
fn compares() {
    assert_eq!(
        u32_le(Inst::CmpRR {
            op: CmpOp::Cmp,
            rn: xreg(0),
            rm: xreg(1),
        }),
        0xe150_0001 // cmp r0, r1
    );
    assert_eq!(
        u32_le(Inst::CmpRImm {
            op: CmpOp::Cmp,
            rn: xreg(0),
            imm12: rot(0),
        }),
        0xe350_0000 // cmp r0, #0
    );
}

#[test]
fn memory() {
    assert_eq!(
        u32_le(Inst::Load {
            rt: writable_xreg(0),
            mem: AMode::RegOffset {
                rn: xreg(1),
                offset: 4
            },
            kind: LoadKind::Word,
        }),
        0xe591_0004 // ldr r0, [r1, #4]
    );
    assert_eq!(
        u32_le(Inst::Store {
            rt: xreg(0),
            mem: AMode::RegOffset {
                rn: xreg(1),
                offset: 4
            },
            kind: StoreKind::Word,
        }),
        0xe581_0004 // str r0, [r1, #4]
    );
    assert_eq!(
        u32_le(Inst::Load {
            rt: writable_xreg(0),
            mem: AMode::RegOffset {
                rn: xreg(1),
                offset: 4
            },
            kind: LoadKind::UByte,
        }),
        0xe5d1_0004 // ldrb r0, [r1, #4]
    );
    assert_eq!(
        u32_le(Inst::Store {
            rt: xreg(0),
            mem: AMode::RegOffset {
                rn: xreg(1),
                offset: 4
            },
            kind: StoreKind::Byte,
        }),
        0xe5c1_0004 // strb r0, [r1, #4]
    );
    assert_eq!(
        u32_le(Inst::Load {
            rt: writable_xreg(0),
            mem: AMode::RegOffset {
                rn: xreg(1),
                offset: 4
            },
            kind: LoadKind::UHalf,
        }),
        0xe1d1_00b4 // ldrh r0, [r1, #4]
    );
    assert_eq!(
        u32_le(Inst::Store {
            rt: xreg(0),
            mem: AMode::RegOffset {
                rn: xreg(1),
                offset: 4
            },
            kind: StoreKind::Half,
        }),
        0xe1c1_00b4 // strh r0, [r1, #4]
    );
    assert_eq!(
        u32_le(Inst::Load {
            rt: writable_xreg(0),
            mem: AMode::RegOffset {
                rn: xreg(1),
                offset: 4
            },
            kind: LoadKind::SByte,
        }),
        0xe1d1_00d4 // ldrsb r0, [r1, #4]
    );
    assert_eq!(
        u32_le(Inst::Load {
            rt: writable_xreg(0),
            mem: AMode::RegOffset {
                rn: xreg(1),
                offset: 4
            },
            kind: LoadKind::SHalf,
        }),
        0xe1d1_00f4 // ldrsh r0, [r1, #4]
    );
    // Register-offset word forms.
    assert_eq!(
        u32_le(Inst::Load {
            rt: writable_xreg(0),
            mem: AMode::RegReg {
                rn: xreg(1),
                rm: xreg(2)
            },
            kind: LoadKind::Word,
        }),
        0xe791_0002 // ldr r0, [r1, r2]
    );
    assert_eq!(
        u32_le(Inst::Store {
            rt: xreg(0),
            mem: AMode::RegReg {
                rn: xreg(1),
                rm: xreg(2)
            },
            kind: StoreKind::Word,
        }),
        0xe781_0002 // str r0, [r1, r2]
    );
}

#[test]
fn push_pop_and_sp() {
    // push/pop {fp, lr}
    let list = (1 << 11) | (1 << 14);
    assert_eq!(u32_le(Inst::Push { reg_list: list }), 0xe92d_4800);
    assert_eq!(u32_le(Inst::Pop { reg_list: list }), 0xe8bd_4800);
    assert_eq!(u32_le(Inst::AdjustSp { amount: -8 }), 0xe24d_d008);
    assert_eq!(u32_le(Inst::AdjustSp { amount: 8 }), 0xe28d_d008);
}
