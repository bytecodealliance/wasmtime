use super::GcCompiler;
use crate::func_environ::FuncEnvironment;
use crate::gc::ArrayInit;
use crate::translate::{StructFieldsVec, TargetEnvironment};
use crate::DEBUG_ASSERT_TRAP_CODE;
use cranelift_codegen::{
    cursor::FuncCursor,
    ir::{self, condcodes::IntCC, InstBuilder},
};
use cranelift_entity::packed_option::ReservedValue;
use cranelift_frontend::FunctionBuilder;
use wasmtime_environ::{
    GcArrayLayout, GcLayout, GcStructLayout, ModuleInternedTypeIndex, PtrSize, TypeIndex,
    WasmCompositeType, WasmHeapTopType, WasmHeapType, WasmRefType, WasmResult, WasmStorageType,
    WasmValType, I31_DISCRIMINANT, NON_NULL_NON_I31_MASK,
};

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

enum Extension {
    Sign,
    Zero,
}

/// Emit code to read a struct field or array element from its raw address in
/// the GC heap.
///
/// The given address MUST have already been bounds-checked via
/// `prepare_gc_ref_access`.
fn read_field_at_addr(
    func_env: &mut FuncEnvironment<'_>,
    builder: &mut FunctionBuilder<'_>,
    ty: WasmStorageType,
    addr: ir::Value,
    extension: Option<Extension>,
) -> WasmResult<ir::Value> {
    assert_eq!(extension.is_none(), matches!(ty, WasmStorageType::Val(_)));
    assert_eq!(
        extension.is_some(),
        matches!(ty, WasmStorageType::I8 | WasmStorageType::I16)
    );

    // Data inside GC objects is always little endian.
    let flags = ir::MemFlags::trusted().with_endianness(ir::Endianness::Little);

    let value = match ty {
        WasmStorageType::I8 => builder.ins().load(ir::types::I8, flags, addr, 0),
        WasmStorageType::I16 => builder.ins().load(ir::types::I16, flags, addr, 0),
        WasmStorageType::Val(v) => match v {
            WasmValType::I32 => builder.ins().load(ir::types::I32, flags, addr, 0),
            WasmValType::I64 => builder.ins().load(ir::types::I64, flags, addr, 0),
            WasmValType::F32 => builder.ins().load(ir::types::F32, flags, addr, 0),
            WasmValType::F64 => builder.ins().load(ir::types::F64, flags, addr, 0),
            WasmValType::V128 => builder.ins().load(ir::types::I128, flags, addr, 0),
            WasmValType::Ref(r) => match r.heap_type.top() {
                WasmHeapTopType::Any | WasmHeapTopType::Extern => gc_compiler(func_env)?
                    .translate_read_gc_reference(func_env, builder, r, addr, flags)?,
                WasmHeapTopType::Func => {
                    let expected_ty = match r.heap_type {
                        WasmHeapType::Func => ModuleInternedTypeIndex::reserved_value(),
                        WasmHeapType::ConcreteFunc(ty) => ty.unwrap_module_type_index(),
                        WasmHeapType::NoFunc => {
                            let null = builder.ins().iconst(func_env.pointer_type(), 0);
                            if !r.nullable {
                                // Because `nofunc` is uninhabited, and this
                                // reference is non-null, this is unreachable
                                // code. Unconditionally trap via conditional
                                // trap instructions to avoid inserting block
                                // terminators in the middle of this block.
                                builder
                                    .ins()
                                    .trapz(null, ir::TrapCode::User(DEBUG_ASSERT_TRAP_CODE));
                            }
                            return Ok(null);
                        }
                        _ => unreachable!("not a function heap type"),
                    };
                    let expected_ty = builder
                        .ins()
                        .iconst(ir::types::I32, i64::from(expected_ty.as_bits()));

                    let vmctx = func_env.vmctx_val(&mut builder.cursor());

                    let func_ref_id = builder.ins().load(ir::types::I32, flags, addr, 0);
                    let get_interned_func_ref = func_env
                        .builtin_functions
                        .get_interned_func_ref(builder.func);

                    let call_inst = builder
                        .ins()
                        .call(get_interned_func_ref, &[vmctx, func_ref_id, expected_ty]);
                    builder.func.dfg.first_result(call_inst)
                }
            },
        },
    };

    let value = match extension {
        Some(Extension::Sign) => builder.ins().sextend(ir::types::I32, value),
        Some(Extension::Zero) => builder.ins().uextend(ir::types::I32, value),
        None => value,
    };

    Ok(value)
}

