//! This module defines x86_64-specific machine instruction types.

use crate::binemit::{Addend, CodeOffset, Reloc, StackMap};
use crate::ir::{types, ExternalName, Opcode, SourceLoc, TrapCode, Type, ValueLabel};
use crate::isa::unwind::UnwindInst;
use crate::isa::x64::abi::X64ABIMachineSpec;
use crate::isa::x64::settings as x64_settings;
use crate::isa::CallConv;
use crate::machinst::*;
use crate::{settings, settings::Flags, CodegenError, CodegenResult};
use alloc::boxed::Box;
use alloc::vec::Vec;
use regalloc::{
    PrettyPrint, PrettyPrintSized, RealRegUniverse, Reg, RegClass, RegUsageCollector, SpillSlot,
    VirtualReg, Writable,
};
use smallvec::{smallvec, SmallVec};
use std::fmt;
use std::string::{String, ToString};

pub mod args;
mod emit;
#[cfg(test)]
mod emit_tests;
pub mod regs;
pub mod unwind;

use args::*;
use regs::{create_reg_universe_systemv, show_ireg_sized};

//=============================================================================
// Instructions (top level): definition

// Don't build these directly.  Instead use the Inst:: functions to create them.

/// Instructions.
#[derive(Clone)]
pub enum Inst {
    /// Nops of various sizes, including zero.
    Nop { len: u8 },

    // =====================================
    // Integer instructions.
    /// Integer arithmetic/bit-twiddling: (add sub and or xor mul adc? sbb?) (32 64) (reg addr imm) reg
    AluRmiR {
        size: OperandSize, // 4 or 8
        op: AluRmiROpcode,
        src1: Reg,
        src2: RegMemImm,
        dst: Writable<Reg>,
    },

    /// Instructions on GPR that only read src and defines dst (dst is not modified): bsr, etc.
    UnaryRmR {
        size: OperandSize, // 2, 4 or 8
        op: UnaryRmROpcode,
        src: RegMem,
        dst: Writable<Reg>,
    },

    /// Bitwise not
    Not {
        size: OperandSize, // 1, 2, 4 or 8
        src: Reg,
        dst: Writable<Reg>,
    },

    /// Integer negation
    Neg {
        size: OperandSize, // 1, 2, 4 or 8
        src: Reg,
        dst: Writable<Reg>,
    },

    /// Integer quotient and remainder: (div idiv) $rax $rdx (reg addr)
    Div {
        size: OperandSize, // 1, 2, 4 or 8
        signed: bool,
        divisor: RegMem,
        dividend: Reg,
        dst_quotient: Writable<Reg>,
        dst_remainder: Writable<Reg>,
    },

    /// The high bits (RDX) of a (un)signed multiply: RDX:RAX := RAX * rhs.
    MulHi {
        size: OperandSize, // 2, 4, or 8
        signed: bool,
        src1: Reg,
        src2: RegMem,
        dst_lo: Writable<Reg>,
        dst_hi: Writable<Reg>,
    },

    /// A synthetic sequence to implement the right inline checks for remainder and division,
    /// assuming the dividend is in %rax.
    ///
    /// Puts the result back into %rax if is_div, %rdx if !is_div, to mimic what the div
    /// instruction does.
    ///
    /// The generated code sequence is described in the emit's function match arm for this
    /// instruction.
    ///
    /// Note: %rdx is marked as modified by this instruction, to avoid an early clobber problem
    /// with the temporary and divisor registers. Make sure to zero %rdx right before this
    /// instruction, or you might run into regalloc failures where %rdx is live before its first
    /// def!
    CheckedDivOrRemSeq {
        kind: DivOrRemKind,
        size: OperandSize,
        dividend: Reg,
        /// The divisor operand. Note it's marked as modified so that it gets assigned a register
        /// different from the temporary.
        divisor: Writable<Reg>,
        dst_quotient: Writable<Reg>,
        dst_remainder: Writable<Reg>,
        tmp: Option<Writable<Reg>>,
    },

    /// Do a sign-extend based on the sign of the value in rax into rdx: (cwd cdq cqo)
    /// or al into ah: (cbw)
    SignExtendData {
        size: OperandSize, // 1, 2, 4 or 8
        src: Reg,
        dst: Writable<Reg>,
    },

    /// Constant materialization: (imm32 imm64) reg.
    ///
    /// Either: movl $imm32, %reg32 or movabsq $imm64, %reg32.
    Imm {
        dst_size: OperandSize, // 4 or 8
        simm64: u64,
        dst: Writable<Reg>,
    },

    /// GPR to GPR move: mov (64 32) reg reg.
    MovRR {
        size: OperandSize, // 4 or 8
        src: Reg,
        dst: Writable<Reg>,
    },

    /// Zero-extended loads, except for 64 bits: movz (bl bq wl wq lq) addr reg.
    /// Note that the lq variant doesn't really exist since the default zero-extend rule makes it
    /// unnecessary. For that case we emit the equivalent "movl AM, reg32".
    MovzxRmR {
        ext_mode: ExtMode,
        src: RegMem,
        dst: Writable<Reg>,
    },

    /// A plain 64-bit integer load, since MovZX_RM_R can't represent that.
    Mov64MR {
        src: SyntheticAmode,
        dst: Writable<Reg>,
    },

    /// Loads the memory address of addr into dst.
    LoadEffectiveAddress {
        addr: SyntheticAmode,
        dst: Writable<Reg>,
    },

    /// Sign-extended loads and moves: movs (bl bq wl wq lq) addr reg.
    MovsxRmR {
        ext_mode: ExtMode,
        src: RegMem,
        dst: Writable<Reg>,
    },

    /// Integer stores: mov (b w l q) reg addr.
    MovRM {
        size: OperandSize, // 1, 2, 4 or 8.
        src: Reg,
        dst: SyntheticAmode,
    },

    /// Arithmetic shifts: (shl shr sar) (b w l q) imm reg.
    ShiftR {
        size: OperandSize, // 1, 2, 4 or 8
        kind: ShiftKind,
        src: Reg,
        /// shift count: Some(0 .. #bits-in-type - 1), or None to mean "%cl".
        num_bits: Imm8Reg,
        dst: Writable<Reg>,
    },

    /// Arithmetic SIMD shifts.
    XmmRmiReg {
        opcode: SseOpcode,
        src1: Reg,
        src2: RegMemImm,
        dst: Writable<Reg>,
    },

    /// Integer comparisons/tests: cmp or test (b w l q) (reg addr imm) reg.
    CmpRmiR {
        size: OperandSize, // 1, 2, 4 or 8
        opcode: CmpOpcode,
        src: RegMemImm,
        dst: Reg,
    },

    /// Materializes the requested condition code in the destination reg.
    Setcc { cc: CC, dst: Writable<Reg> },

    /// Integer conditional move.
    /// Overwrites the destination register.
    Cmove {
        size: OperandSize, // 2, 4, or 8
        cc: CC,
        consequent: RegMem,
        alternative: Reg,
        dst: Writable<Reg>,
    },

    // =====================================
    // Stack manipulation.
    /// pushq (reg addr imm)
    Push64 { src: RegMemImm },

    /// popq reg
    Pop64 { dst: Writable<Reg> },

    // =====================================
    // Floating-point operations.
    /// XMM (scalar or vector) binary op: (add sub and or xor mul adc? sbb?) (32 64) (reg addr) reg
    XmmRmR {
        op: SseOpcode,
        src1: Reg,
        src2: RegMem,
        dst: Writable<Reg>,
    },

    XmmRmREvex {
        op: Avx512Opcode,
        src1: RegMem,
        src2: Reg,
        dst: Writable<Reg>,
    },

    /// XMM (scalar or vector) unary op: mov between XMM registers (32 64) (reg addr) reg, sqrt,
    /// etc.
    ///
    /// This differs from XMM_RM_R in that the dst register of XmmUnaryRmR is not used in the
    /// computation of the instruction dst value and so does not have to be a previously valid
    /// value. This is characteristic of mov instructions.
    XmmUnaryRmR {
        op: SseOpcode,
        src: RegMem,
        dst: Writable<Reg>,
    },

    XmmUnaryRmREvex {
        op: Avx512Opcode,
        src: RegMem,
        dst: Writable<Reg>,
    },

    /// XMM (scalar or vector) unary op (from xmm to reg/mem): stores, movd, movq
    XmmMovRM {
        op: SseOpcode,
        src: Reg,
        dst: SyntheticAmode,
    },

    /// XMM (vector) unary op (to move a constant value into an xmm register): movups
    XmmLoadConst {
        src: VCodeConstant,
        dst: Writable<Reg>,
        ty: Type,
    },

    /// XMM (scalar) unary op (from xmm to integer reg): movd, movq, cvtts{s,d}2si
    XmmToGpr {
        op: SseOpcode,
        src: Reg,
        dst: Writable<Reg>,
        dst_size: OperandSize,
    },

    /// XMM (scalar) unary op (from integer to float reg): movd, movq, cvtsi2s{s,d}
    GprToXmm {
        op: SseOpcode,
        src: RegMem,
        dst: Writable<Reg>,
        src_size: OperandSize,
    },

    /// Converts an unsigned int64 to a float32/float64.
    CvtUint64ToFloatSeq {
        dst_size: OperandSize, // 4 or 8
        /// A copy of the source register, fed by lowering. It is marked as modified during
        /// register allocation to make sure that the temporary registers differ from the src
        /// register, since both registers are live at the same time in the generated code
        /// sequence.
        src: Writable<Reg>,
        dst: Writable<Reg>,
        tmp_gpr1: Writable<Reg>,
        tmp_gpr2: Writable<Reg>,
    },

    /// Converts a scalar xmm to a signed int32/int64.
    CvtFloatToSintSeq {
        dst_size: OperandSize,
        src_size: OperandSize,
        is_saturating: bool,
        /// A copy of the source register, fed by lowering. It is marked as modified during
        /// register allocation to make sure that the temporary xmm register differs from the src
        /// register, since both registers are live at the same time in the generated code
        /// sequence.
        src: Writable<Reg>,
        dst: Writable<Reg>,
        tmp_gpr: Writable<Reg>,
        tmp_xmm: Writable<Reg>,
    },

    /// Converts a scalar xmm to an unsigned int32/int64.
    CvtFloatToUintSeq {
        src_size: OperandSize,
        dst_size: OperandSize,
        is_saturating: bool,
        /// A copy of the source register, fed by lowering, reused as a temporary. It is marked as
        /// modified during register allocation to make sure that the temporary xmm register
        /// differs from the src register, since both registers are live at the same time in the
        /// generated code sequence.
        src: Writable<Reg>,
        dst: Writable<Reg>,
        tmp_gpr: Writable<Reg>,
        tmp_xmm: Writable<Reg>,
    },

    /// A sequence to compute min/max with the proper NaN semantics for xmm registers.
    XmmMinMaxSeq {
        size: OperandSize,
        is_min: bool,
        lhs: Reg,
        rhs_dst: Writable<Reg>,
    },

    /// XMM (scalar) conditional move.
    /// Overwrites the destination register if cc is set.
    XmmCmove {
        size: OperandSize, // 4 or 8
        cc: CC,
        src: RegMem,
        dst: Writable<Reg>,
    },

    /// Float comparisons/tests: cmp (b w l q) (reg addr imm) reg.
    XmmCmpRmR {
        op: SseOpcode,
        src: RegMem,
        dst: Reg,
    },

    /// A binary XMM instruction with an 8-bit immediate: e.g. cmp (ps pd) imm (reg addr) reg
    XmmRmRImm {
        op: SseOpcode,
        src1: Reg,
        src2: RegMem,
        dst: Writable<Reg>,
        imm: u8,
        size: OperandSize, // 4 or 8
    },

    // =====================================
    // Control flow instructions.
    /// Direct call: call simm32.
    CallKnown {
        dest: ExternalName,
        uses: Vec<Reg>,
        defs: Vec<Writable<Reg>>,
        opcode: Opcode,
    },

    /// Indirect call: callq (reg mem).
    CallUnknown {
        dest: RegMem,
        uses: Vec<Reg>,
        defs: Vec<Writable<Reg>>,
        opcode: Opcode,
    },

    /// Return.
    Ret,

    /// A placeholder instruction, generating no code, meaning that a function epilogue must be
    /// inserted there.
    EpiloguePlaceholder,

    /// Jump to a known target: jmp simm32.
    JmpKnown { dst: MachLabel },

    /// One-way conditional branch: jcond cond target.
    ///
    /// This instruction is useful when we have conditional jumps depending on more than two
    /// conditions, see for instance the lowering of Brz/brnz with Fcmp inputs.
    ///
    /// A note of caution: in contexts where the branch target is another block, this has to be the
    /// same successor as the one specified in the terminator branch of the current block.
    /// Otherwise, this might confuse register allocation by creating new invisible edges.
    JmpIf { cc: CC, taken: MachLabel },

    /// Two-way conditional branch: jcond cond target target.
    /// Emitted as a compound sequence; the MachBuffer will shrink it as appropriate.
    JmpCond {
        cc: CC,
        taken: MachLabel,
        not_taken: MachLabel,
    },

    /// Jump-table sequence, as one compound instruction (see note in lower.rs for rationale).
    /// The generated code sequence is described in the emit's function match arm for this
    /// instruction.
    /// See comment in lowering about the temporaries signedness.
    JmpTableSeq {
        idx: Reg,
        tmp1: Writable<Reg>,
        tmp2: Writable<Reg>,
        default_target: MachLabel,
        targets: Vec<MachLabel>,
        targets_for_term: Vec<MachLabel>,
    },

    /// Indirect jump: jmpq (reg mem).
    JmpUnknown { target: RegMem },

    /// Traps if the condition code is set.
    TrapIf { cc: CC, trap_code: TrapCode },

    /// A debug trap.
    Hlt,

    /// An instruction that will always trigger the illegal instruction exception.
    Ud2 { trap_code: TrapCode },

    /// Loads an external symbol in a register, with a relocation:
    ///
    /// movq $name@GOTPCREL(%rip), dst    if PIC is enabled, or
    /// movabsq $name, dst                otherwise.
    LoadExtName {
        dst: Writable<Reg>,
        name: Box<ExternalName>,
        offset: i64,
    },

    // =====================================
    // Instructions pertaining to atomic memory accesses.
    /// A standard (native) `lock cmpxchg src, (amode)`, with register conventions:
    ///
    /// `mem`          (read) address
    /// `replacement`  (read) replacement value
    /// %rax           (modified) in: expected value, out: value that was actually at `dst`
    /// %rflags is written.  Do not assume anything about it after the instruction.
    ///
    /// The instruction "succeeded" iff the lowest `ty` bits of %rax afterwards are the same as
    /// they were before.
    LockCmpxchg {
        ty: Type, // I8, I16, I32 or I64
        replacement: Reg,
        expected: Reg,
        mem: SyntheticAmode,
        dst_old: Writable<Reg>,
    },

