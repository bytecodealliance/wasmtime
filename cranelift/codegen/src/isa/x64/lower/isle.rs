//! ISLE integration glue code for x64 lowering.

// Pull in the ISLE generated code.
pub(crate) mod generated_code;
use crate::{ir::AtomicRmwOp, ir::types};
use generated_code::{AssemblerOutputs, Context, MInst, RegisterClass};

// Types that the generated ISLE code uses via `use super::*`.
use super::external::{CraneliftRegisters, PairedGpr, PairedXmm, isle_assembler_methods};
use super::{MergeableLoadSize, is_int_or_ref_ty, is_mergeable_load, lower_to_amode};
use crate::ir::condcodes::{FloatCC, IntCC};
use crate::ir::immediates::*;
use crate::ir::types::*;
use crate::ir::{
    BlockCall, Inst, InstructionData, LibCall, MemFlags, Opcode, TrapCode, Value, ValueList,
};
use crate::isa::x64::X64Backend;
use crate::isa::x64::inst::{ReturnCallInfo, args::*, regs};
use crate::isa::x64::lower::{InsnInput, emit_vm_call};
use crate::machinst::isle::*;
use crate::machinst::{
    ArgPair, CallArgList, CallInfo, CallRetList, InstOutput, MachInst, VCodeConstant,
    VCodeConstantData,
};
use alloc::boxed::Box;
use alloc::vec::Vec;
use cranelift_assembler_x64 as asm;
use regalloc2::PReg;

/// Type representing out-of-line data for calls. This type optional because the
/// call instruction is also used by Winch to emit calls, but the
/// `Box<CallInfo>` field is not used, it's only used by Cranelift. By making it
/// optional, we reduce the number of heap allocations in Winch.
type BoxCallInfo = Box<CallInfo<ExternalName>>;
type BoxCallIndInfo = Box<CallInfo<RegMem>>;
type BoxReturnCallInfo = Box<ReturnCallInfo<ExternalName>>;
type BoxReturnCallIndInfo = Box<ReturnCallInfo<Reg>>;
type VecArgPair = Vec<ArgPair>;
type BoxAtomic128RmwSeqArgs = Box<Atomic128RmwSeqArgs>;
type BoxAtomic128XchgSeqArgs = Box<Atomic128XchgSeqArgs>;

/// When interacting with the external assembler (see `external.rs`), we
/// need to fix the types we'll use.
type AssemblerInst = asm::Inst<CraneliftRegisters>;

#[derive(Clone)]
pub struct SinkableLoad {
    inst: Inst,
    addr_input: InsnInput,
    offset: i32,
}

/// Width-tagged wrappers around [`SinkableLoad`].
///
/// A `SinkableLoad` is a load instruction that's a candidate to be sunk (fused)
/// into another instruction as a memory operand. Because an ISLE rule can be
/// written where the *consumer* instruction expects a different-width load
/// than the original CLIF load, using an untagged `SinkableLoad` makes it easy
/// to construct a "load N bytes into an instruction that reads M bytes" bug
/// (the exact class we're catching with sized `XmmMem{N}` / `GprMem{N}`
/// elsewhere).
///
/// These newtypes pin the width at the type level: `SinkableLoadN` can only
/// be constructed via `sinkable_load_N`, which extracts only when the source
/// `Value`'s CLIF type is exactly N bits wide. A converter
/// `SinkableLoadN -> {Gpr,Xmm}MemN` is then safe.
#[derive(Clone)]
pub struct SinkableLoad8(SinkableLoad);
#[derive(Clone)]
pub struct SinkableLoad16(SinkableLoad);
#[derive(Clone)]
pub struct SinkableLoad32(SinkableLoad);
#[derive(Clone)]
pub struct SinkableLoad64(SinkableLoad);
#[derive(Clone)]
pub struct SinkableLoad128(SinkableLoad);

/// Generate the sized-operand helpers for one GPR width. Invoked inside
/// the `impl generated_code::Context for IsleContext` block.
///
/// Parameters: the bit width, the sized newtype names, and explicit names
/// for each generated method. See the invocation sites for layout.
macro_rules! sized_gpr_mem_helpers {
    (
        $n:literal, $gpr_mem_n:ident, $gpr_mem_imm_n:ident,
        $put_in_gpr_mem_n:ident, $put_in_gpr_mem_imm_n:ident,
        $gpr_to_gpr_mem_n:ident, $gpr_to_gpr_mem_imm_n:ident,
        $gpr_mem_imm_n_imm:ident,
        $gpr_mem_n_to_gpr_mem:ident, $gpr_mem_imm_n_to_gpr_mem_imm:ident
    ) => {
        fn $put_in_gpr_mem_n(&mut self, val: Value) -> $gpr_mem_n {
            let ty = self.lower_ctx.dfg().value_type(val);
            assert_eq!(
                ty.bits(),
                $n,
                "{}: expected a {}-bit value, got value of type {ty}",
                stringify!($put_in_gpr_mem_n),
                $n,
            );
            $gpr_mem_n::unwrap_new(self.put_in_reg_mem(val))
        }

        fn $put_in_gpr_mem_imm_n(&mut self, val: Value) -> $gpr_mem_imm_n {
            let ty = self.lower_ctx.dfg().value_type(val);
            assert_eq!(
                ty.bits(),
                $n,
                "{}: expected a {}-bit value, got value of type {ty}",
                stringify!($put_in_gpr_mem_imm_n),
                $n,
            );
            $gpr_mem_imm_n::unwrap_new(self.put_in_reg_mem_imm(val))
        }

        #[inline]
        fn $gpr_to_gpr_mem_n(&mut self, gpr: Gpr) -> $gpr_mem_n {
            $gpr_mem_n::from(gpr)
        }

        #[inline]
        fn $gpr_to_gpr_mem_imm_n(&mut self, gpr: Gpr) -> $gpr_mem_imm_n {
            $gpr_mem_imm_n::from(gpr)
        }

        #[inline]
        fn $gpr_mem_imm_n_imm(&mut self, simm32: u32) -> $gpr_mem_imm_n {
            $gpr_mem_imm_n::unwrap_new(RegMemImm::Imm { simm32 })
        }

        #[inline]
        fn $gpr_mem_n_to_gpr_mem(&mut self, x: &$gpr_mem_n) -> GprMem {
            GprMem::unwrap_new(x.clone().into())
        }

        #[inline]
        fn $gpr_mem_imm_n_to_gpr_mem_imm(&mut self, x: &$gpr_mem_imm_n) -> GprMemImm {
            GprMemImm::unwrap_new(x.clone().into())
        }
    };
}

/// Generate the sized-operand helpers for one XMM width. Invoked inside
/// the `impl generated_code::Context for IsleContext` block. Covers the
/// non-aligned/aligned variants plus their `Imm` forms.
macro_rules! sized_xmm_mem_helpers {
    (
        $n:literal,
        $xmm_mem_n:ident, $xmm_mem_aligned_n:ident,
        $xmm_mem_imm_n:ident, $xmm_mem_aligned_imm_n:ident,
        $put_in_xmm_mem_n:ident, $put_in_xmm_mem_aligned_n:ident,
        $put_in_xmm_mem_imm_n:ident, $put_in_xmm_mem_aligned_imm_n:ident,
        $xmm_to_xmm_mem_n:ident, $xmm_to_xmm_mem_aligned_n:ident,
        $xmm_to_xmm_mem_imm_n:ident, $xmm_to_xmm_mem_aligned_imm_n:ident,
        $xmm_mem_imm_n_imm:ident,
        $xmm_mem_n_to_xmm_mem:ident, $xmm_mem_aligned_n_to_xmm_mem_aligned:ident,
        $xmm_mem_imm_n_to_xmm_mem_imm:ident,
        $xmm_mem_aligned_imm_n_to_xmm_mem_aligned_imm:ident
    ) => {
        fn $put_in_xmm_mem_n(&mut self, val: Value) -> $xmm_mem_n {
            let ty = self.lower_ctx.dfg().value_type(val);
            assert_eq!(
                ty.bits(),
                $n,
                "{}: expected a {}-bit value, got value of type {ty}",
                stringify!($put_in_xmm_mem_n),
                $n,
            );
            $xmm_mem_n::unwrap_new(self.put_in_xmm_mem(val).to_reg_mem())
        }

        fn $put_in_xmm_mem_aligned_n(&mut self, val: Value) -> $xmm_mem_aligned_n {
            let ty = self.lower_ctx.dfg().value_type(val);
            assert_eq!(
                ty.bits(),
                $n,
                "{}: expected a {}-bit value, got value of type {ty}",
                stringify!($put_in_xmm_mem_aligned_n),
                $n,
            );
            let sized = $xmm_mem_n::unwrap_new(self.put_in_xmm_mem(val).to_reg_mem());
            // Round-trip through the un-sized aligning helper so unaligned
            // memory sources are copied to a register first.
            let aligned = self.xmm_mem_to_xmm_mem_aligned(&XmmMem::unwrap_new(sized.into()));
            $xmm_mem_aligned_n::unwrap_new(aligned.into())
        }

        fn $put_in_xmm_mem_imm_n(&mut self, val: Value) -> $xmm_mem_imm_n {
            let ty = self.lower_ctx.dfg().value_type(val);
            assert_eq!(
                ty.bits(),
                $n,
                "{}: expected a {}-bit value, got value of type {ty}",
                stringify!($put_in_xmm_mem_imm_n),
                $n,
            );
            $xmm_mem_imm_n::unwrap_new(self.put_in_xmm_mem_imm(val).into())
        }

        fn $put_in_xmm_mem_aligned_imm_n(&mut self, val: Value) -> $xmm_mem_aligned_imm_n {
            let ty = self.lower_ctx.dfg().value_type(val);
            assert_eq!(
                ty.bits(),
                $n,
                "{}: expected a {}-bit value, got value of type {ty}",
                stringify!($put_in_xmm_mem_aligned_imm_n),
                $n,
            );
            let imm = self.put_in_xmm_mem_imm(val);
            let aligned = self.xmm_mem_imm_to_xmm_mem_aligned_imm(&imm);
            $xmm_mem_aligned_imm_n::unwrap_new(aligned.into())
        }

        #[inline]
        fn $xmm_to_xmm_mem_n(&mut self, xmm: Xmm) -> $xmm_mem_n {
            $xmm_mem_n::from(xmm)
        }

        #[inline]
        fn $xmm_to_xmm_mem_aligned_n(&mut self, xmm: Xmm) -> $xmm_mem_aligned_n {
            $xmm_mem_aligned_n::from(xmm)
        }

        #[inline]
        fn $xmm_to_xmm_mem_imm_n(&mut self, xmm: Xmm) -> $xmm_mem_imm_n {
            $xmm_mem_imm_n::from(xmm)
        }

        #[inline]
        fn $xmm_to_xmm_mem_aligned_imm_n(&mut self, xmm: Xmm) -> $xmm_mem_aligned_imm_n {
            $xmm_mem_aligned_imm_n::from(xmm)
        }

        #[inline]
        fn $xmm_mem_imm_n_imm(&mut self, simm32: u32) -> $xmm_mem_imm_n {
            $xmm_mem_imm_n::unwrap_new(RegMemImm::Imm { simm32 })
        }

        #[inline]
        fn $xmm_mem_n_to_xmm_mem(&mut self, x: &$xmm_mem_n) -> XmmMem {
            XmmMem::unwrap_new(x.clone().into())
        }

        #[inline]
        fn $xmm_mem_aligned_n_to_xmm_mem_aligned(
            &mut self,
            x: &$xmm_mem_aligned_n,
        ) -> XmmMemAligned {
            XmmMemAligned::unwrap_new(x.clone().into())
        }

        #[inline]
        fn $xmm_mem_imm_n_to_xmm_mem_imm(&mut self, x: &$xmm_mem_imm_n) -> XmmMemImm {
            XmmMemImm::unwrap_new(x.clone().into())
        }

        #[inline]
        fn $xmm_mem_aligned_imm_n_to_xmm_mem_aligned_imm(
            &mut self,
            x: &$xmm_mem_aligned_imm_n,
        ) -> XmmMemAlignedImm {
            XmmMemAlignedImm::unwrap_new(x.clone().into())
        }
    };
}

