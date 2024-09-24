use crate::type_registry::RegisteredType;
use crate::{linker::DefinitionType, Engine, FuncType};
use crate::{prelude::*, ArrayType, StructType};
use wasmtime_environ::{
    EntityType, Global, IndexType, Memory, ModuleTypes, Table, TypeTrace, VMSharedTypeIndex,
    WasmHeapType, WasmRefType, WasmSubType, WasmValType,
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
            let expected = RegisteredType::root(self.engine, expected).unwrap();
            let actual = RegisteredType::root(self.engine, actual).unwrap();
            if expected.is_array() && actual.is_array() {
                let expected = ArrayType::from_registered_type(expected);
                let actual = ArrayType::from_registered_type(actual);
                actual.matches(&expected)
            } else if expected.is_func() && actual.is_func() {
                let expected = FuncType::from_registered_type(expected);
                let actual = FuncType::from_registered_type(actual);
                actual.matches(&expected)
            } else if expected.is_struct() && actual.is_struct() {
                let expected = StructType::from_registered_type(expected);
                let actual = StructType::from_registered_type(actual);
                actual.matches(&expected)
            } else {
                false
            }
        };
        if matches {
            return Ok(());
        }
        let msg = "types incompatible";
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
    anyhow!("{msg}: expected type `{expected}`, found type `{actual}`")
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

fn table_ty(expected: &Table, actual: &Table, actual_runtime_size: Option<u64>) -> Result<()> {
    equal_ty(
        WasmValType::Ref(expected.ref_type),
        WasmValType::Ref(actual.ref_type),
        "table",
    )?;
    match_index(expected.idx_type, actual.idx_type, "table")?;
    match_limits(
        expected.limits.min,
        expected.limits.max,
        actual_runtime_size.unwrap_or(actual.limits.min),
        actual.limits.max,
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
    match_index(expected.idx_type, actual.idx_type, "memory")?;
    match_limits(
        expected.limits.min,
        expected.limits.max,
        actual_runtime_size.unwrap_or(actual.limits.min),
        actual.limits.max,
        "memory",
    )?;
    if expected.page_size_log2 != actual.page_size_log2 {
        bail!(
            "memory types incompatible: expected a memory with a page size of \
             {}, but received a memory with a page size of {}",
            expected.page_size(),
            actual.page_size(),
        )
    }
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

        (H::Extern | H::NoExtern, H::Extern) => true,
        (_, H::Extern) => false,

        (H::NoExtern, H::NoExtern) => true,
        (_, H::NoExtern) => false,

        (
            H::Any
            | H::Eq
            | H::I31
            | H::Array
            | H::ConcreteArray(_)
            | H::Struct
            | H::ConcreteStruct(_)
            | H::None,
            H::Any,
        ) => true,
        (_, H::Any) => false,

        (
            H::Eq
            | H::I31
            | H::Array
            | H::ConcreteArray(_)
            | H::Struct
            | H::ConcreteStruct(_)
            | H::None,
            H::Eq,
        ) => true,
        (_, H::Eq) => false,

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

fn match_index(expected: IndexType, actual: IndexType, desc: &str) -> Result<()> {
    if expected == actual {
        return Ok(());
    }
    const S64: &str = "64-bit";
    const S32: &str = "32-bit";
    let expected = if matches!(expected, IndexType::I64) {
        S64
    } else {
        S32
    };
    let actual = if matches!(actual, IndexType::I64) {
        S64
    } else {
        S32
    };
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
