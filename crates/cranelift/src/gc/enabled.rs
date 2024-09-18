use super::GcCompiler;
use crate::func_environ::FuncEnvironment;
use cranelift_codegen::{
    cursor::FuncCursor,
    ir::{self, condcodes::IntCC, InstBuilder},
};
use cranelift_frontend::FunctionBuilder;
use cranelift_wasm::{
    ModuleInternedTypeIndex, StructFieldsVec, TargetEnvironment, TypeIndex, WasmCompositeType,
    WasmHeapTopType, WasmHeapType, WasmRefType, WasmResult, WasmStorageType, WasmValType,
};
use wasmtime_environ::{GcStructLayout, PtrSize, I31_DISCRIMINANT, NON_NULL_NON_I31_MASK};

mod drc;

/// Get the default GC compiler.
pub fn gc_compiler(_func_env: &FuncEnvironment<'_>) -> WasmResult<Box<dyn GcCompiler>> {
    Ok(Box::new(drc::DrcCompiler::default()))
}

fn unbarriered_load_gc_ref(
    builder: &mut FunctionBuilder,
    ty: WasmHeapType,
    ptr_to_gc_ref: ir::Value,
    flags: ir::MemFlags,
) -> WasmResult<ir::Value> {
    debug_assert!(ty.is_vmgcref_type());
    let gc_ref = builder.ins().load(ir::types::I32, flags, ptr_to_gc_ref, 0);
    if ty != WasmHeapType::I31 {
        builder.declare_value_needs_stack_map(gc_ref);
    }
    Ok(gc_ref)
}

fn unbarriered_store_gc_ref(
    builder: &mut FunctionBuilder,
    ty: WasmHeapType,
    dst: ir::Value,
    gc_ref: ir::Value,
    flags: ir::MemFlags,
) -> WasmResult<()> {
    debug_assert!(ty.is_vmgcref_type());
    builder.ins().store(flags, gc_ref, dst, 0);
    Ok(())
}

pub fn gc_ref_table_grow_builtin(
    ty: WasmHeapType,
    func_env: &mut FuncEnvironment<'_>,
    func: &mut ir::Function,
) -> WasmResult<ir::FuncRef> {
    debug_assert!(ty.is_vmgcref_type());
    Ok(func_env.builtin_functions.table_grow_gc_ref(func))
}

pub fn gc_ref_table_fill_builtin(
    ty: WasmHeapType,
    func_env: &mut FuncEnvironment<'_>,
    func: &mut ir::Function,
) -> WasmResult<ir::FuncRef> {
    debug_assert!(ty.is_vmgcref_type());
    Ok(func_env.builtin_functions.table_fill_gc_ref(func))
}

pub fn translate_struct_new(
    func_env: &mut FuncEnvironment<'_>,
    builder: &mut FunctionBuilder<'_>,
    struct_type_index: TypeIndex,
    fields: &[ir::Value],
) -> WasmResult<ir::Value> {
    gc_compiler(func_env)?.alloc_struct(func_env, builder, struct_type_index, &fields)
}

fn default_value(
    cursor: &mut FuncCursor,
    func_env: &FuncEnvironment<'_>,
    ty: &WasmStorageType,
) -> ir::Value {
    match ty {
        WasmStorageType::I8 | WasmStorageType::I16 => cursor.ins().iconst(ir::types::I32, 0),
        WasmStorageType::Val(v) => match v {
            WasmValType::I32 => cursor.ins().iconst(ir::types::I32, 0),
            WasmValType::I64 => cursor.ins().iconst(ir::types::I64, 0),
            WasmValType::F32 => cursor.ins().f32const(0.0),
            WasmValType::F64 => cursor.ins().f64const(0.0),
            WasmValType::V128 => cursor.ins().iconst(ir::types::I128, 0),
            WasmValType::Ref(r) => {
                assert!(r.nullable);
                let (ty, needs_stack_map) = func_env.reference_type(r.heap_type);

                // NB: The collector doesn't need to know about null references.
                let _ = needs_stack_map;

                cursor.ins().iconst(ty, 0)
            }
        },
    }
}

