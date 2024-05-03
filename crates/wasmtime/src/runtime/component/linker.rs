use crate::component::func::HostFunc;
use crate::component::instance::RuntimeImport;
use crate::component::matching::{InstanceType, TypeChecker};
use crate::component::types;
use crate::component::{
    Component, ComponentNamedList, Instance, InstancePre, Lift, Lower, ResourceType, Val,
};
use crate::prelude::*;
use crate::{AsContextMut, Engine, Module, StoreContextMut};
use alloc::sync::Arc;
use anyhow::{bail, Context, Result};
use core::future::Future;
use core::marker;
use core::pin::Pin;
use hashbrown::hash_map::{Entry, HashMap};
use semver::Version;
use wasmtime_environ::PrimaryMap;

/// A type used to instantiate [`Component`]s.
///
/// This type is used to both link components together as well as supply host
/// functionality to components. Values are defined in a [`Linker`] by their
/// import name and then components are instantiated with a [`Linker`] using the
/// names provided for name resolution of the component's imports.
///
/// # Names and Semver
///
/// Names defined in a [`Linker`] correspond to import names in the Component
/// Model. Names in the Component Model are allowed to be semver-qualified, for
/// example:
///
/// * `wasi:cli/stdout@0.2.0`
/// * `wasi:http/types@0.2.0-rc-2023-10-25`
/// * `my:custom/plugin@1.0.0-pre.2`
///
/// These version strings are taken into account when looking up names within a
/// [`Linker`]. You're allowed to define any number of versions within a
/// [`Linker`] still, for example you can define `a:b/c@0.2.0`, `a:b/c@0.2.1`,
/// and `a:b/c@0.3.0` all at the same time.
///
/// Specifically though when names are looked up within a linker, for example
/// during instantiation, semver-compatible names are automatically consulted.
/// This means that if you define `a:b/c@0.2.1` in a [`Linker`] but a component
/// imports `a:b/c@0.2.0` then that import will resolve to the `0.2.1` version.
///
/// This lookup behavior relies on hosts being well-behaved when using Semver,
/// specifically that interfaces once defined are never changed. This reflects
/// how Semver works at the Component Model layer, and it's assumed that if
/// versions are present then hosts are respecting this.
///
/// Note that this behavior goes the other direction, too. If a component
/// imports `a:b/c@0.2.1` and the host has provided `a:b/c@0.2.0` then that
/// will also resolve correctly. This is because if an API was defined at 0.2.0
/// and 0.2.1 then it must be the same API.
///
/// This behavior is intended to make it easier for hosts to upgrade WASI and
/// for guests to upgrade WASI. So long as the actual "meat" of the
/// functionality is defined then it should align correctly and components can
/// be instantiated.
pub struct Linker<T> {
    engine: Engine,
    strings: Strings,
    map: NameMap,
    path: Vec<usize>,
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
    allow_shadowing: bool,
    _marker: marker::PhantomData<fn() -> T>,
}

#[derive(Clone, Default)]
pub(crate) struct NameMap {
    /// A map of interned strings to the name that they define.
    ///
    /// Note that this map is "exact" where the name here is the exact name that
    /// was specified when the `Linker` was configured. This doesn't have any
    /// semver-mangling or anything like that.
    ///
    /// This map is always consulted first during lookups.
    definitions: HashMap<usize, Definition>,

    /// An auxiliary map tracking semver-compatible names. This is a map from
    /// "semver compatible alternate name" to a name present in `definitions`
    /// and the semver version it was registered at.
    ///
    /// The `usize` entries here map to intern'd keys, so an example map could
    /// be:
    ///
    /// ```text
    /// {
    ///     "a:b/c@0.2": ("a:b/c@0.2.1", 0.2.1),
    ///     "a:b/c@2": ("a:b/c@2.0.0+abc", 2.0.0+abc),
    /// }
    /// ```
    ///
    /// As names are inserted into `definitions` each name may have up to one
    /// semver-compatible name with extra numbers/info chopped off which is
    /// inserted into this map. This map is the lookup table from `@0.2` to
    /// `@0.2.x` where `x` is what was inserted manually.
    ///
    /// The `Version` here is tracked to ensure that when multiple versions on
    /// one track are defined that only the maximal version here is retained.
    alternate_lookups: HashMap<usize, (usize, Version)>,
}

