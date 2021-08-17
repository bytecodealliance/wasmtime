use crate::{SignatureIndex, WasmError, WasmResult};
use cranelift_codegen::ir::{types, Type};
use cranelift_entity::PrimaryMap;
use std::boxed::Box;
use std::vec::Vec;

/// Map of signatures to a function's parameter and return types.
pub(crate) type WasmTypes =
    PrimaryMap<SignatureIndex, (Box<[wasmparser::Type]>, Box<[wasmparser::Type]>)>;

/// Contains information decoded from the Wasm module that must be referenced
/// during each Wasm function's translation.
///
/// This is only for data that is maintained by `cranelift-wasm` itself, as
/// opposed to being maintained by the embedder. Data that is maintained by the
/// embedder is represented with `ModuleEnvironment`.
#[derive(Debug)]
pub struct ModuleTranslationState {
    /// A map containing a Wasm module's original, raw signatures.
    ///
    /// This is used for translating multi-value Wasm blocks inside functions,
    /// which are encoded to refer to their type signature via index.
    pub(crate) wasm_types: WasmTypes,
}

fn cranelift_to_wasmparser_type(ty: Type) -> WasmResult<wasmparser::Type> {
    Ok(match ty {
        types::I32 => wasmparser::Type::I32,
        types::I64 => wasmparser::Type::I64,
        types::F32 => wasmparser::Type::F32,
        types::F64 => wasmparser::Type::F64,
        types::R32 | types::R64 => wasmparser::Type::ExternRef,
        _ => {
            return Err(WasmError::Unsupported(format!(
                "Cannot convert Cranelift type to Wasm signature: {:?}",
                ty
            )));
        }
    })
}

impl ModuleTranslationState {
    /// Creates a new empty ModuleTranslationState.
    pub fn new() -> Self {
        Self {
            wasm_types: PrimaryMap::new(),
        }
    }

    /// Create a new ModuleTranslationState with the given function signatures,
    /// provided in terms of Cranelift types. The provided slice of signatures
    /// is indexed by signature number, and contains pairs of (args, results)
    /// slices.
    pub fn from_func_sigs(sigs: &[(&[Type], &[Type])]) -> WasmResult<Self> {
        let mut wasm_types = PrimaryMap::with_capacity(sigs.len());
        for &(ref args, ref results) in sigs {
            let args: Vec<wasmparser::Type> = args
                .iter()
                .map(|&ty| cranelift_to_wasmparser_type(ty))
                .collect::<Result<_, _>>()?;
            let results: Vec<wasmparser::Type> = results
                .iter()
                .map(|&ty| cranelift_to_wasmparser_type(ty))
                .collect::<Result<_, _>>()?;
            wasm_types.push((args.into_boxed_slice(), results.into_boxed_slice()));
        }
        Ok(Self { wasm_types })
    }
}
