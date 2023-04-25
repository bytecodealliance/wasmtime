use crate::component::func::HostFunc;
use crate::component::instance::ImportedResources;
use crate::component::linker::{Definition, NameMap, Strings};
use crate::component::ResourceType;
use crate::store::{StoreId, StoreOpaque};
use crate::types::matching;
use crate::Module;
use anyhow::{anyhow, bail, Context, Result};
use std::sync::Arc;
use wasmtime_environ::component::{
    ComponentTypes, ResourceIndex, TypeComponentInstance, TypeDef, TypeFuncIndex, TypeModule,
    TypeResourceTableIndex,
};
use wasmtime_environ::PrimaryMap;
use wasmtime_runtime::component::ComponentInstance;

pub struct TypeChecker<'a> {
    pub types: &'a Arc<ComponentTypes>,
    pub component: &'a wasmtime_environ::component::Component,
    pub strings: &'a Strings,
    pub imported_resources: PrimaryMap<ResourceIndex, ResourceType>,
}

#[derive(Copy, Clone)]
#[doc(hidden)]
pub struct InstanceType<'a> {
    pub instance: Option<(StoreId, &'a ComponentInstance)>,
    pub types: &'a ComponentTypes,
    pub imported_resources: &'a ImportedResources,
}

impl TypeChecker<'_> {
    pub fn definition(&mut self, expected: &TypeDef, actual: Option<&Definition>) -> Result<()> {
        match *expected {
            TypeDef::Module(t) => match actual {
                Some(Definition::Module(actual)) => self.module(&self.types[t], actual),
                _ => bail!("expected module found {}", desc(actual)),
            },
            TypeDef::ComponentInstance(t) => match actual {
                Some(Definition::Instance(actual)) => self.instance(&self.types[t], Some(actual)),
                None => self.instance(&self.types[t], None),
                _ => bail!("expected instance found {}", desc(actual)),
            },
            TypeDef::ComponentFunc(t) => match actual {
                Some(Definition::Func(actual)) => self.func(t, actual),
                _ => bail!("expected func found {}", desc(actual)),
            },
            TypeDef::Component(_) => bail!("expected component found {}", desc(actual)),
            TypeDef::Interface(_) => bail!("expected type found {}", desc(actual)),

            TypeDef::Resource(i) => {
                let i = self.types[i].ty;
                match actual {
                    Some(Definition::Resource(actual, _dtor)) => {
                        match self.imported_resources.get(i) {
                            // If `i` hasn't been pushed onto `imported_resources`
                            // yet then that means that it's the first time a new
                            // resource was introduced, so record the type of this
                            // resource. It should always be the case that the next
                            // index assigned is equal to `i` since types should be
                            // checked in the same order they were assigned into the
                            // `Component` type.
                            None => {
                                let id = self.imported_resources.push(*actual);
                                assert_eq!(id, i);
                            }

                            // If `i` has been defined, however, then that means
                            // that this is an `(eq ..)` bounded type imported
                            // because it's referring to a previously defined type.
                            // In this situation it's not required to provide a type
                            // import but if it's supplied then it must be equal. In
                            // this situation it's supplied, so test for equality.
                            Some(expected) => {
                                if expected != actual {
                                    bail!("mismatched resource types");
                                }
                            }
                        }
                        Ok(())
                    }

                    // If a resource is imported yet nothing was supplied then
                    // that's only successful if the resource has itself alredy been
                    // defined. If it's already defined then that means that this is
                    // an `(eq ...)` import which is not required to be satisfied
                    // via `Linker` definitions in the Wasmtime API.
                    None if self.imported_resources.get(i).is_some() => Ok(()),

                    _ => bail!("expected resource found {}", desc(actual)),
                }
            }

            // not possible for valid components to import
            TypeDef::CoreFunc(_) => unreachable!(),
        }
    }

    fn module(&self, expected: &TypeModule, actual: &Module) -> Result<()> {
        let actual_types = actual.types();
        let actual = actual.env_module();

        // Every export that is expected should be in the actual module we have
        for (name, expected) in expected.exports.iter() {
            let idx = actual
                .exports
                .get(name)
                .ok_or_else(|| anyhow!("module export `{name}` not defined"))?;
            let actual = actual.type_of(*idx);
            matching::entity_ty(expected, self.types.module_types(), &actual, actual_types)
                .with_context(|| format!("module export `{name}` has the wrong type"))?;
        }

        // Note the opposite order of checks here. Every import that the actual
        // module expects should be imported by the expected module since the
        // expected module has the set of items given to the actual module.
        // Additionally the "matches" check is inverted here.
        for (module, name, actual) in actual.imports() {
            // TODO: shouldn't need a `.to_string()` here ideally
            let expected = expected
                .imports
                .get(&(module.to_string(), name.to_string()))
                .ok_or_else(|| anyhow!("module import `{module}::{name}` not defined"))?;
            matching::entity_ty(&actual, actual_types, expected, self.types.module_types())
                .with_context(|| format!("module import `{module}::{name}` has the wrong type"))?;
        }
        Ok(())
    }

    fn instance(
        &mut self,
        expected: &TypeComponentInstance,
        actual: Option<&NameMap>,
    ) -> Result<()> {
        // Like modules, every export in the expected type must be present in
        // the actual type. It's ok, though, to have extra exports in the actual
        // type.
        for (name, expected) in expected.exports.iter() {
            // Interface types may be exported from a component in order to give them a name, but
            // they don't have a definition in the sense that this search is interested in, so
            // ignore them.
            if let TypeDef::Interface(_) = expected {
                continue;
            }
            let actual = self
                .strings
                .lookup(name)
                .and_then(|name| actual?.get(&name));
            self.definition(expected, actual)
                .with_context(|| format!("instance export `{name}` has the wrong type"))?;
        }
        Ok(())
    }

    fn func(&self, expected: TypeFuncIndex, actual: &HostFunc) -> Result<()> {
        let instance_type = InstanceType {
            types: self.types,
            imported_resources: &self.imported_resources,
            instance: None,
        };
        actual.typecheck(expected, &instance_type)
    }
}

fn desc(def: Option<&Definition>) -> &'static str {
    match def {
        Some(def) => def.desc(),
        None => "nothing",
    }
}

impl Definition {
    fn desc(&self) -> &'static str {
        match self {
            Definition::Module(_) => "module",
            Definition::Func(_) => "func",
            Definition::Instance(_) => "instance",
            Definition::Resource(..) => "resource",
        }
    }
}

impl<'a> InstanceType<'a> {
    pub fn new(store: &StoreOpaque, instance: &'a ComponentInstance) -> InstanceType<'a> {
        InstanceType {
            instance: Some((store.id(), instance)),
            types: instance.component_types(),
            imported_resources: instance.imported_resources().downcast_ref().unwrap(),
        }
    }

    pub fn resource_type(&self, index: TypeResourceTableIndex) -> ResourceType {
        let index = self.types[index].ty;
        if let Some((store, instance)) = self.instance {
            if let Some(index) = instance.component().defined_resource_index(index) {
                return ResourceType::guest(store, instance, index);
            }
        }
        self.imported_resources[index]
    }
}
