use super::regs;
use crate::{
    abi::{align_to, ABIOperand, ABIParams, ABIResults, ABISig, ParamsOrReturns, ABI},
    isa::{reg::Reg, CallingConvention},
    masm::OperandSize,
};
use smallvec::SmallVec;
use wasmtime_environ::{WasmFuncType, WasmHeapType, WasmType};

/// Helper environment to track argument-register
/// assignment in x64.
///
/// The first element tracks the general purpose register index.
/// The second element tracks the floating point register index.
#[derive(Default)]
struct RegIndexEnv {
    /// General purpose register index or the field used for absolute
    /// counts.
    gpr_or_absolute_count: u8,
    /// Floating point register index.
    fpr: u8,
    /// Whether the count should be absolute rather than per register class.
    /// When this field is true, only the `gpr_or_absolute_count` field is
    /// incremented.
    absolute_count: bool,
}

impl RegIndexEnv {
    fn with_absolute_count() -> Self {
        Self {
            gpr_or_absolute_count: 0,
            fpr: 0,
            absolute_count: true,
        }
    }
}

impl RegIndexEnv {
    fn next_gpr(&mut self) -> u8 {
        Self::increment(&mut self.gpr_or_absolute_count)
    }

    fn next_fpr(&mut self) -> u8 {
        if self.absolute_count {
            Self::increment(&mut self.gpr_or_absolute_count)
        } else {
            Self::increment(&mut self.fpr)
        }
    }

    fn increment(index: &mut u8) -> u8 {
        let current = *index;
        *index += 1;
        current
    }
}

#[derive(Default)]
pub(crate) struct X64ABI;

impl ABI for X64ABI {
    // TODO: change to 16 once SIMD is supported
    fn stack_align() -> u8 {
        8
    }

    fn call_stack_align() -> u8 {
        16
    }

    fn arg_base_offset() -> u8 {
        // Two 8-byte slots, one for the return address and another
        // one for the frame pointer.
        // ┌──────────┬───────── Argument base
        // │   Ret    │
        // │   Addr   │
        // ├──────────┼
        // │          │
        // │   FP     │
        // └──────────┴
        16
    }

    fn ret_addr_offset() -> u8 {
        // 1 8-byte slot.
        // ┌──────────┬
        // │   Ret    │
        // │   Addr   │
        // ├──────────┼ * offset
        // │          │
        // │   FP     │
        // └──────────┴
        8
    }

    fn word_bits() -> u32 {
        64
    }

    fn sig_from(
        params: &[WasmType],
        returns: &[WasmType],
        call_conv: &CallingConvention,
    ) -> ABISig {
        assert!(call_conv.is_fastcall() || call_conv.is_systemv() || call_conv.is_default());
        let is_fastcall = call_conv.is_fastcall();
        // In the fastcall calling convention, the callee gets a contiguous
        // stack area of 32 bytes (4 register arguments) just before its frame.
        // See
        // https://learn.microsoft.com/en-us/cpp/build/stack-usage?view=msvc-170#stack-allocation
        let (params_stack_offset, mut params_index_env) = if is_fastcall {
            (32, RegIndexEnv::with_absolute_count())
        } else {
            (0, RegIndexEnv::default())
        };

        let results = Self::abi_results(returns, call_conv);
        let params = ABIParams::from::<_, Self>(
            params,
            params_stack_offset,
            results.on_stack(),
            |ty, stack_offset| {
                Self::to_abi_operand(
                    ty,
                    stack_offset,
                    &mut params_index_env,
                    call_conv,
                    ParamsOrReturns::Params,
                )
            },
        );

        ABISig::new(params, results)
    }

    fn sig(wasm_sig: &WasmFuncType, call_conv: &CallingConvention) -> ABISig {
        Self::sig_from(wasm_sig.params(), wasm_sig.returns(), call_conv)
    }

    fn abi_results(returns: &[WasmType], call_conv: &CallingConvention) -> ABIResults {
        // Use absolute count for results given that for Winch's
        // default CallingConvention only one register is used for results
        // independent of the register class. This also aligns with how
        // multiple results are handled by Wasmtime.
        let mut results_index_env = RegIndexEnv::with_absolute_count();
        ABIResults::from(returns, call_conv, |ty, offset| {
            Self::to_abi_operand(
                ty,
                offset,
                &mut results_index_env,
                call_conv,
                ParamsOrReturns::Returns,
            )
        })
    }

    fn scratch_reg() -> Reg {
        regs::scratch()
    }

    fn float_scratch_reg() -> Reg {
        regs::scratch_xmm()
    }

    fn fp_reg() -> Reg {
        regs::rbp()
    }

    fn sp_reg() -> Reg {
        regs::rsp()
    }

    fn vmctx_reg() -> Reg {
        regs::vmctx()
    }

