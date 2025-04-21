//! Materialize mutable instruction operands, e.g., for register allocation.

use crate::{
    Amode, Fixed, Gpr, GprMem, Imm16, Imm32, Imm8, Registers, Simm16, Simm32, Simm8, Xmm, XmmMem,
};

/// An instruction operand.
///
/// This is useful for iterating over the operands of an [`Inst`][crate::Inst].
///
/// ```
/// # use cranelift_assembler_x64::{Fixed, Imm8, inst, Inst, Operand, Registers};
/// pub struct Regs;
/// impl Registers for Regs {
///     type ReadGpr = u8;
///     type ReadWriteGpr = u8;
///     type ReadXmm = u8;
///     type ReadWriteXmm = u8;
/// }
///
/// let rax = 0;
/// let mut inst: Inst<Regs> = inst::addb_i::new(Fixed(rax), Imm8::new(0x42)).into();
/// let operands = inst.operands();
/// assert_eq!(operands.len(), 2);
/// assert!(matches!(operands[0], Operand::ReadWriteGpr { fixed: Some(0), .. }));
/// assert!(matches!(operands[1], Operand::Imm8(_)));
/// ```

/// TODO
#[derive(Debug)]
pub enum Operand<'a, R: Registers> {
    // Memory operands.
    Amode(&'a mut Amode<R::ReadGpr>),

    // Register operands.
    ReadGpr {
        gpr: &'a mut R::ReadGpr,
        fixed: Option<u8>,
    },
    ReadWriteGpr {
        gpr: &'a mut R::ReadWriteGpr,
        fixed: Option<u8>,
    },
    ReadXmm {
        xmm: &'a mut R::ReadXmm,
        fixed: Option<u8>,
    },
    ReadWriteXmm {
        xmm: &'a mut R::ReadWriteXmm,
        fixed: Option<u8>,
    },

    // Immediate operands.
    Imm8(&'a mut Imm8),
    Imm16(&'a mut Imm16),
    Imm32(&'a mut Imm32),
    Simm8(&'a mut Simm8),
    Simm16(&'a mut Simm16),
    Simm32(&'a mut Simm32),
}

impl<'a, R: Registers> Operand<'a, R> {
    pub fn from_read_gpr(gpr: &'a mut Gpr<R::ReadGpr>) -> Self {
        let gpr = &mut gpr.0;
        Operand::ReadGpr { gpr, fixed: None }
    }
    pub fn from_read_write_gpr(gpr: &'a mut Gpr<R::ReadWriteGpr>) -> Self {
        let gpr = &mut gpr.0;
        Operand::ReadWriteGpr { gpr, fixed: None }
    }
    pub fn from_read_fixed_gpr<const E: u8>(gpr: &'a mut Fixed<R::ReadGpr, E>) -> Self {
        let gpr = &mut gpr.0;
        let fixed = Some(E);
        Operand::ReadGpr { gpr, fixed }
    }
    pub fn from_read_write_fixed_gpr<const E: u8>(gpr: &'a mut Fixed<R::ReadWriteGpr, E>) -> Self {
        let gpr = &mut gpr.0;
        let fixed = Some(E);
        Operand::ReadWriteGpr { gpr, fixed }
    }
    pub fn from_read_xmm(xmm: &'a mut Xmm<R::ReadXmm>) -> Self {
        let xmm = &mut xmm.0;
        Operand::ReadXmm { xmm, fixed: None }
    }
    pub fn from_read_write_xmm(xmm: &'a mut Xmm<R::ReadWriteXmm>) -> Self {
        let xmm = &mut xmm.0;
        Operand::ReadWriteXmm { xmm, fixed: None }
    }
    pub fn from_read_gpr_mem(gpr_mem: &'a mut GprMem<R::ReadGpr, R::ReadGpr>) -> Self {
        match gpr_mem {
            GprMem::Gpr(gpr) => Operand::ReadGpr { gpr, fixed: None },
            GprMem::Mem(amode) => Operand::Amode(amode),
        }
    }
    pub fn from_read_write_gpr_mem(gpr_mem: &'a mut GprMem<R::ReadWriteGpr, R::ReadGpr>) -> Self {
        match gpr_mem {
            GprMem::Gpr(gpr) => Operand::ReadWriteGpr { gpr, fixed: None },
            GprMem::Mem(amode) => Operand::Amode(amode),
        }
    }
    pub fn from_read_xmm_mem(xmm_mem: &'a mut XmmMem<R::ReadXmm, R::ReadGpr>) -> Self {
        match xmm_mem {
            XmmMem::Xmm(xmm) => Operand::ReadXmm { xmm, fixed: None },
            XmmMem::Mem(amode) => Operand::Amode(amode),
        }
    }
    pub fn from_read_write_xmm_mem(xmm_mem: &'a mut XmmMem<R::ReadWriteXmm, R::ReadGpr>) -> Self {
        match xmm_mem {
            XmmMem::Xmm(xmm) => Operand::ReadWriteXmm { xmm, fixed: None },
            XmmMem::Mem(amode) => Operand::Amode(amode),
        }
    }
    pub fn from_amode(amode: &'a mut Amode<R::ReadGpr>) -> Self {
        Operand::Amode(amode)
    }
}