#[derive(Clone)]
pub(crate) enum Definition {
    Instance(NameMap),
    Func(Arc<HostFunc>),
    Module(Module),
    Resource(ResourceType, Arc<crate::func::HostFunc>),
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
            types: component.types(),
            strings: &self.strings,
            imported_resources: Default::default(),
        };

        // Walk over the component's list of import names and use that to lookup
        // the definition within this linker that it corresponds to. When found
        // perform a typecheck against the component's expected type.
        let env_component = component.env_component();
        for (_idx, (name, ty)) in env_component.import_types.iter() {
            let import = self.map.get(name, &self.strings);
            cx.definition(ty, import)
                .with_context(|| format!("component imports {desc} `{name}`, but a matching implementation was not found in the linker", desc = ty.desc()))?;
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
        for (idx, (import, names)) in env_component.imports.iter() {
            let (root, _) = &env_component.import_types[*import];

            // This is the flattening process where we go from a definition
            // optionally through a list of exported names to get to the final
            // item.
            let mut cur = self.map.get(root, &self.strings).unwrap();
            for name in names {
                cur = match cur {
                    Definition::Instance(map) => map.get(&name, &self.strings).unwrap(),
                    _ => unreachable!(),
                };
            }
            let import = match cur {
                Definition::Module(m) => RuntimeImport::Module(m.clone()),
                Definition::Func(f) => RuntimeImport::Func(f.clone()),
                Definition::Resource(t, dtor) => RuntimeImport::Resource {
                    ty: t.clone(),
                    _dtor: dtor.clone(),
                    dtor_funcref: component.resource_drop_func_ref(dtor),
                },

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
        self.insert(name, Definition::Func(HostFunc::from_closure(func)))?;
        Ok(())
    }

    /// Defines a new host-provided async function into this [`Linker`].
    ///
    /// This is exactly like [`Self::func_wrap`] except it takes an async
    /// host function.
    #[cfg(feature = "async")]
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

    /// Define a new host-provided function using dynamically typed values.
    ///
    /// The `name` provided is the name of the function to define and the
    /// `func` provided is the host-defined closure to invoke when this
    /// function is called.
    ///
    /// This function is the "dynamic" version of defining a host function as
    /// compared to [`LinkerInstance::func_wrap`]. With
    /// [`LinkerInstance::func_wrap`] a function's type is statically known but
    /// with this method the `func` argument's type isn't known ahead of time.
    /// That means that `func` can be by imported component so long as it's
    /// imported as a matching name.
    ///
    /// Type information will be available at execution time, however. For
    /// example when `func` is invoked the second argument, a `&[Val]` list,
    /// contains [`Val`] entries that say what type they are. Additionally the
    /// third argument, `&mut [Val]`, is the expected number of results. Note
    /// that the expected types of the results cannot be learned during the
    /// execution of `func`. Learning that would require runtime introspection
    /// of a component.
    ///
    /// Return values, stored in the third argument of `&mut [Val]`, are
    /// type-checked at runtime to ensure that they have the appropriate type.
    /// A trap will be raised if they do not have the right type.
    ///
    /// # Examples
    ///
    /// ```
    /// use wasmtime::{Store, Engine};
    /// use wasmtime::component::{Component, Linker, Val};
    ///
    /// # fn main() -> wasmtime::Result<()> {
    /// let engine = Engine::default();
    /// let component = Component::new(
    ///     &engine,
    ///     r#"
    ///         (component
    ///             (import "thunk" (func $thunk))
    ///             (import "is-even" (func $is-even (param "x" u32) (result bool)))
    ///
    ///             (core module $m
    ///                 (import "" "thunk" (func $thunk))
    ///                 (import "" "is-even" (func $is-even (param i32) (result i32)))
    ///
    ///                 (func (export "run")
    ///                     call $thunk
    ///
    ///                     (call $is-even (i32.const 1))
    ///                     if unreachable end
    ///
    ///                     (call $is-even (i32.const 2))
    ///                     i32.eqz
    ///                     if unreachable end
    ///                 )
    ///             )
    ///             (core func $thunk (canon lower (func $thunk)))
    ///             (core func $is-even (canon lower (func $is-even)))
    ///             (core instance $i (instantiate $m
    ///                 (with "" (instance
    ///                     (export "thunk" (func $thunk))
    ///                     (export "is-even" (func $is-even))
    ///                 ))
    ///             ))
    ///
    ///             (func (export "run") (canon lift (core func $i "run")))
    ///         )
    ///     "#,
    /// )?;
    ///
    /// let mut linker = Linker::<()>::new(&engine);
    ///
    /// // Sample function that takes no arguments.
    /// linker.root().func_new("thunk", |_store, params, results| {
    ///     assert!(params.is_empty());
    ///     assert!(results.is_empty());
    ///     println!("Look ma, host hands!");
    ///     Ok(())
    /// })?;
    ///
    /// // This function takes one argument and returns one result.
    /// linker.root().func_new("is-even", |_store, params, results| {
    ///     assert_eq!(params.len(), 1);
    ///     let param = match params[0] {
    ///         Val::U32(n) => n,
    ///         _ => panic!("unexpected type"),
    ///     };
    ///
    ///     assert_eq!(results.len(), 1);
    ///     results[0] = Val::Bool(param % 2 == 0);
    ///     Ok(())
    /// })?;
    ///
    /// let mut store = Store::new(&engine, ());
    /// let instance = linker.instantiate(&mut store, &component)?;
    /// let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;
    /// run.call(&mut store, ())?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn func_new(
        &mut self,
        name: &str,
        func: impl Fn(StoreContextMut<'_, T>, &[Val], &mut [Val]) -> Result<()> + Send + Sync + 'static,
    ) -> Result<()> {
        self.insert(name, Definition::Func(HostFunc::new_dynamic(func)))?;
        Ok(())
    }

    /// Define a new host-provided async function using dynamic types.
    ///
    /// This is exactly like [`Self::func_new`] except it takes an async
    /// host function.
    #[cfg(feature = "async")]
    pub fn func_new_async<F>(&mut self, name: &str, f: F) -> Result<()>
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
        self.func_new(name, ff)
    }

    /// Defines a [`Module`] within this instance.
    ///
    /// This can be used to provide a core wasm [`Module`] as an import to a
    /// component. The [`Module`] provided is saved within the linker for the
    /// specified `name` in this instance.
    pub fn module(&mut self, name: &str, module: &Module) -> Result<()> {
        self.insert(name, Definition::Module(module.clone()))?;
        Ok(())
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
    ) -> Result<()> {
        let dtor = Arc::new(crate::func::HostFunc::wrap(
            &self.engine,
            move |mut cx: crate::Caller<'_, T>, param: u32| dtor(cx.as_context_mut(), param),
        ));
        self.insert(name, Definition::Resource(ty, dtor))?;
        Ok(())
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
        let name = self.insert(name, Definition::Instance(NameMap::default()))?;
        self.map = match self.map.definitions.get_mut(&name) {
            Some(Definition::Instance(map)) => map,
            _ => unreachable!(),
        };
        self.path.truncate(self.path_len);
        self.path.push(name);
        self.path_len += 1;
        Ok(self)
    }

    fn insert(&mut self, name: &str, item: Definition) -> Result<usize> {
        self.map
            .insert(name, &mut self.strings, self.allow_shadowing, item)
    }
}

