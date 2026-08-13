use super::{CodeGen, Emission};
use crate::{
    Result,
    masm::{IntCmpKind, IntScratch, MacroAssembler, OperandSize, RegImm},
    reg::{Reg, writable},
};
use cranelift_codegen::MachLabel;
use wasmtime_cranelift::TRAP_GC_HEAP_CORRUPT;
use wasmtime_environ::{I31_DISCRIMINANT, PtrSize};

impl<'a, 'translation, 'data, M> CodeGen<'a, 'translation, 'data, M, Emission>
where
    M: MacroAssembler,
{
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
    pub(super) fn emit_load_gc_heap_base_and_bound(&mut self) -> Result<(Reg, Reg)> {
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
    pub(super) fn emit_gc_ref_addr(&mut self, gc_ref: Reg, heap_base: Reg) -> Result<Reg> {
        let dst = self.context.any_gpr(self.masm)?;
        self.masm
            .mov(writable!(dst), heap_base.into(), OperandSize::S64)?;
        self.masm
            .add(writable!(dst), dst, gc_ref.into(), OperandSize::S64)?;
        Ok(dst)
    }
}
