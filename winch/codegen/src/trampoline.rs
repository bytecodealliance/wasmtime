//! Trampoline implementation for Winch.
//!
//! This module contains all the necessary pieces to emit the various
//! trampolines required by Wasmtime to call JIT code.
//
// TODO
//
// * Remove the usage of hardcoded operand sizes (`OperandSize::S64`) when
// loading/storing the VM context pointer. The real value of the operand size
// and VM context type should be derived from the ABI's pointer size. This is
// going to be relevant once 32-bit architectures are supported.
use crate::{
    abi::{ABIArg, ABIParams, ABIResult, ABISig, ABI},
    isa::CallingConvention,
    masm::{CalleeKind, MacroAssembler, OperandSize, RegImm},
    reg::Reg,
};
use anyhow::{anyhow, Result};
use smallvec::SmallVec;
use std::mem;
use wasmtime_environ::{FuncIndex, PtrSize, WasmFuncType, WasmType};

/// The supported trampoline kinds.
/// See <https://github.com/bytecodealliance/rfcs/blob/main/accepted/tail-calls.md#new-trampolines-and-vmcallercheckedanyfunc-changes>
/// for more details.
pub enum TrampolineKind {
    /// Calling from native to Wasm, using the array calling convention.
    ArrayToWasm(FuncIndex),
    /// Calling from native to Wasm.
    NativeToWasm(FuncIndex),
    /// Calling from Wasm to native.
    WasmToNative,
}

/// The main trampoline abstraction.
pub(crate) struct Trampoline<'a, M>
where
    M: MacroAssembler,
{
    /// The macro assembler.
    masm: &'a mut M,
    /// The main scratch register for the current architecture. It is
    /// not allocatable for the callee.
    scratch_reg: Reg,
    /// A second scratch register. This will be allocatable for the
    /// callee, so it can only be used after the callee-saved
    /// registers are on the stack.
    alloc_scratch_reg: Reg,
    /// Registers to be saved as part of the trampoline's prologue
    /// and to be restored as part of the trampoline's epilogue.
    callee_saved_regs: SmallVec<[Reg; 9]>,
    /// The calling convention used by the trampoline,
    /// which is the Wasmtime variant of the system ABI's
    /// calling convention.
    call_conv: &'a CallingConvention,
    /// The pointer size of the current ISA.
    pointer_size: M::Ptr,
}

