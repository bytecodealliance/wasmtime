//! Helper functions and structures for the translation.
use crate::environ::TargetEnvironment;
use crate::wasm_unsupported;
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
    ty: wasmparser::Type,
    environ: &PE,
) -> WasmResult<ir::Type> {
    match ty {
        wasmparser::Type::I32 => Ok(ir::types::I32),
        wasmparser::Type::I64 => Ok(ir::types::I64),
        wasmparser::Type::F32 => Ok(ir::types::F32),
        wasmparser::Type::F64 => Ok(ir::types::F64),
        wasmparser::Type::V128 => Ok(ir::types::I8X16),
        wasmparser::Type::ExternRef | wasmparser::Type::FuncRef => {
            Ok(environ.reference_type(ty.try_into()?))
        }
        ty => Err(wasm_unsupported!("type_to_type: wasm type {:?}", ty)),
    }
}

/// Helper function translating wasmparser possible table types to Cranelift types when possible,
/// or None for Func tables.
pub fn tabletype_to_type<PE: TargetEnvironment + ?Sized>(
    ty: wasmparser::Type,
    environ: &PE,
) -> WasmResult<Option<ir::Type>> {
    match ty {
        wasmparser::Type::I32 => Ok(Some(ir::types::I32)),
        wasmparser::Type::I64 => Ok(Some(ir::types::I64)),
        wasmparser::Type::F32 => Ok(Some(ir::types::F32)),
        wasmparser::Type::F64 => Ok(Some(ir::types::F64)),
        wasmparser::Type::V128 => Ok(Some(ir::types::I8X16)),
        wasmparser::Type::ExternRef => Ok(Some(environ.reference_type(ty.try_into()?))),
        wasmparser::Type::FuncRef => Ok(None),
        ty => Err(wasm_unsupported!(
            "tabletype_to_type: table wasm type {:?}",
            ty
        )),
    }
}

/// Get the parameter and result types for the given Wasm blocktype.
pub fn blocktype_params_results<'a, T>(
    validator: &'a FuncValidator<T>,
    ty_or_ft: wasmparser::TypeOrFuncType,
) -> WasmResult<(
    impl ExactSizeIterator<Item = wasmparser::Type> + Clone + 'a,
    impl ExactSizeIterator<Item = wasmparser::Type> + Clone + 'a,
)>
where
    T: WasmModuleResources,
{
    return Ok(match ty_or_ft {
        wasmparser::TypeOrFuncType::Type(ty) => {
            let (params, results): (&'static [wasmparser::Type], &'static [wasmparser::Type]) =
                match ty {
                    wasmparser::Type::I32 => (&[], &[wasmparser::Type::I32]),
                    wasmparser::Type::I64 => (&[], &[wasmparser::Type::I64]),
                    wasmparser::Type::F32 => (&[], &[wasmparser::Type::F32]),
                    wasmparser::Type::F64 => (&[], &[wasmparser::Type::F64]),
                    wasmparser::Type::V128 => (&[], &[wasmparser::Type::V128]),
                    wasmparser::Type::ExternRef => (&[], &[wasmparser::Type::ExternRef]),
                    wasmparser::Type::FuncRef => (&[], &[wasmparser::Type::FuncRef]),
                    wasmparser::Type::EmptyBlockType => (&[], &[]),
                    ty => return Err(wasm_unsupported!("blocktype_params_results: type {:?}", ty)),
                };
            (
                itertools::Either::Left(params.iter().copied()),
                itertools::Either::Left(results.iter().copied()),
            )
        }
        wasmparser::TypeOrFuncType::FuncType(ty_index) => {
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
    params: impl IntoIterator<Item = wasmparser::Type>,
    environ: &PE,
) -> WasmResult<ir::Block> {
    let block = builder.create_block();
    for ty in params {
        match ty {
            wasmparser::Type::I32 => {
                builder.append_block_param(block, ir::types::I32);
            }
            wasmparser::Type::I64 => {
                builder.append_block_param(block, ir::types::I64);
            }
            wasmparser::Type::F32 => {
                builder.append_block_param(block, ir::types::F32);
            }
            wasmparser::Type::F64 => {
                builder.append_block_param(block, ir::types::F64);
            }
            wasmparser::Type::ExternRef | wasmparser::Type::FuncRef => {
                builder.append_block_param(block, environ.reference_type(ty.try_into()?));
            }
            wasmparser::Type::V128 => {
                builder.append_block_param(block, ir::types::I8X16);
            }
            ty => {
                return Err(wasm_unsupported!(
                    "block_with_params: type {:?} in multi-value block's signature",
                    ty
                ))
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