/// The main entry point for lowering with ISLE.
pub(crate) fn lower(
    lower_ctx: &mut Lower<MInst>,
    backend: &X64Backend,
    inst: Inst,
) -> Option<InstOutput> {
    // TODO: reuse the ISLE context across lowerings so we can reuse its
    // internal heap allocations.
    let mut isle_ctx = IsleContext { lower_ctx, backend };
    generated_code::constructor_lower(&mut isle_ctx, inst)
}

pub(crate) fn lower_branch(
    lower_ctx: &mut Lower<MInst>,
    backend: &X64Backend,
    branch: Inst,
    targets: &[MachLabel],
) -> Option<()> {
    // TODO: reuse the ISLE context across lowerings so we can reuse its
    // internal heap allocations.
    let mut isle_ctx = IsleContext { lower_ctx, backend };
    generated_code::constructor_lower_branch(&mut isle_ctx, branch, &targets)
}

impl Context for IsleContext<'_, '_, MInst, X64Backend> {
    isle_lower_prelude_methods!();
    isle_assembler_methods!();

    fn gen_call_info(
        &mut self,
        sig: Sig,
        dest: ExternalName,
        uses: CallArgList,
        defs: CallRetList,
        try_call_info: Option<TryCallInfo>,
        patchable: bool,
    ) -> BoxCallInfo {
        let stack_ret_space = self.lower_ctx.sigs()[sig].sized_stack_ret_space();
        let stack_arg_space = self.lower_ctx.sigs()[sig].sized_stack_arg_space();
        self.lower_ctx
            .abi_mut()
            .accumulate_outgoing_args_size(stack_ret_space + stack_arg_space);

        Box::new(
            self.lower_ctx
                .gen_call_info(sig, dest, uses, defs, try_call_info, patchable),
        )
    }

    fn gen_call_ind_info(
        &mut self,
        sig: Sig,
        dest: &RegMem,
        uses: CallArgList,
        defs: CallRetList,
        try_call_info: Option<TryCallInfo>,
    ) -> BoxCallIndInfo {
        let stack_ret_space = self.lower_ctx.sigs()[sig].sized_stack_ret_space();
        let stack_arg_space = self.lower_ctx.sigs()[sig].sized_stack_arg_space();
        self.lower_ctx
            .abi_mut()
            .accumulate_outgoing_args_size(stack_ret_space + stack_arg_space);

        Box::new(
            self.lower_ctx
                .gen_call_info(sig, dest.clone(), uses, defs, try_call_info, false),
        )
    }

    fn gen_return_call_info(
        &mut self,
        sig: Sig,
        dest: ExternalName,
        uses: CallArgList,
    ) -> BoxReturnCallInfo {
        let new_stack_arg_size = self.lower_ctx.sigs()[sig].sized_stack_arg_space();
        self.lower_ctx
            .abi_mut()
            .accumulate_tail_args_size(new_stack_arg_size);

        Box::new(ReturnCallInfo {
            dest,
            uses,
            tmp: self.lower_ctx.temp_writable_gpr(),
            new_stack_arg_size,
        })
    }

    fn gen_return_call_ind_info(
        &mut self,
        sig: Sig,
        dest: Reg,
        uses: CallArgList,
    ) -> BoxReturnCallIndInfo {
        let new_stack_arg_size = self.lower_ctx.sigs()[sig].sized_stack_arg_space();
        self.lower_ctx
            .abi_mut()
            .accumulate_tail_args_size(new_stack_arg_size);

        Box::new(ReturnCallInfo {
            dest,
            uses,
            tmp: self.lower_ctx.temp_writable_gpr(),
            new_stack_arg_size,
        })
    }

    #[inline]
    fn operand_size_of_type_32_64(&mut self, ty: Type) -> OperandSize {
        if ty.bits() == 64 {
            OperandSize::Size64
        } else {
            OperandSize::Size32
        }
    }

    #[inline]
    fn raw_operand_size_of_type(&mut self, ty: Type) -> OperandSize {
        OperandSize::from_ty(ty)
    }

    fn put_in_reg_mem_imm(&mut self, val: Value) -> RegMemImm {
        if let Some(imm) = self.i64_from_iconst(val) {
            if let Ok(imm) = i32::try_from(imm) {
                return RegMemImm::Imm {
                    simm32: imm.cast_unsigned(),
                };
            }
        }

        self.put_in_reg_mem(val).into()
    }

    fn put_in_xmm_mem_imm(&mut self, val: Value) -> XmmMemImm {
        if let Some(imm) = self.i64_from_iconst(val) {
            if let Ok(imm) = i32::try_from(imm) {
                return XmmMemImm::unwrap_new(RegMemImm::Imm {
                    simm32: imm.cast_unsigned(),
                });
            }
        }

        let res = match self.put_in_xmm_mem(val).to_reg_mem() {
            RegMem::Reg { reg } => RegMemImm::Reg { reg },
            RegMem::Mem { addr } => RegMemImm::Mem { addr },
        };

        XmmMemImm::unwrap_new(res)
    }

    fn put_in_xmm_mem(&mut self, val: Value) -> XmmMem {
        let inputs = self.lower_ctx.get_value_as_source_or_const(val);

        if let Some(c) = inputs.constant {
            // A load from the constant pool is better than a rematerialization into a register,
            // because it reduces register pressure.
            //
            // NOTE: this is where behavior differs from `put_in_reg_mem`, as we always force
            // constants to be 16 bytes when a constant will be used in place of an xmm register.
            let vcode_constant = self.emit_u128_le_const(c as u128);
            return XmmMem::unwrap_new(RegMem::mem(SyntheticAmode::ConstantOffset(vcode_constant)));
        }

        XmmMem::unwrap_new(self.put_in_reg_mem(val))
    }

    fn put_in_reg_mem(&mut self, val: Value) -> RegMem {
        let inputs = self.lower_ctx.get_value_as_source_or_const(val);

        if let Some(c) = inputs.constant {
            // A load from the constant pool is better than a
            // rematerialization into a register, because it reduces
            // register pressure.
            let vcode_constant = self.emit_u64_le_const(c);
            return RegMem::mem(SyntheticAmode::ConstantOffset(vcode_constant));
        }

        if let Some(load) = self.sinkable_load(val) {
            return RegMem::Mem {
                addr: self.sink_load(&load),
            };
        }

        RegMem::reg(self.put_in_reg(val))
    }

    #[inline]
    fn encode_fcmp_imm(&mut self, imm: &FcmpImm) -> u8 {
        imm.encode()
    }

    #[inline]
    fn encode_round_imm(&mut self, imm: &RoundImm) -> u8 {
        imm.encode()
    }

    #[inline]
    fn has_avx(&mut self) -> bool {
        self.backend.x64_flags.has_avx()
    }

    #[inline]
    fn use_avx2(&mut self) -> bool {
        self.backend.x64_flags.has_avx() && self.backend.x64_flags.has_avx2()
    }

    #[inline]
    fn has_avx512vl(&mut self) -> bool {
        self.backend.x64_flags.has_avx512vl()
    }

    #[inline]
    fn has_avx512dq(&mut self) -> bool {
        self.backend.x64_flags.has_avx512dq()
    }

    #[inline]
    fn has_avx512f(&mut self) -> bool {
        self.backend.x64_flags.has_avx512f()
    }

    #[inline]
    fn has_avx512bitalg(&mut self) -> bool {
        self.backend.x64_flags.has_avx512bitalg()
    }

    #[inline]
    fn has_avx512vbmi(&mut self) -> bool {
        self.backend.x64_flags.has_avx512vbmi()
    }

    #[inline]
    fn has_lzcnt(&mut self) -> bool {
        self.backend.x64_flags.has_lzcnt()
    }

    #[inline]
    fn has_bmi1(&mut self) -> bool {
        self.backend.x64_flags.has_bmi1()
    }

    #[inline]
    fn has_bmi2(&mut self) -> bool {
        self.backend.x64_flags.has_bmi2()
    }

    #[inline]
    fn use_popcnt(&mut self) -> bool {
        self.backend.x64_flags.has_popcnt() && self.backend.x64_flags.has_sse42()
    }

    #[inline]
    fn use_fma(&mut self) -> bool {
        self.backend.x64_flags.has_avx() && self.backend.x64_flags.has_fma()
    }

    #[inline]
    fn has_sse3(&mut self) -> bool {
        self.backend.x64_flags.has_sse3()
    }

    #[inline]
    fn has_ssse3(&mut self) -> bool {
        self.backend.x64_flags.has_ssse3()
    }