pub fn translate_struct_new_default(
    func_env: &mut FuncEnvironment<'_>,
    builder: &mut FunctionBuilder<'_>,
    struct_type_index: TypeIndex,
) -> WasmResult<ir::Value> {
    let interned_ty = func_env.module.types[struct_type_index];
    let struct_ty = match &func_env.types[interned_ty].composite_type {
        WasmCompositeType::Struct(s) => s,
        _ => unreachable!(),
    };
    let fields = struct_ty
        .fields
        .iter()
        .map(|f| default_value(&mut builder.cursor(), func_env, &f.element_type))
        .collect::<StructFieldsVec>();
    gc_compiler(func_env)?.alloc_struct(func_env, builder, struct_type_index, &fields)
}

pub fn translate_struct_get(
    func_env: &mut FuncEnvironment<'_>,
    builder: &mut FunctionBuilder<'_>,
    struct_type_index: TypeIndex,
    field_index: u32,
    struct_ref: ir::Value,
) -> WasmResult<ir::Value> {
    // TODO: If we know we have a `(ref $my_struct)` here, instead of maybe a
    // `(ref null $my_struct)`, we could omit the `trapz`. But plumbing that
    // type info from `wasmparser` and through to here is a bit funky.
    builder.ins().trapz(struct_ref, ir::TrapCode::NullReference);

    let field_index = usize::try_from(field_index).unwrap();
    let interned_type_index = func_env.module.types[struct_type_index];

    let struct_layout = func_env.struct_layout(interned_type_index);
    let field_offset = struct_layout.fields[field_index];

    let field_ty = match &func_env.types[interned_type_index].composite_type {
        WasmCompositeType::Struct(s) => &s.fields[field_index],
        _ => unreachable!(),
    };

    let field_size = wasmtime_environ::byte_size_of_wasm_ty_in_gc_heap(&field_ty.element_type);

    // TODO: We should claim we are accessing the whole object here so that
    // repeated accesses to different fields can have their bounds checks
    // deduped by GVN. This is a bit tricky to do right now because the last
    // parameter of `prepare_gc_ref_access` is the size of the access, and is
    // relative to `gc_ref[offset]`, rather than the size of the object itself,
    // and relative to `gc_ref[0]`.
    let field_addr = func_env.prepare_gc_ref_access(builder, struct_ref, field_offset, field_size);

    let field_val = match field_ty.element_type {
        WasmStorageType::Val(v) => match v {
            WasmValType::I32 => {
                builder
                    .ins()
                    .load(ir::types::I32, ir::MemFlags::trusted(), field_addr, 0)
            }
            WasmValType::I64 => {
                builder
                    .ins()
                    .load(ir::types::I64, ir::MemFlags::trusted(), field_addr, 0)
            }
            WasmValType::F32 => {
                builder
                    .ins()
                    .load(ir::types::F32, ir::MemFlags::trusted(), field_addr, 0)
            }
            WasmValType::F64 => {
                builder
                    .ins()
                    .load(ir::types::F64, ir::MemFlags::trusted(), field_addr, 0)
            }
            WasmValType::V128 => {
                builder
                    .ins()
                    .load(ir::types::I128, ir::MemFlags::trusted(), field_addr, 0)
            }
            WasmValType::Ref(r) => match r.heap_type.top() {
                WasmHeapTopType::Any | WasmHeapTopType::Extern => gc_compiler(func_env)?
                    .translate_read_gc_reference(
                        func_env,
                        builder,
                        r,
                        field_addr,
                        ir::MemFlags::trusted(),
                    )?,
                WasmHeapTopType::Func => {
                    unimplemented!("funcrefs inside the GC heap")
                }
            },
        },
        WasmStorageType::I8 | WasmStorageType::I16 => {
            unreachable!()
        }
    };

    Ok(field_val)
}

enum Extension {
    Sign,
    Zero,
}

