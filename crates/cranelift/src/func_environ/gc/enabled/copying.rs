//! Compiler for the copying (semi-space/Cheney) collector.
//!
//! Allocation is performed inline with a bump pointer when possible, falling
//! back to the `gc_alloc_raw` libcall when the active semi-space is full.
//! Read and write barriers are unnecessary (e.g. no reference counting and
//! no concurrent mutation during collection) but we do need stack maps so the
//! collector can find and update roots when it relocates objects.

use super::*;
use crate::TRAP_INTERNAL_ASSERT;
use crate::func_environ::FuncEnvironment;
use crate::translate::TargetEnvironment;
use cranelift_codegen::ir::{self, InstBuilder};
use cranelift_frontend::FunctionBuilder;
use wasmtime_environ::copying::{
    ALIGN, EXCEPTION_TAG_DEFINED_OFFSET, EXCEPTION_TAG_INSTANCE_OFFSET,
};
use wasmtime_environ::{
    GcTypeLayouts, ModuleInternedTypeIndex, PtrSize, TypeIndex, VMGcKind, WasmHeapTopType,
    WasmHeapType, WasmRefType, WasmResult, WasmStorageType, WasmValType,
    copying::CopyingTypeLayouts,
};

#[derive(Default)]
pub struct CopyingCompiler {
    layouts: CopyingTypeLayouts,
}

impl CopyingCompiler {
    /// Load the pointer to the `VMCopyingHeapData` from vmctx.
    fn load_vmcopying_heap_data_ptr(
        func_env: &mut FuncEnvironment<'_>,
        builder: &mut FunctionBuilder,
    ) -> ir::Value {
        let pointer_type = func_env.pointer_type();
        let vmctx = func_env.vmctx_val(&mut builder.cursor());
        builder.ins().load(
            pointer_type,
            ir::MemFlagsData::trusted().with_readonly().with_can_move(),
            vmctx,
            i32::from(func_env.offsets.ptr.vmctx_gc_heap_data()),
        )
    }

    /// Load the current bump pointer and active-space end from a `*mut
    /// VMCopyingHeapData`.
    ///
    /// Returns `(bump_ptr, active_space_end)` as `i32` values.
    fn load_bump_state(
        func_env: &mut FuncEnvironment<'_>,
        builder: &mut FunctionBuilder,
        ptr_to_heap_data: ir::Value,
    ) -> (ir::Value, ir::Value) {
        let bump_ptr = builder.ins().load(
            ir::types::I32,
            ir::MemFlagsData::trusted().with_can_move(),
            ptr_to_heap_data,
            i32::from(func_env.offsets.ptr.vmcopying_heap_data_bump_ptr()),
        );
        let active_space_end = builder.ins().load(
            ir::types::I32,
            ir::MemFlagsData::trusted().with_readonly().with_can_move(),
            ptr_to_heap_data,
            i32::from(func_env.offsets.ptr.vmcopying_heap_data_active_space_end()),
        );
        (bump_ptr, active_space_end)
    }

    /// Round `size` (an `i32`) up to `ALIGN`, returning the result as an `i64`.
    ///
    /// Uses `i64` arithmetic so that overflow produces a value larger than any
    /// valid heap index, which sends us to the slow allocation path instead of
    /// wrapping around.
    fn aligned_size(builder: &mut FunctionBuilder, size: ir::Value) -> ir::Value {
        let size_64 = builder.ins().uextend(ir::types::I64, size);
        let align_mask = builder.ins().iconst(ir::types::I64, i64::from(ALIGN - 1));
        let inv_align_mask = builder.ins().iconst(ir::types::I64, !i64::from(ALIGN - 1));
        let size_plus_mask = builder.ins().iadd(size_64, align_mask);
        builder.ins().band(size_plus_mask, inv_align_mask)
    }

