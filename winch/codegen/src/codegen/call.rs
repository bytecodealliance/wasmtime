//! Function call emission.  For more details around the ABI and
//! calling convention, see [ABI].
//!
//! This module exposes a single function [`FnCall::emit`], which is responsible
//! of orchestrating the emission of calls. In general such orchestration
//! takes place in 6 steps:
//!
//! 1. [`Callee`] resolution.
//! 2. Mapping of the [`Callee`] to the [`CalleeKind`].
//! 3. Spilling the value stack.
//! 4. Calculate the return area, for 1+ results.
//! 5. Emission.
//! 6. Stack space cleanup.
//!
//! The stack space consumed by the function call is the amount
//! of space used by any memory entries in the value stack present
//! at the callsite (after spilling the value stack), that will be
//! used as arguments for the function call. Any memory values in the
//! value stack that are needed as part of the function
//! arguments will be consumed by the function call (either by
//! assigning those values to a register or by storing those
//! values in a memory location if the callee argument is on
//! the stack).
//! This could also be done when assigning arguments every time a
//! memory entry needs to be assigned to a particular location,
//! but doing so will emit more instructions (e.g. a pop per
//! argument that needs to be assigned); it's more efficient to
//! calculate the space used by those memory values and reclaim it
//! at once when cleaning up the stack after the call has been
//! emitted.
//!
//! The machine stack throughout the function call is as follows:
//! ┌──────────────────────────────────────────────────┐
//! │                                                  │
//! │                  1                               │
//! │  Stack space created by any previous spills      │
//! │  from the value stack; and which memory values   │
//! │  are used as function arguments.                 │
//! │                                                  │
//! ├──────────────────────────────────────────────────┤ ---> The Wasm value stack at this point in time would look like:
//! │                                                  │      [ Mem(offset) | Mem(offset) | Local(index) | Local(index) ]
//! │                   2                              │
//! │   Stack space created by spilling locals and     |
//! │   registers at the callsite.                     │
//! │                                                  │
//! │                                                  │
//! ├─────────────────────────────────────────────────┬┤ ---> The Wasm value stack at this point in time would look like:
//! │                                                  │      [ Mem(offset) | Mem(offset) | Mem(offset) | Mem(offset) ]
//! │                                                  │      Assuming that the callee takes 4 arguments, we calculate
//! │                                                  │      4 memory values; all of which will be used as arguments to
//! │   Stack space allocated for                      │      the call via `assign_args`, thus the sum of the size of the
//! │   the callee function arguments in the stack;    │      memory they represent is considered to be consumed by the call.
//! │   represented by `arg_stack_space`               │
//! │                                                  │
//! │                                                  │
//! │                                                  │
//! └──────────────────────────────────────────────────┘ ------> Stack pointer when emitting the call

use crate::{
    abi::{ABIOperand, ABIResultsData, ABISig, RetArea, ABI},
    codegen::{
        ptr_type_from_ptr_size, BuiltinFunction, BuiltinType, Callee, CalleeInfo, CodeGenContext,
        TypedReg,
    },
    masm::{CalleeKind, MacroAssembler, OperandSize, SPOffset},
    reg::Reg,
    stack::Val,
    CallingConvention,
};
use smallvec::SmallVec;
use std::borrow::Cow;
use wasmtime_environ::{PtrSize, VMOffsets, WasmType};

/// All the information needed to emit a function call.
#[derive(Copy, Clone)]
pub(crate) struct FnCall {}

impl FnCall {
    /// Orchestrates the emission of a function call:
    /// 1. Resolves the [`Callee`] through the given callback.
    /// 2. Maps the resolved [`Callee`] to the [`CalleeKind`].
    /// 3. Spills the value stack.
    /// 4. Creates the stack space needed for the return area.
    /// 5. Emits the call.
    /// 6. Cleans up the stack space.
    pub fn emit<M: MacroAssembler, P: PtrSize, R>(
        masm: &mut M,
        context: &mut CodeGenContext,
        mut resolve: R,
    ) where
        R: FnMut(&mut CodeGenContext) -> Callee,
    {
        let callee = resolve(context);
        let ptr_type = ptr_type_from_ptr_size(context.vmoffsets.ptr.size());
        let sig = Self::get_sig::<M>(&callee, ptr_type);
        let sig = sig.as_ref();
        let kind = Self::map(&context.vmoffsets, &callee, sig, context, masm);

        context.spill(masm);
        let ret_area = Self::make_ret_area(&sig, masm);
        let arg_stack_space = sig.params_stack_size();
        let reserved_stack = masm.call(arg_stack_space, |masm| {
            Self::assign(sig, ret_area.as_ref(), context, masm);
            kind
        });

        match kind {
            CalleeKind::Indirect(r) => context.free_reg(r),
            _ => {}
        }

        Self::cleanup(sig, reserved_stack, ret_area, masm, context);
    }

