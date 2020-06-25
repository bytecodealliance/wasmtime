//! This module defines x86_64-specific machine instruction types.

#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use alloc::vec::Vec;
use smallvec::SmallVec;
use std::fmt;
use std::string::{String, ToString};

use regalloc::RegUsageCollector;
use regalloc::{RealRegUniverse, Reg, RegClass, RegUsageMapper, SpillSlot, VirtualReg, Writable};

use crate::binemit::CodeOffset;
use crate::ir::types::{B1, B128, B16, B32, B64, B8, F32, F64, I128, I16, I32, I64, I8};
use crate::ir::{ExternalName, Opcode, SourceLoc, TrapCode, Type};
use crate::machinst::*;
use crate::settings::Flags;
use crate::{settings, CodegenError, CodegenResult};

pub mod args;
mod emit;
#[cfg(test)]
mod emit_tests;
pub mod regs;

use args::*;
use regs::{create_reg_universe_systemv, show_ireg_sized};

//=============================================================================
// Instructions (top level): definition

// Don't build these directly.  Instead use the Inst:: functions to create them.

/// Instructions.  Destinations are on the RIGHT (a la AT&T syntax).
#[derive(Clone)]
pub enum Inst {
    /// nops of various sizes, including zero
    Nop { len: u8 },

    // =====================================
    // Integer instructions.
    /// Integer arithmetic/bit-twiddling: (add sub and or xor mul adc? sbb?) (32 64) (reg addr imm) reg
    Alu_RMI_R {
        is_64: bool,
        op: AluRmiROpcode,
        src: RegMemImm,
        dst: Writable<Reg>,
    },

    /// Constant materialization: (imm32 imm64) reg.
    /// Either: movl $imm32, %reg32 or movabsq $imm64, %reg32.
    Imm_R {
        dst_is_64: bool,
        simm64: u64,
        dst: Writable<Reg>,
    },

    /// GPR to GPR move: mov (64 32) reg reg.
    Mov_R_R {
        is_64: bool,
        src: Reg,
        dst: Writable<Reg>,
    },

    /// Zero-extended loads, except for 64 bits: movz (bl bq wl wq lq) addr reg.
    /// Note that the lq variant doesn't really exist since the default zero-extend rule makes it
    /// unnecessary. For that case we emit the equivalent "movl AM, reg32".
    MovZX_RM_R {
        ext_mode: ExtMode,
        src: RegMem,
        dst: Writable<Reg>,
    },

    /// A plain 64-bit integer load, since MovZX_RM_R can't represent that.
    Mov64_M_R {
        src: SyntheticAmode,
        dst: Writable<Reg>,
    },

    /// Loads the memory address of addr into dst.
    LoadEffectiveAddress {
        addr: SyntheticAmode,
        dst: Writable<Reg>,
    },

    /// Sign-extended loads and moves: movs (bl bq wl wq lq) addr reg.
    MovSX_RM_R {
        ext_mode: ExtMode,
        src: RegMem,
        dst: Writable<Reg>,
    },

    /// Integer stores: mov (b w l q) reg addr.
    Mov_R_M {
        size: u8, // 1, 2, 4 or 8.
        src: Reg,
        dst: SyntheticAmode,
    },

    /// Arithmetic shifts: (shl shr sar) (l q) imm reg.
    Shift_R {
        is_64: bool,
        kind: ShiftKind,
        /// shift count: Some(0 .. #bits-in-type - 1), or None to mean "%cl".
        num_bits: Option<u8>,
        dst: Writable<Reg>,
    },

    /// Integer comparisons/tests: cmp (b w l q) (reg addr imm) reg.
    Cmp_RMI_R {
        size: u8, // 1, 2, 4 or 8
        src: RegMemImm,
        dst: Reg,
    },

    /// Materializes the requested condition code in the destination reg.
    Setcc { cc: CC, dst: Writable<Reg> },

