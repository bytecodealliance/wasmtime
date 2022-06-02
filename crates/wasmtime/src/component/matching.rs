use crate::component::linker::{Definition, NameMap, Strings};
use crate::types::matching;
use crate::Module;
use anyhow::{anyhow, bail, Context, Result};
use wasmtime_environ::component::{ComponentInstanceType, ComponentTypes, ModuleType, TypeDef};

pub struct TypeChecker<'a> {
    pub types: &'a ComponentTypes,
    pub strings: &'a Strings,
}

impl TypeChecker<'_> {
    pub fn definition(&self, expected: &TypeDef, actual: &Definition) -> Result<()> {
        match *expected {
            TypeDef::Module(t) => match actual {
                Definition::Module(actual) => self.module(&self.types[t], actual),
                _ => bail!("expected module found {}", actual.desc()),
            },
            TypeDef::ComponentInstance(t) => match actual {
                Definition::Instance(actual) => self.instance(&self.types[t], actual),
                _ => bail!("expected instance found {}", actual.desc()),
            },
            TypeDef::Func(_) => bail!("expected func found {}", actual.desc()),
            TypeDef::Component(_) => bail!("expected component found {}", actual.desc()),
            TypeDef::Interface(_) => bail!("expected type found {}", actual.desc()),
        }
    }

    fn module(&self, expected: &ModuleType, actual: &Module) -> Result<()> {
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

    fn instance(&self, expected: &ComponentInstanceType, actual: &NameMap) -> Result<()> {
        // Like modules, every export in the expected type must be present in
        // the actual type. It's ok, though, to have extra exports in the actual
        // type.
        for (name, expected) in expected.exports.iter() {
            let actual = self
                .strings
                .lookup(name)
                .and_then(|name| actual.get(&name))
                .ok_or_else(|| anyhow!("instance export `{name}` not defined"))?;
            self.definition(expected, actual)
                .with_context(|| format!("instance export `{name}` has the wrong type"))?;
        }
        Ok(())
    }
}

impl Definition {
    fn desc(&self) -> &'static str {
        match self {
            Definition::Module(_) => "module",
            Definition::Instance(_) => "instance",
        }
    }
}