    /// Calculates the return area for the callee, if any.
    fn make_ret_area<M: MacroAssembler>(callee_sig: &ABISig, masm: &mut M) -> Option<RetArea> {
        callee_sig.results.has_stack_results().then(|| {
            masm.reserve_stack(callee_sig.results_stack_size());
            RetArea::sp(masm.sp_offset())
        })
    }

    /// Derive the [`ABISig`] for a particular [`Callee`].
    fn get_sig<M: MacroAssembler>(callee: &Callee, ptr_type: WasmType) -> Cow<'_, ABISig> {
        match callee {
            Callee::Builtin(info) => Cow::Borrowed(info.sig()),
            Callee::Import(info) => {
                let mut params: SmallVec<[WasmType; 6]> =
                    SmallVec::with_capacity(info.ty.params().len() + 2);
                params.extend_from_slice(&[ptr_type, ptr_type]);
                params.extend_from_slice(info.ty.params());
                Cow::Owned(<M::ABI as ABI>::sig_from(
                    &params,
                    info.ty.returns(),
                    &CallingConvention::Default,
                ))
            }
            Callee::Local(info) => {
                Cow::Owned(<M::ABI as ABI>::sig(&info.ty, &CallingConvention::Default))
            }
            Callee::FuncRef(ty) => {
                Cow::Owned(<M::ABI as ABI>::sig(&ty, &CallingConvention::Default))
            }
        }
    }

    /// Maps the given [`Callee`] to a [`CalleeKind`].
    fn map<P: PtrSize, M: MacroAssembler>(
        vmoffsets: &VMOffsets<P>,
        callee: &Callee,
        sig: &ABISig,
        context: &mut CodeGenContext,
        masm: &mut M,
    ) -> CalleeKind {
        match callee {
            Callee::Builtin(b) => Self::load_builtin(b, context, masm),
            Callee::FuncRef(_) => Self::load_funcref(sig, vmoffsets.ptr.size(), context, masm),
            Callee::Local(i) => Self::map_local(i),
            Callee::Import(i) => Self::load_import(i, sig, context, masm, vmoffsets),
        }
    }

    /// Load a built-in function to the next available register.
    fn load_builtin<M: MacroAssembler>(
        builtin: &BuiltinFunction,
        context: &mut CodeGenContext,
        masm: &mut M,
    ) -> CalleeKind {
        match builtin.ty() {
            BuiltinType::Dynamic { index, base } => {
                let sig = builtin.sig();
                let callee = context.without::<Reg, _, _>(&sig.regs, masm, |cx, masm| {
                    let scratch = <M::ABI as ABI>::scratch_reg();
                    let builtins_base = masm.address_at_vmctx(base);
                    masm.load_ptr(builtins_base, scratch);
                    let addr = masm.address_at_reg(scratch, index);
                    let callee = cx.any_gpr(masm);
                    masm.load_ptr(addr, callee);
                    callee
                });
                CalleeKind::indirect(callee)
            }
            BuiltinType::Known(c) => CalleeKind::known(c),
        }
    }

    /// Map a local function to a [`CalleeKind`].
    fn map_local(info: &CalleeInfo) -> CalleeKind {
        CalleeKind::direct(info.index.as_u32())
    }

    /// Loads a function import to the next available register.
    fn load_import<M: MacroAssembler, P: PtrSize>(
        info: &CalleeInfo,
        sig: &ABISig,
        context: &mut CodeGenContext,
        masm: &mut M,
        vmoffsets: &VMOffsets<P>,
    ) -> CalleeKind {
        let ptr_type = ptr_type_from_ptr_size(vmoffsets.ptr.size());
        let caller_vmctx = <M::ABI as ABI>::vmctx_reg();
        let (callee, callee_vmctx) =
            context.without::<(Reg, Reg), M, _>(&sig.regs, masm, |context, masm| {
                (context.any_gpr(masm), context.any_gpr(masm))
            });
        let callee_vmctx_offset = vmoffsets.vmctx_vmfunction_import_vmctx(info.index);
        let callee_vmctx_addr = masm.address_at_vmctx(callee_vmctx_offset);
        masm.load_ptr(callee_vmctx_addr, callee_vmctx);

        let callee_body_offset = vmoffsets.vmctx_vmfunction_import_wasm_call(info.index);
        let callee_addr = masm.address_at_vmctx(callee_body_offset);
        masm.load_ptr(callee_addr, callee);

        // Put the callee / caller vmctx at the start of the
        // range of the stack so that they are used as first
        // and second arguments.
        let stack = &mut context.stack;
        let location = stack.len().checked_sub(sig.params.len() - 2).unwrap_or(0);
        context.stack.insert_many(
            location,
            [
                TypedReg::new(ptr_type, callee_vmctx).into(),
                TypedReg::new(ptr_type, caller_vmctx).into(),
            ],
        );

        CalleeKind::indirect(callee)
    }

    /// Loads a function reference to the next available register.
    fn load_funcref<M: MacroAssembler>(
        sig: &ABISig,
        ptr: impl PtrSize,
        context: &mut CodeGenContext,
        masm: &mut M,
    ) -> CalleeKind {
        // Pop the funcref pointer to a register and allocate a register to hold the
        // address of the funcref. Since the callee is not addressed from a global non
        // allocatable register (like the vmctx in the case of an import), we load the
        // funcref to a register ensuring that it doesn't get assigned to a register
        // used in the callee's signature.
        let (funcref_ptr, funcref) = context.without::<_, M, _>(&sig.regs, masm, |cx, masm| {
            (cx.pop_to_reg(masm, None).into(), cx.any_gpr(masm))
        });

        masm.load_ptr(
            masm.address_at_reg(funcref_ptr, ptr.vm_func_ref_wasm_call().into()),
            funcref,
        );
        context.free_reg(funcref_ptr);
        CalleeKind::indirect(funcref)
    }

    /// Assign arguments for the function call.
    fn assign<M: MacroAssembler>(
        sig: &ABISig,
        ret_area: Option<&RetArea>,
        context: &mut CodeGenContext,
        masm: &mut M,
    ) {
        let arg_count = sig.params.len_without_retptr();
        let stack = &context.stack;
        let stack_values = stack.peekn(arg_count);
        for (arg, val) in sig.params_without_retptr().iter().zip(stack_values) {
            match arg {
                &ABIOperand::Reg { reg, .. } => {
                    context.move_val_to_reg(&val, reg, masm);
                }
                &ABIOperand::Stack { ty, offset, .. } => {
                    let addr = masm.address_at_sp(SPOffset::from_u32(offset));
                    let size: OperandSize = ty.into();
                    let scratch = <M::ABI as ABI>::scratch_for(&ty);
                    context.move_val_to_reg(val, scratch, masm);
                    masm.store(scratch.into(), addr, size);
                }
            }
        }

        if sig.has_stack_results() {
            let operand = sig.params.unwrap_results_area_operand();
            let base = ret_area.unwrap().unwrap_sp();
            let addr = masm.address_from_sp(base);

            match operand {
                &ABIOperand::Reg { ty, reg, .. } => {
                    masm.load_addr(addr, reg, ty.into());
                }
                &ABIOperand::Stack { ty, offset, .. } => {
                    let slot = masm.address_at_sp(SPOffset::from_u32(offset));
                    // Don't rely on `ABI::scratch_for` as we always use
                    // an int register as the return pointer.
                    let scratch = <M::ABI as ABI>::scratch_reg();
                    masm.load_addr(addr, scratch, ty.into());
                    masm.store(scratch.into(), slot, ty.into());
                }
            }
        }
    }

    /// Cleanup stack space, handle multiple results, and free registers after
    /// emitting the call.
    fn cleanup<M: MacroAssembler>(
        sig: &ABISig,
        reserved_space: u32,
        ret_area: Option<RetArea>,
        masm: &mut M,
        context: &mut CodeGenContext,
    ) {
        // Deallocate the reserved space for stack arguments and for alignment,
        // which was allocated last.
        masm.free_stack(reserved_space);

        // Drop params from value stack and calculate amount of machine stack
        // space they consumed.
        let mut stack_consumed = 0;
        context.drop_last(sig.params.len_without_retptr(), |_regalloc, v| {
            debug_assert!(v.is_mem() || v.is_const());
            if let Val::Memory(mem) = v {
                stack_consumed += mem.slot.size;
            }
        });

        if let Some(ret_area) = ret_area {
            if stack_consumed > 0 {
                // Perform a memory move, by shuffling the result area to
                // higher addresses. This is needed because the result area
                // is located after any memory addresses located on the stack,
                // and after spilled values consumed by the call.
                let sp = ret_area.unwrap_sp();
                let result_bytes = sig.results_stack_size();
                debug_assert!(sp.as_u32() >= stack_consumed + result_bytes);
                let dst = SPOffset::from_u32(sp.as_u32() - stack_consumed);
                masm.memmove(sp, dst, result_bytes);
            }
        };

        // Free the bytes consumed by the call.
        masm.free_stack(stack_consumed);

        let mut results_data = ABIResultsData::wrap(sig.results.clone());
        results_data.ret_area = ret_area;

        context.push_abi_results(&results_data, masm);
    }
}
