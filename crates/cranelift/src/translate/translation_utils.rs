//! Helper functions and structures for the translation.
use crate::func_environ::FuncEnvironment;
use crate::translate::environ::TargetEnvironment;
use core::u32;
use cranelift_codegen::ir;
use cranelift_frontend::FunctionBuilder;
use smallvec::SmallVec;
use wasmparser::{FuncValidator, WasmModuleResources};
use wasmtime_environ::{TypeConvert, WasmResult};

/// Get the parameter and result types for the given Wasm blocktype.
pub fn blocktype_params_results<'a, T>(
    validator: &'a FuncValidator<T>,
    ty: wasmparser::BlockType,
) -> WasmResult<(
    impl ExactSizeIterator<Item = wasmparser::ValType> + Clone + 'a,
    impl ExactSizeIterator<Item = wasmparser::ValType> + Clone + 'a,
)>
where
    T: WasmModuleResources,
{
    return Ok(match ty {
        wasmparser::BlockType::Empty => (
            itertools::Either::Left(std::iter::empty()),
            itertools::Either::Left(None.into_iter()),
        ),
        wasmparser::BlockType::Type(ty) => (
            itertools::Either::Left(std::iter::empty()),
            itertools::Either::Left(Some(ty).into_iter()),
        ),
        wasmparser::BlockType::FuncType(ty_index) => {
            let ty = validator
                .resources()
                .sub_type_at(ty_index)
                .expect("should be valid")
                .unwrap_func();

            (
                itertools::Either::Right(ty.params().iter().copied()),
                itertools::Either::Right(ty.results().iter().copied()),
            )
        }
    });
}

/// Set the parameter `Variable`s of `destination` to `values` ahead of an
/// argument-less branch to that block.
pub fn set_block_params(
    environ: &FuncEnvironment<'_>,
    builder: &mut FunctionBuilder,
    destination: ir::Block,
    values: &[ir::Value],
) {
    let vars = &environ.stacks.block_param_vars[destination];
    debug_assert_eq!(vars.len(), values.len());
    for (var, val) in vars.iter().zip(values) {
        builder.def_var(*var, *val);
    }
}

/// Create a `Block` representing a Wasm control-flow target with the given Wasm
/// stack parameters.
///
/// Rather than giving the block CLIF block parameters, we create a
/// `cranelift_frontend::Variable` for each Wasm stack parameter and record the
/// block-to-variables mapping in `environ.stacks.block_param_vars`. See the
/// `block_param_vars` docs for more details.
pub fn block_with_params(
    builder: &mut FunctionBuilder,
    params: impl IntoIterator<Item = wasmparser::ValType>,
    environ: &mut FuncEnvironment<'_>,
) -> WasmResult<ir::Block> {
    let block = builder.create_block();
    let mut vars = SmallVec::<[_; 6]>::new();
    for ty in params {
        let (clif_ty, needs_stack_map) = match ty {
            wasmparser::ValType::I32 => (ir::types::I32, false),
            wasmparser::ValType::I64 => (ir::types::I64, false),
            wasmparser::ValType::F32 => (ir::types::F32, false),
            wasmparser::ValType::F64 => (ir::types::F64, false),
            wasmparser::ValType::Ref(rt) => {
                let hty = environ.convert_heap_type(rt.heap_type())?;
                environ.reference_type(hty)
            }
            wasmparser::ValType::V128 => (ir::types::I8X16, false),
        };
        let var = builder.declare_var(clif_ty);
        if needs_stack_map {
            builder.declare_var_needs_stack_map(var);
        }
        vars.push(var);
    }
    let old = environ.stacks.block_param_vars.insert(block, vars);
    debug_assert!(old.is_none());
    Ok(block)
}

/// Turns a `wasmparser` `f32` into a `Cranelift` one.
pub fn f32_translation(x: wasmparser::Ieee32) -> ir::immediates::Ieee32 {
    ir::immediates::Ieee32::with_bits(x.bits())
}

/// Turns a `wasmparser` `f64` into a `Cranelift` one.
pub fn f64_translation(x: wasmparser::Ieee64) -> ir::immediates::Ieee64 {
    ir::immediates::Ieee64::with_bits(x.bits())
}

/// Special VMContext value label. It is tracked as 0xffff_fffe label.
pub fn get_vmctx_value_label() -> ir::ValueLabel {
    const VMCTX_LABEL: u32 = 0xffff_fffe;
    ir::ValueLabel::from_u32(VMCTX_LABEL)
}
