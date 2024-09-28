//! Compiler for the deferred reference-counting (DRC) collector and its
//! barriers.

use super::{unbarriered_load_gc_ref, unbarriered_store_gc_ref};
use crate::{func_environ::FuncEnvironment, gc::GcCompiler};
use cranelift_codegen::ir::{self, InstBuilder};
use cranelift_frontend::FunctionBuilder;
use cranelift_wasm::TargetEnvironment;
use smallvec::SmallVec;
use wasmtime_environ::{
    drc::DrcTypeLayouts, GcTypeLayouts, PtrSize, TypeIndex, VMGcKind, WasmCompositeType,
    WasmHeapTopType, WasmHeapType, WasmRefType, WasmResult, WasmStorageType, WasmValType,
};

#[derive(Default)]
pub struct DrcCompiler {
    layouts: DrcTypeLayouts,
}

impl DrcCompiler {
    /// Generate code to load the given GC reference's ref count.
    ///
    /// Assumes that the given `gc_ref` is a non-null, non-i31 GC reference.
    fn load_ref_count(
        &mut self,
        func_env: &mut FuncEnvironment<'_>,
        builder: &mut FunctionBuilder,
        gc_ref: ir::Value,
    ) -> ir::Value {
        let offset = func_env.offsets.vm_drc_header_ref_count();
        let size = ir::types::I64.bytes();
        let pointer = func_env.prepare_gc_ref_access(builder, gc_ref, offset, size);
        builder
            .ins()
            .load(ir::types::I64, ir::MemFlags::trusted(), pointer, 0)
    }

    /// Generate code to update the given GC reference's ref count to the new
    /// value.
    ///
    /// Assumes that the given `gc_ref` is a non-null, non-i31 GC reference.
    fn store_ref_count(
        &mut self,
        func_env: &mut FuncEnvironment<'_>,
        builder: &mut FunctionBuilder,
        gc_ref: ir::Value,
        new_ref_count: ir::Value,
    ) {
        let offset = func_env.offsets.vm_drc_header_ref_count();
        let size = ir::types::I64.bytes();
        let pointer = func_env.prepare_gc_ref_access(builder, gc_ref, offset, size);
        builder
            .ins()
            .store(ir::MemFlags::trusted(), new_ref_count, pointer, 0);
    }

    /// Generate code to increment or decrement the given GC reference's ref
    /// count.
    ///
    /// The new ref count is returned.
    ///
    /// Assumes that the given `gc_ref` is a non-null, non-i31 GC reference.
    fn mutate_ref_count(
        &mut self,
        func_env: &mut FuncEnvironment<'_>,
        builder: &mut FunctionBuilder,
        gc_ref: ir::Value,
        delta: i64,
    ) -> ir::Value {
        debug_assert!(delta == -1 || delta == 1);
        let old_ref_count = self.load_ref_count(func_env, builder, gc_ref);
        let new_ref_count = builder.ins().iadd_imm(old_ref_count, delta);
        self.store_ref_count(func_env, builder, gc_ref, new_ref_count);
        new_ref_count
    }

    /// Load the `*mut VMGcRefActivationsTable` from the vmctx, its `next` bump
    /// finger, and its `end` bump boundary.
    fn load_bump_region(
        &mut self,
        func_env: &mut FuncEnvironment<'_>,
        builder: &mut FunctionBuilder,
    ) -> (ir::Value, ir::Value, ir::Value) {
        let ptr_ty = func_env.pointer_type();
        let vmctx = func_env.vmctx(&mut builder.func);
        let vmctx = builder.ins().global_value(ptr_ty, vmctx);
        let activations_table = builder.ins().load(
            ptr_ty,
            ir::MemFlags::trusted(),
            vmctx,
            i32::from(func_env.offsets.ptr.vmctx_gc_heap_data()),
        );
        let next = builder.ins().load(
            ptr_ty,
            ir::MemFlags::trusted(),
            activations_table,
            i32::try_from(func_env.offsets.vm_gc_ref_activation_table_next()).unwrap(),
        );
        let end = builder.ins().load(
            ptr_ty,
            ir::MemFlags::trusted(),
            activations_table,
            i32::try_from(func_env.offsets.vm_gc_ref_activation_table_end()).unwrap(),
        );
        (activations_table, next, end)
    }

