use crate::linker::Definition;
use crate::store::StoreOpaque;
use crate::{signatures::SignatureCollection, Engine, Extern};
use anyhow::{bail, Result};
use wasmtime_environ::{EntityType, Global, Memory, SignatureIndex, Table, WasmFuncType, WasmType};
use wasmtime_jit::TypeTables;
use wasmtime_runtime::VMSharedSignatureIndex;

pub struct MatchCx<'a> {
    pub signatures: &'a SignatureCollection,
    pub types: &'a TypeTables,
    pub store: &'a StoreOpaque,
    pub engine: &'a Engine,
}

impl MatchCx<'_> {
    pub fn global(&self, expected: &Global, actual: &crate::Global) -> Result<()> {
        self.global_ty(expected, actual.wasmtime_ty(self.store.store_data()))
    }

    fn global_ty(&self, expected: &Global, actual: &Global) -> Result<()> {
        match_ty(expected.wasm_ty, actual.wasm_ty, "global")?;
        match_bool(
            expected.mutability,
            actual.mutability,
            "global",
            "mutable",
            "immutable",
        )?;
        Ok(())
    }

    pub fn table(&self, expected: &Table, actual: &crate::Table) -> Result<()> {
        self.table_ty(
            expected,
            actual.wasmtime_ty(self.store.store_data()),
            Some(actual.internal_size(self.store)),
        )
    }

    fn table_ty(
        &self,
        expected: &Table,
        actual: &Table,
        actual_runtime_size: Option<u32>,
    ) -> Result<()> {
        match_ty(expected.wasm_ty, actual.wasm_ty, "table")?;
        match_limits(
            expected.minimum.into(),
            expected.maximum.map(|i| i.into()),
            actual_runtime_size.unwrap_or(actual.minimum).into(),
            actual.maximum.map(|i| i.into()),
            "table",
        )?;
        Ok(())
    }

    pub fn memory(&self, expected: &Memory, actual: &crate::Memory) -> Result<()> {
        self.memory_ty(
            expected,
            actual.wasmtime_ty(self.store.store_data()),
            Some(actual.internal_size(self.store)),
        )
    }

    fn memory_ty(
        &self,
        expected: &Memory,
        actual: &Memory,
        actual_runtime_size: Option<u64>,
    ) -> Result<()> {
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

    pub fn func(&self, expected: SignatureIndex, actual: &crate::Func) -> Result<()> {
        self.vmshared_signature_index(expected, actual.sig_index(self.store.store_data()))
    }

    pub(crate) fn host_func(
        &self,
        expected: SignatureIndex,
        actual: &crate::func::HostFunc,
    ) -> Result<()> {
        self.vmshared_signature_index(expected, actual.sig_index())
    }

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
        let expected = &self.types.wasm_signatures[expected];
        let actual = match self.engine.signatures().lookup_type(actual) {
            Some(ty) => ty,
            None => {
                debug_assert!(false, "all signatures should be registered");
                bail!("{}", msg);
            }
        };

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
        bail!(
            "{}: expected func of type {}, found func of type {}",
            msg,
            render(expected),
            render(&actual)
        )
    }

    /// Validates that the `expected` type matches the type of `actual`
    pub fn extern_(&self, expected: &EntityType, actual: &Extern) -> Result<()> {
        match expected {
            EntityType::Global(expected) => match actual {
                Extern::Global(actual) => self.global(expected, actual),
                _ => bail!("expected global, but found {}", actual.desc()),
            },
            EntityType::Table(expected) => match actual {
                Extern::Table(actual) => self.table(expected, actual),
                _ => bail!("expected table, but found {}", actual.desc()),
            },
            EntityType::Memory(expected) => match actual {
                Extern::Memory(actual) => self.memory(expected, actual),
                _ => bail!("expected memory, but found {}", actual.desc()),
            },
            EntityType::Function(expected) => match actual {
                Extern::Func(actual) => self.func(*expected, actual),
                _ => bail!("expected func, but found {}", actual.desc()),
            },
            EntityType::Tag(_) => unimplemented!(),
        }
    }

    /// Validates that the `expected` type matches the type of `actual`
    pub(crate) fn definition(&self, expected: &EntityType, actual: &Definition) -> Result<()> {
        match actual {
            Definition::Extern(e) => self.extern_(expected, e),
            Definition::HostFunc(f) => match expected {
                EntityType::Function(expected) => self.host_func(*expected, f),
                _ => bail!("expected {}, but found func", entity_desc(expected)),
            },
        }
    }
}

fn match_ty(expected: WasmType, actual: WasmType, desc: &str) -> Result<()> {
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
