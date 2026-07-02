//! This module defines arm32 (AArch32 / A32) machine instruction types.

use crate::binemit::{Addend, CodeOffset, Reloc};
use crate::ir::types::{I8, I16, I32};
use crate::ir::{Type, types};
use crate::isa::FunctionAlignment;
use crate::isa::arm32::abi::Arm32MachineDeps;
use crate::machinst::*;
use crate::{CodegenError, CodegenResult, settings};

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write;
use regalloc2::RegClass;

pub mod regs;
pub use self::regs::*;
pub mod emit;
pub use self::emit::*;

#[cfg(test)]
mod emit_tests;

// The `Inst` type itself is generated from `inst.isle` and re-exported here so
// that the rest of the backend can refer to `Inst` variants directly.
pub use crate::isa::arm32::lower::isle::generated_code::MInst as Inst;

//=============================================================================
// Operand collection.

fn arm32_get_operands(inst: &mut Inst, collector: &mut impl OperandVisitor) {
    // Only collect virtual (allocatable) registers as operands. Physical
    // registers such as sp/fp/lr are fixed in the encoding and are not tracked
    // by the register allocator.
    fn use_if_virtual(collector: &mut impl OperandVisitor, reg: &mut Reg) {
        if reg.to_real_reg().is_none() {
            collector.reg_use(reg);
        }
    }
    fn def_if_virtual(collector: &mut impl OperandVisitor, reg: &mut Writable<Reg>) {
        if reg.to_reg().to_real_reg().is_none() {
            collector.reg_def(reg);
        }
    }

    match inst {
        Inst::Nop0 | Inst::Nop4 | Inst::Ret | Inst::AdjustSp { .. } | Inst::Jump { .. } => {}
        Inst::MovImm { rd, .. } => def_if_virtual(collector, rd),
        Inst::Mov { rd, rm } => {
            use_if_virtual(collector, rm);
            def_if_virtual(collector, rd);
        }
        Inst::Store { rt, base, .. } => {
            use_if_virtual(collector, rt);
            use_if_virtual(collector, base);
        }
        Inst::Load { rt, base, .. } => {
            use_if_virtual(collector, base);
            def_if_virtual(collector, rt);
        }
        Inst::Args { args } => {
            for ArgPair { vreg, preg } in args {
                collector.reg_fixed_def(vreg, *preg);
            }
        }
        Inst::Rets { rets } => {
            for RetPair { vreg, preg } in rets {
                collector.reg_fixed_use(vreg, *preg);
            }
        }
    }
}

impl MachInst for Inst {
    type LabelUse = LabelUse;
    type ABIMachineSpec = Arm32MachineDeps;

    // The undefined instruction `udf`. Used to fill trap-reachable padding.
    const TRAP_OPCODE: &'static [u8] = &0xe7f000f0u32.to_le_bytes();

    fn gen_dummy_use(_reg: Reg) -> Self {
        // Represented as a zero-length nop that "uses" nothing; a dummy use is
        // only needed by targets that must keep a value live artificially.
        Inst::Nop0
    }

    fn canonical_type_for_rc(rc: RegClass) -> Type {
        match rc {
            RegClass::Int => I32,
            RegClass::Float => types::F64,
            RegClass::Vector => types::I8X16,
        }
    }

    fn is_safepoint(&self) -> bool {
        false
    }

    fn get_operands(&mut self, collector: &mut impl OperandVisitor) {
        arm32_get_operands(self, collector);
    }

    fn is_move(&self) -> Option<(Writable<Reg>, Reg)> {
        match self {
            Inst::Mov { rd, rm } => Some((*rd, *rm)),
            _ => None,
        }
    }

    fn is_included_in_clobbers(&self) -> bool {
        !matches!(self, Inst::Args { .. })
    }

    fn is_trap(&self) -> bool {
        false
    }

    fn is_args(&self) -> bool {
        matches!(self, Inst::Args { .. })
    }

    fn call_type(&self) -> CallType {
        CallType::None
    }

    fn is_term(&self) -> MachTerminator {
        match self {
            Inst::Rets { .. } => MachTerminator::Ret,
            Inst::Jump { .. } => MachTerminator::Branch,
            _ => MachTerminator::None,
        }
    }

    fn is_mem_access(&self) -> bool {
        matches!(self, Inst::Load { .. } | Inst::Store { .. })
    }

    fn gen_move(to_reg: Writable<Reg>, from_reg: Reg, _ty: Type) -> Inst {
        Inst::Mov {
            rd: to_reg,
            rm: from_reg,
        }
    }

    fn gen_nop(preferred_size: usize) -> Inst {
        if preferred_size == 0 {
            return Inst::Nop0;
        }
        assert!(preferred_size >= 4);
        Inst::Nop4
    }

    fn gen_nop_units() -> Vec<Vec<u8>> {
        vec![0xe320f000u32.to_le_bytes().to_vec()]
    }

