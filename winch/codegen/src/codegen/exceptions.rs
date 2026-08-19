use super::{Callee, CodeGen, CodeGenError, ControlStackFrame, Emission, FnCall};
use crate::{
    Result,
    codegen::{UnconditionalBranch, control_index},
    ensure, format_err,
    masm::{IntScratch, MacroAssembler, OperandSize, RegImm},
    reg::{Reg, writable},
    stack::{TypedReg, Val},
};
use cranelift_codegen::{MachExceptionHandler, MachLabel, ir::ExceptionTag};
use smallvec::SmallVec;
use wasmtime_environ::{
    Collector, GcStructLayout, GcTypeLayouts, ModuleInternedTypeIndex, PtrSize, TagIndex, VMGcKind,
    WasmExnType, WasmHeapType, WasmStorageType, WasmValType, packed_option::ReservedValue,
};
use wasmtime_environ::{WasmCompositeInnerType, copying::InlineTraceInfo};

/// The exception handlers that are currently in scope.
#[derive(Default)]
pub(crate) struct HandlerState {
    handlers: Vec<(Option<ExceptionTag>, MachLabel)>,
}

/// A checkpoint that can restore the exception handlers in scope.
#[derive(Clone, Copy, Debug)]
pub(crate) struct HandlerStateCheckpoint(usize);

#[derive(Debug)]
pub(crate) struct CatchInfo {
    pub(crate) tag: Option<TagIndex>,
    pub(crate) target_depth: u32,
    pub(crate) landing_pad: MachLabel,
}

#[derive(Debug)]
pub(crate) struct TryTableInfo {
    pub(crate) checkpoint: HandlerStateCheckpoint,
    pub(crate) catches: Vec<CatchInfo>,
}

impl HandlerState {
    /// Adds an exception handler.
    pub(crate) fn add_handler(&mut self, tag: Option<ExceptionTag>, label: MachLabel) {
        self.handlers.push((tag, label));
    }

    /// Takes a checkpoint of the exception handlers currently in scope.
    pub(crate) fn take_checkpoint(&self) -> HandlerStateCheckpoint {
        HandlerStateCheckpoint(self.handlers.len())
    }

    /// Restores the exception handlers to a previous checkpoint.
    pub(crate) fn restore_checkpoint(&mut self, checkpoint: HandlerStateCheckpoint) {
        assert!(checkpoint.0 <= self.handlers.len());
        self.handlers.truncate(checkpoint.0);
    }

    /// Iterates over exception handlers from the innermost to the outermost.
    pub(crate) fn handlers(&self) -> impl Iterator<Item = MachExceptionHandler> + '_ {
        self.handlers
            .iter()
            .copied()
            .rev()
            .map(|(tag, label)| match tag {
                Some(tag) => MachExceptionHandler::Tag(tag, label),
                None => MachExceptionHandler::Default(label),
            })
    }

    /// Returns whether there are no exception handlers in scope.
    pub(crate) fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }
}

