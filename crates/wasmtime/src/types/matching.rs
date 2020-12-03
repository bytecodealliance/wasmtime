use crate::Store;
use std::sync::Arc;
use wasmtime_environ::wasm::{
    EntityType, Global, InstanceTypeIndex, Memory, ModuleTypeIndex, SignatureIndex, Table,
};
use wasmtime_jit::TypeTables;

pub struct MatchCx<'a> {
    pub types: &'a TypeTables,
    pub store: &'a Store,
}

impl MatchCx<'_> {
    pub fn global(&self, expected: &Global, actual: &crate::Global) -> bool {
        self.global_ty(expected, actual.wasmtime_ty())
    }

    fn global_ty(&self, expected: &Global, actual: &Global) -> bool {
        expected.ty == actual.ty
            && expected.wasm_ty == actual.wasm_ty
            && expected.mutability == actual.mutability
    }

    pub fn table(&self, expected: &Table, actual: &crate::Table) -> bool {
        self.table_ty(expected, actual.wasmtime_ty())
    }

    fn table_ty(&self, expected: &Table, actual: &Table) -> bool {
        expected.wasm_ty == actual.wasm_ty
            && expected.ty == actual.ty
            && expected.minimum <= actual.minimum
            && match expected.maximum {
                Some(expected) => match actual.maximum {
                    Some(actual) => expected >= actual,
                    None => false,
                },
                None => true,
            }
    }

    pub fn memory(&self, expected: &Memory, actual: &crate::Memory) -> bool {
        self.memory_ty(expected, actual.wasmtime_ty())
    }

    fn memory_ty(&self, expected: &Memory, actual: &Memory) -> bool {
        expected.shared == actual.shared
            && expected.minimum <= actual.minimum
            && match expected.maximum {
                Some(expected) => match actual.maximum {
                    Some(actual) => expected >= actual,
                    None => false,
                },
                None => true,
            }
    }

    pub fn func(&self, expected: SignatureIndex, actual: &crate::Func) -> bool {
        match self
            .store
            .signatures()
            .borrow()
            .lookup(&self.types.wasm_signatures[expected])
        {
            Some(idx) => actual.sig_index() == idx,
            // If our expected signature isn't registered, then there's no way
            // that `actual` can match it.
            None => false,
        }
    }

    pub fn instance(&self, expected: InstanceTypeIndex, actual: &crate::Instance) -> bool {
        let module = actual.handle.module();
        self.exports_match(
            expected,
            actual
                .handle
                .host_state()
                .downcast_ref::<Arc<TypeTables>>()
                .unwrap(),
            |name| module.exports.get(name).map(|idx| module.type_of(*idx)),
        )
    }

    /// Validates that the type signature of `actual` matches the `expected`
    /// module type signature.
    pub fn module(&self, expected: ModuleTypeIndex, actual: &crate::Module) -> bool {
        let expected_sig = &self.types.module_signatures[expected];
        let module = actual.compiled_module().module();
        self.imports_match(expected, actual.types(), module.imports())
            && self.exports_match(expected_sig.exports, actual.types(), |name| {
                module.exports.get(name).map(|idx| module.type_of(*idx))
            })
    }

    /// Validates that the `actual_imports` list of module imports matches the
    /// `expected` module type signature.
    ///
    /// Types specified in `actual_imports` are relative to `actual_types`.
    fn imports_match<'a>(
        &self,
        expected: ModuleTypeIndex,
        actual_types: &TypeTables,
        mut actual_imports: impl Iterator<Item = (&'a str, Option<&'a str>, EntityType)>,
    ) -> bool {
        let expected_sig = &self.types.module_signatures[expected];
        for (_, _, expected) in expected_sig.imports.iter() {
            let (_, _, ty) = match actual_imports.next() {
                Some(e) => e,
                None => return false,
            };
            if !self.extern_ty_matches(expected, &ty, actual_types) {
                return false;
            }
        }
        actual_imports.next().is_none()
    }

    /// Validates that all exports in `expected` are defined by `lookup` within
    /// `actual_types`.
    fn exports_match(
        &self,
        expected: InstanceTypeIndex,
        actual_types: &TypeTables,
        lookup: impl Fn(&str) -> Option<EntityType>,
    ) -> bool {
        // The `expected` type must be a subset of `actual`, meaning that all
        // names in `expected` must be present in `actual`. Note that we do
        // name-based lookup here instead of index-based lookup.
        self.types.instance_signatures[expected].exports.iter().all(
            |(name, expected)| match lookup(name) {
                Some(ty) => self.extern_ty_matches(expected, &ty, actual_types),
                None => false,
            },
        )
    }

    /// Validates that the `expected` entity matches the `actual_ty` defined
    /// within `actual_types`.
    fn extern_ty_matches(
        &self,
        expected: &EntityType,
        actual_ty: &EntityType,
        actual_types: &TypeTables,
    ) -> bool {
        match expected {
            EntityType::Global(expected) => match actual_ty {
                EntityType::Global(actual) => self.global_ty(expected, actual),
                _ => false,
            },
            EntityType::Table(expected) => match actual_ty {
                EntityType::Table(actual) => self.table_ty(expected, actual),
                _ => false,
            },
            EntityType::Memory(expected) => match actual_ty {
                EntityType::Memory(actual) => self.memory_ty(expected, actual),
                _ => false,
            },
            EntityType::Function(expected) => match *actual_ty {
                EntityType::Function(actual) => {
                    self.types.wasm_signatures[*expected] == actual_types.wasm_signatures[actual]
                }
                _ => false,
            },
            EntityType::Instance(expected) => match actual_ty {
                EntityType::Instance(actual) => {
                    let sig = &actual_types.instance_signatures[*actual];
                    self.exports_match(*expected, actual_types, |name| {
                        sig.exports.get(name).cloned()
                    })
                }
                _ => false,
            },
            EntityType::Module(expected) => match actual_ty {
                EntityType::Module(actual) => {
                    let expected_module_sig = &self.types.module_signatures[*expected];
                    let actual_module_sig = &actual_types.module_signatures[*actual];
                    let actual_instance_sig =
                        &actual_types.instance_signatures[actual_module_sig.exports];

                    self.imports_match(
                        *expected,
                        actual_types,
                        actual_module_sig.imports.iter().map(|(module, field, ty)| {
                            (module.as_str(), field.as_deref(), ty.clone())
                        }),
                    ) && self.exports_match(expected_module_sig.exports, actual_types, |name| {
                        actual_instance_sig.exports.get(name).cloned()
                    })
                }
                _ => false,
            },
            EntityType::Event(_) => unimplemented!(),
        }
    }
}