fn write_func_ref_at_addr(
    func_env: &mut FuncEnvironment<'_>,
    builder: &mut FunctionBuilder<'_>,
    ref_type: WasmRefType,
    flags: ir::MemFlags,
    field_addr: ir::Value,
    func_ref: ir::Value,
) -> WasmResult<()> {
    assert_eq!(ref_type.heap_type.top(), WasmHeapTopType::Func);

    let vmctx = func_env.vmctx_val(&mut builder.cursor());

    let intern_func_ref_for_gc_heap = func_env
        .builtin_functions
        .intern_func_ref_for_gc_heap(builder.func);

    let func_ref = if ref_type.heap_type == WasmHeapType::NoFunc {
        let null = builder.ins().iconst(func_env.pointer_type(), 0);
        if !ref_type.nullable {
            // Because `nofunc` is uninhabited, and this reference is
            // non-null, this is unreachable code. Unconditionally trap
            // via conditional trap instructions to avoid inserting
            // block terminators in the middle of this block.
            builder
                .ins()
                .trapz(null, ir::TrapCode::User(DEBUG_ASSERT_TRAP_CODE));
        }
        null
    } else {
        func_ref
    };

    // Convert the raw `funcref` into a `FuncRefTableId` for use in the
    // GC heap.
    let call_inst = builder
        .ins()
        .call(intern_func_ref_for_gc_heap, &[vmctx, func_ref]);
    let func_ref_id = builder.func.dfg.first_result(call_inst);

    // Store the id in the field.
    builder.ins().store(flags, func_ref_id, field_addr, 0);

    Ok(())
}

fn write_field_at_addr(
    func_env: &mut FuncEnvironment<'_>,
    builder: &mut FunctionBuilder<'_>,
    field_ty: WasmStorageType,
    field_addr: ir::Value,
    new_val: ir::Value,
) -> WasmResult<()> {
    // Data inside GC objects is always little endian.
    let flags = ir::MemFlags::trusted().with_endianness(ir::Endianness::Little);

    match field_ty {
        WasmStorageType::I8 => {
            builder.ins().istore8(flags, new_val, field_addr, 0);
        }
        WasmStorageType::I16 => {
            builder.ins().istore16(flags, new_val, field_addr, 0);
        }
        WasmStorageType::Val(WasmValType::Ref(r)) if r.heap_type.top() == WasmHeapTopType::Func => {
            write_func_ref_at_addr(func_env, builder, r, flags, field_addr, new_val)?;
        }
        WasmStorageType::Val(WasmValType::Ref(r)) => {
            gc_compiler(func_env)?
                .translate_write_gc_reference(func_env, builder, r, field_addr, new_val, flags)?;
        }
        WasmStorageType::Val(_) => {
            assert_eq!(
                builder.func.dfg.value_type(new_val).bytes(),
                wasmtime_environ::byte_size_of_wasm_ty_in_gc_heap(&field_ty)
            );
            builder.ins().store(flags, new_val, field_addr, 0);
        }
    }
    Ok(())
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
    let struct_size = struct_layout.size;
    let struct_size_val = builder.ins().iconst(ir::types::I32, i64::from(struct_size));

    let field_offset = struct_layout.fields[field_index];

    let field_ty = match &func_env.types[interned_type_index].composite_type {
        WasmCompositeType::Struct(s) => &s.fields[field_index],
        _ => unreachable!(),
    };

    let field_size = wasmtime_environ::byte_size_of_wasm_ty_in_gc_heap(&field_ty.element_type);
    assert!(field_offset + field_size <= struct_size);

    let field_addr = func_env.prepare_gc_ref_access(
        builder,
        struct_ref,
        Offset::Static(field_offset),
        BoundsCheck::Object(struct_size_val),
    );

    read_field_at_addr(func_env, builder, field_ty.element_type, field_addr, None)
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
    let struct_size = struct_layout.size;
    let struct_size_val = builder.ins().iconst(ir::types::I32, i64::from(struct_size));

    let field_offset = struct_layout.fields[field_index];

    let field_ty = match &func_env.types[interned_type_index].composite_type {
        WasmCompositeType::Struct(s) => &s.fields[field_index],
        _ => unreachable!(),
    };

    let field_size = wasmtime_environ::byte_size_of_wasm_ty_in_gc_heap(&field_ty.element_type);
    assert!(field_offset + field_size <= struct_size);

    let field_addr = func_env.prepare_gc_ref_access(
        builder,
        struct_ref,
        Offset::Static(field_offset),
        BoundsCheck::Object(struct_size_val),
    );

    read_field_at_addr(
        func_env,
        builder,
        field_ty.element_type,
        field_addr,
        Some(extension),
    )
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
    let struct_size = struct_layout.size;
    let struct_size_val = builder.ins().iconst(ir::types::I32, i64::from(struct_size));

    let field_offset = struct_layout.fields[field_index];

    let field_ty = match &func_env.types[interned_type_index].composite_type {
        WasmCompositeType::Struct(s) => &s.fields[field_index],
        _ => unreachable!(),
    };

    let field_size = wasmtime_environ::byte_size_of_wasm_ty_in_gc_heap(&field_ty.element_type);
    assert!(field_offset + field_size <= struct_size);

    let field_addr = func_env.prepare_gc_ref_access(
        builder,
        struct_ref,
        Offset::Static(field_offset),
        BoundsCheck::Object(struct_size_val),
    );

    write_field_at_addr(
        func_env,
        builder,
        field_ty.element_type,
        field_addr,
        new_val,
    )
}