    #[inline]
    fn has_sse41(&mut self) -> bool {
        self.backend.x64_flags.has_sse41()
    }

    #[inline]
    fn use_sse42(&mut self) -> bool {
        self.backend.x64_flags.has_sse41() && self.backend.x64_flags.has_sse42()
    }

    #[inline]
    fn has_cmpxchg16b(&mut self) -> bool {
        self.backend.x64_flags.has_cmpxchg16b()
    }

    #[inline]
    fn shift_mask(&mut self, ty: Type) -> u8 {
        debug_assert!(ty.lane_bits().is_power_of_two());

        (ty.lane_bits() - 1) as u8
    }

    fn shift_amount_masked(&mut self, ty: Type, val: Imm64) -> u8 {
        (val.bits() as u8) & self.shift_mask(ty)
    }

    #[inline]
    fn simm32_from_value(&mut self, val: Value) -> Option<GprMemImm> {
        let imm = self.i64_from_iconst(val)?;
        Some(GprMemImm::unwrap_new(RegMemImm::Imm {
            simm32: i32::try_from(imm).ok()?.cast_unsigned(),
        }))
    }

    fn sinkable_load(&mut self, val: Value) -> Option<SinkableLoad> {
        if let Some(inst) = self.is_sinkable_inst(val) {
            if let Some((addr_input, offset)) =
                is_mergeable_load(self.lower_ctx, inst, MergeableLoadSize::Min32)
            {
                return Some(SinkableLoad {
                    inst,
                    addr_input,
                    offset,
                });
            }
        }
        None
    }

    fn sinkable_load_exact(&mut self, val: Value) -> Option<SinkableLoad> {
        if let Some(inst) = self.is_sinkable_inst(val) {
            if let Some((addr_input, offset)) =
                is_mergeable_load(self.lower_ctx, inst, MergeableLoadSize::Exact)
            {
                return Some(SinkableLoad {
                    inst,
                    addr_input,
                    offset,
                });
            }
        }
        None
    }

    // Width-tagged sinkable-load extractors. Each only fires when the `Value`
    // being sunk has a CLIF type of exactly N bits, so the resulting
    // `SinkableLoadN` carries that width in its type and can be folded into
    // an N-bit memory operand without a dynamic width check.

    fn sinkable_load_8(&mut self, val: Value) -> Option<SinkableLoad8> {
        if self.lower_ctx.dfg().value_type(val).bits() != 8 {
            return None;
        }
        self.sinkable_load_exact(val).map(SinkableLoad8)
    }

    fn sinkable_load_16(&mut self, val: Value) -> Option<SinkableLoad16> {
        if self.lower_ctx.dfg().value_type(val).bits() != 16 {
            return None;
        }
        self.sinkable_load_exact(val).map(SinkableLoad16)
    }

    fn sinkable_load_32(&mut self, val: Value) -> Option<SinkableLoad32> {
        if self.lower_ctx.dfg().value_type(val).bits() != 32 {
            return None;
        }
        self.sinkable_load_exact(val).map(SinkableLoad32)
    }

    fn sinkable_load_64(&mut self, val: Value) -> Option<SinkableLoad64> {
        if self.lower_ctx.dfg().value_type(val).bits() != 64 {
            return None;
        }
        self.sinkable_load_exact(val).map(SinkableLoad64)
    }

    fn sinkable_load_128(&mut self, val: Value) -> Option<SinkableLoad128> {
        if self.lower_ctx.dfg().value_type(val).bits() != 128 {
            return None;
        }
        self.sinkable_load_exact(val).map(SinkableLoad128)
    }

    // Conversions from width-tagged sinkable loads to sized memory operands.
    // These are safe because the width was pinned at extractor tim.e

    #[inline]
    fn sink_load_8(&mut self, load: &SinkableLoad8) -> SyntheticAmode {
        self.sink_load(&load.0)
    }
    #[inline]
    fn sink_load_16(&mut self, load: &SinkableLoad16) -> SyntheticAmode {
        self.sink_load(&load.0)
    }
    #[inline]
    fn sink_load_32(&mut self, load: &SinkableLoad32) -> SyntheticAmode {
        self.sink_load(&load.0)
    }
    #[inline]
    fn sink_load_64(&mut self, load: &SinkableLoad64) -> SyntheticAmode {
        self.sink_load(&load.0)
    }
    #[inline]
    fn sink_load_128(&mut self, load: &SinkableLoad128) -> SyntheticAmode {
        self.sink_load(&load.0)
    }

    // Direct sized-memory wrappers. Each sinks the load and then wraps the
    // resulting address at the matching width. Safe by construction.

    #[inline]
    fn sink_load_8_to_gpr_mem_8(&mut self, load: &SinkableLoad8) -> GprMem8 {
        GprMem8::unwrap_new(RegMem::mem(self.sink_load(&load.0)))
    }
    #[inline]
    fn sink_load_16_to_gpr_mem_16(&mut self, load: &SinkableLoad16) -> GprMem16 {
        GprMem16::unwrap_new(RegMem::mem(self.sink_load(&load.0)))
    }
    #[inline]
    fn sink_load_32_to_gpr_mem_32(&mut self, load: &SinkableLoad32) -> GprMem32 {
        GprMem32::unwrap_new(RegMem::mem(self.sink_load(&load.0)))
    }
    #[inline]
    fn sink_load_64_to_gpr_mem_64(&mut self, load: &SinkableLoad64) -> GprMem64 {
        GprMem64::unwrap_new(RegMem::mem(self.sink_load(&load.0)))
    }

    #[inline]
    fn sink_load_8_to_xmm_mem_8(&mut self, load: &SinkableLoad8) -> XmmMem8 {
        XmmMem8::unwrap_new(RegMem::mem(self.sink_load(&load.0)))
    }
    #[inline]
    fn sink_load_16_to_xmm_mem_16(&mut self, load: &SinkableLoad16) -> XmmMem16 {
        XmmMem16::unwrap_new(RegMem::mem(self.sink_load(&load.0)))
    }
    #[inline]
    fn sink_load_32_to_xmm_mem_32(&mut self, load: &SinkableLoad32) -> XmmMem32 {
        XmmMem32::unwrap_new(RegMem::mem(self.sink_load(&load.0)))
    }
    #[inline]
    fn sink_load_64_to_xmm_mem_64(&mut self, load: &SinkableLoad64) -> XmmMem64 {
        XmmMem64::unwrap_new(RegMem::mem(self.sink_load(&load.0)))
    }
    #[inline]
    fn sink_load_128_to_xmm_mem_128(&mut self, load: &SinkableLoad128) -> XmmMem128 {
        XmmMem128::unwrap_new(RegMem::mem(self.sink_load(&load.0)))
    }

    fn sink_load(&mut self, load: &SinkableLoad) -> SyntheticAmode {
        self.lower_ctx.sink_inst(load.inst);
        let addr = lower_to_amode(self.lower_ctx, load.addr_input, load.offset);
        SyntheticAmode::Real(addr)
    }

    #[inline]
    fn ext_mode(&mut self, from_bits: u16, to_bits: u16) -> ExtMode {
        ExtMode::new(from_bits, to_bits).unwrap()
    }

    fn emit(&mut self, inst: &MInst) -> Unit {
        self.lower_ctx.emit(inst.clone());
    }

    #[inline]
    fn sse_insertps_lane_imm(&mut self, lane: u8) -> u8 {
        // Insert 32-bits from replacement (at index 00, bits 7:8) to vector (lane
        // shifted into bits 5:6).
        0b00_00_00_00 | lane << 4
    }

    #[inline]
    fn synthetic_amode_to_reg_mem(&mut self, addr: &SyntheticAmode) -> RegMem {
        RegMem::mem(addr.clone())
    }

    #[inline]
    fn amode_to_synthetic_amode(&mut self, amode: &Amode) -> SyntheticAmode {
        amode.clone().into()
    }

    #[inline]
    fn synthetic_amode_slot(&mut self, offset: i32) -> SyntheticAmode {
        SyntheticAmode::SlotOffset { simm32: offset }
    }

    #[inline]
    fn const_to_synthetic_amode(&mut self, c: VCodeConstant) -> SyntheticAmode {
        SyntheticAmode::ConstantOffset(c)
    }

    /// Wrap a `VCodeConstant` that holds a 128-bit value as an `XmmMem128`
    /// operand. The caller is responsible for ensuring the pool entry is
    /// actually 128 bits wide (e.g. via `emit_u128_le_const`). There is
    /// intentionally no auto-conversion `VCodeConstant -> XmmMem128`
    /// because `VCodeConstant` doesn't carry width.
    #[inline]
    fn const_to_xmm_mem_128(&mut self, c: VCodeConstant) -> XmmMem128 {
        XmmMem128::unwrap_new(RegMem::mem(SyntheticAmode::ConstantOffset(c)))
    }

    /// Wrap a `SyntheticAmode` as a 128-bit XMM memory operand. Pins the
    /// width at the call site; no auto-conversion is provided.
    #[inline]
    fn synthetic_amode_to_xmm_mem_128(&mut self, amode: &SyntheticAmode) -> XmmMem128 {
        XmmMem128::unwrap_new(RegMem::mem(amode.clone()))
    }

    /// Wrap a `SyntheticAmode` as a 64-bit XMM memory operand.
    #[inline]
    fn synthetic_amode_to_xmm_mem_64(&mut self, amode: &SyntheticAmode) -> XmmMem64 {
        XmmMem64::unwrap_new(RegMem::mem(amode.clone()))
    }

    /// Wrap a `SyntheticAmode` as a 32-bit XMM memory operand.
    #[inline]
    fn synthetic_amode_to_xmm_mem_32(&mut self, amode: &SyntheticAmode) -> XmmMem32 {
        XmmMem32::unwrap_new(RegMem::mem(amode.clone()))
    }

    /// Unchecked narrowing from an un-sized `GprMem` to a sized
    /// `GprMem{N}`. Used only by `x64_movzx` / `x64_movsx`, whose
    /// per-ExtMode rules statically pin the source width.
    #[inline]
    fn gpr_mem_as_gpr_mem_8(&mut self, gm: &GprMem) -> GprMem8 {
        GprMem8::unwrap_new(gm.clone().into())
    }
    #[inline]
    fn gpr_mem_as_gpr_mem_16(&mut self, gm: &GprMem) -> GprMem16 {
        GprMem16::unwrap_new(gm.clone().into())
    }
    #[inline]
    fn gpr_mem_as_gpr_mem_32(&mut self, gm: &GprMem) -> GprMem32 {
        GprMem32::unwrap_new(gm.clone().into())
    }
    #[inline]
    fn gpr_mem_as_gpr_mem_64(&mut self, gm: &GprMem) -> GprMem64 {
        GprMem64::unwrap_new(gm.clone().into())
    }

