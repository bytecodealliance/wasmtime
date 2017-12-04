//! Intel ABI implementation.

use ir;
use isa::{RegClass, RegUnit};
use regalloc::AllocatableSet;
use settings as shared_settings;
use super::registers::{GPR, FPR, RU};
use abi::{ArgAction, ValueConversion, ArgAssigner, legalize_args};
use ir::{AbiParam, ArgumentPurpose, ArgumentLoc, ArgumentExtension, CallConv};
use std::i32;

/// Argument registers for x86-64
static ARG_GPRS: [RU; 6] = [RU::rdi, RU::rsi, RU::rdx, RU::rcx, RU::r8, RU::r9];

/// Return value registers.
static RET_GPRS: [RU; 3] = [RU::rax, RU::rdx, RU::rcx];

struct Args {
    pointer_bytes: u32,
    pointer_bits: u16,
    pointer_type: ir::Type,
    gpr: &'static [RU],
    gpr_used: usize,
    fpr_limit: usize,
    fpr_used: usize,
    offset: u32,
    call_conv: CallConv,
}

impl Args {
    fn new(bits: u16, gpr: &'static [RU], fpr_limit: usize, call_conv: CallConv) -> Args {
        Args {
            pointer_bytes: u32::from(bits) / 8,
            pointer_bits: bits,
            pointer_type: ir::Type::int(bits).unwrap(),
            gpr,
            gpr_used: 0,
            fpr_limit,
            fpr_used: 0,
            offset: 0,
            call_conv: call_conv,
        }
    }
}

impl ArgAssigner for Args {
    fn assign(&mut self, arg: &AbiParam) -> ArgAction {
        let ty = arg.value_type;

        // Check for a legal type.
        // We don't support SIMD yet, so break all vectors down.
        if ty.is_vector() {
            return ValueConversion::VectorSplit.into();
        }

        // Large integers and booleans are broken down to fit in a register.
        if !ty.is_float() && ty.bits() > self.pointer_bits {
            return ValueConversion::IntSplit.into();
        }

        // Small integers are extended to the size of a pointer register.
        if ty.is_int() && ty.bits() < self.pointer_bits {
            match arg.extension {
                ArgumentExtension::None => {}
                ArgumentExtension::Uext => return ValueConversion::Uext(self.pointer_type).into(),
                ArgumentExtension::Sext => return ValueConversion::Sext(self.pointer_type).into(),
            }
        }

        // Handle special-purpose arguments.
        if ty.is_int() && self.call_conv == CallConv::SpiderWASM {
            match arg.purpose {
                // This is SpiderMonkey's `WasmTlsReg`.
                ArgumentPurpose::VMContext => {
                    return ArgumentLoc::Reg(if self.pointer_bits == 64 {
                        RU::r14
                    } else {
                        RU::rsi
                    } as RegUnit).into()
                }
                // This is SpiderMonkey's `WasmTableCallSigReg`.
                ArgumentPurpose::SignatureId => return ArgumentLoc::Reg(RU::rbx as RegUnit).into(),
                _ => {}
            }
        }

        // Try to use a GPR.
        if !ty.is_float() && self.gpr_used < self.gpr.len() {
            let reg = self.gpr[self.gpr_used] as RegUnit;
            self.gpr_used += 1;
            return ArgumentLoc::Reg(reg).into();
        }

        // Try to use an FPR.
        if ty.is_float() && self.fpr_used < self.fpr_limit {
            let reg = FPR.unit(self.fpr_used);
            self.fpr_used += 1;
            return ArgumentLoc::Reg(reg).into();
        }

        // Assign a stack location.
        let loc = ArgumentLoc::Stack(self.offset as i32);
        self.offset += self.pointer_bytes;
        assert!(self.offset <= i32::MAX as u32);
        loc.into()
    }
}

/// Legalize `sig`.
pub fn legalize_signature(sig: &mut ir::Signature, flags: &shared_settings::Flags, _current: bool) {
    let bits;
    let mut args;

    if flags.is_64bit() {
        bits = 64;
        args = Args::new(bits, &ARG_GPRS, 8, sig.call_conv);
    } else {
        bits = 32;
        args = Args::new(bits, &[], 0, sig.call_conv);
    }

    legalize_args(&mut sig.params, &mut args);

    let mut rets = Args::new(bits, &RET_GPRS, 2, sig.call_conv);
    legalize_args(&mut sig.returns, &mut rets);
}

/// Get register class for a type appearing in a legalized signature.
pub fn regclass_for_abi_type(ty: ir::Type) -> RegClass {
    if ty.is_int() || ty.is_bool() {
        GPR
    } else {
        FPR
    }
}

/// Get the set of allocatable registers for `func`.
pub fn allocatable_registers(
    _func: &ir::Function,
    flags: &shared_settings::Flags,
) -> AllocatableSet {
    let mut regs = AllocatableSet::new();
    regs.take(GPR, RU::rsp as RegUnit);
    regs.take(GPR, RU::rbp as RegUnit);

    // 32-bit arch only has 8 registers.
    if !flags.is_64bit() {
        for i in 8..16 {
            regs.take(GPR, GPR.unit(i));
            regs.take(FPR, FPR.unit(i));
        }
    }

    regs
}

pub fn callee_saved_registers(flags: &shared_settings::Flags) -> &'static [RU] {
    if flags.is_64bit() {
        &[RU::rbx, RU::r12, RU::r13, RU::r14, RU::r15]
    } else {
        &[RU::rbx, RU::rsi, RU::rdi]
    }
}
