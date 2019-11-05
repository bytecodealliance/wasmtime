//! RISC-V ABI implementation.
//!
//! This module implements the RISC-V calling convention through the primary `legalize_signature()`
//! entry point.
//!
//! This doesn't support the soft-float ABI at the moment.

use super::registers::{FPR, GPR};
use super::settings;
use crate::abi::{legalize_args, ArgAction, ArgAssigner, ValueConversion};
use crate::ir::{self, AbiParam, ArgumentExtension, ArgumentLoc, ArgumentPurpose, Type};
use crate::isa::RegClass;
use crate::regalloc::RegisterSet;
use alloc::borrow::Cow;
use core::i32;
use target_lexicon::Triple;

struct Args {
    pointer_bits: u8,
    pointer_bytes: u8,
    pointer_type: Type,
    regs: u32,
    reg_limit: u32,
    offset: u32,
}

impl Args {
    fn new(bits: u8, enable_e: bool) -> Self {
        Self {
            pointer_bits: bits,
            pointer_bytes: bits / 8,
            pointer_type: Type::int(u16::from(bits)).unwrap(),
            regs: 0,
            reg_limit: if enable_e { 6 } else { 8 },
            offset: 0,
        }
    }
}

impl ArgAssigner for Args {
    fn assign(&mut self, arg: &AbiParam) -> ArgAction {
        fn align(value: u32, to: u32) -> u32 {
            (value + to - 1) & !(to - 1)
        }

        let ty = arg.value_type;

        // Check for a legal type.
        // RISC-V doesn't have SIMD at all, so break all vectors down.
        if ty.is_vector() {
            return ValueConversion::VectorSplit.into();
        }

        // Large integers and booleans are broken down to fit in a register.
        if !ty.is_float() && ty.bits() > u16::from(self.pointer_bits) {
            // Align registers and stack to a multiple of two pointers.
            self.regs = align(self.regs, 2);
            self.offset = align(self.offset, 2 * u32::from(self.pointer_bytes));
            return ValueConversion::IntSplit.into();
        }

        // Small integers are extended to the size of a pointer register.
        if ty.is_int() && ty.bits() < u16::from(self.pointer_bits) {
            match arg.extension {
                ArgumentExtension::None => {}
                ArgumentExtension::Uext => return ValueConversion::Uext(self.pointer_type).into(),
                ArgumentExtension::Sext => return ValueConversion::Sext(self.pointer_type).into(),
            }
        }

        if self.regs < self.reg_limit {
            // Assign to a register.
            let reg = if ty.is_float() {
                FPR.unit(10 + self.regs as usize)
            } else {
                GPR.unit(10 + self.regs as usize)
            };
            self.regs += 1;
            ArgumentLoc::Reg(reg).into()
        } else {
            // Assign a stack location.
            let loc = ArgumentLoc::Stack(self.offset as i32);
            self.offset += u32::from(self.pointer_bytes);
            debug_assert!(self.offset <= i32::MAX as u32);
            loc.into()
        }
    }
}

/// Legalize `sig` for RISC-V.
pub fn legalize_signature(
    sig: &mut Cow<ir::Signature>,
    triple: &Triple,
    isa_flags: &settings::Flags,
    current: bool,
) {
    let bits = triple.pointer_width().unwrap().bits();

    let mut args = Args::new(bits, isa_flags.enable_e());
    if let Some(new_params) = legalize_args(&sig.params, &mut args) {
        sig.to_mut().params = new_params;
    }

    let mut rets = Args::new(bits, isa_flags.enable_e());
    if let Some(new_returns) = legalize_args(&sig.returns, &mut rets) {
        sig.to_mut().returns = new_returns;
    }

    if current {
        let ptr = Type::int(u16::from(bits)).unwrap();

        // Add the link register as an argument and return value.
        //
        // The `jalr` instruction implementing a return can technically accept the return address
        // in any register, but a micro-architecture with a return address predictor will only
        // recognize it as a return if the address is in `x1`.
        let link = AbiParam::special_reg(ptr, ArgumentPurpose::Link, GPR.unit(1));
        sig.to_mut().params.push(link);
        sig.to_mut().returns.push(link);
    }
}

/// Get register class for a type appearing in a legalized signature.
pub fn regclass_for_abi_type(ty: Type) -> RegClass {
    if ty.is_float() {
        FPR
    } else {
        GPR
    }
}

pub fn allocatable_registers(_func: &ir::Function, isa_flags: &settings::Flags) -> RegisterSet {
    let mut regs = RegisterSet::new();
    regs.take(GPR, GPR.unit(0)); // Hard-wired 0.
                                 // %x1 is the link register which is available for allocation.
    regs.take(GPR, GPR.unit(2)); // Stack pointer.
    regs.take(GPR, GPR.unit(3)); // Global pointer.
    regs.take(GPR, GPR.unit(4)); // Thread pointer.
                                 // TODO: %x8 is the frame pointer. Reserve it?

    // Remove %x16 and up for RV32E.
    if isa_flags.enable_e() {
        for u in 16..32 {
            regs.take(GPR, GPR.unit(u));
        }
    }

    regs
}