    /// Unchecked narrowing from an un-sized `XmmMem` / `XmmMemAligned`
    /// to a sized variant. Used at fixed-width call sites (e.g. vector
    /// shift helpers `x64_psllw` etc.) where the width is implied by
    /// the opcode rather than a Cranelift `Type`.
    #[inline]
    fn xmm_mem_as_xmm_mem_32(&mut self, xm: &XmmMem) -> XmmMem32 {
        XmmMem32::unwrap_new(xm.clone().into())
    }
    #[inline]
    fn xmm_mem_as_xmm_mem_64(&mut self, xm: &XmmMem) -> XmmMem64 {
        XmmMem64::unwrap_new(xm.clone().into())
    }
    #[inline]
    fn xmm_mem_as_xmm_mem_128(&mut self, xm: &XmmMem) -> XmmMem128 {
        XmmMem128::unwrap_new(xm.clone().into())
    }
    #[inline]
    fn xmm_mem_aligned_as_xmm_mem_aligned_32(&mut self, xm: &XmmMemAligned) -> XmmMemAligned32 {
        XmmMemAligned32::unwrap_new(xm.clone().into())
    }
    #[inline]
    fn xmm_mem_aligned_as_xmm_mem_aligned_64(&mut self, xm: &XmmMemAligned) -> XmmMemAligned64 {
        XmmMemAligned64::unwrap_new(xm.clone().into())
    }
    #[inline]
    fn xmm_mem_aligned_as_xmm_mem_aligned_128(&mut self, xm: &XmmMemAligned) -> XmmMemAligned128 {
        XmmMemAligned128::unwrap_new(xm.clone().into())
    }

    fn gpr_mem_8_for_ty(&mut self, ty: Type, gm: &GprMem) -> GprMem8 {
        assert_eq!(
            ty.bits(),
            8,
            "gpr_mem_8_for_ty: expected an 8-bit Type, got {ty}",
        );
        GprMem8::unwrap_new(gm.clone().into())
    }
    fn gpr_mem_16_for_ty(&mut self, ty: Type, gm: &GprMem) -> GprMem16 {
        assert_eq!(
            ty.bits(),
            16,
            "gpr_mem_16_for_ty: expected a 16-bit Type, got {ty}",
        );
        GprMem16::unwrap_new(gm.clone().into())
    }
    fn gpr_mem_32_for_ty(&mut self, ty: Type, gm: &GprMem) -> GprMem32 {
        assert_eq!(
            ty.bits(),
            32,
            "gpr_mem_32_for_ty: expected a 32-bit Type, got {ty}",
        );
        GprMem32::unwrap_new(gm.clone().into())
    }
    fn gpr_mem_64_for_ty(&mut self, ty: Type, gm: &GprMem) -> GprMem64 {
        assert_eq!(
            ty.bits(),
            64,
            "gpr_mem_64_for_ty: expected a 64-bit Type, got {ty}",
        );
        GprMem64::unwrap_new(gm.clone().into())
    }
    fn gpr_mem_as_gpr_mem_32_or_widened_in_reg(&mut self, ty: Type, gm: &GprMem) -> GprMem32 {
        let safe = match gm.clone().to_reg_mem() {
            _ if ty.bits() == 32 => true,
            RegMem::Reg { .. } => true,
            RegMem::Mem {
                addr: SyntheticAmode::ConstantOffset(_),
            } => true,
            RegMem::Mem { .. } => false,
        };
        assert!(
            safe,
            "gpr_mem_as_gpr_mem_32_or_widened_in_reg: cannot widen a {ty} memory operand"
        );
        GprMem32::unwrap_new(gm.clone().into())
    }
    fn xmm_mem_32_for_ty(&mut self, ty: Type, xm: &XmmMem) -> XmmMem32 {
        assert_eq!(
            ty.bits(),
            32,
            "xmm_mem_32_for_ty: expected a 32-bit Type, got {ty}",
        );
        XmmMem32::unwrap_new(xm.clone().into())
    }
    fn xmm_mem_64_for_ty(&mut self, ty: Type, xm: &XmmMem) -> XmmMem64 {
        assert_eq!(
            ty.bits(),
            64,
            "xmm_mem_64_for_ty: expected a 64-bit Type, got {ty}",
        );
        XmmMem64::unwrap_new(xm.clone().into())
    }
    fn xmm_mem_128_for_ty(&mut self, ty: Type, xm: &XmmMem) -> XmmMem128 {
        assert_eq!(
            ty.bits(),
            128,
            "xmm_mem_128_for_ty: expected a 128-bit Type, got {ty}",
        );
        XmmMem128::unwrap_new(xm.clone().into())
    }
    fn xmm_mem_aligned_128_for_ty(&mut self, ty: Type, xm: &XmmMemAligned) -> XmmMemAligned128 {
        assert_eq!(
            ty.bits(),
            128,
            "xmm_mem_aligned_128_for_ty: expected a 128-bit Type, got {ty}",
        );
        XmmMemAligned128::unwrap_new(xm.clone().into())
    }

    #[inline]
    fn writable_gpr_to_reg(&mut self, r: WritableGpr) -> WritableReg {
        r.to_writable_reg()
    }

    #[inline]
    fn writable_xmm_to_reg(&mut self, r: WritableXmm) -> WritableReg {
        r.to_writable_reg()
    }

    #[inline]
    fn synthetic_amode_to_gpr_mem_8(&mut self, amode: &SyntheticAmode) -> GprMem8 {
        GprMem8::unwrap_new(RegMem::mem(amode.clone()))
    }
    #[inline]
    fn synthetic_amode_to_gpr_mem_16(&mut self, amode: &SyntheticAmode) -> GprMem16 {
        GprMem16::unwrap_new(RegMem::mem(amode.clone()))
    }
    #[inline]
    fn synthetic_amode_to_gpr_mem_32(&mut self, amode: &SyntheticAmode) -> GprMem32 {
        GprMem32::unwrap_new(RegMem::mem(amode.clone()))
    }
    #[inline]
    fn synthetic_amode_to_gpr_mem_64(&mut self, amode: &SyntheticAmode) -> GprMem64 {
        GprMem64::unwrap_new(RegMem::mem(amode.clone()))
    }
    #[inline]
    fn synthetic_amode_to_xmm_mem_8(&mut self, amode: &SyntheticAmode) -> XmmMem8 {
        XmmMem8::unwrap_new(RegMem::mem(amode.clone()))
    }
    #[inline]
    fn synthetic_amode_to_xmm_mem_16(&mut self, amode: &SyntheticAmode) -> XmmMem16 {
        XmmMem16::unwrap_new(RegMem::mem(amode.clone()))
    }

    #[inline]
    fn reg_to_gpr_mem_8(&mut self, r: Reg) -> GprMem8 {
        GprMem8::unwrap_new(RegMem::reg(r))
    }
    #[inline]
    fn reg_to_gpr_mem_16(&mut self, r: Reg) -> GprMem16 {
        GprMem16::unwrap_new(RegMem::reg(r))
    }
    #[inline]
    fn reg_to_gpr_mem_32(&mut self, r: Reg) -> GprMem32 {
        GprMem32::unwrap_new(RegMem::reg(r))
    }
    #[inline]
    fn reg_to_gpr_mem_64(&mut self, r: Reg) -> GprMem64 {
        GprMem64::unwrap_new(RegMem::reg(r))
    }

    #[inline]
    fn writable_gpr_to_gpr_mem_8(&mut self, r: WritableGpr) -> GprMem8 {
        GprMem8::unwrap_new(RegMem::reg(r.to_reg().to_reg()))
    }
    #[inline]
    fn writable_gpr_to_gpr_mem_16(&mut self, r: WritableGpr) -> GprMem16 {
        GprMem16::unwrap_new(RegMem::reg(r.to_reg().to_reg()))
    }
    #[inline]
    fn writable_gpr_to_gpr_mem_32(&mut self, r: WritableGpr) -> GprMem32 {
        GprMem32::unwrap_new(RegMem::reg(r.to_reg().to_reg()))
    }
    #[inline]
    fn writable_gpr_to_gpr_mem_64(&mut self, r: WritableGpr) -> GprMem64 {
        GprMem64::unwrap_new(RegMem::reg(r.to_reg().to_reg()))
    }
    #[inline]
    fn writable_xmm_to_xmm_mem_128(&mut self, r: WritableXmm) -> XmmMem128 {
        XmmMem128::unwrap_new(RegMem::reg(r.to_reg().to_reg()))
    }
    #[inline]
    fn writable_xmm_to_xmm_mem_aligned_128(&mut self, r: WritableXmm) -> XmmMemAligned128 {
        XmmMemAligned128::unwrap_new(RegMem::reg(r.to_reg().to_reg()))
    }

    #[inline]
    fn xmm_mem_32_to_xmm_mem_aligned_32(&mut self, x: &XmmMem32) -> XmmMemAligned32 {
        match XmmMemAligned32::new(x.clone().into()) {
            Some(a) => a,
            None => match x.clone().into() {
                RegMem::Mem { addr } => self.load_xmm_unaligned(addr).into(),
                _ => unreachable!(),
            },
        }
    }
    #[inline]
    fn xmm_mem_64_to_xmm_mem_aligned_64(&mut self, x: &XmmMem64) -> XmmMemAligned64 {
        match XmmMemAligned64::new(x.clone().into()) {
            Some(a) => a,
            None => match x.clone().into() {
                RegMem::Mem { addr } => self.load_xmm_unaligned(addr).into(),
                _ => unreachable!(),
            },
        }
    }
    #[inline]
    fn xmm_mem_128_to_xmm_mem_aligned_128(&mut self, x: &XmmMem128) -> XmmMemAligned128 {
        match XmmMemAligned128::new(x.clone().into()) {
            Some(a) => a,
            None => match x.clone().into() {
                RegMem::Mem { addr } => self.load_xmm_unaligned(addr).into(),
                _ => unreachable!(),
            },
        }
    }

