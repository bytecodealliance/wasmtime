use crate::isa::x64::inst::*;
use regalloc::Reg;

fn low8willSXto64(x: u32) -> bool {
    let xs = (x as i32) as i64;
    xs == ((xs << 56) >> 56)
}

fn low8willSXto32(x: u32) -> bool {
    let xs = x as i32;
    xs == ((xs << 24) >> 24)
}

//=============================================================================
// Instructions and subcomponents: emission

// For all of the routines that take both a memory-or-reg operand (sometimes
// called "E" in the Intel documentation) and a reg-only operand ("G" in
// Intelese), the order is always G first, then E.
//
// "enc" in the following means "hardware register encoding number".

#[inline(always)]
fn mkModRegRM(m0d: u8, encRegG: u8, rmE: u8) -> u8 {
    debug_assert!(m0d < 4);
    debug_assert!(encRegG < 8);
    debug_assert!(rmE < 8);
    ((m0d & 3) << 6) | ((encRegG & 7) << 3) | (rmE & 7)
}

#[inline(always)]
fn mkSIB(shift: u8, encIndex: u8, encBase: u8) -> u8 {
    debug_assert!(shift < 4);
    debug_assert!(encIndex < 8);
    debug_assert!(encBase < 8);
    ((shift & 3) << 6) | ((encIndex & 7) << 3) | (encBase & 7)
}

/// Get the encoding number from something which we sincerely hope is a real
/// register of class I64.
#[inline(always)]
fn iregEnc(reg: Reg) -> u8 {
    debug_assert!(reg.is_real());
    reg.get_hw_encoding()
}

// F_*: these flags describe special handling of the insn to be generated.  Be
// careful with these.  It is easy to create nonsensical combinations.
const F_NONE: u32 = 0;

/// Emit the REX prefix byte even if it appears to be redundant (== 0x40).
const F_RETAIN_REDUNDANT_REX: u32 = 1;

/// Set the W bit in the REX prefix to zero.  By default it will be set to 1,
/// indicating a 64-bit operation.
const F_CLEAR_REX_W: u32 = 2;

