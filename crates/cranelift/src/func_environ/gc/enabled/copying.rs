//! Compiler for the copying collector.
//!
//! This is a skeleton implementation that is not yet functional. All methods
//! return `WasmError::Unsupported` errors.

use super::*;
use crate::func_environ::FuncEnvironment;
use cranelift_codegen::ir;
use cranelift_frontend::FunctionBuilder;
use wasmtime_environ::{
    GcTypeLayouts, TypeIndex, WasmRefType, WasmResult, copying::CopyingTypeLayouts,
    wasm_unsupported,
};

#[derive(Default)]
pub struct CopyingCompiler {
    layouts: CopyingTypeLayouts,
}

impl GcCompiler for CopyingCompiler {
    fn layouts(&self) -> &dyn GcTypeLayouts {
        &self.layouts
    }

    fn alloc_array(
        &mut self,
        _func_env: &mut FuncEnvironment<'_>,
        _builder: &mut FunctionBuilder<'_>,
        _array_type_index: TypeIndex,
        _init: super::ArrayInit<'_>,
    ) -> WasmResult<ir::Value> {
        Err(wasm_unsupported!(
            "copying collector is not yet implemented"
        ))
    }

    fn alloc_struct(
        &mut self,
        _func_env: &mut FuncEnvironment<'_>,
        _builder: &mut FunctionBuilder<'_>,
        _struct_type_index: TypeIndex,
        _field_vals: &[ir::Value],
    ) -> WasmResult<ir::Value> {
        Err(wasm_unsupported!(
            "copying collector is not yet implemented"
        ))
    }

    fn alloc_exn(
        &mut self,
        _func_env: &mut FuncEnvironment<'_>,
        _builder: &mut FunctionBuilder<'_>,
        _tag_index: TagIndex,
        _field_vals: &[ir::Value],
        _instance_id: ir::Value,
        _tag: ir::Value,
    ) -> WasmResult<ir::Value> {
        Err(wasm_unsupported!(
            "copying collector is not yet implemented"
        ))
    }

    fn translate_read_gc_reference(
        &mut self,
        _func_env: &mut FuncEnvironment<'_>,
        _builder: &mut FunctionBuilder,
        _ty: WasmRefType,
        _src: ir::Value,
        _flags: ir::MemFlags,
    ) -> WasmResult<ir::Value> {
        Err(wasm_unsupported!(
            "copying collector is not yet implemented"
        ))
    }

    fn translate_write_gc_reference(
        &mut self,
        _func_env: &mut FuncEnvironment<'_>,
        _builder: &mut FunctionBuilder,
        _ty: WasmRefType,
        _dst: ir::Value,
        _new_val: ir::Value,
        _flags: ir::MemFlags,
    ) -> WasmResult<()> {
        Err(wasm_unsupported!(
            "copying collector is not yet implemented"
        ))
    }
}