pub fn translate_array_new(
    func_env: &mut FuncEnvironment<'_>,
    builder: &mut FunctionBuilder,
    array_type_index: TypeIndex,
    elem: ir::Value,
    len: ir::Value,
) -> WasmResult<ir::Value> {
    gc_compiler(func_env)?.alloc_array(
        func_env,
        builder,
        array_type_index,
        ArrayInit::Fill { elem, len },
    )
}

pub fn translate_array_new_default(
    func_env: &mut FuncEnvironment<'_>,
    builder: &mut FunctionBuilder,
    array_type_index: TypeIndex,
    len: ir::Value,
) -> WasmResult<ir::Value> {
    let interned_ty = func_env.module.types[array_type_index];
    let WasmCompositeType::Array(array_ty) = &func_env.types[interned_ty].composite_type else {
        unreachable!()
    };
    let elem = default_value(&mut builder.cursor(), func_env, &array_ty.0.element_type);
    gc_compiler(func_env)?.alloc_array(
        func_env,
        builder,
        array_type_index,
        ArrayInit::Fill { elem, len },
    )
}

pub fn translate_array_new_fixed(
    func_env: &mut FuncEnvironment<'_>,
    builder: &mut FunctionBuilder,
    array_type_index: TypeIndex,
    elems: &[ir::Value],
) -> WasmResult<ir::Value> {
    gc_compiler(func_env)?.alloc_array(func_env, builder, array_type_index, ArrayInit::Elems(elems))
}

