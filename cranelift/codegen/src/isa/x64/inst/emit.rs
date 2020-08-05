use crate::binemit::Reloc;
use crate::ir::immediates::{Ieee32, Ieee64};
use crate::ir::{types, TrapCode};
use crate::isa::x64::inst::args::*;
use crate::isa::x64::inst::*;
use crate::machinst::{MachBuffer, MachInstEmit, MachLabel};
use core::convert::TryInto;
use log::debug;
use regalloc::{Reg, RegClass, Writable};
use std::convert::TryFrom;

fn low8_will_sign_extend_to_64(x: u32) -> bool {
    let xs = (x as i32) as i64;
    xs == ((xs << 56) >> 56)
}

fn low8_will_sign_extend_to_32(x: u32) -> bool {
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
fn encode_modrm(m0d: u8, enc_reg_g: u8, rm_e: u8) -> u8 {
    debug_assert!(m0d < 4);
    debug_assert!(enc_reg_g < 8);
    debug_assert!(rm_e < 8);
    ((m0d & 3) << 6) | ((enc_reg_g & 7) << 3) | (rm_e & 7)
}

#[inline(always)]
fn encode_sib(shift: u8, enc_index: u8, enc_base: u8) -> u8 {
    debug_assert!(shift < 4);
    debug_assert!(enc_index < 8);
    debug_assert!(enc_base < 8);
    ((shift & 3) << 6) | ((enc_index & 7) << 3) | (enc_base & 7)
}

/// Get the encoding number of a GPR.
#[inline(always)]
fn int_reg_enc(reg: Reg) -> u8 {
    debug_assert!(reg.is_real());
    debug_assert_eq!(reg.get_class(), RegClass::I64);
    reg.get_hw_encoding()
}

/// Get the encoding number of any register.
#[inline(always)]
fn reg_enc(reg: Reg) -> u8 {
    debug_assert!(reg.is_real());
    reg.get_hw_encoding()
}

/// A small bit field to record a REX prefix specification:
/// - bit 0 set to 1 indicates REX.W must be 0 (cleared).
/// - bit 1 set to 1 indicates the REX prefix must always be emitted.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct RexFlags(u8);

impl RexFlags {
    /// By default, set the W field, and don't always emit.
    #[inline(always)]
    fn set_w() -> Self {
        Self(0)
    }
    /// Creates a new RexPrefix for which the REX.W bit will be cleared.
    #[inline(always)]
    fn clear_w() -> Self {
        Self(1)
    }

    #[inline(always)]
    fn always_emit(&mut self) -> &mut Self {
        self.0 = self.0 | 2;
        self
    }

    #[inline(always)]
    fn must_clear_w(&self) -> bool {
        (self.0 & 1) != 0
    }
    #[inline(always)]
    fn must_always_emit(&self) -> bool {
        (self.0 & 2) != 0
    }

    #[inline(always)]
    fn emit_two_op(&self, sink: &mut MachBuffer<Inst>, enc_g: u8, enc_e: u8) {
        let w = if self.must_clear_w() { 0 } else { 1 };
        let r = (enc_g >> 3) & 1;
        let x = 0;
        let b = (enc_e >> 3) & 1;
        let rex = 0x40 | (w << 3) | (r << 2) | (x << 1) | b;
        if rex != 0x40 || self.must_always_emit() {
            sink.put1(rex);
        }
    }

    #[inline(always)]
    fn emit_three_op(&self, sink: &mut MachBuffer<Inst>, enc_g: u8, enc_index: u8, enc_base: u8) {
        let w = if self.must_clear_w() { 0 } else { 1 };
        let r = (enc_g >> 3) & 1;
        let x = (enc_index >> 3) & 1;
        let b = (enc_base >> 3) & 1;
        let rex = 0x40 | (w << 3) | (r << 2) | (x << 1) | b;
        if rex != 0x40 || self.must_always_emit() {
            sink.put1(rex);
        }
    }
}

/// For specifying the legacy prefixes (or `None` if no prefix required) to
/// be used at the start an instruction. A given prefix may be required for
/// various operations, including instructions that operate on GPR, SSE, and Vex
/// registers.
enum LegacyPrefix {
    None,
    _66,
    _F2,
    _F3,
}

impl LegacyPrefix {
    #[inline(always)]
    fn emit(&self, sink: &mut MachBuffer<Inst>) {
        match self {
            LegacyPrefix::_66 => sink.put1(0x66),
            LegacyPrefix::_F2 => sink.put1(0xF2),
            LegacyPrefix::_F3 => sink.put1(0xF3),
            LegacyPrefix::None => (),
        }
    }
}

/// This is the core 'emit' function for instructions that reference memory.
///
/// For an instruction that has as operands a reg encoding `enc_g` and a memory address `mem_e`,
/// create and emit:
/// - first the REX prefix,
/// - then caller-supplied opcode byte(s) (`opcodes` and `num_opcodes`),
/// - then the MOD/RM byte,
/// - then optionally, a SIB byte,
/// - and finally optionally an immediate that will be derived from the `mem_e` operand.
///
/// For most instructions up to and including SSE4.2, that will be the whole instruction: this is
/// what we call "standard" instructions, and abbreviate "std" in the name here. VEX instructions
/// will require their own emitter functions.
///
/// This will also work for 32-bits x86 instructions, assuming no REX prefix is provided.
///
/// The opcodes are written bigendianly for the convenience of callers.  For example, if the opcode
/// bytes to be emitted are, in this order, F3 0F 27, then the caller should pass `opcodes` ==
/// 0xF3_0F_27 and `num_opcodes` == 3.
///
/// The register operand is represented here not as a `Reg` but as its hardware encoding, `enc_g`.
/// `rex` can specify special handling for the REX prefix.  By default, the REX prefix will
/// indicate a 64-bit operation and will be deleted if it is redundant (0x40).  Note that for a
/// 64-bit operation, the REX prefix will normally never be redundant, since REX.W must be 1 to
/// indicate a 64-bit operation.
fn emit_std_enc_mem(
    sink: &mut MachBuffer<Inst>,
    prefix: LegacyPrefix,
    opcodes: u32,
    mut num_opcodes: usize,
    enc_g: u8,
    mem_e: &Amode,
    rex: RexFlags,
) {
    // General comment for this function: the registers in `mem_e` must be
    // 64-bit integer registers, because they are part of an address
    // expression.  But `enc_g` can be derived from a register of any class.

    prefix.emit(sink);

    match mem_e {
        Amode::ImmReg { simm32, base } => {
            // First, the REX byte.
            let enc_e = int_reg_enc(*base);
            rex.emit_two_op(sink, enc_g, enc_e);

            // Now the opcode(s).  These include any other prefixes the caller
            // hands to us.
            while num_opcodes > 0 {
                num_opcodes -= 1;
                sink.put1(((opcodes >> (num_opcodes << 3)) & 0xFF) as u8);
            }

            // Now the mod/rm and associated immediates.  This is
            // significantly complicated due to the multiple special cases.
            if *simm32 == 0
                && enc_e != regs::ENC_RSP
                && enc_e != regs::ENC_RBP
                && enc_e != regs::ENC_R12
                && enc_e != regs::ENC_R13
            {
                // FIXME JRS 2020Feb11: those four tests can surely be
                // replaced by a single mask-and-compare check.  We should do
                // that because this routine is likely to be hot.
                sink.put1(encode_modrm(0, enc_g & 7, enc_e & 7));
            } else if *simm32 == 0 && (enc_e == regs::ENC_RSP || enc_e == regs::ENC_R12) {
                sink.put1(encode_modrm(0, enc_g & 7, 4));
                sink.put1(0x24);
            } else if low8_will_sign_extend_to_32(*simm32)
                && enc_e != regs::ENC_RSP
                && enc_e != regs::ENC_R12
            {
                sink.put1(encode_modrm(1, enc_g & 7, enc_e & 7));
                sink.put1((simm32 & 0xFF) as u8);
            } else if enc_e != regs::ENC_RSP && enc_e != regs::ENC_R12 {
                sink.put1(encode_modrm(2, enc_g & 7, enc_e & 7));
                sink.put4(*simm32);
            } else if (enc_e == regs::ENC_RSP || enc_e == regs::ENC_R12)
                && low8_will_sign_extend_to_32(*simm32)
            {
                // REX.B distinguishes RSP from R12
                sink.put1(encode_modrm(1, enc_g & 7, 4));
                sink.put1(0x24);
                sink.put1((simm32 & 0xFF) as u8);
            } else if enc_e == regs::ENC_R12 || enc_e == regs::ENC_RSP {
                //.. wait for test case for RSP case
                // REX.B distinguishes RSP from R12
                sink.put1(encode_modrm(2, enc_g & 7, 4));
                sink.put1(0x24);
                sink.put4(*simm32);
            } else {
                unreachable!("ImmReg");
            }
        }

        Amode::ImmRegRegShift {
            simm32,
            base: reg_base,
            index: reg_index,
            shift,
        } => {
            let enc_base = int_reg_enc(*reg_base);
            let enc_index = int_reg_enc(*reg_index);

            // The rex byte.
            rex.emit_three_op(sink, enc_g, enc_index, enc_base);

            // All other prefixes and opcodes.
            while num_opcodes > 0 {
                num_opcodes -= 1;
                sink.put1(((opcodes >> (num_opcodes << 3)) & 0xFF) as u8);
            }

            // modrm, SIB, immediates.
            if low8_will_sign_extend_to_32(*simm32) && enc_index != regs::ENC_RSP {
                sink.put1(encode_modrm(1, enc_g & 7, 4));
                sink.put1(encode_sib(*shift, enc_index & 7, enc_base & 7));
                sink.put1(*simm32 as u8);
            } else if enc_index != regs::ENC_RSP {
                sink.put1(encode_modrm(2, enc_g & 7, 4));
                sink.put1(encode_sib(*shift, enc_index & 7, enc_base & 7));
                sink.put4(*simm32);
            } else {
                panic!("ImmRegRegShift");
            }
        }

        Amode::RipRelative { ref target } => {
            // First, the REX byte, with REX.B = 0.
            rex.emit_two_op(sink, enc_g, 0);

            // Now the opcode(s).  These include any other prefixes the caller
            // hands to us.
            while num_opcodes > 0 {
                num_opcodes -= 1;
                sink.put1(((opcodes >> (num_opcodes << 3)) & 0xFF) as u8);
            }

            // RIP-relative is mod=00, rm=101.
            sink.put1(encode_modrm(0, enc_g & 7, 0b101));

            match *target {
                BranchTarget::Label(label) => {
                    let offset = sink.cur_offset();
                    sink.use_label_at_offset(offset, label, LabelUse::JmpRel32);
                    sink.put4(0);
                }
                BranchTarget::ResolvedOffset(offset) => {
                    let offset =
                        u32::try_from(offset).expect("rip-relative can't hold >= U32_MAX values");
                    sink.put4(offset);
                }
            }
        }
    }
}

/// This is the core 'emit' function for instructions that do not reference memory.
///
/// This is conceptually the same as emit_modrm_sib_enc_ge, except it is for the case where the E
/// operand is a register rather than memory.  Hence it is much simpler.
fn emit_std_enc_enc(
    sink: &mut MachBuffer<Inst>,
    prefix: LegacyPrefix,
    opcodes: u32,
    mut num_opcodes: usize,
    enc_g: u8,
    enc_e: u8,
    rex: RexFlags,
) {
    // EncG and EncE can be derived from registers of any class, and they
    // don't even have to be from the same class.  For example, for an
    // integer-to-FP conversion insn, one might be RegClass::I64 and the other
    // RegClass::V128.

    // The operand-size override.
    prefix.emit(sink);

    // The rex byte.
    rex.emit_two_op(sink, enc_g, enc_e);

    // All other prefixes and opcodes.
    while num_opcodes > 0 {
        num_opcodes -= 1;
        sink.put1(((opcodes >> (num_opcodes << 3)) & 0xFF) as u8);
    }

    // Now the mod/rm byte.  The instruction we're generating doesn't access
    // memory, so there is no SIB byte or immediate -- we're done.
    sink.put1(encode_modrm(3, enc_g & 7, enc_e & 7));
}

// These are merely wrappers for the above two functions that facilitate passing
// actual `Reg`s rather than their encodings.

fn emit_std_reg_mem(
    sink: &mut MachBuffer<Inst>,
    prefix: LegacyPrefix,
    opcodes: u32,
    num_opcodes: usize,
    reg_g: Reg,
    mem_e: &Amode,
    rex: RexFlags,
) {
    let enc_g = reg_enc(reg_g);
    emit_std_enc_mem(sink, prefix, opcodes, num_opcodes, enc_g, mem_e, rex);
}

fn emit_std_reg_reg(
    sink: &mut MachBuffer<Inst>,
    prefix: LegacyPrefix,
    opcodes: u32,
    num_opcodes: usize,
    reg_g: Reg,
    reg_e: Reg,
    rex: RexFlags,
) {
    let enc_g = reg_enc(reg_g);
    let enc_e = reg_enc(reg_e);
    emit_std_enc_enc(sink, prefix, opcodes, num_opcodes, enc_g, enc_e, rex);
}

/// Write a suitable number of bits from an imm64 to the sink.
fn emit_simm(sink: &mut MachBuffer<Inst>, size: u8, simm32: u32) {
    match size {
        8 | 4 => sink.put4(simm32),
        2 => sink.put2(simm32 as u16),
        1 => sink.put1(simm32 as u8),
        _ => unreachable!(),
    }
}

/// A small helper to generate a signed conversion instruction.
fn emit_signed_cvt(
    sink: &mut MachBuffer<Inst>,
    flags: &settings::Flags,
    state: &mut EmitState,
    src: Reg,
    dst: Writable<Reg>,
    to_f64: bool,
) {
    // Handle an unsigned int, which is the "easy" case: a signed conversion will do the
    // right thing.
    let op = if to_f64 {
        SseOpcode::Cvtsi2sd
    } else {
        SseOpcode::Cvtsi2ss
    };
    let inst = Inst::gpr_to_xmm(op, RegMem::reg(src), OperandSize::Size64, dst);
    inst.emit(sink, flags, state);
}

/// Emits a one way conditional jump if CC is set (true).
fn one_way_jmp(sink: &mut MachBuffer<Inst>, cc: CC, label: MachLabel) {
    let cond_start = sink.cur_offset();
    let cond_disp_off = cond_start + 2;
    sink.use_label_at_offset(cond_disp_off, label, LabelUse::JmpRel32);
    sink.put1(0x0F);
    sink.put1(0x80 + cc.get_enc());
    sink.put4(0x0);
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
pub(crate) fn emit(
    inst: &Inst,
    sink: &mut MachBuffer<Inst>,
    flags: &settings::Flags,
    state: &mut EmitState,
) {
    match inst {
        Inst::Alu_RMI_R {
            is_64,
            op,
            src,
            dst: reg_g,
        } => {
            let rex = if *is_64 {
                RexFlags::set_w()
            } else {
                RexFlags::clear_w()
            };

            if *op == AluRmiROpcode::Mul {
                // We kinda freeloaded Mul into RMI_R_Op, but it doesn't fit the usual pattern, so
                // we have to special-case it.
                match src {
                    RegMemImm::Reg { reg: reg_e } => {
                        emit_std_reg_reg(
                            sink,
                            LegacyPrefix::None,
                            0x0FAF,
                            2,
                            reg_g.to_reg(),
                            *reg_e,
                            rex,
                        );
                    }

                    RegMemImm::Mem { addr } => {
                        emit_std_reg_mem(
                            sink,
                            LegacyPrefix::None,
                            0x0FAF,
                            2,
                            reg_g.to_reg(),
                            &addr.finalize(state),
                            rex,
                        );
                    }

                    RegMemImm::Imm { simm32 } => {
                        let useImm8 = low8_will_sign_extend_to_32(*simm32);
                        let opcode = if useImm8 { 0x6B } else { 0x69 };
                        // Yes, really, reg_g twice.
                        emit_std_reg_reg(
                            sink,
                            LegacyPrefix::None,
                            opcode,
                            1,
                            reg_g.to_reg(),
                            reg_g.to_reg(),
                            rex,
                        );
                        emit_simm(sink, if useImm8 { 1 } else { 4 }, *simm32);
                    }
                }
            } else {
                let (opcode_r, opcode_m, subopcode_i) = match op {
                    AluRmiROpcode::Add => (0x01, 0x03, 0),
                    AluRmiROpcode::Sub => (0x29, 0x2B, 5),
                    AluRmiROpcode::And => (0x21, 0x23, 4),
                    AluRmiROpcode::Or => (0x09, 0x0B, 1),
                    AluRmiROpcode::Xor => (0x31, 0x33, 6),
                    AluRmiROpcode::Mul => panic!("unreachable"),
                };

                match src {
                    RegMemImm::Reg { reg: reg_e } => {
                        // GCC/llvm use the swapped operand encoding (viz., the R/RM vs RM/R
                        // duality). Do this too, so as to be able to compare generated machine
                        // code easily.
                        emit_std_reg_reg(
                            sink,
                            LegacyPrefix::None,
                            opcode_r,
                            1,
                            *reg_e,
                            reg_g.to_reg(),
                            rex,
                        );
                        // NB: if this is ever extended to handle byte size ops, be sure to retain
                        // redundant REX prefixes.
                    }

                    RegMemImm::Mem { addr } => {
                        // Here we revert to the "normal" G-E ordering.
                        emit_std_reg_mem(
                            sink,
                            LegacyPrefix::None,
                            opcode_m,
                            1,
                            reg_g.to_reg(),
                            &addr.finalize(state),
                            rex,
                        );
                    }

                    RegMemImm::Imm { simm32 } => {
                        let use_imm8 = low8_will_sign_extend_to_32(*simm32);
                        let opcode = if use_imm8 { 0x83 } else { 0x81 };
                        // And also here we use the "normal" G-E ordering.
                        let enc_g = int_reg_enc(reg_g.to_reg());
                        emit_std_enc_enc(
                            sink,
                            LegacyPrefix::None,
                            opcode,
                            1,
                            subopcode_i,
                            enc_g,
                            rex,
                        );
                        emit_simm(sink, if use_imm8 { 1 } else { 4 }, *simm32);
                    }
                }
            }
        }

        Inst::UnaryRmR { size, op, src, dst } => {
            let (prefix, rex_flags) = match size {
                2 => (LegacyPrefix::_66, RexFlags::clear_w()),
                4 => (LegacyPrefix::None, RexFlags::clear_w()),
                8 => (LegacyPrefix::None, RexFlags::set_w()),
                _ => unreachable!(),
            };

            let (opcode, num_opcodes) = match op {
                UnaryRmROpcode::Bsr => (0x0fbd, 2),
                UnaryRmROpcode::Bsf => (0x0fbc, 2),
            };

            match src {
                RegMem::Reg { reg: src } => emit_std_reg_reg(
                    sink,
                    prefix,
                    opcode,
                    num_opcodes,
                    dst.to_reg(),
                    *src,
                    rex_flags,
                ),
                RegMem::Mem { addr: src } => emit_std_reg_mem(
                    sink,
                    prefix,
                    opcode,
                    num_opcodes,
                    dst.to_reg(),
                    &src.finalize(state),
                    rex_flags,
                ),
            }
        }

        Inst::Div {
            size,
            signed,
            divisor,
            loc,
        } => {
            let (prefix, rex_flags) = match size {
                2 => (LegacyPrefix::_66, RexFlags::clear_w()),
                4 => (LegacyPrefix::None, RexFlags::clear_w()),
                8 => (LegacyPrefix::None, RexFlags::set_w()),
                _ => unreachable!(),
            };

            sink.add_trap(*loc, TrapCode::IntegerDivisionByZero);

            let subopcode = if *signed { 7 } else { 6 };
            match divisor {
                RegMem::Reg { reg } => {
                    let src = int_reg_enc(*reg);
                    emit_std_enc_enc(sink, prefix, 0xF7, 1, subopcode, src, rex_flags)
                }
                RegMem::Mem { addr: src } => emit_std_enc_mem(
                    sink,
                    prefix,
                    0xF7,
                    1,
                    subopcode,
                    &src.finalize(state),
                    rex_flags,
                ),
            }
        }

        Inst::MulHi { size, signed, rhs } => {
            let (prefix, rex_flags) = match size {
                2 => (LegacyPrefix::_66, RexFlags::clear_w()),
                4 => (LegacyPrefix::None, RexFlags::clear_w()),
                8 => (LegacyPrefix::None, RexFlags::set_w()),
                _ => unreachable!(),
            };

            let subopcode = if *signed { 5 } else { 4 };
            match rhs {
                RegMem::Reg { reg } => {
                    let src = int_reg_enc(*reg);
                    emit_std_enc_enc(sink, prefix, 0xF7, 1, subopcode, src, rex_flags)
                }
                RegMem::Mem { addr: src } => emit_std_enc_mem(
                    sink,
                    prefix,
                    0xF7,
                    1,
                    subopcode,
                    &src.finalize(state),
                    rex_flags,
                ),
            }
        }

        Inst::SignExtendRaxRdx { size } => {
            match size {
                2 => sink.put1(0x66),
                4 => {}
                8 => sink.put1(0x48),
                _ => unreachable!(),
            }
            sink.put1(0x99);
        }

        Inst::CheckedDivOrRemSeq {
            kind,
            size,
            divisor,
            loc,
            tmp,
        } => {
            // Generates the following code sequence:
            //
            // ;; check divide by zero:
            // cmp 0 %divisor
            // jnz $after_trap
            // ud2
            // $after_trap:
            //
            // ;; for signed modulo/div:
            // cmp -1 %divisor
            // jnz $do_op
            // ;;   for signed modulo, result is 0
            //    mov #0, %rdx
            //    j $done
            // ;;   for signed div, check for integer overflow against INT_MIN of the right size
            // cmp INT_MIN, %rax
            // jnz $do_op
            // ud2
            //
            // $do_op:
            // ;; if signed
            //     cdq ;; sign-extend from rax into rdx
            // ;; else
            //     mov #0, %rdx
            // idiv %divisor
            //
            // $done:
            debug_assert!(flags.avoid_div_traps());

            // Check if the divisor is zero, first.
            let inst = Inst::cmp_rmi_r(*size, RegMemImm::imm(0), divisor.to_reg());
            inst.emit(sink, flags, state);

            let inst = Inst::trap_if(CC::Z, TrapCode::IntegerDivisionByZero, *loc);
            inst.emit(sink, flags, state);

            let (do_op, done_label) = if kind.is_signed() {
                // Now check if the divisor is -1.
                let inst = Inst::cmp_rmi_r(*size, RegMemImm::imm(0xffffffff), divisor.to_reg());
                inst.emit(sink, flags, state);

                let do_op = sink.get_label();

                // If not equal, jump to do-op.
                one_way_jmp(sink, CC::NZ, do_op);

                // Here, divisor == -1.
                if !kind.is_div() {
                    // x % -1 = 0; put the result into the destination, $rdx.
                    let done_label = sink.get_label();

                    let inst = Inst::imm_r(*size == 8, 0, Writable::from_reg(regs::rdx()));
                    inst.emit(sink, flags, state);

                    let inst = Inst::jmp_known(BranchTarget::Label(done_label));
                    inst.emit(sink, flags, state);

                    (Some(do_op), Some(done_label))
                } else {
                    // Check for integer overflow.
                    if *size == 8 {
                        let tmp = tmp.expect("temporary for i64 sdiv");

                        let inst = Inst::imm_r(true, 0x8000000000000000, tmp);
                        inst.emit(sink, flags, state);

                        let inst = Inst::cmp_rmi_r(8, RegMemImm::reg(tmp.to_reg()), regs::rax());
                        inst.emit(sink, flags, state);
                    } else {
                        let inst = Inst::cmp_rmi_r(*size, RegMemImm::imm(0x80000000), regs::rax());
                        inst.emit(sink, flags, state);
                    }

                    // If not equal, jump over the trap.
                    let inst = Inst::trap_if(CC::Z, TrapCode::IntegerOverflow, *loc);
                    inst.emit(sink, flags, state);

                    (Some(do_op), None)
                }
            } else {
                (None, None)
            };

            if let Some(do_op) = do_op {
                sink.bind_label(do_op);
            }

            // Fill in the high parts:
            if kind.is_signed() {
                // sign-extend the sign-bit of rax into rdx, for signed opcodes.
                let inst = Inst::sign_extend_rax_to_rdx(*size);
                inst.emit(sink, flags, state);
            } else {
                // zero for unsigned opcodes.
                let inst = Inst::imm_r(true /* is_64 */, 0, Writable::from_reg(regs::rdx()));
                inst.emit(sink, flags, state);
            }

            let inst = Inst::div(*size, kind.is_signed(), RegMem::reg(divisor.to_reg()), *loc);
            inst.emit(sink, flags, state);

            // Lowering takes care of moving the result back into the right register, see comment
            // there.

            if let Some(done) = done_label {
                sink.bind_label(done);
            }
        }

        Inst::Imm_R {
            dst_is_64,
            simm64,
            dst,
        } => {
            let enc_dst = int_reg_enc(dst.to_reg());
            if *dst_is_64 {
                // FIXME JRS 2020Feb10: also use the 32-bit case here when
                // possible
                sink.put1(0x48 | ((enc_dst >> 3) & 1));
                sink.put1(0xB8 | (enc_dst & 7));
                sink.put8(*simm64);
            } else {
                if ((enc_dst >> 3) & 1) == 1 {
                    sink.put1(0x41);
                }
                sink.put1(0xB8 | (enc_dst & 7));
                sink.put4(*simm64 as u32);
            }
        }

        Inst::Mov_R_R { is_64, src, dst } => {
            let rex = if *is_64 {
                RexFlags::set_w()
            } else {
                RexFlags::clear_w()
            };
            emit_std_reg_reg(sink, LegacyPrefix::None, 0x89, 1, *src, dst.to_reg(), rex);
        }

        Inst::MovZX_RM_R {
            ext_mode,
            src,
            dst,
            srcloc,
        } => {
            let (opcodes, num_opcodes, mut rex_flags) = match ext_mode {
                ExtMode::BL => {
                    // MOVZBL is (REX.W==0) 0F B6 /r
                    (0x0FB6, 2, RexFlags::clear_w())
                }
                ExtMode::BQ => {
                    // MOVZBQ is (REX.W==1) 0F B6 /r
                    // I'm not sure why the Intel manual offers different
                    // encodings for MOVZBQ than for MOVZBL.  AIUI they should
                    // achieve the same, since MOVZBL is just going to zero out
                    // the upper half of the destination anyway.
                    (0x0FB6, 2, RexFlags::set_w())
                }
                ExtMode::WL => {
                    // MOVZWL is (REX.W==0) 0F B7 /r
                    (0x0FB7, 2, RexFlags::clear_w())
                }
                ExtMode::WQ => {
                    // MOVZWQ is (REX.W==1) 0F B7 /r
                    (0x0FB7, 2, RexFlags::set_w())
                }
                ExtMode::LQ => {
                    // This is just a standard 32 bit load, and we rely on the
                    // default zero-extension rule to perform the extension.
                    // Note that in reg/reg mode, gcc seems to use the swapped form R/RM, which we
                    // don't do here, since it's the same encoding size.
                    // MOV r/m32, r32 is (REX.W==0) 8B /r
                    (0x8B, 1, RexFlags::clear_w())
                }
            };

            match src {
                RegMem::Reg { reg: src } => {
                    match ext_mode {
                        ExtMode::BL | ExtMode::BQ => {
                            // A redundant REX prefix must be emitted for certain register inputs.
                            let enc_src = int_reg_enc(*src);
                            if enc_src >= 4 && enc_src <= 7 {
                                rex_flags.always_emit();
                            };
                        }
                        _ => {}
                    }
                    emit_std_reg_reg(
                        sink,
                        LegacyPrefix::None,
                        opcodes,
                        num_opcodes,
                        dst.to_reg(),
                        *src,
                        rex_flags,
                    )
                }

                RegMem::Mem { addr: src } => {
                    let src = &src.finalize(state);

                    if let Some(srcloc) = *srcloc {
                        // Register the offset at which the actual load instruction starts.
                        sink.add_trap(srcloc, TrapCode::HeapOutOfBounds);
                    }

                    emit_std_reg_mem(
                        sink,
                        LegacyPrefix::None,
                        opcodes,
                        num_opcodes,
                        dst.to_reg(),
                        src,
                        rex_flags,
                    )
                }
            }
        }

        Inst::Mov64_M_R { src, dst, srcloc } => {
            let src = &src.finalize(state);

            if let Some(srcloc) = *srcloc {
                // Register the offset at which the actual load instruction starts.
                sink.add_trap(srcloc, TrapCode::HeapOutOfBounds);
            }

            emit_std_reg_mem(
                sink,
                LegacyPrefix::None,
                0x8B,
                1,
                dst.to_reg(),
                src,
                RexFlags::set_w(),
            )
        }

        Inst::LoadEffectiveAddress { addr, dst } => emit_std_reg_mem(
            sink,
            LegacyPrefix::None,
            0x8D,
            1,
            dst.to_reg(),
            &addr.finalize(state),
            RexFlags::set_w(),
        ),

        Inst::MovSX_RM_R {
            ext_mode,
            src,
            dst,
            srcloc,
        } => {
            let (opcodes, num_opcodes, mut rex_flags) = match ext_mode {
                ExtMode::BL => {
                    // MOVSBL is (REX.W==0) 0F BE /r
                    (0x0FBE, 2, RexFlags::clear_w())
                }
                ExtMode::BQ => {
                    // MOVSBQ is (REX.W==1) 0F BE /r
                    (0x0FBE, 2, RexFlags::set_w())
                }
                ExtMode::WL => {
                    // MOVSWL is (REX.W==0) 0F BF /r
                    (0x0FBF, 2, RexFlags::clear_w())
                }
                ExtMode::WQ => {
                    // MOVSWQ is (REX.W==1) 0F BF /r
                    (0x0FBF, 2, RexFlags::set_w())
                }
                ExtMode::LQ => {
                    // MOVSLQ is (REX.W==1) 63 /r
                    (0x63, 1, RexFlags::set_w())
                }
            };

            match src {
                RegMem::Reg { reg: src } => {
                    match ext_mode {
                        ExtMode::BL | ExtMode::BQ => {
                            // A redundant REX prefix must be emitted for certain register inputs.
                            let enc_src = int_reg_enc(*src);
                            if enc_src >= 4 && enc_src <= 7 {
                                rex_flags.always_emit();
                            };
                        }
                        _ => {}
                    }
                    emit_std_reg_reg(
                        sink,
                        LegacyPrefix::None,
                        opcodes,
                        num_opcodes,
                        dst.to_reg(),
                        *src,
                        rex_flags,
                    )
                }

                RegMem::Mem { addr: src } => {
                    let src = &src.finalize(state);

                    if let Some(srcloc) = *srcloc {
                        // Register the offset at which the actual load instruction starts.
                        sink.add_trap(srcloc, TrapCode::HeapOutOfBounds);
                    }

                    emit_std_reg_mem(
                        sink,
                        LegacyPrefix::None,
                        opcodes,
                        num_opcodes,
                        dst.to_reg(),
                        src,
                        rex_flags,
                    )
                }
            }
        }

        Inst::Mov_R_M {
            size,
            src,
            dst,
            srcloc,
        } => {
            let dst = &dst.finalize(state);

            if let Some(srcloc) = *srcloc {
                // Register the offset at which the actual load instruction starts.
                sink.add_trap(srcloc, TrapCode::HeapOutOfBounds);
            }

            match size {
                1 => {
                    // This is one of the few places where the presence of a
                    // redundant REX prefix changes the meaning of the
                    // instruction.
                    let mut rex = RexFlags::clear_w();

                    let enc_src = int_reg_enc(*src);
                    if enc_src >= 4 && enc_src <= 7 {
                        rex.always_emit();
                    };

                    // MOV r8, r/m8 is (REX.W==0) 88 /r
                    emit_std_reg_mem(sink, LegacyPrefix::None, 0x88, 1, *src, dst, rex)
                }

                2 => {
                    // MOV r16, r/m16 is 66 (REX.W==0) 89 /r
                    emit_std_reg_mem(
                        sink,
                        LegacyPrefix::_66,
                        0x89,
                        1,
                        *src,
                        dst,
                        RexFlags::clear_w(),
                    )
                }

                4 => {
                    // MOV r32, r/m32 is (REX.W==0) 89 /r
                    emit_std_reg_mem(
                        sink,
                        LegacyPrefix::None,
                        0x89,
                        1,
                        *src,
                        dst,
                        RexFlags::clear_w(),
                    )
                }

                8 => {
                    // MOV r64, r/m64 is (REX.W==1) 89 /r
                    emit_std_reg_mem(
                        sink,
                        LegacyPrefix::None,
                        0x89,
                        1,
                        *src,
                        dst,
                        RexFlags::set_w(),
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
            let enc_dst = int_reg_enc(dst.to_reg());
            let subopcode = match kind {
                ShiftKind::RotateLeft => 0,
                ShiftKind::RotateRight => 1,
                ShiftKind::ShiftLeft => 4,
                ShiftKind::ShiftRightLogical => 5,
                ShiftKind::ShiftRightArithmetic => 7,
            };

            let rex = if *is_64 {
                RexFlags::set_w()
            } else {
                RexFlags::clear_w()
            };

            match num_bits {
                None => {
                    // SHL/SHR/SAR %cl, reg32 is (REX.W==0) D3 /subopcode
                    // SHL/SHR/SAR %cl, reg64 is (REX.W==1) D3 /subopcode
                    emit_std_enc_enc(sink, LegacyPrefix::None, 0xD3, 1, subopcode, enc_dst, rex);
                }

                Some(num_bits) => {
                    // SHL/SHR/SAR $ib, reg32 is (REX.W==0) C1 /subopcode ib
                    // SHL/SHR/SAR $ib, reg64 is (REX.W==1) C1 /subopcode ib
                    // When the shift amount is 1, there's an even shorter encoding, but we don't
                    // bother with that nicety here.
                    emit_std_enc_enc(sink, LegacyPrefix::None, 0xC1, 1, subopcode, enc_dst, rex);
                    sink.put1(*num_bits);
                }
            }
        }

        Inst::XmmRmiReg { opcode, src, dst } => {
            let rex = RexFlags::clear_w();
            let prefix = LegacyPrefix::_66;
            if let RegMemImm::Imm { simm32 } = src {
                let (opcode_bytes, reg_digit) = match opcode {
                    SseOpcode::Psllw => (0x0F71, 6),
                    SseOpcode::Pslld => (0x0F72, 6),
                    SseOpcode::Psllq => (0x0F73, 6),
                    SseOpcode::Psraw => (0x0F71, 4),
                    SseOpcode::Psrad => (0x0F72, 4),
                    SseOpcode::Psrlw => (0x0F71, 2),
                    SseOpcode::Psrld => (0x0F72, 2),
                    SseOpcode::Psrlq => (0x0F73, 2),
                    _ => panic!("invalid opcode: {}", opcode),
                };
                let dst_enc = reg_enc(dst.to_reg());
                emit_std_enc_enc(sink, prefix, opcode_bytes, 2, reg_digit, dst_enc, rex);
                let imm = (*simm32)
                    .try_into()
                    .expect("the immediate must be convertible to a u8");
                sink.put1(imm);
            } else {
                let opcode_bytes = match opcode {
                    SseOpcode::Psllw => 0x0FF1,
                    SseOpcode::Pslld => 0x0FF2,
                    SseOpcode::Psllq => 0x0FF3,
                    SseOpcode::Psraw => 0x0FE1,
                    SseOpcode::Psrad => 0x0FE2,
                    SseOpcode::Psrlw => 0x0FD1,
                    SseOpcode::Psrld => 0x0FD2,
                    SseOpcode::Psrlq => 0x0FD3,
                    _ => panic!("invalid opcode: {}", opcode),
                };

                match src {
                    RegMemImm::Reg { reg } => {
                        emit_std_reg_reg(sink, prefix, opcode_bytes, 2, dst.to_reg(), *reg, rex);
                    }
                    RegMemImm::Mem { addr } => {
                        let addr = &addr.finalize(state);
                        emit_std_reg_mem(sink, prefix, opcode_bytes, 2, dst.to_reg(), addr, rex);
                    }
                    RegMemImm::Imm { .. } => unreachable!(),
                }
            };
        }

        Inst::Cmp_RMI_R {
            size,
            src: src_e,
            dst: reg_g,
        } => {
            let mut prefix = LegacyPrefix::None;
            if *size == 2 {
                prefix = LegacyPrefix::_66;
            }

            let mut rex = match size {
                8 => RexFlags::set_w(),
                4 | 2 => RexFlags::clear_w(),
                1 => {
                    let mut rex = RexFlags::clear_w();
                    // Here, a redundant REX prefix changes the meaning of the instruction.
                    let enc_g = int_reg_enc(*reg_g);
                    if enc_g >= 4 && enc_g <= 7 {
                        rex.always_emit();
                    }
                    rex
                }
                _ => panic!("x64::Inst::Cmp_RMI_R::emit: unreachable"),
            };

            match src_e {
                RegMemImm::Reg { reg: reg_e } => {
                    if *size == 1 {
                        // Check whether the E register forces the use of a redundant REX.
                        let enc_e = int_reg_enc(*reg_e);
                        if enc_e >= 4 && enc_e <= 7 {
                            rex.always_emit();
                        }
                    }

                    // Use the swapped operands encoding, to stay consistent with the output of
                    // gcc/llvm.
                    let opcode = if *size == 1 { 0x38 } else { 0x39 };
                    emit_std_reg_reg(sink, prefix, opcode, 1, *reg_e, *reg_g, rex);
                }

                RegMemImm::Mem { addr } => {
                    let addr = &addr.finalize(state);
                    // Whereas here we revert to the "normal" G-E ordering.
                    let opcode = if *size == 1 { 0x3A } else { 0x3B };
                    emit_std_reg_mem(sink, prefix, opcode, 1, *reg_g, addr, rex);
                }

                RegMemImm::Imm { simm32 } => {
                    // FIXME JRS 2020Feb11: there are shorter encodings for
                    // cmp $imm, rax/eax/ax/al.
                    let use_imm8 = low8_will_sign_extend_to_32(*simm32);

                    // And also here we use the "normal" G-E ordering.
                    let opcode = if *size == 1 {
                        0x80
                    } else if use_imm8 {
                        0x83
                    } else {
                        0x81
                    };

                    let enc_g = int_reg_enc(*reg_g);
                    emit_std_enc_enc(sink, prefix, opcode, 1, 7 /*subopcode*/, enc_g, rex);
                    emit_simm(sink, if use_imm8 { 1 } else { *size }, *simm32);
                }
            }
        }

        Inst::Setcc { cc, dst } => {
            let opcode = 0x0f90 + cc.get_enc() as u32;
            let mut rex_flags = RexFlags::clear_w();
            rex_flags.always_emit();
            emit_std_enc_enc(
                sink,
                LegacyPrefix::None,
                opcode,
                2,
                0,
                reg_enc(dst.to_reg()),
                rex_flags,
            );
        }

        Inst::Cmove {
            size,
            cc,
            src,
            dst: reg_g,
        } => {
            let (prefix, rex_flags) = match size {
                2 => (LegacyPrefix::_66, RexFlags::clear_w()),
                4 => (LegacyPrefix::None, RexFlags::clear_w()),
                8 => (LegacyPrefix::None, RexFlags::set_w()),
                _ => unreachable!("invalid size spec for cmove"),
            };
            let opcode = 0x0F40 + cc.get_enc() as u32;
            match src {
                RegMem::Reg { reg: reg_e } => {
                    emit_std_reg_reg(sink, prefix, opcode, 2, reg_g.to_reg(), *reg_e, rex_flags);
                }
                RegMem::Mem { addr } => {
                    let addr = &addr.finalize(state);
                    emit_std_reg_mem(sink, prefix, opcode, 2, reg_g.to_reg(), addr, rex_flags);
                }
            }
        }

        Inst::XmmCmove {
            is_64,
            cc,
            src,
            dst,
        } => {
            let next = sink.get_label();

            // Jump if cc is *not* set.
            one_way_jmp(sink, cc.invert(), next);

            let op = if *is_64 {
                SseOpcode::Movsd
            } else {
                SseOpcode::Movss
            };
            let inst = Inst::xmm_unary_rm_r(op, src.clone(), *dst);
            inst.emit(sink, flags, state);

            sink.bind_label(next);
        }

        Inst::Push64 { src } => {
            match src {
                RegMemImm::Reg { reg } => {
                    let enc_reg = int_reg_enc(*reg);
                    let rex = 0x40 | ((enc_reg >> 3) & 1);
                    if rex != 0x40 {
                        sink.put1(rex);
                    }
                    sink.put1(0x50 | (enc_reg & 7));
                }

                RegMemImm::Mem { addr } => {
                    let addr = &addr.finalize(state);
                    emit_std_enc_mem(
                        sink,
                        LegacyPrefix::None,
                        0xFF,
                        1,
                        6, /*subopcode*/
                        addr,
                        RexFlags::clear_w(),
                    );
                }

                RegMemImm::Imm { simm32 } => {
                    if low8_will_sign_extend_to_64(*simm32) {
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
            let encDst = int_reg_enc(dst.to_reg());
            if encDst >= 8 {
                // 0x41 == REX.{W=0, B=1}.  It seems that REX.W is irrelevant
                // here.
                sink.put1(0x41);
            }
            sink.put1(0x58 + (encDst & 7));
        }

        Inst::CallKnown {
            dest, loc, opcode, ..
        } => {
            if let Some(s) = state.take_stackmap() {
                sink.add_stackmap(StackmapExtent::UpcomingBytes(5), s);
            }
            sink.put1(0xE8);
            // The addend adjusts for the difference between the end of the instruction and the
            // beginning of the immediate field.
            sink.add_reloc(*loc, Reloc::X86CallPCRel4, &dest, -4);
            sink.put4(0);
            if opcode.is_call() {
                sink.add_call_site(*loc, *opcode);
            }
        }

        Inst::CallUnknown {
            dest, opcode, loc, ..
        } => {
            let start_offset = sink.cur_offset();
            match dest {
                RegMem::Reg { reg } => {
                    let reg_enc = int_reg_enc(*reg);
                    emit_std_enc_enc(
                        sink,
                        LegacyPrefix::None,
                        0xFF,
                        1,
                        2, /*subopcode*/
                        reg_enc,
                        RexFlags::clear_w(),
                    );
                }

                RegMem::Mem { addr } => {
                    let addr = &addr.finalize(state);
                    emit_std_enc_mem(
                        sink,
                        LegacyPrefix::None,
                        0xFF,
                        1,
                        2, /*subopcode*/
                        addr,
                        RexFlags::clear_w(),
                    );
                }
            }
            if let Some(s) = state.take_stackmap() {
                sink.add_stackmap(StackmapExtent::StartedAtOffset(start_offset), s);
            }
            if opcode.is_call() {
                sink.add_call_site(*loc, *opcode);
            }
        }

        Inst::Ret {} => sink.put1(0xC3),

        Inst::JmpKnown { dst } => {
            let br_start = sink.cur_offset();
            let br_disp_off = br_start + 1;
            let br_end = br_start + 5;
            if let Some(l) = dst.as_label() {
                sink.use_label_at_offset(br_disp_off, l, LabelUse::JmpRel32);
                sink.add_uncond_branch(br_start, br_end, l);
            }

            let disp = dst.as_offset32_or_zero();
            let disp = disp as u32;
            sink.put1(0xE9);
            sink.put4(disp);
        }

        Inst::JmpCond {
            cc,
            taken,
            not_taken,
        } => {
            // If taken.
            let cond_start = sink.cur_offset();
            let cond_disp_off = cond_start + 2;
            let cond_end = cond_start + 6;
            if let Some(l) = taken.as_label() {
                sink.use_label_at_offset(cond_disp_off, l, LabelUse::JmpRel32);
                let inverted: [u8; 6] =
                    [0x0F, 0x80 + (cc.invert().get_enc()), 0x00, 0x00, 0x00, 0x00];
                sink.add_cond_branch(cond_start, cond_end, l, &inverted[..]);
            }

            let taken_disp = taken.as_offset32_or_zero();
            let taken_disp = taken_disp as u32;
            sink.put1(0x0F);
            sink.put1(0x80 + cc.get_enc());
            sink.put4(taken_disp);

            // If not taken.
            let uncond_start = sink.cur_offset();
            let uncond_disp_off = uncond_start + 1;
            let uncond_end = uncond_start + 5;
            if let Some(l) = not_taken.as_label() {
                sink.use_label_at_offset(uncond_disp_off, l, LabelUse::JmpRel32);
                sink.add_uncond_branch(uncond_start, uncond_end, l);
            }

            let nt_disp = not_taken.as_offset32_or_zero();
            let nt_disp = nt_disp as u32;
            sink.put1(0xE9);
            sink.put4(nt_disp);
        }

        Inst::JmpUnknown { target } => {
            match target {
                RegMem::Reg { reg } => {
                    let reg_enc = int_reg_enc(*reg);
                    emit_std_enc_enc(
                        sink,
                        LegacyPrefix::None,
                        0xFF,
                        1,
                        4, /*subopcode*/
                        reg_enc,
                        RexFlags::clear_w(),
                    );
                }

                RegMem::Mem { addr } => {
                    let addr = &addr.finalize(state);
                    emit_std_enc_mem(
                        sink,
                        LegacyPrefix::None,
                        0xFF,
                        1,
                        4, /*subopcode*/
                        addr,
                        RexFlags::clear_w(),
                    );
                }
            }
        }

        Inst::JmpTableSeq {
            idx,
            tmp1,
            tmp2,
            ref targets,
            default_target,
            ..
        } => {
            // This sequence is *one* instruction in the vcode, and is expanded only here at
            // emission time, because we cannot allow the regalloc to insert spills/reloads in
            // the middle; we depend on hardcoded PC-rel addressing below.
            //
            // We don't have to worry about emitting islands, because the only label-use type has a
            // maximum range of 2 GB. If we later consider using shorter-range label references,
            // this will need to be revisited.

            // Save index in a tmp (the live range of ridx only goes to start of this
            // sequence; rtmp1 or rtmp2 may overwrite it).

            // We generate the following sequence:
            // ;; generated by lowering: cmp #jmp_table_size, %idx
            // jnb $default_target
            // movl %idx, %tmp2
            // lea start_of_jump_table_offset(%rip), %tmp1
            // movslq [%tmp1, %tmp2, 4], %tmp2 ;; shift of 2, viz. multiply index by 4
            // addq %tmp2, %tmp1
            // j *%tmp1
            // $start_of_jump_table:
            // -- jump table entries
            let default_label = match default_target {
                BranchTarget::Label(label) => label,
                _ => unreachable!(),
            };
            one_way_jmp(sink, CC::NB, *default_label); // idx unsigned >= jmp table size

            // Copy the index (and make sure to clear the high 32-bits lane of tmp2).
            let inst = Inst::movzx_rm_r(ExtMode::LQ, RegMem::reg(*idx), *tmp2, None);
            inst.emit(sink, flags, state);

            // Load base address of jump table.
            let start_of_jumptable = sink.get_label();
            let inst = Inst::lea(
                Amode::rip_relative(BranchTarget::Label(start_of_jumptable)),
                *tmp1,
            );
            inst.emit(sink, flags, state);

            // Load value out of the jump table. It's a relative offset to the target block, so it
            // might be negative; use a sign-extension.
            let inst = Inst::movsx_rm_r(
                ExtMode::LQ,
                RegMem::mem(Amode::imm_reg_reg_shift(0, tmp1.to_reg(), tmp2.to_reg(), 2)),
                *tmp2,
                None,
            );
            inst.emit(sink, flags, state);

            // Add base of jump table to jump-table-sourced block offset.
            let inst = Inst::alu_rmi_r(
                true, /* is_64 */
                AluRmiROpcode::Add,
                RegMemImm::reg(tmp2.to_reg()),
                *tmp1,
            );
            inst.emit(sink, flags, state);

            // Branch to computed address.
            let inst = Inst::jmp_unknown(RegMem::reg(tmp1.to_reg()));
            inst.emit(sink, flags, state);

            // Emit jump table (table of 32-bit offsets).
            sink.bind_label(start_of_jumptable);
            let jt_off = sink.cur_offset();
            for &target in targets.iter() {
                let word_off = sink.cur_offset();
                // off_into_table is an addend here embedded in the label to be later patched at
                // the end of codegen. The offset is initially relative to this jump table entry;
                // with the extra addend, it'll be relative to the jump table's start, after
                // patching.
                let off_into_table = word_off - jt_off;
                sink.use_label_at_offset(word_off, target.as_label().unwrap(), LabelUse::PCRel32);
                sink.put4(off_into_table);
            }
        }

        Inst::TrapIf {
            cc,
            trap_code,
            srcloc,
        } => {
            let else_label = sink.get_label();

            // Jump over if the invert of CC is set (i.e. CC is not set).
            one_way_jmp(sink, cc.invert(), else_label);

            // Trap!
            let inst = Inst::trap(*srcloc, *trap_code);
            inst.emit(sink, flags, state);

            sink.bind_label(else_label);
        }

        Inst::XmmUnaryRmR {
            op,
            src: src_e,
            dst: reg_g,
            srcloc,
        } => {
            let rex = RexFlags::clear_w();

            let (prefix, opcode) = match op {
                SseOpcode::Movaps => (LegacyPrefix::None, 0x0F28),
                SseOpcode::Movapd => (LegacyPrefix::_66, 0x0F28),
                SseOpcode::Movsd => (LegacyPrefix::_F2, 0x0F10),
                SseOpcode::Movss => (LegacyPrefix::_F3, 0x0F10),
                SseOpcode::Movups => (LegacyPrefix::None, 0x0F10),
                SseOpcode::Movupd => (LegacyPrefix::_66, 0x0F10),
                SseOpcode::Sqrtps => (LegacyPrefix::None, 0x0F51),
                SseOpcode::Sqrtpd => (LegacyPrefix::_66, 0x0F51),
                SseOpcode::Sqrtss => (LegacyPrefix::_F3, 0x0F51),
                SseOpcode::Sqrtsd => (LegacyPrefix::_F2, 0x0F51),
                SseOpcode::Cvtss2sd => (LegacyPrefix::_F3, 0x0F5A),
                SseOpcode::Cvtsd2ss => (LegacyPrefix::_F2, 0x0F5A),
                _ => unimplemented!("Opcode {:?} not implemented", op),
            };

            match src_e {
                RegMem::Reg { reg: reg_e } => {
                    emit_std_reg_reg(sink, prefix, opcode, 2, reg_g.to_reg(), *reg_e, rex);
                }
                RegMem::Mem { addr } => {
                    let addr = &addr.finalize(state);
                    if let Some(srcloc) = *srcloc {
                        // Register the offset at which the actual load instruction starts.
                        sink.add_trap(srcloc, TrapCode::HeapOutOfBounds);
                    }
                    emit_std_reg_mem(sink, prefix, opcode, 2, reg_g.to_reg(), addr, rex);
                }
            };
        }

        Inst::XMM_RM_R {
            op,
            src: src_e,
            dst: reg_g,
        } => {
            let rex = RexFlags::clear_w();
            let (prefix, opcode) = match op {
                SseOpcode::Addps => (LegacyPrefix::None, 0x0F58),
                SseOpcode::Addpd => (LegacyPrefix::_66, 0x0F58),
                SseOpcode::Addss => (LegacyPrefix::_F3, 0x0F58),
                SseOpcode::Addsd => (LegacyPrefix::_F2, 0x0F58),
                SseOpcode::Andpd => (LegacyPrefix::_66, 0x0F54),
                SseOpcode::Andps => (LegacyPrefix::None, 0x0F54),
                SseOpcode::Andnps => (LegacyPrefix::None, 0x0F55),
                SseOpcode::Andnpd => (LegacyPrefix::_66, 0x0F55),
                SseOpcode::Divps => (LegacyPrefix::None, 0x0F5E),
                SseOpcode::Divpd => (LegacyPrefix::_66, 0x0F5E),
                SseOpcode::Divss => (LegacyPrefix::_F3, 0x0F5E),
                SseOpcode::Divsd => (LegacyPrefix::_F2, 0x0F5E),
                SseOpcode::Minps => (LegacyPrefix::None, 0x0F5D),
                SseOpcode::Minpd => (LegacyPrefix::_66, 0x0F5D),
                SseOpcode::Minss => (LegacyPrefix::_F3, 0x0F5D),
                SseOpcode::Minsd => (LegacyPrefix::_F2, 0x0F5D),
                SseOpcode::Maxps => (LegacyPrefix::None, 0x0F5F),
                SseOpcode::Maxpd => (LegacyPrefix::_66, 0x0F5F),
                SseOpcode::Maxss => (LegacyPrefix::_F3, 0x0F5F),
                SseOpcode::Maxsd => (LegacyPrefix::_F2, 0x0F5F),
                SseOpcode::Mulps => (LegacyPrefix::None, 0x0F59),
                SseOpcode::Mulpd => (LegacyPrefix::_66, 0x0F59),
                SseOpcode::Mulss => (LegacyPrefix::_F3, 0x0F59),
                SseOpcode::Mulsd => (LegacyPrefix::_F2, 0x0F59),
                SseOpcode::Orpd => (LegacyPrefix::_66, 0x0F56),
                SseOpcode::Orps => (LegacyPrefix::None, 0x0F56),
                SseOpcode::Paddb => (LegacyPrefix::_66, 0x0FFC),
                SseOpcode::Paddd => (LegacyPrefix::_66, 0x0FFE),
                SseOpcode::Paddq => (LegacyPrefix::_66, 0x0FD4),
                SSeOpcode::Paddw => (LegacyPrefix::_66, 0x0FFD),
                SseOpcode::Subps => (LegacyPrefix::None, 0x0F5C),
                SseOpcode::Subpd => (LegacyPrefix::_66, 0x0F5C),
                SseOpcode::Subss => (LegacyPrefix::_F3, 0x0F5C),
                SseOpcode::Subsd => (LegacyPrefix::_F2, 0x0F5C),
                SseOpcode::Xorps => (LegacyPrefix::None, 0x0F57),
                SseOpcode::Xorpd => (LegacyPrefix::_66, 0x0F57),
                _ => unimplemented!("Opcode {:?} not implemented", op),
            };

            match src_e {
                RegMem::Reg { reg: reg_e } => {
                    emit_std_reg_reg(sink, prefix, opcode, 2, reg_g.to_reg(), *reg_e, rex);
                }
                RegMem::Mem { addr } => {
                    let addr = &addr.finalize(state);
                    emit_std_reg_mem(sink, prefix, opcode, 2, reg_g.to_reg(), addr, rex);
                }
            }
        }

        Inst::XmmMinMaxSeq {
            size,
            is_min,
            lhs,
            rhs_dst,
        } => {
            // Generates the following sequence:
            // cmpss/cmpsd %lhs, %rhs_dst
            // jnz do_min_max
            // jp propagate_nan
            //
            // ;; ordered and equal: propagate the sign bit (for -0 vs 0):
            // {and,or}{ss,sd} %lhs, %rhs_dst
            // j done
            //
            // ;; to get the desired NaN behavior (signalling NaN transformed into a quiet NaN, the
            // ;; NaN value is returned), we add both inputs.
            // propagate_nan:
            // add{ss,sd} %lhs, %rhs_dst
            // j done
            //
            // do_min_max:
            // {min,max}{ss,sd} %lhs, %rhs_dst
            //
            // done:
            let done = sink.get_label();
            let propagate_nan = sink.get_label();
            let do_min_max = sink.get_label();

            let (add_op, cmp_op, and_op, or_op, min_max_op) = match size {
                OperandSize::Size32 => (
                    SseOpcode::Addss,
                    SseOpcode::Ucomiss,
                    SseOpcode::Andps,
                    SseOpcode::Orps,
                    if *is_min {
                        SseOpcode::Minss
                    } else {
                        SseOpcode::Maxss
                    },
                ),
                OperandSize::Size64 => (
                    SseOpcode::Addsd,
                    SseOpcode::Ucomisd,
                    SseOpcode::Andpd,
                    SseOpcode::Orpd,
                    if *is_min {
                        SseOpcode::Minsd
                    } else {
                        SseOpcode::Maxsd
                    },
                ),
            };

            let inst = Inst::xmm_cmp_rm_r(cmp_op, RegMem::reg(*lhs), rhs_dst.to_reg());
            inst.emit(sink, flags, state);

            one_way_jmp(sink, CC::NZ, do_min_max);
            one_way_jmp(sink, CC::P, propagate_nan);

            // Ordered and equal. The operands are bit-identical unless they are zero
            // and negative zero. These instructions merge the sign bits in that
            // case, and are no-ops otherwise.
            let op = if *is_min { or_op } else { and_op };
            let inst = Inst::xmm_rm_r(op, RegMem::reg(*lhs), *rhs_dst);
            inst.emit(sink, flags, state);

            let inst = Inst::jmp_known(BranchTarget::Label(done));
            inst.emit(sink, flags, state);

            // x86's min/max are not symmetric; if either operand is a NaN, they return the
            // read-only operand: perform an addition between the two operands, which has the
            // desired NaN propagation effects.
            sink.bind_label(propagate_nan);
            let inst = Inst::xmm_rm_r(add_op, RegMem::reg(*lhs), *rhs_dst);
            inst.emit(sink, flags, state);

            one_way_jmp(sink, CC::P, done);

            sink.bind_label(do_min_max);
            let inst = Inst::xmm_rm_r(min_max_op, RegMem::reg(*lhs), *rhs_dst);
            inst.emit(sink, flags, state);

            sink.bind_label(done);
        }

        Inst::XmmRmRImm { op, src, dst, imm } => {
            let prefix = match op {
                SseOpcode::Cmpps => LegacyPrefix::None,
                SseOpcode::Cmppd => LegacyPrefix::_66,
                SseOpcode::Cmpss => LegacyPrefix::_F3,
                SseOpcode::Cmpsd => LegacyPrefix::_F2,
                _ => unimplemented!("Opcode {:?} not implemented", op),
            };
            let opcode = 0x0FC2;
            let rex = RexFlags::clear_w();
            match src {
                RegMem::Reg { reg } => {
                    emit_std_reg_reg(sink, prefix, opcode, 2, dst.to_reg(), *reg, rex);
                }
                RegMem::Mem { addr } => {
                    let addr = &addr.finalize(state);
                    emit_std_reg_mem(sink, prefix, opcode, 2, dst.to_reg(), addr, rex);
                }
            }
            sink.put1(*imm)
        }

        Inst::XmmLoadConstSeq { val, dst, ty } => {
            // This sequence is *one* instruction in the vcode, and is expanded only here at
            // emission time, because we cannot allow the regalloc to insert spills/reloads in
            // the middle; we depend on hardcoded PC-rel addressing below. TODO Eventually this
            // "constant inline" code should be replaced by constant pool integration.

            // Load the inline constant.
            let opcode = match *ty {
                types::F32X4 => SseOpcode::Movups,
                types::F64X2 => SseOpcode::Movupd,
                types::I8X16 => SseOpcode::Movupd, // TODO replace with MOVDQU
                _ => unimplemented!("cannot yet load constants for type: {}", ty),
            };
            let constant_start_label = sink.get_label();
            let load_offset = RegMem::mem(Amode::rip_relative(BranchTarget::Label(
                constant_start_label,
            )));
            let load = Inst::xmm_unary_rm_r(opcode, load_offset, *dst);
            load.emit(sink, flags, state);

            // Jump over the constant.
            let constant_end_label = sink.get_label();
            let continue_at_offset = BranchTarget::Label(constant_end_label);
            let jump = Inst::jmp_known(continue_at_offset);
            jump.emit(sink, flags, state);

            // Emit the constant.
            sink.bind_label(constant_start_label);
            for i in val.iter() {
                sink.put1(*i);
            }
            sink.bind_label(constant_end_label);
        }

        Inst::Xmm_Mov_R_M {
            op,
            src,
            dst,
            srcloc,
        } => {
            let (prefix, opcode) = match op {
                SseOpcode::Movss => (LegacyPrefix::_F3, 0x0F11),
                SseOpcode::Movsd => (LegacyPrefix::_F2, 0x0F11),
                SseOpcode::Movaps => (LegacyPrefix::None, 0x0F29),
                SseOpcode::Movups => (LegacyPrefix::None, 0x0F11),
                _ => unimplemented!("Opcode {:?} not implemented", op),
            };
            let dst = &dst.finalize(state);
            if let Some(srcloc) = *srcloc {
                // Register the offset at which the actual load instruction starts.
                sink.add_trap(srcloc, TrapCode::HeapOutOfBounds);
            }
            emit_std_reg_mem(sink, prefix, opcode, 2, *src, dst, RexFlags::clear_w());
        }

        Inst::XmmToGpr {
            op,
            src,
            dst,
            dst_size,
        } => {
            let (prefix, opcode, dst_first) = match op {
                // Movd and movq use the same opcode; the presence of the REX prefix (set below)
                // actually determines which is used.
                SseOpcode::Movd | SseOpcode::Movq => (LegacyPrefix::_66, 0x0F7E, false),
                SseOpcode::Cvttss2si => (LegacyPrefix::_F3, 0x0F2C, true),
                SseOpcode::Cvttsd2si => (LegacyPrefix::_F2, 0x0F2C, true),
                _ => panic!("unexpected opcode {:?}", op),
            };
            let rex = match dst_size {
                OperandSize::Size32 => RexFlags::clear_w(),
                OperandSize::Size64 => RexFlags::set_w(),
            };

            let (src, dst) = if dst_first {
                (dst.to_reg(), *src)
            } else {
                (*src, dst.to_reg())
            };

            emit_std_reg_reg(sink, prefix, opcode, 2, src, dst, rex);
        }

        Inst::GprToXmm {
            op,
            src: src_e,
            dst: reg_g,
            src_size,
        } => {
            let (prefix, opcode) = match op {
                // Movd and movq use the same opcode; the presence of the REX prefix (set below)
                // actually determines which is used.
                SseOpcode::Movd | SseOpcode::Movq => (LegacyPrefix::_66, 0x0F6E),
                SseOpcode::Cvtsi2ss => (LegacyPrefix::_F3, 0x0F2A),
                SseOpcode::Cvtsi2sd => (LegacyPrefix::_F2, 0x0F2A),
                _ => panic!("unexpected opcode {:?}", op),
            };
            let rex = match *src_size {
                OperandSize::Size32 => RexFlags::clear_w(),
                OperandSize::Size64 => RexFlags::set_w(),
            };
            match src_e {
                RegMem::Reg { reg: reg_e } => {
                    emit_std_reg_reg(sink, prefix, opcode, 2, reg_g.to_reg(), *reg_e, rex);
                }
                RegMem::Mem { addr } => {
                    let addr = &addr.finalize(state);
                    emit_std_reg_mem(sink, prefix, opcode, 2, reg_g.to_reg(), addr, rex);
                }
            }
        }

        Inst::XMM_Cmp_RM_R { op, src, dst } => {
            let rex = RexFlags::clear_w();
            let (prefix, opcode) = match op {
                SseOpcode::Ucomisd => (LegacyPrefix::_66, 0x0F2E),
                SseOpcode::Ucomiss => (LegacyPrefix::None, 0x0F2E),
                _ => unimplemented!("Emit xmm cmp rm r"),
            };

            match src {
                RegMem::Reg { reg } => {
                    emit_std_reg_reg(sink, prefix, opcode, 2, *dst, *reg, rex);
                }
                RegMem::Mem { addr } => {
                    let addr = &addr.finalize(state);
                    emit_std_reg_mem(sink, prefix, opcode, 2, *dst, addr, rex);
                }
            }
        }

        Inst::CvtUint64ToFloatSeq {
            to_f64,
            src,
            dst,
            tmp_gpr1,
            tmp_gpr2,
        } => {
            // Note: this sequence is specific to 64-bit mode; a 32-bit mode would require a
            // different sequence.
            //
            // Emit the following sequence:
            //
            //  cmp 0, %src
            //  jl handle_negative
            //
            //  ;; handle positive, which can't overflow
            //  cvtsi2sd/cvtsi2ss %src, %dst
            //  j done
            //
            //  ;; handle negative: see below for an explanation of what it's doing.
            //  handle_negative:
            //  mov %src, %tmp_gpr1
            //  shr $1, %tmp_gpr1
            //  mov %src, %tmp_gpr2
            //  and $1, %tmp_gpr2
            //  or %tmp_gpr1, %tmp_gpr2
            //  cvtsi2sd/cvtsi2ss %tmp_gpr2, %dst
            //  addsd/addss %dst, %dst
            //
            //  done:

            assert_ne!(src, tmp_gpr1);
            assert_ne!(src, tmp_gpr2);
            assert_ne!(tmp_gpr1, tmp_gpr2);

            let handle_negative = sink.get_label();
            let done = sink.get_label();

            // If x seen as a signed int64 is not negative, a signed-conversion will do the right
            // thing.
            // TODO use tst src, src here.
            let inst = Inst::cmp_rmi_r(8, RegMemImm::imm(0), src.to_reg());
            inst.emit(sink, flags, state);

            one_way_jmp(sink, CC::L, handle_negative);

            // Handle a positive int64, which is the "easy" case: a signed conversion will do the
            // right thing.
            emit_signed_cvt(sink, flags, state, src.to_reg(), *dst, *to_f64);

            let inst = Inst::jmp_known(BranchTarget::Label(done));
            inst.emit(sink, flags, state);

            sink.bind_label(handle_negative);

            // Divide x by two to get it in range for the signed conversion, keep the LSB, and
            // scale it back up on the FP side.
            let inst = Inst::gen_move(*tmp_gpr1, src.to_reg(), types::I64);
            inst.emit(sink, flags, state);

            // tmp_gpr1 := src >> 1
            let inst = Inst::shift_r(
                /*is_64*/ true,
                ShiftKind::ShiftRightLogical,
                Some(1),
                *tmp_gpr1,
            );
            inst.emit(sink, flags, state);

            let inst = Inst::gen_move(*tmp_gpr2, src.to_reg(), types::I64);
            inst.emit(sink, flags, state);

            let inst = Inst::alu_rmi_r(
                true, /* 64bits */
                AluRmiROpcode::And,
                RegMemImm::imm(1),
                *tmp_gpr2,
            );
            inst.emit(sink, flags, state);

            let inst = Inst::alu_rmi_r(
                true, /* 64bits */
                AluRmiROpcode::Or,
                RegMemImm::reg(tmp_gpr1.to_reg()),
                *tmp_gpr2,
            );
            inst.emit(sink, flags, state);

            emit_signed_cvt(sink, flags, state, tmp_gpr2.to_reg(), *dst, *to_f64);

            let add_op = if *to_f64 {
                SseOpcode::Addsd
            } else {
                SseOpcode::Addss
            };
            let inst = Inst::xmm_rm_r(add_op, RegMem::reg(dst.to_reg()), *dst);
            inst.emit(sink, flags, state);

            sink.bind_label(done);
        }

        Inst::CvtFloatToSintSeq {
            src_size,
            dst_size,
            is_saturating,
            src,
            dst,
            tmp_gpr,
            tmp_xmm,
            srcloc,
        } => {
            // Emits the following common sequence:
            //
            // cvttss2si/cvttsd2si %src, %dst
            // cmp %dst, 1
            // jno done
            //
            // Then, for saturating conversions:
            //
            // ;; check for NaN
            // cmpss/cmpsd %src, %src
            // jnp not_nan
            // xor %dst, %dst
            //
            // ;; positive inputs get saturated to INT_MAX; negative ones to INT_MIN, which is
            // ;; already in %dst.
            // xorpd %tmp_xmm, %tmp_xmm
            // cmpss/cmpsd %src, %tmp_xmm
            // jnb done
            // mov/movaps $INT_MAX, %dst
            //
            // done:
            //
            // Then, for non-saturating conversions:
            //
            // ;; check for NaN
            // cmpss/cmpsd %src, %src
            // jnp not_nan
            // ud2 trap BadConversionToInteger
            //
            // ;; check if INT_MIN was the correct result, against a magic constant:
            // not_nan:
            // movaps/mov $magic, %tmp_gpr
            // movq/movd %tmp_gpr, %tmp_xmm
            // cmpss/cmpsd %tmp_xmm, %src
            // jnb/jnbe $check_positive
            // ud2 trap IntegerOverflow
            //
            // ;; if positive, it was a real overflow
            // check_positive:
            // xorpd %tmp_xmm, %tmp_xmm
            // cmpss/cmpsd %src, %tmp_xmm
            // jnb done
            // ud2 trap IntegerOverflow
            //
            // done:

            let src = src.to_reg();

            let (cast_op, cmp_op, trunc_op) = match src_size {
                OperandSize::Size64 => (SseOpcode::Movq, SseOpcode::Ucomisd, SseOpcode::Cvttsd2si),
                OperandSize::Size32 => (SseOpcode::Movd, SseOpcode::Ucomiss, SseOpcode::Cvttss2si),
            };

            let done = sink.get_label();
            let not_nan = sink.get_label();

            // The truncation.
            let inst = Inst::xmm_to_gpr(trunc_op, src, *dst, *dst_size);
            inst.emit(sink, flags, state);

            // Compare against 1, in case of overflow the dst operand was INT_MIN.
            let inst = Inst::cmp_rmi_r(dst_size.to_bytes(), RegMemImm::imm(1), dst.to_reg());
            inst.emit(sink, flags, state);

            one_way_jmp(sink, CC::NO, done); // no overflow => done

            // Check for NaN.

            let inst = Inst::xmm_cmp_rm_r(cmp_op, RegMem::reg(src), src);
            inst.emit(sink, flags, state);

            one_way_jmp(sink, CC::NP, not_nan); // go to not_nan if not a NaN

            if *is_saturating {
                // For NaN, emit 0.
                let inst = Inst::alu_rmi_r(
                    *dst_size == OperandSize::Size64,
                    AluRmiROpcode::Xor,
                    RegMemImm::reg(dst.to_reg()),
                    *dst,
                );
                inst.emit(sink, flags, state);

                let inst = Inst::jmp_known(BranchTarget::Label(done));
                inst.emit(sink, flags, state);

                sink.bind_label(not_nan);

                // If the input was positive, saturate to INT_MAX.

                // Zero out tmp_xmm.
                let inst =
                    Inst::xmm_rm_r(SseOpcode::Xorpd, RegMem::reg(tmp_xmm.to_reg()), *tmp_xmm);
                inst.emit(sink, flags, state);

                let inst = Inst::xmm_cmp_rm_r(cmp_op, RegMem::reg(src), tmp_xmm.to_reg());
                inst.emit(sink, flags, state);

                // Jump if >= to done.
                one_way_jmp(sink, CC::NB, done);

                // Otherwise, put INT_MAX.
                if *dst_size == OperandSize::Size64 {
                    let inst = Inst::imm_r(true, 0x7fffffffffffffff, *dst);
                    inst.emit(sink, flags, state);
                } else {
                    let inst = Inst::imm_r(false, 0x7fffffff, *dst);
                    inst.emit(sink, flags, state);
                }
            } else {
                let check_positive = sink.get_label();

                let inst = Inst::trap(*srcloc, TrapCode::BadConversionToInteger);
                inst.emit(sink, flags, state);

                // Check if INT_MIN was the correct result: determine the smallest floating point
                // number that would convert to INT_MIN, put it in a temporary register, and compare
                // against the src register.
                // If the src register is less (or in some cases, less-or-equal) than the threshold,
                // trap!

                sink.bind_label(not_nan);

                let mut no_overflow_cc = CC::NB; // >=
                let output_bits = dst_size.to_bits();
                match *src_size {
                    OperandSize::Size32 => {
                        let cst = Ieee32::pow2(output_bits - 1).neg().bits();
                        let inst = Inst::imm32_r_unchecked(cst as u64, *tmp_gpr);
                        inst.emit(sink, flags, state);
                    }
                    OperandSize::Size64 => {
                        // An f64 can represent `i32::min_value() - 1` exactly with precision to spare,
                        // so there are values less than -2^(N-1) that convert correctly to INT_MIN.
                        let cst = if output_bits < 64 {
                            no_overflow_cc = CC::NBE; // >
                            Ieee64::fcvt_to_sint_negative_overflow(output_bits)
                        } else {
                            Ieee64::pow2(output_bits - 1).neg()
                        };
                        let inst = Inst::imm_r(true, cst.bits(), *tmp_gpr);
                        inst.emit(sink, flags, state);
                    }
                }

                let inst =
                    Inst::gpr_to_xmm(cast_op, RegMem::reg(tmp_gpr.to_reg()), *src_size, *tmp_xmm);
                inst.emit(sink, flags, state);

                let inst = Inst::xmm_cmp_rm_r(cmp_op, RegMem::reg(tmp_xmm.to_reg()), src);
                inst.emit(sink, flags, state);

                // jump over trap if src >= or > threshold
                one_way_jmp(sink, no_overflow_cc, check_positive);

                let inst = Inst::trap(*srcloc, TrapCode::IntegerOverflow);
                inst.emit(sink, flags, state);

                // If positive, it was a real overflow.

                sink.bind_label(check_positive);

                // Zero out the tmp_xmm register.
                let inst =
                    Inst::xmm_rm_r(SseOpcode::Xorpd, RegMem::reg(tmp_xmm.to_reg()), *tmp_xmm);
                inst.emit(sink, flags, state);

                let inst = Inst::xmm_cmp_rm_r(cmp_op, RegMem::reg(src), tmp_xmm.to_reg());
                inst.emit(sink, flags, state);

                one_way_jmp(sink, CC::NB, done); // jump over trap if 0 >= src

                let inst = Inst::trap(*srcloc, TrapCode::IntegerOverflow);
                inst.emit(sink, flags, state);
            }

            sink.bind_label(done);
        }

        Inst::CvtFloatToUintSeq {
            src_size,
            dst_size,
            is_saturating,
            src,
            dst,
            tmp_gpr,
            tmp_xmm,
            srcloc,
        } => {
            // The only difference in behavior between saturating and non-saturating is how we
            // handle errors. Emits the following sequence:
            //
            // movaps/mov 2**(int_width - 1), %tmp_gpr
            // movq/movd %tmp_gpr, %tmp_xmm
            // cmpss/cmpsd %tmp_xmm, %src
            // jnb is_large
            //
            // ;; check for NaN inputs
            // jnp not_nan
            // -- non-saturating: ud2 trap BadConversionToInteger
            // -- saturating: xor %dst, %dst; j done
            //
            // not_nan:
            // cvttss2si/cvttsd2si %src, %dst
            // cmp 0, %dst
            // jnl done
            // -- non-saturating: ud2 trap IntegerOverflow
            // -- saturating: xor %dst, %dst; j done
            //
            // is_large:
            // subss/subsd %tmp_xmm, %src ; <-- we clobber %src here
            // cvttss2si/cvttss2sd %tmp_x, %dst
            // cmp 0, %dst
            // jnl next_is_large
            // -- non-saturating: ud2 trap IntegerOverflow
            // -- saturating: movaps $UINT_MAX, %dst; j done
            //
            // next_is_large:
            // add 2**(int_width -1), %dst ;; 2 instructions for 64-bits integers
            //
            // done:

            assert_ne!(tmp_xmm, src, "tmp_xmm clobbers src!");

            let (sub_op, cast_op, cmp_op, trunc_op) = if *src_size == OperandSize::Size64 {
                (
                    SseOpcode::Subsd,
                    SseOpcode::Movq,
                    SseOpcode::Ucomisd,
                    SseOpcode::Cvttsd2si,
                )
            } else {
                (
                    SseOpcode::Subss,
                    SseOpcode::Movd,
                    SseOpcode::Ucomiss,
                    SseOpcode::Cvttss2si,
                )
            };

            let done = sink.get_label();

            if *src_size == OperandSize::Size64 {
                let cst = Ieee64::pow2(dst_size.to_bits() - 1).bits();
                let inst = Inst::imm_r(true, cst, *tmp_gpr);
                inst.emit(sink, flags, state);
            } else {
                let cst = Ieee32::pow2(dst_size.to_bits() - 1).bits() as u64;
                let inst = Inst::imm32_r_unchecked(cst, *tmp_gpr);
                inst.emit(sink, flags, state);
            }

            let inst =
                Inst::gpr_to_xmm(cast_op, RegMem::reg(tmp_gpr.to_reg()), *src_size, *tmp_xmm);
            inst.emit(sink, flags, state);

            let inst = Inst::xmm_cmp_rm_r(cmp_op, RegMem::reg(tmp_xmm.to_reg()), src.to_reg());
            inst.emit(sink, flags, state);

            let handle_large = sink.get_label();
            one_way_jmp(sink, CC::NB, handle_large); // jump to handle_large if src >= large_threshold

            let not_nan = sink.get_label();
            one_way_jmp(sink, CC::NP, not_nan); // jump over trap if not NaN

            if *is_saturating {
                // Emit 0.
                let inst = Inst::alu_rmi_r(
                    *dst_size == OperandSize::Size64,
                    AluRmiROpcode::Xor,
                    RegMemImm::reg(dst.to_reg()),
                    *dst,
                );
                inst.emit(sink, flags, state);

                let inst = Inst::jmp_known(BranchTarget::Label(done));
                inst.emit(sink, flags, state);
            } else {
                // Trap.
                let inst = Inst::trap(*srcloc, TrapCode::BadConversionToInteger);
                inst.emit(sink, flags, state);
            }

            sink.bind_label(not_nan);

            // Actual truncation for small inputs: if the result is not positive, then we had an
            // overflow.

            let inst = Inst::xmm_to_gpr(trunc_op, src.to_reg(), *dst, *dst_size);
            inst.emit(sink, flags, state);

            let inst = Inst::cmp_rmi_r(dst_size.to_bytes(), RegMemImm::imm(0), dst.to_reg());
            inst.emit(sink, flags, state);

            one_way_jmp(sink, CC::NL, done); // if dst >= 0, jump to done

            if *is_saturating {
                // The input was "small" (< 2**(width -1)), so the only way to get an integer
                // overflow is because the input was too small: saturate to the min value, i.e. 0.
                let inst = Inst::alu_rmi_r(
                    *dst_size == OperandSize::Size64,
                    AluRmiROpcode::Xor,
                    RegMemImm::reg(dst.to_reg()),
                    *dst,
                );
                inst.emit(sink, flags, state);

                let inst = Inst::jmp_known(BranchTarget::Label(done));
                inst.emit(sink, flags, state);
            } else {
                // Trap.
                let inst = Inst::trap(*srcloc, TrapCode::IntegerOverflow);
                inst.emit(sink, flags, state);
            }

            // Now handle large inputs.

            sink.bind_label(handle_large);

            let inst = Inst::xmm_rm_r(sub_op, RegMem::reg(tmp_xmm.to_reg()), *src);
            inst.emit(sink, flags, state);

            let inst = Inst::xmm_to_gpr(trunc_op, src.to_reg(), *dst, *dst_size);
            inst.emit(sink, flags, state);

            let inst = Inst::cmp_rmi_r(dst_size.to_bytes(), RegMemImm::imm(0), dst.to_reg());
            inst.emit(sink, flags, state);

            let next_is_large = sink.get_label();
            one_way_jmp(sink, CC::NL, next_is_large); // if dst >= 0, jump to next_is_large

            if *is_saturating {
                // The input was "large" (>= 2**(width -1)), so the only way to get an integer
                // overflow is because the input was too large: saturate to the max value.
                let inst = Inst::imm_r(
                    true,
                    if *dst_size == OperandSize::Size64 {
                        u64::max_value()
                    } else {
                        u32::max_value() as u64
                    },
                    *dst,
                );
                inst.emit(sink, flags, state);

                let inst = Inst::jmp_known(BranchTarget::Label(done));
                inst.emit(sink, flags, state);
            } else {
                let inst = Inst::trap(*srcloc, TrapCode::IntegerOverflow);
                inst.emit(sink, flags, state);
            }

            sink.bind_label(next_is_large);

            if *dst_size == OperandSize::Size64 {
                let inst = Inst::imm_r(true, 1 << 63, *tmp_gpr);
                inst.emit(sink, flags, state);

                let inst = Inst::alu_rmi_r(
                    true,
                    AluRmiROpcode::Add,
                    RegMemImm::reg(tmp_gpr.to_reg()),
                    *dst,
                );
                inst.emit(sink, flags, state);
            } else {
                let inst =
                    Inst::alu_rmi_r(false, AluRmiROpcode::Add, RegMemImm::imm(1 << 31), *dst);
                inst.emit(sink, flags, state);
            }

            sink.bind_label(done);
        }

        Inst::LoadExtName {
            dst,
            name,
            offset,
            srcloc,
        } => {
            // The full address can be encoded in the register, with a relocation.
            // Generates: movabsq $name, %dst
            let enc_dst = int_reg_enc(dst.to_reg());
            sink.put1(0x48 | ((enc_dst >> 3) & 1));
            sink.put1(0xB8 | (enc_dst & 7));
            sink.add_reloc(*srcloc, Reloc::Abs8, name, *offset);
            if flags.emit_all_ones_funcaddrs() {
                sink.put8(u64::max_value());
            } else {
                sink.put8(0);
            }
        }

        Inst::Hlt => {
            sink.put1(0xcc);
        }

        Inst::Ud2 { trap_info } => {
            sink.add_trap(trap_info.0, trap_info.1);
            if let Some(s) = state.take_stackmap() {
                sink.add_stackmap(StackmapExtent::UpcomingBytes(2), s);
            }
            sink.put1(0x0f);
            sink.put1(0x0b);
        }

        Inst::VirtualSPOffsetAdj { offset } => {
            debug!(
                "virtual sp offset adjusted by {} -> {}",
                offset,
                state.virtual_sp_offset + offset
            );
            state.virtual_sp_offset += offset;
        }

        Inst::Nop { .. } | Inst::EpiloguePlaceholder => {
            // Generate no code.
        }
    }

    state.clear_post_insn();
}
