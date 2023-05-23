use crate::linker::DefinitionType;
use crate::{signatures::SignatureCollection, Engine};
use anyhow::{anyhow, bail, Result};
use wasmtime_environ::{
    EntityType, Global, Memory, ModuleTypes, SignatureIndex, Table, WasmFuncType, WasmHeapType,
    WasmRefType, WasmType,
};
use wasmtime_runtime::VMSharedSignatureIndex;

pub struct MatchCx<'a> {
    pub signatures: &'a SignatureCollection,
    pub types: &'a ModuleTypes,
    pub engine: &'a Engine,
}

impl MatchCx<'_> {
    pub fn vmshared_signature_index(
        &self,
        expected: SignatureIndex,
        actual: VMSharedSignatureIndex,
    ) -> Result<()> {
        let matches = match self.signatures.shared_signature(expected) {
            Some(idx) => actual == idx,
            // If our expected signature isn't registered, then there's no way
            // that `actual` can match it.
            None => false,
        };
        if matches {
            return Ok(());
        }
        let msg = "function types incompatible";
        let expected = &self.types[expected];
        let actual = match self.engine.signatures().lookup_type(actual) {
            Some(ty) => ty,
            None => {
                debug_assert!(false, "all signatures should be registered");
                bail!("{}", msg);
            }
        };

        Err(func_ty_mismatch(msg, expected, &actual))
    }

    /// Validates that the `expected` type matches the type of `actual`
    pub(crate) fn definition(&self, expected: &EntityType, actual: &DefinitionType) -> Result<()> {
        match expected {
            EntityType::Global(expected) => match actual {
                DefinitionType::Global(actual) => global_ty(expected, actual),
                _ => bail!("expected global, but found {}", actual.desc()),
            },
            EntityType::Table(expected) => match actual {
                DefinitionType::Table(actual, cur_size) => {
                    table_ty(expected, actual, Some(*cur_size))
                }
                _ => bail!("expected table, but found {}", actual.desc()),
            },
            EntityType::Memory(expected) => match actual {
                DefinitionType::Memory(actual, cur_size) => {
                    memory_ty(expected, actual, Some(*cur_size))
                }
                _ => bail!("expected memory, but found {}", actual.desc()),
            },
            EntityType::Function(expected) => match actual {
                DefinitionType::Func(actual) => self.vmshared_signature_index(*expected, *actual),
                _ => bail!("expected func, but found {}", actual.desc()),
            },
            EntityType::Tag(_) => unimplemented!(),
        }
    }
}

#[cfg_attr(not(feature = "component-model"), allow(dead_code))]
pub fn entity_ty(
    expected: &EntityType,
    expected_types: &ModuleTypes,
    actual: &EntityType,
    actual_types: &ModuleTypes,
) -> Result<()> {
    match expected {
        EntityType::Memory(expected) => match actual {
            EntityType::Memory(actual) => memory_ty(expected, actual, None),
            _ => bail!("expected memory found {}", entity_desc(actual)),
        },
        EntityType::Global(expected) => match actual {
            EntityType::Global(actual) => global_ty(expected, actual),
            _ => bail!("expected global found {}", entity_desc(actual)),
        },
        EntityType::Table(expected) => match actual {
            EntityType::Table(actual) => table_ty(expected, actual, None),
            _ => bail!("expected table found {}", entity_desc(actual)),
        },
        EntityType::Function(expected) => match actual {
            EntityType::Function(actual) => {
                let expected = &expected_types[*expected];
                let actual = &actual_types[*actual];
                if expected == actual {
                    Ok(())
                } else {
                    Err(func_ty_mismatch(
                        "function types incompaible",
                        expected,
                        actual,
                    ))
                }
            }
            _ => bail!("expected func found {}", entity_desc(actual)),
        },
        EntityType::Tag(_) => unimplemented!(),
    }
}

fn func_ty_mismatch(msg: &str, expected: &WasmFuncType, actual: &WasmFuncType) -> anyhow::Error {
    let render = |ty: &WasmFuncType| {
        let params = ty
            .params()
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let returns = ty
            .returns()
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        format!("`({}) -> ({})`", params, returns)
    };
    anyhow!(
        "{msg}: expected func of type {}, found func of type {}",
        render(expected),
        render(actual)
    )
}

fn global_ty(expected: &Global, actual: &Global) -> Result<()> {
    // Subtyping is only sound on immutable global
    // references. Therefore if either type is mutable we perform a
    // strict equality check on the types.
    if expected.mutability || actual.mutability {
        equal_ty(expected.wasm_ty, actual.wasm_ty, "global")?;
    } else {
        match_ty(expected.wasm_ty, actual.wasm_ty, "global")?;
    }
    match_bool(
        expected.mutability,
        actual.mutability,
        "global",
        "mutable",
        "immutable",
    )?;
    Ok(())
}

