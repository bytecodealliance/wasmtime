use crate::component::func::HostFunc;
use crate::component::instance::RuntimeImport;
use crate::component::matching::TypeChecker;
use crate::component::{Component, Instance, InstancePre, IntoComponentFunc};
use crate::{AsContextMut, Engine, Module};
use anyhow::{anyhow, bail, Context, Result};
use std::collections::hash_map::{Entry, HashMap};
use std::marker;
use std::sync::Arc;
use wasmtime_environ::PrimaryMap;

/// A type used to instantiate [`Component`]s.
///
/// This type is used to both link components together as well as supply host
/// functionality to components. Values are defined in a [`Linker`] by their
/// import name and then components are instantiated with a [`Linker`] using the
/// names provided for name resolution of the component's imports.
pub struct Linker<T> {
    engine: Engine,
    strings: Strings,
    map: NameMap,
    allow_shadowing: bool,
    _marker: marker::PhantomData<fn() -> T>,
}

#[derive(Default)]
pub struct Strings {
    string2idx: HashMap<Arc<str>, usize>,
    strings: Vec<Arc<str>>,
}

/// Structure representing an "instance" being defined within a linker.
///
/// Instances do not need to be actual [`Instance`]s and instead are defined by
/// a "bag of named items", so each [`LinkerInstance`] can further define items
/// internally.
pub struct LinkerInstance<'a, T> {
    strings: &'a mut Strings,
    map: &'a mut NameMap,
    allow_shadowing: bool,
    _marker: marker::PhantomData<fn() -> T>,
}

pub type NameMap = HashMap<usize, Definition>;

#[derive(Clone)]
pub enum Definition {
    Instance(NameMap),
    Func(Arc<HostFunc>),
    Module(Module),
}

impl<T> Linker<T> {
    /// Creates a new linker for the [`Engine`] specified with no items defined
    /// within it.
    pub fn new(engine: &Engine) -> Linker<T> {
        Linker {
            engine: engine.clone(),
            strings: Strings::default(),
            map: NameMap::default(),
            allow_shadowing: false,
            _marker: marker::PhantomData,
        }
    }

    /// Returns the [`Engine`] this is connected to.
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// Configures whether or not name-shadowing is allowed.
    ///
    /// By default name shadowing is not allowed and it's an error to redefine
    /// the same name within a linker.
    pub fn allow_shadowing(&mut self, allow: bool) -> &mut Self {
        self.allow_shadowing = allow;
        self
    }

    /// Returns the "root instance" of this linker, used to define names into
    /// the root namespace.
    pub fn root(&mut self) -> LinkerInstance<'_, T> {
        LinkerInstance {
            strings: &mut self.strings,
            map: &mut self.map,
            allow_shadowing: self.allow_shadowing,
            _marker: self._marker,
        }
    }

    /// Returns a builder for the named instance specified.
    ///
    /// # Errors
    ///
    /// Returns an error if `name` is already defined within the linker.
    pub fn instance(&mut self, name: &str) -> Result<LinkerInstance<'_, T>> {
        self.root().into_instance(name)
    }

    /// Performs a "pre-instantiation" to resolve the imports of the
    /// [`Component`] specified with the items defined within this linker.
    ///
    /// This method will perform as much work as possible short of actually
    /// instantiating an instance. Internally this will use the names defined
    /// within this linker to satisfy the imports of the [`Component`] provided.
    /// Additionally this will perform type-checks against the component's
    /// imports against all items defined within this linker.
    ///
    /// Note that unlike internally in components where subtyping at the
    /// interface-types layer is supported this is not supported here. Items
    /// defined in this linker must match the component's imports precisely.
    ///
    /// # Errors
    ///
    /// Returns an error if this linker doesn't define a name that the
    /// `component` imports or if a name defined doesn't match the type of the
    /// item imported by the `component` provided.
    pub fn instantiate_pre(&self, component: &Component) -> Result<InstancePre<T>> {
        let cx = TypeChecker {
            types: component.types(),
            strings: &self.strings,
        };

        // Walk over the component's list of import names and use that to lookup
        // the definition within this linker that it corresponds to. When found
        // perform a typecheck against the component's expected type.
        let env_component = component.env_component();
        for (_idx, (name, ty)) in env_component.import_types.iter() {
            let import = self
                .strings
                .lookup(name)
                .and_then(|name| self.map.get(&name))
                .ok_or_else(|| anyhow!("import `{name}` not defined"))?;
            cx.definition(ty, import)
                .with_context(|| format!("import `{name}` has the wrong type"))?;
        }

        // Now that all imports are known to be defined and satisfied by this
        // linker a list of "flat" import items (aka no instances) is created
        // using the import map within the component created at
        // component-compile-time.
        let mut imports = PrimaryMap::with_capacity(env_component.imports.len());
        for (idx, (import, names)) in env_component.imports.iter() {
            let (root, _) = &env_component.import_types[*import];
            let root = self.strings.lookup(root).unwrap();

            // This is the flattening process where we go from a definition
            // optionally through a list of exported names to get to the final
            // item.
            let mut cur = &self.map[&root];
            for name in names {
                let name = self.strings.lookup(name).unwrap();
                cur = match cur {
                    Definition::Instance(map) => &map[&name],
                    _ => unreachable!(),
                };
            }
            let import = match cur {
                Definition::Module(m) => RuntimeImport::Module(m.clone()),
                Definition::Func(f) => RuntimeImport::Func(f.clone()),

                // This is guaranteed by the compilation process that "leaf"
                // runtime imports are never instances.
                Definition::Instance(_) => unreachable!(),
            };
            let i = imports.push(import);
            assert_eq!(i, idx);
        }
        Ok(unsafe { InstancePre::new_unchecked(component.clone(), imports) })
    }

    /// Instantiates the [`Component`] provided into the `store` specified.
    ///
    /// This function will use the items defined within this [`Linker`] to
    /// satisfy the imports of the [`Component`] provided as necessary. For more
    /// information about this see [`Linker::instantiate_pre`] as well.
    ///
    /// # Errors
    ///
    /// Returns an error if this [`Linker`] doesn't define an import that
    /// `component` requires or if it is of the wrong type. Additionally this
    /// can return an error if something goes wrong during instantiation such as
    /// a runtime trap or a runtime limit being exceeded.
    pub fn instantiate(
        &self,
        store: impl AsContextMut<Data = T>,
        component: &Component,
    ) -> Result<Instance> {
        self.instantiate_pre(component)?.instantiate(store)
    }
}