    fn callee_saved_regs(call_conv: &CallingConvention) -> SmallVec<[(Reg, OperandSize); 18]> {
        regs::callee_saved(call_conv)
    }

    fn stack_slot_size() -> u32 {
        Self::word_bytes()
    }

    fn sizeof(ty: &WasmType) -> u32 {
        match ty {
            WasmType::Ref(rt) => match rt.heap_type {
                WasmHeapType::Func => Self::word_bytes(),
                ht => unimplemented!("Support for WasmHeapType: {ht}"),
            },
            WasmType::F64 | WasmType::I64 => Self::word_bytes(),
            WasmType::F32 | WasmType::I32 => Self::word_bytes() / 2,
            ty => unimplemented!("Support for WasmType: {ty}"),
        }
    }
}

impl X64ABI {
    fn to_abi_operand(
        wasm_arg: &WasmType,
        stack_offset: u32,
        index_env: &mut RegIndexEnv,
        call_conv: &CallingConvention,
        params_or_returns: ParamsOrReturns,
    ) -> (ABIOperand, u32) {
        let (reg, ty) = match wasm_arg {
            ty @ WasmType::Ref(rt) => match rt.heap_type {
                WasmHeapType::Func => (
                    Self::int_reg_for(index_env.next_gpr(), call_conv, params_or_returns),
                    ty,
                ),
                ht => unimplemented!("Support for WasmHeapType: {ht}"),
            },

            ty @ (WasmType::I32 | WasmType::I64) => (
                Self::int_reg_for(index_env.next_gpr(), call_conv, params_or_returns),
                ty,
            ),

            ty @ (WasmType::F32 | WasmType::F64) => (
                Self::float_reg_for(index_env.next_fpr(), call_conv, params_or_returns),
                ty,
            ),

            ty => unimplemented!("Support for argument of WasmType: {ty}"),
        };

        let ty_size = <Self as ABI>::sizeof(wasm_arg);
        let default = || {
            let arg = ABIOperand::stack_offset(stack_offset, *ty, ty_size);
            let slot_size = Self::stack_slot_size();
            // Stack slots for parameters are aligned to a fixed slot size,
            // in the case of x64, 8 bytes.
            // Stack slots for returns are type-size aligned.
            let next_stack = if params_or_returns == ParamsOrReturns::Params {
                align_to(stack_offset, slot_size) + slot_size
            } else {
                // For the default calling convention, we don't type-size align,
                // given that results on the stack must match spills generated
                // from within the compiler, which are not type-size aligned.
                if call_conv.is_default() {
                    stack_offset + ty_size
                } else {
                    align_to(stack_offset, ty_size) + ty_size
                }
            };
            (arg, next_stack)
        };

        reg.map_or_else(default, |reg| {
            (ABIOperand::reg(reg, *ty, ty_size), stack_offset)
        })
    }

    fn int_reg_for(
        index: u8,
        call_conv: &CallingConvention,
        params_or_returns: ParamsOrReturns,
    ) -> Option<Reg> {
        use ParamsOrReturns::*;

        if call_conv.is_fastcall() {
            return match (index, params_or_returns) {
                (0, Params) => Some(regs::rcx()),
                (1, Params) => Some(regs::rdx()),
                (2, Params) => Some(regs::r8()),
                (3, Params) => Some(regs::r9()),
                (0, Returns) => Some(regs::rax()),
                _ => None,
            };
        }

        if call_conv.is_systemv() || call_conv.is_default() {
            return match (index, params_or_returns) {
                (0, Params) => Some(regs::rdi()),
                (1, Params) => Some(regs::rsi()),
                (2, Params) => Some(regs::rdx()),
                (3, Params) => Some(regs::rcx()),
                (4, Params) => Some(regs::r8()),
                (5, Params) => Some(regs::r9()),
                (0, Returns) => Some(regs::rax()),
                _ => None,
            };
        }

        None
    }