    /// Emit inline bump allocation, falling back to `gc_alloc_raw` on failure.
    ///
    /// `size` must be an `i32` value >= `size_of(VMCopyingHeader)`.
    ///
    /// Returns `(gc_ref, raw_ptr_to_object)` where `gc_ref` is the `i32` GC
    /// heap index and `raw_ptr_to_object` is a native pointer into the GC heap.
    fn emit_inline_alloc(
        &mut self,
        func_env: &mut FuncEnvironment<'_>,
        builder: &mut FunctionBuilder,
        kind: VMGcKind,
        ty: ModuleInternedTypeIndex,
        size: ir::Value,
    ) -> (ir::Value, ir::Value) {
        debug_assert_ne!(kind, VMGcKind::ExternRef);
        debug_assert!(!ty.is_reserved_value());
        assert_eq!(builder.func.dfg.value_type(size), ir::types::I32);

        let pointer_type = func_env.pointer_type();
        let current_block = builder.current_block().unwrap();
        let fast_block = builder.create_block();
        let slow_block = builder.create_block();
        let merge_block = builder.create_block();

        builder.ensure_inserted_block();
        builder.insert_block_after(fast_block, current_block);
        builder.insert_block_after(slow_block, fast_block);
        builder.insert_block_after(merge_block, slow_block);

        let ptr_to_heap_data = Self::load_vmcopying_heap_data_ptr(func_env, builder);
        let (bump_ptr, active_space_end) =
            Self::load_bump_state(func_env, builder, ptr_to_heap_data);
        let aligned_size_64 = Self::aligned_size(builder, size);

        // Compute `end_of_object = bump_ptr + aligned_size` (in i64) and check
        // whether it fits within the active semi-space.
        let bump_ptr_64 = builder.ins().uextend(ir::types::I64, bump_ptr);
        let end_64 = builder.ins().iadd(bump_ptr_64, aligned_size_64);
        let active_space_end_64 = builder.ins().uextend(ir::types::I64, active_space_end);
        let fits = builder.ins().icmp(
            ir::condcodes::IntCC::UnsignedLessThanOrEqual,
            end_64,
            active_space_end_64,
        );
        builder.ins().brif(fits, fast_block, &[], slow_block, &[]);

        // Slow path: when there isn't enough room in the bump region, call the
        // `gc_alloc_raw` libcall, which will collect or grow the GC heap as
        // necessary.
        {
            builder.switch_to_block(slow_block);
            builder.seal_block(slow_block);
            builder.set_cold_block(slow_block);
            let gc_ref = emit_gc_raw_alloc(func_env, builder, kind, ty, size, ALIGN);
            let base = func_env.get_gc_heap_base(builder);
            let heap_offset = uextend_i32_to_pointer_type(builder, pointer_type, gc_ref);
            let obj_ptr = builder.ins().iadd(base, heap_offset);
            builder
                .ins()
                .jump(merge_block, &[gc_ref.into(), obj_ptr.into()]);
        }

        // Fast path: there is capacity for the requested object in the bump
        // region, so finish the allocation inline, update our bump pointer,
        // etc...
        {
            builder.switch_to_block(fast_block);
            builder.seal_block(fast_block);

            // The old bump_ptr is the start of the new object.
            let gc_ref = bump_ptr;

            // Update the bump pointer.
            let end_of_object = builder.ins().ireduce(ir::types::I32, end_64);
            builder.ins().store(
                ir::MemFlagsData::trusted().with_alias_region(Some(ir::AliasRegion::Vmctx)),
                end_of_object,
                ptr_to_heap_data,
                i32::from(func_env.offsets.ptr.vmcopying_heap_data_bump_ptr()),
            );

            // Compute the raw pointer to the new object.
            let base = func_env.get_gc_heap_base(builder);
            let heap_offset = uextend_i32_to_pointer_type(builder, pointer_type, gc_ref);
            let obj_ptr = builder.ins().iadd(base, heap_offset);

            // Write `VMGcHeader::kind`.
            let kind_val = builder
                .ins()
                .iconst(ir::types::I32, i64::from(kind.as_u32()));
            builder.ins().store(
                ir::MemFlagsData::trusted(),
                kind_val,
                obj_ptr,
                i32::try_from(wasmtime_environ::VM_GC_HEADER_KIND_OFFSET).unwrap(),
            );

            // Write `VMGcHeader::type_index`.
            let shared_ty = func_env.module_interned_to_shared_ty(&mut builder.cursor(), ty);
            builder.ins().store(
                ir::MemFlagsData::trusted(),
                shared_ty,
                obj_ptr,
                i32::try_from(wasmtime_environ::VM_GC_HEADER_TYPE_INDEX_OFFSET).unwrap(),
            );

            // Write `VMCopyingHeader::object_size`.
            builder.ins().istore32(
                ir::MemFlagsData::trusted(),
                aligned_size_64,
                obj_ptr,
                i32::try_from(wasmtime_environ::VM_GC_HEADER_SIZE).unwrap(),
            );

            builder
                .ins()
                .jump(merge_block, &[gc_ref.into(), obj_ptr.into()]);
        }

        // Merge block: takes the GC ref and the raw pointer to the GC object as
        // block parameters.
        builder.switch_to_block(merge_block);
        let gc_ref = builder.append_block_param(merge_block, ir::types::I32);
        let ptr_to_object = builder.append_block_param(merge_block, pointer_type);
        builder.seal_block(merge_block);
        builder.declare_value_needs_stack_map(gc_ref);

        (gc_ref, ptr_to_object)
    }