impl NameMap {
    /// Looks up `name` within this map, using the interning specified by
    /// `strings`.
    ///
    /// This may return a definition even if `name` wasn't exactly defined in
    /// this map, such as looking up `a:b/c@0.2.0` when the map only has
    /// `a:b/c@0.2.1` defined.
    pub(crate) fn get(&self, name: &str, strings: &Strings) -> Option<&Definition> {
        // First look up an exact match and if that's found return that. This
        // enables defining multiple versions in the map and the requested
        // version is returned if it matches exactly.
        let candidate = strings.lookup(name).and_then(|k| self.definitions.get(&k));
        if let Some(def) = candidate {
            return Some(def);
        }

        // Failing that, then try to look for a semver-compatible alternative.
        // This looks up the key based on `name`, if any, and then looks to see
        // if that was intern'd in `strings`. Given all that look to see if it
        // was defined in `alternate_lookups` and finally at the end that exact
        // key is then used to look up again in `self.definitions`.
        let (alternate_name, _version) = alternate_lookup_key(name)?;
        let alternate_key = strings.lookup(alternate_name)?;
        let (exact_key, _version) = self.alternate_lookups.get(&alternate_key)?;
        self.definitions.get(exact_key)
    }

    /// Inserts the `name` specified into this map.
    ///
    /// The name is intern'd through the `strings` argument and shadowing is
    /// controlled by the `allow_shadowing` variable.
    ///
    /// This function will automatically insert an entry in
    /// `self.alternate_lookups` if `name` is a semver-looking name.
    fn insert(
        &mut self,
        name: &str,
        strings: &mut Strings,
        allow_shadowing: bool,
        item: Definition,
    ) -> Result<usize> {
        // Always insert `name` and `item` as an exact definition.
        let key = strings.intern(name);
        match self.definitions.entry(key) {
            Entry::Occupied(_) if !allow_shadowing => {
                bail!("import of `{}` defined twice", strings.strings[key])
            }
            Entry::Occupied(mut e) => {
                e.insert(item);
            }
            Entry::Vacant(v) => {
                v.insert(item);
            }
        }

        // If `name` is a semver-looking thing, like `a:b/c@1.0.0`, then also
        // insert an entry in the semver-compatible map under a key such as
        // `a:b/c@1`.
        //
        // This key is used during `get` later on.
        if let Some((alternate_key, version)) = alternate_lookup_key(name) {
            let alternate_key = strings.intern(alternate_key);
            match self.alternate_lookups.entry(alternate_key) {
                Entry::Occupied(mut e) => {
                    let (_, prev_version) = e.get();
                    // Prefer the latest version, so only do this if we're
                    // greater than the prior version.
                    if version > *prev_version {
                        e.insert((key, version));
                    }
                }
                Entry::Vacant(v) => {
                    v.insert((key, version));
                }
            }
        }
        Ok(key)
    }
}