    /// Write to an uninitialized GC reference field, initializing it.
    ///
    /// ```text
    /// *dst = new_val
    /// ```
    ///
    /// Doesn't need to do a full write barrier: we don't have an old reference
    /// that is being overwritten and needs its refcount decremented, just a new
    /// reference whose count should be incremented.
    fn translate_init_gc_reference(
        &mut self,
        func_env: &mut FuncEnvironment<'_>,
        builder: &mut FunctionBuilder,
        ty: WasmRefType,
        dst: ir::Value,
        new_val: ir::Value,
        flags: ir::MemFlags,
    ) -> WasmResult<()> {
        let (ref_ty, needs_stack_map) = func_env.reference_type(ty.heap_type);
        debug_assert!(needs_stack_map);

        // Special case for references to uninhabited bottom types: see
        // `translate_write_gc_reference` for details.
        if let WasmHeapType::None = ty.heap_type {
            if ty.nullable {
                let null = builder.ins().iconst(ref_ty, 0);
                builder.ins().store(flags, null, dst, 0);
            } else {
                let zero = builder.ins().iconst(ir::types::I32, 0);
                builder
                    .ins()
                    .trapz(zero, ir::TrapCode::User(crate::DEBUG_ASSERT_TRAP_CODE));
            }
            return Ok(());
        };

        // Special case for `i31ref`s: no need for any barriers.
        if let WasmHeapType::I31 = ty.heap_type {
            return unbarriered_store_gc_ref(builder, ty.heap_type, dst, new_val, flags);
        }

        // Our initialization barrier for GC references being copied out of the
        // stack and initializing a table/global/struct field/etc... is roughly
        // equivalent to the following pseudo-CLIF:
        //
        // ```
        // current_block:
        //     ...
        //     let new_val_is_null_or_i31 = ...
        //     brif new_val_is_null_or_i31, continue_block, inc_ref_block
        //
        // inc_ref_block:
        //     let ref_count = load new_val.ref_count
        //     let new_ref_count = iadd_imm ref_count, 1
        //     store new_val.ref_count, new_ref_count
        //     jump check_old_val_block
        //
        // continue_block:
        //     store dst, new_val
        //     ...
        // ```
        //
        // This write barrier is responsible for ensuring that the new value's
        // ref count is incremented now that the table/global/struct/etc... is
        // holding onto it.

        let current_block = builder.current_block().unwrap();
        let inc_ref_block = builder.create_block();
        let continue_block = builder.create_block();

        builder.ensure_inserted_block();
        builder.insert_block_after(inc_ref_block, current_block);
        builder.insert_block_after(continue_block, inc_ref_block);

        // Current block: check whether the new value is non-null and
        // non-i31. If so, branch to the `inc_ref_block`.
        let new_val_is_null_or_i31 = func_env.gc_ref_is_null_or_i31(builder, ty, new_val);
        builder.ins().brif(
            new_val_is_null_or_i31,
            continue_block,
            &[],
            inc_ref_block,
            &[],
        );

        // Block to increment the ref count of the new value when it is non-null
        // and non-i31.
        builder.switch_to_block(inc_ref_block);
        builder.seal_block(inc_ref_block);
        self.mutate_ref_count(func_env, builder, new_val, 1);
        builder.ins().jump(continue_block, &[]);

        // Join point after we're done with the GC barrier: do the actual store
        // to initialize the field.
        builder.switch_to_block(continue_block);
        builder.seal_block(continue_block);
        unbarriered_store_gc_ref(builder, ty.heap_type, dst, new_val, flags)?;

        Ok(())
    }
}

impl GcCompiler for DrcCompiler {
    fn layouts(&self) -> &dyn GcTypeLayouts {
        &self.layouts
    }

