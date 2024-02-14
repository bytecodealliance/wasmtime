use crate::{
    abi::{ABIOperand, ABISig, RetArea, ABI},
    codegen::BlockSig,
    isa::reg::Reg,
    masm::{ExtendKind, IntCmpKind, MacroAssembler, OperandSize, RegImm, SPOffset, TrapCode},
    stack::TypedReg,
};
use anyhow::Result;
use smallvec::SmallVec;
use wasmparser::{
    BinaryReader, FuncValidator, MemArg, Operator, ValidatorResources, VisitOperator,
};
use wasmtime_environ::{
    MemoryIndex, PtrSize, TableIndex, TypeIndex, WasmHeapType, WasmValType, FUNCREF_MASK,
};

mod context;
pub(crate) use context::*;
mod env;
pub use env::*;
mod call;
pub(crate) use call::*;
mod control;
pub(crate) use control::*;
mod builtin;
pub use builtin::*;
pub(crate) mod bounds;

use bounds::{Bounds, ImmOffset, Index};

/// The code generation abstraction.
pub(crate) struct CodeGen<'a, 'translation: 'a, 'data: 'translation, M>
where
    M: MacroAssembler,
{
    /// The ABI-specific representation of the function signature, excluding results.
    pub sig: ABISig,

    /// The code generation context.
    pub context: CodeGenContext<'a>,

    /// A reference to the function compilation environment.
    pub env: FuncEnv<'a, 'translation, 'data, M::Ptr>,

    /// The MacroAssembler.
    pub masm: &'a mut M,

    /// Stack frames for control flow.
    // NB The 64 is set arbitrarily, we can adjust it as
    // we see fit.
    pub control_frames: SmallVec<[ControlStackFrame; 64]>,
}