/// Determines a version-based "alternate lookup key" for the `name` specified.
///
/// Some examples are:
///
/// * `foo` => `None`
/// * `foo:bar/baz` => `None`
/// * `foo:bar/baz@1.1.2` => `Some(foo:bar/baz@1)`
/// * `foo:bar/baz@0.1.0` => `Some(foo:bar/baz@0.1)`
/// * `foo:bar/baz@0.0.1` => `None`
/// * `foo:bar/baz@0.1.0-rc.2` => `None`
///
/// This alternate lookup key is intended to serve the purpose where a
/// semver-compatible definition can be located, if one is defined, at perhaps
/// either a newer or an older version.
fn alternate_lookup_key(name: &str) -> Option<(&str, Version)> {
    let at = name.find('@')?;
    let version_string = &name[at + 1..];
    let version = Version::parse(version_string).ok()?;
    if !version.pre.is_empty() {
        // If there's a prerelease then don't consider that compatible with any
        // other version number.
        None
    } else if version.major != 0 {
        // If the major number is nonzero then compatibility is up to the major
        // version number, so return up to the first decimal.
        let first_dot = version_string.find('.')? + at + 1;
        Some((&name[..first_dot], version))
    } else if version.minor != 0 {
        // Like the major version if the minor is nonzero then patch releases
        // are all considered to be on a "compatible track".
        let first_dot = version_string.find('.')? + at + 1;
        let second_dot = name[first_dot + 1..].find('.')? + first_dot + 1;
        Some((&name[..second_dot], version))
    } else {
        // If the patch number is the first nonzero entry then nothing can be
        // compatible with this patch, e.g. 0.0.1 isn't' compatible with
        // any other version inherently.
        None
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

#[cfg(test)]
mod tests {
    #[test]
    fn alternate_lookup_key() {
        fn alt(s: &str) -> Option<&str> {
            super::alternate_lookup_key(s).map(|(s, _)| s)
        }

        assert_eq!(alt("x"), None);
        assert_eq!(alt("x:y/z"), None);
        assert_eq!(alt("x:y/z@1.0.0"), Some("x:y/z@1"));
        assert_eq!(alt("x:y/z@1.1.0"), Some("x:y/z@1"));
        assert_eq!(alt("x:y/z@1.1.2"), Some("x:y/z@1"));
        assert_eq!(alt("x:y/z@2.1.2"), Some("x:y/z@2"));
        assert_eq!(alt("x:y/z@2.1.2+abc"), Some("x:y/z@2"));
        assert_eq!(alt("x:y/z@0.1.2"), Some("x:y/z@0.1"));
        assert_eq!(alt("x:y/z@0.1.3"), Some("x:y/z@0.1"));
        assert_eq!(alt("x:y/z@0.2.3"), Some("x:y/z@0.2"));
        assert_eq!(alt("x:y/z@0.2.3+abc"), Some("x:y/z@0.2"));
        assert_eq!(alt("x:y/z@0.0.1"), None);
        assert_eq!(alt("x:y/z@0.0.1-pre"), None);
        assert_eq!(alt("x:y/z@0.1.0-pre"), None);
        assert_eq!(alt("x:y/z@1.0.0-pre"), None);
    }
}
