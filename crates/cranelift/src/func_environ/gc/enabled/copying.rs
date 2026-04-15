//! Compiler for the copying (semi-space/Cheney) collector.
//!
//! Allocation is performed via the `gc_alloc_raw` libcall (not inlined) for
//! now. Read and write barriers are unnecessary (e.g. no reference counting and
//! no concurrent mutation during collection) but we do need stack maps so the
//! collector can find and update roots when it relocates objects.

use super::*;
use crate::TRAP_INTERNAL_ASSERT;
use crate::func_environ::FuncEnvironment;
use crate::translate::TargetEnvironment;
use cranelift_codegen::ir::{self, InstBuilder};
use cranelift_frontend::FunctionBuilder;
use smallvec::SmallVec;
use wasmtime_environ::copying::{EXCEPTION_TAG_DEFINED_OFFSET, EXCEPTION_TAG_INSTANCE_OFFSET};
use wasmtime_environ::{
    GcTypeLayouts, TypeIndex, VMGcKind, WasmHeapTopType, WasmHeapType, WasmRefType, WasmResult,
    WasmStorageType, WasmValType, copying::CopyingTypeLayouts,
};

#[derive(Default)]
pub struct CopyingCompiler {
    layouts: CopyingTypeLayouts,
}

impl CopyingCompiler {
    fn init_field(
        &mut self,
        func_env: &mut FuncEnvironment<'_>,
        builder: &mut FunctionBuilder<'_>,
        field_addr: ir::Value,
        ty: WasmStorageType,
        val: ir::Value,
    ) -> WasmResult<()> {
        // Data inside GC objects is always little endian.
        let flags = ir::MemFlags::trusted().with_endianness(ir::Endianness::Little);

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
        let align = array_layout.align;
        let len_to_elems_delta = base_size.checked_sub(len_offset).unwrap();

        // First, compute the array's total size.
        let len = init.len(&mut builder.cursor());
        let size = emit_array_size(func_env, builder, &array_layout, len);

        // Allocate via libcall.
        let array_ref = emit_gc_raw_alloc(
            func_env,
            builder,
            VMGcKind::ArrayRef,
            interned_type_index,
            size,
            align,
        );

        // Write the array's length.
        let base = func_env.get_gc_heap_base(builder);
        let extended_array_ref =
            uextend_i32_to_pointer_type(builder, func_env.pointer_type(), array_ref);
        let object_addr = builder.ins().iadd(base, extended_array_ref);
        let len_addr = builder.ins().iadd_imm(object_addr, i64::from(len_offset));
        let len = init.len(&mut builder.cursor());
        builder
            .ins()
            .store(ir::MemFlags::trusted(), len, len_addr, 0);

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
        let struct_align = struct_layout.align;
        let field_offsets: SmallVec<[_; 8]> = struct_layout.fields.iter().copied().collect();
        assert_eq!(field_vals.len(), field_offsets.len());

        let struct_size_val = builder.ins().iconst(ir::types::I32, i64::from(struct_size));

        let struct_ref = emit_gc_raw_alloc(
            func_env,
            builder,
            VMGcKind::StructRef,
            interned_type_index,
            struct_size_val,
            struct_align,
        );

        // Initialize fields.
        let base = func_env.get_gc_heap_base(builder);
        let extended_struct_ref =
            uextend_i32_to_pointer_type(builder, func_env.pointer_type(), struct_ref);
        let raw_ptr_to_struct = builder.ins().iadd(base, extended_struct_ref);
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
        let exn_align = exn_layout.align;
        let field_offsets: SmallVec<[_; 8]> = exn_layout.fields.iter().copied().collect();
        assert_eq!(field_vals.len(), field_offsets.len());

        let exn_size_val = builder.ins().iconst(ir::types::I32, i64::from(exn_size));

        let exn_ref = emit_gc_raw_alloc(
            func_env,
            builder,
            VMGcKind::ExnRef,
            interned_type_index,
            exn_size_val,
            exn_align,
        );

        // Initialize fields.
        let base = func_env.get_gc_heap_base(builder);
        let extended_exn_ref =
            uextend_i32_to_pointer_type(builder, func_env.pointer_type(), exn_ref);
        let raw_ptr_to_exn = builder.ins().iadd(base, extended_exn_ref);
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
        flags: ir::MemFlags,
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
        flags: ir::MemFlags,
    ) -> WasmResult<()> {
        // No write barrier needed for the copying collector.
        unbarriered_store_gc_ref(builder, ty.heap_type, dst, new_val, flags)
    }
}