    /// A synthetic instruction, based on a loop around a native `lock cmpxchg` instruction.
    /// This atomically modifies a value in memory and returns the old value.  The sequence
    /// consists of an initial "normal" load from `dst`, followed by a loop which computes the
    /// new value and tries to compare-and-swap ("CAS") it into `dst`, using the native
    /// instruction `lock cmpxchg{b,w,l,q}` .  The loop iterates until the CAS is successful.
    /// If there is no contention, there will be only one pass through the loop body.  The
    /// sequence does *not* perform any explicit memory fence instructions
    /// (mfence/sfence/lfence).
    ///
    /// Note that the transaction is atomic in the sense that, as observed by some other thread,
    /// `dst` either has the initial or final value, but no other.  It isn't atomic in the sense
    /// of guaranteeing that no other thread writes to `dst` in between the initial load and the
    /// CAS -- but that would cause the CAS to fail unless the other thread's last write before
    /// the CAS wrote the same value that was already there.  In other words, this
    /// implementation suffers (unavoidably) from the A-B-A problem.
    ///
    /// This instruction sequence has fixed register uses as follows:
    ///
    /// %r9   (read) address
    /// %r10  (read) second operand for `op`
    /// %r11  (written) scratch reg; value afterwards has no meaning
    /// %rax  (written) the old value at %r9
    /// %rflags is written.  Do not assume anything about it after the instruction.
    AtomicRmwSeq {
        ty: Type, // I8, I16, I32 or I64
        op: inst_common::AtomicRmwOp,
        address: Reg,
        operand: Reg,
        temp: Writable<Reg>,
        dst_old: Writable<Reg>,
    },

    /// A memory fence (mfence, lfence or sfence).
    Fence { kind: FenceKind },

    // =====================================
    // Meta-instructions generating no code.
    /// Marker, no-op in generated code: SP "virtual offset" is adjusted. This
    /// controls how MemArg::NominalSPOffset args are lowered.
    VirtualSPOffsetAdj { offset: i64 },

    /// Provides a way to tell the register allocator that the upcoming sequence of instructions
    /// will overwrite `dst` so it should be considered as a `def`; use this with care.
    ///
    /// This is useful when we have a sequence of instructions whose register usages are nominally
    /// `mod`s, but such that the combination of operations creates a result that is independent of
    /// the initial register value. It's thus semantically a `def`, not a `mod`, when all the
    /// instructions are taken together, so we want to ensure the register is defined (its
    /// live-range starts) prior to the sequence to keep analyses happy.
    ///
    /// One alternative would be a compound instruction that somehow encapsulates the others and
    /// reports its own `def`s/`use`s/`mod`s; this adds complexity (the instruction list is no
    /// longer flat) and requires knowledge about semantics and initial-value independence anyway.
    XmmUninitializedValue { dst: Writable<Reg> },

    /// A call to the `ElfTlsGetAddr` libcall. Returns address
    /// of TLS symbol in rax.
    ElfTlsGetAddr { symbol: ExternalName },

    /// A Mach-O TLS symbol access. Returns address of the TLS
    /// symbol in rax.
    MachOTlsGetAddr { symbol: ExternalName },

    /// A definition of a value label.
    ValueLabelMarker { reg: Reg, label: ValueLabel },

    /// An unwind pseudoinstruction describing the state of the
    /// machine at this program point.
    Unwind { inst: UnwindInst },
}

pub(crate) fn low32_will_sign_extend_to_64(x: u64) -> bool {
    let xs = x as i64;
    xs == ((xs << 32) >> 32)
}

impl Inst {
    /// Retrieve a list of ISA feature sets in which the instruction is available. An empty list
    /// indicates that the instruction is available in the baseline feature set (i.e. SSE2 and
    /// below); more than one `InstructionSet` in the list indicates that the instruction is present
    /// *any* of the included ISA feature sets.
    fn available_in_any_isa(&self) -> SmallVec<[InstructionSet; 2]> {
        match self {
            // These instructions are part of SSE2, which is a basic requirement in Cranelift, and
            // don't have to be checked.
            Inst::AluRmiR { .. }
            | Inst::AtomicRmwSeq { .. }
            | Inst::CallKnown { .. }
            | Inst::CallUnknown { .. }
            | Inst::CheckedDivOrRemSeq { .. }
            | Inst::Cmove { .. }
            | Inst::CmpRmiR { .. }
            | Inst::CvtFloatToSintSeq { .. }
            | Inst::CvtFloatToUintSeq { .. }
            | Inst::CvtUint64ToFloatSeq { .. }
            | Inst::Div { .. }
            | Inst::EpiloguePlaceholder
            | Inst::Fence { .. }
            | Inst::Hlt
            | Inst::Imm { .. }
            | Inst::JmpCond { .. }
            | Inst::JmpIf { .. }
            | Inst::JmpKnown { .. }
            | Inst::JmpTableSeq { .. }
            | Inst::JmpUnknown { .. }
            | Inst::LoadEffectiveAddress { .. }
            | Inst::LoadExtName { .. }
            | Inst::LockCmpxchg { .. }
            | Inst::Mov64MR { .. }
            | Inst::MovRM { .. }
            | Inst::MovRR { .. }
            | Inst::MovsxRmR { .. }
            | Inst::MovzxRmR { .. }
            | Inst::MulHi { .. }
            | Inst::Neg { .. }
            | Inst::Not { .. }
            | Inst::Nop { .. }
            | Inst::Pop64 { .. }
            | Inst::Push64 { .. }
            | Inst::Ret
            | Inst::Setcc { .. }
            | Inst::ShiftR { .. }
            | Inst::SignExtendData { .. }
            | Inst::TrapIf { .. }
            | Inst::Ud2 { .. }
            | Inst::VirtualSPOffsetAdj { .. }
            | Inst::XmmCmove { .. }
            | Inst::XmmCmpRmR { .. }
            | Inst::XmmLoadConst { .. }
            | Inst::XmmMinMaxSeq { .. }
            | Inst::XmmUninitializedValue { .. }
            | Inst::ElfTlsGetAddr { .. }
            | Inst::MachOTlsGetAddr { .. }
            | Inst::ValueLabelMarker { .. }
            | Inst::Unwind { .. } => smallvec![],

            Inst::UnaryRmR { op, .. } => op.available_from(),

            // These use dynamic SSE opcodes.
            Inst::GprToXmm { op, .. }
            | Inst::XmmMovRM { op, .. }
            | Inst::XmmRmiReg { opcode: op, .. }
            | Inst::XmmRmR { op, .. }
            | Inst::XmmRmRImm { op, .. }
            | Inst::XmmToGpr { op, .. }
            | Inst::XmmUnaryRmR { op, .. } => smallvec![op.available_from()],

            Inst::XmmUnaryRmREvex { op, .. } | Inst::XmmRmREvex { op, .. } => op.available_from(),
        }
    }
}

// Handy constructors for Insts.

impl Inst {
    pub(crate) fn nop(len: u8) -> Self {
        debug_assert!(len <= 15);
        Self::Nop { len }
    }

    pub(crate) fn alu_rmi_r(
        size: OperandSize,
        op: AluRmiROpcode,
        src: RegMemImm,
        dst: Writable<Reg>,
    ) -> Self {
        debug_assert!(size.is_one_of(&[OperandSize::Size32, OperandSize::Size64]));
        src.assert_regclass_is(RegClass::I64);
        debug_assert!(dst.to_reg().get_class() == RegClass::I64);
        Self::AluRmiR {
            size,
            op,
            src1: dst.to_reg(),
            src2: src,
            dst,
        }
    }

    pub(crate) fn unary_rm_r(
        size: OperandSize,
        op: UnaryRmROpcode,
        src: RegMem,
        dst: Writable<Reg>,
    ) -> Self {
        src.assert_regclass_is(RegClass::I64);
        debug_assert!(dst.to_reg().get_class() == RegClass::I64);
        debug_assert!(size.is_one_of(&[
            OperandSize::Size16,
            OperandSize::Size32,
            OperandSize::Size64
        ]));
        Self::UnaryRmR { size, op, src, dst }
    }

    pub(crate) fn not(size: OperandSize, src: Writable<Reg>) -> Inst {
        debug_assert_eq!(src.to_reg().get_class(), RegClass::I64);
        Inst::Not {
            size,
            src: src.to_reg(),
            dst: src,
        }
    }

    pub(crate) fn neg(size: OperandSize, src: Writable<Reg>) -> Inst {
        debug_assert_eq!(src.to_reg().get_class(), RegClass::I64);
        Inst::Neg {
            size,
            src: src.to_reg(),
            dst: src,
        }
    }

    pub(crate) fn div(size: OperandSize, signed: bool, divisor: RegMem) -> Inst {
        divisor.assert_regclass_is(RegClass::I64);
        Inst::Div {
            size,
            signed,
            divisor,
            dividend: regs::rax(),
            dst_quotient: Writable::from_reg(regs::rax()),
            dst_remainder: Writable::from_reg(regs::rdx()),
        }
    }

    pub(crate) fn mul_hi(size: OperandSize, signed: bool, rhs: RegMem) -> Inst {
        debug_assert!(size.is_one_of(&[
            OperandSize::Size16,
            OperandSize::Size32,
            OperandSize::Size64
        ]));
        rhs.assert_regclass_is(RegClass::I64);
        Inst::MulHi {
            size,
            signed,
            src1: regs::rax(),
            src2: rhs,
            dst_lo: Writable::from_reg(regs::rax()),
            dst_hi: Writable::from_reg(regs::rdx()),
        }
    }

    pub(crate) fn checked_div_or_rem_seq(
        kind: DivOrRemKind,
        size: OperandSize,
        divisor: Writable<Reg>,
        tmp: Option<Writable<Reg>>,
    ) -> Inst {
        debug_assert!(divisor.to_reg().get_class() == RegClass::I64);
        debug_assert!(tmp
            .map(|tmp| tmp.to_reg().get_class() == RegClass::I64)
            .unwrap_or(true));
        Inst::CheckedDivOrRemSeq {
            kind,
            size,
            divisor,
            dividend: regs::rax(),
            dst_quotient: Writable::from_reg(regs::rax()),
            dst_remainder: Writable::from_reg(regs::rdx()),
            tmp,
        }
    }

    pub(crate) fn sign_extend_data(size: OperandSize) -> Inst {
        Inst::SignExtendData {
            size,
            src: regs::rax(),
            dst: Writable::from_reg(regs::rdx()),
        }
    }

    pub(crate) fn imm(dst_size: OperandSize, simm64: u64, dst: Writable<Reg>) -> Inst {
        debug_assert!(dst_size.is_one_of(&[OperandSize::Size32, OperandSize::Size64]));
        debug_assert!(dst.to_reg().get_class() == RegClass::I64);
        // Try to generate a 32-bit immediate when the upper high bits are zeroed (which matches
        // the semantics of movl).
        let dst_size = match dst_size {
            OperandSize::Size64 if simm64 > u32::max_value() as u64 => OperandSize::Size64,
            _ => OperandSize::Size32,
        };
        Inst::Imm {
            dst_size,
            simm64,
            dst,
        }
    }

    pub(crate) fn mov_r_r(size: OperandSize, src: Reg, dst: Writable<Reg>) -> Inst {
        debug_assert!(size.is_one_of(&[OperandSize::Size32, OperandSize::Size64]));
        debug_assert!(src.get_class() == RegClass::I64);
        debug_assert!(dst.to_reg().get_class() == RegClass::I64);
        Inst::MovRR { size, src, dst }
    }

    // TODO Can be replaced by `Inst::move` (high-level) and `Inst::unary_rm_r` (low-level)
    pub(crate) fn xmm_mov(op: SseOpcode, src: RegMem, dst: Writable<Reg>) -> Inst {
        src.assert_regclass_is(RegClass::V128);
        debug_assert!(dst.to_reg().get_class() == RegClass::V128);
        Inst::XmmUnaryRmR { op, src, dst }
    }

    pub(crate) fn xmm_load_const(src: VCodeConstant, dst: Writable<Reg>, ty: Type) -> Inst {
        debug_assert!(dst.to_reg().get_class() == RegClass::V128);
        debug_assert!(ty.is_vector() && ty.bits() == 128);
        Inst::XmmLoadConst { src, dst, ty }
    }

    /// Convenient helper for unary float operations.
    pub(crate) fn xmm_unary_rm_r(op: SseOpcode, src: RegMem, dst: Writable<Reg>) -> Inst {
        src.assert_regclass_is(RegClass::V128);
        debug_assert!(dst.to_reg().get_class() == RegClass::V128);
        Inst::XmmUnaryRmR { op, src, dst }
    }

    pub(crate) fn xmm_unary_rm_r_evex(op: Avx512Opcode, src: RegMem, dst: Writable<Reg>) -> Inst {
        src.assert_regclass_is(RegClass::V128);
        debug_assert!(dst.to_reg().get_class() == RegClass::V128);
        Inst::XmmUnaryRmREvex { op, src, dst }
    }

    pub(crate) fn xmm_rm_r(op: SseOpcode, src: RegMem, dst: Writable<Reg>) -> Self {
        src.assert_regclass_is(RegClass::V128);
        debug_assert!(dst.to_reg().get_class() == RegClass::V128);
        Inst::XmmRmR {
            op,
            src1: dst.to_reg(),
            src2: src,
            dst,
        }
    }

    pub(crate) fn xmm_rm_r_evex(
        op: Avx512Opcode,
        src1: RegMem,
        src2: Reg,
        dst: Writable<Reg>,
    ) -> Self {
        src1.assert_regclass_is(RegClass::V128);
        debug_assert!(src2.get_class() == RegClass::V128);
        debug_assert!(dst.to_reg().get_class() == RegClass::V128);
        Inst::XmmRmREvex {
            op,
            src1,
            src2,
            dst,
        }
    }

    pub(crate) fn xmm_uninit_value(dst: Writable<Reg>) -> Self {
        debug_assert!(dst.to_reg().get_class() == RegClass::V128);
        Inst::XmmUninitializedValue { dst }
    }

    pub(crate) fn xmm_mov_r_m(op: SseOpcode, src: Reg, dst: impl Into<SyntheticAmode>) -> Inst {
        debug_assert!(src.get_class() == RegClass::V128);
        Inst::XmmMovRM {
            op,
            src,
            dst: dst.into(),
        }
    }