    /// Integer conditional move.
    /// Overwrites the destination register.
    Cmove {
        /// Possible values are 2, 4 or 8. Checked in the related factory.
        size: u8,
        cc: CC,
        src: RegMem,
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
    /// Float arithmetic/bit-twiddling: (add sub and or xor mul adc? sbb?) (32 64) (reg addr) reg
    XMM_RM_R {
        op: SseOpcode,
        src: RegMem,
        dst: Writable<Reg>,
    },

    /// mov between XMM registers (32 64) (reg addr) reg XMM_Mov_RM_R differs from XMM_RM_R in
    /// that the dst register of XMM_MOV_RM_R is not used in the computation of the instruction
    /// dst value and so does not have to be a previously valid value. This is characteristic of
    /// mov instructions.
    XMM_Mov_RM_R {
        op: SseOpcode,
        src: RegMem,
        dst: Writable<Reg>,
    },

    /// mov reg addr (good for all memory stores from xmm registers)
    XMM_Mov_R_M {
        op: SseOpcode,
        src: Reg,
        dst: SyntheticAmode,
    },

    // =====================================
    // Control flow instructions.
    /// Direct call: call simm32.
    CallKnown {
        dest: ExternalName,
        uses: Vec<Reg>,
        defs: Vec<Writable<Reg>>,
        loc: SourceLoc,
        opcode: Opcode,
    },

    /// Indirect call: callq (reg mem).
    CallUnknown {
        dest: RegMem,
        uses: Vec<Reg>,
        defs: Vec<Writable<Reg>>,
        loc: SourceLoc,
        opcode: Opcode,
    },

    /// Return.
    Ret,

    /// A placeholder instruction, generating no code, meaning that a function epilogue must be
    /// inserted there.
    EpiloguePlaceholder,

    /// Jump to a known target: jmp simm32.
    JmpKnown { dst: BranchTarget },

    /// Two-way conditional branch: jcond cond target target.
    /// Emitted as a compound sequence; the MachBuffer will shrink it as appropriate.
    JmpCond {
        cc: CC,
        taken: BranchTarget,
        not_taken: BranchTarget,
    },

    /// Indirect jump: jmpq (reg mem).
    JmpUnknown { target: RegMem },

    /// A debug trap.
    Hlt,

    /// An instruction that will always trigger the illegal instruction exception.
    Ud2 { trap_info: (SourceLoc, TrapCode) },

    // =====================================
    // Meta-instructions generating no code.
    /// Marker, no-op in generated code: SP "virtual offset" is adjusted. This
    /// controls how MemArg::NominalSPOffset args are lowered.
    VirtualSPOffsetAdj { offset: i64 },
}

// Handy constructors for Insts.

// For various sizes, will some number of lowest bits sign extend to be the
// same as the whole value?
pub(crate) fn low32willSXto64(x: u64) -> bool {
    let xs = x as i64;
    xs == ((xs << 32) >> 32)
}

impl Inst {
    pub(crate) fn nop(len: u8) -> Self {
        debug_assert!(len <= 16);
        Self::Nop { len }
    }

    pub(crate) fn alu_rmi_r(
        is_64: bool,
        op: AluRmiROpcode,
        src: RegMemImm,
        dst: Writable<Reg>,
    ) -> Self {
        debug_assert!(dst.to_reg().get_class() == RegClass::I64);
        Self::Alu_RMI_R {
            is_64,
            op,
            src,
            dst,
        }
    }

    pub(crate) fn imm_r(dst_is_64: bool, simm64: u64, dst: Writable<Reg>) -> Inst {
        debug_assert!(dst.to_reg().get_class() == RegClass::I64);
        if !dst_is_64 {
            debug_assert!(low32willSXto64(simm64));
        }
        Inst::Imm_R {
            dst_is_64,
            simm64,
            dst,
        }
    }

    pub(crate) fn mov_r_r(is_64: bool, src: Reg, dst: Writable<Reg>) -> Inst {
        debug_assert!(src.get_class() == RegClass::I64);
        debug_assert!(dst.to_reg().get_class() == RegClass::I64);
        Inst::Mov_R_R { is_64, src, dst }
    }

