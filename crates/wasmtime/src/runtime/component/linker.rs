use crate::component::func::HostFunc;
use crate::component::instance::RuntimeImport;
use crate::component::matching::{InstanceType, TypeChecker};
use crate::component::types;
use crate::component::{
    Component, ComponentNamedList, Instance, InstancePre, Lift, Lower, ResourceType, Val,
};
use crate::{AsContextMut, Engine, Module, StoreContextMut};
use anyhow::{anyhow, bail, Context, Result};
use indexmap::IndexMap;
use std::collections::hash_map::{Entry, HashMap};
use std::future::Future;
use std::marker;
use std::ops::Deref;
use std::pin::Pin;
use std::sync::Arc;
use wasmtime_environ::component::TypeDef;
use wasmtime_environ::{EntityRef, PrimaryMap};

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
    path: Vec<usize>,
    resource_imports: usize,
    allow_shadowing: bool,
    _marker: marker::PhantomData<fn() -> T>,
}

impl<T> Clone for Linker<T> {
    fn clone(&self) -> Linker<T> {
        Linker {
            engine: self.engine.clone(),
            strings: self.strings.clone(),
            map: self.map.clone(),
            path: self.path.clone(),
            resource_imports: self.resource_imports.clone(),
            allow_shadowing: self.allow_shadowing,
            _marker: self._marker,
        }
    }
}

#[derive(Clone, Default)]
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
    engine: &'a Engine,
    path: &'a mut Vec<usize>,
    path_len: usize,
    strings: &'a mut Strings,
    map: &'a mut NameMap,
    resource_imports: &'a mut usize,
    allow_shadowing: bool,
    _marker: marker::PhantomData<fn() -> T>,
}

/// Index correlating a resource definition to the import path.
/// This is assigned by [`Linker::resource`] and may be used to associate it to
/// [`RuntimeImportIndex`](wasmtime_environ::component::RuntimeImportIndex)
/// at a later stage
///
/// [`Linker::resource`]: crate::component::LinkerInstance::resource
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ResourceImportIndex(usize);

impl EntityRef for ResourceImportIndex {
    fn new(idx: usize) -> Self {
        Self(idx)
    }

    fn index(self) -> usize {
        self.0
    }
}

impl Deref for ResourceImportIndex {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<usize> for ResourceImportIndex {
    fn from(idx: usize) -> Self {
        Self(idx)
    }
}

impl From<ResourceImportIndex> for usize {
    fn from(idx: ResourceImportIndex) -> Self {
        idx.0
    }
}

pub(crate) type NameMap = HashMap<usize, Definition>;

#[derive(Clone)]
pub(crate) enum Definition {
    Instance(NameMap),
    Func(Arc<HostFunc>),
    Module(Module),
    Resource(
        ResourceImportIndex,
        ResourceType,
        Arc<crate::func::HostFunc>,
    ),
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
            path: Vec::new(),
            resource_imports: 0,
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
            engine: &self.engine,
            path: &mut self.path,
            path_len: 0,
            strings: &mut self.strings,
            map: &mut self.map,
            resource_imports: &mut self.resource_imports,
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

    fn typecheck<'a>(&'a self, component: &'a Component) -> Result<TypeChecker<'a>> {
        let mut cx = TypeChecker {
            component: component.env_component(),
            types: component.types(),
            strings: &self.strings,
            imported_resources: Default::default(),
        };

        // Walk over the component's list of import names and use that to lookup
        // the definition within this linker that it corresponds to. When found
        // perform a typecheck against the component's expected type.
        let env_component = component.env_component();
        for (_idx, (name, ty)) in env_component.import_types.iter() {
            let import = self
                .strings
                .lookup(name)
                .and_then(|name| self.map.get(&name));
            cx.definition(ty, import)
                .with_context(|| format!("import `{name}` has the wrong type"))?;
        }
        Ok(cx)
    }