    pub(crate) fn xmm_to_gpr(
        op: SseOpcode,
        src: Reg,
        dst: Writable<Reg>,
        dst_size: OperandSize,
    ) -> Inst {
        debug_assert!(src.get_class() == RegClass::V128);
        debug_assert!(dst.to_reg().get_class() == RegClass::I64);
        debug_assert!(dst_size.is_one_of(&[OperandSize::Size32, OperandSize::Size64]));
        Inst::XmmToGpr {
            op,
            src,
            dst,
            dst_size,
        }
    }

    pub(crate) fn gpr_to_xmm(
        op: SseOpcode,
        src: RegMem,
        src_size: OperandSize,
        dst: Writable<Reg>,
    ) -> Inst {
        src.assert_regclass_is(RegClass::I64);
        debug_assert!(src_size.is_one_of(&[OperandSize::Size32, OperandSize::Size64]));
        debug_assert!(dst.to_reg().get_class() == RegClass::V128);
        Inst::GprToXmm {
            op,
            src,
            dst,
            src_size,
        }
    }

    pub(crate) fn xmm_cmp_rm_r(op: SseOpcode, src: RegMem, dst: Reg) -> Inst {
        src.assert_regclass_is(RegClass::V128);
        debug_assert!(dst.get_class() == RegClass::V128);
        Inst::XmmCmpRmR { op, src, dst }
    }

    pub(crate) fn cvt_u64_to_float_seq(
        dst_size: OperandSize,
        src: Writable<Reg>,
        tmp_gpr1: Writable<Reg>,
        tmp_gpr2: Writable<Reg>,
        dst: Writable<Reg>,
    ) -> Inst {
        debug_assert!(dst_size.is_one_of(&[OperandSize::Size32, OperandSize::Size64]));
        debug_assert!(src.to_reg().get_class() == RegClass::I64);
        debug_assert!(tmp_gpr1.to_reg().get_class() == RegClass::I64);
        debug_assert!(tmp_gpr2.to_reg().get_class() == RegClass::I64);
        debug_assert!(dst.to_reg().get_class() == RegClass::V128);
        Inst::CvtUint64ToFloatSeq {
            src,
            dst,
            tmp_gpr1,
            tmp_gpr2,
            dst_size,
        }
    }

    pub(crate) fn cvt_float_to_sint_seq(
        src_size: OperandSize,
        dst_size: OperandSize,
        is_saturating: bool,
        src: Writable<Reg>,
        dst: Writable<Reg>,
        tmp_gpr: Writable<Reg>,
        tmp_xmm: Writable<Reg>,
    ) -> Inst {
        debug_assert!(src_size.is_one_of(&[OperandSize::Size32, OperandSize::Size64]));
        debug_assert!(dst_size.is_one_of(&[OperandSize::Size32, OperandSize::Size64]));
        debug_assert!(src.to_reg().get_class() == RegClass::V128);
        debug_assert!(tmp_xmm.to_reg().get_class() == RegClass::V128);
        debug_assert!(tmp_gpr.to_reg().get_class() == RegClass::I64);
        debug_assert!(dst.to_reg().get_class() == RegClass::I64);
        Inst::CvtFloatToSintSeq {
            src_size,
            dst_size,
            is_saturating,
            src,
            dst,
            tmp_gpr,
            tmp_xmm,
        }
    }

    pub(crate) fn cvt_float_to_uint_seq(
        src_size: OperandSize,
        dst_size: OperandSize,
        is_saturating: bool,
        src: Writable<Reg>,
        dst: Writable<Reg>,
        tmp_gpr: Writable<Reg>,
        tmp_xmm: Writable<Reg>,
    ) -> Inst {
        debug_assert!(src_size.is_one_of(&[OperandSize::Size32, OperandSize::Size64]));
        debug_assert!(dst_size.is_one_of(&[OperandSize::Size32, OperandSize::Size64]));
        debug_assert!(src.to_reg().get_class() == RegClass::V128);
        debug_assert!(tmp_xmm.to_reg().get_class() == RegClass::V128);
        debug_assert!(tmp_gpr.to_reg().get_class() == RegClass::I64);
        debug_assert!(dst.to_reg().get_class() == RegClass::I64);
        Inst::CvtFloatToUintSeq {
            src_size,
            dst_size,
            is_saturating,
            src,
            dst,
            tmp_gpr,
            tmp_xmm,
        }
    }

    pub(crate) fn xmm_min_max_seq(
        size: OperandSize,
        is_min: bool,
        lhs: Reg,
        rhs_dst: Writable<Reg>,
    ) -> Inst {
        debug_assert!(size.is_one_of(&[OperandSize::Size32, OperandSize::Size64]));
        debug_assert_eq!(lhs.get_class(), RegClass::V128);
        debug_assert_eq!(rhs_dst.to_reg().get_class(), RegClass::V128);
        Inst::XmmMinMaxSeq {
            size,
            is_min,
            lhs,
            rhs_dst,
        }
    }

    pub(crate) fn xmm_rm_r_imm(
        op: SseOpcode,
        src: RegMem,
        dst: Writable<Reg>,
        imm: u8,
        size: OperandSize,
    ) -> Inst {
        debug_assert!(size.is_one_of(&[OperandSize::Size32, OperandSize::Size64]));
        Inst::XmmRmRImm {
            op,
            src1: dst.to_reg(),
            src2: src,
            dst,
            imm,
            size,
        }
    }

    pub(crate) fn movzx_rm_r(ext_mode: ExtMode, src: RegMem, dst: Writable<Reg>) -> Inst {
        src.assert_regclass_is(RegClass::I64);
        debug_assert!(dst.to_reg().get_class() == RegClass::I64);
        Inst::MovzxRmR { ext_mode, src, dst }
    }

    pub(crate) fn xmm_rmi_reg(opcode: SseOpcode, src: RegMemImm, dst: Writable<Reg>) -> Inst {
        src.assert_regclass_is(RegClass::V128);
        debug_assert!(dst.to_reg().get_class() == RegClass::V128);
        Inst::XmmRmiReg {
            opcode,
            src1: dst.to_reg(),
            src2: src,
            dst,
        }
    }

    pub(crate) fn movsx_rm_r(ext_mode: ExtMode, src: RegMem, dst: Writable<Reg>) -> Inst {
        src.assert_regclass_is(RegClass::I64);
        debug_assert!(dst.to_reg().get_class() == RegClass::I64);
        Inst::MovsxRmR { ext_mode, src, dst }
    }

    pub(crate) fn mov64_m_r(src: impl Into<SyntheticAmode>, dst: Writable<Reg>) -> Inst {
        debug_assert!(dst.to_reg().get_class() == RegClass::I64);
        Inst::Mov64MR {
            src: src.into(),
            dst,
        }
    }

    /// A convenience function to be able to use a RegMem as the source of a move.
    pub(crate) fn mov64_rm_r(src: RegMem, dst: Writable<Reg>) -> Inst {
        src.assert_regclass_is(RegClass::I64);
        match src {
            RegMem::Reg { reg } => Self::mov_r_r(OperandSize::Size64, reg, dst),
            RegMem::Mem { addr } => Self::mov64_m_r(addr, dst),
        }
    }

    pub(crate) fn mov_r_m(size: OperandSize, src: Reg, dst: impl Into<SyntheticAmode>) -> Inst {
        debug_assert!(src.get_class() == RegClass::I64);
        Inst::MovRM {
            size,
            src,
            dst: dst.into(),
        }
    }

    pub(crate) fn lea(addr: impl Into<SyntheticAmode>, dst: Writable<Reg>) -> Inst {
        debug_assert!(dst.to_reg().get_class() == RegClass::I64);
        Inst::LoadEffectiveAddress {
            addr: addr.into(),
            dst,
        }
    }

    pub(crate) fn shift_r(
        size: OperandSize,
        kind: ShiftKind,
        num_bits: Option<u8>,
        dst: Writable<Reg>,
    ) -> Inst {
        debug_assert!(if let Some(num_bits) = num_bits {
            num_bits < size.to_bits()
        } else {
            true
        });
        debug_assert!(dst.to_reg().get_class() == RegClass::I64);
        Inst::ShiftR {
            size,
            kind,
            src: dst.to_reg(),
            num_bits: match num_bits {
                Some(imm) => Imm8Reg::Imm8 { imm },
                None => Imm8Reg::Reg { reg: regs::rcx() },
            },
            dst,
        }
    }

    /// Does a comparison of dst - src for operands of size `size`, as stated by the machine
    /// instruction semantics. Be careful with the order of parameters!
    pub(crate) fn cmp_rmi_r(size: OperandSize, src: RegMemImm, dst: Reg) -> Inst {
        src.assert_regclass_is(RegClass::I64);
        debug_assert_eq!(dst.get_class(), RegClass::I64);
        Inst::CmpRmiR {
            size,
            src,
            dst,
            opcode: CmpOpcode::Cmp,
        }
    }

    /// Does a comparison of dst & src for operands of size `size`.
    pub(crate) fn test_rmi_r(size: OperandSize, src: RegMemImm, dst: Reg) -> Inst {
        src.assert_regclass_is(RegClass::I64);
        debug_assert_eq!(dst.get_class(), RegClass::I64);
        Inst::CmpRmiR {
            size,
            src,
            dst,
            opcode: CmpOpcode::Test,
        }
    }

    pub(crate) fn trap(trap_code: TrapCode) -> Inst {
        Inst::Ud2 {
            trap_code: trap_code,
        }
    }

    pub(crate) fn setcc(cc: CC, dst: Writable<Reg>) -> Inst {
        debug_assert!(dst.to_reg().get_class() == RegClass::I64);
        Inst::Setcc { cc, dst }
    }

    pub(crate) fn cmove(size: OperandSize, cc: CC, src: RegMem, dst: Writable<Reg>) -> Inst {
        debug_assert!(size.is_one_of(&[
            OperandSize::Size16,
            OperandSize::Size32,
            OperandSize::Size64
        ]));
        debug_assert!(dst.to_reg().get_class() == RegClass::I64);
        Inst::Cmove {
            size,
            cc,
            consequent: src,
            alternative: dst.to_reg(),
            dst,
        }
    }

    pub(crate) fn xmm_cmove(size: OperandSize, cc: CC, src: RegMem, dst: Writable<Reg>) -> Inst {
        debug_assert!(size.is_one_of(&[OperandSize::Size32, OperandSize::Size64]));
        src.assert_regclass_is(RegClass::V128);
        debug_assert!(dst.to_reg().get_class() == RegClass::V128);
        Inst::XmmCmove { size, cc, src, dst }
    }

    pub(crate) fn push64(src: RegMemImm) -> Inst {
        src.assert_regclass_is(RegClass::I64);
        Inst::Push64 { src }
    }

    pub(crate) fn pop64(dst: Writable<Reg>) -> Inst {
        debug_assert!(dst.to_reg().get_class() == RegClass::I64);
        Inst::Pop64 { dst }
    }

    pub(crate) fn call_known(
        dest: ExternalName,
        uses: Vec<Reg>,
        defs: Vec<Writable<Reg>>,
        opcode: Opcode,
    ) -> Inst {
        Inst::CallKnown {
            dest,
            uses,
            defs,
            opcode,
        }
    }

    pub(crate) fn call_unknown(
        dest: RegMem,
        uses: Vec<Reg>,
        defs: Vec<Writable<Reg>>,
        opcode: Opcode,
    ) -> Inst {
        dest.assert_regclass_is(RegClass::I64);
        Inst::CallUnknown {
            dest,
            uses,
            defs,
            opcode,
        }
    }

    pub(crate) fn ret() -> Inst {
        Inst::Ret
    }

    pub(crate) fn epilogue_placeholder() -> Inst {
        Inst::EpiloguePlaceholder
    }

    pub(crate) fn jmp_known(dst: MachLabel) -> Inst {
        Inst::JmpKnown { dst }
    }

    pub(crate) fn jmp_if(cc: CC, taken: MachLabel) -> Inst {
        Inst::JmpIf { cc, taken }
    }

    pub(crate) fn jmp_cond(cc: CC, taken: MachLabel, not_taken: MachLabel) -> Inst {
        Inst::JmpCond {
            cc,
            taken,
            not_taken,
        }
    }

    pub(crate) fn jmp_unknown(target: RegMem) -> Inst {
        target.assert_regclass_is(RegClass::I64);
        Inst::JmpUnknown { target }
    }

    pub(crate) fn trap_if(cc: CC, trap_code: TrapCode) -> Inst {
        Inst::TrapIf { cc, trap_code }
    }

    /// Choose which instruction to use for loading a register value from memory. For loads smaller
    /// than 64 bits, this method expects a way to extend the value (i.e. [ExtKind::SignExtend],
    /// [ExtKind::ZeroExtend]); loads with no extension necessary will ignore this.
    pub(crate) fn load(
        ty: Type,
        from_addr: impl Into<SyntheticAmode>,
        to_reg: Writable<Reg>,
        ext_kind: ExtKind,
    ) -> Inst {
        let rc = to_reg.to_reg().get_class();
        match rc {
            RegClass::I64 => {
                let ext_mode = match ty.bytes() {
                    1 => Some(ExtMode::BQ),
                    2 => Some(ExtMode::WQ),
                    4 => Some(ExtMode::LQ),
                    8 => None,
                    _ => unreachable!("the type should never use a scalar load: {}", ty),
                };
                if let Some(ext_mode) = ext_mode {
                    // Values smaller than 64 bits must be extended in some way.
                    match ext_kind {
                        ExtKind::SignExtend => {
                            Inst::movsx_rm_r(ext_mode, RegMem::mem(from_addr), to_reg)
                        }
                        ExtKind::ZeroExtend => {
                            Inst::movzx_rm_r(ext_mode, RegMem::mem(from_addr), to_reg)
                        }
                        ExtKind::None => panic!(
                            "expected an extension kind for extension mode: {:?}",
                            ext_mode
                        ),
                    }
                } else {
                    // 64-bit values can be moved directly.
                    Inst::mov64_m_r(from_addr, to_reg)
                }
            }
            RegClass::V128 => {
                let opcode = match ty {
                    types::F32 => SseOpcode::Movss,
                    types::F64 => SseOpcode::Movsd,
                    types::F32X4 => SseOpcode::Movups,
                    types::F64X2 => SseOpcode::Movupd,
                    _ if ty.is_vector() && ty.bits() == 128 => SseOpcode::Movdqu,
                    _ => unimplemented!("unable to load type: {}", ty),
                };
                Inst::xmm_unary_rm_r(opcode, RegMem::mem(from_addr), to_reg)
            }
            _ => panic!("unable to generate load for register class: {:?}", rc),
        }
    }