fn table_ty(expected: &Table, actual: &Table, actual_runtime_size: Option<u32>) -> Result<()> {
    equal_ty(
        WasmType::Ref(expected.wasm_ty),
        WasmType::Ref(actual.wasm_ty),
        "table",
    )?;
    match_limits(
        expected.minimum.into(),
        expected.maximum.map(|i| i.into()),
        actual_runtime_size.unwrap_or(actual.minimum).into(),
        actual.maximum.map(|i| i.into()),
        "table",
    )?;
    Ok(())
}

fn memory_ty(expected: &Memory, actual: &Memory, actual_runtime_size: Option<u64>) -> Result<()> {
    match_bool(
        expected.shared,
        actual.shared,
        "memory",
        "shared",
        "non-shared",
    )?;
    match_bool(
        expected.memory64,
        actual.memory64,
        "memory",
        "64-bit",
        "32-bit",
    )?;
    match_limits(
        expected.minimum,
        expected.maximum,
        actual_runtime_size.unwrap_or(actual.minimum),
        actual.maximum,
        "memory",
    )?;
    Ok(())
}

fn match_heap(expected: WasmHeapType, actual: WasmHeapType, desc: &str) -> Result<()> {
    let result = match (actual, expected) {
        (WasmHeapType::TypedFunc(actual), WasmHeapType::TypedFunc(expected)) => {
            // TODO(dhil): we need either canonicalised types or a context here.
            actual == expected
        }
        (WasmHeapType::TypedFunc(_), WasmHeapType::Func)
        | (WasmHeapType::Func, WasmHeapType::Func)
        | (WasmHeapType::Extern, WasmHeapType::Extern) => true,
        (WasmHeapType::Func, _) | (WasmHeapType::Extern, _) | (WasmHeapType::TypedFunc(_), _) => {
            false
        }
    };
    if result {
        Ok(())
    } else {
        bail!(
            "{} types incompatible: expected {0} of type `{}`, found {0} of type `{}`",
            desc,
            expected,
            actual,
        )
    }
}

fn match_ref(expected: WasmRefType, actual: WasmRefType, desc: &str) -> Result<()> {
    if actual.nullable == expected.nullable || expected.nullable {
        return match_heap(expected.heap_type, actual.heap_type, desc);
    }
    bail!(
        "{} types incompatible: expected {0} of type `{}`, found {0} of type `{}`",
        desc,
        expected,
        actual,
    )
}

// Checks whether actual is a subtype of expected, i.e. `actual <: expected`
// (note the parameters are given the other way around in code).
fn match_ty(expected: WasmType, actual: WasmType, desc: &str) -> Result<()> {
    match (actual, expected) {
        (WasmType::Ref(actual), WasmType::Ref(expected)) => match_ref(expected, actual, desc),
        (actual, expected) => equal_ty(expected, actual, desc),
    }
}

fn equal_ty(expected: WasmType, actual: WasmType, desc: &str) -> Result<()> {
    if expected == actual {
        return Ok(());
    }
    bail!(
        "{} types incompatible: expected {0} of type `{}`, found {0} of type `{}`",
        desc,
        expected,
        actual,
    )
}

fn match_bool(
    expected: bool,
    actual: bool,
    desc: &str,
    if_true: &str,
    if_false: &str,
) -> Result<()> {
    if expected == actual {
        return Ok(());
    }
    bail!(
        "{} types incompatible: expected {} {0}, found {} {0}",
        desc,
        if expected { if_true } else { if_false },
        if actual { if_true } else { if_false },
    )
}

fn match_limits(
    expected_min: u64,
    expected_max: Option<u64>,
    actual_min: u64,
    actual_max: Option<u64>,
    desc: &str,
) -> Result<()> {
    if expected_min <= actual_min
        && match expected_max {
            Some(expected) => match actual_max {
                Some(actual) => expected >= actual,
                None => false,
            },
            None => true,
        }
    {
        return Ok(());
    }
    let limits = |min: u64, max: Option<u64>| {
        format!(
            "min: {}, max: {}",
            min,
            max.map(|s| s.to_string()).unwrap_or(String::from("none"))
        )
    };
    bail!(
        "{} types incompatible: expected {0} limits ({}) doesn't match provided {0} limits ({})",
        desc,
        limits(expected_min, expected_max),
        limits(actual_min, actual_max)
    )
}

fn entity_desc(ty: &EntityType) -> &'static str {
    match ty {
        EntityType::Global(_) => "global",
        EntityType::Table(_) => "table",
        EntityType::Memory(_) => "memory",
        EntityType::Function(_) => "func",
        EntityType::Tag(_) => "tag",
    }
}