impl<'a, 'translation, 'data, M> CodeGen<'a, 'translation, 'data, M, Emission>
where
    M: MacroAssembler,
{
    /// Emits the end of a try-table block and its exception landing pads.
    pub(crate) fn emit_try_table_end(
        &mut self,
        mut control: ControlStackFrame,
        info: TryTableInfo,
    ) -> Result<()> {
        let stack_state = *control.stack_state();

        let fallthrough_reachable = self.context.reachable;
        let end_reachable = fallthrough_reachable || control.is_next_sequence_reachable();

        if fallthrough_reachable {
            ensure!(
                control.stack_state().target_len == self.context.stack.len(),
                CodeGenError::control_frame_state_mismatch()
            );

            control.pop_abi_results(&mut self.context, self.masm, |results, _, _| {
                Ok(results.ret_area().copied())
            })?;

            self.masm.jmp(*control.label())?;
        }

        for catch in info.catches {
            self.masm.bind(catch.landing_pad)?;

            self.context.reachable = true;
            let exception_reg = self
                .masm
                .prepare_for_exception_handler(stack_state.base_offset)?;
            self.context.truncate_stack_to(stack_state.base_len)?;
            self.context.load_vmctx(self.masm)?;

            if let Some(tag) = catch.tag {
                let exception_reg = self.context.reg(exception_reg, self.masm)?;
                self.emit_load_exception_payload_fields(tag, exception_reg)?;
            }
            self.emit_catch_branch(catch.target_depth)?;
        }

        self.context.reachable = end_reachable;

        if end_reachable {
            if !fallthrough_reachable {
                control.ensure_stack_state(self.masm, &mut self.context)?;
            }
            control.bind_end(self.masm, &mut self.context)
        } else {
            Ok(())
        }
    }

    fn emit_catch_branch(&mut self, target_depth: u32) -> Result<()> {
        let index = control_index(target_depth, self.control_frames.len())?;
        let frame = &mut self.control_frames[index];

        self.context
            .br::<_, _, UnconditionalBranch>(frame, self.masm, |masm, context, frame| {
                frame.pop_abi_results::<M, _>(context, masm, |results, _, _| {
                    Ok(results.ret_area().copied())
                })
            })
    }

    fn emit_load_exception_payload_fields(
        &mut self,
        tag_index: TagIndex,
        mut exception_reg: Reg,
    ) -> Result<()> {
        let interned = self.env.translation.module.tags[tag_index]
            .exception
            .unwrap_module_type_index();

        let exn_ty = match &self.env.types[interned].composite_type.inner {
            WasmCompositeInnerType::Exn(exn_ty) => exn_ty,
            _ => return Err(format_err!(CodeGenError::unsupported_wasm_type())),
        };
        let gc_codegen_config = self.require_gc_codegen_config();
        let layouts = gc_codegen_config.layouts();

        let layout = layouts
            .exn_layout(exn_ty)
            .map_err(|_| format_err!(CodeGenError::unsupported_wasm_type()))?;

        let fields: SmallVec<[(WasmStorageType, u32); 8]> = exn_ty
            .fields
            .iter()
            .zip(layout.fields.iter())
            .map(|(field_ty, field_layout)| (field_ty.element_type, field_layout.offset))
            .collect();

        let (heap_base, heap_bound) = self.emit_load_gc_heap_base_and_bound()?;

        self.emit_gc_ref_bounds_check(exception_reg, heap_bound, i64::from(layout.size))?;

        self.context.free_reg(heap_bound);

        let mut object_addr = Some(self.emit_gc_ref_addr(exception_reg, heap_base)?);
        self.context.free_reg(heap_base);
        for (field_ty, field_offset) in fields {
            let field_base = match object_addr {
                Some(reg) => reg,
                None => {
                    let (heap_base, heap_bound) = self.emit_load_gc_heap_base_and_bound()?;
                    self.context.free_reg(heap_bound);
                    let object_addr = self.emit_gc_ref_addr(exception_reg, heap_base)?;
                    self.context.free_reg(heap_base);
                    object_addr
                }
            };
            object_addr = Some(field_base);

            let ty = match field_ty {
                WasmStorageType::Val(ty @ WasmValType::Ref(r))
                    if r.heap_type == WasmHeapType::Func =>
                {
                    let func_ref_id = self.context.any_gpr(self.masm)?;
                    let addr = self.masm.address_at_reg(field_base, field_offset)?;
                    self.masm
                        .load(addr, writable!(func_ref_id), OperandSize::S32)?;

                    // The builtin call can clobber allocated registers. Preserve
                    // the exception and its object address beneath the call's
                    // arguments so later payload fields can still be loaded.
                    self.context.stack.push(TypedReg::i64(field_base).into());
                    self.context.stack.push(TypedReg::i32(exception_reg).into());
                    self.context.stack.push(TypedReg::i32(func_ref_id).into());
                    self.context.stack.push(Val::i32(
                        ModuleInternedTypeIndex::reserved_value()
                            .as_bits()
                            .cast_signed(),
                    ));

                    let get = self.env.builtins.get_interned_func_ref::<M::ABI>()?;
                    FnCall::emit::<M>(
                        &mut self.env,
                        self.masm,
                        &mut self.context,
                        Callee::Builtin(get),
                    )?;

                    let func_ref = self.context.pop_to_reg(self.masm, None)?;
                    exception_reg = self.context.pop_to_reg(self.masm, None)?.reg;
                    object_addr = Some(self.context.pop_to_reg(self.masm, None)?.reg);
                    self.context
                        .stack
                        .push(TypedReg::new(ty, func_ref.reg).into());
                    continue;
                }
                WasmStorageType::Val(ty @ WasmValType::Ref(r))
                    if r.heap_type == WasmHeapType::Extern =>
                {
                    let addr = self.masm.address_at_reg(field_base, field_offset)?;
                    if gc_codegen_config.collector() == Collector::DeferredReferenceCounting {
                        // The DRC read barrier can make an out-of-line call and
                        // consumes the field's base register. Preserve the
                        // exception so a later field can recompute its address.
                        // The runtime rooted the exception when transferring it
                        // to this handler; this stack value only preserves its
                        // raw bits across the call.
                        self.context.stack.push(TypedReg::i32(exception_reg).into());
                        self.emit_drc_read_barrier(ty, field_base, addr)?;
                        let payload = self.context.pop_to_reg(self.masm, None)?;
                        exception_reg = self.context.pop_to_reg(self.masm, None)?.reg;
                        self.context.stack.push(payload.into());
                        object_addr = None;
                    } else {
                        let value = self.context.reg_for_type(ty, self.masm)?;
                        self.masm.load(addr, writable!(value), ty.try_into()?)?;
                        self.context.stack.push(TypedReg::new(ty, value).into());
                    }
                    continue;
                }
                WasmStorageType::Val(WasmValType::Ref(_)) => {
                    return Err(format_err!(CodeGenError::unsupported_wasm_type()));
                }
                WasmStorageType::Val(ty) => ty,
                WasmStorageType::I8 | WasmStorageType::I16 => {
                    return Err(format_err!(CodeGenError::unsupported_wasm_type()));
                }
            };

            let value = self.context.reg_for_type(ty, self.masm)?;
            let addr = self.masm.address_at_reg(field_base, field_offset)?;

            self.masm.load(addr, writable!(value), ty.try_into()?)?;

            self.context.stack.push(TypedReg::new(ty, value).into());
        }

        if let Some(object_addr) = object_addr {
            self.context.free_reg(object_addr);
        }
        self.context.free_reg(exception_reg);
        Ok(())
    }

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
        let (gc_ref, object_addr) =
            self.emit_gc_raw_alloc(VMGcKind::ExnRef, interned, &layout.layout(), reserved_bits)?;

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
    pub(crate) fn emit_store_exception_payload_fields(
        &mut self,
        exn_ty: &WasmExnType,
        layout: &GcStructLayout,
        mut gc_ref: TypedReg,
        mut object_addr: Reg,
    ) -> Result<TypedReg> {
        assert_eq!(exn_ty.fields.len(), layout.fields.len());

        for (field, field_layout) in exn_ty.fields.iter().zip(layout.fields.iter()).rev() {
            let field_ty = field.element_type;
            let field_offset = field_layout.offset;
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
                WasmStorageType::Val(ty @ WasmValType::Ref(r)) => match r.heap_type {
                    WasmHeapType::Func => {
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
                    WasmHeapType::Extern
                        if self.tunables.collector
                            == Some(Collector::DeferredReferenceCounting) =>
                    {
                        let addr = self.masm.address_at_reg(object_addr, field_offset)?;
                        self.emit_drc_init_barrier(ty, addr)?;
                    }
                    WasmHeapType::Extern => {
                        let addr = self.masm.address_at_reg(object_addr, field_offset)?;
                        self.context.pop_to_addr(self.masm, addr)?;
                    }
                    _ => return Err(format_err!(CodeGenError::unsupported_wasm_type())),
                },
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