/// For specifying the legacy prefixes (or `PfxNone` if no prefix required) to
/// be used at the start an instruction. A select prefix may be required for
/// various operations, including instructions that operate on GPR, SSE, and Vex
/// registers.
enum LegacyPrefix {
    PfxNone,
    Pfx66,
    PfxF2,
    PfxF3,
}
/// This is the core 'emit' function for instructions that reference memory.
///
/// For an instruction that has as operands a register `encG` and a memory
/// address `memE`, create and emit, first the REX prefix, then caller-supplied
/// opcode byte(s) (`opcodes` and `numOpcodes`), then the MOD/RM byte, then
/// optionally, a SIB byte, and finally optionally an immediate that will be
/// derived from the `memE` operand.  For most instructions up to and including
/// SSE4.2, that will be the whole instruction.
///
/// The opcodes are written bigendianly for the convenience of callers.  For
/// example, if the opcode bytes to be emitted are, in this order, F3 0F 27,
/// then the caller should pass `opcodes` == 0xF3_0F_27 and `numOpcodes` == 3.
///
/// The register operand is represented here not as a `Reg` but as its hardware
/// encoding, `encG`.  `flags` can specify special handling for the REX prefix.
/// By default, the REX prefix will indicate a 64-bit operation and will be
/// deleted if it is redundant (0x40).  Note that for a 64-bit operation, the
/// REX prefix will normally never be redundant, since REX.W must be 1 to
/// indicate a 64-bit operation.
fn emit_REX_OPCODES_MODRM_SIB_IMM_encG_memE(
    sink: &mut MachBuffer<Inst>,
    prefix: LegacyPrefix,
    opcodes: u32,
    mut numOpcodes: usize,
    encG: u8,
    memE: &Addr,
    flags: u32,
) {
    // General comment for this function: the registers in `memE` must be
    // 64-bit integer registers, because they are part of an address
    // expression.  But `encG` can be derived from a register of any class.
    let clearRexW = (flags & F_CLEAR_REX_W) != 0;
    let retainRedundant = (flags & F_RETAIN_REDUNDANT_REX) != 0;

    // Lower the prefix if applicable.
    match prefix {
        LegacyPrefix::Pfx66 => sink.put1(0x66),
        LegacyPrefix::PfxF2 => sink.put1(0xF2),
        LegacyPrefix::PfxF3 => sink.put1(0xF3),
        LegacyPrefix::PfxNone => (),
    }

    match memE {
        Addr::ImmReg { simm32, base: regE } => {
            // First, cook up the REX byte.  This is easy.
            let encE = iregEnc(*regE);
            let w = if clearRexW { 0 } else { 1 };
            let r = (encG >> 3) & 1;
            let x = 0;
            let b = (encE >> 3) & 1;
            let rex = 0x40 | (w << 3) | (r << 2) | (x << 1) | b;
            if rex != 0x40 || retainRedundant {
                sink.put1(rex);
            }
            // Now the opcode(s).  These include any other prefixes the caller
            // hands to us.
            while numOpcodes > 0 {
                numOpcodes -= 1;
                sink.put1(((opcodes >> (numOpcodes << 3)) & 0xFF) as u8);
            }
            // Now the mod/rm and associated immediates.  This is
            // significantly complicated due to the multiple special cases.
            if *simm32 == 0
                && encE != regs::ENC_RSP
                && encE != regs::ENC_RBP
                && encE != regs::ENC_R12
                && encE != regs::ENC_R13
            {
                // FIXME JRS 2020Feb11: those four tests can surely be
                // replaced by a single mask-and-compare check.  We should do
                // that because this routine is likely to be hot.
                sink.put1(mkModRegRM(0, encG & 7, encE & 7));
            } else if *simm32 == 0 && (encE == regs::ENC_RSP || encE == regs::ENC_R12) {
                sink.put1(mkModRegRM(0, encG & 7, 4));
                sink.put1(0x24);
            } else if low8willSXto32(*simm32) && encE != regs::ENC_RSP && encE != regs::ENC_R12 {
                sink.put1(mkModRegRM(1, encG & 7, encE & 7));
                sink.put1((simm32 & 0xFF) as u8);
            } else if encE != regs::ENC_RSP && encE != regs::ENC_R12 {
                sink.put1(mkModRegRM(2, encG & 7, encE & 7));
                sink.put4(*simm32);
            } else if (encE == regs::ENC_RSP || encE == regs::ENC_R12) && low8willSXto32(*simm32) {
                // REX.B distinguishes RSP from R12
                sink.put1(mkModRegRM(1, encG & 7, 4));
                sink.put1(0x24);
                sink.put1((simm32 & 0xFF) as u8);
            } else if encE == regs::ENC_R12 || encE == regs::ENC_RSP {
                //.. wait for test case for RSP case
                // REX.B distinguishes RSP from R12
                sink.put1(mkModRegRM(2, encG & 7, 4));
                sink.put1(0x24);
                sink.put4(*simm32);
            } else {
                unreachable!("emit_REX_OPCODES_MODRM_SIB_IMM_encG_memE: ImmReg");
            }
        }

        Addr::ImmRegRegShift {
            simm32,
            base: regBase,
            index: regIndex,
            shift,
        } => {
            let encBase = iregEnc(*regBase);
            let encIndex = iregEnc(*regIndex);
            // The rex byte
            let w = if clearRexW { 0 } else { 1 };
            let r = (encG >> 3) & 1;
            let x = (encIndex >> 3) & 1;
            let b = (encBase >> 3) & 1;
            let rex = 0x40 | (w << 3) | (r << 2) | (x << 1) | b;
            if rex != 0x40 || retainRedundant {
                sink.put1(rex);
            }
            // All other prefixes and opcodes
            while numOpcodes > 0 {
                numOpcodes -= 1;
                sink.put1(((opcodes >> (numOpcodes << 3)) & 0xFF) as u8);
            }
            // modrm, SIB, immediates
            if low8willSXto32(*simm32) && encIndex != regs::ENC_RSP {
                sink.put1(mkModRegRM(1, encG & 7, 4));
                sink.put1(mkSIB(*shift, encIndex & 7, encBase & 7));
                sink.put1(*simm32 as u8);
            } else if encIndex != regs::ENC_RSP {
                sink.put1(mkModRegRM(2, encG & 7, 4));
                sink.put1(mkSIB(*shift, encIndex & 7, encBase & 7));
                sink.put4(*simm32);
            } else {
                panic!("emit_REX_OPCODES_MODRM_SIB_IMM_encG_memE: ImmRegRegShift");
            }
        }
    }
}

