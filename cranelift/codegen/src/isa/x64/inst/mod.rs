//! This module defines x86_64-specific machine instruction types.

#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::fmt;
use std::string::{String, ToString};

use regalloc::RegUsageCollector;
use regalloc::Set;
use regalloc::{RealRegUniverse, Reg, RegClass, RegUsageMapper, SpillSlot, VirtualReg, Writable};

use crate::binemit::CodeOffset;
use crate::ir::types::{B1, B128, B16, B32, B64, B8, F32, F64, I128, I16, I32, I64, I8};
use crate::ir::ExternalName;
use crate::ir::Type;
use crate::machinst::*;
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
pub(crate) enum Inst {
    /// nops of various sizes, including zero
    Nop { len: u8 },

    /// (add sub and or xor mul adc? sbb?) (32 64) (reg addr imm) reg
    Alu_RMI_R {
        is_64: bool,
        op: RMI_R_Op,
        src: RMI,
        dst: Writable<Reg>,
    },

    /// (imm32 imm64) reg.
    /// Either: movl $imm32, %reg32 or movabsq $imm64, %reg32
    Imm_R {
        dst_is_64: bool,
        simm64: u64,
        dst: Writable<Reg>,
    },

    /// mov (64 32) reg reg
    Mov_R_R {
        is_64: bool,
        src: Reg,
        dst: Writable<Reg>,
    },

    /// movz (bl bq wl wq lq) addr reg (good for all ZX loads except 64->64).
    /// Note that the lq variant doesn't really exist since the default
    /// zero-extend rule makes it unnecessary.  For that case we emit the
    /// equivalent "movl AM, reg32".
    MovZX_M_R {
        extMode: ExtMode,
        addr: Addr,
        dst: Writable<Reg>,
    },

    /// A plain 64-bit integer load, since MovZX_M_R can't represent that
    Mov64_M_R { addr: Addr, dst: Writable<Reg> },

    /// movs (bl bq wl wq lq) addr reg (good for all SX loads)
    MovSX_M_R {
        extMode: ExtMode,
        addr: Addr,
        dst: Writable<Reg>,
    },

    /// mov (b w l q) reg addr (good for all integer stores)
    Mov_R_M {
        size: u8, // 1, 2, 4 or 8
        src: Reg,
        addr: Addr,
    },

    /// (shl shr sar) (l q) imm reg
    Shift_R {
        is_64: bool,
        kind: ShiftKind,
        /// shift count: Some(0 .. #bits-in-type - 1), or None to mean "%cl".
        num_bits: Option<u8>,
        dst: Writable<Reg>,
    },

    /// cmp (b w l q) (reg addr imm) reg
    Cmp_RMI_R {
        size: u8, // 1, 2, 4 or 8
        src: RMI,
        dst: Reg,
    },

    /// pushq (reg addr imm)
    Push64 { src: RMI },

    /// popq reg
    Pop64 { dst: Writable<Reg> },

    /// call simm32
    CallKnown {
        dest: ExternalName,
        uses: Set<Reg>,
        defs: Set<Writable<Reg>>,
    },

    /// callq (reg mem)
    CallUnknown {
        dest: RM,
        //uses: Set<Reg>,
        //defs: Set<Writable<Reg>>,
    },

    // ---- branches (exactly one must appear at end of BB) ----
    /// ret
    Ret,

    /// A placeholder instruction, generating no code, meaning that a function epilogue must be
    /// inserted there.
    EpiloguePlaceholder,

    /// jmp simm32
    JmpKnown { dest: BranchTarget },

    /// jcond cond target target
    // Symmetrical two-way conditional branch.
    // Should never reach the emitter.
    JmpCondSymm {
        cc: CC,
        taken: BranchTarget,
        not_taken: BranchTarget,
    },

    /// Lowered conditional branch: contains the original instruction, and a
    /// flag indicating whether to invert the taken-condition or not. Only one
    /// BranchTarget is retained, and the other is implicitly the next
    /// instruction, given the final basic-block layout.
    JmpCond {
        cc: CC,
        //inverted: bool, is this needed?
        target: BranchTarget,
    },