fn translate_struct_get_and_extend(
    func_env: &mut FuncEnvironment<'_>,
    builder: &mut FunctionBuilder<'_>,
    struct_type_index: TypeIndex,
    field_index: u32,
    struct_ref: ir::Value,
    extension: Extension,
) -> WasmResult<ir::Value> {
    // TODO: See comment in `translate_struct_get` about the `trapz`.
    builder.ins().trapz(struct_ref, ir::TrapCode::NullReference);

    let field_index = usize::try_from(field_index).unwrap();
    let interned_type_index = func_env.module.types[struct_type_index];

    let struct_layout = func_env.struct_layout(interned_type_index);
    let field_offset = struct_layout.fields[field_index];

    let field_ty = match &func_env.types[interned_type_index].composite_type {
        WasmCompositeType::Struct(s) => &s.fields[field_index],
        _ => unreachable!(),
    };

    let field_size = wasmtime_environ::byte_size_of_wasm_ty_in_gc_heap(&field_ty.element_type);

    // TODO: See comment in `translate_struct_get` about the `prepare_gc_ref_access`.
    let field_addr = func_env.prepare_gc_ref_access(builder, struct_ref, field_offset, field_size);

    let field_val = match field_ty.element_type {
        WasmStorageType::I8 => {
            builder
                .ins()
                .load(ir::types::I8, ir::MemFlags::trusted(), field_addr, 0)
        }
        WasmStorageType::I16 => {
            builder
                .ins()
                .load(ir::types::I16, ir::MemFlags::trusted(), field_addr, 0)
        }
        WasmStorageType::Val(_) => unreachable!(),
    };

    let extended = match extension {
        Extension::Sign => builder.ins().sextend(ir::types::I32, field_val),
        Extension::Zero => builder.ins().uextend(ir::types::I32, field_val),
    };

    Ok(extended)
}

pub fn translate_struct_get_s(
    func_env: &mut FuncEnvironment<'_>,
    builder: &mut FunctionBuilder<'_>,
    struct_type_index: TypeIndex,
    field_index: u32,
    struct_ref: ir::Value,
) -> WasmResult<ir::Value> {
    translate_struct_get_and_extend(
        func_env,
        builder,
        struct_type_index,
        field_index,
        struct_ref,
        Extension::Sign,
    )
}

pub fn translate_struct_get_u(
    func_env: &mut FuncEnvironment<'_>,
    builder: &mut FunctionBuilder<'_>,
    struct_type_index: TypeIndex,
    field_index: u32,
    struct_ref: ir::Value,
) -> WasmResult<ir::Value> {
    translate_struct_get_and_extend(
        func_env,
        builder,
        struct_type_index,
        field_index,
        struct_ref,
        Extension::Zero,
    )
}

pub fn translate_struct_set(
    func_env: &mut FuncEnvironment<'_>,
    builder: &mut FunctionBuilder<'_>,
    struct_type_index: TypeIndex,
    field_index: u32,
    struct_ref: ir::Value,
    new_val: ir::Value,
) -> WasmResult<()> {
    // TODO: See comment in `translate_struct_get` about the `trapz`.
    builder.ins().trapz(struct_ref, ir::TrapCode::NullReference);

    let field_index = usize::try_from(field_index).unwrap();
    let interned_type_index = func_env.module.types[struct_type_index];

    let struct_layout = func_env.struct_layout(interned_type_index);
    let field_offset = struct_layout.fields[field_index];

    let field_ty = match &func_env.types[interned_type_index].composite_type {
        WasmCompositeType::Struct(s) => &s.fields[field_index],
        _ => unreachable!(),
    };

    let field_size = wasmtime_environ::byte_size_of_wasm_ty_in_gc_heap(&field_ty.element_type);

    // TODO: See comment in `translate_struct_get` about the `prepare_gc_ref_access`.
    let field_addr = func_env.prepare_gc_ref_access(builder, struct_ref, field_offset, field_size);

    match &field_ty.element_type {
        WasmStorageType::I8 => {
            builder
                .ins()
                .istore8(ir::MemFlags::trusted(), new_val, field_addr, 0);
        }
        WasmStorageType::I16 => {
            builder
                .ins()
                .istore16(ir::MemFlags::trusted(), new_val, field_addr, 0);
        }
        WasmStorageType::Val(WasmValType::Ref(r)) if r.heap_type.top() == WasmHeapTopType::Func => {
            unimplemented!("funcrefs inside the GC heap")
        }
        WasmStorageType::Val(WasmValType::Ref(r)) => {
            gc_compiler(func_env)?.translate_write_gc_reference(
                func_env,
                builder,
                *r,
                field_addr,
                new_val,
                ir::MemFlags::trusted(),
            )?;
        }
        WasmStorageType::Val(_) => {
            assert_eq!(builder.func.dfg.value_type(new_val).bytes(), field_size);
            builder
                .ins()
                .store(ir::MemFlags::trusted(), new_val, field_addr, 0);
        }
    }

    Ok(())
}

