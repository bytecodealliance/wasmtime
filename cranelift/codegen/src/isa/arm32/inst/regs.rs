//! Arm32 (AArch32 / A32) ISA definitions: registers.
//!
//! We model the 16 general-purpose ARM registers r0-r15 as `RegClass::Int`.
//! Following the AAPCS calling convention:
//!
//! * r0-r3   : argument / caller-saved (scratch)
//! * r4-r10  : callee-saved
//! * r11 (fp): frame pointer
//! * r12 (ip): intra-procedure scratch / spill temporary (reserved)
//! * r13 (sp): stack pointer
//! * r14 (lr): link register
//! * r15 (pc): program counter
//!
//! Floating-point (VFP/NEON) registers are not modelled yet.

use crate::machinst::{Reg, Writable};
use regalloc2::{MachineEnv, PReg, RegClass, VReg};

/// Construct a virtual-ish `Reg` referencing GPR `enc` (0-15).
#[inline]
pub const fn xreg(enc: u8) -> Reg {
    let p_reg = PReg::new(enc as usize, RegClass::Int);
    let v_reg = VReg::new(p_reg.index(), p_reg.class());
    Reg::from_virtual_reg(v_reg)
}

/// Construct a `PReg` referencing GPR `enc` (0-15).
pub const fn preg(enc: u8) -> PReg {
    PReg::new(enc as usize, RegClass::Int)
}

/// Construct a `Reg` referencing VFP double-register `D<enc>` (0-15). An `f32`
/// value uses the low single-register half (`S<2*enc>`) of its `D` register, so
/// each value owns a whole `D` register and the S/D register files never alias.
#[inline]
pub const fn dreg(enc: u8) -> Reg {
    let p_reg = PReg::new(enc as usize, RegClass::Float);
    let v_reg = VReg::new(p_reg.index(), p_reg.class());
    Reg::from_virtual_reg(v_reg)
}

/// Construct a `PReg` referencing VFP double-register `D<enc>`.
pub const fn pdreg(enc: u8) -> PReg {
    PReg::new(enc as usize, RegClass::Float)
}

/// Get a writable reference to `D<enc>`.
#[inline]
#[allow(dead_code, reason = "part of the register API, used as the backend grows")]
pub fn writable_dreg(enc: u8) -> Writable<Reg> {
    Writable::from_reg(dreg(enc))
}

/// Get a writable reference to GPR `enc`.
#[inline]
pub fn writable_xreg(enc: u8) -> Writable<Reg> {
    Writable::from_reg(xreg(enc))
}

/// Frame pointer (r11).
#[inline]
pub fn fp_reg() -> Reg {
    xreg(11)
}

/// Writable frame pointer.
#[inline]
pub fn writable_fp_reg() -> Writable<Reg> {
    Writable::from_reg(fp_reg())
}

/// Intra-procedure-call scratch register (r12), used as the spill temporary.
#[inline]
pub fn spilltmp_reg() -> Reg {
    xreg(12)
}

/// Stack pointer (r13).
#[inline]
pub fn stack_reg() -> Reg {
    xreg(13)
}

/// Writable stack pointer.
#[inline]
#[allow(
    dead_code,
    reason = "part of the register API, used as the backend grows"
)]
pub fn writable_stack_reg() -> Writable<Reg> {
    Writable::from_reg(stack_reg())
}

/// Link register (r14).
#[inline]
#[allow(dead_code, reason = "part of the register API, used as the backend grows")]
pub fn link_reg() -> Reg {
    xreg(14)
}

/// Writable link register.
#[inline]
#[allow(dead_code, reason = "part of the register API, used as the backend grows")]
pub fn writable_link_reg() -> Writable<Reg> {
    Writable::from_reg(link_reg())
}

/// Program counter (r15).
#[inline]
#[allow(
    dead_code,
    reason = "part of the register API, used as the backend grows"
)]
pub fn pc_reg() -> Reg {
    xreg(15)
}

/// Pretty-print a register with its AAPCS / VFP name.
pub fn reg_name(reg: Reg) -> alloc::string::String {
    use alloc::string::ToString;
    match reg.to_real_reg() {
        Some(real) => match real.class() {
            RegClass::Float => alloc::format!("d{}", real.hw_enc()),
            _ => match real.hw_enc() {
                11 => "fp".to_string(),
                12 => "ip".to_string(),
                13 => "sp".to_string(),
                14 => "lr".to_string(),
                15 => "pc".to_string(),
                enc => alloc::format!("r{enc}"),
            },
        },
        None => alloc::format!("{reg:?}"),
    }
}

/// Build the register environment describing which registers the allocator may
/// use, in preference order.
pub const fn create_reg_environment() -> MachineEnv {
    // Preferred registers are the caller-saved (scratch) registers; the
    // allocator reaches for these first since they need no save/restore.
    let preferred_int = preg_set(RegClass::Int, &[0, 1, 2, 3]);
    let non_preferred_int = preg_set(RegClass::Int, &[4, 5, 6, 7, 8, 9, 10]);
    // AAPCS: D0-D7 are caller-saved, D8-D15 callee-saved.
    let preferred_float = preg_set(RegClass::Float, &[0, 1, 2, 3, 4, 5, 6, 7]);
    let non_preferred_float = preg_set(RegClass::Float, &[8, 9, 10, 11, 12, 13, 14, 15]);

    MachineEnv {
        preferred_regs_by_class: [preferred_int, preferred_float, empty()],
        non_preferred_regs_by_class: [non_preferred_int, non_preferred_float, empty()],
        fixed_stack_slots: vec![],
        scratch_by_class: [None, None, None],
    }
}

const fn preg_set(class: RegClass, encs: &[u8]) -> regalloc2::PRegSet {
    let mut set = regalloc2::PRegSet::empty();
    let mut i = 0;
    while i < encs.len() {
        set = set.with(PReg::new(encs[i] as usize, class));
        i += 1;
    }
    set
}

const fn empty() -> regalloc2::PRegSet {
    regalloc2::PRegSet::empty()
}
