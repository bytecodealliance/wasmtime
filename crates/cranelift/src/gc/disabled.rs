//! `GcCompiler` implementation when GC support is disabled.

use super::GcCompiler;
use crate::func_environ::FuncEnvironment;
use cranelift_codegen::ir;
use cranelift_frontend::FunctionBuilder;
use cranelift_wasm::{wasm_unsupported, WasmHeapType, WasmRefType, WasmResult};
use wasmtime_environ::{GcTypeLayouts, TypeIndex};

/// Get the default GC compiler.
pub fn gc_compiler(_: &FuncEnvironment<'_>) -> Box<dyn GcCompiler> {
    Box::new(DisabledGcCompiler)
}

pub fn gc_ref_table_grow_builtin(
    ty: WasmHeapType,
    _func_env: &mut FuncEnvironment<'_>,
    _func: &mut ir::Function,
) -> WasmResult<ir::FuncRef> {
    Err(wasm_unsupported!(
        "support for `{ty}` references disabled at compile time because the `gc` cargo \
         feature was not enabled"
    ))
}

pub fn gc_ref_table_fill_builtin(
    ty: WasmHeapType,
    _func_env: &mut FuncEnvironment<'_>,
    _func: &mut ir::Function,
) -> WasmResult<ir::FuncRef> {
    Err(wasm_unsupported!(
        "support for `{ty}` references disabled at compile time because the `gc` cargo \
         feature was not enabled"
    ))
}

pub fn translate_struct_new(
    _func_env: &mut FuncEnvironment<'_>,
    _builder: &mut FunctionBuilder<'_>,
    _struct_type_index: TypeIndex,
    _fields: &[ir::Value],
) -> WasmResult<ir::Value> {
    Err(wasm_unsupported!(
        "support for GC references disabled at compile time because the `gc` cargo \
         feature was not enabled"
    ))
}

pub fn translate_struct_new_default(
    _func_env: &mut FuncEnvironment<'_>,
    _builder: &mut FunctionBuilder<'_>,
    _struct_type_index: TypeIndex,
) -> WasmResult<ir::Value> {
    Err(wasm_unsupported!(
        "support for GC references disabled at compile time because the `gc` cargo \
         feature was not enabled"
    ))
}

pub fn translate_struct_get(
    _func_env: &mut FuncEnvironment<'_>,
    _builder: &mut FunctionBuilder<'_>,
    _struct_type_index: TypeIndex,
    _field_index: u32,
    _struct_ref: ir::Value,
) -> WasmResult<ir::Value> {
    Err(wasm_unsupported!(
        "support for GC references disabled at compile time because the `gc` cargo \
         feature was not enabled"
    ))
}

pub fn translate_struct_get_s(
    _func_env: &mut FuncEnvironment<'_>,
    _builder: &mut FunctionBuilder<'_>,
    _struct_type_index: TypeIndex,
    _field_index: u32,
    _struct_ref: ir::Value,
) -> WasmResult<ir::Value> {
    Err(wasm_unsupported!(
        "support for GC references disabled at compile time because the `gc` cargo \
         feature was not enabled"
    ))
}

pub fn translate_struct_get_u(
    _func_env: &mut FuncEnvironment<'_>,
    _builder: &mut FunctionBuilder<'_>,
    _struct_type_index: TypeIndex,
    _field_index: u32,
    _struct_ref: ir::Value,
) -> WasmResult<ir::Value> {
    Err(wasm_unsupported!(
        "support for GC references disabled at compile time because the `gc` cargo \
         feature was not enabled"
    ))
}

pub fn translate_struct_set(
    _func_env: &mut FuncEnvironment<'_>,
    _builder: &mut FunctionBuilder<'_>,
    _struct_type_index: TypeIndex,
    _field_index: u32,
    _struct_ref: ir::Value,
    _new_val: ir::Value,
) -> WasmResult<()> {
    Err(wasm_unsupported!(
        "support for GC references disabled at compile time because the `gc` cargo \
         feature was not enabled"
    ))
}

struct DisabledGcCompiler;

impl GcCompiler for DisabledGcCompiler {
    fn layouts(&self) -> &dyn GcTypeLayouts {
        panic!(
            "support for GC types disabled at compile time because the `gc` cargo \
             feature was not enabled"
        )
    }

    fn alloc_struct(
        &mut self,
        _func_env: &mut FuncEnvironment<'_>,
        _builder: &mut FunctionBuilder<'_>,
        _struct_type_index: TypeIndex,
        _fields: &[ir::Value],
    ) -> WasmResult<ir::Value> {
        Err(wasm_unsupported!(
            "support for GC types disabled at compile time because the `gc` cargo \
             feature was not enabled"
        ))
    }

    fn translate_read_gc_reference(
        &mut self,
        _func_env: &mut FuncEnvironment<'_>,
        _builder: &mut FunctionBuilder,
        ty: WasmRefType,
        _src: ir::Value,
        _flags: ir::MemFlags,
    ) -> WasmResult<ir::Value> {
        Err(wasm_unsupported!(
            "support for `{ty}` disabled at compile time because the `gc` cargo \
             feature was not enabled"
        ))
    }

    fn translate_write_gc_reference(
        &mut self,
        _func_env: &mut FuncEnvironment<'_>,
        _builder: &mut FunctionBuilder,
        ty: WasmRefType,
        _dst: ir::Value,
        _new_val: ir::Value,
        _flags: ir::MemFlags,
    ) -> WasmResult<()> {
        Err(wasm_unsupported!(
            "support for `{ty}` disabled at compile time because the `gc` cargo \
             feature was not enabled"
        ))
    }
}