    fn rc_for_type(ty: Type) -> CodegenResult<(&'static [RegClass], &'static [Type])> {
        match ty {
            I8 => Ok((&[RegClass::Int], &[I8])),
            I16 => Ok((&[RegClass::Int], &[I16])),
            I32 => Ok((&[RegClass::Int], &[I32])),
            _ => Err(CodegenError::Unsupported(alloc::format!(
                "Unsupported type on arm32 (only i8/i16/i32 are implemented so far): {ty}"
            ))),
        }
    }

    fn gen_jump(target: MachLabel) -> Inst {
        Inst::Jump { dest: target }
    }

    fn worst_case_size() -> CodeOffset {
        // The largest instruction is `MovImm`, which expands to a `movw` plus a
        // `movt`: 8 bytes.
        8
    }

    fn worst_case_island_growth() -> CodeOffset {
        8
    }

    fn ref_type_regclass(_settings: &settings::Flags) -> RegClass {
        RegClass::Int
    }

    fn function_alignment() -> FunctionAlignment {
        FunctionAlignment {
            minimum: 4,
            preferred: 4,
        }
    }
}

//=============================================================================
// Pretty-printing.

impl Inst {
    pub(crate) fn print_with_state(&self, _state: &mut EmitState) -> String {
        let r = |reg: Reg| reg_name(reg);
        match self {
            Inst::Nop0 => "nop-zero-len".to_string(),
            Inst::Nop4 => "nop".to_string(),
            Inst::Ret => "bx lr".to_string(),
            Inst::MovImm { rd, imm } => {
                let rd = r(rd.to_reg());
                if *imm >> 16 == 0 {
                    alloc::format!("movw {rd}, #{imm}")
                } else {
                    alloc::format!("movw {rd}, #{}; movt {rd}, #{}", imm & 0xffff, imm >> 16)
                }
            }
            Inst::Mov { rd, rm } => {
                alloc::format!("mov {}, {}", r(rd.to_reg()), r(*rm))
            }
            Inst::AdjustSp { amount } => {
                if *amount < 0 {
                    alloc::format!("sub sp, sp, #{}", -*amount)
                } else {
                    alloc::format!("add sp, sp, #{amount}")
                }
            }
            Inst::Store { rt, base, offset } => {
                alloc::format!("str {}, [{}, #{}]", r(*rt), r(*base), offset)
            }
            Inst::Load { rt, base, offset } => {
                alloc::format!("ldr {}, [{}, #{}]", r(rt.to_reg()), r(*base), offset)
            }
            Inst::Jump { dest } => alloc::format!("b {}", dest.to_string()),
            Inst::Args { args } => {
                let mut s = "args".to_string();
                for arg in args {
                    write!(&mut s, " {}={}", r(arg.vreg.to_reg()), r(arg.preg)).unwrap();
                }
                s
            }
            Inst::Rets { rets } => {
                let mut s = "rets".to_string();
                for ret in rets {
                    write!(&mut s, " {}={}", r(ret.vreg), r(ret.preg)).unwrap();
                }
                s
            }
        }
    }
}

//=============================================================================
// Label uses (branch fixups).

/// A use of a label by an instruction, for branch fixups.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelUse {
    /// A 24-bit signed PC-relative branch offset, as used by the A32 `B`/`BL`
    /// instructions. The stored immediate is the offset in units of 4 bytes,
    /// relative to the branch instruction's address plus 8.
    Branch26,
}

impl MachInstLabelUse for LabelUse {
    const ALIGN: CodeOffset = 4;

    fn max_pos_range(self) -> CodeOffset {
        match self {
            // 24-bit signed immediate, scaled by 4.
            LabelUse::Branch26 => ((1 << 23) - 1) * 4,
        }
    }

    fn max_neg_range(self) -> CodeOffset {
        match self {
            LabelUse::Branch26 => (1 << 23) * 4,
        }
    }

    fn patch_size(self) -> CodeOffset {
        4
    }

    fn patch(self, buffer: &mut [u8], use_offset: CodeOffset, label_offset: CodeOffset) {
        match self {
            LabelUse::Branch26 => {
                // The ARM PC reads as the instruction address plus 8.
                let pc_base = use_offset as i64 + 8;
                let offset = label_offset as i64 - pc_base;
                debug_assert!(offset & 0b11 == 0);
                let imm24 = ((offset >> 2) as u32) & 0x00ff_ffff;
                let insn = u32::from_le_bytes(buffer[0..4].try_into().unwrap());
                let insn = (insn & 0xff00_0000) | imm24;
                buffer[0..4].copy_from_slice(&insn.to_le_bytes());
            }
        }
    }

    fn supports_veneer(self) -> bool {
        false
    }

    fn veneer_size(self) -> CodeOffset {
        0
    }

    fn worst_case_veneer_size() -> CodeOffset {
        0
    }

    fn generate_veneer(
        self,
        _buffer: &mut [u8],
        _veneer_offset: CodeOffset,
    ) -> (CodeOffset, LabelUse) {
        panic!("arm32 does not support branch veneers yet");
    }

    fn from_reloc(reloc: Reloc, _addend: Addend) -> Option<LabelUse> {
        match reloc {
            Reloc::Arm32Call => Some(LabelUse::Branch26),
            _ => None,
        }
    }
}