    /// Choose which instruction to use for storing a register value to memory.
    pub(crate) fn store(ty: Type, from_reg: Reg, to_addr: impl Into<SyntheticAmode>) -> Inst {
        let rc = from_reg.get_class();
        match rc {
            RegClass::I64 => Inst::mov_r_m(OperandSize::from_ty(ty), from_reg, to_addr),
            RegClass::V128 => {
                let opcode = match ty {
                    types::F32 => SseOpcode::Movss,
                    types::F64 => SseOpcode::Movsd,
                    types::F32X4 => SseOpcode::Movups,
                    types::F64X2 => SseOpcode::Movupd,
                    _ if ty.is_vector() && ty.bits() == 128 => SseOpcode::Movdqu,
                    _ => unimplemented!("unable to store type: {}", ty),
                };
                Inst::xmm_mov_r_m(opcode, from_reg, to_addr)
            }
            _ => panic!("unable to generate store for register class: {:?}", rc),
        }
    }
}

// Inst helpers.

impl Inst {
    /// In certain cases, instructions of this format can act as a definition of an XMM register,
    /// producing a value that is independent of its initial value.
    ///
    /// For example, a vector equality comparison (`cmppd` or `cmpps`) that compares a register to
    /// itself will generate all ones as a result, regardless of its value. From the register
    /// allocator's point of view, we should (i) record the first register, which is normally a
    /// mod, as a def instead; and (ii) not record the second register as a use, because it is the
    /// same as the first register (already handled).
    fn produces_const(&self) -> bool {
        match self {
            Self::AluRmiR { op, src2, dst, .. } => {
                src2.to_reg() == Some(dst.to_reg())
                    && (*op == AluRmiROpcode::Xor || *op == AluRmiROpcode::Sub)
            }

            Self::XmmRmR { op, src2, dst, .. } => {
                src2.to_reg() == Some(dst.to_reg())
                    && (*op == SseOpcode::Xorps
                        || *op == SseOpcode::Xorpd
                        || *op == SseOpcode::Pxor
                        || *op == SseOpcode::Pcmpeqb
                        || *op == SseOpcode::Pcmpeqw
                        || *op == SseOpcode::Pcmpeqd
                        || *op == SseOpcode::Pcmpeqq)
            }

            Self::XmmRmRImm {
                op, src2, dst, imm, ..
            } => {
                src2.to_reg() == Some(dst.to_reg())
                    && (*op == SseOpcode::Cmppd || *op == SseOpcode::Cmpps)
                    && *imm == FcmpImm::Equal.encode()
            }

            _ => false,
        }
    }

    /// Choose which instruction to use for comparing two values for equality.
    pub(crate) fn equals(ty: Type, from: RegMem, to: Writable<Reg>) -> Inst {
        match ty {
            types::I8X16 | types::B8X16 => Inst::xmm_rm_r(SseOpcode::Pcmpeqb, from, to),
            types::I16X8 | types::B16X8 => Inst::xmm_rm_r(SseOpcode::Pcmpeqw, from, to),
            types::I32X4 | types::B32X4 => Inst::xmm_rm_r(SseOpcode::Pcmpeqd, from, to),
            types::I64X2 | types::B64X2 => Inst::xmm_rm_r(SseOpcode::Pcmpeqq, from, to),
            types::F32X4 => Inst::xmm_rm_r_imm(
                SseOpcode::Cmpps,
                from,
                to,
                FcmpImm::Equal.encode(),
                OperandSize::Size32,
            ),
            types::F64X2 => Inst::xmm_rm_r_imm(
                SseOpcode::Cmppd,
                from,
                to,
                FcmpImm::Equal.encode(),
                OperandSize::Size32,
            ),
            _ => unimplemented!("unimplemented type for Inst::equals: {}", ty),
        }
    }

    /// Choose which instruction to use for computing a bitwise AND on two values.
    pub(crate) fn and(ty: Type, from: RegMem, to: Writable<Reg>) -> Inst {
        match ty {
            types::F32X4 => Inst::xmm_rm_r(SseOpcode::Andps, from, to),
            types::F64X2 => Inst::xmm_rm_r(SseOpcode::Andpd, from, to),
            _ if ty.is_vector() && ty.bits() == 128 => Inst::xmm_rm_r(SseOpcode::Pand, from, to),
            _ => unimplemented!("unimplemented type for Inst::and: {}", ty),
        }
    }

    /// Choose which instruction to use for computing a bitwise AND NOT on two values.
    pub(crate) fn and_not(ty: Type, from: RegMem, to: Writable<Reg>) -> Inst {
        match ty {
            types::F32X4 => Inst::xmm_rm_r(SseOpcode::Andnps, from, to),
            types::F64X2 => Inst::xmm_rm_r(SseOpcode::Andnpd, from, to),
            _ if ty.is_vector() && ty.bits() == 128 => Inst::xmm_rm_r(SseOpcode::Pandn, from, to),
            _ => unimplemented!("unimplemented type for Inst::and_not: {}", ty),
        }
    }

    /// Choose which instruction to use for computing a bitwise OR on two values.
    pub(crate) fn or(ty: Type, from: RegMem, to: Writable<Reg>) -> Inst {
        match ty {
            types::F32X4 => Inst::xmm_rm_r(SseOpcode::Orps, from, to),
            types::F64X2 => Inst::xmm_rm_r(SseOpcode::Orpd, from, to),
            _ if ty.is_vector() && ty.bits() == 128 => Inst::xmm_rm_r(SseOpcode::Por, from, to),
            _ => unimplemented!("unimplemented type for Inst::or: {}", ty),
        }
    }

    /// Choose which instruction to use for computing a bitwise XOR on two values.
    pub(crate) fn xor(ty: Type, from: RegMem, to: Writable<Reg>) -> Inst {
        match ty {
            types::F32X4 => Inst::xmm_rm_r(SseOpcode::Xorps, from, to),
            types::F64X2 => Inst::xmm_rm_r(SseOpcode::Xorpd, from, to),
            _ if ty.is_vector() && ty.bits() == 128 => Inst::xmm_rm_r(SseOpcode::Pxor, from, to),
            _ => unimplemented!("unimplemented type for Inst::xor: {}", ty),
        }
    }

    /// Translate three-operand instructions into a sequence of two-operand
    /// instructions.
    ///
    /// For example:
    ///
    /// ```text
    /// x = add a, b
    /// ```
    ///
    /// Becomes:
    ///
    /// ```text
    /// mov x, a
    /// add x, b
    /// ```
    ///
    /// The three-operand form for instructions allows our ISLE DSL code to have
    /// a value-based, SSA view of the world. This method is responsible for
    /// undoing that.
    ///
    /// Note that register allocation cleans up most of these inserted `mov`s
    /// with its move coalescing.
    pub(crate) fn mov_mitosis(mut self) -> impl Iterator<Item = Self> {
        log::trace!("mov_mitosis({:?})", self);

        let mut insts = SmallVec::<[Self; 4]>::new();

        match &mut self {
            Inst::AluRmiR { src1, dst, .. } => {
                if *src1 != dst.to_reg() {
                    debug_assert!(src1.is_virtual());
                    insts.push(Self::gen_move(*dst, *src1, types::I64));
                    *src1 = dst.to_reg();
                }
                insts.push(self);
            }
            Inst::XmmRmiReg { src1, dst, .. } => {
                if *src1 != dst.to_reg() {
                    debug_assert!(src1.is_virtual());
                    insts.push(Self::gen_move(*dst, *src1, types::I8X16));
                    *src1 = dst.to_reg();
                }
                insts.push(self);
            }
            Inst::XmmRmR { src1, dst, .. } => {
                if *src1 != dst.to_reg() {
                    debug_assert!(src1.is_virtual());
                    insts.push(Self::gen_move(*dst, *src1, types::I8X16));
                    *src1 = dst.to_reg();
                }
                insts.push(self);
            }
            Inst::XmmRmRImm { src1, dst, .. } => {
                if *src1 != dst.to_reg() {
                    debug_assert!(src1.is_virtual());
                    insts.push(Self::gen_move(*dst, *src1, types::I8X16));
                    *src1 = dst.to_reg();
                }
                insts.push(self);
            }
            Inst::Cmove {
                size,
                alternative,
                dst,
                ..
            } => {
                if *alternative != dst.to_reg() {
                    debug_assert!(alternative.is_virtual());
                    insts.push(Self::mov_r_r(*size, *alternative, *dst));
                    *alternative = dst.to_reg();
                }
                insts.push(self);
            }
            Inst::Not { src, dst, .. } | Inst::Neg { src, dst, .. } => {
                if *src != dst.to_reg() {
                    debug_assert!(src.is_virtual());
                    insts.push(Self::gen_move(*dst, *src, types::I64));
                    *src = dst.to_reg();
                }
                insts.push(self);
            }
            Inst::Div {
                dividend,
                dst_quotient,
                dst_remainder,
                ..
            }
            | Inst::CheckedDivOrRemSeq {
                dividend,
                dst_quotient,
                dst_remainder,
                ..
            } => {
                if *dividend != regs::rax() {
                    debug_assert!(dividend.is_virtual());
                    insts.push(Self::gen_move(
                        Writable::from_reg(regs::rax()),
                        *dividend,
                        types::I64,
                    ));
                    *dividend = regs::rax();
                }
                let mut quotient_mov = None;
                if dst_quotient.to_reg() != regs::rax() {
                    debug_assert!(dst_quotient.to_reg().is_virtual());
                    quotient_mov = Some(Self::gen_move(*dst_quotient, regs::rax(), types::I64));
                    *dst_quotient = Writable::from_reg(regs::rax());
                }
                let mut remainder_mov = None;
                if dst_remainder.to_reg() != regs::rdx() {
                    debug_assert!(dst_remainder.to_reg().is_virtual());
                    remainder_mov = Some(Self::gen_move(*dst_remainder, regs::rdx(), types::I64));
                    *dst_remainder = Writable::from_reg(regs::rdx());
                }
                insts.push(self);
                insts.extend(quotient_mov);
                insts.extend(remainder_mov);
            }
            Inst::MulHi {
                src1,
                dst_lo,
                dst_hi,
                ..
            } => {
                if *src1 != regs::rax() {
                    debug_assert!(src1.is_virtual());
                    insts.push(Self::gen_move(
                        Writable::from_reg(regs::rax()),
                        *src1,
                        types::I64,
                    ));
                    *src1 = regs::rax();
                }
                let mut dst_lo_mov = None;
                if dst_lo.to_reg() != regs::rax() {
                    debug_assert!(dst_lo.to_reg().is_virtual());
                    dst_lo_mov = Some(Self::gen_move(*dst_lo, regs::rax(), types::I64));
                    *dst_lo = Writable::from_reg(regs::rax());
                }
                let mut dst_hi_mov = None;
                if dst_hi.to_reg() != regs::rdx() {
                    debug_assert!(dst_hi.to_reg().is_virtual());
                    dst_hi_mov = Some(Self::gen_move(*dst_hi, regs::rdx(), types::I64));
                    *dst_hi = Writable::from_reg(regs::rdx());
                }
                insts.push(self);
                insts.extend(dst_lo_mov);
                insts.extend(dst_hi_mov);
            }
            Inst::SignExtendData { src, dst, .. } => {
                if *src != regs::rax() {
                    debug_assert!(src.is_virtual());
                    insts.push(Self::gen_move(
                        Writable::from_reg(regs::rax()),
                        *src,
                        types::I64,
                    ));
                    *src = regs::rax();
                }
                let mut dst_mov = None;
                if dst.to_reg() != regs::rax() {
                    debug_assert!(dst.to_reg().is_virtual());
                    dst_mov = Some(Self::gen_move(*dst, dst.to_reg(), types::I64));
                    *dst = Writable::from_reg(regs::rax());
                }
                insts.push(self);
                insts.extend(dst_mov);
            }
            Inst::ShiftR {
                src, num_bits, dst, ..
            } => {
                if *src != dst.to_reg() {
                    debug_assert!(src.is_virtual());
                    insts.push(Self::gen_move(*dst, *src, types::I64));
                    *src = dst.to_reg();
                }
                if let Imm8Reg::Reg { reg } = num_bits {
                    if *reg != regs::rcx() {
                        debug_assert!(reg.is_virtual());
                        insts.push(Self::gen_move(
                            Writable::from_reg(regs::rcx()),
                            *reg,
                            types::I64,
                        ));
                        *reg = regs::rcx();
                    }
                }
                insts.push(self);
            }
            Inst::LockCmpxchg {
                ty,
                expected,
                dst_old,
                ..
            } => {
                if *expected != regs::rax() {
                    debug_assert!(expected.is_virtual());
                    insts.push(Self::gen_move(
                        Writable::from_reg(regs::rax()),
                        *expected,
                        *ty,
                    ));
                }
                let mut dst_old_mov = None;
                if dst_old.to_reg() != regs::rax() {
                    debug_assert!(dst_old.to_reg().is_virtual());
                    dst_old_mov = Some(Self::gen_move(*dst_old, regs::rax(), *ty));
                    *dst_old = Writable::from_reg(regs::rax());
                }
                insts.push(self);
                insts.extend(dst_old_mov);
            }
            Inst::AtomicRmwSeq {
                ty,
                address,
                operand,
                dst_old,
                ..
            } => {
                if *address != regs::r9() {
                    debug_assert!(address.is_virtual());
                    insts.push(Self::gen_move(
                        Writable::from_reg(regs::r9()),
                        *address,
                        types::I64,
                    ));
                    *address = regs::r9();
                }
                if *operand != regs::r10() {
                    debug_assert!(operand.is_virtual());
                    insts.push(Self::gen_move(
                        Writable::from_reg(regs::r10()),
                        *operand,
                        *ty,
                    ));
                    *address = regs::r10();
                }
                let mut dst_old_mov = None;
                if dst_old.to_reg() != regs::rax() {
                    debug_assert!(dst_old.to_reg().is_virtual());
                    dst_old_mov = Some(Self::gen_move(*dst_old, regs::rax(), *ty));
                    *dst_old = Writable::from_reg(regs::rax());
                }
                insts.push(self);
                insts.extend(dst_old_mov);
            }
            // No other instruction needs 3-operand to 2-operand legalization.
            _ => insts.push(self),
        }

        if log::log_enabled!(log::Level::Trace) {
            for inst in &insts {
                log::trace!("  -> {:?}", inst);
            }
        }

        insts.into_iter()
    }
}

//=============================================================================
// Instructions: printing