impl FuncEnvironment<'_> {
    /// Get the `GcStructLayout` for the struct type at the given `type_index`.
    fn struct_layout(&mut self, type_index: ModuleInternedTypeIndex) -> &GcStructLayout {
        // Lazily compute and cache the struct layout. Note that we can't use
        // the entry API because of borrowck shenanigans.
        if !self.ty_to_struct_layout.contains_key(&type_index) {
            let ty = &self.types[type_index];
            let WasmCompositeType::Struct(s) = &ty.composite_type else {
                panic!("{type_index:?} is not a struct type: {ty:?}")
            };
            let s = s.clone();
            let layout = gc_compiler(self).unwrap().layouts().struct_layout(&s);
            self.ty_to_struct_layout.insert(type_index, layout);
        }

        self.ty_to_struct_layout.get(&type_index).unwrap()
    }

    /// Get the GC heap's base pointer and bound.
    fn get_gc_heap_base_bound(&mut self, builder: &mut FunctionBuilder) -> (ir::Value, ir::Value) {
        let ptr_ty = self.pointer_type();
        let flags = ir::MemFlags::trusted().with_readonly();

        let vmctx = self.vmctx(builder.func);
        let vmctx = builder.ins().global_value(ptr_ty, vmctx);

        let base_offset = self.offsets.ptr.vmctx_gc_heap_base();
        let base_offset = i32::from(base_offset);

        let bound_offset = self.offsets.ptr.vmctx_gc_heap_bound();
        let bound_offset = i32::from(bound_offset);

        let base = builder.ins().load(ptr_ty, flags, vmctx, base_offset);
        let bound = builder.ins().load(ptr_ty, flags, vmctx, bound_offset);

        (base, bound)
    }

    /// Get the raw pointer of `gc_ref[offset]` bounds checked for an access of
    /// `size` bytes.
    ///
    /// The given `gc_ref` must be a non-null, non-i31 GC reference.
    ///
    /// Returns the raw pointer to `gc_ref[offset]` -- not a raw pointer to the
    /// GC object itself (unless `offset == 0`). This raw pointer may be used to
    /// read or write up to `size` bytes. Do NOT attempt accesses larger than
    /// `size` bytes; that may lead to unchecked out-of-bounds accesses.
    ///
    /// This method is collector-agnostic.
    fn prepare_gc_ref_access(
        &mut self,
        builder: &mut FunctionBuilder,
        gc_ref: ir::Value,
        offset: u32,
        size: u32,
    ) -> ir::Value {
        let pointer_type = self.pointer_type();
        let (base, bound) = self.get_gc_heap_base_bound(builder);

        debug_assert_eq!(builder.func.dfg.value_type(gc_ref), ir::types::I32);
        let index = match pointer_type {
            ir::types::I32 => gc_ref,
            ir::types::I64 => builder.ins().uextend(ir::types::I64, gc_ref),
            _ => unreachable!(),
        };

        let offset = builder
            .ins()
            .iconst(pointer_type, i64::try_from(offset).unwrap());

        // Check that `index + offset + sizeof(i64)` is in bounds.
        let index_and_offset = builder.ins().uadd_overflow_trap(
            index,
            offset,
            ir::TrapCode::User(crate::DEBUG_ASSERT_TRAP_CODE),
        );
        let size = builder
            .ins()
            .iconst(pointer_type, i64::try_from(size).unwrap());
        let index_offset_and_size = builder.ins().uadd_overflow_trap(
            index_and_offset,
            size,
            ir::TrapCode::User(crate::DEBUG_ASSERT_TRAP_CODE),
        );
        let in_bounds = builder.ins().icmp(
            ir::condcodes::IntCC::UnsignedLessThan,
            index_offset_and_size,
            bound,
        );
        builder
            .ins()
            .trapz(in_bounds, ir::TrapCode::User(crate::DEBUG_ASSERT_TRAP_CODE));

        builder.ins().iadd(base, index_and_offset)
    }

    /// Emit checks (if necessary) for whether the given `gc_ref` is null or is
    /// an `i31ref`.
    ///
    /// Takes advantage of static information based on `ty` as to whether the GC
    /// reference is nullable or can ever be an `i31`.
    ///
    /// Returns an `ir::Value` that is an `i32` will be non-zero if the GC
    /// reference is null or is an `i31ref`; otherwise, it will be zero.
    ///
    /// This method is collector-agnostic.
    fn gc_ref_is_null_or_i31(
        &mut self,
        builder: &mut FunctionBuilder,
        ty: WasmRefType,
        gc_ref: ir::Value,
    ) -> ir::Value {
        assert!(ty.is_vmgcref_type_and_not_i31());

        let might_be_i31 = match ty.heap_type {
            // If we are definitely dealing with an i31, we shouldn't be
            // emitting dynamic checks for it, and the caller shouldn't call
            // this function. Should have been caught by the assertion at the
            // start of the function.
            WasmHeapType::I31 => unreachable!(),

            // Could potentially be an i31.
            WasmHeapType::Any | WasmHeapType::Eq => true,

            // If it is definitely a struct, array, or uninhabited type, then it
            // is definitely not an i31.
            WasmHeapType::Array
            | WasmHeapType::ConcreteArray(_)
            | WasmHeapType::Struct
            | WasmHeapType::ConcreteStruct(_)
            | WasmHeapType::None => false,

            // Wrong type hierarchy: cannot be an i31.
            WasmHeapType::Extern | WasmHeapType::NoExtern => false,

            // Wrong type hierarchy, and also funcrefs are not GC-managed
            // types. Should have been caught by the assertion at the start of
            // the function.
            WasmHeapType::Func | WasmHeapType::ConcreteFunc(_) | WasmHeapType::NoFunc => {
                unreachable!()
            }
        };

        match (ty.nullable, might_be_i31) {
            // This GC reference statically cannot be null nor an i31. (Let
            // Cranelift's optimizer const-propagate this value and erase any
            // unnecessary control flow resulting from branching on this value.)
            (false, false) => builder.ins().iconst(ir::types::I32, 0),

            // This GC reference is always non-null, but might be an i31.
            (false, true) => builder.ins().band_imm(gc_ref, I31_DISCRIMINANT as i64),

            // This GC reference might be null, but can never be an i31.
            (true, false) => builder.ins().icmp_imm(IntCC::Equal, gc_ref, 0),

            // Fully general case: this GC reference could be either null or an
            // i31.
            (true, true) => {
                // Mask for checking whether any bits are set, other than the
                // `i31ref` discriminant, which should not be set. This folds
                // the null and i31ref checks together into a single `band`.
                let mask = builder.ins().iconst(
                    ir::types::I32,
                    (NON_NULL_NON_I31_MASK & u32::MAX as u64) as i64,
                );
                let is_non_null_and_non_i31 = builder.ins().band(gc_ref, mask);
                builder
                    .ins()
                    .icmp_imm(ir::condcodes::IntCC::Equal, is_non_null_and_non_i31, 0)
            }
        }
    }
}
