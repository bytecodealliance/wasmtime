use super::regs;
use crate::{
    abi::{ABIArg, ABIResult, ABISig, ABI},
    isa::{reg::Reg, CallingConvention},
};
use smallvec::SmallVec;
use wasmtime_environ::{WasmFuncType, WasmType};

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

    fn sig(wasm_sig: &WasmFuncType, call_conv: &CallingConvention) -> ABISig {
        assert!(call_conv.is_fastcall() || call_conv.is_systemv() || call_conv.is_default());

        if wasm_sig.returns().len() > 1 {
            panic!("multi-value not supported");
        }

        let is_fastcall = call_conv.is_fastcall();
        // In the fastcall calling convention, the callee gets a contiguous
        // stack area of 32 bytes (4 register arguments) just before its frame.
        // See
        // https://learn.microsoft.com/en-us/cpp/build/stack-usage?view=msvc-170#stack-allocation
        let (mut stack_offset, mut index_env) = if is_fastcall {
            (32, RegIndexEnv::with_absolute_count())
        } else {
            (0, RegIndexEnv::default())
        };

        let params: SmallVec<[ABIArg; 6]> = wasm_sig
            .params()
            .iter()
            .map(|arg| Self::to_abi_arg(arg, &mut stack_offset, &mut index_env, is_fastcall))
            .collect();

        let result = Self::result(wasm_sig.returns(), call_conv);
        ABISig::new(params, result, stack_offset)
    }

    fn result(returns: &[WasmType], _call_conv: &CallingConvention) -> ABIResult {
        // The `Default`, `WasmtimeFastcall` and `WasmtimeSystemV use `rax`.
        // NOTE This should be updated when supporting multi-value.
        let reg = regs::rax();
        // This invariant will be lifted once support for multi-value is added.
        assert!(returns.len() <= 1, "multi-value not supported");

        let ty = returns.get(0).copied();
        ABIResult::reg(ty, reg)
    }

    fn scratch_reg() -> Reg {
        regs::scratch()
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

    fn callee_saved_regs(call_conv: &CallingConvention) -> SmallVec<[Reg; 9]> {
        regs::callee_saved(call_conv)
    }
}

impl X64ABI {
    fn to_abi_arg(
        wasm_arg: &WasmType,
        stack_offset: &mut u32,
        index_env: &mut RegIndexEnv,
        fastcall: bool,
    ) -> ABIArg {
        let (reg, ty) = match wasm_arg {
            ty @ (WasmType::I32 | WasmType::I64) => {
                (Self::int_reg_for(index_env.next_gpr(), fastcall), ty)
            }

            ty @ (WasmType::F32 | WasmType::F64) => {
                (Self::float_reg_for(index_env.next_fpr(), fastcall), ty)
            }

            ty => unreachable!("Unsupported argument type {:?}", ty),
        };

        let default = || {
            let arg = ABIArg::stack_offset(*stack_offset, *ty);
            let size = Self::word_bytes();
            *stack_offset += size;
            arg
        };

        reg.map_or_else(default, |reg| ABIArg::reg(reg, *ty))
    }

    fn int_reg_for(index: u8, fastcall: bool) -> Option<Reg> {
        match (fastcall, index) {
            (false, 0) => Some(regs::rdi()),
            (false, 1) => Some(regs::rsi()),
            (false, 2) => Some(regs::rdx()),
            (false, 3) => Some(regs::rcx()),
            (false, 4) => Some(regs::r8()),
            (false, 5) => Some(regs::r9()),
            (true, 0) => Some(regs::rcx()),
            (true, 1) => Some(regs::rdx()),
            (true, 2) => Some(regs::r8()),
            (true, 3) => Some(regs::r9()),
            _ => None,
        }
    }

    fn float_reg_for(index: u8, fastcall: bool) -> Option<Reg> {
        match (fastcall, index) {
            (false, 0) => Some(regs::xmm0()),
            (false, 1) => Some(regs::xmm1()),
            (false, 2) => Some(regs::xmm2()),
            (false, 3) => Some(regs::xmm3()),
            (false, 4) => Some(regs::xmm4()),
            (false, 5) => Some(regs::xmm5()),
            (false, 6) => Some(regs::xmm6()),
            (false, 7) => Some(regs::xmm7()),
            (true, 0) => Some(regs::xmm0()),
            (true, 1) => Some(regs::xmm1()),
            (true, 2) => Some(regs::xmm2()),
            (true, 3) => Some(regs::xmm3()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{RegIndexEnv, X64ABI};
    use crate::{
        abi::{ABIArg, ABI},
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

        let sig = X64ABI::sig(&wasm_sig, &CallingConvention::WasmtimeSystemV);
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
