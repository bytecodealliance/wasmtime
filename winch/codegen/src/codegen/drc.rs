use super::{Callee, CodeGen, CodeGenError, Emission, FnCall};
use crate::{
    Result,
    masm::{IntCmpKind, MacroAssembler, OperandSize, RegImm},
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
        let null_or_i31_scratch = self.context.any_gpr(self.masm)?;
        self.emit_skip_if_gc_ref_is_null_or_i31(ref_reg, null_or_i31_scratch, skip_barrier)?;
        self.context.free_reg(null_or_i31_scratch);

        let (heap_reg, bound_reg) = self.emit_load_gc_heap_base_and_bound()?;

        // The read barrier accesses through the header's `next` field.
        let header_extent = i64::from(
            self.env
                .vmoffsets
                .vm_drc_header_next_over_approximated_stack_root(),
        ) + 4;
        let extent_reg = self.context.any_gpr(self.masm)?;
        self.emit_gc_ref_bounds_check(ref_reg, bound_reg, header_extent, extent_reg)?;
        self.context.free_reg(extent_reg);

        // The bound is dead after the check, so reuse its register for the
        // native object address and keep register pressure low.
        let object_addr = bound_reg;
        self.emit_gc_ref_addr(ref_reg, heap_reg, object_addr)?;

        let bits_reg = self.context.any_gpr(self.masm)?;
        self.emit_skip_if_in_over_approx_stack_roots(object_addr, bits_reg, skip_barrier)?;

        let heap_data_reg =
            self.emit_push_over_approx_stack_root(ref_reg, object_addr, bits_reg)?;

        // `ref_reg` and the heap base are no longer needed. The remaining
        // three registers are consumed and freed by `emit_maybe_force_gc`.
        self.context.free_reg(ref_reg);
        self.context.free_reg(heap_reg);
        self.emit_maybe_force_gc(bits_reg, heap_data_reg, object_addr)?;

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
        let count_addr_reg = self.context.any_gpr(self.masm)?;
        let count_reg = self.context.any_gpr(self.masm)?;

        // Retain the new heap reference before publishing it. This ordering
        // keeps self-assignment from temporarily dropping the final owner.
        let skip_inc = self.masm.get_label()?;
        self.emit_skip_if_gc_ref_is_null_or_i31(new_ref.reg, count_addr_reg, skip_inc)?;
        self.emit_gc_ref_bounds_check(new_ref.reg, bound_reg, header_extent, count_addr_reg)?;
        self.emit_gc_ref_addr(new_ref.reg, heap_reg, count_addr_reg)?;
        self.emit_mutate_ref_count(count_addr_reg, count_reg, RefCountMutation::Increment)?;
        self.emit_store_ref_count(count_addr_reg, count_reg)?;

        self.masm.bind(skip_inc)?;
        // Publish the new value before releasing the old one because the
        // zero-count path below can make an out-of-line runtime call.
        self.masm.store(new_ref.reg.into(), addr, ty.try_into()?)?;

        // Release the old heap reference, storing a nonzero count inline and
        // delegating the zero-count case to `drop_gc_ref`.
        let skip_dec = self.masm.get_label()?;
        self.emit_skip_if_gc_ref_is_null_or_i31(old_reg, count_addr_reg, skip_dec)?;
        self.emit_gc_ref_bounds_check(old_reg, bound_reg, header_extent, count_addr_reg)?;
        self.emit_gc_ref_addr(old_reg, heap_reg, count_addr_reg)?;
        self.emit_mutate_ref_count(count_addr_reg, count_reg, RefCountMutation::Decrement)?;

        let drop_old = self.masm.get_label()?;
        self.masm.branch(
            IntCmpKind::Eq,
            count_reg,
            RegImm::i64(0),
            drop_old,
            OperandSize::S64,
        )?;
        self.emit_store_ref_count(count_addr_reg, count_reg)?;
        self.masm.jmp(skip_dec)?;

        self.context.free_reg(count_reg);
        self.context.free_reg(count_addr_reg);
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
    fn emit_skip_if_in_over_approx_stack_roots(
        &mut self,
        object_addr: Reg,
        scratch: Reg,
        skip: MachLabel,
    ) -> Result<()> {
        let reserved_offset = self.env.vmoffsets.vm_gc_header_reserved_bits();
        self.masm.load(
            self.masm.address_at_reg(object_addr, reserved_offset)?,
            writable!(scratch),
            OperandSize::S32,
        )?;
        self.masm.and(
            writable!(scratch),
            scratch,
            RegImm::i32(DRC_HEADER_IN_OVER_APPROX_LIST_BIT as i32),
            OperandSize::S32,
        )?;
        self.masm.branch(
            IntCmpKind::Ne,
            scratch,
            scratch.into(),
            skip,
            OperandSize::S32,
        )
    }

    /// Adds `gc_ref` to the DRC heap's over-approximated stack-roots list.
    ///
    /// The object is linked at the head, marked as linked, and retained for
    /// the list's ownership.
    fn emit_push_over_approx_stack_root(
        &mut self,
        gc_ref: Reg,
        object_addr: Reg,
        scratch: Reg,
    ) -> Result<Reg> {
        let heap_data_offset = self.env.vmoffsets.ptr.vmctx_gc_heap_data();
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

        // Link this object to the old head of the over-approximated list.
        self.masm.load(
            self.masm.address_at_reg(heap_data_reg, roots_head_offset)?,
            writable!(scratch),
            OperandSize::S32,
        )?;
        self.masm.store(
            scratch.into(),
            self.masm.address_at_reg(object_addr, next_offset)?,
            OperandSize::S32,
        )?;

        // Mark the object as present so subsequent reads can take the fast
        // path, and retain it for the list's ownership.
        self.masm.load(
            self.masm.address_at_reg(object_addr, reserved_offset)?,
            writable!(scratch),
            OperandSize::S32,
        )?;
        self.masm.or(
            writable!(scratch),
            scratch,
            RegImm::i32(DRC_HEADER_IN_OVER_APPROX_LIST_BIT as i32),
            OperandSize::S32,
        )?;
        self.masm.store(
            scratch.into(),
            self.masm.address_at_reg(object_addr, reserved_offset)?,
            OperandSize::S32,
        )?;
        self.emit_mutate_ref_count(object_addr, scratch, RefCountMutation::Increment)?;
        self.emit_store_ref_count(object_addr, scratch)?;

        // Publish the new head and updated length. Leave the length in
        // `scratch` for the collection-threshold check.
        self.masm.store(
            gc_ref.into(),
            self.masm.address_at_reg(heap_data_reg, roots_head_offset)?,
            OperandSize::S32,
        )?;
        self.masm.load(
            self.masm.address_at_reg(heap_data_reg, roots_len_offset)?,
            writable!(scratch),
            OperandSize::S32,
        )?;
        self.masm.add(
            writable!(scratch),
            scratch,
            RegImm::i32(1),
            OperandSize::S32,
        )?;
        self.masm.store(
            scratch.into(),
            self.masm.address_at_reg(heap_data_reg, roots_len_offset)?,
            OperandSize::S32,
        )?;

        Ok(heap_data_reg)
    }

    /// Forces a collection when the over-approximated roots list has reached
    /// both twice its post-GC length and the absolute minimum threshold.
    ///
    /// This method consumes and frees all three register arguments before a
    /// possible call to `force_gc`.
    fn emit_maybe_force_gc(
        &mut self,
        current_len: Reg,
        heap_data: Reg,
        scratch: Reg,
    ) -> Result<()> {
        let last_len_offset = u32::from(
            self.env
                .vmoffsets
                .ptr
                .vmdrc_heap_data_over_approximated_stack_roots_len_after_last_gc(),
        );
        self.masm.load(
            self.masm.address_at_reg(heap_data, last_len_offset)?,
            writable!(scratch),
            OperandSize::S32,
        )?;
        self.masm.add(
            writable!(scratch),
            scratch,
            scratch.into(),
            OperandSize::S32,
        )?;

        let skip_gc = self.masm.get_label()?;
        self.masm.branch(
            IntCmpKind::LtU,
            current_len,
            scratch.into(),
            skip_gc,
            OperandSize::S32,
        )?;
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
        self.context.free_reg(scratch);

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

    /// Loads a reference count and applies `mutation`, leaving the updated
    /// value in `count`. The caller decides whether and when to store it.
    fn emit_mutate_ref_count(
        &mut self,
        object_addr: Reg,
        count: Reg,
        mutation: RefCountMutation,
    ) -> Result<()> {
        let ref_count_offset = self.env.vmoffsets.vm_drc_header_ref_count();
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
        Ok(())
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