    /// Returns the [`types::Component`] corresponding to `component` with resource
    /// types imported by it replaced using imports present in [`Self`].
    pub fn substituted_component_type(&self, component: &Component) -> Result<types::Component> {
        let cx = self.typecheck(&component)?;
        Ok(types::Component::from(
            component.ty(),
            &InstanceType {
                types: cx.types,
                resources: &cx.imported_resources,
            },
        ))
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
        self.typecheck(&component)?;

        // Now that all imports are known to be defined and satisfied by this
        // linker a list of "flat" import items (aka no instances) is created
        // using the import map within the component created at
        // component-compile-time.
        let env_component = component.env_component();
        let mut imports = PrimaryMap::with_capacity(env_component.imports.len());
        let mut resource_imports = PrimaryMap::from(vec![None; self.resource_imports]);
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
                Definition::Resource(res_idx, t, dtor) => {
                    resource_imports[*res_idx] = Some(idx);
                    RuntimeImport::Resource {
                        ty: t.clone(),
                        _dtor: dtor.clone(),
                        dtor_funcref: component.resource_drop_func_ref(dtor),
                    }
                }

                // This is guaranteed by the compilation process that "leaf"
                // runtime imports are never instances.
                Definition::Instance(_) => unreachable!(),
            };
            let i = imports.push(import);
            assert_eq!(i, idx);
        }
        Ok(unsafe { InstancePre::new_unchecked(component.clone(), imports, resource_imports) })
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
        assert!(
            !store.as_context().async_support(),
            "must use async instantiation when async support is enabled"
        );
        self.instantiate_pre(component)?.instantiate(store)
    }

    /// Instantiates the [`Component`] provided into the `store` specified.
    ///
    /// This is exactly like [`Linker::instantiate`] except for async stores.
    ///
    /// # Errors
    ///
    /// Returns an error if this [`Linker`] doesn't define an import that
    /// `component` requires or if it is of the wrong type. Additionally this
    /// can return an error if something goes wrong during instantiation such as
    /// a runtime trap or a runtime limit being exceeded.
    #[cfg(feature = "async")]
    #[cfg_attr(docsrs, doc(cfg(feature = "async")))]
    pub async fn instantiate_async(
        &self,
        store: impl AsContextMut<Data = T>,
        component: &Component,
    ) -> Result<Instance>
    where
        T: Send,
    {
        assert!(
            store.as_context().async_support(),
            "must use sync instantiation when async support is disabled"
        );
        self.instantiate_pre(component)?
            .instantiate_async(store)
            .await
    }
}

