use crate::{
    abi::{scratch, vmctx, ABIOperand, ABISig, RetArea},
    codegen::BlockSig,
    isa::reg::{writable, Reg},
    masm::{
        ExtendKind, IntCmpKind, MacroAssembler, OperandSize, RegImm, SPOffset, ShiftKind, TrapCode,
    },
    stack::TypedReg,
};
use anyhow::Result;
use smallvec::SmallVec;
use wasmparser::{
    BinaryReader, FuncValidator, MemArg, Operator, ValidatorResources, VisitOperator,
};
use wasmtime_environ::{
    GlobalIndex, MemoryIndex, PtrSize, TableIndex, Tunables, TypeIndex, WasmHeapType, WasmValType,
    FUNCREF_MASK,
};

use cranelift_codegen::{
    binemit::CodeOffset,
    ir::{RelSourceLoc, SourceLoc},
};
use wasmtime_cranelift::{TRAP_BAD_SIGNATURE, TRAP_TABLE_OUT_OF_BOUNDS};

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

/// Holds metadata about the source code location and the machine code emission.
/// The fields of this struct are opaque and are not interpreted in any way.
/// They serve as a mapping between source code and machine code.
#[derive(Default)]
pub(crate) struct SourceLocation {
    /// The base source location.
    pub base: Option<SourceLoc>,
    /// The current relative source code location along with its associated
    /// machine code offset.
    pub current: (CodeOffset, RelSourceLoc),
}

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

    /// Information about the source code location.
    pub source_location: SourceLocation,

    /// Flag indicating whether during translation an unsupported instruction
    /// was found.
    pub found_unsupported_instruction: Option<&'static str>,

    /// Compilation settings for code generation.
    pub tunables: &'a Tunables,

    /// Local counter to track fuel consumption.
    pub fuel_consumed: i64,
}