    pub(crate) fn xmm_mov_rm_r(op: SseOpcode, src: RegMem, dst: Writable<Reg>) -> Inst {
        debug_assert!(dst.to_reg().get_class() == RegClass::V128);
        Inst::XMM_Mov_RM_R { op, src, dst }
    }

    pub(crate) fn xmm_rm_r(op: SseOpcode, src: RegMem, dst: Writable<Reg>) -> Self {
        debug_assert!(dst.to_reg().get_class() == RegClass::V128);
        Inst::XMM_RM_R { op, src, dst }
    }

    pub(crate) fn xmm_mov_r_m(op: SseOpcode, src: Reg, dst: impl Into<SyntheticAmode>) -> Inst {
        debug_assert!(src.get_class() == RegClass::V128);
        Inst::XMM_Mov_R_M {
            op,
            src,
            dst: dst.into(),
        }
    }

    pub(crate) fn movzx_rm_r(ext_mode: ExtMode, src: RegMem, dst: Writable<Reg>) -> Inst {
        debug_assert!(dst.to_reg().get_class() == RegClass::I64);
        Inst::MovZX_RM_R { ext_mode, src, dst }
    }

    pub(crate) fn mov64_m_r(src: impl Into<SyntheticAmode>, dst: Writable<Reg>) -> Inst {
        debug_assert!(dst.to_reg().get_class() == RegClass::I64);
        Inst::Mov64_M_R {
            src: src.into(),
            dst,
        }
    }

    pub(crate) fn movsx_rm_r(ext_mode: ExtMode, src: RegMem, dst: Writable<Reg>) -> Inst {
        debug_assert!(dst.to_reg().get_class() == RegClass::I64);
        Inst::MovSX_RM_R { ext_mode, src, dst }
    }

    pub(crate) fn mov_r_m(
        size: u8, // 1, 2, 4 or 8
        src: Reg,
        dst: impl Into<SyntheticAmode>,
    ) -> Inst {
        debug_assert!(size == 8 || size == 4 || size == 2 || size == 1);
        debug_assert!(src.get_class() == RegClass::I64);
        Inst::Mov_R_M {
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
        is_64: bool,
        kind: ShiftKind,
        num_bits: Option<u8>,
        dst: Writable<Reg>,
    ) -> Inst {
        debug_assert!(if let Some(num_bits) = num_bits {
            num_bits < if is_64 { 64 } else { 32 }
        } else {
            true
        });
        debug_assert!(dst.to_reg().get_class() == RegClass::I64);
        Inst::Shift_R {
            is_64,
            kind,
            num_bits,
            dst,
        }
    }

    /// Does a comparison of dst - src for operands of size `size`, as stated by the machine
    /// instruction semantics. Be careful with the order of parameters!
    pub(crate) fn cmp_rmi_r(
        size: u8, // 1, 2, 4 or 8
        src: RegMemImm,
        dst: Reg,
    ) -> Inst {
        debug_assert!(size == 8 || size == 4 || size == 2 || size == 1);
        debug_assert!(dst.get_class() == RegClass::I64);
        Inst::Cmp_RMI_R { size, src, dst }
    }

    pub(crate) fn setcc(cc: CC, dst: Writable<Reg>) -> Inst {
        debug_assert!(dst.to_reg().get_class() == RegClass::I64);
        Inst::Setcc { cc, dst }
    }

    pub(crate) fn cmove(size: u8, cc: CC, src: RegMem, dst: Writable<Reg>) -> Inst {
        debug_assert!(size == 8 || size == 4 || size == 2);
        debug_assert!(dst.to_reg().get_class() == RegClass::I64);
        Inst::Cmove { size, cc, src, dst }
    }

    pub(crate) fn push64(src: RegMemImm) -> Inst {
        Inst::Push64 { src }
    }

    pub(crate) fn pop64(dst: Writable<Reg>) -> Inst {
        Inst::Pop64 { dst }
    }

    pub(crate) fn call_known(
        dest: ExternalName,
        uses: Vec<Reg>,
        defs: Vec<Writable<Reg>>,
        loc: SourceLoc,
        opcode: Opcode,
    ) -> Inst {
        Inst::CallKnown {
            dest,
            uses,
            defs,
            loc,
            opcode,
        }
    }

    pub(crate) fn call_unknown(
        dest: RegMem,
        uses: Vec<Reg>,
        defs: Vec<Writable<Reg>>,
        loc: SourceLoc,
        opcode: Opcode,
    ) -> Inst {
        Inst::CallUnknown {
            dest,
            uses,
            defs,
            loc,
            opcode,
        }
    }

    pub(crate) fn ret() -> Inst {
        Inst::Ret
    }

    pub(crate) fn epilogue_placeholder() -> Inst {
        Inst::EpiloguePlaceholder
    }

    pub(crate) fn jmp_known(dst: BranchTarget) -> Inst {
        Inst::JmpKnown { dst }
    }

    pub(crate) fn jmp_cond(cc: CC, taken: BranchTarget, not_taken: BranchTarget) -> Inst {
        Inst::JmpCond {
            cc,
            taken,
            not_taken,
        }
    }

    pub(crate) fn jmp_unknown(target: RegMem) -> Inst {
        Inst::JmpUnknown { target }
    }
}

//=============================================================================
// Instructions: printing

impl ShowWithRRU for Inst {
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