fn emit_array_fill_impl(
    func_env: &mut FuncEnvironment<'_>,
    builder: &mut FunctionBuilder<'_>,
    elem_addr: ir::Value,
    elem_size: ir::Value,
    fill_end: ir::Value,
    mut emit_elem_write: impl FnMut(
        &mut FuncEnvironment<'_>,
        &mut FunctionBuilder<'_>,
        ir::Value,
    ) -> WasmResult<()>,
) -> WasmResult<()> {
    let pointer_ty = func_env.pointer_type();

    assert_eq!(builder.func.dfg.value_type(elem_addr), pointer_ty);
    assert_eq!(builder.func.dfg.value_type(elem_size), pointer_ty);
    assert_eq!(builder.func.dfg.value_type(fill_end), pointer_ty);

    // Loop to fill the elements, emitting the equivalent of the following
    // pseudo-CLIF:
    //
    // current_block:
    //     ...
    //     jump loop_header_block(elem_addr)
    //
    // loop_header_block(elem_addr: i32):
    //     done = icmp eq elem_addr, fill_end
    //     brif done, continue_block, loop_body_block
    //
    // loop_body_block:
    //     emit_elem_write()
    //     next_elem_addr = iadd elem_addr, elem_size
    //     jump loop_header_block(next_elem_addr)
    //
    // continue_block:
    //     ...

    let current_block = builder.current_block().unwrap();
    let loop_header_block = builder.create_block();
    let loop_body_block = builder.create_block();
    let continue_block = builder.create_block();

    builder.ensure_inserted_block();
    builder.insert_block_after(loop_header_block, current_block);
    builder.insert_block_after(loop_body_block, loop_header_block);
    builder.insert_block_after(continue_block, loop_body_block);

    // Current block: jump to the loop header block with the first element's
    // address.
    builder.ins().jump(loop_header_block, &[elem_addr]);

    // Loop header block: check if we're done, then jump to either the continue
    // block or the loop body block.
    builder.switch_to_block(loop_header_block);
    builder.append_block_param(loop_header_block, pointer_ty);
    let elem_addr = builder.block_params(loop_header_block)[0];
    let done = builder.ins().icmp(IntCC::Equal, elem_addr, fill_end);
    builder
        .ins()
        .brif(done, continue_block, &[], loop_body_block, &[]);

    // Loop body block: write the value to the current element, compute the next
    // element's address, and then jump back to the loop header block.
    builder.switch_to_block(loop_body_block);
    emit_elem_write(func_env, builder, elem_addr)?;
    let next_elem_addr = builder.ins().iadd(elem_addr, elem_size);
    builder.ins().jump(loop_header_block, &[next_elem_addr]);

    // Continue...
    builder.switch_to_block(continue_block);
    builder.seal_block(loop_header_block);
    builder.seal_block(loop_body_block);
    builder.seal_block(continue_block);
    Ok(())
}

pub fn translate_array_fill(
    func_env: &mut FuncEnvironment<'_>,
    builder: &mut FunctionBuilder<'_>,
    array_type_index: TypeIndex,
    array_ref: ir::Value,
    index: ir::Value,
    value: ir::Value,
    n: ir::Value,
) -> WasmResult<()> {
    let len = translate_array_len(func_env, builder, array_ref)?;

    // Check that the full range of elements we want to fill is within bounds.
    let end_index = builder
        .ins()
        .uadd_overflow_trap(index, n, ir::TrapCode::ArrayOutOfBounds);
    let out_of_bounds = builder
        .ins()
        .icmp(IntCC::UnsignedGreaterThan, end_index, len);
    builder
        .ins()
        .trapnz(out_of_bounds, ir::TrapCode::ArrayOutOfBounds);

    // Get the address of the first element we want to fill.
    let interned_type_index = func_env.module.types[array_type_index];
    let ArraySizeInfo {
        obj_size,
        one_elem_size,
        base_size,
    } = emit_array_size_info(func_env, builder, interned_type_index, len);
    let offset_in_elems = builder.ins().imul(index, one_elem_size);
    let obj_offset = builder.ins().iadd(base_size, offset_in_elems);
    let elem_addr = func_env.prepare_gc_ref_access(
        builder,
        array_ref,
        Offset::Dynamic(obj_offset),
        BoundsCheck::Object(obj_size),
    );

    // Calculate the end address, just after the filled region.
    let fill_size = uextend_i32_to_pointer_type(builder, func_env.pointer_type(), offset_in_elems);
    let fill_end = builder.ins().iadd(elem_addr, fill_size);

    let one_elem_size =
        uextend_i32_to_pointer_type(builder, func_env.pointer_type(), one_elem_size);

    emit_array_fill_impl(
        func_env,
        builder,
        elem_addr,
        one_elem_size,
        fill_end,
        |func_env, builder, elem_addr| {
            let elem_ty = func_env.types[interned_type_index]
                .composite_type
                .unwrap_array()
                .0
                .element_type;
            write_field_at_addr(func_env, builder, elem_ty, elem_addr, value)
        },
    )
}

pub fn translate_array_len(
    func_env: &mut FuncEnvironment<'_>,
    builder: &mut FunctionBuilder,
    array_ref: ir::Value,
) -> WasmResult<ir::Value> {
    builder.ins().trapz(array_ref, ir::TrapCode::NullReference);

    let len_offset = gc_compiler(func_env)?.layouts().array_length_field_offset();
    let len_field = func_env.prepare_gc_ref_access(
        builder,
        array_ref,
        Offset::Static(len_offset),
        // Note: We can't bounds check the whole array object's size because we
        // don't know its length yet. Chicken and egg problem.
        BoundsCheck::Access(ir::types::I32.bytes()),
    );
    Ok(builder
        .ins()
        .load(ir::types::I32, ir::MemFlags::trusted(), len_field, 0))
}

