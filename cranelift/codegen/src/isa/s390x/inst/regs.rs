//! S390x ISA definitions: registers.

use alloc::string::String;
use regalloc2::MachineEnv;
use regalloc2::PReg;
use regalloc2::VReg;

use crate::machinst::*;
use crate::settings;

//=============================================================================
// Registers, the Universe thereof, and printing

/// Get a reference to a GPR (integer register).
pub fn gpr(num: u8) -> Reg {
    assert!(num < 16);
    let preg = PReg::new(num as usize, RegClass::Int);
    Reg::from(VReg::new(preg.index(), RegClass::Int))
}

/// Get a writable reference to a GPR.
pub fn writable_gpr(num: u8) -> Writable<Reg> {
    Writable::from_reg(gpr(num))
}

/// Get a reference to a FPR (floating-point register).
pub fn fpr(num: u8) -> Reg {
    assert!(num < 16);
    let preg = PReg::new(num as usize, RegClass::Float);
    Reg::from(VReg::new(preg.index(), RegClass::Float))
}

/// Get a writable reference to a V-register.
pub fn writable_fpr(num: u8) -> Writable<Reg> {
    Writable::from_reg(fpr(num))
}

/// Get a reference to the stack-pointer register.
pub fn stack_reg() -> Reg {
    gpr(15)
}

/// Get a writable reference to the stack-pointer register.
pub fn writable_stack_reg() -> Writable<Reg> {
    Writable::from_reg(stack_reg())
}

/// Get a reference to the first temporary, sometimes "spill temporary", register. This register is
/// used to compute the address of a spill slot when a direct offset addressing mode from FP is not
/// sufficient (+/- 2^11 words). We exclude this register from regalloc and reserve it for this
/// purpose for simplicity; otherwise we need a multi-stage analysis where we first determine how
/// many spill slots we have, then perhaps remove the reg from the pool and recompute regalloc.
///
/// We use r1 for this because it's a scratch register but is slightly special (used for linker
/// veneers). We're free to use it as long as we don't expect it to live through call instructions.
pub fn spilltmp_reg() -> Reg {
    gpr(1)
}

/// Get a writable reference to the spilltmp reg.
pub fn writable_spilltmp_reg() -> Writable<Reg> {
    Writable::from_reg(spilltmp_reg())
}

pub fn zero_reg() -> Reg {
    gpr(0)
}

/// Create the register universe for AArch64.
pub fn create_machine_env(_flags: &settings::Flags) -> MachineEnv {
    fn preg(r: Reg) -> PReg {
        r.to_real_reg().unwrap().into()
    }

    MachineEnv {
        preferred_regs_by_class: [
            vec![
                // no r0; can't use for addressing?
                // no r1; it is our spilltmp.
                preg(gpr(2)),
                preg(gpr(3)),
                preg(gpr(4)),
                preg(gpr(5)),
            ],
            vec![
                preg(fpr(0)),
                preg(fpr(1)),
                preg(fpr(2)),
                preg(fpr(3)),
                preg(fpr(4)),
                preg(fpr(5)),
                preg(fpr(6)),
                preg(fpr(7)),
            ],
        ],
        non_preferred_regs_by_class: [
            vec![
                preg(gpr(6)),
                preg(gpr(7)),
                preg(gpr(8)),
                preg(gpr(9)),
                preg(gpr(10)),
                preg(gpr(11)),
                preg(gpr(12)),
                // no r13; it is our scratch reg.
                preg(gpr(14)),
                // no r15; it is the stack pointer.
            ],
            vec![
                preg(fpr(8)),
                preg(fpr(9)),
                preg(fpr(10)),
                preg(fpr(11)),
                preg(fpr(12)),
                preg(fpr(13)),
                preg(fpr(14)),
                // no f15; it is our scratch reg.
            ],
        ],
        scratch_by_class: [preg(gpr(13)), preg(fpr(15))],
        fixed_stack_slots: vec![],
    }
}

pub fn show_reg(reg: Reg) -> String {
    if let Some(rreg) = reg.to_real_reg() {
        match rreg.class() {
            RegClass::Int => format!("%r{}", rreg.hw_enc()),
            RegClass::Float => format!("%f{}", rreg.hw_enc()),
        }
    } else {
        format!("%{:?}", reg)
    }
}

pub fn pretty_print_reg(reg: Reg, allocs: &mut AllocationConsumer<'_>) -> String {
    let reg = allocs.next(reg);
    show_reg(reg)
}