impl<T> LinkerInstance<'_, T> {
    fn as_mut(&mut self) -> LinkerInstance<'_, T> {
        LinkerInstance {
            strings: self.strings,
            map: self.map,
            allow_shadowing: self.allow_shadowing,
            _marker: self._marker,
        }
    }

    /// Defines a new host-provided function into this [`Linker`].
    ///
    /// This method is used to give host functions to wasm components. The
    /// `func` provided will be callable from linked components with the type
    /// signature dictated by `Params` and `Return`. The `Params` is a tuple of
    /// types that will come from wasm and `Return` is a value coming from the
    /// host going back to wasm.
    ///
    /// The [`IntoComponentFunc`] trait is implemented for functions whose
    /// arguments and return values implement the
    /// [`ComponentType`](crate::component::ComponentType) trait. Additionally
    /// the `func` may take a [`StoreContextMut`](crate::StoreContextMut) as its
    /// first parameter.
    ///
    /// Note that `func` must be an `Fn` and must also be `Send + Sync +
    /// 'static`. Shared state within a func is typically accessed with the `T`
    /// type parameter from [`Store<T>`](crate::Store) which is accessible
    /// through the leading [`StoreContextMut<'_, T>`](crate::StoreContextMut)
    /// argument which can be provided to the `func` given here.
    //
    // TODO: needs more words and examples
    pub fn func_wrap<Params, Return>(
        &mut self,
        name: &str,
        func: impl IntoComponentFunc<T, Params, Return>,
    ) -> Result<()> {
        let name = self.strings.intern(name);
        self.insert(name, Definition::Func(func.into_host_func()))
    }

    /// Defines a [`Module`] within this instance.
    ///
    /// This can be used to provide a core wasm [`Module`] as an import to a
    /// component. The [`Module`] provided is saved within the linker for the
    /// specified `name` in this instance.
    pub fn module(&mut self, name: &str, module: &Module) -> Result<()> {
        let name = self.strings.intern(name);
        self.insert(name, Definition::Module(module.clone()))
    }

    /// Defines a nested instance within this instance.
    ///
    /// This can be used to describe arbitrarily nested levels of instances
    /// within a linker to satisfy nested instance exports of components.
    pub fn instance(&mut self, name: &str) -> Result<LinkerInstance<'_, T>> {
        self.as_mut().into_instance(name)
    }

    /// Same as [`LinkerInstance::instance`] except with different lifetime
    /// parameters.
    pub fn into_instance(mut self, name: &str) -> Result<Self> {
        let name = self.strings.intern(name);
        let item = Definition::Instance(NameMap::default());
        let slot = match self.map.entry(name) {
            Entry::Occupied(_) if !self.allow_shadowing => {
                bail!("import of `{}` defined twice", self.strings.strings[name])
            }
            Entry::Occupied(o) => {
                let slot = o.into_mut();
                *slot = item;
                slot
            }
            Entry::Vacant(v) => v.insert(item),
        };
        self.map = match slot {
            Definition::Instance(map) => map,
            _ => unreachable!(),
        };
        Ok(self)
    }

    fn insert(&mut self, key: usize, item: Definition) -> Result<()> {
        match self.map.entry(key) {
            Entry::Occupied(_) if !self.allow_shadowing => {
                bail!("import of `{}` defined twice", self.strings.strings[key])
            }
            Entry::Occupied(mut e) => {
                e.insert(item);
            }
            Entry::Vacant(v) => {
                v.insert(item);
            }
        }
        Ok(())
    }
}

impl Strings {
    fn intern(&mut self, string: &str) -> usize {
        if let Some(idx) = self.string2idx.get(string) {
            return *idx;
        }
        let string: Arc<str> = string.into();
        let idx = self.strings.len();
        self.strings.push(string.clone());
        self.string2idx.insert(string, idx);
        idx
    }

    pub fn lookup(&self, string: &str) -> Option<usize> {
        self.string2idx.get(string).cloned()
    }
}