impl PrettyPrint for Inst {
    fn show_rru(&self, mb_rru: Option<&RealRegUniverse>) -> String {
        fn ljustify(s: String) -> String {
            let w = 7;
            if s.len() >= w {
                s
            } else {
                let need = usize::min(w, w - s.len());
                s + &format!("{nil: <width$}", nil = "", width = need)
            }
        }

        fn ljustify2(s1: String, s2: String) -> String {
            ljustify(s1 + &s2)
        }

        fn suffix_lq(size: OperandSize) -> String {
            match size {
                OperandSize::Size32 => "l",
                OperandSize::Size64 => "q",
                _ => unreachable!(),
            }
            .to_string()
        }

        fn suffix_lqb(size: OperandSize, is_8: bool) -> String {
            match (size, is_8) {
                (_, true) => "b",
                (OperandSize::Size32, false) => "l",
                (OperandSize::Size64, false) => "q",
                _ => unreachable!(),
            }
            .to_string()
        }

        fn size_lqb(size: OperandSize, is_8: bool) -> u8 {
            if is_8 {
                return 1;
            }
            size.to_bytes()
        }

        fn suffix_bwlq(size: OperandSize) -> String {
            match size {
                OperandSize::Size8 => "b".to_string(),
                OperandSize::Size16 => "w".to_string(),
                OperandSize::Size32 => "l".to_string(),
                OperandSize::Size64 => "q".to_string(),
            }
        }

        match self {
            Inst::Nop { len } => format!("{} len={}", ljustify("nop".to_string()), len),

            Inst::AluRmiR {
                size,
                op,
                src1: _,
                src2,
                dst,
            } => format!(
                "{} {}, {}",
                ljustify2(op.to_string(), suffix_lqb(*size, op.is_8bit())),
                src2.show_rru_sized(mb_rru, size_lqb(*size, op.is_8bit())),
                show_ireg_sized(dst.to_reg(), mb_rru, size_lqb(*size, op.is_8bit())),
            ),

            Inst::UnaryRmR { src, dst, op, size } => format!(
                "{} {}, {}",
                ljustify2(op.to_string(), suffix_bwlq(*size)),
                src.show_rru_sized(mb_rru, size.to_bytes()),
                show_ireg_sized(dst.to_reg(), mb_rru, size.to_bytes()),
            ),

            Inst::Not { size, src: _, dst } => format!(
                "{} {}",
                ljustify2("not".to_string(), suffix_bwlq(*size)),
                show_ireg_sized(dst.to_reg(), mb_rru, size.to_bytes())
            ),

            Inst::Neg { size, src: _, dst } => format!(
                "{} {}",
                ljustify2("neg".to_string(), suffix_bwlq(*size)),
                show_ireg_sized(dst.to_reg(), mb_rru, size.to_bytes())
            ),

            Inst::Div {
                size,
                signed,
                divisor,
                ..
            } => format!(
                "{} {}",
                ljustify(if *signed {
                    "idiv".to_string()
                } else {
                    "div".into()
                }),
                divisor.show_rru_sized(mb_rru, size.to_bytes())
            ),

            Inst::MulHi {
                size, signed, src2, ..
            } => format!(
                "{} {}",
                ljustify(if *signed {
                    "imul".to_string()
                } else {
                    "mul".to_string()
                }),
                src2.show_rru_sized(mb_rru, size.to_bytes())
            ),

            Inst::CheckedDivOrRemSeq {
                kind,
                size,
                divisor,
                ..
            } => format!(
                "{} $rax:$rdx, {}",
                match kind {
                    DivOrRemKind::SignedDiv => "sdiv",
                    DivOrRemKind::UnsignedDiv => "udiv",
                    DivOrRemKind::SignedRem => "srem",
                    DivOrRemKind::UnsignedRem => "urem",
                },
                show_ireg_sized(divisor.to_reg(), mb_rru, size.to_bytes()),
            ),

            Inst::SignExtendData { size, .. } => match size {
                OperandSize::Size8 => "cbw",
                OperandSize::Size16 => "cwd",
                OperandSize::Size32 => "cdq",
                OperandSize::Size64 => "cqo",
            }
            .into(),

            Inst::XmmUnaryRmR { op, src, dst, .. } => format!(
                "{} {}, {}",
                ljustify(op.to_string()),
                src.show_rru_sized(mb_rru, op.src_size()),
                show_ireg_sized(dst.to_reg(), mb_rru, 8),
            ),

            Inst::XmmUnaryRmREvex { op, src, dst, .. } => format!(
                "{} {}, {}",
                ljustify(op.to_string()),
                src.show_rru_sized(mb_rru, 8),
                show_ireg_sized(dst.to_reg(), mb_rru, 8),
            ),

            Inst::XmmMovRM { op, src, dst, .. } => format!(
                "{} {}, {}",
                ljustify(op.to_string()),
                show_ireg_sized(*src, mb_rru, 8),
                dst.show_rru(mb_rru),
            ),

            Inst::XmmRmR { op, src2, dst, .. } => format!(
                "{} {}, {}",
                ljustify(op.to_string()),
                src2.show_rru_sized(mb_rru, 8),
                show_ireg_sized(dst.to_reg(), mb_rru, 8),
            ),

            Inst::XmmRmREvex {
                op,
                src1,
                src2,
                dst,
                ..
            } => format!(
                "{} {}, {}, {}",
                ljustify(op.to_string()),
                src1.show_rru_sized(mb_rru, 8),
                show_ireg_sized(*src2, mb_rru, 8),
                show_ireg_sized(dst.to_reg(), mb_rru, 8),
            ),

            Inst::XmmMinMaxSeq {
                lhs,
                rhs_dst,
                is_min,
                size,
            } => format!(
                "{} {}, {}",
                ljustify2(
                    if *is_min {
                        "xmm min seq ".to_string()
                    } else {
                        "xmm max seq ".to_string()
                    },
                    format!("f{}", size.to_bits())
                ),
                show_ireg_sized(*lhs, mb_rru, 8),
                show_ireg_sized(rhs_dst.to_reg(), mb_rru, 8),
            ),

            Inst::XmmRmRImm {
                op,
                src2,
                dst,
                imm,
                size,
                ..
            } => format!(
                "{} ${}, {}, {}",
                ljustify(format!(
                    "{}{}",
                    op.to_string(),
                    if *size == OperandSize::Size64 {
                        ".w"
                    } else {
                        ""
                    }
                )),
                imm,
                src2.show_rru(mb_rru),
                dst.show_rru(mb_rru),
            ),

            Inst::XmmUninitializedValue { dst } => {
                format!("{} {}", ljustify("uninit".into()), dst.show_rru(mb_rru),)
            }

            Inst::XmmLoadConst { src, dst, .. } => {
                format!("load_const {:?}, {}", src, dst.show_rru(mb_rru),)
            }

            Inst::XmmToGpr {
                op,
                src,
                dst,
                dst_size,
            } => {
                let dst_size = dst_size.to_bytes();
                format!(
                    "{} {}, {}",
                    ljustify(op.to_string()),
                    src.show_rru(mb_rru),
                    show_ireg_sized(dst.to_reg(), mb_rru, dst_size),
                )
            }

            Inst::GprToXmm {
                op,
                src,
                src_size,
                dst,
            } => format!(
                "{} {}, {}",
                ljustify(op.to_string()),
                src.show_rru_sized(mb_rru, src_size.to_bytes()),
                dst.show_rru(mb_rru)
            ),

            Inst::XmmCmpRmR { op, src, dst } => format!(
                "{} {}, {}",
                ljustify(op.to_string()),
                src.show_rru_sized(mb_rru, 8),
                show_ireg_sized(*dst, mb_rru, 8),
            ),

            Inst::CvtUint64ToFloatSeq {
                src, dst, dst_size, ..
            } => format!(
                "{} {}, {}",
                ljustify(format!(
                    "u64_to_{}_seq",
                    if *dst_size == OperandSize::Size64 {
                        "f64"
                    } else {
                        "f32"
                    }
                )),
                show_ireg_sized(src.to_reg(), mb_rru, 8),
                dst.show_rru(mb_rru),
            ),

            Inst::CvtFloatToSintSeq {
                src,
                dst,
                src_size,
                dst_size,
                ..
            } => format!(
                "{} {}, {}",
                ljustify(format!(
                    "cvt_float{}_to_sint{}_seq",
                    src_size.to_bits(),
                    dst_size.to_bits()
                )),
                show_ireg_sized(src.to_reg(), mb_rru, 8),
                show_ireg_sized(dst.to_reg(), mb_rru, dst_size.to_bytes()),
            ),

            Inst::CvtFloatToUintSeq {
                src,
                dst,
                src_size,
                dst_size,
                ..
            } => format!(
                "{} {}, {}",
                ljustify(format!(
                    "cvt_float{}_to_uint{}_seq",
                    src_size.to_bits(),
                    dst_size.to_bits()
                )),
                show_ireg_sized(src.to_reg(), mb_rru, 8),
                show_ireg_sized(dst.to_reg(), mb_rru, dst_size.to_bytes()),
            ),

            Inst::Imm {
                dst_size,
                simm64,
                dst,
            } => {
                if *dst_size == OperandSize::Size64 {
                    format!(
                        "{} ${}, {}",
                        ljustify("movabsq".to_string()),
                        *simm64 as i64,
                        show_ireg_sized(dst.to_reg(), mb_rru, 8)
                    )
                } else {
                    format!(
                        "{} ${}, {}",
                        ljustify("movl".to_string()),
                        (*simm64 as u32) as i32,
                        show_ireg_sized(dst.to_reg(), mb_rru, 4)
                    )
                }
            }

            Inst::MovRR { size, src, dst } => format!(
                "{} {}, {}",
                ljustify2("mov".to_string(), suffix_lq(*size)),
                show_ireg_sized(*src, mb_rru, size.to_bytes()),
                show_ireg_sized(dst.to_reg(), mb_rru, size.to_bytes())
            ),

            Inst::MovzxRmR {
                ext_mode, src, dst, ..
            } => {
                if *ext_mode == ExtMode::LQ {
                    format!(
                        "{} {}, {}",
                        ljustify("movl".to_string()),
                        src.show_rru_sized(mb_rru, ext_mode.src_size()),
                        show_ireg_sized(dst.to_reg(), mb_rru, 4)
                    )
                } else {
                    format!(
                        "{} {}, {}",
                        ljustify2("movz".to_string(), ext_mode.to_string()),
                        src.show_rru_sized(mb_rru, ext_mode.src_size()),
                        show_ireg_sized(dst.to_reg(), mb_rru, ext_mode.dst_size())
                    )
                }
            }

            Inst::Mov64MR { src, dst, .. } => format!(
                "{} {}, {}",
                ljustify("movq".to_string()),
                src.show_rru(mb_rru),
                dst.show_rru(mb_rru)
            ),

            Inst::LoadEffectiveAddress { addr, dst } => format!(
                "{} {}, {}",
                ljustify("lea".to_string()),
                addr.show_rru(mb_rru),
                dst.show_rru(mb_rru)
            ),

            Inst::MovsxRmR {
                ext_mode, src, dst, ..
            } => format!(
                "{} {}, {}",
                ljustify2("movs".to_string(), ext_mode.to_string()),
                src.show_rru_sized(mb_rru, ext_mode.src_size()),
                show_ireg_sized(dst.to_reg(), mb_rru, ext_mode.dst_size())
            ),

            Inst::MovRM { size, src, dst, .. } => format!(
                "{} {}, {}",
                ljustify2("mov".to_string(), suffix_bwlq(*size)),
                show_ireg_sized(*src, mb_rru, size.to_bytes()),
                dst.show_rru(mb_rru)
            ),

            Inst::ShiftR {
                size,
                kind,
                num_bits,
                dst,
                ..
            } => match num_bits {
                Imm8Reg::Reg { reg } => format!(
                    "{} {}, {}",
                    ljustify2(kind.to_string(), suffix_bwlq(*size)),
                    show_ireg_sized(*reg, mb_rru, 1),
                    show_ireg_sized(dst.to_reg(), mb_rru, size.to_bytes())
                ),

                Imm8Reg::Imm8 { imm: num_bits } => format!(
                    "{} ${}, {}",
                    ljustify2(kind.to_string(), suffix_bwlq(*size)),
                    num_bits,
                    show_ireg_sized(dst.to_reg(), mb_rru, size.to_bytes())
                ),
            },

            Inst::XmmRmiReg {
                opcode, src2, dst, ..
            } => format!(
                "{} {}, {}",
                ljustify(opcode.to_string()),
                src2.show_rru(mb_rru),
                dst.to_reg().show_rru(mb_rru)
            ),

            Inst::CmpRmiR {
                size,
                src,
                dst,
                opcode,
            } => {
                let op = match opcode {
                    CmpOpcode::Cmp => "cmp",
                    CmpOpcode::Test => "test",
                };
                format!(
                    "{} {}, {}",
                    ljustify2(op.to_string(), suffix_bwlq(*size)),
                    src.show_rru_sized(mb_rru, size.to_bytes()),
                    show_ireg_sized(*dst, mb_rru, size.to_bytes())
                )
            }

            Inst::Setcc { cc, dst } => format!(
                "{} {}",
                ljustify2("set".to_string(), cc.to_string()),
                show_ireg_sized(dst.to_reg(), mb_rru, 1)
            ),

            Inst::Cmove {
                size,
                cc,
                consequent: src,
                alternative: _,
                dst,
            } => format!(
                "{} {}, {}",
                ljustify(format!("cmov{}{}", cc.to_string(), suffix_bwlq(*size))),
                src.show_rru_sized(mb_rru, size.to_bytes()),
                show_ireg_sized(dst.to_reg(), mb_rru, size.to_bytes())
            ),

            Inst::XmmCmove { size, cc, src, dst } => {
                format!(
                    "j{} $next; mov{} {}, {}; $next: ",
                    cc.invert().to_string(),
                    if *size == OperandSize::Size64 {
                        "sd"
                    } else {
                        "ss"
                    },
                    src.show_rru_sized(mb_rru, size.to_bytes()),
                    show_ireg_sized(dst.to_reg(), mb_rru, size.to_bytes())
                )
            }

            Inst::Push64 { src } => {
                format!("{} {}", ljustify("pushq".to_string()), src.show_rru(mb_rru))
            }

            Inst::Pop64 { dst } => {
                format!("{} {}", ljustify("popq".to_string()), dst.show_rru(mb_rru))
            }

            Inst::CallKnown { dest, .. } => format!("{} {:?}", ljustify("call".to_string()), dest),

            Inst::CallUnknown { dest, .. } => format!(
                "{} *{}",
                ljustify("call".to_string()),
                dest.show_rru(mb_rru)
            ),

            Inst::Ret => "ret".to_string(),

            Inst::EpiloguePlaceholder => "epilogue placeholder".to_string(),

            Inst::JmpKnown { dst } => {
                format!("{} {}", ljustify("jmp".to_string()), dst.to_string())
            }

            Inst::JmpIf { cc, taken } => format!(
                "{} {}",
                ljustify2("j".to_string(), cc.to_string()),
                taken.to_string(),
            ),

            Inst::JmpCond {
                cc,
                taken,
                not_taken,
            } => format!(
                "{} {}; j {}",
                ljustify2("j".to_string(), cc.to_string()),
                taken.to_string(),
                not_taken.to_string()
            ),

            Inst::JmpTableSeq { idx, .. } => {
                format!("{} {}", ljustify("br_table".into()), idx.show_rru(mb_rru))
            }

            Inst::JmpUnknown { target } => format!(
                "{} *{}",
                ljustify("jmp".to_string()),
                target.show_rru(mb_rru)
            ),

            Inst::TrapIf { cc, trap_code, .. } => {
                format!("j{} ; ud2 {} ;", cc.invert().to_string(), trap_code)
            }

            Inst::LoadExtName {
                dst, name, offset, ..
            } => format!(
                "{} {}+{}, {}",
                ljustify("load_ext_name".into()),
                name,
                offset,
                show_ireg_sized(dst.to_reg(), mb_rru, 8),
            ),

            Inst::LockCmpxchg {
                ty,
                replacement,
                mem,
                ..
            } => {
                let size = ty.bytes() as u8;
                format!(
                    "lock cmpxchg{} {}, {}",
                    suffix_bwlq(OperandSize::from_bytes(size as u32)),
                    show_ireg_sized(*replacement, mb_rru, size),
                    mem.show_rru(mb_rru)
                )
            }

            Inst::AtomicRmwSeq { ty, op, .. } => {
                format!(
                    "atomically {{ {}_bits_at_[%r9]) {:?}= %r10; %rax = old_value_at_[%r9]; %r11, %rflags = trash }}",
                    ty.bits(), op)
            }

            Inst::Fence { kind } => match kind {
                FenceKind::MFence => "mfence".to_string(),
                FenceKind::LFence => "lfence".to_string(),
                FenceKind::SFence => "sfence".to_string(),
            },

            Inst::VirtualSPOffsetAdj { offset } => format!("virtual_sp_offset_adjust {}", offset),

            Inst::Hlt => "hlt".into(),

            Inst::Ud2 { trap_code } => format!("ud2 {}", trap_code),

            Inst::ElfTlsGetAddr { ref symbol } => {
                format!("elf_tls_get_addr {:?}", symbol)
            }

            Inst::MachOTlsGetAddr { ref symbol } => {
                format!("macho_tls_get_addr {:?}", symbol)
            }

            Inst::ValueLabelMarker { label, reg } => {
                format!("value_label {:?}, {}", label, reg.show_rru(mb_rru))
            }

            Inst::Unwind { inst } => {
                format!("unwind {:?}", inst)
            }
        }
    }
}