    fn ishl_i8x16_mask_for_const(&mut self, amt: u32) -> SyntheticAmode {
        // When the shift amount is known, we can statically (i.e. at compile
        // time) determine the mask to use and only emit that.
        debug_assert!(amt < 8);
        let mask_offset = amt as usize * 16;
        let mask_constant = self.lower_ctx.use_constant(VCodeConstantData::WellKnown(
            &I8X16_ISHL_MASKS[mask_offset..mask_offset + 16],
        ));
        SyntheticAmode::ConstantOffset(mask_constant)
    }

    fn ishl_i8x16_mask_table(&mut self) -> SyntheticAmode {
        let mask_table = self
            .lower_ctx
            .use_constant(VCodeConstantData::WellKnown(&I8X16_ISHL_MASKS));
        SyntheticAmode::ConstantOffset(mask_table)
    }

    fn ushr_i8x16_mask_for_const(&mut self, amt: u32) -> SyntheticAmode {
        // When the shift amount is known, we can statically (i.e. at compile
        // time) determine the mask to use and only emit that.
        debug_assert!(amt < 8);
        let mask_offset = amt as usize * 16;
        let mask_constant = self.lower_ctx.use_constant(VCodeConstantData::WellKnown(
            &I8X16_USHR_MASKS[mask_offset..mask_offset + 16],
        ));
        SyntheticAmode::ConstantOffset(mask_constant)
    }

    fn ushr_i8x16_mask_table(&mut self) -> SyntheticAmode {
        let mask_table = self
            .lower_ctx
            .use_constant(VCodeConstantData::WellKnown(&I8X16_USHR_MASKS));
        SyntheticAmode::ConstantOffset(mask_table)
    }

    #[inline]
    fn writable_reg_to_xmm(&mut self, r: WritableReg) -> WritableXmm {
        Writable::from_reg(Xmm::unwrap_new(r.to_reg()))
    }

    #[inline]
    fn writable_xmm_to_xmm(&mut self, r: WritableXmm) -> Xmm {
        r.to_reg()
    }

    #[inline]
    fn writable_gpr_to_gpr(&mut self, r: WritableGpr) -> Gpr {
        r.to_reg()
    }

    #[inline]
    fn gpr_to_reg(&mut self, r: Gpr) -> Reg {
        r.into()
    }

    #[inline]
    fn xmm_to_reg(&mut self, r: Xmm) -> Reg {
        r.into()
    }

    #[inline]
    fn xmm_to_xmm_mem_imm(&mut self, r: Xmm) -> XmmMemImm {
        r.into()
    }

    #[inline]
    fn xmm_mem_to_xmm_mem_imm(&mut self, r: &XmmMem) -> XmmMemImm {
        XmmMemImm::unwrap_new(r.clone().to_reg_mem().into())
    }

    #[inline]
    fn temp_writable_gpr(&mut self) -> WritableGpr {
        self.lower_ctx.temp_writable_gpr()
    }

    #[inline]
    fn temp_writable_xmm(&mut self) -> WritableXmm {
        self.lower_ctx.temp_writable_xmm()
    }

    #[inline]
    fn reg_to_reg_mem_imm(&mut self, reg: Reg) -> RegMemImm {
        RegMemImm::Reg { reg }
    }

    #[inline]
    fn reg_mem_to_xmm_mem(&mut self, rm: &RegMem) -> XmmMem {
        XmmMem::unwrap_new(rm.clone())
    }

    #[inline]
    fn gpr_mem_imm_new(&mut self, rmi: &RegMemImm) -> GprMemImm {
        GprMemImm::unwrap_new(rmi.clone())
    }

    #[inline]
    fn xmm_mem_imm_new(&mut self, rmi: &RegMemImm) -> XmmMemImm {
        XmmMemImm::unwrap_new(rmi.clone())
    }

    #[inline]
    fn xmm_to_xmm_mem(&mut self, r: Xmm) -> XmmMem {
        r.into()
    }

    #[inline]
    fn xmm_mem_to_reg_mem(&mut self, xm: &XmmMem) -> RegMem {
        xm.clone().into()
    }

    #[inline]
    fn gpr_mem_to_reg_mem(&mut self, gm: &GprMem) -> RegMem {
        gm.clone().into()
    }

    #[inline]
    fn xmm_new(&mut self, r: Reg) -> Xmm {
        Xmm::unwrap_new(r)
    }

    #[inline]
    fn gpr_new(&mut self, r: Reg) -> Gpr {
        Gpr::unwrap_new(r)
    }

    #[inline]
    fn reg_mem_to_gpr_mem(&mut self, rm: &RegMem) -> GprMem {
        GprMem::unwrap_new(rm.clone())
    }

    #[inline]
    fn reg_to_gpr_mem(&mut self, r: Reg) -> GprMem {
        GprMem::unwrap_new(RegMem::reg(r))
    }

    #[inline]
    fn gpr_to_gpr_mem(&mut self, gpr: Gpr) -> GprMem {
        GprMem::from(gpr)
    }

    #[inline]
    fn gpr_to_gpr_mem_imm(&mut self, gpr: Gpr) -> GprMemImm {
        GprMemImm::from(gpr)
    }

    // ---- Sized operand conversions ----
    //
    // For each sized newtype `{Gpr,Xmm}Mem{Imm,Aligned,AlignedImm}{N}`, we
    // generate a small fixed set of helpers:
    //
    //   * `put_in_..._N(Value)` converts a CLIF `Value` to the sized
    //     operand. It **release-asserts** (not `debug_assert!`) that the
    //     value's CLIF type is N bits wide, so a width mismatch at lowering
    //     time becomes a hard failure instead of silently emitting a wider
    //     load than intended.
    //   * `{gpr,xmm}_to_..._N({Gpr,Xmm})` wraps a typed register into the
    //     sized operand unconditionally -- `Gpr`/`Xmm` carry no width; the
    //     consumer's instruction determines it.
    //   * `..._N_to_...` downcasts a sized operand back to its un-sized
    //     form. These are transition bridges into still-un-sized helpers
    //     (mostly auto-generated assembler constructors); they should
    //     go away as the refactor proceeds.
    //
    // A `{gpr,xmm}_mem_imm_N_imm(u32)` typed-immediate constructor is also
    // provided so call sites can build `GprMemImm{N}` without having to
    // pierce the sized wrapper via `RegMemImm.Imm`.

    sized_gpr_mem_helpers!(
        8,
        GprMem8,
        GprMemImm8,
        put_in_gpr_mem_8,
        put_in_gpr_mem_imm_8,
        gpr_to_gpr_mem_8,
        gpr_to_gpr_mem_imm_8,
        gpr_mem_imm_8_imm,
        gpr_mem_8_to_gpr_mem,
        gpr_mem_imm_8_to_gpr_mem_imm
    );
    sized_gpr_mem_helpers!(
        16,
        GprMem16,
        GprMemImm16,
        put_in_gpr_mem_16,
        put_in_gpr_mem_imm_16,
        gpr_to_gpr_mem_16,
        gpr_to_gpr_mem_imm_16,
        gpr_mem_imm_16_imm,
        gpr_mem_16_to_gpr_mem,
        gpr_mem_imm_16_to_gpr_mem_imm
    );
    sized_gpr_mem_helpers!(
        32,
        GprMem32,
        GprMemImm32,
        put_in_gpr_mem_32,
        put_in_gpr_mem_imm_32,
        gpr_to_gpr_mem_32,
        gpr_to_gpr_mem_imm_32,
        gpr_mem_imm_32_imm,
        gpr_mem_32_to_gpr_mem,
        gpr_mem_imm_32_to_gpr_mem_imm
    );
    sized_gpr_mem_helpers!(
        64,
        GprMem64,
        GprMemImm64,
        put_in_gpr_mem_64,
        put_in_gpr_mem_imm_64,
        gpr_to_gpr_mem_64,
        gpr_to_gpr_mem_imm_64,
        gpr_mem_imm_64_imm,
        gpr_mem_64_to_gpr_mem,
        gpr_mem_imm_64_to_gpr_mem_imm
    );

    sized_xmm_mem_helpers!(
        8,
        XmmMem8,
        XmmMemAligned8,
        XmmMemImm8,
        XmmMemAlignedImm8,
        put_in_xmm_mem_8,
        put_in_xmm_mem_aligned_8,
        put_in_xmm_mem_imm_8,
        put_in_xmm_mem_aligned_imm_8,
        xmm_to_xmm_mem_8,
        xmm_to_xmm_mem_aligned_8,
        xmm_to_xmm_mem_imm_8,
        xmm_to_xmm_mem_aligned_imm_8,
        xmm_mem_imm_8_imm,
        xmm_mem_8_to_xmm_mem,
        xmm_mem_aligned_8_to_xmm_mem_aligned,
        xmm_mem_imm_8_to_xmm_mem_imm,
        xmm_mem_aligned_imm_8_to_xmm_mem_aligned_imm
    );
    sized_xmm_mem_helpers!(
        16,
        XmmMem16,
        XmmMemAligned16,
        XmmMemImm16,
        XmmMemAlignedImm16,
        put_in_xmm_mem_16,
        put_in_xmm_mem_aligned_16,
        put_in_xmm_mem_imm_16,
        put_in_xmm_mem_aligned_imm_16,
        xmm_to_xmm_mem_16,
        xmm_to_xmm_mem_aligned_16,
        xmm_to_xmm_mem_imm_16,
        xmm_to_xmm_mem_aligned_imm_16,
        xmm_mem_imm_16_imm,
        xmm_mem_16_to_xmm_mem,
        xmm_mem_aligned_16_to_xmm_mem_aligned,
        xmm_mem_imm_16_to_xmm_mem_imm,
        xmm_mem_aligned_imm_16_to_xmm_mem_aligned_imm
    );
    sized_xmm_mem_helpers!(
        32,
        XmmMem32,
        XmmMemAligned32,
        XmmMemImm32,
        XmmMemAlignedImm32,
        put_in_xmm_mem_32,
        put_in_xmm_mem_aligned_32,
        put_in_xmm_mem_imm_32,
        put_in_xmm_mem_aligned_imm_32,
        xmm_to_xmm_mem_32,
        xmm_to_xmm_mem_aligned_32,
        xmm_to_xmm_mem_imm_32,
        xmm_to_xmm_mem_aligned_imm_32,
        xmm_mem_imm_32_imm,
        xmm_mem_32_to_xmm_mem,
        xmm_mem_aligned_32_to_xmm_mem_aligned,
        xmm_mem_imm_32_to_xmm_mem_imm,
        xmm_mem_aligned_imm_32_to_xmm_mem_aligned_imm
    );
    sized_xmm_mem_helpers!(
        64,
        XmmMem64,
        XmmMemAligned64,
        XmmMemImm64,
        XmmMemAlignedImm64,
        put_in_xmm_mem_64,
        put_in_xmm_mem_aligned_64,
        put_in_xmm_mem_imm_64,
        put_in_xmm_mem_aligned_imm_64,
        xmm_to_xmm_mem_64,
        xmm_to_xmm_mem_aligned_64,
        xmm_to_xmm_mem_imm_64,
        xmm_to_xmm_mem_aligned_imm_64,
        xmm_mem_imm_64_imm,
        xmm_mem_64_to_xmm_mem,
        xmm_mem_aligned_64_to_xmm_mem_aligned,
        xmm_mem_imm_64_to_xmm_mem_imm,
        xmm_mem_aligned_imm_64_to_xmm_mem_aligned_imm
    );
    sized_xmm_mem_helpers!(
        128,
        XmmMem128,
        XmmMemAligned128,
        XmmMemImm128,
        XmmMemAlignedImm128,
        put_in_xmm_mem_128,
        put_in_xmm_mem_aligned_128,
        put_in_xmm_mem_imm_128,
        put_in_xmm_mem_aligned_imm_128,
        xmm_to_xmm_mem_128,
        xmm_to_xmm_mem_aligned_128,
        xmm_to_xmm_mem_imm_128,
        xmm_to_xmm_mem_aligned_imm_128,
        xmm_mem_imm_128_imm,
        xmm_mem_128_to_xmm_mem,
        xmm_mem_aligned_128_to_xmm_mem_aligned,
        xmm_mem_imm_128_to_xmm_mem_imm,
        xmm_mem_aligned_imm_128_to_xmm_mem_aligned_imm
    );