    fn init_field(
        &mut self,
        func_env: &mut FuncEnvironment<'_>,
        builder: &mut FunctionBuilder<'_>,
        field_addr: ir::Value,
        ty: WasmStorageType,
        val: ir::Value,
    ) -> WasmResult<()> {
        // Data inside GC objects is always little endian.
        let flags = GC_MEMFLAGS.with_endianness(ir::Endianness::Little);

        match ty {
            WasmStorageType::Val(WasmValType::Ref(r)) => match r.heap_type.top() {
                WasmHeapTopType::Func => {
                    write_func_ref_at_addr(func_env, builder, r, flags, field_addr, val)?
                }
                WasmHeapTopType::Extern | WasmHeapTopType::Any | WasmHeapTopType::Exn => {
                    // No init barrier needed for the copying collector; just
                    // store the reference directly.
                    unbarriered_store_gc_ref(builder, r.heap_type, field_addr, val, flags)?;
                }
                WasmHeapTopType::Cont => return super::stack_switching_unsupported(),
            },
            WasmStorageType::I8 => {
                assert_eq!(builder.func.dfg.value_type(val), ir::types::I32);
                builder.ins().istore8(flags, val, field_addr, 0);
            }
            WasmStorageType::I16 => {
                assert_eq!(builder.func.dfg.value_type(val), ir::types::I32);
                builder.ins().istore16(flags, val, field_addr, 0);
            }
            WasmStorageType::Val(_) => {
                let size_of_access = wasmtime_environ::byte_size_of_wasm_ty_in_gc_heap(&ty);
                assert_eq!(builder.func.dfg.value_type(val).bytes(), size_of_access);
                builder.ins().store(flags, val, field_addr, 0);
            }
        }

        Ok(())
    }
}

impl GcCompiler for CopyingCompiler {
    fn layouts(&self) -> &dyn GcTypeLayouts {
        &self.layouts
    }

    fn alloc_array(
        &mut self,
        func_env: &mut FuncEnvironment<'_>,
        builder: &mut FunctionBuilder<'_>,
        array_type_index: TypeIndex,
        init: super::ArrayInit<'_>,
    ) -> WasmResult<ir::Value> {
        let interned_type_index =
            func_env.module.types[array_type_index].unwrap_module_type_index();
        let ptr_ty = func_env.pointer_type();

        let len_offset = gc_compiler(func_env)?.layouts().array_length_field_offset();
        let array_layout = func_env.array_layout(interned_type_index).clone();
        let base_size = array_layout.base_size;
        let len_to_elems_delta = base_size.checked_sub(len_offset).unwrap();

        // First, compute the array's total size.
        let len = init.len(&mut builder.cursor());
        let size = emit_array_size(func_env, builder, &array_layout, len);

        // Allocate inline (with fallback to libcall).
        let (array_ref, object_addr) = self.emit_inline_alloc(
            func_env,
            builder,
            VMGcKind::ArrayRef,
            interned_type_index,
            size,
        );
        let len_addr = builder.ins().iadd_imm(object_addr, i64::from(len_offset));
        let len = init.len(&mut builder.cursor());
        builder.ins().store(GC_MEMFLAGS, len, len_addr, 0);

        // Initialize elements.
        let len_to_elems_delta = builder.ins().iconst(ptr_ty, i64::from(len_to_elems_delta));
        let elems_addr = builder.ins().iadd(len_addr, len_to_elems_delta);
        init.initialize(
            func_env,
            builder,
            interned_type_index,
            base_size,
            size,
            elems_addr,
            |func_env, builder, elem_ty, elem_addr, val| {
                self.init_field(func_env, builder, elem_addr, elem_ty, val)
            },
        )?;
        Ok(array_ref)
    }