struct ArraySizeInfo {
    /// The `i32` size of the whole array object, in bytes.
    obj_size: ir::Value,

    /// The `i32` size of each one of the array's elements, in bytes.
    one_elem_size: ir::Value,

    /// The `i32` size of the array's base object, in bytes. This is also the
    /// offset from the start of the array object to its elements.
    base_size: ir::Value,
}

/// Emit code to get the dynamic size (in bytes) of a whole array object, along
/// with some other related bits.
fn emit_array_size_info(
    func_env: &mut FuncEnvironment<'_>,
    builder: &mut FunctionBuilder<'_>,
    array_type_index: ModuleInternedTypeIndex,
    // `i32` value containing the array's length.
    array_len: ir::Value,
) -> ArraySizeInfo {
    let array_layout = func_env.array_layout(array_type_index);

    // Note that we check for overflow below because we can't trust the array's
    // length: it came from inside the GC heap.
    //
    // We check for 32-bit multiplication overflow by performing a 64-bit
    // multiplication and testing the high bits.
    let one_elem_size = builder
        .ins()
        .iconst(ir::types::I64, i64::from(array_layout.elem_size));
    let array_len = builder.ins().uextend(ir::types::I64, array_len);
    let all_elems_size = builder.ins().imul(one_elem_size, array_len);

    let high_bits = builder.ins().ushr_imm(all_elems_size, 32);
    builder
        .ins()
        .trapnz(high_bits, ir::TrapCode::User(DEBUG_ASSERT_TRAP_CODE));

    let all_elems_size = builder.ins().ireduce(ir::types::I32, all_elems_size);
    let base_size = builder
        .ins()
        .iconst(ir::types::I32, i64::from(array_layout.base_size));
    let obj_size = builder.ins().uadd_overflow_trap(
        all_elems_size,
        base_size,
        ir::TrapCode::User(DEBUG_ASSERT_TRAP_CODE),
    );

    let one_elem_size = builder.ins().ireduce(ir::types::I32, one_elem_size);

    ArraySizeInfo {
        obj_size,
        one_elem_size,
        base_size,
    }
}

/// Get the bounds-checked address of an element in an array.
///
/// The emitted code will trap if `index >= array.length`.
///
/// Returns the `ir::Value` containing the address of the `index`th element in
/// the array. You may read or write a value of the array's element type at this
/// address. You may not use it for any other kind of access, nor reuse this
/// value across GC safepoints.
fn array_elem_addr(
    func_env: &mut FuncEnvironment<'_>,
    builder: &mut FunctionBuilder<'_>,
    array_type_index: ModuleInternedTypeIndex,
    array_ref: ir::Value,
    index: ir::Value,
) -> ir::Value {
    // First, assert that `index < array.length`.
    //
    // This check is visible at the Wasm-semantics level.
    //
    // TODO: We should emit spectre-safe bounds checks for array accesses (if
    // configured) but we don't currently have a great way to do that here. The
    // proper solution is to use linear memories to back GC heaps and reuse the
    // code in `bounds_check.rs` to implement these bounds checks. That is all
    // planned, but not yet implemented.

    let len = translate_array_len(func_env, builder, array_ref).unwrap();

    let in_bounds = builder.ins().icmp(IntCC::UnsignedLessThan, index, len);
    builder
        .ins()
        .trapz(in_bounds, ir::TrapCode::ArrayOutOfBounds);

    // Compute the size (in bytes) of the whole array object.
    let ArraySizeInfo {
        obj_size,
        one_elem_size,
        base_size,
    } = emit_array_size_info(func_env, builder, array_type_index, len);

    // Compute the offset of the `index`th element within the array object.
    //
    // NB: no need to check for overflow here, since at this point we know that
    // `len * elem_size + base_size` did not overflow and `i < len`.
    let offset_in_elems = builder.ins().imul(index, one_elem_size);
    let offset_in_array = builder.ins().iadd(offset_in_elems, base_size);

    // Finally, use the object size and element offset we just computed to
    // perform our implementation-internal bounds checks.
    //
    // Checking the whole object's size, rather than the `index`th element's
    // size allows these bounds checks to be deduplicated across repeated
    // accesses to the same array at different indices.
    //
    // This check should not be visible to Wasm, and serve to protect us from
    // our own implementation bugs. The goal is to keep any potential widgets
    // confined within the GC heap, and turn what would otherwise be a security
    // vulnerability into a simple bug.
    //
    // TODO: Ideally we should fold the first Wasm-visible bounds check into
    // this internal bounds check, so that we aren't performing multiple,
    // redundant bounds checks. But we should figure out how to do this in a way
    // that doesn't defeat the object-size bounds checking's deduplication
    // mentioned above.
    func_env.prepare_gc_ref_access(
        builder,
        array_ref,
        Offset::Dynamic(offset_in_array),
        BoundsCheck::Object(obj_size),
    )
}