impl<'a, 'translation, 'data, M> CodeGen<'a, 'translation, 'data, M>
where
    M: MacroAssembler,
{
    pub fn new(
        masm: &'a mut M,
        context: CodeGenContext<'a>,
        env: FuncEnv<'a, 'translation, 'data, M::Ptr>,
        sig: ABISig,
    ) -> Self {
        Self {
            sig,
            context,
            masm,
            env,
            control_frames: Default::default(),
        }
    }

    /// Emit the function body to machine code.
    pub fn emit(
        &mut self,
        body: &mut BinaryReader<'a>,
        validator: &mut FuncValidator<ValidatorResources>,
    ) -> Result<()> {
        self.emit_start()
            .and_then(|_| self.emit_body(body, validator))
            .and_then(|_| self.emit_end())?;

        Ok(())
    }

    fn emit_start(&mut self) -> Result<()> {
        self.masm.prologue();
        self.masm.reserve_stack(self.context.frame.locals_size);

        // Check for stack overflow after reserving space, so that we get the most up-to-date view
        // of the stack pointer. This assumes that no writes to the stack occur in `reserve_stack`.
        self.masm.check_stack();

        // We don't have any callee save registers in the winch calling convention, but
        // `save_clobbers` does some useful work for setting up unwinding state.
        self.masm.save_clobbers(&[]);

        // Once we have emitted the epilogue and reserved stack space for the locals, we push the
        // base control flow block.
        self.control_frames.push(ControlStackFrame::block(
            BlockSig::from_sig(self.sig.clone()),
            self.masm,
            &mut self.context,
        ));

        // Set the return area of the results *after* initializing the block. In
        // the function body block case, we'll treat the results as any other
        // case, addressed from the stack pointer, and when ending the function
        // the return area will be set to the return pointer.
        if self.sig.params.has_retptr() {
            self.sig
                .results
                .set_ret_area(RetArea::slot(self.context.frame.results_base_slot.unwrap()));
        }

        Ok(())
    }

    /// The following two helpers, handle else or end instructions when the
    /// compiler has entered into an unreachable code state. These instructions
    /// must be observed to determine if the reachability state should be
    /// restored.
    ///
    /// When the compiler is in an unreachable state, all the other instructions
    /// are not visited.
    pub fn handle_unreachable_else(&mut self) {
        let frame = self.control_frames.last_mut().unwrap();
        debug_assert!(frame.is_if());
        if frame.is_next_sequence_reachable() {
            // We entered an unreachable state when compiling the
            // if-then branch, but if the `if` was reachable at
            // entry, the if-else branch will be reachable.
            self.context.reachable = true;
            frame.ensure_stack_state(self.masm, &mut self.context);
            frame.bind_else(self.masm, &mut self.context);
        }
    }

    pub fn handle_unreachable_end(&mut self) {
        let mut frame = self.control_frames.pop().unwrap();
        // We just popped the outermost block.
        let is_outermost = self.control_frames.len() == 0;

        if frame.is_next_sequence_reachable() {
            self.context.reachable = true;
            frame.ensure_stack_state(self.masm, &mut self.context);
            frame.bind_end(self.masm, &mut self.context);
        } else if is_outermost {
            // If we reach the end of the function in an unreachable
            // state, perform the necessary cleanup to leave the stack
            // and SP in the expected state.  The compiler can enter
            // in this state through an infinite loop.
            frame.ensure_stack_state(self.masm, &mut self.context);
        }
    }

    fn emit_body(
        &mut self,
        body: &mut BinaryReader<'a>,
        validator: &mut FuncValidator<ValidatorResources>,
    ) -> Result<()> {
        self.spill_register_arguments();
        let defined_locals_range = &self.context.frame.defined_locals_range;
        self.masm.zero_mem_range(defined_locals_range.as_range());

        // Save the vmctx pointer to its local slot in case we need to reload it
        // at any point.
        let vmctx_addr = self.masm.local_address(&self.context.frame.vmctx_slot);
        self.masm
            .store_ptr(<M::ABI as ABI>::vmctx_reg().into(), vmctx_addr);

        // Save the results base parameter register into its slot.
        self.sig.params.has_retptr().then(|| {
            match self.sig.params.unwrap_results_area_operand() {
                ABIOperand::Reg { ty, reg, .. } => {
                    let results_base_slot = self.context.frame.results_base_slot.as_ref().unwrap();
                    debug_assert!(results_base_slot.addressed_from_sp());
                    let addr = self.masm.local_address(results_base_slot);
                    self.masm.store((*reg).into(), addr, (*ty).into());
                }
                // The result base parameter is a stack paramter, addressed
                // from FP.
                _ => {}
            }
        });

        while !body.eof() {
            let offset = body.original_position();
            body.visit_operator(&mut ValidateThenVisit(validator.visitor(offset), self))??;
        }
        validator.finish(body.original_position())?;
        return Ok(());

        struct ValidateThenVisit<'a, T, U>(T, &'a mut U);

        macro_rules! validate_then_visit {
            ($( @$proposal:ident $op:ident $({ $($arg:ident: $argty:ty),* })? => $visit:ident)*) => {
                $(
                    fn $visit(&mut self $($(,$arg: $argty)*)?) -> Self::Output {
                        self.0.$visit($($($arg.clone()),*)?)?;
                        // Only visit operators if the compiler is in a reachable code state. If
                        // the compiler is in an unrechable code state, most of the operators are
                        // ignored except for If, Block, Loop, Else and End. These operators need
                        // to be observed in order to keep the control stack frames balanced and to
                        // determine if reachability should be restored.
                        let visit_when_unreachable = visit_op_when_unreachable(Operator::$op $({ $($arg: $arg.clone()),* })?);
                        if self.1.is_reachable() || visit_when_unreachable  {
                            Ok(self.1.$visit($($($arg),*)?))
                        } else {
                            Ok(U::Output::default())
                        }
                    }
                )*
            };
        }

        fn visit_op_when_unreachable(op: Operator) -> bool {
            use Operator::*;
            match op {
                If { .. } | Block { .. } | Loop { .. } | Else | End => true,
                _ => false,
            }
        }

        /// Trait to handle reachability state.
        trait ReachableState {
            /// Returns true if the current state of the program is reachable.
            fn is_reachable(&self) -> bool;
        }

        impl<'a, 'translation, 'data, M: MacroAssembler> ReachableState
            for CodeGen<'a, 'translation, 'data, M>
        {
            fn is_reachable(&self) -> bool {
                self.context.reachable
            }
        }

        impl<'a, T, U> VisitOperator<'a> for ValidateThenVisit<'_, T, U>
        where
            T: VisitOperator<'a, Output = wasmparser::Result<()>>,
            U: VisitOperator<'a> + ReachableState,
            U::Output: Default,
        {
            type Output = Result<U::Output>;

            wasmparser::for_each_operator!(validate_then_visit);
        }
    }

    /// Emits a a series of instructions that will type check a function reference call.
    pub fn emit_typecheck_funcref(&mut self, funcref_ptr: Reg, type_index: TypeIndex) {
        let ptr_size: OperandSize = self.env.ptr_type().into();
        let sig_index_bytes = self.env.vmoffsets.size_of_vmshared_type_index();
        let sig_size = OperandSize::from_bytes(sig_index_bytes);
        let sig_index = self.env.translation.module.types[type_index].unwrap_function();
        let sig_offset = sig_index
            .as_u32()
            .checked_mul(sig_index_bytes.into())
            .unwrap();
        let signatures_base_offset = self.env.vmoffsets.vmctx_type_ids_array();
        let scratch = <M::ABI as ABI>::scratch_reg();
        let funcref_sig_offset = self.env.vmoffsets.ptr.vm_func_ref_type_index();

        // Load the signatures address into the scratch register.
        self.masm.load(
            self.masm.address_at_vmctx(signatures_base_offset),
            scratch,
            ptr_size,
        );

        // Get the caller id.
        let caller_id = self.context.any_gpr(self.masm);
        self.masm.load(
            self.masm.address_at_reg(scratch, sig_offset),
            caller_id,
            sig_size,
        );

        let callee_id = self.context.any_gpr(self.masm);
        self.masm.load(
            self.masm
                .address_at_reg(funcref_ptr, funcref_sig_offset.into()),
            callee_id,
            sig_size,
        );

        // Typecheck.
        self.masm.cmp(callee_id.into(), caller_id, OperandSize::S32);
        self.masm.trapif(IntCmpKind::Ne, TrapCode::BadSignature);
        self.context.free_reg(callee_id);
        self.context.free_reg(caller_id);
    }

    /// Emit the usual function end instruction sequence.
    fn emit_end(&mut self) -> Result<()> {
        // The implicit body block is treated a normal block (it pushes results
        // to the stack); so when reaching the end, we pop them taking as
        // reference the current function's signature.
        let base = SPOffset::from_u32(self.context.frame.locals_size);
        if self.context.reachable {
            ControlStackFrame::pop_abi_results_impl(
                &mut self.sig.results,
                &mut self.context,
                self.masm,
                |results, _, _| results.ret_area().copied(),
            );
        } else {
            // If we reach the end of the function in a unreachable code state,
            // simly truncate to the the expected values.
            // The compiler could enter in this state through an infinite loop.
            self.context.truncate_stack_to(0);
            self.masm.reset_stack_pointer(base);
        }
        debug_assert_eq!(self.context.stack.len(), 0);
        self.masm.restore_clobbers(&[]);
        self.masm.epilogue(self.context.frame.locals_size);
        Ok(())
    }

    fn spill_register_arguments(&mut self) {
        use WasmValType::*;
        self.sig
            // Skip the results base param if any; [Self::emit_body],
            // will handle spilling the results base param if it's in a register.
            .params_without_retptr()
            .iter()
            .enumerate()
            .filter(|(_, a)| a.is_reg())
            .for_each(|(index, arg)| {
                let ty = arg.ty();
                let local = self
                    .context
                    .frame
                    .get_local(index as u32)
                    .expect("valid local slot at location");
                let addr = self.masm.local_address(local);
                let src = arg
                    .get_reg()
                    .expect("arg should be associated to a register");

                match &ty {
                    I32 | I64 | F32 | F64 => self.masm.store(src.into(), addr, ty.into()),
                    Ref(rt) => match rt.heap_type {
                        WasmHeapType::Func => self.masm.store_ptr(src.into(), addr),
                        ht => unimplemented!("Support for WasmHeapType: {ht}"),
                    },
                    _ => unimplemented!("Support for WasmType {ty}"),
                }
            });
    }

    /// Pops the value at the stack top and assigns it to the local at
    /// the given index, returning the typed register holding the
    /// source value.
    pub fn emit_set_local(&mut self, index: u32) -> TypedReg {
        // Materialize any references to the same local index that are in the
        // value stack by spilling.
        if self.context.stack.contains_latent_local(index) {
            self.context.spill(self.masm);
        }
        let src = self.context.pop_to_reg(self.masm, None);
        // Need to get address of local after `pop_to_reg` since `pop_to_reg`
        // will pop the machine stack causing an incorrect address to be
        // calculated.
        let (ty, addr) = self.context.frame.get_local_address(index, self.masm);
        self.masm.store(RegImm::reg(src.reg), addr, ty.into());

        src
    }

    pub fn emit_lazy_init_funcref(&mut self, table_index: TableIndex) {
        let table_data = self.env.resolve_table_data(table_index);
        let ptr_type = self.env.ptr_type();
        let builtin = self
            .env
            .builtins
            .table_get_lazy_init_func_ref::<M::ABI, M::Ptr>();

        // Request the builtin's  result register and use it to hold the
        // table element value. We preemptively request this register to
        // avoid conflict at the control flow merge below.
        // Requesting the result register is safe since we know ahead-of-time
        // the builtin's signature.
        let elem_value: Reg = self
            .context
            .reg(
                builtin.sig().results.unwrap_singleton().unwrap_reg(),
                self.masm,
            )
            .into();

        let index = self.context.pop_to_reg(self.masm, None);
        let base = self.context.any_gpr(self.masm);

        let elem_addr =
            self.masm
                .table_elem_address(index.into(), base, &table_data, &mut self.context);
        self.masm.load_ptr(elem_addr, elem_value);
        // Free the register used as base, once we have loaded the element
        // address into the element value register.
        self.context.free_reg(base);

        let (defined, cont) = (self.masm.get_label(), self.masm.get_label());

        // Push the built-in arguments to the stack.
        self.context.stack.extend([
            TypedReg::new(ptr_type, <M::ABI as ABI>::vmctx_reg()).into(),
            table_index.as_u32().try_into().unwrap(),
            index.into(),
        ]);

        self.masm.branch(
            IntCmpKind::Ne,
            elem_value.into(),
            elem_value,
            defined,
            ptr_type.into(),
        );
        // Free the element value register.
        // This is safe since the FnCall::emit call below, will ensure
        // that the result register is placed on the value stack.
        self.context.free_reg(elem_value);
        FnCall::emit::<M, M::Ptr>(
            self.masm,
            &mut self.context,
            Callee::Builtin(builtin.clone()),
        );

        // We know the signature of the libcall in this case, so we assert that there's
        // one element in the stack and that it's  the ABI signature's result register.
        let top = self.context.stack.peek().unwrap();
        let top = top.unwrap_reg();
        debug_assert!(top.reg == elem_value);
        self.masm.jmp(cont);

        // In the defined case, mask the funcref address in place, by peeking into the
        // last element of the value stack, which was pushed by the `indirect` function
        // call above.
        self.masm.bind(defined);
        let imm = RegImm::i64(FUNCREF_MASK as i64);
        let dst = top.into();
        self.masm.and(dst, dst, imm, top.ty.into());

        self.masm.bind(cont);
    }

    /// Emits a series of instructions to bounds check and calculate the address
    /// of the given WebAssembly memory.
    /// This function returns a register containing the requested address.
    ///
    /// In essence, when computing the heap address for a WebAssembly load or
    /// store instruction the objective is to ensure that such access is safe,
    /// but also to perform the least amount of checks, and rely on the system to
    /// detect illegal memory accesses where applicable.
    ///
    /// Winch follows almost the same principles as Cranelift when it comes to
    /// bounds checks, for a more detailed explanation refer to
    /// [cranelift_wasm::code_translator::prepare_addr].
    ///
    /// Winch implementation differs in that, it defaults to the general case
    /// for dynamic heaps rather than optimizing for doing the least amount of
    /// work possible at runtime, this is done to align with Winch's principle
    /// of doing the least amount of work possible at compile time. For static
    /// heaps, Winch does a bit more of work, given that some of the cases that
    /// are checked against, can benefit compilation times, like for example,
    /// detecting an out of bouds access at compile time.
    pub fn emit_compute_heap_address(
        &mut self,
        memarg: &MemArg,
        access_size: OperandSize,
    ) -> Option<Reg> {
        let ptr_size: OperandSize = self.env.ptr_type().into();
        let enable_spectre_mitigation = self.env.heap_access_spectre_mitigation();
        let add_offset_and_access_size = |offset: ImmOffset, access_size: OperandSize| {
            (access_size.bytes() as u64) + (offset.as_u32() as u64)
        };

        let memory_index = MemoryIndex::from_u32(memarg.memory);
        let heap = self.env.resolve_heap(memory_index);
        let index = Index::from_typed_reg(self.context.pop_to_reg(self.masm, None));
        let offset = bounds::ensure_index_and_offset(self.masm, index, memarg.offset, ptr_size);
        let offset_with_access_size = add_offset_and_access_size(offset, access_size);

        let addr = match heap.style {
            // == Dynamic Heaps ==

            // Account for the general case for dynamic memories. The access is
            // out of bounds if:
            // * index + offset + access_size overflows
            //   OR
            // * index + offset + access_size > bound
            HeapStyle::Dynamic => {
                let bounds =
                    bounds::load_dynamic_heap_bounds(&mut self.context, self.masm, &heap, ptr_size);

                let index_reg = index.as_typed_reg().reg;
                // Perform
                // index = index + offset + access_size, trapping if the
                // addition overflows.
                self.masm.checked_uadd(
                    index_reg,
                    index_reg,
                    RegImm::i64(offset_with_access_size as i64),
                    ptr_size,
                    TrapCode::HeapOutOfBounds,
                );

                let addr = bounds::load_heap_addr_checked(
                    self.masm,
                    &mut self.context,
                    ptr_size,
                    &heap,
                    enable_spectre_mitigation,
                    bounds,
                    index,
                    offset,
                    |masm, bounds, index| {
                        let index_reg = index.as_typed_reg().reg;
                        let bounds_reg = bounds.as_typed_reg().reg;
                        masm.cmp(bounds_reg.into(), index_reg.into(), heap.ty.into());
                        IntCmpKind::GtU
                    },
                );
                self.context.free_reg(bounds.as_typed_reg().reg);
                Some(addr)
            }

            // == Static Heaps ==

            // Detect at compile time if the access is out of bounds.
            // Doing so will put the compiler in an unreachable code state,
            // optimizing the work that the compiler has to do until the
            // reachability is restored or when reaching the end of the
            // function.
            HeapStyle::Static { bound } if offset_with_access_size > bound => {
                self.masm.trap(TrapCode::HeapOutOfBounds);
                self.context.reachable = false;
                None
            }

            // Account for the case in which we can completely elide the bounds
            // checks.
            //
            // This case, makes use of the fact that if a memory access uses
            // a 32-bit index, then we be certain that
            //
            //      index <= u32::MAX
            //
            // Therfore if any 32-bit index access occurs in the region
            // represented by
            //
            //      bound + guard_size - (offset + access_size)
            //
            // We are certain that it's in bounds or that the underlying virtual
            // memory subsystem will report an illegal access at runtime.
            //
            // Note:
            //
            // * bound - (offset + access_size) cannot wrap, because it's checked
            // in the condition above.
            // * bound + heap.offset_guard_size is guaranteed to not overflow if
            // the heap configuration is correct, given that it's address must
            // fit in 64-bits.
            // * If the heap type is 32-bits, the offset is at most u32::MAX, so
            // no  adjustment is needed as part of
            // [bounds::ensure_index_and_offset].
            HeapStyle::Static { bound }
                if heap.ty == WasmValType::I32
                    && u64::from(u32::MAX)
                        <= u64::from(bound) + u64::from(heap.offset_guard_size)
                            - offset_with_access_size =>
            {
                let addr = self.context.any_gpr(self.masm);
                bounds::load_heap_addr_unchecked(self.masm, &heap, index, offset, addr, ptr_size);
                Some(addr)
            }

            // Account for the general case of static memories. The access is out
            // of bounds if:
            //
            // index > bound - (offset + access_size)
            //
            // bound - (offset + access_size) cannot wrap, because we already
            // checked that (offset + access_size) > bound, above.
            HeapStyle::Static { bound } => {
                let bounds = Bounds::from_u64(bound);
                let addr = bounds::load_heap_addr_checked(
                    self.masm,
                    &mut self.context,
                    ptr_size,
                    &heap,
                    enable_spectre_mitigation,
                    bounds,
                    index,
                    offset,
                    |masm, bounds, index| {
                        let adjusted_bounds = bounds.as_u64() - offset_with_access_size;
                        let index_reg = index.as_typed_reg().reg;
                        masm.cmp(RegImm::i64(adjusted_bounds as i64), index_reg, ptr_size);
                        IntCmpKind::GtU
                    },
                );
                Some(addr)
            }
        };

        self.context.free_reg(index.as_typed_reg().reg);
        addr
    }

    /// Emit a WebAssembly load.
    pub fn emit_wasm_load(
        &mut self,
        arg: &MemArg,
        ty: WasmValType,
        size: OperandSize,
        sextend: Option<ExtendKind>,
    ) {
        if let Some(addr) = self.emit_compute_heap_address(&arg, size) {
            let dst = match ty {
                WasmValType::I32 | WasmValType::I64 => self.context.any_gpr(self.masm),
                WasmValType::F32 | WasmValType::F64 => self.context.any_fpr(self.masm),
                _ => unreachable!(),
            };

            let src = self.masm.address_at_reg(addr, 0);
            self.masm.wasm_load(src, dst, size, sextend);
            self.context.stack.push(TypedReg::new(ty, dst).into());
            self.context.free_reg(addr);
        }
    }

    /// Emit a WebAssembly store.
    pub fn emit_wasm_store(&mut self, arg: &MemArg, size: OperandSize) {
        let src = self.context.pop_to_reg(self.masm, None);
        if let Some(addr) = self.emit_compute_heap_address(&arg, size) {
            self.masm
                .wasm_store(src.reg.into(), self.masm.address_at_reg(addr, 0), size);

            self.context.free_reg(addr);
            self.context.free_reg(src);
        }
    }
}

/// Returns the index of the [`ControlStackFrame`] for the given
/// depth.
pub fn control_index(depth: u32, control_length: usize) -> usize {
    (control_length - 1)
        .checked_sub(depth as usize)
        .unwrap_or_else(|| panic!("expected valid control stack frame at index: {}", depth))
}