    fn alloc_struct(
        &mut self,
        func_env: &mut FuncEnvironment<'_>,
        builder: &mut FunctionBuilder<'_>,
        struct_type_index: TypeIndex,
        field_vals: &[ir::Value],
    ) -> WasmResult<ir::Value> {
        let interned_type_index =
            func_env.module.types[struct_type_index].unwrap_module_type_index();
        let struct_layout = func_env.struct_or_exn_layout(interned_type_index);

        let struct_size = struct_layout.size;

        let struct_size_val = builder.ins().iconst(ir::types::I32, i64::from(struct_size));

        let (struct_ref, raw_ptr_to_struct) = self.emit_inline_alloc(
            func_env,
            builder,
            VMGcKind::StructRef,
            interned_type_index,
            struct_size_val,
        );

        // Initialize fields.
        initialize_struct_fields(
            func_env,
            builder,
            interned_type_index,
            raw_ptr_to_struct,
            field_vals,
            |func_env, builder, ty, field_addr, val| {
                self.init_field(func_env, builder, field_addr, ty, val)
            },
        )?;

        Ok(struct_ref)
    }

    fn alloc_exn(
        &mut self,
        func_env: &mut FuncEnvironment<'_>,
        builder: &mut FunctionBuilder<'_>,
        tag_index: TagIndex,
        field_vals: &[ir::Value],
        instance_id: ir::Value,
        tag: ir::Value,
    ) -> WasmResult<ir::Value> {
        let interned_type_index = func_env.module.tags[tag_index]
            .exception
            .unwrap_module_type_index();
        let exn_layout = func_env.struct_or_exn_layout(interned_type_index);

        let exn_size = exn_layout.size;

        let exn_size_val = builder.ins().iconst(ir::types::I32, i64::from(exn_size));

        let (exn_ref, raw_ptr_to_exn) = self.emit_inline_alloc(
            func_env,
            builder,
            VMGcKind::ExnRef,
            interned_type_index,
            exn_size_val,
        );

        // Initialize fields.
        initialize_struct_fields(
            func_env,
            builder,
            interned_type_index,
            raw_ptr_to_exn,
            field_vals,
            |func_env, builder, ty, field_addr, val| {
                self.init_field(func_env, builder, field_addr, ty, val)
            },
        )?;

        // Initialize tag fields.
        let instance_id_addr = builder
            .ins()
            .iadd_imm(raw_ptr_to_exn, i64::from(EXCEPTION_TAG_INSTANCE_OFFSET));
        self.init_field(
            func_env,
            builder,
            instance_id_addr,
            WasmStorageType::Val(WasmValType::I32),
            instance_id,
        )?;
        let tag_addr = builder
            .ins()
            .iadd_imm(raw_ptr_to_exn, i64::from(EXCEPTION_TAG_DEFINED_OFFSET));
        self.init_field(
            func_env,
            builder,
            tag_addr,
            WasmStorageType::Val(WasmValType::I32),
            tag,
        )?;

        Ok(exn_ref)
    }

    fn translate_read_gc_reference(
        &mut self,
        func_env: &mut FuncEnvironment<'_>,
        builder: &mut FunctionBuilder,
        ty: WasmRefType,
        src: ir::Value,
        flags: ir::MemFlagsData,
    ) -> WasmResult<ir::Value> {
        assert!(ty.is_vmgcref_type());

        let (reference_type, _) = func_env.reference_type(ty.heap_type);

        // Special case for references to uninhabited bottom types.
        if let WasmHeapType::None = ty.heap_type {
            let null = builder.ins().iconst(reference_type, 0);
            if flags.trap_code().is_some() {
                let _ = builder.ins().load(reference_type, flags, src, 0);
            }
            if !ty.nullable {
                let zero = builder.ins().iconst(ir::types::I32, 0);
                builder.ins().trapz(zero, TRAP_INTERNAL_ASSERT);
            }
            return Ok(null);
        };

        // Special case for `i31` references: they don't need stack maps.
        if let WasmHeapType::I31 = ty.heap_type {
            return unbarriered_load_gc_ref(builder, ty.heap_type, src, flags);
        }

        // No read barrier needed for the copying collector, but we do need
        // stack maps so the collector can find and relocate roots.
        unbarriered_load_gc_ref(builder, ty.heap_type, src, flags)
    }

    fn translate_write_gc_reference(
        &mut self,
        _func_env: &mut FuncEnvironment<'_>,
        builder: &mut FunctionBuilder,
        ty: WasmRefType,
        dst: ir::Value,
        new_val: ir::Value,
        flags: ir::MemFlagsData,
    ) -> WasmResult<()> {
        // No write barrier needed for the copying collector.
        unbarriered_store_gc_ref(builder, ty.heap_type, dst, new_val, flags)
    }
}