pub fn translate_array_get(
    func_env: &mut FuncEnvironment<'_>,
    builder: &mut FunctionBuilder,
    array_type_index: TypeIndex,
    array_ref: ir::Value,
    index: ir::Value,
) -> WasmResult<ir::Value> {
    let array_type_index = func_env.module.types[array_type_index];
    let elem_addr = array_elem_addr(func_env, builder, array_type_index, array_ref, index);

    let array_ty = func_env.types[array_type_index]
        .composite_type
        .unwrap_array();
    let elem_ty = array_ty.0.element_type;

    read_field_at_addr(func_env, builder, elem_ty, elem_addr, None)
}

pub fn translate_array_get_s(
    func_env: &mut FuncEnvironment<'_>,
    builder: &mut FunctionBuilder,
    array_type_index: TypeIndex,
    array_ref: ir::Value,
    index: ir::Value,
) -> WasmResult<ir::Value> {
    let array_type_index = func_env.module.types[array_type_index];
    let elem_addr = array_elem_addr(func_env, builder, array_type_index, array_ref, index);

    let array_ty = func_env.types[array_type_index]
        .composite_type
        .unwrap_array();
    let elem_ty = array_ty.0.element_type;

    read_field_at_addr(func_env, builder, elem_ty, elem_addr, Some(Extension::Sign))
}

pub fn translate_array_get_u(
    func_env: &mut FuncEnvironment<'_>,
    builder: &mut FunctionBuilder,
    array_type_index: TypeIndex,
    array_ref: ir::Value,
    index: ir::Value,
) -> WasmResult<ir::Value> {
    let array_type_index = func_env.module.types[array_type_index];
    let elem_addr = array_elem_addr(func_env, builder, array_type_index, array_ref, index);

    let array_ty = func_env.types[array_type_index]
        .composite_type
        .unwrap_array();
    let elem_ty = array_ty.0.element_type;

    read_field_at_addr(func_env, builder, elem_ty, elem_addr, Some(Extension::Zero))
}

pub fn translate_array_set(
    func_env: &mut FuncEnvironment<'_>,
    builder: &mut FunctionBuilder,
    array_type_index: TypeIndex,
    array_ref: ir::Value,
    index: ir::Value,
    value: ir::Value,
) -> WasmResult<()> {
    let array_type_index = func_env.module.types[array_type_index];
    let elem_addr = array_elem_addr(func_env, builder, array_type_index, array_ref, index);

    let array_ty = func_env.types[array_type_index]
        .composite_type
        .unwrap_array();
    let elem_ty = array_ty.0.element_type;

    write_field_at_addr(func_env, builder, elem_ty, elem_addr, value)
}

/// A static or dynamic offset from a GC reference.
enum Offset {
    /// A static offset from a GC reference.
    Static(u32),

    /// A dynamic `i32` offset from a GC reference.
    Dynamic(ir::Value),
}

/// The kind of bounds check to perform when accessing a GC object's fields and
/// elements.
enum BoundsCheck {
    /// Check that this whole object is inside the GC heap:
    ///
    /// ```ignore
    /// gc_ref + size <= gc_heap_bound
    /// ```
    ///
    /// The object size must be an `i32` value.
    Object(ir::Value),

    /// Check that this one access in particular is inside the GC heap:
    ///
    /// ```ignore
    /// gc_ref + offset + access_size <= gc_heap_bound
    /// ```
    ///
    /// Prefer `Bound::Object` over `Bound::Access` when possible, as that
    /// approach allows the mid-end to deduplicate bounds checks across multiple
    /// accesses to the same object.
    Access(u32),
}