        fn suffixLQ(is_64: bool) -> String {
            (if is_64 { "q" } else { "l" }).to_string()
        }

        fn sizeLQ(is_64: bool) -> u8 {
            if is_64 {
                8
            } else {
                4
            }
        }

        fn suffixBWLQ(size: u8) -> String {
            match size {
                1 => "b".to_string(),
                2 => "w".to_string(),
                4 => "l".to_string(),
                8 => "q".to_string(),
                _ => panic!("Inst(x64).show.suffixBWLQ: size={}", size),
            }
        }

        match self {
            Inst::Nop { len } => format!("{} len={}", ljustify("nop".to_string()), len),
            Inst::Alu_RMI_R {
                is_64,
                op,
                src,
                dst,
            } => format!(
                "{} {}, {}",
                ljustify2(op.to_string(), suffixLQ(*is_64)),
                src.show_rru_sized(mb_rru, sizeLQ(*is_64)),
                show_ireg_sized(dst.to_reg(), mb_rru, sizeLQ(*is_64)),
            ),
            Inst::XMM_Mov_RM_R { op, src, dst } => format!(
                "{} {}, {}",
                ljustify(op.to_string()),
                src.show_rru_sized(mb_rru, op.src_size()),
                show_ireg_sized(dst.to_reg(), mb_rru, 8),
            ),
            Inst::XMM_Mov_R_M { op, src, dst } => format!(
                "{} {}, {}",
                ljustify(op.to_string()),
                show_ireg_sized(*src, mb_rru, 8),
                dst.show_rru(mb_rru)
            ),
            Inst::XMM_RM_R { op, src, dst } => format!(
                "{} {}, {}",
                ljustify(op.to_string()),
                src.show_rru_sized(mb_rru, 8),
                show_ireg_sized(dst.to_reg(), mb_rru, 8),
            ),
            Inst::Imm_R {
                dst_is_64,
                simm64,
                dst,
            } => {
                if *dst_is_64 {
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
            Inst::Mov_R_R { is_64, src, dst } => format!(
                "{} {}, {}",
                ljustify2("mov".to_string(), suffixLQ(*is_64)),
                show_ireg_sized(*src, mb_rru, sizeLQ(*is_64)),
                show_ireg_sized(dst.to_reg(), mb_rru, sizeLQ(*is_64))
            ),
            Inst::MovZX_RM_R { ext_mode, src, dst } => {
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
            Inst::Mov64_M_R { src, dst } => format!(
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
            Inst::MovSX_RM_R { ext_mode, src, dst } => format!(
                "{} {}, {}",
                ljustify2("movs".to_string(), ext_mode.to_string()),
                src.show_rru_sized(mb_rru, ext_mode.src_size()),
                show_ireg_sized(dst.to_reg(), mb_rru, ext_mode.dst_size())
            ),
            Inst::Mov_R_M { size, src, dst } => format!(
                "{} {}, {}",
                ljustify2("mov".to_string(), suffixBWLQ(*size)),
                show_ireg_sized(*src, mb_rru, *size),
                dst.show_rru(mb_rru)
            ),
            Inst::Shift_R {
                is_64,
                kind,
                num_bits,
                dst,
            } => match num_bits {
                None => format!(
                    "{} %cl, {}",
                    ljustify2(kind.to_string(), suffixLQ(*is_64)),
                    show_ireg_sized(dst.to_reg(), mb_rru, sizeLQ(*is_64))
                ),

                Some(num_bits) => format!(
                    "{} ${}, {}",
                    ljustify2(kind.to_string(), suffixLQ(*is_64)),
                    num_bits,
                    show_ireg_sized(dst.to_reg(), mb_rru, sizeLQ(*is_64))
                ),
            },
            Inst::Cmp_RMI_R { size, src, dst } => format!(
                "{} {}, {}",
                ljustify2("cmp".to_string(), suffixBWLQ(*size)),
                src.show_rru_sized(mb_rru, *size),
                show_ireg_sized(*dst, mb_rru, *size)
            ),
            Inst::Setcc { cc, dst } => format!(
                "{} {}",
                ljustify2("set".to_string(), cc.to_string()),
                show_ireg_sized(dst.to_reg(), mb_rru, 1)
            ),
            Inst::Cmove { size, cc, src, dst } => format!(
                "{} {}, {}",
                ljustify(format!("cmov{}{}", cc.to_string(), suffixBWLQ(*size))),
                src.show_rru_sized(mb_rru, *size),
                show_ireg_sized(dst.to_reg(), mb_rru, *size)
            ),
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
                format!("{} {}", ljustify("jmp".to_string()), dst.show_rru(mb_rru))
            }
            Inst::JmpCond {
                cc,
                taken,
                not_taken,
            } => format!(
                "{} taken={} not_taken={}",
                ljustify2("j".to_string(), cc.to_string()),
                taken.show_rru(mb_rru),
                not_taken.show_rru(mb_rru)
            ),
            //
            Inst::JmpUnknown { target } => format!(
                "{} *{}",
                ljustify("jmp".to_string()),
                target.show_rru(mb_rru)
            ),
            Inst::VirtualSPOffsetAdj { offset } => format!("virtual_sp_offset_adjust {}", offset),
            Inst::Hlt => "hlt".into(),
            Inst::Ud2 { trap_info } => format!("ud2 {}", trap_info.1),
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
    // regalloc.rs will "fix" this for us by removing the the modified set from the use and def
    // sets.
    match inst {
        Inst::Alu_RMI_R {
            is_64: _,
            op: _,
            src,
            dst,
        } => {
            src.get_regs_as_uses(collector);
            collector.add_mod(*dst);
        }
        Inst::XMM_Mov_RM_R { src, dst, .. } => {
            src.get_regs_as_uses(collector);
            collector.add_def(*dst);
        }
        Inst::XMM_RM_R { src, dst, .. } => {
            src.get_regs_as_uses(collector);
            collector.add_mod(*dst);
        }
        Inst::XMM_Mov_R_M { src, dst, .. } => {
            collector.add_use(*src);
            dst.get_regs_as_uses(collector);
        }
        Inst::Imm_R { dst, .. } => {
            collector.add_def(*dst);
        }
        Inst::Mov_R_R { src, dst, .. } => {
            collector.add_use(*src);
            collector.add_def(*dst);
        }
        Inst::MovZX_RM_R { src, dst, .. } => {
            src.get_regs_as_uses(collector);
            collector.add_def(*dst);
        }
        Inst::Mov64_M_R { src, dst } | Inst::LoadEffectiveAddress { addr: src, dst } => {
            src.get_regs_as_uses(collector);
            collector.add_def(*dst)
        }
        Inst::MovSX_RM_R { src, dst, .. } => {
            src.get_regs_as_uses(collector);
            collector.add_def(*dst);
        }
        Inst::Mov_R_M { src, dst, .. } => {
            collector.add_use(*src);
            dst.get_regs_as_uses(collector);
        }
        Inst::Shift_R {
            is_64: _,
            kind: _,
            num_bits,
            dst,
        } => {
            if num_bits.is_none() {
                collector.add_use(regs::rcx());
            }
            collector.add_mod(*dst);
        }
        Inst::Cmp_RMI_R { size: _, src, dst } => {
            src.get_regs_as_uses(collector);
            collector.add_use(*dst); // yes, really `add_use`
        }
        Inst::Setcc { dst, .. } => {
            collector.add_def(*dst);
        }
        Inst::Cmove { src, dst, .. } => {
            src.get_regs_as_uses(collector);
            collector.add_def(*dst);
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

        Inst::Ret
        | Inst::EpiloguePlaceholder
        | Inst::JmpKnown { .. }
        | Inst::JmpCond { .. }
        | Inst::Nop { .. }
        | Inst::JmpUnknown { .. }
        | Inst::VirtualSPOffsetAdj { .. }
        | Inst::Hlt
        | Inst::Ud2 { .. } => {
            // No registers are used.
        }
    }
}

//=============================================================================
// Instructions and subcomponents: map_regs

fn map_use<RUM: RegUsageMapper>(m: &RUM, r: &mut Reg) {
    if let Some(reg) = r.as_virtual_reg() {
        let new = m.get_use(reg).unwrap().to_reg();
        *r = new;
    }
}

fn map_def<RUM: RegUsageMapper>(m: &RUM, r: &mut Writable<Reg>) {
    if let Some(reg) = r.to_reg().as_virtual_reg() {
        let new = m.get_def(reg).unwrap().to_reg();
        *r = Writable::from_reg(new);
    }
}

fn map_mod<RUM: RegUsageMapper>(m: &RUM, r: &mut Writable<Reg>) {
    if let Some(reg) = r.to_reg().as_virtual_reg() {
        let new = m.get_mod(reg).unwrap().to_reg();
        *r = Writable::from_reg(new);
    }
}

impl Amode {
    fn map_uses<RUM: RegUsageMapper>(&mut self, map: &RUM) {
        match self {
            Amode::ImmReg {
                simm32: _,
                ref mut base,
            } => map_use(map, base),
            Amode::ImmRegRegShift {
                simm32: _,
                ref mut base,
                ref mut index,
                shift: _,
            } => {
                map_use(map, base);
                map_use(map, index);
            }
        }
    }
}

impl RegMemImm {
    fn map_uses<RUM: RegUsageMapper>(&mut self, map: &RUM) {
        match self {
            RegMemImm::Reg { ref mut reg } => map_use(map, reg),
            RegMemImm::Mem { ref mut addr } => addr.map_uses(map),
            RegMemImm::Imm { simm32: _ } => {}
        }
    }
}

impl RegMem {
    fn map_uses<RUM: RegUsageMapper>(&mut self, map: &RUM) {
        match self {
            RegMem::Reg { ref mut reg } => map_use(map, reg),
            RegMem::Mem { ref mut addr } => addr.map_uses(map),
        }
    }
}

fn x64_map_regs<RUM: RegUsageMapper>(inst: &mut Inst, mapper: &RUM) {
    // Note this must be carefully synchronized with x64_get_regs.
    match inst {
        // ** Nop
        Inst::Alu_RMI_R {
            is_64: _,
            op: _,
            ref mut src,
            ref mut dst,
        } => {
            src.map_uses(mapper);
            map_mod(mapper, dst);
        }
        Inst::XMM_Mov_RM_R {
            ref mut src,
            ref mut dst,
            ..
        } => {
            src.map_uses(mapper);
            map_def(mapper, dst);
        }
        Inst::XMM_RM_R {
            ref mut src,
            ref mut dst,
            ..
        } => {
            src.map_uses(mapper);
            map_mod(mapper, dst);
        }
        Inst::XMM_Mov_R_M {
            ref mut src,
            ref mut dst,
            ..
        } => {
            map_use(mapper, src);
            dst.map_uses(mapper);
        }
        Inst::Imm_R {
            dst_is_64: _,
            simm64: _,
            ref mut dst,
        } => map_def(mapper, dst),
        Inst::Mov_R_R {
            is_64: _,
            ref mut src,
            ref mut dst,
        } => {
            map_use(mapper, src);
            map_def(mapper, dst);
        }
        Inst::MovZX_RM_R {
            ref mut src,
            ref mut dst,
            ..
        } => {
            src.map_uses(mapper);
            map_def(mapper, dst);
        }
        Inst::Mov64_M_R { src, dst } | Inst::LoadEffectiveAddress { addr: src, dst } => {
            src.map_uses(mapper);
            map_def(mapper, dst);
        }
        Inst::MovSX_RM_R {
            ref mut src,
            ref mut dst,
            ..
        } => {
            src.map_uses(mapper);
            map_def(mapper, dst);
        }
        Inst::Mov_R_M {
            ref mut src,
            ref mut dst,
            ..
        } => {
            map_use(mapper, src);
            dst.map_uses(mapper);
        }
        Inst::Shift_R {
            is_64: _,
            kind: _,
            num_bits: _,
            ref mut dst,
        } => {
            map_mod(mapper, dst);
        }
        Inst::Cmp_RMI_R {
            size: _,
            ref mut src,
            ref mut dst,
        } => {
            src.map_uses(mapper);
            map_use(mapper, dst);
        }
        Inst::Setcc { ref mut dst, .. } => map_def(mapper, dst),
        Inst::Cmove {
            ref mut src,
            ref mut dst,
            ..
        } => {
            src.map_uses(mapper);
            map_def(mapper, dst)
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

        Inst::Ret
        | Inst::EpiloguePlaceholder
        | Inst::JmpKnown { .. }
        | Inst::JmpCond { .. }
        | Inst::Nop { .. }
        | Inst::JmpUnknown { .. }
        | Inst::VirtualSPOffsetAdj { .. }
        | Inst::Ud2 { .. }
        | Inst::Hlt => {
            // No registers are used.
        }
    }
}

//=============================================================================
// Instructions: misc functions and external interface

impl MachInst for Inst {
    fn get_regs(&self, collector: &mut RegUsageCollector) {
        x64_get_regs(&self, collector)
    }

    fn map_regs<RUM: RegUsageMapper>(&mut self, mapper: &RUM) {
        x64_map_regs(self, mapper);
    }

    fn is_move(&self) -> Option<(Writable<Reg>, Reg)> {
        // Note (carefully!) that a 32-bit mov *isn't* a no-op since it zeroes
        // out the upper 32 bits of the destination.  For example, we could
        // conceivably use `movl %reg, %reg` to zero out the top 32 bits of
        // %reg.
        match self {
            Self::Mov_R_R { is_64, src, dst } if *is_64 => Some((*dst, *src)),
            Self::XMM_Mov_RM_R { op, src, dst }
                if *op == SseOpcode::Movss
                    || *op == SseOpcode::Movsd
                    || *op == SseOpcode::Movaps =>
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
            &Self::JmpKnown { dst } => MachTerminator::Uncond(dst.as_label().unwrap()),
            &Self::JmpCond {
                cc: _,
                taken,
                not_taken,
            } => MachTerminator::Cond(taken.as_label().unwrap(), not_taken.as_label().unwrap()),
            // All other cases are boring.
            _ => MachTerminator::None,
        }
    }

    fn gen_move(dst_reg: Writable<Reg>, src_reg: Reg, ty: Type) -> Inst {
        let rc_dst = dst_reg.to_reg().get_class();
        let rc_src = src_reg.get_class();
        // If this isn't true, we have gone way off the rails.
        debug_assert!(rc_dst == rc_src);
        match rc_dst {
            RegClass::I64 => Inst::mov_r_r(true, src_reg, dst_reg),
            RegClass::V128 => match ty {
                F32 => Inst::xmm_mov_rm_r(SseOpcode::Movss, RegMem::reg(src_reg), dst_reg),
                F64 => Inst::xmm_mov_rm_r(SseOpcode::Movsd, RegMem::reg(src_reg), dst_reg),
                _ => panic!("unexpected V128 type in gen_move"),
            },
            _ => panic!("gen_move(x64): unhandled regclass"),
        }
    }

    fn gen_zero_len_nop() -> Inst {
        Inst::Nop { len: 0 }
    }

    fn gen_nop(_preferred_size: usize) -> Inst {
        unimplemented!()
    }

    fn maybe_direct_reload(&self, _reg: VirtualReg, _slot: SpillSlot) -> Option<Inst> {
        None
    }

    fn rc_for_type(ty: Type) -> CodegenResult<RegClass> {
        match ty {
            I8 | I16 | I32 | I64 | B1 | B8 | B16 | B32 | B64 => Ok(RegClass::I64),
            F32 | F64 | I128 | B128 => Ok(RegClass::V128),
            _ => Err(CodegenError::Unsupported(format!(
                "Unexpected SSA-value type: {}",
                ty
            ))),
        }
    }

    fn gen_jump(label: MachLabel) -> Inst {
        Inst::jmp_known(BranchTarget::Label(label))
    }

    fn gen_constant(to_reg: Writable<Reg>, value: u64, _: Type) -> SmallVec<[Self; 4]> {
        let mut ret = SmallVec::new();
        let is64 = value > 0xffff_ffff;
        ret.push(Inst::imm_r(is64, value, to_reg));
        ret
    }

    fn reg_universe(flags: &Flags) -> RealRegUniverse {
        create_reg_universe_systemv(flags)
    }

    fn worst_case_size() -> CodeOffset {
        15
    }

    type LabelUse = LabelUse;
}

/// State carried between emissions of a sequence of instructions.
#[derive(Default, Clone, Debug)]
pub struct EmitState {
    virtual_sp_offset: i64,
}

impl MachInstEmit for Inst {
    type State = EmitState;

    fn emit(&self, sink: &mut MachBuffer<Inst>, flags: &settings::Flags, state: &mut Self::State) {
        emit::emit(self, sink, flags, state);
    }
}

/// A label-use (internal relocation) in generated code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelUse {
    /// A 32-bit offset from location of relocation itself, added to the existing value at that
    /// location. Used for control flow instructions which consider an offset from the start of the
    /// next instruction (so the size of the payload -- 4 bytes -- is subtracted from the payload).
    JmpRel32,
}

impl MachInstLabelUse for LabelUse {
    const ALIGN: CodeOffset = 1;

    fn max_pos_range(self) -> CodeOffset {
        match self {
            LabelUse::JmpRel32 => 0x7fff_ffff,
        }
    }

    fn max_neg_range(self) -> CodeOffset {
        match self {
            LabelUse::JmpRel32 => 0x8000_0000,
        }
    }

    fn patch_size(self) -> CodeOffset {
        match self {
            LabelUse::JmpRel32 => 4,
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
        }
    }

    fn supports_veneer(self) -> bool {
        match self {
            LabelUse::JmpRel32 => false,
        }
    }

    fn veneer_size(self) -> CodeOffset {
        match self {
            LabelUse::JmpRel32 => 0,
        }
    }

    fn generate_veneer(self, _: &mut [u8], _: CodeOffset) -> (CodeOffset, LabelUse) {
        match self {
            LabelUse::JmpRel32 => {
                panic!("Veneer not supported for JumpRel32 label-use.");
            }
        }
    }
}
