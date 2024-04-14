use super::regs;
use crate::abi::{align_to, ABIOperand, ABIParams, ABIResults, ABISig, ParamsOrReturns, ABI};
use crate::isa::{reg::Reg, CallingConvention};
use wasmtime_environ::{WasmHeapType, WasmValType};

#[derive(Default)]
pub(crate) struct Aarch64ABI;

/// Helper environment to track argument-register
/// assignment in aarch64.
///
/// The first element tracks the general purpose register index, capped at 7 (x0-x7).
/// The second element tracks the floating point register index, capped at 7 (v0-v7).
// Follows
// https://github.com/ARM-software/abi-aa/blob/2021Q1/aapcs64/aapcs64.rst#64parameter-passing
struct RegIndexEnv {
    xregs: u8,
    vregs: u8,
    limit: u8,
}

impl Default for RegIndexEnv {
    fn default() -> Self {
        Self {
            xregs: 0,
            vregs: 0,
            limit: 8,
        }
    }
}

impl RegIndexEnv {
    fn with_limit(limit: u8) -> Self {
        let mut default = Self::default();
        default.limit = limit;
        default
    }

    fn next_xreg(&mut self) -> Option<u8> {
        if self.xregs < self.limit {
            return Some(Self::increment(&mut self.xregs));
        }

        None
    }

    fn next_vreg(&mut self) -> Option<u8> {
        if self.vregs < self.limit {
            return Some(Self::increment(&mut self.vregs));
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

    fn word_bits() -> u8 {
        64
    }

    fn sig_from(
        params: &[WasmValType],
        returns: &[WasmValType],
        call_conv: &CallingConvention,
    ) -> ABISig {
        assert!(call_conv.is_apple_aarch64() || call_conv.is_default());

        let mut params_index_env = RegIndexEnv::default();
        let results = Self::abi_results(returns, call_conv);
        let params =
            ABIParams::from::<_, Self>(params, 0, results.on_stack(), |ty, stack_offset| {
                Self::to_abi_operand(
                    ty,
                    stack_offset,
                    &mut params_index_env,
                    ParamsOrReturns::Params,
                )
            });

        ABISig::new(params, results)
    }

    fn abi_results(returns: &[WasmValType], call_conv: &CallingConvention) -> ABIResults {
        assert!(call_conv.is_apple_aarch64() || call_conv.is_default());

        let mut returns_index_env = RegIndexEnv::with_limit(1);
        ABIResults::from(returns, call_conv, |ty, stack_offset| {
            Self::to_abi_operand(
                ty,
                stack_offset,
                &mut returns_index_env,
                ParamsOrReturns::Returns,
            )
        })
    }

    fn scratch_reg() -> Reg {
        regs::scratch()
    }

    fn float_scratch_reg() -> Reg {
        regs::float_scratch()
    }

    fn vmctx_reg() -> Reg {
        regs::xreg(9)
    }

    fn stack_slot_size() -> u8 {
        Self::word_bytes()
    }

    fn sizeof(ty: &WasmValType) -> u8 {
        match ty {
            WasmValType::Ref(rt) => match rt.heap_type {
                WasmHeapType::Func => Self::word_bytes(),
                ht => unimplemented!("Support for WasmHeapType: {ht}"),
            },
            WasmValType::F64 | WasmValType::I64 => Self::word_bytes(),
            WasmValType::F32 | WasmValType::I32 => Self::word_bytes() / 2,
            ty => unimplemented!("Support for WasmType: {ty}"),
        }
    }
}

impl Aarch64ABI {
    fn to_abi_operand(
        wasm_arg: &WasmValType,
        stack_offset: u32,
        index_env: &mut RegIndexEnv,
        params_or_returns: ParamsOrReturns,
    ) -> (ABIOperand, u32) {
        let (reg, ty) = match wasm_arg {
            ty @ (WasmValType::I32 | WasmValType::I64) => {
                (index_env.next_xreg().map(regs::xreg), ty)
            }

            ty @ (WasmValType::F32 | WasmValType::F64) => {
                (index_env.next_vreg().map(regs::vreg), ty)
            }

            ty => unreachable!("Unsupported argument type {:?}", ty),
        };

        let ty_size = <Self as ABI>::sizeof(wasm_arg);
        let default = || {
            let arg = ABIOperand::stack_offset(stack_offset, *ty, ty_size as u32);
            let slot_size = Self::stack_slot_size();
            // Stack slots for parameters are aligned to a fixed slot size,
            // in the case of Aarch64, 8 bytes.
            // Stack slots for returns are type-size aligned.
            let next_stack = if params_or_returns == ParamsOrReturns::Params {
                align_to(stack_offset, slot_size as u32) + (slot_size as u32)
            } else {
                align_to(stack_offset, ty_size as u32) + (ty_size as u32)
            };
            (arg, next_stack)
        };
        reg.map_or_else(default, |reg| {
            (ABIOperand::reg(reg, *ty, ty_size as u32), stack_offset)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{Aarch64ABI, RegIndexEnv};
    use crate::{
        abi::{ABIOperand, ABI},
        isa::aarch64::regs,
        isa::reg::Reg,
        isa::CallingConvention,
    };
    use wasmtime_environ::{
        WasmFuncType,
        WasmValType::{self, *},
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

    fn match_reg_arg(abi_arg: &ABIOperand, expected_ty: WasmValType, expected_reg: Reg) {
        match abi_arg {
            &ABIOperand::Reg { reg, ty, .. } => {
                assert_eq!(reg, expected_reg);
                assert_eq!(ty, expected_ty);
            }
            stack => panic!("Expected reg argument, got {:?}", stack),
        }
    }

    fn match_stack_arg(abi_arg: &ABIOperand, expected_ty: WasmValType, expected_offset: u32) {
        match abi_arg {
            &ABIOperand::Stack { offset, ty, .. } => {
                assert_eq!(offset, expected_offset);
                assert_eq!(ty, expected_ty);
            }
            reg => panic!("Expected stack argument, got {:?}", reg),
        }
    }
}
