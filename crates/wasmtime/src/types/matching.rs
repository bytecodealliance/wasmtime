use crate::instance::InstanceData;
use crate::linker::Definition;
use crate::store::StoreInnermost;
use crate::{signatures::SignatureCollection, Engine, Extern};
use anyhow::{bail, Context, Result};
use wasmtime_environ::wasm::{
    EntityType, Global, InstanceTypeIndex, Memory, ModuleTypeIndex, SignatureIndex, Table,
};
use wasmtime_jit::TypeTables;
use wasmtime_runtime::VMSharedSignatureIndex;

pub struct MatchCx<'a> {
    pub signatures: &'a SignatureCollection,
    pub types: &'a TypeTables,
    pub store: &'a StoreInnermost,
    pub engine: &'a Engine,
}

impl MatchCx<'_> {
    pub fn global(&self, expected: &Global, actual: &crate::Global) -> Result<()> {
        self.global_ty(expected, actual.wasmtime_ty(self.store.store_data()))
    }

    fn global_ty(&self, expected: &Global, actual: &Global) -> Result<()> {
        if expected.ty == actual.ty
            && expected.wasm_ty == actual.wasm_ty
            && expected.mutability == actual.mutability
        {
            Ok(())
        } else {
            bail!("global types incompatible")
        }
    }

    pub fn table(&self, expected: &Table, actual: &crate::Table) -> Result<()> {
        self.table_ty(expected, actual.wasmtime_ty(self.store.store_data()))
    }

    fn table_ty(&self, expected: &Table, actual: &Table) -> Result<()> {
        if expected.wasm_ty == actual.wasm_ty
            && expected.ty == actual.ty
            && expected.minimum <= actual.minimum
            && match expected.maximum {
                Some(expected) => match actual.maximum {
                    Some(actual) => expected >= actual,
                    None => false,
                },
                None => true,
            }
        {
            Ok(())
        } else {
            bail!("table types incompatible")
        }
    }

    pub fn memory(&self, expected: &Memory, actual: &crate::Memory) -> Result<()> {
        self.memory_ty(expected, actual.wasmtime_ty(self.store.store_data()))
    }

    fn memory_ty(&self, expected: &Memory, actual: &Memory) -> Result<()> {
        if expected.shared == actual.shared
            && expected.minimum <= actual.minimum
            && match expected.maximum {
                Some(expected) => match actual.maximum {
                    Some(actual) => expected >= actual,
                    None => false,
                },
                None => true,
            }
        {
            Ok(())
        } else {
            bail!("memory types incompatible")
        }
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
            Ok(())
        } else {
            bail!("function types incompatible")
        }
    }

    pub fn instance(&self, expected: InstanceTypeIndex, actual: &crate::Instance) -> Result<()> {
        for (name, expected) in self.types.instance_signatures[expected].exports.iter() {
            match actual.data(self.store.store_data()) {
                InstanceData::Synthetic(names) => match names.get(name) {
                    Some(item) => {
                        self.extern_(expected, item)
                            .with_context(|| format!("instance export {:?} incompatible", name))?;
                    }
                    None => bail!("instance type missing export {:?}", name),
                },
                InstanceData::Instantiated {
                    id,
                    types,
                    signatures,
                    ..
                } => {
                    let module = self.store.instance(*id).module();
                    match module.exports.get(name) {
                        Some(index) => {
                            let actual_ty = module.type_of(*index);
                            self.extern_ty_matches(expected, &actual_ty, signatures, types)
                                .with_context(|| {
                                    format!("instance export {:?} incompatible", name)
                                })?;
                        }
                        None => bail!("instance type missing export {:?}", name),
                    }

                    // let
                }
            }
        }
        Ok(())
    }

    /// Validates that the type signature of `actual` matches the `expected`
    /// module type signature.
    pub fn module(&self, expected: ModuleTypeIndex, actual: &crate::Module) -> Result<()> {
        // This should only ever be invoked with module linking, and this is an
        // early check that our `field` assertion below should always work as
        // well.
        assert!(self.engine.config().features.module_linking);

        let expected_sig = &self.types.module_signatures[expected];
        let module = actual.compiled_module().module();
        self.imports_match(
            expected,
            actual.signatures(),
            actual.types(),
            module.imports().map(|(name, field, ty)| {
                assert!(field.is_none()); // should be true if module linking is enabled
                (name, ty)
            }),
        )?;
        self.exports_match(
            expected_sig.exports,
            actual.signatures(),
            actual.types(),
            |name| module.exports.get(name).map(|idx| module.type_of(*idx)),
        )?;
        Ok(())
    }

    /// Validates that the `actual_imports` list of module imports matches the
    /// `expected` module type signature.
    ///
    /// Types specified in `actual_imports` are relative to `actual_types`.
    fn imports_match<'a>(
        &self,
        expected: ModuleTypeIndex,
        actual_signatures: &SignatureCollection,
        actual_types: &TypeTables,
        actual_imports: impl Iterator<Item = (&'a str, EntityType)>,
    ) -> Result<()> {
        // Imports match if all of the actual imports are satisfied by the
        // expected set of imports. Note that we're reversing the order of the
        // subtytpe matching here too.
        let expected_sig = &self.types.module_signatures[expected];
        for (name, actual_ty) in actual_imports {
            let expected_ty = match expected_sig.imports.get(name) {
                Some(ty) => ty,
                None => bail!("expected type doesn't import {:?}", name),
            };
            MatchCx {
                signatures: actual_signatures,
                types: actual_types,
                store: self.store,
                engine: self.engine,
            }
            .extern_ty_matches(&actual_ty, expected_ty, self.signatures, self.types)
            .with_context(|| format!("module import {:?} incompatible", name))?;
        }
        Ok(())
    }

    /// Validates that all exports in `expected` are defined by `lookup` within
    /// `actual_types`.
    fn exports_match(
        &self,
        expected: InstanceTypeIndex,
        actual_signatures: &SignatureCollection,
        actual_types: &TypeTables,
        lookup: impl Fn(&str) -> Option<EntityType>,
    ) -> Result<()> {
        // The `expected` type must be a subset of `actual`, meaning that all
        // names in `expected` must be present in `actual`. Note that we do
        // name-based lookup here instead of index-based lookup.
        for (name, expected) in self.types.instance_signatures[expected].exports.iter() {
            match lookup(name) {
                Some(ty) => self
                    .extern_ty_matches(expected, &ty, actual_signatures, actual_types)
                    .with_context(|| format!("export {:?} incompatible", name))?,
                None => bail!("failed to find export {:?}", name),
            }
        }
        Ok(())
    }

    /// Validates that the `expected` entity matches the `actual_ty` defined
    /// within `actual_types`.
    fn extern_ty_matches(
        &self,
        expected: &EntityType,
        actual_ty: &EntityType,
        actual_signatures: &SignatureCollection,
        actual_types: &TypeTables,
    ) -> Result<()> {
        let actual_desc = match actual_ty {
            EntityType::Global(_) => "global",
            EntityType::Module(_) => "module",
            EntityType::Memory(_) => "memory",
            EntityType::Tag(_) => "tag",
            EntityType::Instance(_) => "instance",
            EntityType::Table(_) => "table",
            EntityType::Function(_) => "function",
        };
        match expected {
            EntityType::Global(expected) => match actual_ty {
                EntityType::Global(actual) => self.global_ty(expected, actual),
                _ => bail!("expected global, but found {}", actual_desc),
            },
            EntityType::Table(expected) => match actual_ty {
                EntityType::Table(actual) => self.table_ty(expected, actual),
                _ => bail!("expected table, but found {}", actual_desc),
            },
            EntityType::Memory(expected) => match actual_ty {
                EntityType::Memory(actual) => self.memory_ty(expected, actual),
                _ => bail!("expected memory, but found {}", actual_desc),
            },
            EntityType::Function(expected) => match *actual_ty {
                EntityType::Function(actual) => {
                    if self.types.wasm_signatures[*expected] == actual_types.wasm_signatures[actual]
                    {
                        Ok(())
                    } else {
                        bail!("function types incompatible")
                    }
                }
                _ => bail!("expected function, but found {}", actual_desc),
            },
            EntityType::Instance(expected) => match actual_ty {
                EntityType::Instance(actual) => {
                    let sig = &actual_types.instance_signatures[*actual];
                    self.exports_match(*expected, actual_signatures, actual_types, |name| {
                        sig.exports.get(name).cloned()
                    })?;
                    Ok(())
                }
                _ => bail!("expected instance, but found {}", actual_desc),
            },
            EntityType::Module(expected) => match actual_ty {
                EntityType::Module(actual) => {
                    let expected_module_sig = &self.types.module_signatures[*expected];
                    let actual_module_sig = &actual_types.module_signatures[*actual];
                    let actual_instance_sig =
                        &actual_types.instance_signatures[actual_module_sig.exports];

                    self.imports_match(
                        *expected,
                        actual_signatures,
                        actual_types,
                        actual_module_sig
                            .imports
                            .iter()
                            .map(|(module, ty)| (module.as_str(), ty.clone())),
                    )?;
                    self.exports_match(
                        expected_module_sig.exports,
                        actual_signatures,
                        actual_types,
                        |name| actual_instance_sig.exports.get(name).cloned(),
                    )?;
                    Ok(())
                }
                _ => bail!("expected module, but found {}", actual_desc),
            },
            EntityType::Tag(_) => unimplemented!(),
        }
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
            EntityType::Instance(expected) => match actual {
                Extern::Instance(actual) => self.instance(*expected, actual),
                _ => bail!("expected instance, but found {}", actual.desc()),
            },
            EntityType::Module(expected) => match actual {
                Extern::Module(actual) => self.module(*expected, actual),
                _ => bail!("expected module, but found {}", actual.desc()),
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
            Definition::Instance(items) => match expected {
                EntityType::Instance(expected) => {
                    for (name, expected) in self.types.instance_signatures[*expected].exports.iter()
                    {
                        match items.get(name) {
                            Some(item) => {
                                self.definition(expected, item).with_context(|| {
                                    format!("instance export {:?} incompatible", name)
                                })?;
                            }
                            None => bail!("instance type missing export {:?}", name),
                        }
                    }
                    Ok(())
                }
                _ => bail!("expected {}, but found instance", entity_desc(expected)),
            },
        }
    }
}

fn entity_desc(ty: &EntityType) -> &'static str {
    match ty {
        EntityType::Global(_) => "global",
        EntityType::Table(_) => "table",
        EntityType::Memory(_) => "memory",
        EntityType::Function(_) => "func",
        EntityType::Instance(_) => "instance",
        EntityType::Module(_) => "module",
        EntityType::Tag(_) => "tag",
    }
}
