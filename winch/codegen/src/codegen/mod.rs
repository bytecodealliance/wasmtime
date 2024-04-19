use crate::{
    abi::{vmctx, ABIOperand, ABISig, RetArea, ABI},
    codegen::BlockSig,
    isa::reg::Reg,
    masm::{
        ExtendKind, IntCmpKind, MacroAssembler, OperandSize, RegImm, SPOffset, ShiftKind, TrapCode,
    },
    stack::{TypedReg, Val},
};
use anyhow::Result;
use smallvec::SmallVec;
use wasmparser::{
    BinaryReader, FuncValidator, MemArg, Operator, ValidatorResources, VisitOperator,
};
use wasmtime_environ::{
    GlobalIndex, MemoryIndex, PtrSize, TableIndex, TypeIndex, WasmHeapType, WasmValType,
    FUNCREF_MASK, WASM_PAGE_SIZE,
};

use cranelift_codegen::{
    binemit::CodeOffset,
    ir::{RelSourceLoc, SourceLoc},
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
            source_location: Default::default(),
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
        // We need to use the vmctx paramter before pinning it for stack checking.
        self.masm.prologue(vmctx);
        // Pin the `VMContext` pointer.
        self.masm
            .mov(vmctx.into(), vmctx!(M), self.env.ptr_type().into());

        self.masm.reserve_stack(self.context.frame.locals_size);

        self.masm.end_source_loc();

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
                // The result base parameter is a stack paramter, addressed
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
        }
        validator.finish(body.original_position())?;
        return Ok(());

        struct ValidateThenVisit<'a, T, U>(T, &'a mut U, usize);

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
                            let location = SourceLoc::new(self.2 as u32);
                            self.1.start(location);
                            let res = Ok(self.1.$visit($($($arg),*)?));
                            self.1.end();
                            res
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

        /// Trait to map source locations to machine code.
        trait SourceLocator {
            fn start(&mut self, loc: SourceLoc);
            fn end(&mut self);
        }

        impl<'a, 'translation, 'data, M: MacroAssembler> ReachableState
            for CodeGen<'a, 'translation, 'data, M>
        {
            fn is_reachable(&self) -> bool {
                self.context.reachable
            }
        }

        impl<'a, 'translation, 'data, M: MacroAssembler> SourceLocator
            for CodeGen<'a, 'translation, 'data, M>
        {
            fn start(&mut self, loc: SourceLoc) {
                let rel = self.source_loc_from(loc);
                self.source_location.current = self.masm.start_source_loc(rel);
            }

            fn end(&mut self) {
                // Because in Winch binary emission is done in a single pass
                // and because the MachBuffer performs optimizations during
                // emission, we have to be careful when calling
                // [MacroAssembler::end_source_location] to avoid breaking the
                // invariant that checks that the end [CodeOffset] must be equal
                // or greater than the start [CodeOffset].
                if self.masm.current_code_offset() >= self.source_location.current.0 {
                    self.masm.end_source_loc();
                }
            }
        }

        impl<'a, T, U> VisitOperator<'a> for ValidateThenVisit<'_, T, U>
        where
            T: VisitOperator<'a, Output = wasmparser::Result<()>>,
            U: VisitOperator<'a> + ReachableState + SourceLocator,
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
        self.masm.cmp(caller_id, callee_id.into(), OperandSize::S32);
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
        self.masm.start_source_loc(Default::default());
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

    /// Loads the address of the given global.
    pub fn emit_get_global_addr(&mut self, index: GlobalIndex) -> (WasmValType, M::Address) {
        let data = self.env.resolve_global(index);

        let addr = if data.imported {
            let global_base = self.masm.address_at_reg(vmctx!(M), data.offset);
            let scratch = <M::ABI as ABI>::scratch_reg();
            self.masm.load_ptr(global_base, scratch);
            self.masm.address_at_reg(scratch, 0)
        } else {
            self.masm.address_at_reg(vmctx!(M), data.offset)
        };

        (data.ty, addr)
    }

    pub fn emit_lazy_init_funcref(&mut self, table_index: TableIndex) {
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
        self.masm.load_ptr(elem_addr, elem_value);
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
                self.masm.mov(
                    index_reg.into(),
                    index_offset_and_access_size,
                    heap.ty.into(),
                );
                // Perform
                // index = index + offset + access_size, trapping if the
                // addition overflows.
                self.masm.checked_uadd(
                    index_offset_and_access_size,
                    index_offset_and_access_size,
                    RegImm::i64(offset_with_access_size as i64),
                    heap.ty.into(),
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
                    |masm, bounds, _| {
                        let bounds_reg = bounds.as_typed_reg().reg;
                        masm.cmp(
                            index_offset_and_access_size.into(),
                            bounds_reg.into(),
                            heap.ty.into(),
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
                        masm.cmp(
                            index_reg,
                            RegImm::i64(adjusted_bounds as i64),
                            heap.ty.into(),
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
        let scratch = <M::ABI as ABI>::scratch_reg();
        let bound = self.context.any_gpr(self.masm);
        let tmp = self.context.any_gpr(self.masm);
        let ptr_size: OperandSize = self.env.ptr_type().into();

        if let Some(offset) = table_data.import_from {
            // If the table data declares a particular offset base,
            // load the address into a register to further use it as
            // the table address.
            self.masm.load_ptr(self.masm.address_at_vmctx(offset), base);
        } else {
            // Else, simply move the vmctx register into the addr register as
            // the base to calculate the table address.
            self.masm.mov(vmctx!(M).into(), base, ptr_size);
        };

        // OOB check.
        let bound_addr = self
            .masm
            .address_at_reg(base, table_data.current_elems_offset);
        let bound_size = table_data.current_elements_size;
        self.masm.load(bound_addr, bound, bound_size.into());
        self.masm.cmp(index, bound.into(), bound_size);
        self.masm
            .trapif(IntCmpKind::GeU, TrapCode::TableOutOfBounds);

        // Move the index into the scratch register to calcualte the table
        // element address.
        // Moving the value of the index register to the scratch register
        // also avoids overwriting the context of the index register.
        self.masm.mov(index.into(), scratch, bound_size);
        self.masm.mul(
            scratch,
            scratch,
            RegImm::i32(table_data.element_size.bytes() as i32),
            table_data.element_size,
        );
        self.masm
            .load_ptr(self.masm.address_at_reg(base, table_data.offset), base);
        // Copy the value of the table base into a temporary register
        // so that we can use it later in case of a misspeculation.
        self.masm.mov(base.into(), tmp, ptr_size);
        // Calculate the address of the table element.
        self.masm.add(base, base, scratch.into(), ptr_size);
        if self.env.table_access_spectre_mitigation() {
            // Perform a bounds check and override the value of the
            // table element address in case the index is out of bounds.
            self.masm.cmp(index, bound.into(), OperandSize::S32);
            self.masm.cmov(tmp, base, IntCmpKind::GeU, ptr_size);
        }
        self.context.free_reg(bound);
        self.context.free_reg(tmp);
        self.masm.address_at_reg(base, 0)
    }

    /// Retrieves the size of the table, pushing the result to the value stack.
    pub fn emit_compute_table_size(&mut self, table_data: &TableData) {
        let scratch = <M::ABI as ABI>::scratch_reg();
        let size = self.context.any_gpr(self.masm);
        let ptr_size: OperandSize = self.env.ptr_type().into();

        if let Some(offset) = table_data.import_from {
            self.masm
                .load_ptr(self.masm.address_at_vmctx(offset), scratch);
        } else {
            self.masm.mov(vmctx!(M).into(), scratch, ptr_size);
        };

        let size_addr = self
            .masm
            .address_at_reg(scratch, table_data.current_elems_offset);
        self.masm
            .load(size_addr, size, table_data.current_elements_size.into());

        self.context.stack.push(TypedReg::i32(size).into());
    }

    /// Retrieves the size of the memory, pushing the result to the value stack.
    pub fn emit_compute_memory_size(&mut self, heap_data: &HeapData) {
        let size_reg = self.context.any_gpr(self.masm);
        let scratch = <M::ABI as ABI>::scratch_reg();

        let base = if let Some(offset) = heap_data.import_from {
            self.masm
                .load_ptr(self.masm.address_at_vmctx(offset), scratch);
            scratch
        } else {
            vmctx!(M)
        };

        let size_addr = self
            .masm
            .address_at_reg(base, heap_data.current_length_offset);
        self.masm.load_ptr(size_addr, size_reg);
        // Prepare the stack to emit a shift to get the size in pages rather
        // than in bytes.
        self.context
            .stack
            .push(TypedReg::new(heap_data.ty, size_reg).into());

        // Since the page size is a power-of-two, verify that 2^16, equals the
        // defined constant. This is mostly a safeguard in case the constant
        // value ever changes.
        let pow = 16;
        debug_assert_eq!(2u32.pow(pow), WASM_PAGE_SIZE);

        // Ensure that the constant is correctly typed according to the heap
        // type to reduce register pressure when emitting the shift operation.
        match heap_data.ty {
            WasmValType::I32 => self.context.stack.push(Val::i32(pow as i32)),
            WasmValType::I64 => self.context.stack.push(Val::i64(pow as i64)),
            _ => unreachable!(),
        }

        self.masm
            .shift(&mut self.context, ShiftKind::ShrU, heap_data.ty.into());
    }
}

/// Returns the index of the [`ControlStackFrame`] for the given
/// depth.
pub fn control_index(depth: u32, control_length: usize) -> usize {
    (control_length - 1)
        .checked_sub(depth as usize)
        .unwrap_or_else(|| panic!("expected valid control stack frame at index: {}", depth))
}