impl<'a, M> Trampoline<'a, M>
where
    M: MacroAssembler,
{
    /// Create a new trampoline.
    pub fn new(
        masm: &'a mut M,
        scratch_reg: Reg,
        alloc_scratch_reg: Reg,
        call_conv: &'a CallingConvention,
        pointer_size: M::Ptr,
    ) -> Self {
        Self {
            masm,
            scratch_reg,
            alloc_scratch_reg,
            callee_saved_regs: <M::ABI as ABI>::callee_saved_regs(call_conv),
            call_conv,
            pointer_size,
        }
    }

    /// Emit an array-to-wasm trampoline.
    pub fn emit_array_to_wasm(&mut self, ty: &WasmFuncType, callee_index: FuncIndex) -> Result<()> {
        let native_ty = WasmFuncType::new(
            [WasmType::I64, WasmType::I64, WasmType::I64, WasmType::I64].into(),
            [].into(),
        );

        let native_sig = self.native_sig(&native_ty);
        let wasm_sig = self.wasm_sig(ty);

        let val_ptr = &native_sig.params[2]
            .get_reg()
            .map(RegImm::reg)
            .ok_or_else(|| anyhow!("Expected value pointer to be in a register"))?;

        self.prologue_with_callee_saved();

        // Get the VM context pointer and move it to the designated pinned
        // register.
        let (vmctx, caller_vmctx) = Self::callee_and_caller_vmctx(&native_sig.params)?;

        self.masm.mov(
            vmctx.into(),
            <M::ABI as ABI>::vmctx_reg().into(),
            OperandSize::S64,
        );

        let vmctx_runtime_limits_addr = self.vmctx_runtime_limits_addr(caller_vmctx);
        let (offsets, spill_size) = self.spill(&native_sig.params);

        let val_ptr_offset = offsets[2];

        // Call the function that was passed into the trampoline.
        let allocated_stack = self.masm.call(wasm_sig.stack_bytes, |masm| {
            // Save the SP when entering Wasm.
            // TODO: Once Winch supports comparison operators,
            // check that the caller VM context is what we expect.
            // See [`wasmtime_environ::MAGIC`].
            Self::save_last_wasm_entry_sp(
                masm,
                vmctx_runtime_limits_addr,
                self.scratch_reg,
                &self.pointer_size,
            );

            // Move the values register to the scratch
            // register for argument assignment.
            masm.mov(*val_ptr, self.scratch_reg.into(), OperandSize::S64);
            Self::assign_args_from_array(masm, &wasm_sig, self.scratch_reg, self.alloc_scratch_reg);
            CalleeKind::Direct(callee_index.as_u32())
        });

        self.masm.free_stack(allocated_stack);

        // Move the val ptr back into the scratch register so we can
        // load the return values.
        self.masm.load(
            self.masm.address_from_sp(val_ptr_offset),
            self.scratch_reg,
            OperandSize::S64,
        );

        // Move the return values into the value ptr.  We are only
        // supporting a single return value at this time.
        let ABIResult::Reg { reg, ty } = &wasm_sig.result;
        if let Some(ty) = ty {
            self.masm.store(
                RegImm::reg(*reg),
                self.masm.address_at_reg(self.scratch_reg, 0),
                (*ty).into(),
            );
        }

        self.epilogue_with_callee_saved_restore(spill_size);
        Ok(())
    }

    /// Emit a native-to-wasm trampoline.
    pub fn emit_native_to_wasm(
        &mut self,
        ty: &WasmFuncType,
        callee_index: FuncIndex,
    ) -> Result<()> {
        let native_sig = self.native_sig(&ty);
        let wasm_sig = self.wasm_sig(&ty);
        let (vmctx, caller_vmctx) = Self::callee_and_caller_vmctx(&native_sig.params)?;

        self.prologue_with_callee_saved();
        // Move the VM context pointer to the designated pinned register.
        self.masm.mov(
            vmctx.into(),
            <M::ABI as ABI>::vmctx_reg().into(),
            OperandSize::S64,
        );

        let vmctx_runtime_limits_addr = self.vmctx_runtime_limits_addr(caller_vmctx);
        let (offsets, spill_size) = self.spill(&native_sig.params);

        let reserved_stack = self.masm.call(wasm_sig.stack_bytes, |masm| {
            // Save the SP when entering Wasm.
            // TODO: Once Winch supports comparison operators,
            // check that the caller VM context is what we expect.
            // See [`wasmtime_environ::MAGIC`].
            Self::save_last_wasm_entry_sp(
                masm,
                vmctx_runtime_limits_addr,
                self.scratch_reg,
                &self.pointer_size,
            );
            Self::assign_args(
                masm,
                &wasm_sig.params,
                &native_sig.params[2..],
                &offsets[2..],
                self.scratch_reg,
                <M::ABI as ABI>::arg_base_offset().into(),
            );
            CalleeKind::Direct(callee_index.as_u32())
        });

        self.masm.free_stack(reserved_stack);
        self.epilogue_with_callee_saved_restore(spill_size);

        Ok(())
    }

    /// Emit a wasm-to-native trampoline.
    pub fn emit_wasm_to_native(&mut self, ty: &WasmFuncType) -> Result<()> {
        let mut params = Self::callee_and_caller_vmctx_types();
        params.extend_from_slice(ty.params());

        let func_ty = WasmFuncType::new(params.into(), ty.returns().into());
        let wasm_sig = self.wasm_sig(&func_ty);
        let native_sig = self.native_sig(ty);

        let (vmctx, caller_vmctx) = Self::callee_and_caller_vmctx(&wasm_sig.params).unwrap();
        let vmctx_runtime_limits_addr = self.vmctx_runtime_limits_addr(caller_vmctx);

        self.prologue();

        // Save the FP and return address when exiting Wasm.
        // TODO: Once Winch supports comparison operators,
        // check that the caller VM context is what we expect.
        // See [`wasmtime_environ::MAGIC`].
        Self::save_last_wasm_exit_fp_and_pc(
            self.masm,
            vmctx_runtime_limits_addr,
            self.scratch_reg,
            self.alloc_scratch_reg,
            &self.pointer_size,
        );

        let (offsets, spill_size) = self.spill(&wasm_sig.params);

        let reserved_stack = self.masm.call(native_sig.stack_bytes, |masm| {
            // Move the VM context into one of the scratch registers.
            masm.mov(
                vmctx.into(),
                self.alloc_scratch_reg.into(),
                OperandSize::S64,
            );

            Self::assign_args(
                masm,
                &native_sig.params,
                &wasm_sig.params,
                &offsets,
                self.scratch_reg,
                <M::ABI as ABI>::arg_base_offset().into(),
            );

            let body_offset = self.pointer_size.vmnative_call_host_func_context_func_ref()
                + self.pointer_size.vm_func_ref_native_call();
            let callee_addr = masm.address_at_reg(self.alloc_scratch_reg, body_offset.into());
            masm.load(callee_addr, self.scratch_reg, OperandSize::S64);

            CalleeKind::Indirect(self.scratch_reg)
        });

        self.masm.free_stack(reserved_stack);
        self.epilogue(spill_size);

        Ok(())
    }

    /// Perfom argument assignment, translating between
    /// caller and callee calling conventions.
    fn assign_args(
        masm: &mut M,
        callee_params: &[ABIArg],
        caller_params: &[ABIArg],
        caller_stack_offsets: &[u32],
        scratch: Reg,
        arg_base_offset: u32,
    ) {
        assert!(callee_params.len() == caller_params.len());
        let fp = <M::ABI as ABI>::fp_reg();
        let mut offset_index = 0;

        callee_params
            .iter()
            .zip(caller_params)
            .for_each(
                |(callee_param, caller_param)| match (callee_param, caller_param) {
                    (ABIArg::Reg { ty, reg: dst }, ABIArg::Reg { .. }) => {
                        let offset = caller_stack_offsets[offset_index];
                        let addr = masm.address_from_sp(offset);
                        masm.load(addr, *dst, (*ty).into());
                        offset_index += 1;
                    }

                    (ABIArg::Stack { ty, offset }, ABIArg::Reg { .. }) => {
                        let spill_offset = caller_stack_offsets[offset_index];
                        let addr = masm.address_from_sp(spill_offset);
                        masm.load(addr, scratch, (*ty).into());

                        let arg_addr = masm.address_at_sp(*offset);
                        masm.store(scratch.into(), arg_addr, (*ty).into());
                        offset_index += 1;
                    }

                    (ABIArg::Reg { ty, reg: dst }, ABIArg::Stack { ty: _, offset }) => {
                        let addr = masm.address_at_reg(fp, arg_base_offset + offset);
                        masm.load(addr, *dst, (*ty).into());
                    }

                    (
                        ABIArg::Stack {
                            ty,
                            offset: callee_offset,
                        },
                        ABIArg::Stack {
                            offset: caller_offset,
                            ..
                        },
                    ) => {
                        let addr = masm.address_at_reg(fp, arg_base_offset + caller_offset);
                        masm.load(addr, scratch, (*ty).into());

                        let arg_addr = masm.address_at_sp(*callee_offset);
                        masm.store(scratch.into(), arg_addr, (*ty).into());
                    }
                },
            )
    }

    /// Get the type of the caller and callee VM contexts.
    fn callee_and_caller_vmctx_types() -> Vec<WasmType> {
        vec![WasmType::I64, WasmType::I64]
    }

    /// Returns a signature using the system's calling convention.
    fn native_sig(&self, ty: &WasmFuncType) -> ABISig {
        let mut params = Self::callee_and_caller_vmctx_types();
        params.extend_from_slice(ty.params());
        let native_type = WasmFuncType::new(params.into(), ty.returns().into());

        <M::ABI as ABI>::sig(&native_type, self.call_conv)
    }

    /// Returns a signature using the Winch's default calling convention.
    fn wasm_sig(&self, ty: &WasmFuncType) -> ABISig {
        <M::ABI as ABI>::sig(ty, &CallingConvention::Default)
    }

    /// Returns the register pair containing the callee and caller VM context pointers.
    fn callee_and_caller_vmctx(params: &ABIParams) -> Result<(Reg, Reg)> {
        let vmctx = params[0]
            .get_reg()
            .ok_or_else(|| anyhow!("Expected vm context pointer to be in a register"))?;

        let caller_vmctx = params[1]
            .get_reg()
            .ok_or_else(|| anyhow!("Expected caller vm context pointer to be in a register"))?;

        Ok((vmctx, caller_vmctx))
    }

    /// Returns the address of the VM context runtime limits
    /// field.
    fn vmctx_runtime_limits_addr(&mut self, caller_vmctx: Reg) -> M::Address {
        self.masm.address_at_reg(
            caller_vmctx,
            self.pointer_size.vmcontext_runtime_limits().into(),
        )
    }

    /// Performs a spill of the register params.
    fn spill(&mut self, params: &ABIParams) -> (SmallVec<[u32; 6]>, u32) {
        let mut offsets = SmallVec::new();
        let mut spilled = 0;
        params.iter().for_each(|param| {
            if let Some(reg) = param.get_reg() {
                let offset = self.masm.push(reg);
                offsets.push(offset);
                spilled += 1;
            }
        });

        // The stack size for the spill, calculated
        // from the number of spilled register times
        // the size of each push (8 bytes).
        let size = spilled * <M::ABI as ABI>::word_bytes();

        (offsets, size)
    }

    /// Assigns arguments for the callee, loading them from a register.
    fn assign_args_from_array(masm: &mut M, callee_sig: &ABISig, values_reg: Reg, scratch: Reg) {
        // The max size a value can be when reading from the params
        // memory location.
        let value_size = mem::size_of::<u128>();
        callee_sig.params.iter().enumerate().for_each(|(i, param)| {
            let value_offset = (i * value_size) as u32;

            match param {
                ABIArg::Reg { reg, ty } => masm.load(
                    masm.address_at_reg(values_reg, value_offset),
                    *reg,
                    (*ty).into(),
                ),
                ABIArg::Stack { offset, ty } => {
                    masm.load(
                        masm.address_at_reg(values_reg, value_offset),
                        scratch,
                        (*ty).into(),
                    );
                    masm.store(
                        RegImm::reg(scratch),
                        masm.address_at_sp(*offset),
                        (*ty).into(),
                    );
                }
            }
        });
    }

    fn save_last_wasm_entry_sp(
        masm: &mut M,
        vm_runtime_limits_addr: M::Address,
        scratch: Reg,
        ptr: &impl PtrSize,
    ) {
        let sp = <M::ABI as ABI>::sp_reg();
        masm.load(vm_runtime_limits_addr, scratch, OperandSize::S64);
        let addr = masm.address_at_reg(scratch, ptr.vmruntime_limits_last_wasm_entry_sp().into());
        masm.store(sp.into(), addr, OperandSize::S64);
    }

    fn save_last_wasm_exit_fp_and_pc(
        masm: &mut M,
        vm_runtime_limits_addr: M::Address,
        scratch: Reg,
        alloc_scratch: Reg,
        ptr: &impl PtrSize,
    ) {
        masm.load(vm_runtime_limits_addr, alloc_scratch, OperandSize::S64);
        let last_wasm_exit_fp_addr = masm.address_at_reg(
            alloc_scratch,
            ptr.vmruntime_limits_last_wasm_exit_fp().into(),
        );
        let last_wasm_exit_pc_addr = masm.address_at_reg(
            alloc_scratch,
            ptr.vmruntime_limits_last_wasm_exit_pc().into(),
        );

        // Handle the frame pointer.
        let fp = <M::ABI as ABI>::fp_reg();
        let fp_addr = masm.address_at_reg(fp, 0);
        masm.load(fp_addr, scratch, OperandSize::S64);
        masm.store(scratch.into(), last_wasm_exit_fp_addr, OperandSize::S64);

        // Handle the return address.
        let ret_addr_offset = <M::ABI as ABI>::ret_addr_offset();
        let ret_addr = masm.address_at_reg(fp, ret_addr_offset.into());
        masm.load(ret_addr, scratch, OperandSize::S64);
        masm.store(scratch.into(), last_wasm_exit_pc_addr, OperandSize::S64);
    }

    /// The trampoline's prologue.
    fn prologue(&mut self) {
        self.masm.prologue();
    }

    /// Similar to [Trampoline::prologue], but saves
    /// callee-saved registers.
    fn prologue_with_callee_saved(&mut self) {
        self.masm.prologue();
        // Save any callee-saved registers.
        for r in &self.callee_saved_regs {
            self.masm.push(*r);
        }
    }

    /// Similar to [Trampoline::epilogue], but restores
    /// callee-saved registers.
    fn epilogue_with_callee_saved_restore(&mut self, arg_size: u32) {
        // Free the stack space allocated by pushing the trampoline arguments.
        self.masm.free_stack(arg_size);
        // Restore the callee-saved registers.
        for r in self.callee_saved_regs.iter().rev() {
            self.masm.pop(*r);
        }
        self.masm.epilogue(0);
    }

    /// The trampoline's epilogue.
    fn epilogue(&mut self, arg_size: u32) {
        // Free the stack space allocated by pushing the trampoline arguments.
        self.masm.free_stack(arg_size);
        self.masm.epilogue(0);
    }
}
