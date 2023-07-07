use super::regs;
use crate::abi::{ABIArg, ABIResult, ABISig, ABI};
use crate::isa::{reg::Reg, CallingConvention};
use smallvec::SmallVec;
use wasmtime_environ::{WasmFuncType, WasmType};

#[derive(Default)]
pub(crate) struct Aarch64ABI;

/// Helper environment to track argument-register
/// assignment in aarch64.
///
/// The first element tracks the general purpose register index, capped at 7 (x0-x7).
/// The second element tracks the floating point register index, capped at 7 (v0-v7).
// Follows
// https://github.com/ARM-software/abi-aa/blob/2021Q1/aapcs64/aapcs64.rst#64parameter-passing
#[derive(Default)]
struct RegIndexEnv(u8, u8);

impl RegIndexEnv {
    fn next_xreg(&mut self) -> Option<u8> {
        if self.0 < 8 {
            return Some(Self::increment(&mut self.0));
        }

        None
    }

    fn next_vreg(&mut self) -> Option<u8> {
        if self.1 < 8 {
            return Some(Self::increment(&mut self.1));
        }

        None
    }

    fn increment(index: &mut u8) -> u8 {
        let current = *index;
        *index += 1;
        current
    }
}

impl ABI for Aarch64ABI {
    // TODO change to 16 once SIMD is supported
    fn stack_align() -> u8 {
        8
    }

    fn call_stack_align() -> u8 {
        16
    }

    fn arg_base_offset() -> u8 {
        16
    }

    fn ret_addr_offset() -> u8 {
        8
    }

    fn word_bits() -> u32 {
        64
    }

    fn sig(wasm_sig: &WasmFuncType, call_conv: &CallingConvention) -> ABISig {
        assert!(call_conv.is_apple_aarch64() || call_conv.is_default());

        if wasm_sig.returns().len() > 1 {
            panic!("multi-value not supported");
        }

        let mut stack_offset = 0;
        let mut index_env = RegIndexEnv::default();

        let params: SmallVec<[ABIArg; 6]> = wasm_sig
            .params()
            .iter()
            .map(|arg| Self::to_abi_arg(arg, &mut stack_offset, &mut index_env))
            .collect();

        let result = Self::result(wasm_sig.returns(), call_conv);
        ABISig::new(params, result, stack_offset)
    }

    fn result(returns: &[WasmType], _call_conv: &CallingConvention) -> ABIResult {
        // NOTE temporarily defaulting to x0;
        let reg = regs::xreg(0);

        // This invariant will be lifted once support for multi-value is added.
        assert!(returns.len() <= 1, "multi-value not supported");

        let ty = returns.get(0).copied();
        ABIResult::reg(ty, reg)
    }

    fn scratch_reg() -> Reg {
        todo!()
    }

    fn sp_reg() -> Reg {
        todo!()
    }

    fn fp_reg() -> Reg {
        regs::fp()
    }

    fn vmctx_reg() -> Reg {
        regs::xreg(9)
    }

    fn callee_saved_regs(_call_conv: &CallingConvention) -> SmallVec<[Reg; 9]> {
        regs::callee_saved()
    }
}

impl Aarch64ABI {
    fn to_abi_arg(
        wasm_arg: &WasmType,
        stack_offset: &mut u32,
        index_env: &mut RegIndexEnv,
    ) -> ABIArg {
        let (reg, ty) = match wasm_arg {
            ty @ (WasmType::I32 | WasmType::I64) => (index_env.next_xreg().map(regs::xreg), ty),

            ty @ (WasmType::F32 | WasmType::F64) => (index_env.next_vreg().map(regs::vreg), ty),

            ty => unreachable!("Unsupported argument type {:?}", ty),
        };

        let ty = *ty;
        let default = || {
            let size = Self::word_bytes();
            let arg = ABIArg::stack_offset(*stack_offset, ty);
            *stack_offset += size;
            arg
        };
        reg.map_or_else(default, |reg| ABIArg::Reg { ty, reg })
    }
}

#[cfg(test)]
mod tests {
    use super::{Aarch64ABI, RegIndexEnv};
    use crate::{
        abi::{ABIArg, ABI},
        isa::aarch64::regs,
        isa::reg::Reg,
        isa::CallingConvention,
    };
    use wasmtime_environ::{
        WasmFuncType,
        WasmType::{self, *},
    };