    #[inline]
    fn type_register_class(&mut self, ty: Type) -> Option<RegisterClass> {
        if is_int_or_ref_ty(ty) || ty == I128 {
            Some(RegisterClass::Gpr {
                single_register: ty != I128,
            })
        } else if ty.is_float() || (ty.is_vector() && ty.bits() <= 128) {
            Some(RegisterClass::Xmm)
        } else {
            None
        }
    }

    #[inline]
    fn ty_int_bool_or_ref(&mut self, ty: Type) -> Option<()> {
        match ty {
            types::I8 | types::I16 | types::I32 | types::I64 => Some(()),
            _ => None,
        }
    }

    #[inline]
    fn intcc_to_cc(&mut self, intcc: &IntCC) -> CC {
        CC::from_intcc(*intcc)
    }

    #[inline]
    fn cc_invert(&mut self, cc: &CC) -> CC {
        cc.invert()
    }

    #[inline]
    fn cc_nz_or_z(&mut self, cc: &CC) -> Option<CC> {
        match cc {
            CC::Z => Some(*cc),
            CC::NZ => Some(*cc),
            _ => None,
        }
    }

    #[inline]
    fn sum_extend_fits_in_32_bits(
        &mut self,
        extend_from_ty: Type,
        constant_value: Imm64,
        offset: Offset32,
    ) -> Option<u32> {
        let offset: i64 = offset.into();
        let constant_value: u64 = constant_value.bits() as u64;
        // If necessary, zero extend `constant_value` up to 64 bits.
        let shift = 64 - extend_from_ty.bits();
        let zero_extended_constant_value = (constant_value << shift) >> shift;
        // Sum up the two operands.
        let sum = offset.wrapping_add(zero_extended_constant_value as i64);
        // Check that the sum will fit in 32-bits.
        if sum == ((sum << 32) >> 32) {
            Some(sum as u32)
        } else {
            None
        }
    }

    #[inline]
    fn amode_try_offset(&mut self, addr: &SyntheticAmode, offset: i32) -> Option<SyntheticAmode> {
        addr.offset(offset)
    }

    #[inline]
    fn zero_offset(&mut self) -> Offset32 {
        Offset32::new(0)
    }

    #[inline]
    fn preg_rbp(&mut self) -> PReg {
        regs::rbp().to_real_reg().unwrap().into()
    }

    #[inline]
    fn preg_rsp(&mut self) -> PReg {
        regs::rsp().to_real_reg().unwrap().into()
    }

    #[inline]
    fn preg_pinned(&mut self) -> PReg {
        regs::pinned_reg().to_real_reg().unwrap().into()
    }

    fn libcall_1(&mut self, libcall: &LibCall, a: Reg) -> Reg {
        let outputs = emit_vm_call(
            self.lower_ctx,
            &self.backend.flags,
            &self.backend.triple,
            *libcall,
            &[ValueRegs::one(a)],
        )
        .expect("Failed to emit LibCall");

        debug_assert_eq!(outputs.len(), 1);

        outputs[0].only_reg().unwrap()
    }

    fn libcall_2(&mut self, libcall: &LibCall, a: Reg, b: Reg) -> Reg {
        let outputs = emit_vm_call(
            self.lower_ctx,
            &self.backend.flags,
            &self.backend.triple,
            *libcall,
            &[ValueRegs::one(a), ValueRegs::one(b)],
        )
        .expect("Failed to emit LibCall");

        debug_assert_eq!(outputs.len(), 1);

        outputs[0].only_reg().unwrap()
    }

    fn libcall_3(&mut self, libcall: &LibCall, a: Reg, b: Reg, c: Reg) -> Reg {
        let outputs = emit_vm_call(
            self.lower_ctx,
            &self.backend.flags,
            &self.backend.triple,
            *libcall,
            &[ValueRegs::one(a), ValueRegs::one(b), ValueRegs::one(c)],
        )
        .expect("Failed to emit LibCall");

        debug_assert_eq!(outputs.len(), 1);

        outputs[0].only_reg().unwrap()
    }

    #[inline]
    fn vconst_all_ones_or_all_zeros(&mut self, constant: Constant) -> Option<()> {
        let const_data = self.lower_ctx.get_constant_data(constant);
        if const_data.iter().all(|&b| b == 0 || b == 0xFF) {
            return Some(());
        }
        None
    }

    #[inline]
    fn shuffle_0_31_mask(&mut self, mask: &VecMask) -> VCodeConstant {
        let mask = mask
            .iter()
            .map(|&b| if b > 15 { b.wrapping_sub(16) } else { b })
            .map(|b| if b > 15 { 0b10000000 } else { b })
            .collect();
        self.lower_ctx
            .use_constant(VCodeConstantData::Generated(mask))
    }

    #[inline]
    fn shuffle_0_15_mask(&mut self, mask: &VecMask) -> VCodeConstant {
        let mask = mask
            .iter()
            .map(|&b| if b > 15 { 0b10000000 } else { b })
            .collect();
        self.lower_ctx
            .use_constant(VCodeConstantData::Generated(mask))
    }

    #[inline]
    fn shuffle_16_31_mask(&mut self, mask: &VecMask) -> VCodeConstant {
        let mask = mask
            .iter()
            .map(|&b| b.wrapping_sub(16))
            .map(|b| if b > 15 { 0b10000000 } else { b })
            .collect();
        self.lower_ctx
            .use_constant(VCodeConstantData::Generated(mask))
    }

    #[inline]
    fn perm_from_mask_with_zeros(
        &mut self,
        mask: &VecMask,
    ) -> Option<(VCodeConstant, VCodeConstant)> {
        if !mask.iter().any(|&b| b > 31) {
            return None;
        }

        let zeros = mask
            .iter()
            .map(|&b| if b > 31 { 0x00 } else { 0xff })
            .collect();

        Some((
            self.perm_from_mask(mask),
            self.lower_ctx
                .use_constant(VCodeConstantData::Generated(zeros)),
        ))
    }

    #[inline]
    fn perm_from_mask(&mut self, mask: &VecMask) -> VCodeConstant {
        let mask = mask.iter().cloned().collect();
        self.lower_ctx
            .use_constant(VCodeConstantData::Generated(mask))
    }

    fn xmm_mem_to_xmm_mem_aligned(&mut self, arg: &XmmMem) -> XmmMemAligned {
        match XmmMemAligned::new(arg.clone().into()) {
            Some(aligned) => aligned,
            None => match arg.clone().into() {
                RegMem::Mem { addr } => self.load_xmm_unaligned(addr).into(),
                _ => unreachable!(),
            },
        }
    }

    fn xmm_mem_imm_to_xmm_mem_aligned_imm(&mut self, arg: &XmmMemImm) -> XmmMemAlignedImm {
        match XmmMemAlignedImm::new(arg.clone().into()) {
            Some(aligned) => aligned,
            None => match arg.clone().into() {
                RegMemImm::Mem { addr } => self.load_xmm_unaligned(addr).into(),
                _ => unreachable!(),
            },
        }
    }

    fn pshufd_lhs_imm(&mut self, imm: Immediate) -> Option<u8> {
        let (a, b, c, d) = self.shuffle32_from_imm(imm)?;
        if a < 4 && b < 4 && c < 4 && d < 4 {
            Some(a | (b << 2) | (c << 4) | (d << 6))
        } else {
            None
        }
    }

    fn pshufd_rhs_imm(&mut self, imm: Immediate) -> Option<u8> {
        let (a, b, c, d) = self.shuffle32_from_imm(imm)?;
        // When selecting from the right-hand-side, subtract these all by 4
        // which will bail out if anything is less than 4. Afterwards the check
        // is the same as `pshufd_lhs_imm` above.
        let a = a.checked_sub(4)?;
        let b = b.checked_sub(4)?;
        let c = c.checked_sub(4)?;
        let d = d.checked_sub(4)?;
        if a < 4 && b < 4 && c < 4 && d < 4 {
            Some(a | (b << 2) | (c << 4) | (d << 6))
        } else {
            None
        }
    }

    fn shufps_imm(&mut self, imm: Immediate) -> Option<u8> {
        // The `shufps` instruction selects the first two elements from the
        // first vector and the second two elements from the second vector, so
        // offset the third/fourth selectors by 4 and then make sure everything
        // fits in 32-bits.
        let (a, b, c, d) = self.shuffle32_from_imm(imm)?;
        let c = c.checked_sub(4)?;
        let d = d.checked_sub(4)?;
        if a < 4 && b < 4 && c < 4 && d < 4 {
            Some(a | (b << 2) | (c << 4) | (d << 6))
        } else {
            None
        }
    }