/// This is the core 'emit' function for instructions that do not reference
/// memory.
///
/// This is conceptually the same as
/// emit_REX_OPCODES_MODRM_SIB_IMM_encG_memE, except it is for the case
/// where the E operand is a register rather than memory.  Hence it is much
/// simpler.
fn emit_REX_OPCODES_MODRM_encG_encE(
    sink: &mut MachBuffer<Inst>,
    prefix: LegacyPrefix,
    opcodes: u32,
    mut numOpcodes: usize,
    encG: u8,
    encE: u8,
    flags: u32,
) {
    // EncG and EncE can be derived from registers of any class, and they
    // don't even have to be from the same class.  For example, for an
    // integer-to-FP conversion insn, one might be RegClass::I64 and the other
    // RegClass::V128.
    let clearRexW = (flags & F_CLEAR_REX_W) != 0;
    let retainRedundant = (flags & F_RETAIN_REDUNDANT_REX) != 0;

    // The operand-size override
    match prefix {
        LegacyPrefix::Pfx66 => sink.put1(0x66),
        LegacyPrefix::PfxF2 => sink.put1(0xF2),
        LegacyPrefix::PfxF3 => sink.put1(0xF3),
        LegacyPrefix::PfxNone => (),
    }

    // The rex byte
    let w = if clearRexW { 0 } else { 1 };
    let r = (encG >> 3) & 1;
    let x = 0;
    let b = (encE >> 3) & 1;
    let rex = 0x40 | (w << 3) | (r << 2) | (x << 1) | b;

    if rex != 0x40 || retainRedundant {
        sink.put1(rex);
    }

    // All other prefixes and opcodes
    while numOpcodes > 0 {
        numOpcodes -= 1;
        sink.put1(((opcodes >> (numOpcodes << 3)) & 0xFF) as u8);
    }
    // Now the mod/rm byte.  The instruction we're generating doesn't access
    // memory, so there is no SIB byte or immediate -- we're done.
    sink.put1(mkModRegRM(3, encG & 7, encE & 7));
}

// These are merely wrappers for the above two functions that facilitate passing
// actual `Reg`s rather than their encodings.

fn emit_REX_OPCODES_MODRM_SIB_IMM_regG_memE(
    sink: &mut MachBuffer<Inst>,
    prefix: LegacyPrefix,
    opcodes: u32,
    numOpcodes: usize,
    regG: Reg,
    memE: &Addr,
    flags: u32,
) {
    // JRS FIXME 2020Feb07: this should really just be `regEnc` not `iregEnc`
    let encG = iregEnc(regG);
    emit_REX_OPCODES_MODRM_SIB_IMM_encG_memE(sink, prefix, opcodes, numOpcodes, encG, memE, flags);
}

fn emit_REX_OPCODES_MODRM_regG_regE(
    sink: &mut MachBuffer<Inst>,
    prefix: LegacyPrefix,
    opcodes: u32,
    numOpcodes: usize,
    regG: Reg,
    regE: Reg,
    flags: u32,
) {
    // JRS FIXME 2020Feb07: these should really just be `regEnc` not `iregEnc`
    let encG = iregEnc(regG);
    let encE = iregEnc(regE);
    emit_REX_OPCODES_MODRM_encG_encE(sink, prefix, opcodes, numOpcodes, encG, encE, flags);
}

/// Write a suitable number of bits from an imm64 to the sink.
fn emit_simm(sink: &mut MachBuffer<Inst>, size: u8, simm32: u32) {
    match size {
        8 | 4 => sink.put4(simm32),
        2 => sink.put2(simm32 as u16),
        1 => sink.put1(simm32 as u8),
        _ => panic!("x64::Inst::emit_simm: unreachable"),
    }
}