fn uextend_i32_to_pointer_type(
    builder: &mut FunctionBuilder,
    pointer_type: ir::Type,
    value: ir::Value,
) -> ir::Value {
    assert_eq!(builder.func.dfg.value_type(value), ir::types::I32);
    match pointer_type {
        ir::types::I32 => value,
        ir::types::I64 => builder.ins().uextend(ir::types::I64, value),
        _ => unreachable!(),
    }
}

impl FuncEnvironment<'_> {
    fn gc_layout(&mut self, type_index: ModuleInternedTypeIndex) -> &GcLayout {
        // Lazily compute and cache the layout.
        if !self.ty_to_gc_layout.contains_key(&type_index) {
            let ty = &self.types[type_index].composite_type;
            let layout = gc_compiler(self)
                .unwrap()
                .layouts()
                .gc_layout(ty)
                .expect("should only call `FuncEnvironment::gc_layout` for GC types");
            self.ty_to_gc_layout.insert(type_index, layout);
        }

        self.ty_to_gc_layout.get(&type_index).unwrap()
    }

    /// Get the `GcArrayLayout` for the array type at the given `type_index`.
    fn array_layout(&mut self, type_index: ModuleInternedTypeIndex) -> &GcArrayLayout {
        self.gc_layout(type_index).unwrap_array()
    }

    /// Get the `GcStructLayout` for the struct type at the given `type_index`.
    fn struct_layout(&mut self, type_index: ModuleInternedTypeIndex) -> &GcStructLayout {
        self.gc_layout(type_index).unwrap_struct()
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
    /// If `check` is a `BoundsCheck::Object`, then it is the callers
    /// responsibility to ensure that `offset + access_size <= object_size`.
    ///
    /// Returns a raw pointer to `gc_ref[offset]` -- not a raw pointer to the GC
    /// object itself (unless `offset` happens to be `0`). This raw pointer may
    /// be used to read or write up to as many bytes as described by `bound`. Do
    /// NOT attempt accesses bytes outside of `bound`; that may lead to
    /// unchecked out-of-bounds accesses.
    ///
    /// This method is collector-agnostic.
    fn prepare_gc_ref_access(
        &mut self,
        builder: &mut FunctionBuilder,
        gc_ref: ir::Value,
        offset: Offset,
        check: BoundsCheck,
    ) -> ir::Value {
        assert_eq!(builder.func.dfg.value_type(gc_ref), ir::types::I32);

        let pointer_type = self.pointer_type();
        let (base, bound) = self.get_gc_heap_base_bound(builder);
        let index = uextend_i32_to_pointer_type(builder, pointer_type, gc_ref);

        let offset = match offset {
            Offset::Dynamic(offset) => uextend_i32_to_pointer_type(builder, pointer_type, offset),
            Offset::Static(offset) => builder
                .ins()
                .iconst(pointer_type, i64::try_from(offset).unwrap()),
        };

        let index_and_offset = builder.ins().uadd_overflow_trap(
            index,
            offset,
            ir::TrapCode::User(crate::DEBUG_ASSERT_TRAP_CODE),
        );

        let end = match check {
            BoundsCheck::Object(object_size) => {
                // Check that `index + object_size` is in bounds. This can be
                // deduplicated across multiple accesses to different fields
                // within the same object.
                let object_size = uextend_i32_to_pointer_type(builder, pointer_type, object_size);
                builder.ins().uadd_overflow_trap(
                    index,
                    object_size,
                    ir::TrapCode::User(crate::DEBUG_ASSERT_TRAP_CODE),
                )
            }
            BoundsCheck::Access(access_size) => {
                // Check that `index + offset + access_size` is in bounds.
                let access_size = builder
                    .ins()
                    .iconst(pointer_type, i64::try_from(access_size).unwrap());
                builder.ins().uadd_overflow_trap(
                    index_and_offset,
                    access_size,
                    ir::TrapCode::User(crate::DEBUG_ASSERT_TRAP_CODE),
                )
            }
        };

        let is_in_bounds =
            builder
                .ins()
                .icmp(ir::condcodes::IntCC::UnsignedLessThanOrEqual, end, bound);
        builder.ins().trapz(
            is_in_bounds,
            ir::TrapCode::User(crate::DEBUG_ASSERT_TRAP_CODE),
        );

        // NB: No need to check for overflow here, as that would mean that the
        // GC heap is hanging off the end of the address space, which is
        // impossible.
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