    fn shufps_rev_imm(&mut self, imm: Immediate) -> Option<u8> {
        // This is almost the same as `shufps_imm` except the elements that are
        // subtracted are reversed. This handles the case that `shufps`
        // instruction can be emitted if the order of the operands are swapped.
        let (a, b, c, d) = self.shuffle32_from_imm(imm)?;
        let a = a.checked_sub(4)?;
        let b = b.checked_sub(4)?;
        if a < 4 && b < 4 && c < 4 && d < 4 {
            Some(a | (b << 2) | (c << 4) | (d << 6))
        } else {
            None
        }
    }

    fn pshuflw_lhs_imm(&mut self, imm: Immediate) -> Option<u8> {
        // Similar to `shufps` except this operates over 16-bit values so four
        // of them must be fixed and the other four must be in-range to encode
        // in the immediate.
        let (a, b, c, d, e, f, g, h) = self.shuffle16_from_imm(imm)?;
        if a < 4 && b < 4 && c < 4 && d < 4 && [e, f, g, h] == [4, 5, 6, 7] {
            Some(a | (b << 2) | (c << 4) | (d << 6))
        } else {
            None
        }
    }

    fn pshuflw_rhs_imm(&mut self, imm: Immediate) -> Option<u8> {
        let (a, b, c, d, e, f, g, h) = self.shuffle16_from_imm(imm)?;
        let a = a.checked_sub(8)?;
        let b = b.checked_sub(8)?;
        let c = c.checked_sub(8)?;
        let d = d.checked_sub(8)?;
        let e = e.checked_sub(8)?;
        let f = f.checked_sub(8)?;
        let g = g.checked_sub(8)?;
        let h = h.checked_sub(8)?;
        if a < 4 && b < 4 && c < 4 && d < 4 && [e, f, g, h] == [4, 5, 6, 7] {
            Some(a | (b << 2) | (c << 4) | (d << 6))
        } else {
            None
        }
    }

    fn pshufhw_lhs_imm(&mut self, imm: Immediate) -> Option<u8> {
        // Similar to `pshuflw` except that the first four operands must be
        // fixed and the second four are offset by an extra 4 and tested to
        // make sure they're all in the range [4, 8).
        let (a, b, c, d, e, f, g, h) = self.shuffle16_from_imm(imm)?;
        let e = e.checked_sub(4)?;
        let f = f.checked_sub(4)?;
        let g = g.checked_sub(4)?;
        let h = h.checked_sub(4)?;
        if e < 4 && f < 4 && g < 4 && h < 4 && [a, b, c, d] == [0, 1, 2, 3] {
            Some(e | (f << 2) | (g << 4) | (h << 6))
        } else {
            None
        }
    }

    fn pshufhw_rhs_imm(&mut self, imm: Immediate) -> Option<u8> {
        // Note that everything here is offset by at least 8 and the upper
        // bits are offset by 12 to test they're in the range of [12, 16).
        let (a, b, c, d, e, f, g, h) = self.shuffle16_from_imm(imm)?;
        let a = a.checked_sub(8)?;
        let b = b.checked_sub(8)?;
        let c = c.checked_sub(8)?;
        let d = d.checked_sub(8)?;
        let e = e.checked_sub(12)?;
        let f = f.checked_sub(12)?;
        let g = g.checked_sub(12)?;
        let h = h.checked_sub(12)?;
        if e < 4 && f < 4 && g < 4 && h < 4 && [a, b, c, d] == [0, 1, 2, 3] {
            Some(e | (f << 2) | (g << 4) | (h << 6))
        } else {
            None
        }
    }

    fn palignr_imm_from_immediate(&mut self, imm: Immediate) -> Option<u8> {
        let bytes = self.lower_ctx.get_immediate_data(imm).as_slice();

        if bytes.windows(2).all(|a| a[0] + 1 == a[1]) {
            Some(bytes[0])
        } else {
            None
        }
    }

    fn pblendw_imm(&mut self, imm: Immediate) -> Option<u8> {
        // First make sure that the shuffle immediate is selecting 16-bit lanes.
        let (a, b, c, d, e, f, g, h) = self.shuffle16_from_imm(imm)?;

        // Next build up an 8-bit mask from each of the bits of the selected
        // lanes above. This instruction can only be used when each lane
        // selector chooses from the corresponding lane in either of the two
        // operands, meaning the Nth lane selection must satisfy `lane % 8 ==
        // N`.
        //
        // This helper closure is used to calculate the value of the
        // corresponding bit.
        let bit = |x: u8, c: u8| {
            if x % 8 == c {
                if x < 8 { Some(0) } else { Some(1 << c) }
            } else {
                None
            }
        };
        Some(
            bit(a, 0)?
                | bit(b, 1)?
                | bit(c, 2)?
                | bit(d, 3)?
                | bit(e, 4)?
                | bit(f, 5)?
                | bit(g, 6)?
                | bit(h, 7)?,
        )
    }

    fn xmi_imm(&mut self, imm: u32) -> XmmMemImm {
        XmmMemImm::unwrap_new(RegMemImm::imm(imm))
    }

    fn insert_i8x16_lane_hole(&mut self, hole_idx: u8) -> VCodeConstant {
        let mask = -1i128 as u128;
        self.emit_u128_le_const(mask ^ (0xff << (hole_idx * 8)))
    }

    fn writable_invalid_gpr(&mut self) -> WritableGpr {
        let reg = Gpr::new(self.invalid_reg()).unwrap();
        WritableGpr::from_reg(reg)
    }

    fn atomic128_rmw_seq_args(
        &mut self,
        op: &Atomic128RmwSeqOp,
        mem_low: &SyntheticAmode,
        mem_high: &SyntheticAmode,
        operand_low: Gpr,
        operand_high: Gpr,
        temp_low: WritableGpr,
        temp_high: WritableGpr,
        dst_old_low: WritableGpr,
        dst_old_high: WritableGpr,
    ) -> BoxAtomic128RmwSeqArgs {
        Box::new(Atomic128RmwSeqArgs {
            op: *op,
            mem_low: mem_low.clone(),
            mem_high: mem_high.clone(),
            operand_low,
            operand_high,
            temp_low,
            temp_high,
            dst_old_low,
            dst_old_high,
        })
    }

    fn atomic128_xchg_seq_args(
        &mut self,
        mem_low: &SyntheticAmode,
        mem_high: &SyntheticAmode,
        operand_low: Gpr,
        operand_high: Gpr,
        dst_old_low: WritableGpr,
        dst_old_high: WritableGpr,
    ) -> BoxAtomic128XchgSeqArgs {
        Box::new(Atomic128XchgSeqArgs {
            mem_low: mem_low.clone(),
            mem_high: mem_high.clone(),
            operand_low,
            operand_high,
            dst_old_low,
            dst_old_high,
        })
    }

    ////////////////////////////////////////////////////////////////////////////
    ///// External assembler methods.
    ////////////////////////////////////////////////////////////////////////////

    fn is_imm8(&mut self, src: &GprMemImm) -> Option<u8> {
        match src.clone().to_reg_mem_imm() {
            RegMemImm::Imm { simm32 } => {
                Some(i8::try_from(simm32.cast_signed()).ok()?.cast_unsigned())
            }
            _ => None,
        }
    }

    fn is_imm8_xmm(&mut self, src: &XmmMemImm) -> Option<u8> {
        match src.clone().to_reg_mem_imm() {
            RegMemImm::Imm { simm32 } => {
                Some(i8::try_from(simm32.cast_signed()).ok()?.cast_unsigned())
            }
            _ => None,
        }
    }

    fn is_simm8(&mut self, src: &GprMemImm) -> Option<i8> {
        match src.clone().to_reg_mem_imm() {
            RegMemImm::Imm { simm32 } => Some(i8::try_from(simm32.cast_signed()).ok()?),
            _ => None,
        }
    }

    fn is_imm16(&mut self, src: &GprMemImm) -> Option<u16> {
        match src.clone().to_reg_mem_imm() {
            RegMemImm::Imm { simm32 } => {
                Some(i16::try_from(simm32.cast_signed()).ok()?.cast_unsigned())
            }
            _ => None,
        }
    }

    fn is_simm16(&mut self, src: &GprMemImm) -> Option<i16> {
        match src.clone().to_reg_mem_imm() {
            RegMemImm::Imm { simm32 } => Some(i16::try_from(simm32.cast_signed()).ok()?),
            _ => None,
        }
    }

    fn is_imm32(&mut self, src: &GprMemImm) -> Option<u32> {
        match src.clone().to_reg_mem_imm() {
            RegMemImm::Imm { simm32 } => Some(simm32),
            _ => None,
        }
    }

    fn is_simm32(&mut self, src: &GprMemImm) -> Option<i32> {
        match src.clone().to_reg_mem_imm() {
            RegMemImm::Imm { simm32 } => Some(simm32 as i32),
            _ => None,
        }
    }

    fn is_gpr(&mut self, src: &GprMemImm) -> Option<Gpr> {
        match src.clone().to_reg_mem_imm() {
            RegMemImm::Reg { reg } => Gpr::new(reg),
            _ => None,
        }
    }

    fn is_xmm(&mut self, src: &XmmMem) -> Option<Xmm> {
        match src.clone().to_reg_mem() {
            RegMem::Reg { reg } => Xmm::new(reg),
            _ => None,
        }
    }

    fn is_gpr_mem(&mut self, src: &GprMemImm) -> Option<GprMem> {
        match src.clone().to_reg_mem_imm() {
            RegMemImm::Reg { reg } => GprMem::new(RegMem::Reg { reg }),
            RegMemImm::Mem { addr } => GprMem::new(RegMem::Mem { addr }),
            _ => None,
        }
    }

    fn is_xmm_mem(&mut self, src: &XmmMemImm) -> Option<XmmMem> {
        match src.clone().to_reg_mem_imm() {
            RegMemImm::Reg { reg } => XmmMem::new(RegMem::Reg { reg }),
            RegMemImm::Mem { addr } => XmmMem::new(RegMem::Mem { addr }),
            _ => None,
        }
    }

    fn is_mem(&mut self, src: &XmmMem) -> Option<SyntheticAmode> {
        match src.clone().to_reg_mem() {
            RegMem::Reg { .. } => None,
            RegMem::Mem { addr } => Some(addr),
        }
    }

