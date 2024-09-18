//! `GcCompiler` implementation when GC support is disabled.

use super::GcCompiler;
use crate::func_environ::FuncEnvironment;
use cranelift_codegen::ir;
use cranelift_frontend::FunctionBuilder;
use cranelift_wasm::{wasm_unsupported, WasmHeapType, WasmResult};
use wasmtime_environ::TypeIndex;

fn disabled<T>() -> WasmResult<T> {
    Err(wasm_unsupported!(
        "support for Wasm GC disabled at compile time because the `gc` cargo \
         feature was not enabled"
    ))
}

/// Get the default GC compiler.
pub fn gc_compiler(_: &FuncEnvironment<'_>) -> WasmResult<Box<dyn GcCompiler>> {
    disabled()
}

pub fn gc_ref_table_grow_builtin(
    _ty: WasmHeapType,
    _func_env: &mut FuncEnvironment<'_>,
    _func: &mut ir::Function,
) -> WasmResult<ir::FuncRef> {
    disabled()
}

pub fn gc_ref_table_fill_builtin(
    _ty: WasmHeapType,
    _func_env: &mut FuncEnvironment<'_>,
    _func: &mut ir::Function,
) -> WasmResult<ir::FuncRef> {
    disabled()
}

pub fn translate_struct_new(
    _func_env: &mut FuncEnvironment<'_>,
    _builder: &mut FunctionBuilder<'_>,
    _struct_type_index: TypeIndex,
    _fields: &[ir::Value],
) -> WasmResult<ir::Value> {
    disabled()
}

pub fn translate_struct_new_default(
    _func_env: &mut FuncEnvironment<'_>,
    _builder: &mut FunctionBuilder<'_>,
    _struct_type_index: TypeIndex,
) -> WasmResult<ir::Value> {
    disabled()
}

pub fn translate_struct_get(
    _func_env: &mut FuncEnvironment<'_>,
    _builder: &mut FunctionBuilder<'_>,
    _struct_type_index: TypeIndex,
    _field_index: u32,
    _struct_ref: ir::Value,
) -> WasmResult<ir::Value> {
    disabled()
}

pub fn translate_struct_get_s(
    _func_env: &mut FuncEnvironment<'_>,
    _builder: &mut FunctionBuilder<'_>,
    _struct_type_index: TypeIndex,
    _field_index: u32,
    _struct_ref: ir::Value,
) -> WasmResult<ir::Value> {
    disabled()
}

pub fn translate_struct_get_u(
    _func_env: &mut FuncEnvironment<'_>,
    _builder: &mut FunctionBuilder<'_>,
    _struct_type_index: TypeIndex,
    _field_index: u32,
    _struct_ref: ir::Value,
) -> WasmResult<ir::Value> {
    disabled()
}

pub fn translate_struct_set(
    _func_env: &mut FuncEnvironment<'_>,
    _builder: &mut FunctionBuilder<'_>,
    _struct_type_index: TypeIndex,
    _field_index: u32,
    _struct_ref: ir::Value,
    _new_val: ir::Value,
) -> WasmResult<()> {
    disabled()
}
