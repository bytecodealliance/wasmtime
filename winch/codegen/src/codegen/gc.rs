use super::{CodeGen, CodeGenError, Emission};
use crate::{
    Result,
    codegen::{Callee, FnCall},
    masm::{IntCmpKind, IntScratch, MacroAssembler, OperandSize, RegImm},
    reg::{Reg, writable},
    stack::{TypedReg, Val},
};
use cranelift_codegen::MachLabel;
use wasmtime_cranelift::{TRAP_ALLOCATION_TOO_LARGE, TRAP_GC_HEAP_CORRUPT};
use wasmtime_environ::{
    Collector, GcTypeLayouts, I31_DISCRIMINANT, ModuleInternedTypeIndex, PtrSize,
    VM_GC_HEADER_KIND_OFFSET, VM_GC_HEADER_TYPE_INDEX_OFFSET, VMGcKind,
    copying::CopyingTypeLayouts, drc::DrcTypeLayouts, null::NullTypeLayouts,
};

/// Collector-specific configuration used while generating GC operations.
#[derive(Clone, Copy)]
pub(crate) struct GcCodegenConfig {
    collector: Collector,
    layouts: &'static dyn GcTypeLayouts,
}

impl GcCodegenConfig {
    pub(super) fn new(collector: Collector) -> Self {
        let layouts = match collector {
            Collector::DeferredReferenceCounting => &DrcTypeLayouts as &dyn GcTypeLayouts,
            Collector::Null => &NullTypeLayouts,
            Collector::Copying => &CopyingTypeLayouts,
        };
        Self { collector, layouts }
    }

    pub(crate) fn collector(self) -> Collector {
        self.collector
    }

    pub(crate) fn layouts(self) -> &'static dyn GcTypeLayouts {
        self.layouts
    }
}