    fn alloc_struct(
        &mut self,
        func_env: &mut FuncEnvironment<'_>,
        builder: &mut FunctionBuilder<'_>,
        struct_type_index: TypeIndex,
        field_vals: &[ir::Value],
    ) -> WasmResult<ir::Value> {
        // First, call the `gc_alloc_raw` builtin libcall to allocate the
        // struct.

        let gc_alloc_raw_builtin = func_env.builtin_functions.gc_alloc_raw(builder.func);
        let vmctx = func_env.vmctx_val(&mut builder.cursor());
        let kind = builder
            .ins()
            .iconst(ir::types::I32, i64::from(VMGcKind::StructRef.as_u32()));

        let interned_type_index = func_env.module.types[struct_type_index];
        let interned_type_index_val = builder
            .ins()
            .iconst(ir::types::I32, i64::from(interned_type_index.as_u32()));

        let struct_layout = func_env.struct_layout(interned_type_index);
        let struct_size = struct_layout.size;
        let field_offsets: SmallVec<[_; 8]> = struct_layout.fields.iter().copied().collect();
        assert_eq!(field_vals.len(), field_offsets.len());

        let size = builder
            .ins()
            .iconst(ir::types::I32, i64::from(struct_layout.size));
        let align = builder
            .ins()
            .iconst(ir::types::I32, i64::from(struct_layout.align));

        let call_inst = builder.ins().call(
            gc_alloc_raw_builtin,
            &[vmctx, kind, interned_type_index_val, size, align],
        );
        let struct_ref = builder.inst_results(call_inst)[0];

        let struct_ty = match &func_env.types[interned_type_index].composite_type {
            WasmCompositeType::Struct(s) => s,
            _ => unreachable!(),
        };
        let field_types: SmallVec<[_; 8]> = struct_ty.fields.iter().cloned().collect();
        assert_eq!(field_vals.len(), field_types.len());

        // Second, initialize each of the newly-allocated struct's fields.

        for ((ty, val), offset) in field_types.into_iter().zip(field_vals).zip(field_offsets) {
            let size_of_access =
                wasmtime_environ::byte_size_of_wasm_ty_in_gc_heap(&ty.element_type);
            assert!(offset + size_of_access <= struct_size);

            let field_addr =
                func_env.prepare_gc_ref_access(builder, struct_ref, offset, size_of_access);

            match &ty.element_type {
                WasmStorageType::Val(WasmValType::Ref(r))
                    if r.heap_type.top() == WasmHeapTopType::Func =>
                {
                    unimplemented!("funcrefs inside the GC heap")
                }
                WasmStorageType::Val(WasmValType::Ref(r)) => {
                    self.translate_init_gc_reference(
                        func_env,
                        builder,
                        *r,
                        field_addr,
                        *val,
                        ir::MemFlags::trusted(),
                    )?;
                }
                WasmStorageType::I8 => {
                    builder
                        .ins()
                        .istore8(ir::MemFlags::trusted(), *val, field_addr, 0);
                }
                WasmStorageType::I16 => {
                    builder
                        .ins()
                        .istore16(ir::MemFlags::trusted(), *val, field_addr, 0);
                }
                WasmStorageType::Val(_) => {
                    assert_eq!(builder.func.dfg.value_type(*val).bytes(), size_of_access);
                    builder
                        .ins()
                        .store(ir::MemFlags::trusted(), *val, field_addr, 0);
                }
            }
        }

        Ok(struct_ref)
    }

