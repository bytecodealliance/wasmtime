use log::debug;
use regalloc::Reg;

use std::convert::TryFrom;

use crate::binemit::Reloc;
use crate::isa::x64::inst::*;

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
            is_div,
            is_signed,
            size,
            divisor,
            loc,
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
            let inst = Inst::cmp_rmi_r(*size, RegMemImm::imm(0), *divisor);
            inst.emit(sink, flags, state);

            let inst = Inst::trap_if(CC::Z, TrapCode::IntegerDivisionByZero, *loc);
            inst.emit(sink, flags, state);

            let (do_op, done_label) = if *is_signed {
                // Now check if the divisor is -1.
                let inst = Inst::cmp_rmi_r(*size, RegMemImm::imm(0xffffffff), *divisor);
                inst.emit(sink, flags, state);

                let do_op = sink.get_label();

                // If not equal, jump to do-op.
                one_way_jmp(sink, CC::NZ, do_op);

                // Here, divisor == -1.
                if !*is_div {
                    // x % -1 = 0; put the result into the destination, $rdx.
                    let done_label = sink.get_label();

                    let inst = Inst::imm_r(*size == 8, 0, Writable::from_reg(regs::rdx()));
                    inst.emit(sink, flags, state);

                    let inst = Inst::jmp_known(BranchTarget::Label(done_label));
                    inst.emit(sink, flags, state);

                    (Some(do_op), Some(done_label))
                } else {
                    // Check for integer overflow.
                    let inst = Inst::cmp_rmi_r(*size, RegMemImm::imm(0x80000000), regs::rax());
                    inst.emit(sink, flags, state);

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
            if *is_signed {
                // sign-extend the sign-bit of rax into rdx, for signed opcodes.
                let inst = Inst::sign_extend_rax_to_rdx(*size);
                inst.emit(sink, flags, state);
            } else {
                // zero for unsigned opcodes.
                let inst = Inst::imm_r(true /* is_64 */, 0, Writable::from_reg(regs::rdx()));
                inst.emit(sink, flags, state);
            }

            let inst = Inst::div(*size, *is_signed, RegMem::reg(*divisor), *loc);
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

        Inst::MovZX_RM_R { ext_mode, src, dst } => {
            let (opcodes, num_opcodes, rex_flags) = match ext_mode {
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
                RegMem::Reg { reg: src } => emit_std_reg_reg(
                    sink,
                    LegacyPrefix::None,
                    opcodes,
                    num_opcodes,
                    dst.to_reg(),
                    *src,
                    rex_flags,
                ),
                RegMem::Mem { addr: src } => emit_std_reg_mem(
                    sink,
                    LegacyPrefix::None,
                    opcodes,
                    num_opcodes,
                    dst.to_reg(),
                    &src.finalize(state),
                    rex_flags,
                ),
            }
        }

        Inst::Mov64_M_R { src, dst } => emit_std_reg_mem(
            sink,
            LegacyPrefix::None,
            0x8B,
            1,
            dst.to_reg(),
            &src.finalize(state),
            RexFlags::set_w(),
        ),

        Inst::LoadEffectiveAddress { addr, dst } => emit_std_reg_mem(
            sink,
            LegacyPrefix::None,
            0x8D,
            1,
            dst.to_reg(),
            &addr.finalize(state),
            RexFlags::set_w(),
        ),

        Inst::MovSX_RM_R { ext_mode, src, dst } => {
            let (opcodes, num_opcodes, rex_flags) = match ext_mode {
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
                RegMem::Reg { reg: src } => emit_std_reg_reg(
                    sink,
                    LegacyPrefix::None,
                    opcodes,
                    num_opcodes,
                    dst.to_reg(),
                    *src,
                    rex_flags,
                ),
                RegMem::Mem { addr: src } => emit_std_reg_mem(
                    sink,
                    LegacyPrefix::None,
                    opcodes,
                    num_opcodes,
                    dst.to_reg(),
                    &src.finalize(state),
                    rex_flags,
                ),
            }
        }

        Inst::Mov_R_M { size, src, dst } => {
            let dst = &dst.finalize(state);

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
                ShiftKind::Left => 4,
                ShiftKind::RightZ => 5,
                ShiftKind::RightS => 7,
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
            // mov %idx, %tmp2
            // lea start_of_jump_table_offset(%rip), %tmp1
            // movzlq [%tmp1, %tmp2], %tmp2
            // addq %tmp2, %tmp1
            // j *%tmp1
            // $start_of_jump_table:
            // -- jump table entries
            let default_label = match default_target {
                BranchTarget::Label(label) => label,
                _ => unreachable!(),
            };
            one_way_jmp(sink, CC::NB, *default_label); // idx unsigned >= jmp table size

            let inst = Inst::gen_move(*tmp2, *idx, I64);
            inst.emit(sink, flags, state);

            // Load base address of jump table.
            let start_of_jumptable = sink.get_label();
            let inst = Inst::lea(
                Amode::rip_relative(BranchTarget::Label(start_of_jumptable)),
                *tmp1,
            );
            inst.emit(sink, flags, state);

            // Load value out of jump table.
            let inst = Inst::movzx_rm_r(
                ExtMode::LQ,
                RegMem::mem(Amode::imm_reg_reg_shift(0, tmp1.to_reg(), tmp2.to_reg(), 2)),
                *tmp2,
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

        Inst::XMM_Mov_RM_R {
            op,
            src: src_e,
            dst: reg_g,
        } => {
            let rex = RexFlags::clear_w();
            let (prefix, opcode) = match op {
                SseOpcode::Movaps => (LegacyPrefix::None, 0x0F28),
                SseOpcode::Movd => (LegacyPrefix::_66, 0x0F6E),
                SseOpcode::Movsd => (LegacyPrefix::_F2, 0x0F10),
                SseOpcode::Movss => (LegacyPrefix::_F3, 0x0F10),
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

        Inst::XMM_RM_R {
            op,
            src: src_e,
            dst: reg_g,
        } => {
            let rex = RexFlags::clear_w();
            let (prefix, opcode) = match op {
                SseOpcode::Addss => (LegacyPrefix::_F3, 0x0F58),
                SseOpcode::Andps => (LegacyPrefix::None, 0x0F54),
                SseOpcode::Andnps => (LegacyPrefix::None, 0x0F55),
                SseOpcode::Divss => (LegacyPrefix::_F3, 0x0F5E),
                SseOpcode::Mulss => (LegacyPrefix::_F3, 0x0F59),
                SseOpcode::Orps => (LegacyPrefix::None, 0x0F56),
                SseOpcode::Subss => (LegacyPrefix::_F3, 0x0F5C),
                SseOpcode::Sqrtss => (LegacyPrefix::_F3, 0x0F51),
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
        Inst::XMM_Mov_R_M { op, src, dst } => {
            let rex = RexFlags::clear_w();
            let (prefix, opcode) = match op {
                SseOpcode::Movd => (LegacyPrefix::_66, 0x0F7E),
                SseOpcode::Movss => (LegacyPrefix::_F3, 0x0F11),
                _ => unimplemented!("Emit xmm mov r m"),
            };

            let dst = &dst.finalize(state);
            emit_std_reg_mem(sink, prefix, opcode, 2, *src, dst, rex);
        }
        Inst::Hlt => {
            sink.put1(0xcc);
        }

        Inst::Ud2 { trap_info } => {
            sink.add_trap(trap_info.0, trap_info.1);
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
}
