use super::{Callee, CodeGen, CodeGenError, Emission, FnCall};
use crate::{
    Result,
    masm::{IntCmpKind, IntScratch, MacroAssembler, OperandSize, RegImm},
    reg::{Reg, writable},
    stack::{TypedReg, Val},
};
use cranelift_codegen::MachLabel;
use wasmtime_environ::{
    DRC_HEADER_IN_OVER_APPROX_LIST_BIT, DRC_MIN_OVER_APPROX_STACK_ROOTS_GC_THRESHOLD, PtrSize,
    WasmValType,
};

#[derive(Clone, Copy)]
enum RefCountMutation {
    Increment,
    Decrement,
}

impl<'a, 'translation, 'data, M> CodeGen<'a, 'translation, 'data, M, Emission>
where
    M: MacroAssembler,
{
    /// Emits a DRC read barrier for a value loaded from `addr`.
    ///
    /// The loaded reference is first pushed and spilled so it is represented
    /// in a stack map if this barrier calls `force_gc`. Null and i31 references
    /// do not point into the GC heap, so they skip the heap access and barrier
    /// bookkeeping. For a heap reference, the barrier bounds-checks the header
    /// access and, unless the object is already present, links it into the
    /// over-approximated stack-roots list, marks it as linked, and retains it.
    /// Finally it forces a collection when the roots list reaches both the
    /// proportional and absolute thresholds.
    ///
    /// Leaves the loaded reference on the value stack and marks
    /// `storage_base` available for register reuse before returning.
    pub(crate) fn emit_drc_read_barrier(
        &mut self,
        ty: WasmValType,
        storage_base: Reg,
        addr: M::Address,
    ) -> Result<()> {
        let gc_ref = self.context.reg_for_type(ty, self.masm)?;
        self.masm.load(addr, writable!(gc_ref), ty.try_into()?)?;
        self.context.stack.push(Val::reg(gc_ref, ty));

        // Spill the loaded result into a stack-map-visible slot before the
        // possible `force_gc` call, then reload a temporary copy for the
        // barrier calculations.
        self.context.spill(self.masm)?;
        let slot = self
            .context
            .stack
            .peek()
            .ok_or_else(|| CodeGenError::missing_values_in_stack())?
            .unwrap_mem()
            .slot;
        let ref_reg = self.context.any_gpr(self.masm)?;
        self.masm.load(
            self.masm.address_from_sp(slot.offset)?,
            writable!(ref_reg),
            OperandSize::S32,
        )?;

        let skip_barrier = self.masm.get_label()?;
        self.emit_skip_if_gc_ref_is_null_or_i31(ref_reg, skip_barrier)?;

        let (heap_reg, bound_reg) = self.emit_load_gc_heap_base_and_bound()?;

        // The read barrier accesses through the header's `next` field.
        let header_extent = i64::from(
            self.env
                .vmoffsets
                .vm_drc_header_next_over_approximated_stack_root(),
        ) + 4;
        self.emit_gc_ref_bounds_check(ref_reg, bound_reg, header_extent)?;

        // The bound is dead after the check. Free it before requesting the
        // object address so the allocator can hand the same register back and
        // keep register pressure low.
        self.context.free_reg(bound_reg);
        let object_addr = self.emit_gc_ref_addr(ref_reg, heap_reg)?;

        self.emit_skip_if_in_over_approx_stack_roots(object_addr, skip_barrier)?;

        let (heap_data_reg, roots_len) =
            self.emit_push_over_approx_stack_root(ref_reg, object_addr)?;

        // Only the roots-list length and the heap data pointer are still live;
        // both are consumed and freed by `emit_maybe_force_gc`.
        self.context.free_reg(ref_reg);
        self.context.free_reg(heap_reg);
        self.context.free_reg(object_addr);
        self.emit_maybe_force_gc(roots_len, heap_data_reg)?;

        self.masm.bind(skip_barrier)?;
        self.context.free_reg(storage_base);
        Ok(())
    }

    /// Emits a DRC write barrier for replacing the reference at `addr`.
    ///
    /// The barrier increments the new reference's count before decrementing the
    /// old reference's count. It stores the new reference before a possible
    /// out-of-line `drop_gc_ref` call. Null and i31 references skip their
    /// respective retain and release operations.
    ///
    /// Consumes the new reference from the value stack and marks
    /// `storage_base` available for register reuse before a possible runtime
    /// call.
    pub(crate) fn emit_drc_write_barrier(
        &mut self,
        ty: WasmValType,
        storage_base: Reg,
        addr: M::Address,
    ) -> Result<()> {
        self.context.spill(self.masm)?;
        let new_ref = self.context.pop_to_reg(self.masm, None)?;
        let (heap_reg, bound_reg) = self.emit_load_gc_heap_base_and_bound()?;

        let old_reg = self.context.any_gpr(self.masm)?;
        self.masm.load(addr, writable!(old_reg), OperandSize::S32)?;

        let ref_count_offset = self.env.vmoffsets.vm_drc_header_ref_count();
        let header_extent = i64::from(ref_count_offset) + 8;

        // Retain the new heap reference before publishing it. This ordering
        // keeps self-assignment from temporarily dropping the final owner.
        let skip_inc = self.masm.get_label()?;
        self.emit_skip_if_gc_ref_is_null_or_i31(new_ref.reg, skip_inc)?;
        self.emit_gc_ref_bounds_check(new_ref.reg, bound_reg, header_extent)?;
        let new_addr = self.emit_gc_ref_addr(new_ref.reg, heap_reg)?;
        let count = self.emit_mutate_ref_count(new_addr, RefCountMutation::Increment)?;
        self.emit_store_ref_count(new_addr, count)?;
        self.context.free_reg(count);
        self.context.free_reg(new_addr);

        self.masm.bind(skip_inc)?;
        // Publish the new value before releasing the old one because the
        // zero-count path below can make an out-of-line runtime call.
        self.masm.store(new_ref.reg.into(), addr, ty.try_into()?)?;

        // Release the old heap reference, storing a nonzero count inline and
        // delegating the zero-count case to `drop_gc_ref`.
        let skip_dec = self.masm.get_label()?;
        self.emit_skip_if_gc_ref_is_null_or_i31(old_reg, skip_dec)?;
        self.emit_gc_ref_bounds_check(old_reg, bound_reg, header_extent)?;
        let old_addr = self.emit_gc_ref_addr(old_reg, heap_reg)?;
        let count = self.emit_mutate_ref_count(old_addr, RefCountMutation::Decrement)?;

        let drop_old = self.masm.get_label()?;
        self.masm.branch(
            IntCmpKind::Eq,
            count,
            RegImm::i64(0),
            drop_old,
            OperandSize::S64,
        )?;
        self.emit_store_ref_count(old_addr, count)?;
        self.masm.jmp(skip_dec)?;

        self.context.free_reg(count);
        self.context.free_reg(old_addr);
        self.context.free_reg(bound_reg);
        self.context.free_reg(heap_reg);
        self.context.free_reg(new_ref.reg);
        self.context.free_reg(storage_base);

        self.masm.bind(drop_old)?;
        let drop_gc_ref = self.env.builtins.drop_gc_ref::<M::ABI>()?;
        self.context.stack.push(TypedReg::i32(old_reg).into());
        FnCall::emit::<M>(
            &mut self.env,
            self.masm,
            &mut self.context,
            Callee::Builtin(drop_gc_ref),
        )?;

        self.masm.bind(skip_dec)
    }

    /// Branches to `skip` when `object_addr` is already linked into the
    /// over-approximated stack-roots list.
    ///
    /// Uses the dedicated scratch register, so it must not be called from
    /// inside another `with_scratch` scope.
    fn emit_skip_if_in_over_approx_stack_roots(
        &mut self,
        object_addr: Reg,
        skip: MachLabel,
    ) -> Result<()> {
        let reserved_offset = self.env.vmoffsets.vm_gc_header_reserved_bits();
        self.masm.with_scratch::<IntScratch, _>(|masm, scratch| {
            masm.load(
                masm.address_at_reg(object_addr, reserved_offset)?,
                scratch.writable(),
                OperandSize::S32,
            )?;
            masm.and(
                scratch.writable(),
                scratch.inner(),
                RegImm::i32(DRC_HEADER_IN_OVER_APPROX_LIST_BIT as i32),
                OperandSize::S32,
            )?;
            masm.branch(
                IntCmpKind::Ne,
                scratch.inner(),
                scratch.inner().into(),
                skip,
                OperandSize::S32,
            )
        })
    }

    /// Adds `gc_ref` to the DRC heap's over-approximated stack-roots list.
    ///
    /// The object is linked at the head, marked as linked, and retained for
    /// the list's ownership.
    ///
    /// Returns the DRC heap data pointer and the updated roots-list length,
    /// both allocated here and owned by the caller, which must eventually free
    /// them. The length feeds the collection-threshold check.
    fn emit_push_over_approx_stack_root(
        &mut self,
        gc_ref: Reg,
        object_addr: Reg,
    ) -> Result<(Reg, Reg)> {
        let heap_data_offset = self.env.vmoffsets.ptr.vmctx().gc_heap_data();
        let roots_head_offset = u32::from(
            self.env
                .vmoffsets
                .ptr
                .vmdrc_heap_data_over_approximated_stack_roots(),
        );
        let roots_len_offset = u32::from(
            self.env
                .vmoffsets
                .ptr
                .vmdrc_heap_data_current_over_approximated_stack_roots_len(),
        );
        let next_offset = self
            .env
            .vmoffsets
            .vm_drc_header_next_over_approximated_stack_root();
        let reserved_offset = self.env.vmoffsets.vm_gc_header_reserved_bits();

        let heap_data_reg = self.context.any_gpr(self.masm)?;
        self.masm.load_ptr(
            self.masm.address_at_vmctx(u32::from(heap_data_offset))?,
            writable!(heap_data_reg),
        )?;

        // Link this object to the old head of the over-approximated list, then
        // mark it as present so subsequent reads can take the fast path. Both
        // steps only need a temporary, so they share the scratch register.
        self.masm.with_scratch::<IntScratch, _>(|masm, scratch| {
            masm.load(
                masm.address_at_reg(heap_data_reg, roots_head_offset)?,
                scratch.writable(),
                OperandSize::S32,
            )?;
            masm.store(
                scratch.inner().into(),
                masm.address_at_reg(object_addr, next_offset)?,
                OperandSize::S32,
            )?;

            masm.load(
                masm.address_at_reg(object_addr, reserved_offset)?,
                scratch.writable(),
                OperandSize::S32,
            )?;
            masm.or(
                scratch.writable(),
                scratch.inner(),
                RegImm::i32(DRC_HEADER_IN_OVER_APPROX_LIST_BIT as i32),
                OperandSize::S32,
            )?;
            masm.store(
                scratch.inner().into(),
                masm.address_at_reg(object_addr, reserved_offset)?,
                OperandSize::S32,
            )
        })?;

        // Retain the object for the list's ownership.
        let count = self.emit_mutate_ref_count(object_addr, RefCountMutation::Increment)?;
        self.emit_store_ref_count(object_addr, count)?;
        self.context.free_reg(count);

        // Publish the new head, then the updated length. The length outlives
        // this helper, so it gets its own allocated register.
        self.masm.store(
            gc_ref.into(),
            self.masm.address_at_reg(heap_data_reg, roots_head_offset)?,
            OperandSize::S32,
        )?;
        let roots_len = self.context.any_gpr(self.masm)?;
        self.masm.load(
            self.masm.address_at_reg(heap_data_reg, roots_len_offset)?,
            writable!(roots_len),
            OperandSize::S32,
        )?;
        self.masm.add(
            writable!(roots_len),
            roots_len,
            RegImm::i32(1),
            OperandSize::S32,
        )?;
        self.masm.store(
            roots_len.into(),
            self.masm.address_at_reg(heap_data_reg, roots_len_offset)?,
            OperandSize::S32,
        )?;

        Ok((heap_data_reg, roots_len))
    }

    /// Forces a collection when the over-approximated roots list has reached
    /// both twice its post-GC length and the absolute minimum threshold.
    ///
    /// This method consumes and frees both register arguments before a
    /// possible call to `force_gc`.
    ///
    /// The doubled post-GC length is only needed for the first comparison, so
    /// it lives in the dedicated scratch register. This helper must therefore
    /// not be called from inside another `with_scratch` scope.
    fn emit_maybe_force_gc(&mut self, current_len: Reg, heap_data: Reg) -> Result<()> {
        let last_len_offset = u32::from(
            self.env
                .vmoffsets
                .ptr
                .vmdrc_heap_data_over_approximated_stack_roots_len_after_last_gc(),
        );
        let skip_gc = self.masm.get_label()?;
        self.masm.with_scratch::<IntScratch, _>(|masm, scratch| {
            masm.load(
                masm.address_at_reg(heap_data, last_len_offset)?,
                scratch.writable(),
                OperandSize::S32,
            )?;
            masm.add(
                scratch.writable(),
                scratch.inner(),
                scratch.inner().into(),
                OperandSize::S32,
            )?;
            masm.branch(
                IntCmpKind::LtU,
                current_len,
                scratch.inner().into(),
                skip_gc,
                OperandSize::S32,
            )
        })?;
        let min_threshold = i32::try_from(DRC_MIN_OVER_APPROX_STACK_ROOTS_GC_THRESHOLD).unwrap();
        self.masm.branch(
            IntCmpKind::LtU,
            current_len,
            RegImm::i32(min_threshold),
            skip_gc,
            OperandSize::S32,
        )?;

        self.context.free_reg(heap_data);
        self.context.free_reg(current_len);

        let force_gc = self.env.builtins.force_gc::<M::ABI>()?;
        FnCall::emit::<M>(
            &mut self.env,
            self.masm,
            &mut self.context,
            Callee::Builtin(force_gc),
        )?;
        self.context.pop_and_free(self.masm)?;

        self.masm.bind(skip_gc)
    }

    /// Loads a reference count and applies `mutation`, returning the register
    /// holding the updated value. The caller decides whether and when to store
    /// it, and must eventually free the returned register.
    ///
    /// The count is allocated rather than taken from the caller because the
    /// decrement path inspects it after this helper returns, to decide between
    /// storing it and calling `drop_gc_ref`.
    fn emit_mutate_ref_count(
        &mut self,
        object_addr: Reg,
        mutation: RefCountMutation,
    ) -> Result<Reg> {
        let ref_count_offset = self.env.vmoffsets.vm_drc_header_ref_count();
        let count = self.context.any_gpr(self.masm)?;
        self.masm.load(
            self.masm.address_at_reg(object_addr, ref_count_offset)?,
            writable!(count),
            OperandSize::S64,
        )?;
        match mutation {
            RefCountMutation::Increment => {
                self.masm
                    .add(writable!(count), count, RegImm::i64(1), OperandSize::S64)?
            }
            RefCountMutation::Decrement => {
                self.masm
                    .sub(writable!(count), count, RegImm::i64(1), OperandSize::S64)?
            }
        }
        Ok(count)
    }

    fn emit_store_ref_count(&mut self, object_addr: Reg, count: Reg) -> Result<()> {
        let ref_count_offset = self.env.vmoffsets.vm_drc_header_ref_count();
        self.masm.store(
            count.into(),
            self.masm.address_at_reg(object_addr, ref_count_offset)?,
            OperandSize::S64,
        )
    }
}
