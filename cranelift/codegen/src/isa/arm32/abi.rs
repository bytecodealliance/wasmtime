//! Implementation of a standard ARM32 ABI (AAPCS).

use crate::isa::arm32::inst::Inst;
use crate::isa::arm32::settings::Flags as Arm32Flags;
use crate::machinst::{ABIMachineSpec, Callee, IsaFlags};

pub(crate) type Arm32Callee = Callee<Arm32MachineDeps>;

pub struct Arm32MachineDeps;

impl IsaFlags for Arm32Flags {}

impl ABIMachineSpec for Arm32MachineDeps {
    type I = Inst;
    type F = Arm32Flags;

    const STACK_ARG_RET_SIZE_LIMIT: u32 = 128 * 1024 * 1024;

    fn word_bits() -> u32 {
        32
    }

    fn stack_align(_call_conv: crate::isa::CallConv) -> u32 {
        8
    }

    fn compute_arg_locs(
        _call_conv: crate::isa::CallConv,
        _flags: &crate::settings::Flags,
        _params: &[crate::ir::AbiParam],
        _args_or_rets: crate::machinst::ArgsOrRets,
        _add_ret_area_ptr: bool,
        _args: crate::machinst::ArgsAccumulator,
    ) -> crate::CodegenResult<(u32, Option<usize>)> {
        todo!()
    }

    fn gen_load_stack(
        _mem: crate::machinst::StackAMode,
        _into_reg: crate::Writable<crate::Reg>,
        _ty: crate::ir::Type,
    ) -> Self::I {
        todo!()
    }

    fn gen_store_stack(
        _mem: crate::machinst::StackAMode,
        _from_reg: crate::Reg,
        _ty: crate::ir::Type,
    ) -> Self::I {
        todo!()
    }

    fn gen_move(
        _to_reg: crate::Writable<crate::Reg>,
        _from_reg: crate::Reg,
        _ty: crate::ir::Type,
    ) -> Self::I {
        todo!()
    }

    fn gen_extend(
        _to_reg: crate::Writable<crate::Reg>,
        _from_reg: crate::Reg,
        _is_signed: bool,
        _from_bits: u8,
        _to_bits: u8,
    ) -> Self::I {
        todo!()
    }

    fn gen_args(_args: std::prelude::v1::Vec<crate::machinst::ArgPair>) -> Self::I {
        todo!()
    }

    fn gen_rets(_rets: std::prelude::v1::Vec<crate::machinst::RetPair>) -> Self::I {
        todo!()
    }

    fn gen_add_imm(
        _call_conv: crate::isa::CallConv,
        _into_reg: crate::Writable<crate::Reg>,
        _from_reg: crate::Reg,
        _imm: u32,
    ) -> crate::machinst::SmallInstVec<Self::I> {
        todo!()
    }

    fn gen_stack_lower_bound_trap(
        _limit_reg: crate::Reg,
    ) -> crate::machinst::SmallInstVec<Self::I> {
        todo!()
    }

    fn gen_get_stack_addr(
        _mem: crate::machinst::StackAMode,
        _into_reg: crate::Writable<crate::Reg>,
    ) -> Self::I {
        todo!()
    }

    fn get_stacklimit_reg(_call_conv: crate::isa::CallConv) -> crate::Reg {
        todo!()
    }

    fn gen_load_base_offset(
        _into_reg: crate::Writable<crate::Reg>,
        _base: crate::Reg,
        _offset: i32,
        _ty: crate::ir::Type,
    ) -> Self::I {
        todo!()
    }

    fn gen_store_base_offset(
        _base: crate::Reg,
        _offset: i32,
        _from_reg: crate::Reg,
        _ty: crate::ir::Type,
    ) -> Self::I {
        todo!()
    }

    fn gen_sp_reg_adjust(_amount: i32) -> crate::machinst::SmallInstVec<Self::I> {
        todo!()
    }

    fn compute_frame_layout(
        _call_conv: crate::isa::CallConv,
        _flags: &crate::settings::Flags,
        _sig: &crate::ir::Signature,
        _regs: &[crate::Writable<crate::RealReg>],
        _function_calls: crate::machinst::FunctionCalls,
        _incoming_args_size: u32,
        _tail_args_size: u32,
        _stackslots_size: u32,
        _fixed_frame_storage_size: u32,
        _outgoing_args_size: u32,
    ) -> crate::FrameLayout {
        todo!()
    }

    fn gen_prologue_frame_setup(
        _call_conv: crate::isa::CallConv,
        _flags: &crate::settings::Flags,
        _isa_flags: &Self::F,
        _frame_layout: &crate::FrameLayout,
    ) -> crate::machinst::SmallInstVec<Self::I> {
        todo!()
    }

    fn gen_epilogue_frame_restore(
        _call_conv: crate::isa::CallConv,
        _flags: &crate::settings::Flags,
        _isa_flags: &Self::F,
        _frame_layout: &crate::FrameLayout,
    ) -> crate::machinst::SmallInstVec<Self::I> {
        todo!()
    }

    fn gen_return(
        _call_conv: crate::isa::CallConv,
        _isa_flags: &Self::F,
        _frame_layout: &crate::FrameLayout,
    ) -> crate::machinst::SmallInstVec<Self::I> {
        todo!()
    }

    fn gen_probestack(_insts: &mut crate::machinst::SmallInstVec<Self::I>, _frame_size: u32) {
        todo!()
    }

    fn gen_inline_probestack(
        _insts: &mut crate::machinst::SmallInstVec<Self::I>,
        _call_conv: crate::isa::CallConv,
        _frame_size: u32,
        _guard_size: u32,
    ) {
        todo!()
    }

    fn gen_clobber_save(
        _call_conv: crate::isa::CallConv,
        _flags: &crate::settings::Flags,
        _frame_layout: &crate::FrameLayout,
    ) -> smallvec::SmallVec<[Self::I; 16]> {
        todo!()
    }

    fn gen_clobber_restore(
        _call_conv: crate::isa::CallConv,
        _flags: &crate::settings::Flags,
        _frame_layout: &crate::FrameLayout,
    ) -> smallvec::SmallVec<[Self::I; 16]> {
        todo!()
    }

    fn gen_memcpy<F: FnMut(crate::ir::Type) -> crate::Writable<crate::Reg>>(
        _call_conv: crate::isa::CallConv,
        _dst: crate::Reg,
        _src: crate::Reg,
        _size: usize,
        _alloc_tmp: F,
    ) -> smallvec::SmallVec<[Self::I; 8]> {
        todo!()
    }

    fn get_number_of_spillslots_for_value(
        _rc: crate::RegClass,
        _target_vector_bytes: u32,
        _isa_flags: &Self::F,
    ) -> u32 {
        todo!()
    }

    fn get_machine_env(
        _flags: &crate::settings::Flags,
        _call_conv: crate::isa::CallConv,
    ) -> &regalloc2::MachineEnv {
        todo!()
    }

    fn get_regs_clobbered_by_call(
        _call_conv_of_callee: crate::isa::CallConv,
        _is_exception: bool,
    ) -> regalloc2::PRegSet {
        todo!()
    }

    fn get_ext_mode(
        _call_conv: crate::isa::CallConv,
        _specified: crate::ir::ArgumentExtension,
    ) -> crate::ir::ArgumentExtension {
        todo!()
    }

    fn retval_temp_reg(_call_conv_of_callee: crate::isa::CallConv) -> crate::Writable<crate::Reg> {
        todo!()
    }
}
