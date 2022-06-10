//! Helper functions and structures for the translation.
use crate::environ::TargetEnvironment;
use crate::WasmResult;
use core::convert::TryInto;
use core::u32;
use cranelift_codegen::ir;
use cranelift_frontend::FunctionBuilder;
#[cfg(feature = "enable-serde")]
use serde::{Deserialize, Serialize};
use wasmparser::{FuncValidator, WasmFuncType, WasmModuleResources};

/// WebAssembly table element. Can be a function or a scalar type.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
#[cfg_attr(feature = "enable-serde", derive(Serialize, Deserialize))]
pub enum TableElementType {
    /// A scalar type.
    Val(ir::Type),
    /// A function.
    Func,
}

/// Helper function translating wasmparser types to Cranelift types when possible.
pub fn type_to_type<PE: TargetEnvironment + ?Sized>(
    ty: wasmparser::ValType,
    environ: &PE,
) -> WasmResult<ir::Type> {
    match ty {
        wasmparser::ValType::I32 => Ok(ir::types::I32),
        wasmparser::ValType::I64 => Ok(ir::types::I64),
        wasmparser::ValType::F32 => Ok(ir::types::F32),
        wasmparser::ValType::F64 => Ok(ir::types::F64),
        wasmparser::ValType::V128 => Ok(ir::types::I8X16),
        wasmparser::ValType::ExternRef | wasmparser::ValType::FuncRef => {
            Ok(environ.reference_type(ty.try_into()?))
        }
    }
}

/// Helper function translating wasmparser possible table types to Cranelift types when possible,
/// or None for Func tables.
pub fn tabletype_to_type<PE: TargetEnvironment + ?Sized>(
    ty: wasmparser::ValType,
    environ: &PE,
) -> WasmResult<Option<ir::Type>> {
    match ty {
        wasmparser::ValType::I32 => Ok(Some(ir::types::I32)),
        wasmparser::ValType::I64 => Ok(Some(ir::types::I64)),
        wasmparser::ValType::F32 => Ok(Some(ir::types::F32)),
        wasmparser::ValType::F64 => Ok(Some(ir::types::F64)),
        wasmparser::ValType::V128 => Ok(Some(ir::types::I8X16)),
        wasmparser::ValType::ExternRef => Ok(Some(environ.reference_type(ty.try_into()?))),
        wasmparser::ValType::FuncRef => Ok(None),
    }
}

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
        wasmparser::BlockType::Empty => {
            let params: &'static [wasmparser::ValType] = &[];
            let results: &'static [wasmparser::ValType] = &[];
            (
                itertools::Either::Left(params.iter().copied()),
                itertools::Either::Left(results.iter().copied()),
            )
        }
        wasmparser::BlockType::Type(ty) => {
            let params: &'static [wasmparser::ValType] = &[];
            let results: &'static [wasmparser::ValType] = match ty {
                wasmparser::ValType::I32 => &[wasmparser::ValType::I32],
                wasmparser::ValType::I64 => &[wasmparser::ValType::I64],
                wasmparser::ValType::F32 => &[wasmparser::ValType::F32],
                wasmparser::ValType::F64 => &[wasmparser::ValType::F64],
                wasmparser::ValType::V128 => &[wasmparser::ValType::V128],
                wasmparser::ValType::ExternRef => &[wasmparser::ValType::ExternRef],
                wasmparser::ValType::FuncRef => &[wasmparser::ValType::FuncRef],
            };
            (
                itertools::Either::Left(params.iter().copied()),
                itertools::Either::Left(results.iter().copied()),
            )
        }
        wasmparser::BlockType::FuncType(ty_index) => {
            let ty = validator
                .resources()
                .func_type_at(ty_index)
                .expect("should be valid");
            (
                itertools::Either::Right(ty.inputs()),
                itertools::Either::Right(ty.outputs()),
            )
        }
    });
}

/// Create a `Block` with the given Wasm parameters.
pub fn block_with_params<PE: TargetEnvironment + ?Sized>(
    builder: &mut FunctionBuilder,
    params: impl IntoIterator<Item = wasmparser::ValType>,
    environ: &PE,
) -> WasmResult<ir::Block> {
    let block = builder.create_block();
    for ty in params {
        match ty {
            wasmparser::ValType::I32 => {
                builder.append_block_param(block, ir::types::I32);
            }
            wasmparser::ValType::I64 => {
                builder.append_block_param(block, ir::types::I64);
            }
            wasmparser::ValType::F32 => {
                builder.append_block_param(block, ir::types::F32);
            }
            wasmparser::ValType::F64 => {
                builder.append_block_param(block, ir::types::F64);
            }
            wasmparser::ValType::ExternRef | wasmparser::ValType::FuncRef => {
                builder.append_block_param(block, environ.reference_type(ty.try_into()?));
            }
            wasmparser::ValType::V128 => {
                builder.append_block_param(block, ir::types::I8X16);
            }
        }
    }
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