    // Sized variants of `is_xmm`/`is_mem`. Each extracts the inner
    // `Xmm`/`SyntheticAmode` from a `XmmMem{N}`. No width check is
    // needed -- the sized wrapper already pinned it.

    fn is_xmm_32(&mut self, src: &XmmMem32) -> Option<Xmm> {
        match src.clone().into() {
            RegMem::Reg { reg } => Xmm::new(reg),
            _ => None,
        }
    }
    fn is_xmm_64(&mut self, src: &XmmMem64) -> Option<Xmm> {
        match src.clone().into() {
            RegMem::Reg { reg } => Xmm::new(reg),
            _ => None,
        }
    }
    fn is_xmm_128(&mut self, src: &XmmMem128) -> Option<Xmm> {
        match src.clone().into() {
            RegMem::Reg { reg } => Xmm::new(reg),
            _ => None,
        }
    }
    fn is_mem_32(&mut self, src: &XmmMem32) -> Option<SyntheticAmode> {
        match src.clone().into() {
            RegMem::Reg { .. } => None,
            RegMem::Mem { addr } => Some(addr),
        }
    }
    fn is_mem_64(&mut self, src: &XmmMem64) -> Option<SyntheticAmode> {
        match src.clone().into() {
            RegMem::Reg { .. } => None,
            RegMem::Mem { addr } => Some(addr),
        }
    }
    fn is_mem_128(&mut self, src: &XmmMem128) -> Option<SyntheticAmode> {
        match src.clone().into() {
            RegMem::Reg { .. } => None,
            RegMem::Mem { addr } => Some(addr),
        }
    }

    // Custom constructors for `mulx` which only calculates the high half of the
    // result meaning that the same output operand is used in both destination
    // registers. This is in contrast to the assembler-generated version of this
    // instruction which generates two distinct temporary registers for output
    // which calculates both the high and low halves of the result.

    fn x64_mulxl_rvm_hi(&mut self, src1: &GprMem, src2: Gpr) -> Gpr {
        let ret = self.temp_writable_gpr();
        let src1 = self.convert_gpr_mem_to_assembler_read_gpr_mem(src1);
        let inst = asm::inst::mulxl_rvm::new(ret, ret, src1, src2);
        self.emit(&MInst::External { inst: inst.into() });
        ret.to_reg()
    }

    fn x64_mulxq_rvm_hi(&mut self, src1: &GprMem, src2: Gpr) -> Gpr {
        let ret = self.temp_writable_gpr();
        let src1 = self.convert_gpr_mem_to_assembler_read_gpr_mem(src1);
        let inst = asm::inst::mulxq_rvm::new(ret, ret, src1, src2);
        self.emit(&MInst::External { inst: inst.into() });
        ret.to_reg()
    }

    fn bt_imm(&mut self, val: u64) -> Option<u8> {
        if val.count_ones() == 1 {
            Some(u8::try_from(val.trailing_zeros()).unwrap())
        } else {
            None
        }
    }
}

impl IsleContext<'_, '_, MInst, X64Backend> {
    fn load_xmm_unaligned(&mut self, addr: SyntheticAmode) -> Xmm {
        let tmp = self.lower_ctx.alloc_tmp(types::F32X4).only_reg().unwrap();
        self.lower_ctx.emit(MInst::External {
            inst: asm::inst::movdqu_a::new(
                Writable::from_reg(Xmm::unwrap_new(tmp.to_reg())),
                asm::XmmMem::Mem(addr.into()),
            )
            .into(),
        });
        Xmm::unwrap_new(tmp.to_reg())
    }

    /// Helper used by code generated by the `cranelift-assembler-x64` crate.
    fn convert_gpr_to_assembler_read_write_gpr(&mut self, read: Gpr) -> asm::Gpr<PairedGpr> {
        let write = self.lower_ctx.alloc_tmp(types::I64).only_reg().unwrap();
        let write = WritableGpr::from_writable_reg(write).unwrap();
        asm::Gpr::new(PairedGpr { read, write })
    }

    /// Helper used by code generated by the `cranelift-assembler-x64` crate.
    fn convert_gpr_to_assembler_fixed_read_write_gpr<const E: u8>(
        &mut self,
        read: Gpr,
    ) -> asm::Fixed<PairedGpr, E> {
        let write = self.lower_ctx.alloc_tmp(types::I64).only_reg().unwrap();
        let write = WritableGpr::from_writable_reg(write).unwrap();
        asm::Fixed(PairedGpr { read, write })
    }

    /// Helper used by code generated by the `cranelift-assembler-x64` crate.
    fn convert_xmm_to_assembler_read_write_xmm(&mut self, read: Xmm) -> asm::Xmm<PairedXmm> {
        let write = self.lower_ctx.alloc_tmp(types::F32X4).only_reg().unwrap();
        let write = WritableXmm::from_writable_reg(write).unwrap();
        asm::Xmm::new(PairedXmm { read, write })
    }

    /// Helper used by code generated by the `cranelift-assembler-x64` crate.
    // These convert helpers are called from code generated by the
    // `cranelift-assembler-x64` crate; each accepts any sized
    // `{Gpr,Xmm}Mem{Aligned}{N}` newtype via `Clone + Into<RegMem>`, so
    // the width-tagged types produced by the generator flow through a
    // single implementation.

    fn convert_gpr_mem_to_assembler_read_gpr_mem<T: Clone + Into<RegMem>>(
        &self,
        read: &T,
    ) -> asm::GprMem<Gpr, Gpr> {
        match read.clone().into() {
            RegMem::Reg { reg } => asm::GprMem::Gpr(Gpr::new(reg).unwrap()),
            RegMem::Mem { addr } => asm::GprMem::Mem(addr.into()),
        }
    }

    fn convert_xmm_mem_to_assembler_read_xmm_mem_aligned<T: Clone + Into<RegMem>>(
        &self,
        read: &T,
    ) -> asm::XmmMem<Xmm, Gpr> {
        match read.clone().into() {
            RegMem::Reg { reg } => asm::XmmMem::Xmm(Xmm::new(reg).unwrap()),
            RegMem::Mem { addr } => asm::XmmMem::Mem(addr.into()),
        }
    }

    fn convert_xmm_mem_to_assembler_read_xmm_mem<T: Clone + Into<RegMem>>(
        &self,
        read: &T,
    ) -> asm::XmmMem<Xmm, Gpr> {
        match read.clone().into() {
            RegMem::Reg { reg } => asm::XmmMem::Xmm(Xmm::new(reg).unwrap()),
            RegMem::Mem { addr } => asm::XmmMem::Mem(addr.into()),
        }
    }

    fn convert_xmm_mem_to_assembler_write_xmm_mem<T: Clone + Into<RegMem>>(
        &self,
        write: &T,
    ) -> asm::XmmMem<Writable<Xmm>, Gpr> {
        match write.clone().into() {
            RegMem::Reg { reg } => asm::XmmMem::Xmm(Writable::from_reg(Xmm::new(reg).unwrap())),
            RegMem::Mem { addr } => asm::XmmMem::Mem(addr.into()),
        }
    }

    fn convert_xmm_mem_to_assembler_write_xmm_mem_aligned<T: Clone + Into<RegMem>>(
        &self,
        write: &T,
    ) -> asm::XmmMem<Writable<Xmm>, Gpr> {
        match write.clone().into() {
            RegMem::Reg { reg } => asm::XmmMem::Xmm(Writable::from_reg(Xmm::new(reg).unwrap())),
            RegMem::Mem { addr } => asm::XmmMem::Mem(addr.into()),
        }
    }

    fn convert_gpr_mem_to_assembler_read_write_gpr_mem<T: Clone + Into<RegMem>>(
        &mut self,
        read: &T,
    ) -> asm::GprMem<PairedGpr, Gpr> {
        match read.clone().into() {
            RegMem::Reg { reg } => asm::GprMem::Gpr(
                *self
                    .convert_gpr_to_assembler_read_write_gpr(Gpr::new(reg).unwrap())
                    .as_ref(),
            ),
            RegMem::Mem { addr } => asm::GprMem::Mem(addr.into()),
        }
    }

    fn convert_gpr_mem_to_assembler_write_gpr_mem<T: Clone + Into<RegMem>>(
        &mut self,
        read: &T,
    ) -> asm::GprMem<WritableGpr, Gpr> {
        match read.clone().into() {
            RegMem::Reg { reg } => asm::GprMem::Gpr(WritableGpr::from_reg(Gpr::new(reg).unwrap())),
            RegMem::Mem { addr } => asm::GprMem::Mem(addr.into()),
        }
    }

    /// Helper used by code generated by the `cranelift-assembler-x64` crate.
    fn convert_amode_to_assembler_amode(&mut self, amode: &SyntheticAmode) -> asm::Amode<Gpr> {
        amode.clone().into()
    }
}

// Since x64 doesn't have 8x16 shifts and we must use a 16x8 shift instead, we
// need to fix up the bits that migrate from one half of the lane to the
// other. Each 16-byte mask is indexed by the shift amount: e.g. if we shift
// right by 0 (no movement), we want to retain all the bits so we mask with
// `0xff`; if we shift right by 1, we want to retain all bits except the MSB so
// we mask with `0x7f`; etc.

#[rustfmt::skip] // Preserve 16 bytes (i.e. one mask) per row.
const I8X16_ISHL_MASKS: [u8; 128] = [
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe,
    0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc, 0xfc,
    0xf8, 0xf8, 0xf8, 0xf8, 0xf8, 0xf8, 0xf8, 0xf8, 0xf8, 0xf8, 0xf8, 0xf8, 0xf8, 0xf8, 0xf8, 0xf8,
    0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0,
    0xe0, 0xe0, 0xe0, 0xe0, 0xe0, 0xe0, 0xe0, 0xe0, 0xe0, 0xe0, 0xe0, 0xe0, 0xe0, 0xe0, 0xe0, 0xe0,
    0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0, 0xc0,
    0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80,
];

#[rustfmt::skip] // Preserve 16 bytes (i.e. one mask) per row.
const I8X16_USHR_MASKS: [u8; 128] = [
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f,
    0x3f, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f, 0x3f,
    0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f, 0x1f,
    0x0f, 0x0f, 0x0f, 0x0f, 0x0f, 0x0f, 0x0f, 0x0f, 0x0f, 0x0f, 0x0f, 0x0f, 0x0f, 0x0f, 0x0f, 0x0f,
    0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07,
    0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
];