    #[test]
    fn test_get_next_reg_index() {
        let mut index_env = RegIndexEnv::default();
        assert_eq!(index_env.next_xreg(), Some(0));
        assert_eq!(index_env.next_vreg(), Some(0));
        assert_eq!(index_env.next_xreg(), Some(1));
        assert_eq!(index_env.next_vreg(), Some(1));
        assert_eq!(index_env.next_xreg(), Some(2));
        assert_eq!(index_env.next_vreg(), Some(2));
    }

    #[test]
    fn xreg_abi_sig() {
        let wasm_sig = WasmFuncType::new(
            [I32, I64, I32, I64, I32, I32, I64, I32, I64].into(),
            [].into(),
        );

        let sig = Aarch64ABI::sig(&wasm_sig, &CallingConvention::Default);
        let params = sig.params;

        match_reg_arg(params.get(0).unwrap(), I32, regs::xreg(0));
        match_reg_arg(params.get(1).unwrap(), I64, regs::xreg(1));
        match_reg_arg(params.get(2).unwrap(), I32, regs::xreg(2));
        match_reg_arg(params.get(3).unwrap(), I64, regs::xreg(3));
        match_reg_arg(params.get(4).unwrap(), I32, regs::xreg(4));
        match_reg_arg(params.get(5).unwrap(), I32, regs::xreg(5));
        match_reg_arg(params.get(6).unwrap(), I64, regs::xreg(6));
        match_reg_arg(params.get(7).unwrap(), I32, regs::xreg(7));
        match_stack_arg(params.get(8).unwrap(), I64, 0);
    }

    #[test]
    fn vreg_abi_sig() {
        let wasm_sig = WasmFuncType::new(
            [F32, F64, F32, F64, F32, F32, F64, F32, F64].into(),
            [].into(),
        );

        let sig = Aarch64ABI::sig(&wasm_sig, &CallingConvention::Default);
        let params = sig.params;

        match_reg_arg(params.get(0).unwrap(), F32, regs::vreg(0));
        match_reg_arg(params.get(1).unwrap(), F64, regs::vreg(1));
        match_reg_arg(params.get(2).unwrap(), F32, regs::vreg(2));
        match_reg_arg(params.get(3).unwrap(), F64, regs::vreg(3));
        match_reg_arg(params.get(4).unwrap(), F32, regs::vreg(4));
        match_reg_arg(params.get(5).unwrap(), F32, regs::vreg(5));
        match_reg_arg(params.get(6).unwrap(), F64, regs::vreg(6));
        match_reg_arg(params.get(7).unwrap(), F32, regs::vreg(7));
        match_stack_arg(params.get(8).unwrap(), F64, 0);
    }

    #[test]
    fn mixed_abi_sig() {
        let wasm_sig = WasmFuncType::new(
            [F32, I32, I64, F64, I32, F32, F64, F32, F64].into(),
            [].into(),
        );

        let sig = Aarch64ABI::sig(&wasm_sig, &CallingConvention::Default);
        let params = sig.params;

        match_reg_arg(params.get(0).unwrap(), F32, regs::vreg(0));
        match_reg_arg(params.get(1).unwrap(), I32, regs::xreg(0));
        match_reg_arg(params.get(2).unwrap(), I64, regs::xreg(1));
        match_reg_arg(params.get(3).unwrap(), F64, regs::vreg(1));
        match_reg_arg(params.get(4).unwrap(), I32, regs::xreg(2));
        match_reg_arg(params.get(5).unwrap(), F32, regs::vreg(2));
        match_reg_arg(params.get(6).unwrap(), F64, regs::vreg(3));
        match_reg_arg(params.get(7).unwrap(), F32, regs::vreg(4));
        match_reg_arg(params.get(8).unwrap(), F64, regs::vreg(5));
    }

    fn match_reg_arg(abi_arg: &ABIArg, expected_ty: WasmType, expected_reg: Reg) {
        match abi_arg {
            &ABIArg::Reg { reg, ty } => {
                assert_eq!(reg, expected_reg);
                assert_eq!(ty, expected_ty);
            }
            stack => panic!("Expected reg argument, got {:?}", stack),
        }
    }

    fn match_stack_arg(abi_arg: &ABIArg, expected_ty: WasmType, expected_offset: u32) {
        match abi_arg {
            &ABIArg::Stack { offset, ty } => {
                assert_eq!(offset, expected_offset);
                assert_eq!(ty, expected_ty);
            }
            stack => panic!("Expected stack argument, got {:?}", stack),
        }
    }
}
