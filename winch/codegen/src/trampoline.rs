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
    abi::{ABIOperand, ABIParams, ABISig, RetArea, ABI},
    codegen::ptr_type_from_ptr_size,
    isa::CallingConvention,
    masm::{CalleeKind, MacroAssembler, OperandSize, RegImm, SPOffset},
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

/// The max value size of an element in the array calling convention.
const VALUE_SIZE: usize = mem::size_of::<u128>();

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
    callee_saved_regs: SmallVec<[(Reg, OperandSize); 18]>,
    /// The calling convention used by the trampoline,
    /// which is the Wasmtime variant of the system ABI's
    /// calling convention.
    call_conv: &'a CallingConvention,
    /// The pointer size of the current ISA.
    pointer_size: M::Ptr,
    /// WasmType representation of the pointer size.
    pointer_type: WasmType,
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
        let size = pointer_size.size();
        Self {
            masm,
            scratch_reg,
            alloc_scratch_reg,
            callee_saved_regs: <M::ABI as ABI>::callee_saved_regs(call_conv),
            call_conv,
            pointer_size,
            pointer_type: ptr_type_from_ptr_size(size),
        }
    }

    /// Emit an array-to-wasm trampoline.
    pub fn emit_array_to_wasm(&mut self, ty: &WasmFuncType, callee_index: FuncIndex) -> Result<()> {
        let array_sig = self.array_sig();
        let wasm_sig = self.wasm_sig(ty);

        let val_ptr = array_sig
            .params
            .get(2)
            .map(|operand| RegImm::reg(operand.unwrap_reg()))
            .ok_or_else(|| anyhow!("Expected value pointer to be in a register"))?;

        self.prologue_with_callee_saved();

        // Get the VM context pointer and move it to the designated pinned
        // register.
        let (vmctx, caller_vmctx) = Self::callee_and_caller_vmctx(&array_sig.params)?;

        self.masm.mov(
            vmctx.into(),
            <M::ABI as ABI>::vmctx_reg().into(),
            OperandSize::S64,
        );

        let ret_area = self.make_ret_area(&wasm_sig);
        let vmctx_runtime_limits_addr = self.vmctx_runtime_limits_addr(caller_vmctx);
        let (offsets, spill_size) = self.spill(array_sig.params());

        // Call the function that was passed into the trampoline.
        let allocated_stack = self.masm.call(wasm_sig.params_stack_size(), |masm| {
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
            masm.mov(val_ptr, self.scratch_reg.into(), OperandSize::S64);
            Self::load_values_from_array(
                masm,
                &wasm_sig,
                ret_area.as_ref(),
                self.scratch_reg,
                self.alloc_scratch_reg,
            );
            CalleeKind::Direct(callee_index.as_u32())
        });

        self.masm.free_stack(allocated_stack);

        // Move the val ptr back into the scratch register so we can
        // load the return values.
        let val_ptr_offset = offsets[2];
        self.masm.load(
            self.masm.address_from_sp(val_ptr_offset),
            self.scratch_reg,
            OperandSize::S64,
        );

        self.store_results_to_array(&wasm_sig, ret_area.as_ref());

        if wasm_sig.has_stack_results() {
            self.masm.free_stack(wasm_sig.results.size());
        }

        self.epilogue_with_callee_saved_restore(spill_size);
        Ok(())
    }

    /// Stores the results into the values array used by the array calling
    /// convention.
    fn store_results_to_array(&mut self, sig: &ABISig, ret_area: Option<&RetArea>) {
        for (i, operand) in sig.results().iter().enumerate() {
            let value_offset = (i * VALUE_SIZE) as u32;
            match operand {
                ABIOperand::Reg { ty, reg, .. } => self.masm.store(
                    (*reg).into(),
                    self.masm.address_at_reg(self.scratch_reg, value_offset),
                    (*ty).into(),
                ),
                ABIOperand::Stack { ty, offset, .. } => {
                    let addr = match ret_area.unwrap() {
                        RetArea::SP(sp_offset) => {
                            let elem_offs = SPOffset::from_u32(sp_offset.as_u32() - offset);
                            self.masm.address_from_sp(elem_offs)
                        }
                        _ => unreachable!(),
                    };
                    self.masm.load(addr, self.alloc_scratch_reg, (*ty).into());
                    self.masm.store(
                        self.alloc_scratch_reg.into(),
                        self.masm.address_at_reg(self.scratch_reg, value_offset),
                        (*ty).into(),
                    );
                }
            }
        }
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
        let ret_area = self.make_ret_area(&wasm_sig);
        let (offsets, spill_size) = self.spill(native_sig.params());

        let reserved_stack = self.masm.call(wasm_sig.params_stack_size(), |masm| {
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
                &wasm_sig.params_without_retptr(),
                &native_sig.params_without_retptr()[2..],
                &offsets[2..],
                self.scratch_reg,
            );
            Self::load_retptr(masm, ret_area.as_ref(), &wasm_sig);
            CalleeKind::Direct(callee_index.as_u32())
        });

        self.masm.free_stack(reserved_stack);
        self.forward_results(&wasm_sig, &native_sig, ret_area.as_ref(), offsets.last());
        if wasm_sig.has_stack_results() {
            self.masm.free_stack(wasm_sig.results.size());
        }
        self.epilogue_with_callee_saved_restore(spill_size);

        Ok(())
    }

    /// Creates the return area in the caller's frame.
    fn make_ret_area(&mut self, sig: &ABISig) -> Option<RetArea> {
        sig.has_stack_results().then(|| {
            self.masm.reserve_stack(sig.results.size());
            let offs = self.masm.sp_offset();
            RetArea::sp(offs)
        })
    }

    /// Loads the return area pointer into its [ABIOperand] destination.
    fn load_retptr(masm: &mut M, ret_area: Option<&RetArea>, callee: &ABISig) {
        if let Some(area) = ret_area {
            match (area, callee.params.unwrap_results_area_operand()) {
                (RetArea::SP(sp_offset), ABIOperand::Reg { ty, reg, .. }) => {
                    let addr = masm.address_from_sp(*sp_offset);
                    masm.load_addr(addr, *reg, (*ty).into());
                }
                (RetArea::SP(sp_offset), ABIOperand::Stack { ty, offset, .. }) => {
                    let retptr = masm.address_from_sp(*sp_offset);
                    let scratch = <M::ABI as ABI>::scratch_reg();
                    masm.load_addr(retptr, scratch, (*ty).into());
                    let retptr_slot = masm.address_from_sp(SPOffset::from_u32(*offset));
                    masm.store(scratch.into(), retptr_slot, (*ty).into());
                }
                _ => unreachable!(),
            }
        }
    }

    /// Forwards results from callee to caller; it loads results from the
    /// callee's return area and stores them into the caller's return area.
    fn forward_results(
        &mut self,
        callee_sig: &ABISig,
        caller_sig: &ABISig,
        callee_ret_area: Option<&RetArea>,
        caller_retptr_offset: Option<&SPOffset>,
    ) {
        // Spill any result registers used by the callee to avoid
        // use-assign issues when forwarding the results.
        let results_spill = self.spill(callee_sig.results());
        let mut spill_offsets_iter = results_spill.0.iter();

        let caller_retptr = caller_sig.has_stack_results().then(|| {
            let fp = <M::ABI as ABI>::fp_reg();
            let arg_base: u32 = <M::ABI as ABI>::arg_base_offset().into();
            match caller_sig.params.unwrap_results_area_operand() {
                ABIOperand::Reg { ty, .. } => {
                    let addr = self.masm.address_from_sp(*caller_retptr_offset.unwrap());
                    self.masm.load(addr, self.scratch_reg, (*ty).into());
                    self.scratch_reg
                }
                ABIOperand::Stack { ty, offset, .. } => {
                    let addr = self.masm.address_at_reg(fp, arg_base + offset);
                    self.masm.load(addr, self.scratch_reg, (*ty).into());
                    self.scratch_reg
                }
            }
        });

        for (callee_operand, caller_operand) in
            callee_sig.results().iter().zip(caller_sig.results())
        {
            match (callee_operand, caller_operand) {
                (ABIOperand::Reg { ty, .. }, ABIOperand::Stack { offset, .. }) => {
                    let reg_offset = spill_offsets_iter.next().unwrap();
                    self.masm.load(
                        self.masm.address_from_sp(*reg_offset),
                        self.alloc_scratch_reg,
                        (*ty).into(),
                    );
                    self.masm.store(
                        self.alloc_scratch_reg.into(),
                        self.masm.address_at_reg(caller_retptr.unwrap(), *offset),
                        (*ty).into(),
                    );
                }
                (
                    ABIOperand::Stack { ty, offset, .. },
                    ABIOperand::Stack {
                        offset: caller_offset,
                        ..
                    },
                ) => {
                    let addr = {
                        let base = callee_ret_area.unwrap().unwrap_sp();
                        let slot_offset = base.as_u32() - *offset;
                        self.masm.address_from_sp(SPOffset::from_u32(slot_offset))
                    };

                    self.masm.load(addr, self.alloc_scratch_reg, (*ty).into());
                    self.masm.store(
                        self.alloc_scratch_reg.into(),
                        self.masm
                            .address_at_reg(caller_retptr.unwrap(), *caller_offset),
                        (*ty).into(),
                    );
                }
                (ABIOperand::Stack { ty, offset, .. }, ABIOperand::Reg { reg, .. }) => {
                    let addr = {
                        let base = callee_ret_area.unwrap().unwrap_sp();
                        let slot_offset = base.as_u32() - *offset;
                        self.masm.address_from_sp(SPOffset::from_u32(slot_offset))
                    };

                    self.masm.load(addr, *reg, (*ty).into());
                }
                (ABIOperand::Reg { ty, .. }, ABIOperand::Reg { reg: dst, .. }) => {
                    let spill_offset = spill_offsets_iter.next().unwrap();
                    self.masm.load(
                        self.masm.address_from_sp(*spill_offset),
                        (*dst).into(),
                        (*ty).into(),
                    );
                }
            }
        }
        self.masm.free_stack(results_spill.1);
    }

    /// Emit a wasm-to-native trampoline.
    pub fn emit_wasm_to_native(&mut self, ty: &WasmFuncType) -> Result<()> {
        let mut params = self.callee_and_caller_vmctx_types();
        params.extend_from_slice(ty.params());

        let wasm_ty = WasmFuncType::new(params.into_boxed_slice(), ty.returns().into());
        let wasm_sig = self.wasm_sig(&wasm_ty);
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

        let ret_area = self.make_ret_area(&native_sig);
        let (offsets, spill_size) = self.spill(wasm_sig.params());

        let reserved_stack = self.masm.call(native_sig.params_stack_size(), |masm| {
            // Move the VM context into one of the scratch registers.
            masm.mov(
                vmctx.into(),
                self.alloc_scratch_reg.into(),
                OperandSize::S64,
            );

            Self::assign_args(
                masm,
                &native_sig.params_without_retptr(),
                &wasm_sig.params_without_retptr(),
                &offsets,
                self.scratch_reg,
            );

            Self::load_retptr(masm, ret_area.as_ref(), &native_sig);

            let body_offset = self.pointer_size.vmnative_call_host_func_context_func_ref()
                + self.pointer_size.vm_func_ref_native_call();
            let callee_addr = masm.address_at_reg(self.alloc_scratch_reg, body_offset.into());
            masm.load(callee_addr, self.scratch_reg, OperandSize::S64);

            CalleeKind::Indirect(self.scratch_reg)
        });

        self.masm.free_stack(reserved_stack);
        self.forward_results(&native_sig, &wasm_sig, ret_area.as_ref(), offsets.last());

        if native_sig.has_stack_results() {
            self.masm.free_stack(native_sig.results.size());
        }

        self.epilogue(spill_size);

        Ok(())
    }

    /// Perfom argument assignment, translating between
    /// caller and callee calling conventions.
    fn assign_args(
        masm: &mut M,
        callee_params: &[ABIOperand],
        caller_params: &[ABIOperand],
        caller_stack_offsets: &[SPOffset],
        scratch: Reg,
    ) {
        assert!(callee_params.len() == caller_params.len());
        let arg_base_offset: u32 = <M::ABI as ABI>::arg_base_offset().into();
        let fp = <M::ABI as ABI>::fp_reg();
        let mut offset_index = 0;

        callee_params
            .iter()
            .zip(caller_params)
            .for_each(
                |(callee_param, caller_param)| match (callee_param, caller_param) {
                    (ABIOperand::Reg { ty, reg: dst, .. }, ABIOperand::Reg { .. }) => {
                        let offset = caller_stack_offsets[offset_index];
                        let addr = masm.address_from_sp(offset);
                        masm.load(addr, *dst, (*ty).into());
                        offset_index += 1;
                    }

                    (ABIOperand::Stack { ty, offset, .. }, ABIOperand::Reg { .. }) => {
                        let spill_offset = caller_stack_offsets[offset_index];
                        let addr = masm.address_from_sp(spill_offset);
                        masm.load(addr, scratch, (*ty).into());

                        let arg_addr = masm.address_at_sp(SPOffset::from_u32(*offset));
                        masm.store(scratch.into(), arg_addr, (*ty).into());
                        offset_index += 1;
                    }

                    (ABIOperand::Reg { ty, reg: dst, .. }, ABIOperand::Stack { offset, .. }) => {
                        let addr = masm.address_at_reg(fp, arg_base_offset + offset);
                        masm.load(addr, *dst, (*ty).into());
                    }

                    (
                        ABIOperand::Stack {
                            ty,
                            offset: callee_offset,
                            ..
                        },
                        ABIOperand::Stack {
                            offset: caller_offset,
                            ..
                        },
                    ) => {
                        let addr = masm.address_at_reg(fp, arg_base_offset + caller_offset);
                        masm.load(addr, scratch, (*ty).into());

                        let arg_addr = masm.address_at_sp(SPOffset::from_u32(*callee_offset));
                        masm.store(scratch.into(), arg_addr, (*ty).into());
                    }
                },
            );
    }

    /// Get the type of the caller and callee VM contexts.
    fn callee_and_caller_vmctx_types(&self) -> SmallVec<[WasmType; 2]> {
        std::iter::repeat(self.pointer_type).take(2).collect()
    }

    /// Returns an [ABISig] for the array calling convention.
    /// The signature looks like:
    /// ```ignore
    /// unsafe extern "C" fn(
    ///     callee_vmctx: *mut VMOpaqueContext,
    ///     caller_vmctx: *mut VMOpaqueContext,
    ///     values_ptr: *mut ValRaw,
    ///     values_len: usize,
    /// )
    /// ```
    fn array_sig(&self) -> ABISig {
        let mut params = self.callee_and_caller_vmctx_types();
        params.extend_from_slice(&[self.pointer_type, self.pointer_type]);
        <M::ABI as ABI>::sig_from(&params, &[], self.call_conv)
    }

    /// Returns an [ABISig] that follows a variation of the system's
    /// calling convention.
    /// The main difference between the flavor of the returned signature
    /// and the vanilla signature is how multiple values are returned.
    /// Multiple returns are handled following Wasmtime's expectations:
    /// * A single value is returned via a register according to the calling
    ///   convention.
    /// * More than one values are returned via a return pointer.
    /// These variations look like:
    ///
    /// Single return value.
    ///
    /// ```ignore
    /// unsafe extern "C" fn(
    ///     callee_vmctx: *mut VMOpaqueContext,
    ///     caller_vmctx: *mut VMOpaqueContext,
    ///     // rest of paramters
    /// ) -> // single result
    /// ```
    ///
    /// Multiple return values.
    ///
    /// ```ignore
    /// unsafe extern "C" fn(
    ///     callee_vmctx: *mut VMOpaqueContext,
    ///     caller_vmctx: *mut VMOpaqueContext,
    ///     // rest of parameters
    ///     retptr: *mut (), // 2+ results
    /// ) -> // first result
    /// ```
    fn native_sig(&self, ty: &WasmFuncType) -> ABISig {
        let mut params = self.callee_and_caller_vmctx_types();
        params.extend_from_slice(ty.params());
        <M::ABI as ABI>::sig_from(&params, ty.returns(), self.call_conv)
    }

    /// Returns an [ABISig] using the Winch's default calling convention.
    fn wasm_sig(&self, ty: &WasmFuncType) -> ABISig {
        <M::ABI as ABI>::sig(ty, &CallingConvention::Default)
    }

    /// Returns the register pair containing the callee and caller VM context pointers.
    fn callee_and_caller_vmctx(params: &ABIParams) -> Result<(Reg, Reg)> {
        let vmctx = params
            .get(0)
            .map(|operand| operand.unwrap_reg())
            .expect("Callee VMContext to be in a register");
        let caller_vmctx = params
            .get(1)
            .map(|operand| operand.unwrap_reg())
            .expect("Caller VMContext to be in a register");
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

    /// Performs a spill of the given operands.
    fn spill(&mut self, operands: &[ABIOperand]) -> (SmallVec<[SPOffset; 6]>, u32) {
        let mut offsets = SmallVec::new();
        let mut spill_size = 0;
        operands.iter().for_each(|param| {
            if let Some(reg) = param.get_reg() {
                let slot = self.masm.push(reg, param.ty().into());
                offsets.push(slot.offset);
                spill_size += slot.size;
            }
        });

        (offsets, spill_size)
    }

    /// Loads and assigns values from the value array used in the array
    /// calling convention.
    fn load_values_from_array(
        masm: &mut M,
        callee_sig: &ABISig,
        ret_area: Option<&RetArea>,
        values_reg: Reg,
        scratch: Reg,
    ) {
        callee_sig
            .params_without_retptr()
            .iter()
            .enumerate()
            .for_each(|(i, param)| {
                let value_offset = (i * VALUE_SIZE) as u32;

                match param {
                    ABIOperand::Reg { reg, ty, .. } => masm.load(
                        masm.address_at_reg(values_reg, value_offset),
                        *reg,
                        (*ty).into(),
                    ),
                    ABIOperand::Stack { offset, ty, .. } => {
                        masm.load(
                            masm.address_at_reg(values_reg, value_offset),
                            scratch,
                            (*ty).into(),
                        );
                        masm.store(
                            scratch.into(),
                            masm.address_at_sp(SPOffset::from_u32(*offset)),
                            (*ty).into(),
                        );
                    }
                }
            });

        // Assign the retpr param.
        if let Some(offs) = ret_area {
            let results_area_operand = callee_sig.params.unwrap_results_area_operand();
            let addr = match offs {
                RetArea::SP(sp_offset) => masm.address_from_sp(*sp_offset),
                _ => unreachable!(),
            };
            match results_area_operand {
                ABIOperand::Reg { ty, reg, .. } => {
                    masm.load_addr(addr, (*reg).into(), (*ty).into());
                }
                ABIOperand::Stack { ty, offset, .. } => {
                    masm.load_addr(addr, scratch, (*ty).into());
                    masm.store(
                        scratch.into(),
                        masm.address_at_sp(SPOffset::from_u32(*offset)),
                        (*ty).into(),
                    );
                }
            }
        }
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
        let mut off = 0;
        for (r, s) in &self.callee_saved_regs {
            let slot = self.masm.save(off, *r, *s);
            off += slot.size;
        }
    }

    /// Similar to [Trampoline::epilogue], but restores
    /// callee-saved registers.
    fn epilogue_with_callee_saved_restore(&mut self, arg_size: u32) {
        // Free the stack space allocated by pushing the trampoline arguments.
        self.masm.free_stack(arg_size);
        // Restore the callee-saved registers.
        for (r, s) in self.callee_saved_regs.iter().rev() {
            self.masm.pop(*r, *s);
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