    /// As for `CondBrLowered`, but represents a condbr/uncond-br sequence (two
    /// actual machine instructions). Needed when the final block layout implies
    /// that neither arm of a conditional branch targets the fallthrough block.
    // Should never reach the emitter
    JmpCondCompound {
        cc: CC,
        taken: BranchTarget,
        not_taken: BranchTarget,
    },

    /// jmpq (reg mem)
    JmpUnknown { target: RM },
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

    pub(crate) fn alu_rmi_r(is_64: bool, op: RMI_R_Op, src: RMI, dst: Writable<Reg>) -> Self {
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

    pub(crate) fn movzx_m_r(extMode: ExtMode, addr: Addr, dst: Writable<Reg>) -> Inst {
        debug_assert!(dst.to_reg().get_class() == RegClass::I64);
        Inst::MovZX_M_R { extMode, addr, dst }
    }

    pub(crate) fn mov64_m_r(addr: Addr, dst: Writable<Reg>) -> Inst {
        debug_assert!(dst.to_reg().get_class() == RegClass::I64);
        Inst::Mov64_M_R { addr, dst }
    }

    pub(crate) fn movsx_m_r(extMode: ExtMode, addr: Addr, dst: Writable<Reg>) -> Inst {
        debug_assert!(dst.to_reg().get_class() == RegClass::I64);
        Inst::MovSX_M_R { extMode, addr, dst }
    }

    pub(crate) fn mov_r_m(
        size: u8, // 1, 2, 4 or 8
        src: Reg,
        addr: Addr,
    ) -> Inst {
        debug_assert!(size == 8 || size == 4 || size == 2 || size == 1);
        debug_assert!(src.get_class() == RegClass::I64);
        Inst::Mov_R_M { size, src, addr }
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

    pub(crate) fn cmp_rmi_r(
        size: u8, // 1, 2, 4 or 8
        src: RMI,
        dst: Reg,
    ) -> Inst {
        debug_assert!(size == 8 || size == 4 || size == 2 || size == 1);
        debug_assert!(dst.get_class() == RegClass::I64);
        Inst::Cmp_RMI_R { size, src, dst }
    }

    pub(crate) fn push64(src: RMI) -> Inst {
        Inst::Push64 { src }
    }

    pub(crate) fn pop64(dst: Writable<Reg>) -> Inst {
        Inst::Pop64 { dst }
    }

    pub(crate) fn call_unknown(dest: RM) -> Inst {
        Inst::CallUnknown { dest }
    }

    pub(crate) fn ret() -> Inst {
        Inst::Ret
    }

    pub(crate) fn epilogue_placeholder() -> Inst {
        Inst::EpiloguePlaceholder
    }

    pub(crate) fn jmp_known(dest: BranchTarget) -> Inst {
        Inst::JmpKnown { dest }
    }

    pub(crate) fn jmp_cond_symm(cc: CC, taken: BranchTarget, not_taken: BranchTarget) -> Inst {
        Inst::JmpCondSymm {
            cc,
            taken,
            not_taken,
        }
    }

    pub(crate) fn jmp_cond(cc: CC, target: BranchTarget) -> Inst {
        Inst::JmpCond { cc, target }
    }

    pub(crate) fn jmp_cond_compound(cc: CC, taken: BranchTarget, not_taken: BranchTarget) -> Inst {
        Inst::JmpCondCompound {
            cc,
            taken,
            not_taken,
        }
    }

    pub(crate) fn jmp_unknown(target: RM) -> Inst {
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
            Inst::MovZX_M_R { extMode, addr, dst } => {
                if *extMode == ExtMode::LQ {
                    format!(
                        "{} {}, {}",
                        ljustify("movl".to_string()),
                        addr.show_rru(mb_rru),
                        show_ireg_sized(dst.to_reg(), mb_rru, 4)
                    )
                } else {
                    format!(
                        "{} {}, {}",
                        ljustify2("movz".to_string(), extMode.to_string()),
                        addr.show_rru(mb_rru),
                        show_ireg_sized(dst.to_reg(), mb_rru, extMode.dst_size())
                    )
                }
            }
            Inst::Mov64_M_R { addr, dst } => format!(
                "{} {}, {}",
                ljustify("movq".to_string()),
                addr.show_rru(mb_rru),
                dst.show_rru(mb_rru)
            ),
            Inst::MovSX_M_R { extMode, addr, dst } => format!(
                "{} {}, {}",
                ljustify2("movs".to_string(), extMode.to_string()),
                addr.show_rru(mb_rru),
                show_ireg_sized(dst.to_reg(), mb_rru, extMode.dst_size())
            ),
            Inst::Mov_R_M { size, src, addr } => format!(
                "{} {}, {}",
                ljustify2("mov".to_string(), suffixBWLQ(*size)),
                show_ireg_sized(*src, mb_rru, *size),
                addr.show_rru(mb_rru)
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
            Inst::Push64 { src } => {
                format!("{} {}", ljustify("pushq".to_string()), src.show_rru(mb_rru))
            }
            Inst::Pop64 { dst } => {
                format!("{} {}", ljustify("popq".to_string()), dst.show_rru(mb_rru))
            }
            //Inst::CallKnown { target } => format!("{} {:?}", ljustify("call".to_string()), target),
            Inst::CallKnown { .. } => "**CallKnown**".to_string(),
            Inst::CallUnknown { dest } => format!(
                "{} *{}",
                ljustify("call".to_string()),
                dest.show_rru(mb_rru)
            ),
            Inst::Ret => "ret".to_string(),
            Inst::EpiloguePlaceholder => "epilogue placeholder".to_string(),
            Inst::JmpKnown { dest } => {
                format!("{} {}", ljustify("jmp".to_string()), dest.show_rru(mb_rru))
            }
            Inst::JmpCondSymm {
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
            Inst::JmpCond { cc, ref target } => format!(
                "{} {}",
                ljustify2("j".to_string(), cc.to_string()),
                target.show_rru(None)
            ),
            //
            Inst::JmpCondCompound { .. } => "**JmpCondCompound**".to_string(),
            Inst::JmpUnknown { target } => format!(
                "{} *{}",
                ljustify("jmp".to_string()),
                target.show_rru(mb_rru)
            ),
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
        // ** Nop
        Inst::Alu_RMI_R {
            is_64: _,
            op: _,
            src,
            dst,
        } => {
            src.get_regs_as_uses(collector);
            collector.add_mod(*dst);
        }
        Inst::Imm_R {
            dst_is_64: _,
            simm64: _,
            dst,
        } => {
            collector.add_def(*dst);
        }
        Inst::Mov_R_R { is_64: _, src, dst } => {
            collector.add_use(*src);
            collector.add_def(*dst);
        }
        Inst::MovZX_M_R {
            extMode: _,
            addr,
            dst,
        } => {
            addr.get_regs_as_uses(collector);
            collector.add_def(*dst);
        }
        Inst::Mov64_M_R { addr, dst } => {
            addr.get_regs_as_uses(collector);
            collector.add_def(*dst);
        }
        Inst::MovSX_M_R {
            extMode: _,
            addr,
            dst,
        } => {
            addr.get_regs_as_uses(collector);
            collector.add_def(*dst);
        }
        Inst::Mov_R_M { size: _, src, addr } => {
            collector.add_use(*src);
            addr.get_regs_as_uses(collector);
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
        Inst::Push64 { src } => {
            src.get_regs_as_uses(collector);
            collector.add_mod(Writable::from_reg(regs::rsp()));
        }
        Inst::Pop64 { dst } => {
            collector.add_def(*dst);
        }
        Inst::CallKnown {
            dest: _,
            uses: _,
            defs: _,
        } => {
            // FIXME add arg regs (iru.used) and caller-saved regs (iru.defined)
            unimplemented!();
        }
        Inst::CallUnknown { dest } => {
            dest.get_regs_as_uses(collector);
        }
        Inst::Ret => {}
        Inst::EpiloguePlaceholder => {}
        Inst::JmpKnown { dest: _ } => {}
        Inst::JmpCondSymm {
            cc: _,
            taken: _,
            not_taken: _,
        } => {}
        //
        // ** JmpCond
        //
        // ** JmpCondCompound
        //
        //Inst::JmpUnknown { target } => {
        //    target.get_regs_as_uses(collector);
        //}
        Inst::Nop { .. }
        | Inst::JmpCond { .. }
        | Inst::JmpCondCompound { .. }
        | Inst::JmpUnknown { .. } => unimplemented!("x64_get_regs inst"),
    }
}

//=============================================================================
// Instructions and subcomponents: map_regs

fn map_use(m: &RegUsageMapper, r: &mut Reg) {
    if r.is_virtual() {
        let new = m.get_use(r.to_virtual_reg()).unwrap().to_reg();
        *r = new;
    }
}

fn map_def(m: &RegUsageMapper, r: &mut Writable<Reg>) {
    if r.to_reg().is_virtual() {
        let new = m.get_def(r.to_reg().to_virtual_reg()).unwrap().to_reg();
        *r = Writable::from_reg(new);
    }
}

fn map_mod(m: &RegUsageMapper, r: &mut Writable<Reg>) {
    if r.to_reg().is_virtual() {
        let new = m.get_mod(r.to_reg().to_virtual_reg()).unwrap().to_reg();
        *r = Writable::from_reg(new);
    }
}

impl Addr {
    fn map_uses(&mut self, map: &RegUsageMapper) {
        match self {
            Addr::IR {
                simm32: _,
                ref mut base,
            } => map_use(map, base),
            Addr::IRRS {
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

impl RMI {
    fn map_uses(&mut self, map: &RegUsageMapper) {
        match self {
            RMI::R { ref mut reg } => map_use(map, reg),
            RMI::M { ref mut addr } => addr.map_uses(map),
            RMI::I { simm32: _ } => {}
        }
    }
}

impl RM {
    fn map_uses(&mut self, map: &RegUsageMapper) {
        match self {
            RM::R { ref mut reg } => map_use(map, reg),
            RM::M { ref mut addr } => addr.map_uses(map),
        }
    }
}

fn x64_map_regs(inst: &mut Inst, mapper: &RegUsageMapper) {
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
        Inst::MovZX_M_R {
            extMode: _,
            ref mut addr,
            ref mut dst,
        } => {
            addr.map_uses(mapper);
            map_def(mapper, dst);
        }
        Inst::Mov64_M_R { addr, dst } => {
            addr.map_uses(mapper);
            map_def(mapper, dst);
        }
        Inst::MovSX_M_R {
            extMode: _,
            ref mut addr,
            ref mut dst,
        } => {
            addr.map_uses(mapper);
            map_def(mapper, dst);
        }
        Inst::Mov_R_M {
            size: _,
            ref mut src,
            ref mut addr,
        } => {
            map_use(mapper, src);
            addr.map_uses(mapper);
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
        Inst::Push64 { ref mut src } => src.map_uses(mapper),
        Inst::Pop64 { ref mut dst } => {
            map_def(mapper, dst);
        }
        Inst::CallKnown {
            dest: _,
            uses: _,
            defs: _,
        } => {}
        Inst::CallUnknown { dest } => dest.map_uses(mapper),
        Inst::Ret => {}
        Inst::EpiloguePlaceholder => {}
        Inst::JmpKnown { dest: _ } => {}
        Inst::JmpCondSymm {
            cc: _,
            taken: _,
            not_taken: _,
        } => {}
        //
        // ** JmpCond
        //
        // ** JmpCondCompound
        //
        //Inst::JmpUnknown { target } => {
        //    target.apply_map(mapper);
        //}
        Inst::Nop { .. }
        | Inst::JmpCond { .. }
        | Inst::JmpCondCompound { .. }
        | Inst::JmpUnknown { .. } => unimplemented!("x64_map_regs opcode"),
    }
}

//=============================================================================
// Instructions: misc functions and external interface

impl MachInst for Inst {
    fn get_regs(&self, collector: &mut RegUsageCollector) {
        x64_get_regs(&self, collector)
    }

    fn map_regs(&mut self, mapper: &RegUsageMapper) {
        x64_map_regs(self, mapper);
    }

    fn is_move(&self) -> Option<(Writable<Reg>, Reg)> {
        // Note (carefully!) that a 32-bit mov *isn't* a no-op since it zeroes
        // out the upper 32 bits of the destination.  For example, we could
        // conceivably use `movl %reg, %reg` to zero out the top 32 bits of
        // %reg.
        match self {
            Self::Mov_R_R { is_64, src, dst } if *is_64 => Some((*dst, *src)),
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
            &Self::JmpKnown { dest } => MachTerminator::Uncond(dest.as_block_index().unwrap()),
            &Self::JmpCondSymm {
                cc: _,
                taken,
                not_taken,
            } => MachTerminator::Cond(
                taken.as_block_index().unwrap(),
                not_taken.as_block_index().unwrap(),
            ),
            &Self::JmpCond { .. } | &Self::JmpCondCompound { .. } => {
                panic!("is_term() called after lowering branches");
            }
            // All other cases are boring.
            _ => MachTerminator::None,
        }
    }

    fn gen_move(dst_reg: Writable<Reg>, src_reg: Reg, _ty: Type) -> Inst {
        let rc_dst = dst_reg.to_reg().get_class();
        let rc_src = src_reg.get_class();
        // If this isn't true, we have gone way off the rails.
        debug_assert!(rc_dst == rc_src);
        match rc_dst {
            RegClass::I64 => Inst::mov_r_r(true, src_reg, dst_reg),
            _ => panic!("gen_move(x64): unhandled regclass"),
        }
    }

    fn gen_zero_len_nop() -> Inst {
        unimplemented!()
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

    fn gen_jump(blockindex: BlockIndex) -> Inst {
        Inst::jmp_known(BranchTarget::Block(blockindex))
    }

    fn with_block_rewrites(&mut self, block_target_map: &[BlockIndex]) {
        // This is identical (modulo renaming) to the arm64 version.
        match self {
            &mut Inst::JmpKnown { ref mut dest } => {
                dest.map(block_target_map);
            }
            &mut Inst::JmpCondSymm {
                cc: _,
                ref mut taken,
                ref mut not_taken,
            } => {
                taken.map(block_target_map);
                not_taken.map(block_target_map);
            }
            &mut Inst::JmpCond { .. } | &mut Inst::JmpCondCompound { .. } => {
                panic!("with_block_rewrites called after branch lowering!");
            }
            _ => {}
        }
    }

    fn with_fallthrough_block(&mut self, fallthrough: Option<BlockIndex>) {
        // This is identical (modulo renaming) to the arm64 version.
        match self {
            &mut Inst::JmpCondSymm {
                cc,
                taken,
                not_taken,
            } => {
                if taken.as_block_index() == fallthrough {
                    *self = Inst::jmp_cond(cc.invert(), not_taken);
                } else if not_taken.as_block_index() == fallthrough {
                    *self = Inst::jmp_cond(cc, taken);
                } else {
                    // We need a compound sequence (condbr / uncond-br).
                    *self = Inst::jmp_cond_compound(cc, taken, not_taken);
                }
            }
            &mut Inst::JmpKnown { dest } => {
                if dest.as_block_index() == fallthrough {
                    *self = Inst::nop(0);
                }
            }
            _ => {}
        }
    }

    fn with_block_offsets(&mut self, my_offset: CodeOffset, targets: &[CodeOffset]) {
        // This is identical (modulo renaming) to the arm64 version.
        match self {
            &mut Self::JmpCond {
                cc: _,
                ref mut target,
            } => {
                target.lower(targets, my_offset);
            }
            &mut Self::JmpCondCompound {
                cc: _,
                ref mut taken,
                ref mut not_taken,
                ..
            } => {
                taken.lower(targets, my_offset);
                not_taken.lower(targets, my_offset);
            }
            &mut Self::JmpKnown { ref mut dest } => {
                dest.lower(targets, my_offset);
            }
            _ => {}
        }
    }

    fn reg_universe(flags: &settings::Flags) -> RealRegUniverse {
        create_reg_universe_systemv(flags)
    }
}

impl<O: MachSectionOutput> MachInstEmit<O> for Inst {
    fn emit(&self, sink: &mut O, _flags: &settings::Flags) {
        emit::emit(self, sink);
    }
}