// Temp hook for legacy printing machinery
impl fmt::Debug for Inst {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        // Print the insn without a Universe :-(
        write!(fmt, "{}", self.show_rru(None))
    }
}

fn x64_get_regs(inst: &Inst, collector: &mut RegUsageCollector) {
    // This is a bit subtle. If some register is in the modified set, then it may not be in either
    // the use or def sets. However, enforcing that directly is somewhat difficult. Instead,
    // regalloc.rs will "fix" this for us by removing the modified set from the use and def
    // sets.
    match inst {
        Inst::AluRmiR {
            src1, src2, dst, ..
        } => {
            debug_assert_eq!(*src1, dst.to_reg());
            if inst.produces_const() {
                // No need to account for src2, since src2 == dst.
                collector.add_def(*dst);
            } else {
                src2.get_regs_as_uses(collector);
                collector.add_mod(*dst);
            }
        }
        Inst::Not { src, dst, .. } => {
            debug_assert_eq!(*src, dst.to_reg());
            collector.add_mod(*dst);
        }
        Inst::Neg { src, dst, .. } => {
            debug_assert_eq!(*src, dst.to_reg());
            collector.add_mod(*dst);
        }
        Inst::Div {
            size,
            divisor,
            dividend,
            dst_quotient,
            dst_remainder,
            ..
        } => {
            debug_assert_eq!(*dividend, regs::rax());
            debug_assert_eq!(dst_quotient.to_reg(), regs::rax());
            collector.add_mod(Writable::from_reg(regs::rax()));

            debug_assert_eq!(dst_remainder.to_reg(), regs::rdx());
            if *size == OperandSize::Size8 {
                collector.add_def(Writable::from_reg(regs::rdx()));
            } else {
                collector.add_mod(Writable::from_reg(regs::rdx()));
            }

            divisor.get_regs_as_uses(collector);
        }
        Inst::MulHi {
            src1,
            src2,
            dst_lo,
            dst_hi,
            ..
        } => {
            debug_assert_eq!(*src1, regs::rax());
            debug_assert_eq!(dst_lo.to_reg(), regs::rax());
            collector.add_mod(Writable::from_reg(regs::rax()));

            debug_assert_eq!(dst_hi.to_reg(), regs::rdx());
            collector.add_def(Writable::from_reg(regs::rdx()));

            src2.get_regs_as_uses(collector);
        }
        Inst::CheckedDivOrRemSeq {
            divisor,
            dividend,
            dst_quotient,
            dst_remainder,
            tmp,
            ..
        } => {
            debug_assert_eq!(*dividend, regs::rax());
            debug_assert_eq!(dst_quotient.to_reg(), regs::rax());
            debug_assert_eq!(dst_remainder.to_reg(), regs::rdx());
            // Mark both fixed registers as mods, to avoid an early clobber problem in codegen
            // (i.e. the temporary is allocated one of the fixed registers). This requires writing
            // the rdx register *before* the instruction, which is not too bad.
            collector.add_mod(Writable::from_reg(regs::rax()));
            collector.add_mod(Writable::from_reg(regs::rdx()));
            collector.add_mod(*divisor);
            if let Some(tmp) = tmp {
                collector.add_def(*tmp);
            }
        }
        Inst::SignExtendData { size, src, dst } => {
            debug_assert_eq!(*src, regs::rax());
            debug_assert_eq!(dst.to_reg(), regs::rdx());
            match size {
                OperandSize::Size8 => collector.add_mod(Writable::from_reg(regs::rax())),
                _ => {
                    collector.add_use(regs::rax());
                    collector.add_def(Writable::from_reg(regs::rdx()));
                }
            }
        }
        Inst::UnaryRmR { src, dst, .. }
        | Inst::XmmUnaryRmR { src, dst, .. }
        | Inst::XmmUnaryRmREvex { src, dst, .. } => {
            src.get_regs_as_uses(collector);
            collector.add_def(*dst);
        }
        Inst::XmmRmR {
            src1,
            src2,
            dst,
            op,
            ..
        } => {
            debug_assert_eq!(*src1, dst.to_reg());
            if inst.produces_const() {
                // No need to account for src, since src == dst.
                collector.add_def(*dst);
            } else {
                src2.get_regs_as_uses(collector);
                collector.add_mod(*dst);
                // Some instructions have an implicit use of XMM0.
                if *op == SseOpcode::Blendvpd
                    || *op == SseOpcode::Blendvps
                    || *op == SseOpcode::Pblendvb
                {
                    collector.add_use(regs::xmm0());
                }
            }
        }
        Inst::XmmRmREvex {
            op,
            src1,
            src2,
            dst,
            ..
        } => {
            src1.get_regs_as_uses(collector);
            collector.add_use(*src2);
            match *op {
                Avx512Opcode::Vpermi2b => collector.add_mod(*dst),
                _ => collector.add_def(*dst),
            }
        }
        Inst::XmmRmRImm {
            op,
            src1,
            src2,
            dst,
            ..
        } => {
            debug_assert_eq!(*src1, dst.to_reg());
            if inst.produces_const() {
                // No need to account for src2, since src2 == dst.
                debug_assert_eq!(src2.to_reg(), Some(dst.to_reg()));
                collector.add_def(*dst);
            } else if *op == SseOpcode::Pextrb
                || *op == SseOpcode::Pextrw
                || *op == SseOpcode::Pextrd
                || *op == SseOpcode::Pshufd
                || *op == SseOpcode::Roundss
                || *op == SseOpcode::Roundsd
                || *op == SseOpcode::Roundps
                || *op == SseOpcode::Roundpd
            {
                src2.get_regs_as_uses(collector);
                collector.add_def(*dst);
            } else {
                src2.get_regs_as_uses(collector);
                collector.add_mod(*dst);
            }
        }
        Inst::XmmUninitializedValue { dst } => collector.add_def(*dst),
        Inst::XmmLoadConst { dst, .. } => collector.add_def(*dst),
        Inst::XmmMinMaxSeq { lhs, rhs_dst, .. } => {
            collector.add_use(*lhs);
            collector.add_mod(*rhs_dst);
        }
        Inst::XmmRmiReg {
            src1, src2, dst, ..
        } => {
            debug_assert_eq!(*src1, dst.to_reg());
            src2.get_regs_as_uses(collector);
            collector.add_mod(*dst);
        }
        Inst::XmmMovRM { src, dst, .. } => {
            collector.add_use(*src);
            dst.get_regs_as_uses(collector);
        }
        Inst::XmmCmpRmR { src, dst, .. } => {
            src.get_regs_as_uses(collector);
            collector.add_use(*dst);
        }
        Inst::Imm { dst, .. } => {
            collector.add_def(*dst);
        }
        Inst::MovRR { src, dst, .. } | Inst::XmmToGpr { src, dst, .. } => {
            collector.add_use(*src);
            collector.add_def(*dst);
        }
        Inst::GprToXmm { src, dst, .. } => {
            src.get_regs_as_uses(collector);
            collector.add_def(*dst);
        }
        Inst::CvtUint64ToFloatSeq {
            src,
            dst,
            tmp_gpr1,
            tmp_gpr2,
            ..
        } => {
            collector.add_mod(*src);
            collector.add_def(*dst);
            collector.add_def(*tmp_gpr1);
            collector.add_def(*tmp_gpr2);
        }
        Inst::CvtFloatToSintSeq {
            src,
            dst,
            tmp_xmm,
            tmp_gpr,
            ..
        }
        | Inst::CvtFloatToUintSeq {
            src,
            dst,
            tmp_gpr,
            tmp_xmm,
            ..
        } => {
            collector.add_mod(*src);
            collector.add_def(*dst);
            collector.add_def(*tmp_gpr);
            collector.add_def(*tmp_xmm);
        }
        Inst::MovzxRmR { src, dst, .. } => {
            src.get_regs_as_uses(collector);
            collector.add_def(*dst);
        }
        Inst::Mov64MR { src, dst, .. } | Inst::LoadEffectiveAddress { addr: src, dst } => {
            src.get_regs_as_uses(collector);
            collector.add_def(*dst)
        }
        Inst::MovsxRmR { src, dst, .. } => {
            src.get_regs_as_uses(collector);
            collector.add_def(*dst);
        }
        Inst::MovRM { src, dst, .. } => {
            collector.add_use(*src);
            dst.get_regs_as_uses(collector);
        }
        Inst::ShiftR { num_bits, dst, .. } => {
            if let Imm8Reg::Reg { reg } = num_bits {
                debug_assert_eq!(*reg, regs::rcx());
                collector.add_use(regs::rcx());
            }
            collector.add_mod(*dst);
        }
        Inst::CmpRmiR { src, dst, .. } => {
            src.get_regs_as_uses(collector);
            collector.add_use(*dst); // yes, really `add_use`
        }
        Inst::Setcc { dst, .. } => {
            collector.add_def(*dst);
        }
        Inst::Cmove {
            consequent: src,
            dst,
            ..
        }
        | Inst::XmmCmove { src, dst, .. } => {
            src.get_regs_as_uses(collector);
            collector.add_mod(*dst);
        }
        Inst::Push64 { src } => {
            src.get_regs_as_uses(collector);
            collector.add_mod(Writable::from_reg(regs::rsp()));
        }
        Inst::Pop64 { dst } => {
            collector.add_def(*dst);
        }

        Inst::CallKnown {
            ref uses, ref defs, ..
        } => {
            collector.add_uses(uses);
            collector.add_defs(defs);
        }

        Inst::CallUnknown {
            ref uses,
            ref defs,
            dest,
            ..
        } => {
            collector.add_uses(uses);
            collector.add_defs(defs);
            dest.get_regs_as_uses(collector);
        }

        Inst::JmpTableSeq {
            ref idx,
            ref tmp1,
            ref tmp2,
            ..
        } => {
            collector.add_use(*idx);
            collector.add_def(*tmp1);
            collector.add_def(*tmp2);
        }

        Inst::JmpUnknown { target } => {
            target.get_regs_as_uses(collector);
        }

        Inst::LoadExtName { dst, .. } => {
            collector.add_def(*dst);
        }

        Inst::LockCmpxchg {
            replacement,
            expected,
            mem,
            dst_old,
            ..
        } => {
            mem.get_regs_as_uses(collector);
            collector.add_use(*replacement);

            debug_assert_eq!(*expected, regs::rax());
            debug_assert_eq!(dst_old.to_reg(), regs::rax());
            collector.add_mod(Writable::from_reg(regs::rax()));
        }

        Inst::AtomicRmwSeq { .. } => {
            collector.add_use(regs::r9());
            collector.add_use(regs::r10());
            collector.add_def(Writable::from_reg(regs::r11()));
            collector.add_def(Writable::from_reg(regs::rax()));
        }

        Inst::Ret
        | Inst::EpiloguePlaceholder
        | Inst::JmpKnown { .. }
        | Inst::JmpIf { .. }
        | Inst::JmpCond { .. }
        | Inst::Nop { .. }
        | Inst::TrapIf { .. }
        | Inst::VirtualSPOffsetAdj { .. }
        | Inst::Hlt
        | Inst::Ud2 { .. }
        | Inst::Fence { .. } => {
            // No registers are used.
        }

        Inst::ElfTlsGetAddr { .. } | Inst::MachOTlsGetAddr { .. } => {
            // All caller-saves are clobbered.
            //
            // We use the SysV calling convention here because the
            // pseudoinstruction (and relocation that it emits) is specific to
            // ELF systems; other x86-64 targets with other conventions (i.e.,
            // Windows) use different TLS strategies.
            for reg in X64ABIMachineSpec::get_regs_clobbered_by_call(CallConv::SystemV) {
                collector.add_def(reg);
            }
        }

        Inst::ValueLabelMarker { reg, .. } => {
            collector.add_use(*reg);
        }

        Inst::Unwind { .. } => {}
    }
}

