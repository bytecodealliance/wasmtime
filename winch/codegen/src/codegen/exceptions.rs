use super::{Callee, CodeGen, Emission, FnCall};
use crate::{
    Result,
    masm::{IntScratch, MacroAssembler, OperandSize, RegImm},
    reg::Reg,
    stack::TypedReg,
};
use wasmtime_environ::copying::InlineTraceInfo;
use wasmtime_environ::{
    Collector, GcStructLayout, GcTypeLayouts, ModuleInternedTypeIndex, PtrSize, TagIndex, VMGcKind,
    WasmHeapType, WasmStorageType, WasmValType,
};

impl<'a, 'translation, 'data, M> CodeGen<'a, 'translation, 'data, M, Emission>
where
    M: MacroAssembler,
{
    /// Allocates an exception and initializes its tag identity.
    ///
    /// Exception tags are identified by the defining instance and the tag's
    /// index within that instance. Imported tags therefore use the exporting
    /// instance's VMContext when resolving the instance ID.
    ///
    /// The returned GC reference and object address are allocated from the
    /// code-generation context and owned by the caller. Payload operands remain
    /// on the value stack.
    pub(crate) fn emit_exception_alloc(
        &mut self,
        tag_index: TagIndex,
        interned: ModuleInternedTypeIndex,
        layout: &GcStructLayout,
        layouts: &dyn GcTypeLayouts,
    ) -> Result<(TypedReg, Reg)> {
        let get_instance_id = self.env.builtins.get_instance_id::<M::ABI>()?;
        let defined_tag = self.env.translation.module.defined_tag_index(tag_index);
        let get_instance_id = match defined_tag {
            Some(_) => Callee::Builtin(get_instance_id),
            None => {
                let vmimport = self.env.vmoffsets.imported_tags().at(tag_index);
                let vmctx_offset =
                    vmimport + u32::from(self.env.vmoffsets.ptr.vm_tag_import().vmctx());
                Callee::BuiltinWithDifferentVmctx(get_instance_id, vmctx_offset)
            }
        };
        FnCall::emit::<M>(&mut self.env, self.masm, &mut self.context, get_instance_id)?;

        let reserved_bits = match self.tunables.collector {
            Some(Collector::Copying) => InlineTraceInfo::r#struct(layout).encode(),
            _ => 0,
        };
        let (gc_ref, object_addr) = self.emit_gc_raw_alloc(
            VMGcKind::ExnRef,
            interned,
            layout.size,
            layout.align,
            reserved_bits,
        )?;

        // Store the tag as its defining instance ID and its index within that
        // instance.
        let instance_id = self.context.pop_to_reg(self.masm, None)?;
        self.masm.store(
            instance_id.reg.into(),
            self.masm
                .address_at_reg(object_addr, layouts.exception_tag_instance_offset())?,
            OperandSize::S32,
        )?;
        self.context.free_reg(instance_id.reg);

        let tag_addr = self
            .masm
            .address_at_reg(object_addr, layouts.exception_tag_defined_offset())?;
        match defined_tag {
            Some(defined) => self.masm.store(
                RegImm::i32(defined.as_u32().cast_signed()),
                tag_addr,
                OperandSize::S32,
            )?,
            None => {
                let vmimport = self.env.vmoffsets.imported_tags().at(tag_index);
                let index_offset =
                    vmimport + u32::from(self.env.vmoffsets.ptr.vm_tag_import().index());
                self.masm
                    .with_scratch::<IntScratch, _>(|masm, defined_tag_id| {
                        masm.load(
                            masm.address_at_vmctx(index_offset)?,
                            defined_tag_id.writable(),
                            OperandSize::S32,
                        )?;
                        masm.store(defined_tag_id.inner().into(), tag_addr, OperandSize::S32)
                    })?;
            }
        }

        Ok((gc_ref, object_addr))
    }

    /// Initializes an exception's payload fields from the value stack.
    ///
    /// Payload operands are stored on the value stack in declaration order, so
    /// this method initializes fields from last to first. Function references
    /// are interned before being written into the GC heap, and DRC-managed
    /// references use an initialization barrier.
    ///
    /// This method consumes and frees `object_addr`. It returns `gc_ref`, which
    /// remains owned by the caller.
    pub(crate) fn emit_exception_payload_fields(
        &mut self,
        field_types: &[WasmStorageType],
        field_offsets: &[u32],
        mut gc_ref: TypedReg,
        mut object_addr: Reg,
    ) -> Result<TypedReg> {
        debug_assert_eq!(field_types.len(), field_offsets.len());

        for (field_ty, field_offset) in field_types
            .iter()
            .copied()
            .zip(field_offsets.iter().copied())
            .rev()
        {
            match field_ty {
                WasmStorageType::I8 => {
                    let value = self.context.pop_to_reg(self.masm, None)?;
                    let addr = self.masm.address_at_reg(object_addr, field_offset)?;
                    self.masm.store(value.reg.into(), addr, OperandSize::S8)?;
                    self.context.free_reg(value.reg);
                }
                WasmStorageType::I16 => {
                    let value = self.context.pop_to_reg(self.masm, None)?;
                    let addr = self.masm.address_at_reg(object_addr, field_offset)?;
                    self.masm.store(value.reg.into(), addr, OperandSize::S16)?;
                    self.context.free_reg(value.reg);
                }
                WasmStorageType::Val(WasmValType::Ref(r)) if r.heap_type == WasmHeapType::Func => {
                    let func_ref = self.context.pop_to_reg(self.masm, None)?;

                    // The call can clobber allocated registers, so preserve the
                    // allocation results beneath its argument.
                    self.context.stack.push(TypedReg::i64(object_addr).into());
                    self.context.stack.push(gc_ref.into());
                    self.context.stack.push(func_ref.into());

                    let intern = self.env.builtins.intern_func_ref_for_gc_heap::<M::ABI>()?;
                    FnCall::emit::<M>(
                        &mut self.env,
                        self.masm,
                        &mut self.context,
                        Callee::Builtin(intern),
                    )?;

                    let func_ref_id = self.context.pop_to_reg(self.masm, None)?;
                    gc_ref = self.context.pop_to_reg(self.masm, None)?;
                    object_addr = self.context.pop_to_reg(self.masm, None)?.reg;
                    let addr = self.masm.address_at_reg(object_addr, field_offset)?;
                    self.masm
                        .store(func_ref_id.reg.into(), addr, OperandSize::S32)?;
                    self.context.free_reg(func_ref_id.reg);
                }
                WasmStorageType::Val(ty @ WasmValType::Ref(_))
                    if self.tunables.collector == Some(Collector::DeferredReferenceCounting) =>
                {
                    let addr = self.masm.address_at_reg(object_addr, field_offset)?;
                    self.emit_drc_init_barrier(ty, addr)?;
                }
                WasmStorageType::Val(_) => {
                    let addr = self.masm.address_at_reg(object_addr, field_offset)?;
                    self.context.pop_to_addr(self.masm, addr)?;
                }
            }
        }

        self.context.free_reg(object_addr);
        Ok(gc_ref)
    }
}