    fn translate_read_gc_reference(
        &mut self,
        func_env: &mut FuncEnvironment<'_>,
        builder: &mut FunctionBuilder,
        ty: WasmRefType,
        src: ir::Value,
        flags: ir::MemFlags,
    ) -> WasmResult<ir::Value> {
        assert!(ty.is_vmgcref_type());
        assert!(
            flags.explicit_endianness().is_none(),
            "GC references are always native-endian"
        );

        let (reference_type, needs_stack_map) = func_env.reference_type(ty.heap_type);
        debug_assert!(needs_stack_map);

        // Special case for references to uninhabited bottom types: the
        // reference must either be nullable and we can just eagerly return
        // null, or we are in dynamically unreachable code and should just trap.
        if let WasmHeapType::None = ty.heap_type {
            let null = builder.ins().iconst(reference_type, 0);
            if !ty.nullable {
                // NB: Don't use an unconditional trap instruction, since that
                // is a block terminator, and we still need to integrate with
                // the rest of the surrounding code.
                let zero = builder.ins().iconst(ir::types::I32, 0);
                builder
                    .ins()
                    .trapz(zero, ir::TrapCode::User(crate::DEBUG_ASSERT_TRAP_CODE));
            }
            return Ok(null);
        };

        // Special case for `i31` references: they don't need barriers.
        if let WasmHeapType::I31 = ty.heap_type {
            return unbarriered_load_gc_ref(builder, ty.heap_type, src, flags);
        }

        // Our read barrier for GC references is roughly equivalent to the
        // following pseudo-CLIF:
        //
        // ```
        // current_block:
        //     ...
        //     let gc_ref = load src
        //     let gc_ref_is_null = is_null gc_ref
        //     let gc_ref_is_i31 = ...
        //     let gc_ref_is_null_or_i31 = bor gc_ref_is_null, gc_ref_is_i31
        //     brif gc_ref_is_null_or_i31, continue_block, non_null_gc_ref_block
        //
        // non_null_gc_ref_block:
        //     let (next, end) = load VMGcRefActivationsTable bump region
        //     let bump_region_is_full = icmp eq next, end
        //     brif bump_region_is_full, gc_block, no_gc_block
        //
        // no_gc_block:
        //     let ref_count = load gc_ref.ref_count
        //     let new_ref_count = iadd_imm ref_count, 1
        //     store new_ref_count, gc_ref.ref_count
        //     let new_next = iadd_imm next, size_of(reference_type)
        //     store new_next, activations_table.next
        //     jump continue_block
        //
        // cold gc_block:
        //     ;; NB: The DRC collector is not a moving GC, so we can reuse
        //     ;; `gc_ref`. This lets us avoid a block parameter for the
        //     ;; `continue_block`.
        //     let _moved_gc_ref = call gc(gc_ref)
        //     jump continue_block
        //
        // continue_block:
        //     ...
        // ```
        //
        // This ensures that all GC references entering the Wasm stack are held
        // alive by the `VMGcRefActivationsTable`.

        let current_block = builder.current_block().unwrap();
        let non_null_gc_ref_block = builder.create_block();
        let gc_block = builder.create_block();
        let no_gc_block = builder.create_block();
        let continue_block = builder.create_block();

        builder.set_cold_block(gc_block);
        builder.ensure_inserted_block();
        builder.insert_block_after(non_null_gc_ref_block, current_block);
        builder.insert_block_after(no_gc_block, non_null_gc_ref_block);
        builder.insert_block_after(gc_block, no_gc_block);
        builder.insert_block_after(continue_block, gc_block);

        // Load the GC reference and check for null/i31.
        let gc_ref = unbarriered_load_gc_ref(builder, ty.heap_type, src, flags)?;
        let gc_ref_is_null_or_i31 = func_env.gc_ref_is_null_or_i31(builder, ty, gc_ref);
        builder.ins().brif(
            gc_ref_is_null_or_i31,
            continue_block,
            &[],
            non_null_gc_ref_block,
            &[],
        );

        // Block for when the GC reference is not null and is not an `i31ref`.
        //
        // Load the `VMGcRefActivationsTable::next` bump finger and the
        // `VMGcRefActivationsTable::end` bump boundary and check whether the
        // bump region is full or not.
        builder.switch_to_block(non_null_gc_ref_block);
        builder.seal_block(non_null_gc_ref_block);
        let (activations_table, next, end) = self.load_bump_region(func_env, builder);
        let bump_region_is_full = builder.ins().icmp(ir::condcodes::IntCC::Equal, next, end);
        builder
            .ins()
            .brif(bump_region_is_full, gc_block, &[], no_gc_block, &[]);

        // Block for when the bump region is not full. We should:
        //
        // * increment this reference's ref count,
        // * store the reference into the bump table at `*next`,
        // * and finally increment the `next` bump finger.
        builder.switch_to_block(no_gc_block);
        builder.seal_block(no_gc_block);
        self.mutate_ref_count(func_env, builder, gc_ref, 1);
        builder
            .ins()
            .store(ir::MemFlags::trusted(), gc_ref, next, 0);
        let new_next = builder
            .ins()
            .iadd_imm(next, i64::from(reference_type.bytes()));
        builder.ins().store(
            ir::MemFlags::trusted(),
            new_next,
            activations_table,
            i32::try_from(func_env.offsets.vm_gc_ref_activation_table_next()).unwrap(),
        );
        builder.ins().jump(continue_block, &[]);

        // Block for when the bump region is full and we need to do a GC.
        builder.switch_to_block(gc_block);
        builder.seal_block(gc_block);
        let gc_libcall = func_env.builtin_functions.gc(builder.func);
        let vmctx = func_env.vmctx_val(&mut builder.cursor());
        builder.ins().call(gc_libcall, &[vmctx, gc_ref]);
        builder.ins().jump(continue_block, &[]);

        // Join point after we're done with the GC barrier.
        builder.switch_to_block(continue_block);
        builder.seal_block(continue_block);
        Ok(gc_ref)
    }