//=============================================================================
// Instructions and subcomponents: map_regs

// Define our own register-mapping trait so we can do arbitrary register
// renaming that are more free form than what `regalloc` constrains us to with
// its `RegUsageMapper` trait definition.
pub trait RegMapper {
    fn get_use(&self, reg: Reg) -> Option<Reg>;
    fn get_def(&self, reg: Reg) -> Option<Reg>;
    fn get_mod(&self, reg: Reg) -> Option<Reg>;
}

impl<T> RegMapper for T
where
    T: regalloc::RegUsageMapper,
{
    fn get_use(&self, reg: Reg) -> Option<Reg> {
        let v = reg.as_virtual_reg()?;
        self.get_use(v).map(|r| r.to_reg())
    }

    fn get_def(&self, reg: Reg) -> Option<Reg> {
        let v = reg.as_virtual_reg()?;
        self.get_def(v).map(|r| r.to_reg())
    }

    fn get_mod(&self, reg: Reg) -> Option<Reg> {
        let v = reg.as_virtual_reg()?;
        self.get_mod(v).map(|r| r.to_reg())
    }
}

fn map_use<RM: RegMapper>(m: &RM, r: &mut Reg) {
    if let Some(new) = m.get_use(*r) {
        *r = new;
    }
}

fn map_def<RM: RegMapper>(m: &RM, r: &mut Writable<Reg>) {
    if let Some(new) = m.get_def(r.to_reg()) {
        *r = Writable::from_reg(new);
    }
}

fn map_mod<RM: RegMapper>(m: &RM, r: &mut Writable<Reg>) {
    if let Some(new) = m.get_mod(r.to_reg()) {
        *r = Writable::from_reg(new);
    }
}

impl Amode {
    fn map_uses<RM: RegMapper>(&mut self, map: &RM) {
        match self {
            Amode::ImmReg { ref mut base, .. } => map_use(map, base),
            Amode::ImmRegRegShift {
                ref mut base,
                ref mut index,
                ..
            } => {
                map_use(map, base);
                map_use(map, index);
            }
            Amode::RipRelative { .. } => {
                // RIP isn't involved in regalloc.
            }
        }
    }

    /// Offset the amode by a fixed offset.
    pub(crate) fn offset(&self, offset: u32) -> Self {
        let mut ret = self.clone();
        match &mut ret {
            &mut Amode::ImmReg { ref mut simm32, .. } => *simm32 += offset,
            &mut Amode::ImmRegRegShift { ref mut simm32, .. } => *simm32 += offset,
            _ => panic!("Cannot offset amode: {:?}", self),
        }
        ret
    }
}

impl RegMemImm {
    fn map_uses<RM: RegMapper>(&mut self, map: &RM) {
        match self {
            RegMemImm::Reg { ref mut reg } => map_use(map, reg),
            RegMemImm::Mem { ref mut addr } => addr.map_uses(map),
            RegMemImm::Imm { .. } => {}
        }
    }

    fn map_as_def<RM: RegMapper>(&mut self, mapper: &RM) {
        match self {
            Self::Reg { reg } => {
                let mut writable_src = Writable::from_reg(*reg);
                map_def(mapper, &mut writable_src);
                *self = Self::reg(writable_src.to_reg());
            }
            _ => panic!("unexpected RegMemImm kind in map_src_reg_as_def"),
        }
    }
}

impl RegMem {
    fn map_uses<RM: RegMapper>(&mut self, map: &RM) {
        match self {
            RegMem::Reg { ref mut reg } => map_use(map, reg),
            RegMem::Mem { ref mut addr, .. } => addr.map_uses(map),
        }
    }

    fn map_as_def<RM: RegMapper>(&mut self, mapper: &RM) {
        match self {
            Self::Reg { reg } => {
                let mut writable_src = Writable::from_reg(*reg);
                map_def(mapper, &mut writable_src);
                *self = Self::reg(writable_src.to_reg());
            }
            _ => panic!("unexpected RegMem kind in map_src_reg_as_def"),
        }
    }
}

pub(crate) fn x64_map_regs<RM: RegMapper>(inst: &mut Inst, mapper: &RM) {
    // Note this must be carefully synchronized with x64_get_regs.
    let produces_const = inst.produces_const();

    match inst {
        // ** Nop
        Inst::AluRmiR {
            ref mut src1,
            ref mut src2,
            ref mut dst,
            ..
        } => {
            debug_assert_eq!(*src1, dst.to_reg());
            if produces_const {
                src2.map_as_def(mapper);
                map_def(mapper, dst);
                *src1 = dst.to_reg();
            } else {
                src2.map_uses(mapper);
                map_mod(mapper, dst);
                *src1 = dst.to_reg();
            }
        }
        Inst::Not { src, dst, .. } | Inst::Neg { src, dst, .. } => {
            debug_assert_eq!(*src, dst.to_reg());
            map_mod(mapper, dst);
            *src = dst.to_reg();
        }
        Inst::Div { divisor, .. } => divisor.map_uses(mapper),
        Inst::MulHi { src2, .. } => src2.map_uses(mapper),
        Inst::CheckedDivOrRemSeq { divisor, tmp, .. } => {
            map_mod(mapper, divisor);
            if let Some(tmp) = tmp {
                map_def(mapper, tmp)
            }
        }
        Inst::SignExtendData { .. } => {}
        Inst::XmmUnaryRmR {
            ref mut src,
            ref mut dst,
            ..
        }
        | Inst::XmmUnaryRmREvex {
            ref mut src,
            ref mut dst,
            ..
        }
        | Inst::UnaryRmR {
            ref mut src,
            ref mut dst,
            ..
        } => {
            src.map_uses(mapper);
            map_def(mapper, dst);
        }
        Inst::XmmRmRImm {
            ref op,
            ref mut src1,
            ref mut src2,
            ref mut dst,
            ..
        } => {
            debug_assert_eq!(*src1, dst.to_reg());
            if produces_const {
                src2.map_as_def(mapper);
                map_def(mapper, dst);
                *src1 = dst.to_reg();
            } else if *op == SseOpcode::Pextrb
                || *op == SseOpcode::Pextrw
                || *op == SseOpcode::Pextrd
                || *op == SseOpcode::Pshufd
                || *op == SseOpcode::Roundss
                || *op == SseOpcode::Roundsd
                || *op == SseOpcode::Roundps
                || *op == SseOpcode::Roundpd
            {
                src2.map_uses(mapper);
                map_def(mapper, dst);
                *src1 = dst.to_reg();
            } else {
                src2.map_uses(mapper);
                map_mod(mapper, dst);
                *src1 = dst.to_reg();
            }
        }
        Inst::XmmRmR {
            ref mut src1,
            ref mut src2,
            ref mut dst,
            ..
        } => {
            debug_assert_eq!(*src1, dst.to_reg());
            if produces_const {
                src2.map_as_def(mapper);
                map_def(mapper, dst);
                *src1 = dst.to_reg();
            } else {
                src2.map_uses(mapper);
                map_mod(mapper, dst);
                *src1 = dst.to_reg();
            }
        }
        Inst::XmmRmREvex {
            op,
            ref mut src1,
            ref mut src2,
            ref mut dst,
            ..
        } => {
            src1.map_uses(mapper);
            map_use(mapper, src2);
            match *op {
                Avx512Opcode::Vpermi2b => map_mod(mapper, dst),
                _ => map_def(mapper, dst),
            }
        }
        Inst::XmmRmiReg {
            ref mut src1,
            ref mut src2,
            ref mut dst,
            ..
        } => {
            debug_assert_eq!(*src1, dst.to_reg());
            src2.map_uses(mapper);
            map_mod(mapper, dst);
            *src1 = dst.to_reg();
        }
        Inst::XmmUninitializedValue { ref mut dst, .. } => {
            map_def(mapper, dst);
        }
        Inst::XmmLoadConst { ref mut dst, .. } => {
            map_def(mapper, dst);
        }
        Inst::XmmMinMaxSeq {
            ref mut lhs,
            ref mut rhs_dst,
            ..
        } => {
            map_use(mapper, lhs);
            map_mod(mapper, rhs_dst);
        }
        Inst::XmmMovRM {
            ref mut src,
            ref mut dst,
            ..
        } => {
            map_use(mapper, src);
            dst.map_uses(mapper);
        }
        Inst::XmmCmpRmR {
            ref mut src,
            ref mut dst,
            ..
        } => {
            src.map_uses(mapper);
            map_use(mapper, dst);
        }
        Inst::Imm { ref mut dst, .. } => map_def(mapper, dst),
        Inst::MovRR {
            ref mut src,
            ref mut dst,
            ..
        }
        | Inst::XmmToGpr {
            ref mut src,
            ref mut dst,
            ..
        } => {
            map_use(mapper, src);
            map_def(mapper, dst);
        }
        Inst::GprToXmm {
            ref mut src,
            ref mut dst,
            ..
        } => {
            src.map_uses(mapper);
            map_def(mapper, dst);
        }
        Inst::CvtUint64ToFloatSeq {
            ref mut src,
            ref mut dst,
            ref mut tmp_gpr1,
            ref mut tmp_gpr2,
            ..
        } => {
            map_mod(mapper, src);
            map_def(mapper, dst);
            map_def(mapper, tmp_gpr1);
            map_def(mapper, tmp_gpr2);
        }
        Inst::CvtFloatToSintSeq {
            ref mut src,
            ref mut dst,
            ref mut tmp_xmm,
            ref mut tmp_gpr,
            ..
        }
        | Inst::CvtFloatToUintSeq {
            ref mut src,
            ref mut dst,
            ref mut tmp_gpr,
            ref mut tmp_xmm,
            ..
        } => {
            map_mod(mapper, src);
            map_def(mapper, dst);
            map_def(mapper, tmp_gpr);
            map_def(mapper, tmp_xmm);
        }
        Inst::MovzxRmR {
            ref mut src,
            ref mut dst,
            ..
        } => {
            src.map_uses(mapper);
            map_def(mapper, dst);
        }
        Inst::Mov64MR { src, dst, .. } | Inst::LoadEffectiveAddress { addr: src, dst } => {
            src.map_uses(mapper);
            map_def(mapper, dst);
        }
        Inst::MovsxRmR {
            ref mut src,
            ref mut dst,
            ..
        } => {
            src.map_uses(mapper);
            map_def(mapper, dst);
        }
        Inst::MovRM {
            ref mut src,
            ref mut dst,
            ..
        } => {
            map_use(mapper, src);
            dst.map_uses(mapper);
        }
        Inst::ShiftR {
            ref mut src,
            ref mut dst,
            ..
        } => {
            debug_assert_eq!(*src, dst.to_reg());
            map_mod(mapper, dst);
            *src = dst.to_reg();
        }
        Inst::CmpRmiR {
            ref mut src,
            ref mut dst,
            ..
        } => {
            src.map_uses(mapper);
            map_use(mapper, dst);
        }
        Inst::Setcc { ref mut dst, .. } => map_def(mapper, dst),
        Inst::Cmove {
            consequent: ref mut src,
            ref mut dst,
            ref mut alternative,
            ..
        } => {
            src.map_uses(mapper);
            map_mod(mapper, dst);
            *alternative = dst.to_reg();
        }
        Inst::XmmCmove {
            ref mut src,
            ref mut dst,
            ..
        } => {
            src.map_uses(mapper);
            map_mod(mapper, dst);
        }
        Inst::Push64 { ref mut src } => src.map_uses(mapper),
        Inst::Pop64 { ref mut dst } => {
            map_def(mapper, dst);
        }

        Inst::CallKnown {
            ref mut uses,
            ref mut defs,
            ..
        } => {
            for r in uses.iter_mut() {
                map_use(mapper, r);
            }
            for r in defs.iter_mut() {
                map_def(mapper, r);
            }
        }

        Inst::CallUnknown {
            ref mut uses,
            ref mut defs,
            ref mut dest,
            ..
        } => {
            for r in uses.iter_mut() {
                map_use(mapper, r);
            }
            for r in defs.iter_mut() {
                map_def(mapper, r);
            }
            dest.map_uses(mapper);
        }

        Inst::JmpTableSeq {
            ref mut idx,
            ref mut tmp1,
            ref mut tmp2,
            ..
        } => {
            map_use(mapper, idx);
            map_def(mapper, tmp1);
            map_def(mapper, tmp2);
        }

        Inst::JmpUnknown { ref mut target } => target.map_uses(mapper),

        Inst::LoadExtName { ref mut dst, .. } => map_def(mapper, dst),

        Inst::LockCmpxchg {
            ref mut replacement,
            ref mut mem,
            ..
        } => {
            map_use(mapper, replacement);
            mem.map_uses(mapper);
        }

        Inst::ValueLabelMarker { ref mut reg, .. } => map_use(mapper, reg),

        Inst::Ret
        | Inst::EpiloguePlaceholder
        | Inst::JmpKnown { .. }
        | Inst::JmpCond { .. }
        | Inst::JmpIf { .. }
        | Inst::Nop { .. }
        | Inst::TrapIf { .. }
        | Inst::VirtualSPOffsetAdj { .. }
        | Inst::Ud2 { .. }
        | Inst::Hlt
        | Inst::AtomicRmwSeq { .. }
        | Inst::ElfTlsGetAddr { .. }
        | Inst::MachOTlsGetAddr { .. }
        | Inst::Fence { .. }
        | Inst::Unwind { .. } => {
            // Instruction doesn't explicitly mention any regs, so it can't have any virtual
            // regs that we'd need to remap.  Hence no action required.
        }
    }
}

//=============================================================================
// Instructions: misc functions and external interface

impl MachInst for Inst {
    fn get_regs(&self, collector: &mut RegUsageCollector) {
        x64_get_regs(&self, collector)
    }

    fn map_regs<RUM>(&mut self, mapper: &RUM)
    where
        RUM: regalloc::RegUsageMapper,
    {
        x64_map_regs(self, mapper);
    }

