use crate::prelude::*;
use crate::{linker::DefinitionType, Engine, FuncType};
use anyhow::{anyhow, bail, Result};
use wasmtime_environ::{
    EntityType, Global, Memory, ModuleTypes, Table, TypeTrace, VMSharedTypeIndex,
    WasmCompositeType, WasmFieldType, WasmHeapType, WasmRefType, WasmSubType, WasmValType,
};

pub struct MatchCx<'a> {
    engine: &'a Engine,
}

impl MatchCx<'_> {
    /// Construct a new matching context for the given module.
    pub fn new(engine: &Engine) -> MatchCx<'_> {
        MatchCx { engine }
    }

    fn type_reference(&self, expected: VMSharedTypeIndex, actual: VMSharedTypeIndex) -> Result<()> {
        // Avoid matching on structure for subtyping checks when we have
        // precisely the same type.
        let matches = expected == actual || {
            let expected = FuncType::from_shared_type_index(self.engine, expected);
            let actual = FuncType::from_shared_type_index(self.engine, actual);
            actual.matches(&expected)
        };
        if matches {
            return Ok(());
        }
        let msg = "function types incompatible";
        let expected = match self.engine.signatures().borrow(expected) {
            Some(ty) => ty,
            None => panic!("{expected:?} is not registered"),
        };
        let actual = match self.engine.signatures().borrow(actual) {
            Some(ty) => ty,
            None => panic!("{actual:?} is not registered"),
        };

        Err(concrete_type_mismatch(msg, &expected, &actual))
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
                DefinitionType::Func(actual) => {
                    self.type_reference(expected.unwrap_engine_type_index(), *actual)
                }
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
                let expected = &expected_types[expected.unwrap_module_type_index()];
                let actual = &actual_types[actual.unwrap_module_type_index()];
                if expected == actual {
                    Ok(())
                } else {
                    Err(concrete_type_mismatch(
                        "function types incompatible",
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

fn concrete_type_mismatch(
    msg: &str,
    expected: &WasmSubType,
    actual: &WasmSubType,
) -> anyhow::Error {
    let render_field = |ty: &WasmFieldType| {
        if ty.mutable {
            format!("(mut {})", ty.element_type)
        } else {
            ty.element_type.to_string()
        }
    };

    let render = |ty: &WasmSubType| match &ty.composite_type {
        WasmCompositeType::Array(ty) => {
            format!("(array {})", render_field(&ty.0))
        }
        WasmCompositeType::Func(ty) => {
            let params = if ty.params().is_empty() {
                String::new()
            } else {
                format!(
                    " (param {})",
                    ty.params()
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                        .join(" ")
                )
            };
            let returns = if ty.returns().is_empty() {
                String::new()
            } else {
                format!(
                    " (result {})",
                    ty.returns()
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                        .join(" ")
                )
            };
            format!("(func{params}{returns})")
        }
        WasmCompositeType::Struct(ty) => {
            let mut s = "(struct".to_string();
            for f in ty.fields.iter() {
                s.push_str(&format!(" {}", render_field(f)));
            }
            s.push(')');
            s
        }
    };

    anyhow!(
        "{msg}: expected type `{}`, found type `{}`",
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
        WasmValType::Ref(expected.wasm_ty),
        WasmValType::Ref(actual.wasm_ty),
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
    use WasmHeapType as H;
    let result = match (actual, expected) {
        // TODO: Wasm GC introduces subtyping between function types, so it will
        // no longer suffice to check whether canonicalized type IDs are equal.
        (H::ConcreteArray(actual), H::ConcreteArray(expected)) => actual == expected,
        (H::ConcreteFunc(actual), H::ConcreteFunc(expected)) => actual == expected,
        (H::ConcreteStruct(actual), H::ConcreteStruct(expected)) => actual == expected,

        (H::NoFunc, H::NoFunc) => true,
        (_, H::NoFunc) => false,

        (H::NoFunc, H::ConcreteFunc(_)) => true,
        (_, H::ConcreteFunc(_)) => false,

        (H::NoFunc | H::ConcreteFunc(_) | H::Func, H::Func) => true,
        (_, H::Func) => false,

        (H::Extern, H::Extern) => true,
        (_, H::Extern) => false,

        (
            H::Any
            | H::I31
            | H::Array
            | H::ConcreteArray(_)
            | H::Struct
            | H::ConcreteStruct(_)
            | H::None,
            H::Any,
        ) => true,
        (_, H::Any) => false,

        (H::I31 | H::None, H::I31) => true,
        (_, H::I31) => false,

        (H::Array | H::ConcreteArray(_) | H::None, H::Array) => true,
        (_, H::Array) => false,

        (H::None, H::ConcreteArray(_)) => true,
        (_, H::ConcreteArray(_)) => false,

        (H::Struct | H::ConcreteStruct(_) | H::None, H::Struct) => true,
        (_, H::Struct) => false,

        (H::None, H::ConcreteStruct(_)) => true,
        (_, H::ConcreteStruct(_)) => false,

        (H::None, H::None) => true,
        (_, H::None) => false,
    };
    if result {
        Ok(())
    } else {
        bail!(
            "{desc} types incompatible: expected {desc} of type `{expected}`, \
             found {desc} of type `{actual}`",
        )
    }
}

fn match_ref(expected: WasmRefType, actual: WasmRefType, desc: &str) -> Result<()> {
    if actual.nullable == expected.nullable || expected.nullable {
        return match_heap(expected.heap_type, actual.heap_type, desc);
    }
    bail!(
        "{desc} types incompatible: expected {desc} of type `{expected}`, \
         found {desc} of type `{actual}`",
    )
}

// Checks whether actual is a subtype of expected, i.e. `actual <: expected`
// (note the parameters are given the other way around in code).
fn match_ty(expected: WasmValType, actual: WasmValType, desc: &str) -> Result<()> {
    // Assert that both our types are engine-level canonicalized. We can't
    // compare types otherwise.
    debug_assert!(
        expected.is_canonicalized_for_runtime_usage(),
        "expected type should be canonicalized for runtime usage: {expected:?}"
    );
    debug_assert!(
        actual.is_canonicalized_for_runtime_usage(),
        "actual type should be canonicalized for runtime usage: {actual:?}"
    );

    match (actual, expected) {
        (WasmValType::Ref(actual), WasmValType::Ref(expected)) => match_ref(expected, actual, desc),
        (actual, expected) => equal_ty(expected, actual, desc),
    }
}

fn equal_ty(expected: WasmValType, actual: WasmValType, desc: &str) -> Result<()> {
    // Assert that both our types are engine-level canonicalized. We can't
    // compare types otherwise.
    debug_assert!(
        expected.is_canonicalized_for_runtime_usage(),
        "expected type should be canonicalized for runtime usage: {expected:?}"
    );
    debug_assert!(
        actual.is_canonicalized_for_runtime_usage(),
        "actual type should be canonicalized for runtime usage: {actual:?}"
    );

    if expected == actual {
        return Ok(());
    }
    bail!(
        "{desc} types incompatible: expected {desc} of type `{expected}`, \
         found {desc} of type `{actual}`",
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
    let expected = if expected { if_true } else { if_false };
    let actual = if actual { if_true } else { if_false };
    bail!(
        "{desc} types incompatible: expected {expected} {desc}, \
         found {actual} {desc}",
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