impl<T> LinkerInstance<'_, T> {
    fn as_mut(&mut self) -> LinkerInstance<'_, T> {
        LinkerInstance {
            engine: self.engine,
            path: self.path,
            path_len: self.path_len,
            strings: self.strings,
            map: self.map,
            resource_imports: self.resource_imports,
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
    /// Additionally the `func` takes a
    /// [`StoreContextMut`](crate::StoreContextMut) as its first parameter.
    ///
    /// Note that `func` must be an `Fn` and must also be `Send + Sync +
    /// 'static`. Shared state within a func is typically accessed with the `T`
    /// type parameter from [`Store<T>`](crate::Store) which is accessible
    /// through the leading [`StoreContextMut<'_, T>`](crate::StoreContextMut)
    /// argument which can be provided to the `func` given here.
    //
    // TODO: needs more words and examples
    pub fn func_wrap<F, Params, Return>(&mut self, name: &str, func: F) -> Result<()>
    where
        F: Fn(StoreContextMut<T>, Params) -> Result<Return> + Send + Sync + 'static,
        Params: ComponentNamedList + Lift + 'static,
        Return: ComponentNamedList + Lower + 'static,
    {
        let name = self.strings.intern(name);
        self.insert(name, Definition::Func(HostFunc::from_closure(func)))
    }

    /// Defines a new host-provided async function into this [`Linker`].
    ///
    /// This is exactly like [`Self::func_wrap`] except it takes an async
    /// host function.
    #[cfg(feature = "async")]
    #[cfg_attr(docsrs, doc(cfg(feature = "async")))]
    pub fn func_wrap_async<Params, Return, F>(&mut self, name: &str, f: F) -> Result<()>
    where
        F: for<'a> Fn(
                StoreContextMut<'a, T>,
                Params,
            ) -> Box<dyn Future<Output = Result<Return>> + Send + 'a>
            + Send
            + Sync
            + 'static,
        Params: ComponentNamedList + Lift + 'static,
        Return: ComponentNamedList + Lower + 'static,
    {
        assert!(
            self.engine.config().async_support,
            "cannot use `func_wrap_async` without enabling async support in the config"
        );
        let ff = move |mut store: StoreContextMut<'_, T>, params: Params| -> Result<Return> {
            let async_cx = store.as_context_mut().0.async_cx().expect("async cx");
            let mut future = Pin::from(f(store.as_context_mut(), params));
            unsafe { async_cx.block_on(future.as_mut()) }?
        };
        self.func_wrap(name, ff)
    }

    /// Define a new host-provided function using dynamic types.
    ///
    /// `name` must refer to a function type import in `component`.  If and when
    /// that import is invoked by the component, the specified `func` will be
    /// called, which must return a `Val` which is an instance of the result
    /// type of the import.
    pub fn func_new<
        F: Fn(StoreContextMut<'_, T>, &[Val], &mut [Val]) -> Result<()> + Send + Sync + 'static,
    >(
        &mut self,
        component: &Component,
        name: &str,
        func: F,
    ) -> Result<()> {
        let mut map = &component
            .env_component()
            .import_types
            .values()
            .map(|(k, v)| (k.clone(), *v))
            .collect::<IndexMap<_, _>>();

        for name in self.path.iter().copied().take(self.path_len) {
            let name = self.strings.strings[name].deref();
            if let Some(ty) = map.get(name) {
                if let TypeDef::ComponentInstance(index) = ty {
                    map = &component.types()[*index].exports;
                } else {
                    bail!("import `{name}` has the wrong type (expected a component instance)");
                }
            } else {
                bail!("import `{name}` not found");
            }
        }

        if let Some(ty) = map.get(name) {
            if let TypeDef::ComponentFunc(index) = ty {
                let name = self.strings.intern(name);
                return self.insert(
                    name,
                    Definition::Func(HostFunc::new_dynamic(func, *index, component.types())),
                );
            } else {
                bail!("import `{name}` has the wrong type (expected a function)");
            }
        } else {
            Err(anyhow!("import `{name}` not found"))
        }
    }

    /// Define a new host-provided async function using dynamic types.
    ///
    /// This is exactly like [`Self::func_new`] except it takes an async
    /// host function.
    #[cfg(feature = "async")]
    #[cfg_attr(docsrs, doc(cfg(feature = "async")))]
    pub fn func_new_async<F>(&mut self, component: &Component, name: &str, f: F) -> Result<()>
    where
        F: for<'a> Fn(
                StoreContextMut<'a, T>,
                &'a [Val],
                &'a mut [Val],
            ) -> Box<dyn Future<Output = Result<()>> + Send + 'a>
            + Send
            + Sync
            + 'static,
    {
        assert!(
            self.engine.config().async_support,
            "cannot use `func_new_async` without enabling async support in the config"
        );
        let ff = move |mut store: StoreContextMut<'_, T>, params: &[Val], results: &mut [Val]| {
            let async_cx = store.as_context_mut().0.async_cx().expect("async cx");
            let mut future = Pin::from(f(store.as_context_mut(), params, results));
            unsafe { async_cx.block_on(future.as_mut()) }?
        };
        self.func_new(component, name, ff)
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

    /// Defines a new resource of a given [`ResourceType`] in this linker.
    ///
    /// This function is used to specify resources defined in the host.
    ///
    /// The `name` argument is the name to define the resource within this
    /// linker.
    ///
    /// The `dtor` provided is a destructor that will get invoked when an owned
    /// version of this resource is destroyed from the guest. Note that this
    /// destructor is not called when a host-owned resource is destroyed as it's
    /// assumed the host knows how to handle destroying its own resources.
    ///
    /// The `dtor` closure is provided the store state as the first argument
    /// along with the representation of the resource that was just destroyed.
    ///
    /// [`Resource<U>`]: crate::component::Resource
    ///
    /// # Errors
    ///
    /// The provided `dtor` closure returns an error if something goes wrong
    /// when a guest calls the `dtor` to drop a `Resource<T>` such as
    /// a runtime trap or a runtime limit being exceeded.
    pub fn resource(
        &mut self,
        name: &str,
        ty: ResourceType,
        dtor: impl Fn(StoreContextMut<'_, T>, u32) -> Result<()> + Send + Sync + 'static,
    ) -> Result<ResourceImportIndex> {
        let name = self.strings.intern(name);
        let dtor = Arc::new(crate::func::HostFunc::wrap(
            &self.engine,
            move |mut cx: crate::Caller<'_, T>, param: u32| dtor(cx.as_context_mut(), param),
        ));
        let idx = ResourceImportIndex::new(*self.resource_imports);
        *self.resource_imports = self
            .resource_imports
            .checked_add(1)
            .context("resource import count would overflow")?;
        self.insert(name, Definition::Resource(idx, ty, dtor))?;
        Ok(idx)
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
        self.path.truncate(self.path_len);
        self.path.push(name);
        self.path_len += 1;
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