    fn is_move(&self) -> Option<(Writable<Reg>, Reg)> {
        match self {
            // Note (carefully!) that a 32-bit mov *isn't* a no-op since it zeroes
            // out the upper 32 bits of the destination.  For example, we could
            // conceivably use `movl %reg, %reg` to zero out the top 32 bits of
            // %reg.
            Self::MovRR { size, src, dst, .. } if *size == OperandSize::Size64 => {
                Some((*dst, *src))
            }
            // Note as well that MOVS[S|D] when used in the `XmmUnaryRmR` context are pure moves of
            // scalar floating-point values (and annotate `dst` as `def`s to the register allocator)
            // whereas the same operation in a packed context, e.g. `XMM_RM_R`, is used to merge a
            // value into the lowest lane of a vector (not a move).
            Self::XmmUnaryRmR { op, src, dst, .. }
                if *op == SseOpcode::Movss
                    || *op == SseOpcode::Movsd
                    || *op == SseOpcode::Movaps
                    || *op == SseOpcode::Movapd
                    || *op == SseOpcode::Movups
                    || *op == SseOpcode::Movupd
                    || *op == SseOpcode::Movdqa
                    || *op == SseOpcode::Movdqu =>
            {
                if let RegMem::Reg { reg } = src {
                    Some((*dst, *reg))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn is_epilogue_placeholder(&self) -> bool {
        if let Self::EpiloguePlaceholder = self {
            true
        } else {
            false
        }
    }

    fn is_term<'a>(&'a self) -> MachTerminator<'a> {
        match self {
            // Interesting cases.
            &Self::Ret | &Self::EpiloguePlaceholder => MachTerminator::Ret,
            &Self::JmpKnown { dst } => MachTerminator::Uncond(dst),
            &Self::JmpCond {
                taken, not_taken, ..
            } => MachTerminator::Cond(taken, not_taken),
            &Self::JmpTableSeq {
                ref targets_for_term,
                ..
            } => MachTerminator::Indirect(&targets_for_term[..]),
            // All other cases are boring.
            _ => MachTerminator::None,
        }
    }

    fn stack_op_info(&self) -> Option<MachInstStackOpInfo> {
        match self {
            Self::VirtualSPOffsetAdj { offset } => Some(MachInstStackOpInfo::NomSPAdj(*offset)),
            Self::MovRM {
                size: OperandSize::Size8,
                src,
                dst: SyntheticAmode::NominalSPOffset { simm32 },
            } => Some(MachInstStackOpInfo::StoreNomSPOff(*src, *simm32 as i64)),
            Self::Mov64MR {
                src: SyntheticAmode::NominalSPOffset { simm32 },
                dst,
            } => Some(MachInstStackOpInfo::LoadNomSPOff(
                dst.to_reg(),
                *simm32 as i64,
            )),
            _ => None,
        }
    }

    fn gen_move(dst_reg: Writable<Reg>, src_reg: Reg, ty: Type) -> Inst {
        let rc_dst = dst_reg.to_reg().get_class();
        let rc_src = src_reg.get_class();
        // If this isn't true, we have gone way off the rails.
        debug_assert!(rc_dst == rc_src);
        match rc_dst {
            RegClass::I64 => Inst::mov_r_r(OperandSize::Size64, src_reg, dst_reg),
            RegClass::V128 => {
                // The Intel optimization manual, in "3.5.1.13 Zero-Latency MOV Instructions",
                // doesn't include MOVSS/MOVSD as instructions with zero-latency. Use movaps for
                // those, which may write more lanes that we need, but are specified to have
                // zero-latency.
                let opcode = match ty {
                    types::F32 | types::F64 | types::F32X4 => SseOpcode::Movaps,
                    types::F64X2 => SseOpcode::Movapd,
                    _ if ty.is_vector() && ty.bits() == 128 => SseOpcode::Movdqa,
                    _ => unimplemented!("unable to move type: {}", ty),
                };
                Inst::xmm_unary_rm_r(opcode, RegMem::reg(src_reg), dst_reg)
            }
            _ => panic!("gen_move(x64): unhandled regclass {:?}", rc_dst),
        }
    }

    fn gen_nop(preferred_size: usize) -> Inst {
        Inst::nop(std::cmp::min(preferred_size, 15) as u8)
    }

    fn maybe_direct_reload(&self, _reg: VirtualReg, _slot: SpillSlot) -> Option<Inst> {
        None
    }

    fn rc_for_type(ty: Type) -> CodegenResult<(&'static [RegClass], &'static [Type])> {
        match ty {
            types::I8 => Ok((&[RegClass::I64], &[types::I8])),
            types::I16 => Ok((&[RegClass::I64], &[types::I16])),
            types::I32 => Ok((&[RegClass::I64], &[types::I32])),
            types::I64 => Ok((&[RegClass::I64], &[types::I64])),
            types::B1 => Ok((&[RegClass::I64], &[types::B1])),
            types::B8 => Ok((&[RegClass::I64], &[types::B8])),
            types::B16 => Ok((&[RegClass::I64], &[types::B16])),
            types::B32 => Ok((&[RegClass::I64], &[types::B32])),
            types::B64 => Ok((&[RegClass::I64], &[types::B64])),
            types::R32 => panic!("32-bit reftype pointer should never be seen on x86-64"),
            types::R64 => Ok((&[RegClass::I64], &[types::R64])),
            types::F32 => Ok((&[RegClass::V128], &[types::F32])),
            types::F64 => Ok((&[RegClass::V128], &[types::F64])),
            types::I128 => Ok((&[RegClass::I64, RegClass::I64], &[types::I64, types::I64])),
            types::B128 => Ok((&[RegClass::I64, RegClass::I64], &[types::B64, types::B64])),
            _ if ty.is_vector() => {
                assert!(ty.bits() <= 128);
                Ok((&[RegClass::V128], &[types::I8X16]))
            }
            types::IFLAGS | types::FFLAGS => Ok((&[RegClass::I64], &[types::I64])),
            _ => Err(CodegenError::Unsupported(format!(
                "Unexpected SSA-value type: {}",
                ty
            ))),
        }
    }

    fn gen_jump(label: MachLabel) -> Inst {
        Inst::jmp_known(label)
    }

    fn gen_constant<F: FnMut(Type) -> Writable<Reg>>(
        to_regs: ValueRegs<Writable<Reg>>,
        value: u128,
        ty: Type,
        mut alloc_tmp: F,
    ) -> SmallVec<[Self; 4]> {
        let mut ret = SmallVec::new();
        if ty == types::I128 {
            let lo = value as u64;
            let hi = (value >> 64) as u64;
            let lo_reg = to_regs.regs()[0];
            let hi_reg = to_regs.regs()[1];
            if lo == 0 {
                ret.push(Inst::alu_rmi_r(
                    OperandSize::Size64,
                    AluRmiROpcode::Xor,
                    RegMemImm::reg(lo_reg.to_reg()),
                    lo_reg,
                ));
            } else {
                ret.push(Inst::imm(OperandSize::Size64, lo, lo_reg));
            }
            if hi == 0 {
                ret.push(Inst::alu_rmi_r(
                    OperandSize::Size64,
                    AluRmiROpcode::Xor,
                    RegMemImm::reg(hi_reg.to_reg()),
                    hi_reg,
                ));
            } else {
                ret.push(Inst::imm(OperandSize::Size64, hi, hi_reg));
            }
        } else {
            let to_reg = to_regs
                .only_reg()
                .expect("multi-reg values not supported on x64");
            if ty == types::F32 {
                if value == 0 {
                    ret.push(Inst::xmm_rm_r(
                        SseOpcode::Xorps,
                        RegMem::reg(to_reg.to_reg()),
                        to_reg,
                    ));
                } else {
                    let tmp = alloc_tmp(types::I32);
                    ret.push(Inst::imm(OperandSize::Size32, value as u64, tmp));

                    ret.push(Inst::gpr_to_xmm(
                        SseOpcode::Movd,
                        RegMem::reg(tmp.to_reg()),
                        OperandSize::Size32,
                        to_reg,
                    ));
                }
            } else if ty == types::F64 {
                if value == 0 {
                    ret.push(Inst::xmm_rm_r(
                        SseOpcode::Xorpd,
                        RegMem::reg(to_reg.to_reg()),
                        to_reg,
                    ));
                } else {
                    let tmp = alloc_tmp(types::I64);
                    ret.push(Inst::imm(OperandSize::Size64, value as u64, tmp));

                    ret.push(Inst::gpr_to_xmm(
                        SseOpcode::Movq,
                        RegMem::reg(tmp.to_reg()),
                        OperandSize::Size64,
                        to_reg,
                    ));
                }
            } else {
                // Must be an integer type.
                debug_assert!(
                    ty == types::B1
                        || ty == types::I8
                        || ty == types::B8
                        || ty == types::I16
                        || ty == types::B16
                        || ty == types::I32
                        || ty == types::B32
                        || ty == types::I64
                        || ty == types::B64
                        || ty == types::R32
                        || ty == types::R64
                );
                // Immediates must be 32 or 64 bits.
                // Smaller types are widened.
                let size = match OperandSize::from_ty(ty) {
                    OperandSize::Size64 => OperandSize::Size64,
                    _ => OperandSize::Size32,
                };
                if value == 0 {
                    ret.push(Inst::alu_rmi_r(
                        size,
                        AluRmiROpcode::Xor,
                        RegMemImm::reg(to_reg.to_reg()),
                        to_reg,
                    ));
                } else {
                    let value = value as u64;
                    ret.push(Inst::imm(size, value.into(), to_reg));
                }
            }
        }
        ret
    }

    fn reg_universe(flags: &Flags) -> RealRegUniverse {
        create_reg_universe_systemv(flags)
    }

    fn worst_case_size() -> CodeOffset {
        15
    }

    fn ref_type_regclass(_: &settings::Flags) -> RegClass {
        RegClass::I64
    }

    fn gen_value_label_marker(label: ValueLabel, reg: Reg) -> Self {
        Inst::ValueLabelMarker { label, reg }
    }

    fn defines_value_label(&self) -> Option<(ValueLabel, Reg)> {
        match self {
            Inst::ValueLabelMarker { label, reg } => Some((*label, *reg)),
            _ => None,
        }
    }

    type LabelUse = LabelUse;
}

/// State carried between emissions of a sequence of instructions.
#[derive(Default, Clone, Debug)]
pub struct EmitState {
    /// Addend to convert nominal-SP offsets to real-SP offsets at the current
    /// program point.
    pub(crate) virtual_sp_offset: i64,
    /// Offset of FP from nominal-SP.
    pub(crate) nominal_sp_to_fp: i64,
    /// Safepoint stack map for upcoming instruction, as provided to `pre_safepoint()`.
    stack_map: Option<StackMap>,
    /// Current source location.
    cur_srcloc: SourceLoc,
}

/// Constant state used during emissions of a sequence of instructions.
pub struct EmitInfo {
    flags: settings::Flags,
    isa_flags: x64_settings::Flags,
}

impl EmitInfo {
    pub(crate) fn new(flags: settings::Flags, isa_flags: x64_settings::Flags) -> Self {
        Self { flags, isa_flags }
    }
}

impl MachInstEmitInfo for EmitInfo {
    fn flags(&self) -> &Flags {
        &self.flags
    }
}

impl MachInstEmit for Inst {
    type State = EmitState;
    type Info = EmitInfo;

    fn emit(&self, sink: &mut MachBuffer<Inst>, info: &Self::Info, state: &mut Self::State) {
        emit::emit(self, sink, info, state);
    }

    fn pretty_print(&self, mb_rru: Option<&RealRegUniverse>, _: &mut Self::State) -> String {
        self.show_rru(mb_rru)
    }
}

impl MachInstEmitState<Inst> for EmitState {
    fn new(abi: &dyn ABICallee<I = Inst>) -> Self {
        EmitState {
            virtual_sp_offset: 0,
            nominal_sp_to_fp: abi.frame_size() as i64,
            stack_map: None,
            cur_srcloc: SourceLoc::default(),
        }
    }

    fn pre_safepoint(&mut self, stack_map: StackMap) {
        self.stack_map = Some(stack_map);
    }

    fn pre_sourceloc(&mut self, srcloc: SourceLoc) {
        self.cur_srcloc = srcloc;
    }
}

impl EmitState {
    fn take_stack_map(&mut self) -> Option<StackMap> {
        self.stack_map.take()
    }

    fn clear_post_insn(&mut self) {
        self.stack_map = None;
    }

    pub(crate) fn cur_srcloc(&self) -> SourceLoc {
        self.cur_srcloc
    }
}

/// A label-use (internal relocation) in generated code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelUse {
    /// A 32-bit offset from location of relocation itself, added to the existing value at that
    /// location. Used for control flow instructions which consider an offset from the start of the
    /// next instruction (so the size of the payload -- 4 bytes -- is subtracted from the payload).
    JmpRel32,

    /// A 32-bit offset from location of relocation itself, added to the existing value at that
    /// location.
    PCRel32,
}

impl MachInstLabelUse for LabelUse {
    const ALIGN: CodeOffset = 1;

    fn max_pos_range(self) -> CodeOffset {
        match self {
            LabelUse::JmpRel32 | LabelUse::PCRel32 => 0x7fff_ffff,
        }
    }

    fn max_neg_range(self) -> CodeOffset {
        match self {
            LabelUse::JmpRel32 | LabelUse::PCRel32 => 0x8000_0000,
        }
    }

    fn patch_size(self) -> CodeOffset {
        match self {
            LabelUse::JmpRel32 | LabelUse::PCRel32 => 4,
        }
    }

    fn patch(self, buffer: &mut [u8], use_offset: CodeOffset, label_offset: CodeOffset) {
        let pc_rel = (label_offset as i64) - (use_offset as i64);
        debug_assert!(pc_rel <= self.max_pos_range() as i64);
        debug_assert!(pc_rel >= -(self.max_neg_range() as i64));
        let pc_rel = pc_rel as u32;
        match self {
            LabelUse::JmpRel32 => {
                let addend = u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
                let value = pc_rel.wrapping_add(addend).wrapping_sub(4);
                buffer.copy_from_slice(&value.to_le_bytes()[..]);
            }
            LabelUse::PCRel32 => {
                let addend = u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
                let value = pc_rel.wrapping_add(addend);
                buffer.copy_from_slice(&value.to_le_bytes()[..]);
            }
        }
    }

    fn supports_veneer(self) -> bool {
        match self {
            LabelUse::JmpRel32 | LabelUse::PCRel32 => false,
        }
    }

    fn veneer_size(self) -> CodeOffset {
        match self {
            LabelUse::JmpRel32 | LabelUse::PCRel32 => 0,
        }
    }

    fn generate_veneer(self, _: &mut [u8], _: CodeOffset) -> (CodeOffset, LabelUse) {
        match self {
            LabelUse::JmpRel32 | LabelUse::PCRel32 => {
                panic!("Veneer not supported for JumpRel32 label-use.");
            }
        }
    }

    fn from_reloc(reloc: Reloc, addend: Addend) -> Option<Self> {
        match (reloc, addend) {
            (Reloc::X86CallPCRel4, -4) => Some(LabelUse::JmpRel32),
            _ => None,
        }
    }
}