impl<'a, 'translation, 'data, M> CodeGen<'a, 'translation, 'data, M>
where
    M: MacroAssembler,
{
    pub fn new(
        tunables: &'a Tunables,
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
            tunables,
            source_location: Default::default(),
            control_frames: Default::default(),
            found_unsupported_instruction: None,
            // Empty functions should consume at least 1 fuel unit.
            fuel_consumed: 1,
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

    /// Derives a [RelSourceLoc] from a [SourceLoc].
    pub fn source_loc_from(&mut self, loc: SourceLoc) -> RelSourceLoc {
        if self.source_location.base.is_none() && !loc.is_default() {
            self.source_location.base = Some(loc);
        }

        RelSourceLoc::from_base_offset(self.source_location.base.unwrap_or_default(), loc)
    }

    fn emit_start(&mut self) -> Result<()> {
        let vmctx = self
            .sig
            .params()
            .first()
            .expect("VMContext argument")
            .unwrap_reg()
            .into();

        self.masm.start_source_loc(Default::default());
        // We need to use the vmctx parameter before pinning it for stack checking.
        self.masm.prologue(vmctx);

        // Pin the `VMContext` pointer.
        self.masm.mov(
            writable!(vmctx!(M)),
            vmctx.into(),
            self.env.ptr_type().into(),
        );

        self.masm.reserve_stack(self.context.frame.locals_size);

        self.masm.end_source_loc();

        if self.tunables.consume_fuel {
            self.emit_fuel_check();
        }

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

        // Save the results base parameter register into its slot.
        self.sig.params.has_retptr().then(|| {
            match self.sig.params.unwrap_results_area_operand() {
                ABIOperand::Reg { ty, reg, .. } => {
                    let results_base_slot = self.context.frame.results_base_slot.as_ref().unwrap();
                    debug_assert!(results_base_slot.addressed_from_sp());
                    let addr = self.masm.local_address(results_base_slot);
                    self.masm.store((*reg).into(), addr, (*ty).into());
                }
                // The result base parameter is a stack parameter, addressed
                // from FP.
                _ => {}
            }
        });

        while !body.eof() {
            let offset = body.original_position();
            body.visit_operator(&mut ValidateThenVisit(
                validator.visitor(offset),
                self,
                offset,
            ))??;

            if let Some(insn) = self.found_unsupported_instruction {
                anyhow::bail!("unsupported instruction in Winch: {insn}")
            }
        }
        validator.finish(body.original_position())?;
        return Ok(());

        struct ValidateThenVisit<'a, T, U>(T, &'a mut U, usize);

        macro_rules! validate_then_visit {
            ($( @$proposal:ident $op:ident $({ $($arg:ident: $argty:ty),* })? => $visit:ident $ann:tt)*) => {
                $(
                    fn $visit(&mut self $($(,$arg: $argty)*)?) -> Self::Output {
                        self.0.$visit($($($arg.clone()),*)?)?;
                        let op = Operator::$op $({ $($arg: $arg.clone()),* })?;
                        if self.1.visit(&op) {
                            self.1.before_visit_op(&op, self.2);
                            let res = Ok(self.1.$visit($($($arg),*)?));
                            self.1.after_visit_op();
                            res
                        } else {
                            Ok(U::Output::default())
                        }
                    }
                )*
            };
        }

        fn visit_op_when_unreachable(op: &Operator) -> bool {
            use Operator::*;
            match op {
                If { .. } | Block { .. } | Loop { .. } | Else | End => true,
                _ => false,
            }
        }

        /// Trait to handle hooks that must happen before and after visiting an
        /// operator.
        trait VisitorHooks {
            /// Hook prior to visiting an operator.
            fn before_visit_op(&mut self, operator: &Operator, offset: usize);
            /// Hook after visiting an operator.
            fn after_visit_op(&mut self);

            /// Returns `true` if the operator will be visited.
            ///
            /// Operators will be visited if the following invariants are met:
            /// * The compiler is in a reachable state.
            /// * The compiler is in an unreachable state, but the current
            ///   operator is a control flow operator. These operators need to be
            ///   visited in order to keep the control stack frames balanced and
            ///   to determine if the reachability state must be restored.
            fn visit(&self, op: &Operator) -> bool;
        }

        impl<'a, 'translation, 'data, M: MacroAssembler> VisitorHooks
            for CodeGen<'a, 'translation, 'data, M>
        {
            fn visit(&self, op: &Operator) -> bool {
                self.context.reachable || visit_op_when_unreachable(op)
            }

            fn before_visit_op(&mut self, operator: &Operator, offset: usize) {
                // Handle source location mapping.
                self.source_location_before_visit_op(offset);

                // Handle fuel.
                if self.tunables.consume_fuel {
                    self.fuel_before_visit_op(operator);
                }
            }

            fn after_visit_op(&mut self) {
                // Handle source code location mapping.
                self.source_location_after_visit_op();
            }
        }

        impl<'a, T, U> VisitOperator<'a> for ValidateThenVisit<'_, T, U>
        where
            T: VisitOperator<'a, Output = wasmparser::Result<()>>,
            U: VisitOperator<'a> + VisitorHooks,
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
        let sig_index = self.env.translation.module.types[type_index];
        let sig_offset = sig_index
            .as_u32()
            .checked_mul(sig_index_bytes.into())
            .unwrap();
        let signatures_base_offset = self.env.vmoffsets.ptr.vmctx_type_ids_array();
        let scratch = scratch!(M);
        let funcref_sig_offset = self.env.vmoffsets.ptr.vm_func_ref_type_index();

        // Load the signatures address into the scratch register.
        self.masm.load(
            self.masm.address_at_vmctx(signatures_base_offset.into()),
            writable!(scratch),
            ptr_size,
        );

        // Get the caller id.
        let caller_id = self.context.any_gpr(self.masm);
        self.masm.load(
            self.masm.address_at_reg(scratch, sig_offset),
            writable!(caller_id),
            sig_size,
        );

        let callee_id = self.context.any_gpr(self.masm);
        self.masm.load(
            self.masm
                .address_at_reg(funcref_ptr, funcref_sig_offset.into()),
            writable!(callee_id),
            sig_size,
        );

        // Typecheck.
        self.masm.cmp(caller_id, callee_id.into(), OperandSize::S32);
        self.masm.trapif(IntCmpKind::Ne, TRAP_BAD_SIGNATURE);
        self.context.free_reg(callee_id);
        self.context.free_reg(caller_id);
    }

    /// Emit the usual function end instruction sequence.
    fn emit_end(&mut self) -> Result<()> {
        // The implicit body block is treated a normal block (it pushes results
        // to the stack); so when reaching the end, we pop them taking as
        // reference the current function's signature.
        let base = SPOffset::from_u32(self.context.frame.locals_size);
        self.masm.start_source_loc(Default::default());
        if self.context.reachable {
            ControlStackFrame::pop_abi_results_impl(
                &mut self.sig.results,
                &mut self.context,
                self.masm,
                |results, _, _| results.ret_area().copied(),
            );
        } else {
            // If we reach the end of the function in an unreachable code state,
            // simply truncate to the expected values.
            // The compiler could enter this state through an infinite loop.
            self.context.truncate_stack_to(0);
            self.masm.reset_stack_pointer(base);
        }
        debug_assert_eq!(self.context.stack.len(), 0);
        self.masm.free_stack(self.context.frame.locals_size);
        self.masm.epilogue();
        self.masm.end_source_loc();
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
                let local = self.context.frame.get_frame_local(index);
                let addr = self.masm.local_address(local);
                let src = arg
                    .get_reg()
                    .expect("arg should be associated to a register");

                match &ty {
                    I32 | I64 | F32 | F64 | V128 => self.masm.store(src.into(), addr, ty.into()),
                    Ref(rt) => match rt.heap_type {
                        WasmHeapType::Func | WasmHeapType::Extern => {
                            self.masm.store_ptr(src.into(), addr)
                        }
                        ht => unimplemented!("Support for WasmHeapType: {ht}"),
                    },
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

    /// Loads the address of the given global.
    pub fn emit_get_global_addr(&mut self, index: GlobalIndex) -> (WasmValType, M::Address) {
        let data = self.env.resolve_global(index);

        let addr = if data.imported {
            let global_base = self.masm.address_at_reg(vmctx!(M), data.offset);
            let scratch = scratch!(M);
            self.masm.load_ptr(global_base, writable!(scratch));
            self.masm.address_at_reg(scratch, 0)
        } else {
            self.masm.address_at_reg(vmctx!(M), data.offset)
        };

        (data.ty, addr)
    }

    pub fn emit_lazy_init_funcref(&mut self, table_index: TableIndex) {
        assert!(self.tunables.table_lazy_init, "unsupported eager init");
        let table_data = self.env.resolve_table_data(table_index);
        let ptr_type = self.env.ptr_type();
        let builtin = self
            .env
            .builtins
            .table_get_lazy_init_func_ref::<M::ABI, M::Ptr>();

        // Request the builtin's  result register and use it to hold the table
        // element value. We preemptively spill and request this register to
        // avoid conflict at the control flow merge below. Requesting the result
        // register is safe since we know ahead-of-time the builtin's signature.
        self.context.spill(self.masm);
        let elem_value: Reg = self
            .context
            .reg(
                builtin.sig().results.unwrap_singleton().unwrap_reg(),
                self.masm,
            )
            .into();

        let index = self.context.pop_to_reg(self.masm, None);
        let base = self.context.any_gpr(self.masm);

        let elem_addr = self.emit_compute_table_elem_addr(index.into(), base, &table_data);
        self.masm.load_ptr(elem_addr, writable!(elem_value));
        // Free the register used as base, once we have loaded the element
        // address into the element value register.
        self.context.free_reg(base);

        let (defined, cont) = (self.masm.get_label(), self.masm.get_label());

        // Push the built-in arguments to the stack.
        self.context
            .stack
            .extend([table_index.as_u32().try_into().unwrap(), index.into()]);

        self.masm.branch(
            IntCmpKind::Ne,
            elem_value,
            elem_value.into(),
            defined,
            ptr_type.into(),
        );
        // Free the element value register.
        // This is safe since the FnCall::emit call below, will ensure
        // that the result register is placed on the value stack.
        self.context.free_reg(elem_value);
        FnCall::emit::<M>(
            &mut self.env,
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
        self.masm.and(writable!(dst), dst, imm, top.ty.into());

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
    /// prepare_addr in wasmtime-cranelift.
    ///
    /// Winch implementation differs in that, it defaults to the general case
    /// for dynamic heaps rather than optimizing for doing the least amount of
    /// work possible at runtime, this is done to align with Winch's principle
    /// of doing the least amount of work possible at compile time. For static
    /// heaps, Winch does a bit more of work, given that some of the cases that
    /// are checked against, can benefit compilation times, like for example,
    /// detecting an out of bounds access at compile time.
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
        let offset =
            bounds::ensure_index_and_offset(self.masm, index, memarg.offset, heap.ty.into());
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
                // Allocate a temporary register to hold
                //      index + offset + access_size
                //  which will serve as the check condition.
                let index_offset_and_access_size = self.context.any_gpr(self.masm);

                // Move the value of the index to the
                // index_offset_and_access_size register to perform the overflow
                // check to avoid clobbering the initial index value.
                //
                // We derive size of the operation from the heap type since:
                //
                // * This is the first assignment to the
                // `index_offset_and_access_size` register
                //
                // * The memory64 proposal specifies that the index is bound to
                // the heap type instead of hardcoding it to 32-bits (i32).
                self.masm.mov(
                    writable!(index_offset_and_access_size),
                    index_reg.into(),
                    heap.ty.into(),
                );
                // Perform
                // index = index + offset + access_size, trapping if the
                // addition overflows.
                //
                // We use the target's pointer size rather than depending on the heap
                // type since we want to check for overflow; even though the
                // offset and access size are guaranteed to be bounded by the heap
                // type, when added, if used with the wrong operand size, their
                // result could be clamped, resulting in an erroneus overflow
                // check.
                self.masm.checked_uadd(
                    writable!(index_offset_and_access_size),
                    index_offset_and_access_size,
                    RegImm::i64(offset_with_access_size as i64),
                    ptr_size,
                    TrapCode::HEAP_OUT_OF_BOUNDS,
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
                    |masm, bounds, _| {
                        let bounds_reg = bounds.as_typed_reg().reg;
                        masm.cmp(
                            index_offset_and_access_size.into(),
                            bounds_reg.into(),
                            // We use the pointer size to keep the bounds
                            // comparison consistent with the result of the
                            // overflow check above.
                            ptr_size,
                        );
                        IntCmpKind::GtU
                    },
                );
                self.context.free_reg(bounds.as_typed_reg().reg);
                self.context.free_reg(index_offset_and_access_size);
                Some(addr)
            }

            // == Static Heaps ==

            // Detect at compile time if the access is out of bounds.
            // Doing so will put the compiler in an unreachable code state,
            // optimizing the work that the compiler has to do until the
            // reachability is restored or when reaching the end of the
            // function.
            HeapStyle::Static { bound } if offset_with_access_size > bound => {
                self.emit_fuel_increment();
                self.masm.trap(TrapCode::HEAP_OUT_OF_BOUNDS);
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
            // Therefore if any 32-bit index access occurs in the region
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
                        <= u64::from(bound) + u64::from(self.tunables.memory_guard_size)
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
                        masm.cmp(
                            index_reg,
                            RegImm::i64(adjusted_bounds as i64),
                            // Similar to the dynamic heap case, even though the
                            // offset and access size are bound through the heap
                            // type, when added they can overflow, resulting in
                            // an erroneus comparison, therfore we rely on the
                            // target pointer size.
                            ptr_size,
                        );
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
                WasmValType::V128 => self.context.reg_for_type(ty, self.masm),
                _ => unreachable!(),
            };

            let src = self.masm.address_at_reg(addr, 0);
            self.masm.wasm_load(src, writable!(dst), size, sextend);
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
        }
        self.context.free_reg(src);
    }

    /// Loads the address of the table element at a given index. Returns the
    /// address of the table element using the provided register as base.
    pub fn emit_compute_table_elem_addr(
        &mut self,
        index: Reg,
        base: Reg,
        table_data: &TableData,
    ) -> M::Address {
        let scratch = scratch!(M);
        let bound = self.context.any_gpr(self.masm);
        let tmp = self.context.any_gpr(self.masm);
        let ptr_size: OperandSize = self.env.ptr_type().into();

        if let Some(offset) = table_data.import_from {
            // If the table data declares a particular offset base,
            // load the address into a register to further use it as
            // the table address.
            self.masm
                .load_ptr(self.masm.address_at_vmctx(offset), writable!(base));
        } else {
            // Else, simply move the vmctx register into the addr register as
            // the base to calculate the table address.
            self.masm.mov(writable!(base), vmctx!(M).into(), ptr_size);
        };

        // OOB check.
        let bound_addr = self
            .masm
            .address_at_reg(base, table_data.current_elems_offset);
        let bound_size = table_data.current_elements_size;
        self.masm
            .load(bound_addr, writable!(bound), bound_size.into());
        self.masm.cmp(index, bound.into(), bound_size);
        self.masm.trapif(IntCmpKind::GeU, TRAP_TABLE_OUT_OF_BOUNDS);

        // Move the index into the scratch register to calculate the table
        // element address.
        // Moving the value of the index register to the scratch register
        // also avoids overwriting the context of the index register.
        self.masm.mov(writable!(scratch), index.into(), bound_size);
        self.masm.mul(
            writable!(scratch),
            scratch,
            RegImm::i32(table_data.element_size.bytes() as i32),
            table_data.element_size,
        );
        self.masm.load_ptr(
            self.masm.address_at_reg(base, table_data.offset),
            writable!(base),
        );
        // Copy the value of the table base into a temporary register
        // so that we can use it later in case of a misspeculation.
        self.masm.mov(writable!(tmp), base.into(), ptr_size);
        // Calculate the address of the table element.
        self.masm
            .add(writable!(base), base, scratch.into(), ptr_size);
        if self.env.table_access_spectre_mitigation() {
            // Perform a bounds check and override the value of the
            // table element address in case the index is out of bounds.
            self.masm.cmp(index, bound.into(), OperandSize::S32);
            self.masm
                .cmov(writable!(base), tmp, IntCmpKind::GeU, ptr_size);
        }
        self.context.free_reg(bound);
        self.context.free_reg(tmp);
        self.masm.address_at_reg(base, 0)
    }

    /// Retrieves the size of the table, pushing the result to the value stack.
    pub fn emit_compute_table_size(&mut self, table_data: &TableData) {
        let scratch = scratch!(M);
        let size = self.context.any_gpr(self.masm);
        let ptr_size: OperandSize = self.env.ptr_type().into();

        if let Some(offset) = table_data.import_from {
            self.masm
                .load_ptr(self.masm.address_at_vmctx(offset), writable!(scratch));
        } else {
            self.masm
                .mov(writable!(scratch), vmctx!(M).into(), ptr_size);
        };

        let size_addr = self
            .masm
            .address_at_reg(scratch, table_data.current_elems_offset);
        self.masm.load(
            size_addr,
            writable!(size),
            table_data.current_elements_size.into(),
        );

        self.context.stack.push(TypedReg::i32(size).into());
    }

    /// Retrieves the size of the memory, pushing the result to the value stack.
    pub fn emit_compute_memory_size(&mut self, heap_data: &HeapData) {
        let size_reg = self.context.any_gpr(self.masm);
        let scratch = scratch!(M);

        let base = if let Some(offset) = heap_data.import_from {
            self.masm
                .load_ptr(self.masm.address_at_vmctx(offset), writable!(scratch));
            scratch
        } else {
            vmctx!(M)
        };

        let size_addr = self
            .masm
            .address_at_reg(base, heap_data.current_length_offset);
        self.masm.load_ptr(size_addr, writable!(size_reg));
        // Emit a shift to get the size in pages rather than in bytes.
        let dst = TypedReg::new(heap_data.ty, size_reg);
        let pow = heap_data.page_size_log2;
        self.masm.shift_ir(
            writable!(dst.reg),
            pow as u64,
            dst.into(),
            ShiftKind::ShrU,
            heap_data.ty.into(),
        );
        self.context.stack.push(dst.into());
    }

    /// Emit a series of instructions that check the current fuel usage by
    /// performing a zero-comparison with the number of units stored in
    /// `VMRuntimeLimits`.
    pub fn emit_fuel_check(&mut self) {
        let fuel_var = self.emit_load_fuel_consumed();
        let continuation = self.masm.get_label();

        // Fuel is stored as a negative i64, so if the number is less than zero,
        // we're still under the fuel limits.
        self.masm.branch(
            IntCmpKind::LtS,
            fuel_var,
            RegImm::i64(0),
            continuation,
            OperandSize::S64,
        );
        // Out-of-fuel branch.
        let out_of_fuel = self.env.builtins.out_of_gas::<M::ABI, M::Ptr>();
        FnCall::emit::<M>(
            &mut self.env,
            self.masm,
            &mut self.context,
            Callee::Builtin(out_of_fuel.clone()),
        );
        // Under fuel limits branch.
        self.masm.bind(continuation);
        self.context.free_reg(fuel_var);
    }

    /// Increments the fuel consumed in `VMRuntimeLimits` by flushing
    /// `self.fuel_consumed` to memory.
    fn emit_fuel_increment(&mut self) {
        let fuel_at_point = std::mem::replace(&mut self.fuel_consumed, 0);
        if fuel_at_point == 0 {
            return;
        }

        let limits_offset = self.env.vmoffsets.ptr.vmctx_runtime_limits();
        let fuel_offset = self.env.vmoffsets.ptr.vmruntime_limits_fuel_consumed();
        let limits_var = self.context.any_gpr(self.masm);

        // Load `VMRuntimeLimits` into the `limits_var` reg.
        self.masm.load_ptr(
            self.masm.address_at_vmctx(u32::from(limits_offset)),
            writable!(limits_var),
        );

        // Load the fuel consumed at point into the scratch register.
        self.masm.load(
            self.masm.address_at_reg(limits_var, u32::from(fuel_offset)),
            writable!(scratch!(M)),
            OperandSize::S64,
        );

        // Add the fuel consumed at point with the value in the scratch
        // register.
        self.masm.add(
            writable!(scratch!(M)),
            scratch!(M),
            RegImm::i64(fuel_at_point),
            OperandSize::S64,
        );

        // Store the updated fuel consumed to `VMRuntimeLimits`.
        self.masm.store(
            scratch!(M).into(),
            self.masm.address_at_reg(limits_var, u32::from(fuel_offset)),
            OperandSize::S64,
        );

        self.context.free_reg(limits_var);
    }

    /// Emits a series of instructions that load the `fuel_consumed` field from
    /// `VMRuntimeLimits`.
    fn emit_load_fuel_consumed(&mut self) -> Reg {
        let limits_offset = self.env.vmoffsets.ptr.vmctx_runtime_limits();
        let fuel_offset = self.env.vmoffsets.ptr.vmruntime_limits_fuel_consumed();
        let fuel_var = self.context.any_gpr(self.masm);
        self.masm.load_ptr(
            self.masm.address_at_vmctx(u32::from(limits_offset)),
            writable!(fuel_var),
        );

        self.masm.load(
            self.masm.address_at_reg(fuel_var, u32::from(fuel_offset)),
            writable!(fuel_var),
            // Fuel is an i64.
            OperandSize::S64,
        );

        fuel_var
    }

    /// Hook to handle fuel before visiting an operator.
    fn fuel_before_visit_op(&mut self, op: &Operator) {
        if !self.context.reachable {
            // `self.fuel_consumed` must be correctly flushed to memory when
            // entering an unreachable state.
            debug_assert_eq!(self.fuel_consumed, 0);
            return;
        }

        // Generally, most instructions require 1 fuel unit.
        //
        // However, there are exceptions, which are detailed in the code below.
        // Note that the fuel accounting semantics align with those of
        // Cranelift; for further information, refer to
        // `crates/cranelift/src/func_environ.rs`.
        //
        // The primary distinction between the two implementations is that Winch
        // does not utilize a local-based cache to track fuel consumption.
        // Instead, each increase in fuel necessitates loading from and storing
        // to memory.
        //
        // Memory traffic will undoubtedly impact runtime performance. One
        // potential optimization is to designate a register as non-allocatable,
        // when fuel consumption is enabled, effectively using it as a local
        // fuel cache.
        self.fuel_consumed += match op {
            Operator::Nop | Operator::Drop => 0,
            Operator::Block { .. }
            | Operator::Loop { .. }
            | Operator::Unreachable
            | Operator::Return
            | Operator::Else
            | Operator::End => 0,
            _ => 1,
        };

        match op {
            Operator::Unreachable
            | Operator::Loop { .. }
            | Operator::If { .. }
            | Operator::Else { .. }
            | Operator::Br { .. }
            | Operator::BrIf { .. }
            | Operator::BrTable { .. }
            | Operator::End
            | Operator::Return
            | Operator::CallIndirect { .. }
            | Operator::Call { .. }
            | Operator::ReturnCall { .. }
            | Operator::ReturnCallIndirect { .. } => {
                self.emit_fuel_increment();
            }
            _ => {}
        }
    }

    // Hook to handle source location mapping before visiting an operator.
    fn source_location_before_visit_op(&mut self, offset: usize) {
        let loc = SourceLoc::new(offset as u32);
        let rel = self.source_loc_from(loc);
        self.source_location.current = self.masm.start_source_loc(rel);
    }

    // Hook to handle source location mapping after visiting an operator.
    fn source_location_after_visit_op(&mut self) {
        // Because in Winch binary emission is done in a single pass
        // and because the MachBuffer performs optimizations during
        // emission, we have to be careful when calling
        // [`MacroAssembler::end_source_location`] to avoid breaking the
        // invariant that checks that the end [CodeOffset] must be equal
        // or greater than the start [CodeOffset].
        if self.masm.current_code_offset() >= self.source_location.current.0 {
            self.masm.end_source_loc();
        }
    }
}

/// Returns the index of the [`ControlStackFrame`] for the given
/// depth.
pub fn control_index(depth: u32, control_length: usize) -> usize {
    (control_length - 1)
        .checked_sub(depth as usize)
        .unwrap_or_else(|| panic!("expected valid control stack frame at index: {depth}"))
}