    fn float_reg_for(
        index: u8,
        call_conv: &CallingConvention,
        params_or_returns: ParamsOrReturns,
    ) -> Option<Reg> {
        use ParamsOrReturns::*;
        if call_conv.is_fastcall() {
            return match (index, params_or_returns) {
                (0, Params) => Some(regs::xmm0()),
                (1, Params) => Some(regs::xmm1()),
                (2, Params) => Some(regs::xmm2()),
                (3, Params) => Some(regs::xmm3()),
                (0, Returns) => Some(regs::xmm0()),
                _ => None,
            };
        }

        if call_conv.is_systemv() || call_conv.is_default() {
            return match (index, params_or_returns) {
                (0, Params) => Some(regs::xmm0()),
                (1, Params) => Some(regs::xmm1()),
                (2, Params) => Some(regs::xmm2()),
                (3, Params) => Some(regs::xmm3()),
                (4, Params) => Some(regs::xmm4()),
                (5, Params) => Some(regs::xmm5()),
                (6, Params) => Some(regs::xmm6()),
                (7, Params) => Some(regs::xmm7()),
                (0, Returns) => Some(regs::xmm0()),
                _ => None,
            };
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::{RegIndexEnv, X64ABI};
    use crate::{
        abi::{ABIOperand, ABI},
        isa::reg::Reg,
        isa::x64::regs,
        isa::CallingConvention,
    };
    use wasmtime_environ::{
        WasmFuncType,
        WasmType::{self, *},
    };

    #[test]
    fn test_get_next_reg_index() {
        let mut index_env = RegIndexEnv::default();
        assert_eq!(index_env.next_fpr(), 0);
        assert_eq!(index_env.next_gpr(), 0);
        assert_eq!(index_env.next_fpr(), 1);
        assert_eq!(index_env.next_gpr(), 1);
        assert_eq!(index_env.next_fpr(), 2);
        assert_eq!(index_env.next_gpr(), 2);
    }

    #[test]
    fn test_reg_index_env_absolute_count() {
        let mut e = RegIndexEnv::with_absolute_count();
        assert!(e.next_gpr() == 0);
        assert!(e.next_fpr() == 1);
        assert!(e.next_gpr() == 2);
        assert!(e.next_fpr() == 3);
    }

    #[test]
    fn int_abi_sig() {
        let wasm_sig =
            WasmFuncType::new([I32, I64, I32, I64, I32, I32, I64, I32].into(), [].into());

        let sig = X64ABI::sig(&wasm_sig, &CallingConvention::Default);
        let params = sig.params;

        match_reg_arg(params.get(0).unwrap(), I32, regs::rdi());
        match_reg_arg(params.get(1).unwrap(), I64, regs::rsi());
        match_reg_arg(params.get(2).unwrap(), I32, regs::rdx());
        match_reg_arg(params.get(3).unwrap(), I64, regs::rcx());
        match_reg_arg(params.get(4).unwrap(), I32, regs::r8());
        match_reg_arg(params.get(5).unwrap(), I32, regs::r9());
        match_stack_arg(params.get(6).unwrap(), I64, 0);
        match_stack_arg(params.get(7).unwrap(), I32, 8);
    }

    #[test]
    fn int_abi_sig_multi_returns() {
        let wasm_sig = WasmFuncType::new(
            [I32, I64, I32, I64, I32, I32, I64, I32].into(),
            [I32, I32, I32].into(),
        );

        let sig = X64ABI::sig(&wasm_sig, &CallingConvention::Default);
        let params = sig.params;
        let results = sig.results;

        match_reg_arg(params.get(0).unwrap(), I32, regs::rdi());
        match_reg_arg(params.get(1).unwrap(), I64, regs::rsi());
        match_reg_arg(params.get(2).unwrap(), I32, regs::rdx());
        match_reg_arg(params.get(3).unwrap(), I64, regs::rcx());
        match_reg_arg(params.get(4).unwrap(), I32, regs::r8());
        match_reg_arg(params.get(5).unwrap(), I32, regs::r9());
        match_stack_arg(params.get(6).unwrap(), I64, 0);
        match_stack_arg(params.get(7).unwrap(), I32, 8);

        match_stack_arg(results.get(0).unwrap(), I32, 4);
        match_stack_arg(results.get(1).unwrap(), I32, 0);
        match_reg_arg(results.get(2).unwrap(), I32, regs::rax());
    }

    #[test]
    fn float_abi_sig() {
        let wasm_sig = WasmFuncType::new(
            [F32, F64, F32, F64, F32, F32, F64, F32, F64].into(),
            [].into(),
        );

        let sig = X64ABI::sig(&wasm_sig, &CallingConvention::Default);
        let params = sig.params;

        match_reg_arg(params.get(0).unwrap(), F32, regs::xmm0());
        match_reg_arg(params.get(1).unwrap(), F64, regs::xmm1());
        match_reg_arg(params.get(2).unwrap(), F32, regs::xmm2());
        match_reg_arg(params.get(3).unwrap(), F64, regs::xmm3());
        match_reg_arg(params.get(4).unwrap(), F32, regs::xmm4());
        match_reg_arg(params.get(5).unwrap(), F32, regs::xmm5());
        match_reg_arg(params.get(6).unwrap(), F64, regs::xmm6());
        match_reg_arg(params.get(7).unwrap(), F32, regs::xmm7());
        match_stack_arg(params.get(8).unwrap(), F64, 0);
    }

    #[test]
    fn mixed_abi_sig() {
        let wasm_sig = WasmFuncType::new(
            [F32, I32, I64, F64, I32, F32, F64, F32, F64].into(),
            [].into(),
        );

        let sig = X64ABI::sig(&wasm_sig, &CallingConvention::Default);
        let params = sig.params;

        match_reg_arg(params.get(0).unwrap(), F32, regs::xmm0());
        match_reg_arg(params.get(1).unwrap(), I32, regs::rdi());
        match_reg_arg(params.get(2).unwrap(), I64, regs::rsi());
        match_reg_arg(params.get(3).unwrap(), F64, regs::xmm1());
        match_reg_arg(params.get(4).unwrap(), I32, regs::rdx());
        match_reg_arg(params.get(5).unwrap(), F32, regs::xmm2());
        match_reg_arg(params.get(6).unwrap(), F64, regs::xmm3());
        match_reg_arg(params.get(7).unwrap(), F32, regs::xmm4());
        match_reg_arg(params.get(8).unwrap(), F64, regs::xmm5());
    }

    #[test]
    fn system_v_call_conv() {
        let wasm_sig = WasmFuncType::new(
            [F32, I32, I64, F64, I32, F32, F64, F32, F64].into(),
            [].into(),
        );

        let sig = X64ABI::sig(&wasm_sig, &CallingConvention::SystemV);
        let params = sig.params;

        match_reg_arg(params.get(0).unwrap(), F32, regs::xmm0());
        match_reg_arg(params.get(1).unwrap(), I32, regs::rdi());
        match_reg_arg(params.get(2).unwrap(), I64, regs::rsi());
        match_reg_arg(params.get(3).unwrap(), F64, regs::xmm1());
        match_reg_arg(params.get(4).unwrap(), I32, regs::rdx());
        match_reg_arg(params.get(5).unwrap(), F32, regs::xmm2());
        match_reg_arg(params.get(6).unwrap(), F64, regs::xmm3());
        match_reg_arg(params.get(7).unwrap(), F32, regs::xmm4());
        match_reg_arg(params.get(8).unwrap(), F64, regs::xmm5());
    }

    #[test]
    fn fastcall_call_conv() {
        let wasm_sig = WasmFuncType::new(
            [F32, I32, I64, F64, I32, F32, F64, F32, F64].into(),
            [].into(),
        );

        let sig = X64ABI::sig(&wasm_sig, &CallingConvention::WindowsFastcall);
        let params = sig.params;

        match_reg_arg(params.get(0).unwrap(), F32, regs::xmm0());
        match_reg_arg(params.get(1).unwrap(), I32, regs::rdx());
        match_reg_arg(params.get(2).unwrap(), I64, regs::r8());
        match_reg_arg(params.get(3).unwrap(), F64, regs::xmm3());
        match_stack_arg(params.get(4).unwrap(), I32, 32);
        match_stack_arg(params.get(5).unwrap(), F32, 40);
    }

    #[test]
    fn fastcall_call_conv_multi_returns() {
        let wasm_sig = WasmFuncType::new(
            [F32, I32, I64, F64, I32, F32, F64, F32, F64].into(),
            [I32, F32, I32, F32, I64].into(),
        );

        let sig = X64ABI::sig(&wasm_sig, &CallingConvention::WindowsFastcall);
        let params = sig.params;
        let results = sig.results;

        match_reg_arg(params.get(0).unwrap(), F32, regs::xmm0());
        match_reg_arg(params.get(1).unwrap(), I32, regs::rdx());
        match_reg_arg(params.get(2).unwrap(), I64, regs::r8());
        match_reg_arg(params.get(3).unwrap(), F64, regs::xmm3());
        // Each argument stack slot is 8 bytes.
        match_stack_arg(params.get(4).unwrap(), I32, 32);
        match_stack_arg(params.get(5).unwrap(), F32, 40);

        match_reg_arg(results.get(0).unwrap(), I32, regs::rax());

        match_stack_arg(results.get(1).unwrap(), F32, 0);
        match_stack_arg(results.get(2).unwrap(), I32, 4);
        match_stack_arg(results.get(3).unwrap(), F32, 8);
        match_stack_arg(results.get(4).unwrap(), I64, 12);
    }

    #[cfg(test)]
    fn match_reg_arg(abi_arg: &ABIOperand, expected_ty: WasmType, expected_reg: Reg) {
        match abi_arg {
            &ABIOperand::Reg { reg, ty, .. } => {
                assert_eq!(reg, expected_reg);
                assert_eq!(ty, expected_ty);
            }
            stack => panic!("Expected reg argument, got {:?}", stack),
        }
    }

    #[cfg(test)]
    fn match_stack_arg(abi_arg: &ABIOperand, expected_ty: WasmType, expected_offset: u32) {
        match abi_arg {
            &ABIOperand::Stack { offset, ty, .. } => {
                assert_eq!(offset, expected_offset);
                assert_eq!(ty, expected_ty);
            }
            reg => panic!("Expected stack argument, got {:?}", reg),
        }
    }
}