impl<'a, 'translation, 'data, M> CodeGen<'a, 'translation, 'data, M, Emission>
where
    M: MacroAssembler,
{
    /// Returns the collector configuration required by a GC operation.
    pub(crate) fn require_gc_codegen_config(&self) -> GcCodegenConfig {
        self.gc_codegen_config.expect(
            "attempted GC code generation when GC support was not enabled at configuration time",
        )
    }

    /// Allocates an uninitialized GC object and returns both its encoded heap
    /// reference and native address.
    ///
    /// The returned registers are allocated from the code-generation context
    /// and are owned by the caller, which must eventually free them. All other
    /// registers used during allocation are allocated and released here.
    pub(crate) fn emit_gc_raw_alloc(
        &mut self,
        kind: VMGcKind,
        interned: ModuleInternedTypeIndex,
        layout: &core::alloc::Layout,
        reserved_bits: u32,
    ) -> Result<(TypedReg, Reg)> {
        let kind = kind.as_u32() | reserved_bits;
        let size =
            u32::try_from(layout.size()).map_err(|_| CodeGenError::allocation_too_large())?;
        let align =
            u32::try_from(layout.align()).map_err(|_| CodeGenError::allocation_too_large())?;
        match self.require_gc_codegen_config().collector() {
            Collector::Null => self.emit_null_gc_raw_alloc(kind, interned, size, align),
            Collector::DeferredReferenceCounting | Collector::Copying => {
                self.emit_builtin_gc_raw_alloc(kind, interned, size, align)
            }
        }
    }

    /// Allocates an uninitialized object with the null collector's bump pointer.
    fn emit_null_gc_raw_alloc(
        &mut self,
        kind: u32,
        interned: ModuleInternedTypeIndex,
        size: u32,
        align: u32,
    ) -> Result<(TypedReg, Reg)> {
        let heap_data_offset = self.env.vmoffsets.ptr.vmctx().gc_heap_data();
        let bump = self.context.any_gpr(self.masm)?;
        self.masm.with_scratch::<IntScratch, _>(|masm, heap_data| {
            masm.load_ptr(
                masm.address_at_vmctx(u32::from(heap_data_offset))?,
                heap_data.writable(),
            )?;
            masm.load(
                masm.address_at_reg(heap_data.inner(), 0)?,
                writable!(bump),
                OperandSize::S32,
            )
        })?;

        debug_assert!(align.is_power_of_two());

        // Round the bump pointer up to `align`:
        //
        //     (bump + (align - 1)) & !(align - 1)
        //
        // Trap if the addition overflows the GC heap's 32-bit offset space.
        let align_minus_one = align - 1;
        self.masm.checked_uadd(
            writable!(bump),
            bump,
            RegImm::i32(align_minus_one.cast_signed()),
            OperandSize::S32,
            TRAP_ALLOCATION_TOO_LARGE,
        )?;

        self.masm.and(
            writable!(bump),
            bump,
            RegImm::i32(!align_minus_one.cast_signed()),
            OperandSize::S32,
        )?;

        // Compute the first byte after the allocation. An overflow cannot
        // describe a valid GC object, so trap before comparing the end against
        // the heap bound.
        let end = self.context.any_gpr(self.masm)?;
        self.masm
            .mov(writable!(end), bump.into(), OperandSize::S32)?;
        self.masm.checked_uadd(
            writable!(end),
            end,
            RegImm::i32(size.cast_signed()),
            OperandSize::S32,
            TRAP_ALLOCATION_TOO_LARGE,
        )?;

        self.context.stack.push(TypedReg::i32(bump).into());
        self.context.stack.push(TypedReg::i32(end).into());
        self.context.spill(self.masm)?;
        let end = self.context.pop_to_reg(self.masm, None)?;
        let (heap_reg, heap_bound) = self.emit_load_gc_heap_base_and_bound()?;
        self.context.free_reg(heap_reg);

        // If the allocation extends beyond the current heap, grow it by the
        // difference (`end - heap_bound`). The subtraction is only reached when
        // `end > heap_bound`, so it cannot underflow. The builtin either grows
        // the heap by at least this amount or traps.
        let in_bounds = self.masm.get_label()?;
        self.masm.branch(
            IntCmpKind::LeU,
            end.reg,
            heap_bound.into(),
            in_bounds,
            OperandSize::S64,
        )?;
        self.masm.sub(
            writable!(end.reg),
            end.reg,
            heap_bound.into(),
            OperandSize::S64,
        )?;
        self.context.free_reg(heap_bound);
        self.context.stack.push(TypedReg::i64(end.reg).into());
        let grow_gc_heap = self.env.builtins.grow_gc_heap::<M::ABI>()?;
        FnCall::emit::<M>(
            &mut self.env,
            self.masm,
            &mut self.context,
            Callee::Builtin(grow_gc_heap),
        )?;
        self.context.pop_and_free(self.masm)?;

        self.masm.bind(in_bounds)?;
        let aligned = self.context.pop_to_reg(self.masm, None)?;

        // Heap growth may relocate the backing memory. Reload the heap base
        // and add the aligned heap offset to obtain the native address used to
        // initialize the object.
        let (heap_reg, heap_bound) = self.emit_load_gc_heap_base_and_bound()?;
        self.context.free_reg(heap_bound);
        let object_addr = self.emit_gc_ref_addr(aligned.reg, heap_reg)?;
        self.context.free_reg(heap_reg);

        // The null collector stores the object's kind and size in the first
        // header word. `VMGcKind::MASK` reserves the upper six bits for the
        // kind; the size occupies the remaining lower 26 bits, so bitwise OR
        // packs both values without overlap.
        let kind_and_size = kind | size;
        self.masm.store(
            RegImm::i32(kind_and_size.cast_signed()),
            self.masm
                .address_at_reg(object_addr, VM_GC_HEADER_KIND_OFFSET)?,
            OperandSize::S32,
        )?;
        let shared_ty_size = self.env.vmoffsets.size_of_vmshared_type_index();
        let shared_ty_offset = interned
            .as_u32()
            .checked_mul(u32::from(shared_ty_size))
            .unwrap();
        let type_ids_offset = self.env.vmoffsets.ptr.vmctx().type_ids();
        self.masm.with_scratch::<IntScratch, _>(|masm, shared_ty| {
            masm.load_ptr(
                masm.address_at_vmctx(type_ids_offset.into())?,
                shared_ty.writable(),
            )?;
            masm.load(
                masm.address_at_reg(shared_ty.inner(), shared_ty_offset)?,
                shared_ty.writable(),
                OperandSize::from_bytes(shared_ty_size),
            )?;
            masm.store(
                shared_ty.inner().into(),
                masm.address_at_reg(object_addr, VM_GC_HEADER_TYPE_INDEX_OFFSET)?,
                OperandSize::from_bytes(shared_ty_size),
            )
        })?;
        let heap_data = self.context.any_gpr(self.masm)?;
        self.masm.load_ptr(
            self.masm.address_at_vmctx(u32::from(heap_data_offset))?,
            writable!(heap_data),
        )?;
        self.masm.with_scratch::<IntScratch, _>(|masm, end| {
            // The checked addition above established that this cannot
            // overflow.
            masm.mov(end.writable(), aligned.reg.into(), OperandSize::S32)?;
            masm.add(
                end.writable(),
                end.inner(),
                RegImm::i32(size.cast_signed()),
                OperandSize::S32,
            )?;
            masm.store(
                end.inner().into(),
                masm.address_at_reg(heap_data, 0)?,
                OperandSize::S32,
            )
        })?;
        self.context.free_reg(heap_data);
        Ok((aligned, object_addr))
    }

    /// Allocates an uninitialized object with the collector's allocation
    /// builtin.
    fn emit_builtin_gc_raw_alloc(
        &mut self,
        kind: u32,
        interned: ModuleInternedTypeIndex,
        size: u32,
        align: u32,
    ) -> Result<(TypedReg, Reg)> {
        let shared_ty_size = self.env.vmoffsets.size_of_vmshared_type_index();
        let shared_ty_offset = interned
            .as_u32()
            .checked_mul(u32::from(shared_ty_size))
            .unwrap();

        // The shared type ID is a call argument, so allocate it here and let
        // call emission consume it rather than keeping a caller-owned register
        // live across this helper.
        let shared_ty = self.context.any_gpr(self.masm)?;
        self.masm.load_ptr(
            self.masm
                .address_at_vmctx(self.env.vmoffsets.ptr.vmctx().type_ids().into())?,
            writable!(shared_ty),
        )?;
        self.masm.load(
            self.masm.address_at_reg(shared_ty, shared_ty_offset)?,
            writable!(shared_ty),
            OperandSize::from_bytes(shared_ty_size),
        )?;
        self.context.stack.push(Val::i32(kind.cast_signed()));
        self.context.stack.push(TypedReg::i32(shared_ty).into());
        self.context.stack.push(Val::i32(size.cast_signed()));
        self.context.stack.push(Val::i32(align.cast_signed()));
        let gc_alloc_raw = self.env.builtins.gc_alloc_raw::<M::ABI>()?;
        FnCall::emit::<M>(
            &mut self.env,
            self.masm,
            &mut self.context,
            Callee::Builtin(gc_alloc_raw),
        )?;

        let gc_ref = self.context.pop_to_reg(self.masm, None)?;
        let (heap_base, heap_bound) = self.emit_load_gc_heap_base_and_bound()?;
        self.context.free_reg(heap_bound);
        let object_addr = self.emit_gc_ref_addr(gc_ref.reg, heap_base)?;
        self.context.free_reg(heap_base);
        Ok((gc_ref, object_addr))
    }

    /// Branches to `skip` when `gc_ref` is null or an unboxed i31 reference.
    ///
    /// A `VMGcRef` is null when all its bits are zero. The first branch tests
    /// this by comparing the register with itself. The second test masks off
    /// every bit except the i31 discriminant and skips the heap access when that bit is set.
    ///
    /// The discriminant test uses the dedicated scratch register rather than an
    /// allocated temporary: the scratch register is non-allocatable, so it can
    /// never alias `gc_ref`, which masking in place would otherwise destroy on
    /// the fall-through path. Note that this means the helper must not be
    /// called from inside another `with_scratch` scope.
    pub(super) fn emit_skip_if_gc_ref_is_null_or_i31(
        &mut self,
        gc_ref: Reg,
        skip: MachLabel,
    ) -> Result<()> {
        self.masm.branch(
            IntCmpKind::Eq,
            gc_ref,
            gc_ref.into(),
            skip,
            OperandSize::S32,
        )?;
        self.masm.with_scratch::<IntScratch, _>(|masm, scratch| {
            masm.mov(scratch.writable(), gc_ref.into(), OperandSize::S32)?;
            masm.and(
                scratch.writable(),
                scratch.inner(),
                RegImm::i32(I31_DISCRIMINANT as i32),
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

    /// Loads the GC heap's base address and current length.
    ///
    /// Both returned registers are allocated from the code-generation context
    /// and are owned by the caller. The caller must eventually free them.
    pub(crate) fn emit_load_gc_heap_base_and_bound(&mut self) -> Result<(Reg, Reg)> {
        self.needs_gc_heap = true;
        let store_context_offset = self.env.vmoffsets.ptr.vmctx().store_context();
        let gc_heap_base_offset = self.env.vmoffsets.ptr.vm_store_context().gc_heap_base();
        let gc_heap_len_offset = self
            .env
            .vmoffsets
            .ptr
            .vm_store_context()
            .gc_heap_current_length();
        let heap_base = self.context.any_gpr(self.masm)?;
        let heap_bound = self.context.any_gpr(self.masm)?;

        self.masm.load_ptr(
            self.masm
                .address_at_vmctx(u32::from(store_context_offset))?,
            writable!(heap_base),
        )?;
        self.masm.load(
            self.masm
                .address_at_reg(heap_base, u32::from(gc_heap_len_offset))?,
            writable!(heap_bound),
            OperandSize::S64,
        )?;
        self.masm.load_ptr(
            self.masm
                .address_at_reg(heap_base, u32::from(gc_heap_base_offset))?,
            writable!(heap_base),
        )?;

        Ok((heap_base, heap_bound))
    }

    /// Checks that accessing `access_extent` bytes starting at `gc_ref` stays
    /// within the GC heap.
    ///
    /// Callers must perform this check before converting a non-null, non-i31
    /// reference into a native address and accessing its object header.
    ///
    /// The end of the access is computed in the dedicated scratch register. It
    /// is non-allocatable, so it can alias neither `gc_ref` — which would leave
    /// the reference offset by `access_extent` — nor `heap_bound`, which would
    /// turn the comparison into `cmp scratch, scratch` and silently disable the
    /// check. Using it also guarantees nothing is emitted between `cmp` and
    /// `trapif`. Note that this means the helper must not be called from inside
    /// another `with_scratch` scope.
    pub(super) fn emit_gc_ref_bounds_check(
        &mut self,
        gc_ref: Reg,
        heap_bound: Reg,
        access_extent: i64,
    ) -> Result<()> {
        self.masm.with_scratch::<IntScratch, _>(|masm, scratch| {
            masm.mov(scratch.writable(), gc_ref.into(), OperandSize::S64)?;
            masm.add(
                scratch.writable(),
                scratch.inner(),
                RegImm::i64(access_extent),
                OperandSize::S64,
            )?;
            masm.cmp(scratch.inner(), heap_bound.into(), OperandSize::S64)?;
            masm.trapif(IntCmpKind::GtU, TRAP_GC_HEAP_CORRUPT)
        })
    }

    /// Converts a bounds-checked `VMGcRef` heap offset into a native address.
    ///
    /// The returned register is allocated from the code-generation context and
    /// is owned by the caller, which must eventually free it. Allocating here
    /// keeps the result from aliasing `gc_ref`, which the addition would
    /// otherwise clobber. Callers that want the address to reuse a register
    /// that is dead by this point should free that register first and let the
    /// allocator hand it back.
    pub(crate) fn emit_gc_ref_addr(&mut self, gc_ref: Reg, heap_base: Reg) -> Result<Reg> {
        let dst = self.context.any_gpr(self.masm)?;
        self.masm
            .mov(writable!(dst), heap_base.into(), OperandSize::S64)?;
        self.masm
            .add(writable!(dst), dst, gc_ref.into(), OperandSize::S64)?;
        Ok(dst)
    }
}