    fn translate_write_gc_reference(
        &mut self,
        func_env: &mut FuncEnvironment<'_>,
        builder: &mut FunctionBuilder,
        ty: WasmRefType,
        dst: ir::Value,
        new_val: ir::Value,
        flags: ir::MemFlags,
    ) -> WasmResult<()> {
        assert!(ty.is_vmgcref_type());
        assert!(
            flags.explicit_endianness().is_none(),
            "GC references are always native-endian"
        );

        let (ref_ty, needs_stack_map) = func_env.reference_type(ty.heap_type);
        debug_assert!(needs_stack_map);

        // Special case for references to uninhabited bottom types: either the
        // reference must either be nullable and we can just eagerly store null
        // into `dst`, or we are in unreachable code and should just trap.
        if let WasmHeapType::None = ty.heap_type {
            if ty.nullable {
                let null = builder.ins().iconst(ref_ty, 0);
                builder.ins().store(flags, null, dst, 0);
            } else {
                // NB: Don't use an unconditional trap instruction, since that
                // is a block terminator, and we still need to integrate with
                // the rest of the surrounding code.
                let zero = builder.ins().iconst(ir::types::I32, 0);
                builder
                    .ins()
                    .trapz(zero, ir::TrapCode::User(crate::DEBUG_ASSERT_TRAP_CODE));
            }
            return Ok(());
        };

        // Special case for `i31` references: they don't need barriers.
        if let WasmHeapType::I31 = ty.heap_type {
            return unbarriered_store_gc_ref(builder, ty.heap_type, dst, new_val, flags);
        }

        // Our write barrier for GC references being copied out of the stack and
        // written into a table/global/etc... is roughly equivalent to the
        // following pseudo-CLIF:
        //
        // ```
        // current_block:
        //     ...
        //     let old_val = *dst
        //     let new_val_is_null = ref.null new_val
        //     let new_val_is_i31 = ...
        //     let new_val_is_null_or_i31 = bor new_val_is_null, new_val_is_i31
        //     brif new_val_is_null_or_i31, check_old_val_block, inc_ref_block
        //
        // inc_ref_block:
        //     let ref_count = load new_val.ref_count
        //     let new_ref_count = iadd_imm ref_count, 1
        //     store new_val.ref_count, new_ref_count
        //     jump check_old_val_block
        //
        // check_old_val_block:
        //     store dst, new_val
        //     let old_val_is_null = ref.null old_val
        //     let old_val_is_i31 = ...
        //     let old_val_is_null_or_i31 = bor old_val_is_null, old_val_is_i31
        //     brif old_val_is_null_or_i31, continue_block, dec_ref_block
        //
        // dec_ref_block:
        //     let ref_count = load old_val.ref_count
        //     let new_ref_count = isub_imm ref_count, 1
        //     let old_val_needs_drop = icmp_imm eq new_ref_count, 0
        //     brif old_val_needs_drop, drop_old_val_block, store_dec_ref_block
        //
        // cold drop_old_val_block:
        //     call drop_gc_ref(old_val)
        //     jump continue_block
        //
        // store_dec_ref_block:
        //     store old_val.ref_count, new_ref_count
        //     jump continue_block
        //
        // continue_block:
        //     ...
        // ```
        //
        // This write barrier is responsible for ensuring that:
        //
        // 1. The new value's ref count is incremented now that the table is
        //    holding onto it.
        //
        // 2. The old value's ref count is decremented, and that it is dropped
        //    if the ref count reaches zero.
        //
        // We must do the increment before the decrement. If we did it in the
        // other order, then when `*dst == new_val`, we could confuse ourselves
        // by observing a zero ref count after the decrement but before it would
        // become non-zero again with the subsequent increment.
        //
        // Additionally, we take care that we don't ever call out-out-of-line to
        // drop the old value until all the new value has been written into
        // `dst` and its reference count has been updated. This makes sure that
        // host code has a consistent view of the world.

        let current_block = builder.current_block().unwrap();
        let inc_ref_block = builder.create_block();
        let check_old_val_block = builder.create_block();
        let dec_ref_block = builder.create_block();
        let drop_old_val_block = builder.create_block();
        let store_dec_ref_block = builder.create_block();
        let continue_block = builder.create_block();

        builder.ensure_inserted_block();
        builder.set_cold_block(drop_old_val_block);

        builder.insert_block_after(inc_ref_block, current_block);
        builder.insert_block_after(check_old_val_block, inc_ref_block);
        builder.insert_block_after(dec_ref_block, check_old_val_block);
        builder.insert_block_after(drop_old_val_block, dec_ref_block);
        builder.insert_block_after(store_dec_ref_block, drop_old_val_block);
        builder.insert_block_after(continue_block, store_dec_ref_block);

        // Load the old value and then check whether the new value is non-null
        // and non-i31.
        let old_val = unbarriered_load_gc_ref(builder, ty.heap_type, dst, flags)?;
        let new_val_is_null_or_i31 = func_env.gc_ref_is_null_or_i31(builder, ty, new_val);
        builder.ins().brif(
            new_val_is_null_or_i31,
            check_old_val_block,
            &[],
            inc_ref_block,
            &[],
        );

        // Block to increment the ref count of the new value when it is non-null
        // and non-i31.
        builder.switch_to_block(inc_ref_block);
        builder.seal_block(inc_ref_block);
        self.mutate_ref_count(func_env, builder, new_val, 1);
        builder.ins().jump(check_old_val_block, &[]);

        // Block to store the new value into `dst` and then check whether the
        // old value is non-null and non-i31 and therefore needs its ref count
        // decremented.
        builder.switch_to_block(check_old_val_block);
        builder.seal_block(check_old_val_block);
        unbarriered_store_gc_ref(builder, ty.heap_type, dst, new_val, flags)?;
        let old_val_is_null_or_i31 = func_env.gc_ref_is_null_or_i31(builder, ty, old_val);
        builder.ins().brif(
            old_val_is_null_or_i31,
            continue_block,
            &[],
            dec_ref_block,
            &[],
        );

        // Block to decrement the ref count of the old value when it is non-null
        // and non-i31.
        builder.switch_to_block(dec_ref_block);
        builder.seal_block(dec_ref_block);
        let ref_count = self.load_ref_count(func_env, builder, old_val);
        let new_ref_count = builder.ins().iadd_imm(ref_count, -1);
        let old_val_needs_drop =
            builder
                .ins()
                .icmp_imm(ir::condcodes::IntCC::Equal, new_ref_count, 0);
        builder.ins().brif(
            old_val_needs_drop,
            drop_old_val_block,
            &[],
            store_dec_ref_block,
            &[],
        );

        // Block to call out-of-line to drop a GC reference when its ref count
        // reaches zero.
        //
        // Note that this libcall does its own dec-ref operation, so we only
        // actually store `new_ref_count` back to the `old_val` object when
        // `new_ref_count != 0`.
        builder.switch_to_block(drop_old_val_block);
        builder.seal_block(drop_old_val_block);
        let drop_gc_ref_libcall = func_env.builtin_functions.drop_gc_ref(builder.func);
        let vmctx = func_env.vmctx_val(&mut builder.cursor());
        builder.ins().call(drop_gc_ref_libcall, &[vmctx, old_val]);
        builder.ins().jump(continue_block, &[]);

        // Block to store the new ref count back to `old_val` for when
        // `new_ref_count != 0`, as explained above.
        builder.switch_to_block(store_dec_ref_block);
        builder.seal_block(store_dec_ref_block);
        self.store_ref_count(func_env, builder, old_val, new_ref_count);
        builder.ins().jump(continue_block, &[]);

        // Join point after we're done with the GC barrier.
        builder.switch_to_block(continue_block);
        builder.seal_block(continue_block);
        Ok(())
    }
}