/// The top-level emit function.
///
/// Important!  Do not add improved (shortened) encoding cases to existing
/// instructions without also adding tests for those improved encodings.  That
/// is a dangerous game that leads to hard-to-track-down errors in the emitted
/// code.
///
/// For all instructions, make sure to have test coverage for all of the
/// following situations.  Do this by creating the cross product resulting from
/// applying the following rules to each operand:
///
/// (1) for any insn that mentions a register: one test using a register from
///     the group [rax, rcx, rdx, rbx, rsp, rbp, rsi, rdi] and a second one
///     using a register from the group [r8, r9, r10, r11, r12, r13, r14, r15].
///     This helps detect incorrect REX prefix construction.
///
/// (2) for any insn that mentions a byte register: one test for each of the
///     four encoding groups [al, cl, dl, bl], [spl, bpl, sil, dil],
///     [r8b .. r11b] and [r12b .. r15b].  This checks that
///     apparently-redundant REX prefixes are retained when required.
///
/// (3) for any insn that contains an immediate field, check the following
///     cases: field is zero, field is in simm8 range (-128 .. 127), field is
///     in simm32 range (-0x8000_0000 .. 0x7FFF_FFFF).  This is because some
///     instructions that require a 32-bit immediate have a short-form encoding
///     when the imm is in simm8 range.
///
/// Rules (1), (2) and (3) don't apply for registers within address expressions
/// (`Addr`s).  Those are already pretty well tested, and the registers in them
/// don't have any effect on the containing instruction (apart from possibly
/// require REX prefix bits).
///
/// When choosing registers for a test, avoid using registers with the same
/// offset within a given group.  For example, don't use rax and r8, since they
/// both have the lowest 3 bits as 000, and so the test won't detect errors
/// where those 3-bit register sub-fields are confused by the emitter.  Instead
/// use (eg) rax (lo3 = 000) and r9 (lo3 = 001).  Similarly, don't use (eg) cl
/// and bpl since they have the same offset in their group; use instead (eg) cl
/// and sil.
///
/// For all instructions, also add a test that uses only low-half registers
/// (rax .. rdi, xmm0 .. xmm7) etc, so as to check that any redundant REX
/// prefixes are correctly omitted.  This low-half restriction must apply to
/// _all_ registers in the insn, even those in address expressions.
///
/// Following these rules creates large numbers of test cases, but it's the
/// only way to make the emitter reliable.
///
/// Known possible improvements:
///
/// * there's a shorter encoding for shl/shr/sar by a 1-bit immediate.  (Do we
///   care?)
pub(crate) fn emit(inst: &Inst, sink: &mut MachBuffer<Inst>) {
    match inst {
        Inst::Nop { len: 0 } => {}
        Inst::Alu_RMI_R {
            is_64,
            op,
            src: srcE,
            dst: regG,
        } => {
            let flags = if *is_64 { F_NONE } else { F_CLEAR_REX_W };
            if *op == AluRmiROpcode::Mul {
                // We kinda freeloaded Mul into RMI_R_Op, but it doesn't fit the usual pattern, so
                // we have to special-case it.
                match srcE {
                    RegMemImm::Reg { reg: regE } => {
                        emit_REX_OPCODES_MODRM_regG_regE(
                            sink,
                            LegacyPrefix::PfxNone,
                            0x0FAF,
                            2,
                            regG.to_reg(),
                            *regE,
                            flags,
                        );
                    }

                    RegMemImm::Mem { addr } => {
                        emit_REX_OPCODES_MODRM_SIB_IMM_regG_memE(
                            sink,
                            LegacyPrefix::PfxNone,
                            0x0FAF,
                            2,
                            regG.to_reg(),
                            addr,
                            flags,
                        );
                    }

                    RegMemImm::Imm { simm32 } => {
                        let useImm8 = low8willSXto32(*simm32);
                        let opcode = if useImm8 { 0x6B } else { 0x69 };
                        // Yes, really, regG twice.
                        emit_REX_OPCODES_MODRM_regG_regE(
                            sink,
                            LegacyPrefix::PfxNone,
                            opcode,
                            1,
                            regG.to_reg(),
                            regG.to_reg(),
                            flags,
                        );
                        emit_simm(sink, if useImm8 { 1 } else { 4 }, *simm32);
                    }
                }
            } else {
                let (opcode_R, opcode_M, subopcode_I) = match op {
                    AluRmiROpcode::Add => (0x01, 0x03, 0),
                    AluRmiROpcode::Sub => (0x29, 0x2B, 5),
                    AluRmiROpcode::And => (0x21, 0x23, 4),
                    AluRmiROpcode::Or => (0x09, 0x0B, 1),
                    AluRmiROpcode::Xor => (0x31, 0x33, 6),
                    AluRmiROpcode::Mul => panic!("unreachable"),
                };

                match srcE {
                    RegMemImm::Reg { reg: regE } => {
                        // Note.  The arguments .. regE .. regG .. sequence
                        // here is the opposite of what is expected.  I'm not
                        // sure why this is.  But I am fairly sure that the
                        // arg order could be switched back to the expected
                        // .. regG .. regE .. if opcode_rr is also switched
                        // over to the "other" basic integer opcode (viz, the
                        // R/RM vs RM/R duality).  However, that would mean
                        // that the test results won't be in accordance with
                        // the GNU as reference output.  In other words, the
                        // inversion exists as a result of using GNU as as a
                        // gold standard.
                        emit_REX_OPCODES_MODRM_regG_regE(
                            sink,
                            LegacyPrefix::PfxNone,
                            opcode_R,
                            1,
                            *regE,
                            regG.to_reg(),
                            flags,
                        );
                        // NB: if this is ever extended to handle byte size
                        // ops, be sure to retain redundant REX prefixes.
                    }

                    RegMemImm::Mem { addr } => {
                        // Whereas here we revert to the "normal" G-E ordering.
                        emit_REX_OPCODES_MODRM_SIB_IMM_regG_memE(
                            sink,
                            LegacyPrefix::PfxNone,
                            opcode_M,
                            1,
                            regG.to_reg(),
                            addr,
                            flags,
                        );
                    }

                    RegMemImm::Imm { simm32 } => {
                        let useImm8 = low8willSXto32(*simm32);
                        let opcode = if useImm8 { 0x83 } else { 0x81 };
                        // And also here we use the "normal" G-E ordering.
                        let encG = iregEnc(regG.to_reg());
                        emit_REX_OPCODES_MODRM_encG_encE(
                            sink,
                            LegacyPrefix::PfxNone,
                            opcode,
                            1,
                            subopcode_I,
                            encG,
                            flags,
                        );
                        emit_simm(sink, if useImm8 { 1 } else { 4 }, *simm32);
                    }
                }
            }
        }

        Inst::Imm_R {
            dst_is_64,
            simm64,
            dst,
        } => {
            let encDst = iregEnc(dst.to_reg());
            if *dst_is_64 {
                // FIXME JRS 2020Feb10: also use the 32-bit case here when
                // possible
                sink.put1(0x48 | ((encDst >> 3) & 1));
                sink.put1(0xB8 | (encDst & 7));
                sink.put8(*simm64);
            } else {
                if ((encDst >> 3) & 1) == 1 {
                    sink.put1(0x41);
                }
                sink.put1(0xB8 | (encDst & 7));
                sink.put4(*simm64 as u32);
            }
        }

        Inst::Mov_R_R { is_64, src, dst } => {
            let flags = if *is_64 { F_NONE } else { F_CLEAR_REX_W };
            emit_REX_OPCODES_MODRM_regG_regE(
                sink,
                LegacyPrefix::PfxNone,
                0x89,
                1,
                *src,
                dst.to_reg(),
                flags,
            );
        }

        Inst::MovZX_M_R { extMode, addr, dst } => {
            match extMode {
                ExtMode::BL => {
                    // MOVZBL is (REX.W==0) 0F B6 /r
                    emit_REX_OPCODES_MODRM_SIB_IMM_regG_memE(
                        sink,
                        LegacyPrefix::PfxNone,
                        0x0FB6,
                        2,
                        dst.to_reg(),
                        addr,
                        F_CLEAR_REX_W,
                    )
                }
                ExtMode::BQ => {
                    // MOVZBQ is (REX.W==1) 0F B6 /r
                    // I'm not sure why the Intel manual offers different
                    // encodings for MOVZBQ than for MOVZBL.  AIUI they should
                    // achieve the same, since MOVZBL is just going to zero out
                    // the upper half of the destination anyway.
                    emit_REX_OPCODES_MODRM_SIB_IMM_regG_memE(
                        sink,
                        LegacyPrefix::PfxNone,
                        0x0FB6,
                        2,
                        dst.to_reg(),
                        addr,
                        F_NONE,
                    )
                }
                ExtMode::WL => {
                    // MOVZWL is (REX.W==0) 0F B7 /r
                    emit_REX_OPCODES_MODRM_SIB_IMM_regG_memE(
                        sink,
                        LegacyPrefix::PfxNone,
                        0x0FB7,
                        2,
                        dst.to_reg(),
                        addr,
                        F_CLEAR_REX_W,
                    )
                }
                ExtMode::WQ => {
                    // MOVZWQ is (REX.W==1) 0F B7 /r
                    emit_REX_OPCODES_MODRM_SIB_IMM_regG_memE(
                        sink,
                        LegacyPrefix::PfxNone,
                        0x0FB7,
                        2,
                        dst.to_reg(),
                        addr,
                        F_NONE,
                    )
                }
                ExtMode::LQ => {
                    // This is just a standard 32 bit load, and we rely on the
                    // default zero-extension rule to perform the extension.
                    // MOV r/m32, r32 is (REX.W==0) 8B /r
                    emit_REX_OPCODES_MODRM_SIB_IMM_regG_memE(
                        sink,
                        LegacyPrefix::PfxNone,
                        0x8B,
                        1,
                        dst.to_reg(),
                        addr,
                        F_CLEAR_REX_W,
                    )
                }
            }
        }
        Inst::Mov64_M_R { addr, dst } => emit_REX_OPCODES_MODRM_SIB_IMM_regG_memE(
            sink,
            LegacyPrefix::PfxNone,
            0x8B,
            1,
            dst.to_reg(),
            addr,
            F_NONE,
        ),
        Inst::MovSX_M_R { extMode, addr, dst } => {
            match extMode {
                ExtMode::BL => {
                    // MOVSBL is (REX.W==0) 0F BE /r
                    emit_REX_OPCODES_MODRM_SIB_IMM_regG_memE(
                        sink,
                        LegacyPrefix::PfxNone,
                        0x0FBE,
                        2,
                        dst.to_reg(),
                        addr,
                        F_CLEAR_REX_W,
                    )
                }
                ExtMode::BQ => {
                    // MOVSBQ is (REX.W==1) 0F BE /r
                    emit_REX_OPCODES_MODRM_SIB_IMM_regG_memE(
                        sink,
                        LegacyPrefix::PfxNone,
                        0x0FBE,
                        2,
                        dst.to_reg(),
                        addr,
                        F_NONE,
                    )
                }
                ExtMode::WL => {
                    // MOVSWL is (REX.W==0) 0F BF /r
                    emit_REX_OPCODES_MODRM_SIB_IMM_regG_memE(
                        sink,
                        LegacyPrefix::PfxNone,
                        0x0FBF,
                        2,
                        dst.to_reg(),
                        addr,
                        F_CLEAR_REX_W,
                    )
                }
                ExtMode::WQ => {
                    // MOVSWQ is (REX.W==1) 0F BF /r
                    emit_REX_OPCODES_MODRM_SIB_IMM_regG_memE(
                        sink,
                        LegacyPrefix::PfxNone,
                        0x0FBF,
                        2,
                        dst.to_reg(),
                        addr,
                        F_NONE,
                    )
                }
                ExtMode::LQ => {
                    // MOVSLQ is (REX.W==1) 63 /r
                    emit_REX_OPCODES_MODRM_SIB_IMM_regG_memE(
                        sink,
                        LegacyPrefix::PfxNone,
                        0x63,
                        1,
                        dst.to_reg(),
                        addr,
                        F_NONE,
                    )
                }
            }
        }
        Inst::Mov_R_M { size, src, addr } => {
            match size {
                1 => {
                    // This is one of the few places where the presence of a
                    // redundant REX prefix changes the meaning of the
                    // instruction.
                    let encSrc = iregEnc(*src);
                    let retainRedundantRex = if encSrc >= 4 && encSrc <= 7 {
                        F_RETAIN_REDUNDANT_REX
                    } else {
                        0
                    };
                    // MOV r8, r/m8 is (REX.W==0) 88 /r
                    emit_REX_OPCODES_MODRM_SIB_IMM_regG_memE(
                        sink,
                        LegacyPrefix::PfxNone,
                        0x88,
                        1,
                        *src,
                        addr,
                        F_CLEAR_REX_W | retainRedundantRex,
                    )
                }
                2 => {
                    // MOV r16, r/m16 is 66 (REX.W==0) 89 /r
                    emit_REX_OPCODES_MODRM_SIB_IMM_regG_memE(
                        sink,
                        LegacyPrefix::Pfx66,
                        0x89,
                        1,
                        *src,
                        addr,
                        F_CLEAR_REX_W,
                    )
                }
                4 => {
                    // MOV r32, r/m32 is (REX.W==0) 89 /r
                    emit_REX_OPCODES_MODRM_SIB_IMM_regG_memE(
                        sink,
                        LegacyPrefix::PfxNone,
                        0x89,
                        1,
                        *src,
                        addr,
                        F_CLEAR_REX_W,
                    )
                }
                8 => {
                    // MOV r64, r/m64 is (REX.W==1) 89 /r
                    emit_REX_OPCODES_MODRM_SIB_IMM_regG_memE(
                        sink,
                        LegacyPrefix::PfxNone,
                        0x89,
                        1,
                        *src,
                        addr,
                        F_NONE,
                    )
                }
                _ => panic!("x64::Inst::Mov_R_M::emit: unreachable"),
            }
        }
        Inst::Shift_R {
            is_64,
            kind,
            num_bits,
            dst,
        } => {
            let encDst = iregEnc(dst.to_reg());
            let subopcode = match kind {
                ShiftKind::Left => 4,
                ShiftKind::RightZ => 5,
                ShiftKind::RightS => 7,
            };
            match num_bits {
                None => {
                    // SHL/SHR/SAR %cl, reg32 is (REX.W==0) D3 /subopcode
                    // SHL/SHR/SAR %cl, reg64 is (REX.W==1) D3 /subopcode
                    emit_REX_OPCODES_MODRM_encG_encE(
                        sink,
                        LegacyPrefix::PfxNone,
                        0xD3,
                        1,
                        subopcode,
                        encDst,
                        if *is_64 { F_NONE } else { F_CLEAR_REX_W },
                    );
                }
                Some(num_bits) => {
                    // SHL/SHR/SAR $ib, reg32 is (REX.W==0) C1 /subopcode ib
                    // SHL/SHR/SAR $ib, reg64 is (REX.W==1) C1 /subopcode ib
                    // When the shift amount is 1, there's an even shorter encoding, but we don't
                    // bother with that nicety here.
                    emit_REX_OPCODES_MODRM_encG_encE(
                        sink,
                        LegacyPrefix::PfxNone,
                        0xC1,
                        1,
                        subopcode,
                        encDst,
                        if *is_64 { F_NONE } else { F_CLEAR_REX_W },
                    );
                    sink.put1(*num_bits);
                }
            }
        }
        Inst::Cmp_RMI_R {
            size,
            src: srcE,
            dst: regG,
        } => {
            let mut retainRedundantRex = 0;

            if *size == 1 {
                // Here, a redundant REX prefix changes the meaning of the
                // instruction.
                let encG = iregEnc(*regG);
                if encG >= 4 && encG <= 7 {
                    retainRedundantRex = F_RETAIN_REDUNDANT_REX;
                }
            }

            let mut prefix = LegacyPrefix::PfxNone;
            if *size == 2 {
                prefix = LegacyPrefix::Pfx66;
            }

            let mut flags = match size {
                8 => F_NONE,
                4 | 2 => F_CLEAR_REX_W,
                1 => F_CLEAR_REX_W | retainRedundantRex,
                _ => panic!("x64::Inst::Cmp_RMI_R::emit: unreachable"),
            };

            match srcE {
                RegMemImm::Reg { reg: regE } => {
                    let opcode = if *size == 1 { 0x38 } else { 0x39 };
                    if *size == 1 {
                        // We also need to check whether the E register forces
                        // the use of a redundant REX.
                        let encE = iregEnc(*regE);
                        if encE >= 4 && encE <= 7 {
                            flags |= F_RETAIN_REDUNDANT_REX;
                        }
                    }
                    // Same comment re swapped args as for Alu_RMI_R.
                    emit_REX_OPCODES_MODRM_regG_regE(sink, prefix, opcode, 1, *regE, *regG, flags);
                }

                RegMemImm::Mem { addr } => {
                    let opcode = if *size == 1 { 0x3A } else { 0x3B };
                    // Whereas here we revert to the "normal" G-E ordering.
                    emit_REX_OPCODES_MODRM_SIB_IMM_regG_memE(
                        sink, prefix, opcode, 1, *regG, addr, flags,
                    );
                }

                RegMemImm::Imm { simm32 } => {
                    // FIXME JRS 2020Feb11: there are shorter encodings for
                    // cmp $imm, rax/eax/ax/al.
                    let useImm8 = low8willSXto32(*simm32);
                    let opcode = if *size == 1 {
                        0x80
                    } else if useImm8 {
                        0x83
                    } else {
                        0x81
                    };
                    // And also here we use the "normal" G-E ordering.
                    let encG = iregEnc(*regG);
                    emit_REX_OPCODES_MODRM_encG_encE(
                        sink, prefix, opcode, 1, 7, /*subopcode*/
                        encG, flags,
                    );
                    emit_simm(sink, if useImm8 { 1 } else { *size }, *simm32);
                }
            }
        }

        Inst::Push64 { src } => {
            match src {
                RegMemImm::Reg { reg } => {
                    let encReg = iregEnc(*reg);
                    let rex = 0x40 | ((encReg >> 3) & 1);
                    if rex != 0x40 {
                        sink.put1(rex);
                    }
                    sink.put1(0x50 | (encReg & 7));
                }

                RegMemImm::Mem { addr } => {
                    emit_REX_OPCODES_MODRM_SIB_IMM_encG_memE(
                        sink,
                        LegacyPrefix::PfxNone,
                        0xFF,
                        1,
                        6, /*subopcode*/
                        addr,
                        F_CLEAR_REX_W,
                    );
                }

                RegMemImm::Imm { simm32 } => {
                    if low8willSXto64(*simm32) {
                        sink.put1(0x6A);
                        sink.put1(*simm32 as u8);
                    } else {
                        sink.put1(0x68);
                        sink.put4(*simm32);
                    }
                }
            }
        }

        Inst::Pop64 { dst } => {
            let encDst = iregEnc(dst.to_reg());
            if encDst >= 8 {
                // 0x41 == REX.{W=0, B=1}.  It seems that REX.W is irrelevant
                // here.
                sink.put1(0x41);
            }
            sink.put1(0x58 + (encDst & 7));
        }

        Inst::CallUnknown { dest } => {
            match dest {
                RegMem::Reg { reg } => {
                    let regEnc = iregEnc(*reg);
                    emit_REX_OPCODES_MODRM_encG_encE(
                        sink,
                        LegacyPrefix::PfxNone,
                        0xFF,
                        1,
                        2, /*subopcode*/
                        regEnc,
                        F_CLEAR_REX_W,
                    );
                }

                RegMem::Mem { addr } => {
                    emit_REX_OPCODES_MODRM_SIB_IMM_encG_memE(
                        sink,
                        LegacyPrefix::PfxNone,
                        0xFF,
                        1,
                        2, /*subopcode*/
                        addr,
                        F_CLEAR_REX_W,
                    );
                }
            }
        }

        Inst::Ret {} => sink.put1(0xC3),

        Inst::JmpKnown { dest } => {
            let disp = dest.as_offset32_or_zero() - 5;
            let disp = disp as u32;
            let br_start = sink.cur_offset();
            let br_disp_off = br_start + 1;
            let br_end = br_start + 5;
            if let Some(l) = dest.as_label() {
                sink.use_label_at_offset(br_disp_off, l, LabelUse::Rel32);
                sink.add_uncond_branch(br_start, br_end, l);
            }
            sink.put1(0xE9);
            sink.put4(disp);
        }

        Inst::JmpCondSymm {
            cc,
            taken,
            not_taken,
        } => {
            // Conditional part.

            // This insn is 6 bytes long.  Currently `offset` is relative to
            // the start of this insn, but the Intel encoding requires it to
            // be relative to the start of the next instruction.  Hence the
            // adjustment.
            let taken_disp = taken.as_offset32_or_zero() - 6;
            let taken_disp = taken_disp as u32;
            let cond_start = sink.cur_offset();
            let cond_disp_off = cond_start + 2;
            let cond_end = cond_start + 6;
            if let Some(l) = taken.as_label() {
                sink.use_label_at_offset(cond_disp_off, l, LabelUse::Rel32);
                let inverted: [u8; 6] =
                    [0x0F, 0x80 + (cc.invert().get_enc()), 0xFA, 0xFF, 0xFF, 0xFF];
                sink.add_cond_branch(cond_start, cond_end, l, &inverted[..]);
            }
            sink.put1(0x0F);
            sink.put1(0x80 + cc.get_enc());
            sink.put4(taken_disp);

            // Unconditional part.

            let nt_disp = not_taken.as_offset32_or_zero() - 5;
            let nt_disp = nt_disp as u32;
            let uncond_start = sink.cur_offset();
            let uncond_disp_off = uncond_start + 1;
            let uncond_end = uncond_start + 5;
            if let Some(l) = not_taken.as_label() {
                sink.use_label_at_offset(uncond_disp_off, l, LabelUse::Rel32);
                sink.add_uncond_branch(uncond_start, uncond_end, l);
            }
            sink.put1(0xE9);
            sink.put4(nt_disp);
        }

        Inst::JmpUnknown { target } => {
            match target {
                RegMem::Reg { reg } => {
                    let regEnc = iregEnc(*reg);
                    emit_REX_OPCODES_MODRM_encG_encE(
                        sink,
                        LegacyPrefix::PfxNone,
                        0xFF,
                        1,
                        4, /*subopcode*/
                        regEnc,
                        F_CLEAR_REX_W,
                    );
                }

                RegMem::Mem { addr } => {
                    emit_REX_OPCODES_MODRM_SIB_IMM_encG_memE(
                        sink,
                        LegacyPrefix::PfxNone,
                        0xFF,
                        1,
                        4, /*subopcode*/
                        addr,
                        F_CLEAR_REX_W,
                    );
                }
            }
        }

        Inst::XMM_R_R { op, src, dst } => {
            let flags = F_CLEAR_REX_W;
            let opcode = match op {
                SseOpcode::Movss => 0x0F10,
                SseOpcode::Movsd => 0x0F10,
                _ => unimplemented!("XMM_R_R opcode"),
            };

            let prefix = match op {
                SseOpcode::Movss => LegacyPrefix::PfxF3,
                SseOpcode::Movsd => LegacyPrefix::PfxF2,
                _ => unimplemented!("XMM_R_R opcode"),
            };

            emit_REX_OPCODES_MODRM_regG_regE(sink, prefix, opcode, 2, dst.to_reg(), *src, flags);
        }

        Inst::XMM_RM_R {
            op,
            src: srcE,
            dst: regG,
        } => {
            let flags = F_CLEAR_REX_W;
            let opcode = match op {
                SseOpcode::Addss => 0x0F58,
                SseOpcode::Subss => 0x0F5C,
                _ => unimplemented!("XMM_RM_R opcode"),
            };

            match srcE {
                RegMem::Reg { reg: regE } => {
                    emit_REX_OPCODES_MODRM_regG_regE(
                        sink,
                        LegacyPrefix::PfxF3,
                        opcode,
                        2,
                        regG.to_reg(),
                        *regE,
                        flags,
                    );
                }

                RegMem::Mem { addr } => {
                    emit_REX_OPCODES_MODRM_SIB_IMM_regG_memE(
                        sink,
                        LegacyPrefix::PfxF3,
                        opcode,
                        2,
                        regG.to_reg(),
                        addr,
                        flags,
                    );
                }
            }
        }

        _ => panic!("x64_emit: unhandled: {} ", inst.show_rru(None)),
    }
}
