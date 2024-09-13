//! `GcCompiler` implementation when GC support is disabled.

use super::GcCompiler;
use crate::func_environ::FuncEnvironment;
use cranelift_codegen::ir;
use cranelift_frontend::FunctionBuilder;
use cranelift_wasm::{wasm_unsupported, WasmHeapType, WasmRefType, WasmResult};

/// Get the default GC compiler.
pub fn gc_compiler(_: &FuncEnvironment<'_>) -> Box<dyn GcCompiler> {
    Box::new(DisabledGcCompiler)
}

pub fn unbarriered_load_gc_ref(
    _func_env: &FuncEnvironment<'_>,
    _builder: &mut FunctionBuilder,
    ty: WasmHeapType,
    _ptr_to_gc_ref: ir::Value,
    _flags: ir::MemFlags,
) -> WasmResult<ir::Value> {
    Err(wasm_unsupported!(
        "support for `{ty}` references disabled at compile time because the `gc` cargo \
         feature was not enabled"
    ))
}

pub fn unbarriered_store_gc_ref(
    _func_env: &FuncEnvironment<'_>,
    _builder: &mut FunctionBuilder<'_>,
    ty: WasmHeapType,
    _dst: ir::Value,
    _gc_ref: ir::Value,
    _flags: ir::MemFlags,
) -> WasmResult<()> {
    Err(wasm_unsupported!(
        "support for `{ty}` references disabled at compile time because the `gc` cargo \
         feature was not enabled"
    ))
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

struct DisabledGcCompiler;

impl GcCompiler for DisabledGcCompiler {
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
