//! RISC-V ABI implementation.
//!
//! This module implements the RISC-V calling convention through the primary `legalize_signature()`
//! entry point.
//!
//! This doesn't support the soft-float ABI at the moment.

use abi::{ArgAction, ValueConversion, ArgAssigner, legalize_args};
use ir::{Signature, ArgumentType, ArgumentLoc};
use isa::riscv::registers::{GPR, FPR};
use settings as shared_settings;

struct Args {
    pointer_bits: u16,
    pointer_bytes: u32,
    regs: u32,
    offset: u32,
}

impl Args {
    fn new(bits: u16) -> Args {
        Args {
            pointer_bits: bits,
            pointer_bytes: bits as u32 / 8,
            regs: 0,
            offset: 0,
        }
    }
}

impl ArgAssigner for Args {
    fn assign(&mut self, arg: &ArgumentType) -> ArgAction {
        fn align(value: u32, to: u32) -> u32 {
            (value + to - 1) & !(to - 1)
        }

        let ty = arg.value_type;

        // Check for a legal type.
        // RISC-V doesn't have SIMD at all, so break all vectors down.
        if !ty.is_scalar() {
            return ArgAction::Convert(ValueConversion::VectorSplit);
        }

        // Large integers and booleans are broken down to fit in a register.
        if !ty.is_float() && ty.bits() > self.pointer_bits {
            // Align registers and stack to a multiple of two pointers.
            self.regs = align(self.regs, 2);
            self.offset = align(self.offset, 2 * self.pointer_bytes);
            return ArgAction::Convert(ValueConversion::IntSplit);
        }

        if self.regs < 8 {
            // Assign to a register.
            let reg = if ty.is_float() {
                FPR.unit(10 + self.regs as usize)
            } else {
                GPR.unit(10 + self.regs as usize)
            };
            self.regs += 1;
            ArgAction::Assign(ArgumentLoc::Reg(reg))
        } else {
            // Assign a stack location.
            let loc = ArgumentLoc::Stack(self.offset);
            self.offset += self.pointer_bytes;
            ArgAction::Assign(loc)
        }
    }
}

/// Legalize `sig` for RISC-V.
pub fn legalize_signature(sig: &mut Signature, flags: &shared_settings::Flags) {
    let bits = if flags.is_64bit() { 64 } else { 32 };

    let mut args = Args::new(bits);
    legalize_args(&mut sig.argument_types, &mut args);

    let mut rets = Args::new(bits);
    legalize_args(&mut sig.return_types, &mut rets);
}
