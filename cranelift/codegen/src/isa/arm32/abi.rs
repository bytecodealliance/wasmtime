//! ARM ABI implementation.
//! This is from the RISC-V target and will need to be updated for ARM32.

use super::registers::{D, GPR, Q, S};
use crate::abi::{legalize_args, ArgAction, ArgAssigner, ValueConversion};
use crate::ir::{self, AbiParam, ArgumentExtension, ArgumentLoc, Type};
use crate::isa::RegClass;
use crate::regalloc::RegisterSet;
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
    fn new(bits: u8) -> Self {
        Self {
            pointer_bits: bits,
            pointer_bytes: bits / 8,
            pointer_type: Type::int(u16::from(bits)).unwrap(),
            regs: 0,
            reg_limit: 8,
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
        // SIMD instructions are currently no implemented, so break down vectors
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
            let reg = GPR.unit(10 + self.regs as usize);
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

/// Legalize `sig`.
pub fn legalize_signature(sig: &mut ir::Signature, triple: &Triple, _current: bool) {
    let bits = triple.pointer_width().unwrap().bits();

    let mut args = Args::new(bits);
    legalize_args(&mut sig.params, &mut args);
}

/// Get register class for a type appearing in a legalized signature.
pub fn regclass_for_abi_type(ty: ir::Type) -> RegClass {
    if ty.is_int() {
        GPR
    } else {
        match ty.bits() {
            32 => S,
            64 => D,
            128 => Q,
            _ => panic!("Unexpected {} ABI type for arm32", ty),
        }
    }
}

/// Get the set of allocatable registers for `func`.
pub fn allocatable_registers(_func: &ir::Function) -> RegisterSet {
    unimplemented!()
}
