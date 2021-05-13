use crate::func::HostFunc;
use crate::instance::InstanceBuilder;
use crate::{
    AsContextMut, Caller, Engine, Extern, ExternType, Func, FuncType, ImportType, Instance,
    IntoFunc, Module, Trap, Val,
};
use anyhow::{anyhow, bail, Context, Error, Result};
use log::warn;
use std::collections::hash_map::{Entry, HashMap};
#[cfg(feature = "async")]
use std::future::Future;
use std::marker;
#[cfg(feature = "async")]
use std::pin::Pin;
use std::sync::Arc;

/// Structure used to link wasm modules/instances together.
///
/// This structure is used to assist in instantiating a [`Module`]. A `Linker`
/// is a way of performing name resolution to make instantiating a module easier
/// (as opposed to calling [`Instance::new`]). `Linker` is a name-based resolver
/// where names are dynamically defined and then used to instantiate a
/// [`Module`]. The goal of a `Linker` is to have a one-argument method,
/// [`Linker::instantiate`], which takes a [`Module`] and produces an
/// [`Instance`].  This method will automatically select all the right imports
/// for the [`Module`] to be instantiated, and will otherwise return an error
/// if an import isn't satisfied.
///
/// ## Name Resolution
///
/// As mentioned previously, `Linker` is a form of name resolver. It will be
/// using the string-based names of imports on a module to attempt to select a
/// matching item to hook up to it. This name resolution has two-levels of
/// namespaces, a module level and a name level. Each item is defined within a
/// module and then has its own name. This basically follows the wasm standard
/// for modularization.
///
/// Names in a `Linker` cannot be defined twice, but allowing duplicates by
/// shadowing the previous definition can be controlled with the
/// [`Linker::allow_shadowing`] method.
pub struct Linker<T> {
    engine: Engine,
    string2idx: HashMap<Arc<str>, usize>,
    strings: Vec<Arc<str>>,
    map: HashMap<ImportKey, Definition<T>>,
    allow_shadowing: bool,
    allow_unknown_exports: bool,
    _marker: marker::PhantomData<fn() -> T>,
}

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
struct ImportKey {
    name: usize,
    module: usize,
}

enum Definition<T> {
    Extern(Extern),
    HostFunc {
        func: Arc<HostFunc>,
        t: marker::PhantomData<fn() -> T>,
    },
}

macro_rules! generate_wrap_async_func {
    ($num:tt $($args:ident)*) => (paste::paste!{
        /// TODO
        #[allow(non_snake_case)]
        #[cfg(feature = "async")]
        #[cfg_attr(nightlydoc, doc(cfg(feature = "async")))]
        pub fn [<func_wrap $num _async>]<$($args,)* R>(
            &mut self,
            module: &str,
            name: &str,
            func: impl for<'a> Fn(Caller<'a, T>, $($args),*) -> Box<dyn Future<Output = R> + Send + 'a> + Send + Sync + 'static,
        ) -> Result<&mut Self>
        where
            $($args: crate::WasmTy,)*
            R: crate::WasmRet,
        {
            assert!(
                self.engine.config().async_support,
                concat!(
                    "cannot use `func_wrap",
                    $num,
                    "_async` without enabling async support on the config",
                ),
            );
            self.func_wrap(module, name, move |mut caller: Caller<'_, T>, $($args: $args),*| {
                let async_cx = caller.store.as_context_mut().opaque().async_cx();
                let mut future = Pin::from(func(caller, $($args),*));
                match unsafe { async_cx.block_on(future.as_mut()) } {
                    Ok(ret) => ret.into_fallible(),
                    Err(e) => R::fallible_from_trap(e),
                }
            })
        }
    })
}

impl<T> Linker<T> {
    /// Creates a new [`Linker`].
    ///
    /// This function will create a new [`Linker`] which is ready to start
    /// linking modules. All items defined in this linker and produced by this
    /// linker will be connected with `store` and must come from the same
    /// `store`.
    ///
    /// # Examples
    ///
    /// ```
    /// use wasmtime::{Linker, Store};
    ///
    /// let store = Store::default();
    /// let mut linker = Linker::new(&store);
    /// // ...
    /// ```
    pub fn new(engine: &Engine) -> Linker<T> {
        Linker {
            engine: engine.clone(),
            map: HashMap::new(),
            string2idx: HashMap::new(),
            strings: Vec::new(),
            allow_shadowing: false,
            allow_unknown_exports: false,
            _marker: marker::PhantomData,
        }
    }

    /// TODO
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// Configures whether this [`Linker`] will shadow previous duplicate
    /// definitions of the same signature.
    ///
    /// By default a [`Linker`] will disallow duplicate definitions of the same
    /// signature. This method, however, can be used to instead allow duplicates
    /// and have the latest definition take precedence when linking modules.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// # let store = Store::default();
    /// let mut linker = Linker::new(&store);
    /// linker.func("", "", || {})?;
    ///
    /// // by default, duplicates are disallowed
    /// assert!(linker.func("", "", || {}).is_err());
    ///
    /// // but shadowing can be configured to be allowed as well
    /// linker.allow_shadowing(true);
    /// linker.func("", "", || {})?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn allow_shadowing(&mut self, allow: bool) -> &mut Self {
        self.allow_shadowing = allow;
        self
    }

    /// Configures whether this [`Linker`] will allow unknown exports from
    /// command modules.
    ///
    /// By default a [`Linker`] will error when unknown exports are encountered
    /// in a command module while using [`Linker::module`].
    ///
    /// This method can be used to allow unknown exports from command modules.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// # let store = Store::default();
    /// # let module = Module::new(store.engine(), "(module)")?;
    /// let mut linker = Linker::new(&store);
    /// linker.allow_unknown_exports(true);
    /// linker.module("mod", &module)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn allow_unknown_exports(&mut self, allow: bool) -> &mut Linker {
        self.allow_unknown_exports = allow;
        self
    }

    /// Defines a new item in this [`Linker`].
    ///
    /// This method will add a new definition, by name, to this instance of
    /// [`Linker`]. The `module` and `name` provided are what to name the
    /// `item`.
    ///
    /// # Errors
    ///
    /// Returns an error if the `module` and `name` already identify an item
    /// of the same type as the `item` provided and if shadowing is disallowed.
    /// For more information see the documentation on [`Linker`].
    ///
    /// Also returns an error if `item` comes from a different store than this
    /// [`Linker`] was created with.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// # let store = Store::default();
    /// let mut linker = Linker::new(&store);
    /// let ty = GlobalType::new(ValType::I32, Mutability::Const);
    /// let global = Global::new(&store, ty, Val::I32(0x1234))?;
    /// linker.define("host", "offset", global)?;
    ///
    /// let wat = r#"
    ///     (module
    ///         (import "host" "offset" (global i32))
    ///         (memory 1)
    ///         (data (global.get 0) "foo")
    ///     )
    /// "#;
    /// let module = Module::new(store.engine(), wat)?;
    /// linker.instantiate(&module)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn define(
        &mut self,
        module: &str,
        name: &str,
        item: impl Into<Extern>,
    ) -> Result<&mut Self> {
        let key = self.import_key(module, Some(name));
        self.insert(key, Definition::Extern(item.into()))?;
        Ok(self)
    }

    /// Same as [`Linker::define`], except only the name of the import is
    /// provided, not a module name as well.
    ///
    /// This is only relevant when working with the module linking proposal
    /// where one-level names are allowed (in addition to two-level names).
    /// Otherwise this method need not be used.
    pub fn define_name(&mut self, name: &str, item: impl Into<Extern>) -> Result<&mut Self> {
        let key = self.import_key(name, None);
        self.insert(key, Definition::Extern(item.into()))?;
        Ok(self)
    }

    pub fn func_new(
        &mut self,
        module: &str,
        name: &str,
        ty: FuncType,
        func: impl Fn(Caller<'_, T>, &[Val], &mut [Val]) -> Result<(), Trap> + Send + Sync + 'static,
    ) -> Result<&mut Self> {
        let func = HostFunc::new(&self.engine, ty, func);
        let key = self.import_key(module, Some(name));
        self.insert(
            key,
            Definition::HostFunc {
                func: Arc::new(func),
                t: marker::PhantomData,
            },
        )?;
        Ok(self)
    }

    #[cfg(feature = "async")]
    #[cfg_attr(nightlydoc, doc(cfg(feature = "async")))]
    pub fn func_new_async<F>(
        &mut self,
        module: &str,
        name: &str,
        ty: FuncType,
        func: F,
    ) -> Result<&mut Self>
    where
        F: for<'a> Fn(
                Caller<'a, T>,
                &'a [Val],
                &'a mut [Val],
            ) -> Box<dyn Future<Output = Result<(), Trap>> + Send + 'a>
            + Send
            + Sync
            + 'static,
    {
        assert!(
            self.engine.config().async_support,
            "cannot use `func_new_async` without enabling async support in the config"
        );
        self.func_new(module, name, ty, move |mut caller, params, results| {
            let async_cx = caller.store.as_context_mut().opaque().async_cx();
            let mut future = Pin::from(func(caller, params, results));
            match unsafe { async_cx.block_on(future.as_mut()) } {
                Ok(Ok(())) => Ok(()),
                Ok(Err(trap)) | Err(trap) => Err(trap),
            }
        })
    }

    /// Convenience wrapper to define a function import.
    ///
    /// This method is a convenience wrapper around [`Linker::define`] which
    /// internally delegates to [`Func::wrap`].
    ///
    /// # Errors
    ///
    /// Returns an error if the `module` and `name` already identify an item
    /// of the same type as the `item` provided and if shadowing is disallowed.
    /// For more information see the documentation on [`Linker`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// # let store = Store::default();
    /// let mut linker = Linker::new(&store);
    /// linker.func("host", "double", |x: i32| x * 2)?;
    /// linker.func("host", "log_i32", |x: i32| println!("{}", x))?;
    /// linker.func("host", "log_str", |caller: Caller, ptr: i32, len: i32| {
    ///     // ...
    /// })?;
    ///
    /// let wat = r#"
    ///     (module
    ///         (import "host" "double" (func (param i32) (result i32)))
    ///         (import "host" "log_i32" (func (param i32)))
    ///         (import "host" "log_str" (func (param i32 i32)))
    ///     )
    /// "#;
    /// let module = Module::new(store.engine(), wat)?;
    /// linker.instantiate(&module)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn func_wrap<Params, Args>(
        &mut self,
        module: &str,
        name: &str,
        func: impl IntoFunc<T, Params, Args>,
    ) -> Result<&mut Self> {
        let func = HostFunc::wrap(&self.engine, func);
        let key = self.import_key(module, Some(name));
        self.insert(
            key,
            Definition::HostFunc {
                func: Arc::new(func),
                t: marker::PhantomData,
            },
        )?;
        Ok(self)
    }

    for_each_function_signature!(generate_wrap_async_func);

    /// Convenience wrapper to define an entire [`Instance`] in this linker.
    ///
    /// This function is a convenience wrapper around [`Linker::define`] which
    /// will define all exports on `instance` into this linker. The module name
    /// for each export is `module_name`, and the name for each export is the
    /// name in the instance itself.
    ///
    /// # Errors
    ///
    /// Returns an error if the any item is redefined twice in this linker (for
    /// example the same `module_name` was already defined) and shadowing is
    /// disallowed, or if `instance` comes from a different
    /// [`Store`](crate::Store) than this [`Linker`] originally was created
    /// with.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// # let store = Store::default();
    /// let mut linker = Linker::new(&store);
    ///
    /// // Instantiate a small instance...
    /// let wat = r#"(module (func (export "run") ))"#;
    /// let module = Module::new(store.engine(), wat)?;
    /// let instance = linker.instantiate(&module)?;
    ///
    /// // ... and inform the linker that the name of this instance is
    /// // `instance1`. This defines the `instance1::run` name for our next
    /// // module to use.
    /// linker.instance("instance1", &instance)?;
    ///
    /// let wat = r#"
    ///     (module
    ///         (import "instance1" "run" (func $instance1_run))
    ///         (func (export "run")
    ///             call $instance1_run
    ///         )
    ///     )
    /// "#;
    /// let module = Module::new(store.engine(), wat)?;
    /// let instance = linker.instantiate(&module)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn instance(
        &mut self,
        mut store: impl AsContextMut<Data = T>,
        module_name: &str,
        instance: Instance,
    ) -> Result<&mut Self> {
        for export in instance.exports(store.as_context_mut()) {
            let key = self.import_key(module_name, Some(export.name()));
            self.insert(key, Definition::Extern(export.into_extern()))?;
        }
        Ok(self)
    }

    /// Define automatic instantiations of a [`Module`] in this linker.
    ///
    /// This automatically handles [Commands and Reactors] instantiation and
    /// initialization.
    ///
    /// Exported functions of a Command module may be called directly, however
    /// instead of having a single instance which is reused for each call,
    /// each call creates a new instance, which lives for the duration of the
    /// call. The imports of the Command are resolved once, and reused for
    /// each instantiation, so all dependencies need to be present at the time
    /// when `Linker::module` is called.
    ///
    /// For Reactors, a single instance is created, and an initialization
    /// function is called, and then its exports may be called.
    ///
    /// Ordinary modules which don't declare themselves to be either Commands
    /// or Reactors are treated as Reactors without any initialization calls.
    ///
    /// [Commands and Reactors]: https://github.com/WebAssembly/WASI/blob/master/design/application-abi.md#current-unstable-abi
    ///
    /// # Errors
    ///
    /// Returns an error if the any item is redefined twice in this linker (for
    /// example the same `module_name` was already defined) and shadowing is
    /// disallowed, if `instance` comes from a different
    /// [`Store`](crate::Store) than this [`Linker`] originally was created
    /// with, or if a Reactor initialization function traps.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// # let store = Store::default();
    /// let mut linker = Linker::new(&store);
    ///
    /// // Instantiate a small instance and inform the linker that the name of
    /// // this instance is `instance1`. This defines the `instance1::run` name
    /// // for our next module to use.
    /// let wat = r#"(module (func (export "run") ))"#;
    /// let module = Module::new(store.engine(), wat)?;
    /// linker.module("instance1", &module)?;
    ///
    /// let wat = r#"
    ///     (module
    ///         (import "instance1" "run" (func $instance1_run))
    ///         (func (export "run")
    ///             call $instance1_run
    ///         )
    ///     )
    /// "#;
    /// let module = Module::new(store.engine(), wat)?;
    /// let instance = linker.instantiate(&module)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// For a Command, a new instance is created for each call.
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// # let store = Store::default();
    /// let mut linker = Linker::new(&store);
    ///
    /// // Create a Command that attempts to count the number of times it is run, but is
    /// // foiled by each call getting a new instance.
    /// let wat = r#"
    ///     (module
    ///         (global $counter (mut i32) (i32.const 0))
    ///         (func (export "_start")
    ///             (global.set $counter (i32.add (global.get $counter) (i32.const 1)))
    ///         )
    ///         (func (export "read_counter") (result i32)
    ///             (global.get $counter)
    ///         )
    ///     )
    /// "#;
    /// let module = Module::new(store.engine(), wat)?;
    /// linker.module("commander", &module)?;
    /// let run = linker.get_default("")?.typed::<(), ()>()?.clone();
    /// run.call(())?;
    /// run.call(())?;
    /// run.call(())?;
    ///
    /// let wat = r#"
    ///     (module
    ///         (import "commander" "_start" (func $commander_start))
    ///         (import "commander" "read_counter" (func $commander_read_counter (result i32)))
    ///         (func (export "run") (result i32)
    ///             call $commander_start
    ///             call $commander_start
    ///             call $commander_start
    ///             call $commander_read_counter
    ///         )
    ///     )
    /// "#;
    /// let module = Module::new(store.engine(), wat)?;
    /// linker.module("", &module)?;
    /// let run = linker.get_one_by_name("", Some("run"))?.into_func().unwrap();
    /// let count = run.typed::<(), i32>()?.call(())?;
    /// assert_eq!(count, 0, "a Command should get a fresh instance on each invocation");
    ///
    /// # Ok(())
    /// # }
    /// ```
    pub fn module(
        &mut self,
        mut store: impl AsContextMut<Data = T>,
        module_name: &str,
        module: &Module,
    ) -> Result<&mut Self> {
        match ModuleKind::categorize(module)? {
            ModuleKind::Command => self.command(store, module_name, module),
            ModuleKind::Reactor => {
                let instance = self.instantiate(&mut store, &module)?;

                if let Some(export) = instance.get_export(&mut store, "_initialize") {
                    if let Extern::Func(func) = export {
                        func.typed::<(), (), _>(&store)
                            .and_then(|f| f.call(&mut store, ()).map_err(Into::into))
                            .context("calling the Reactor initialization function")?;
                    }
                }

                self.instance(store, module_name, instance)
            }
        }
    }

    fn command(
        &mut self,
        mut store: impl AsContextMut<Data = T>,
        module_name: &str,
        module: &Module,
    ) -> Result<&mut Self> {
        for export in module.exports() {
            if let Some(func_ty) = export.ty().func() {
                let imports = self.compute_imports(&mut store, module)?;
                let module = module.clone();
                let export_name = export.name().to_owned();
                let func = Func::new(
                    &mut store,
                    func_ty.clone(),
                    move |mut caller, params, results| {
                        // Create a new instance for this command execution.
                        let instance = Instance::new(&mut caller, &module, &imports)?;

                        // `unwrap()` everything here because we know the instance contains a
                        // function export with the given name and signature because we're
                        // iterating over the module it was instantiated from.
                        let command_results = instance
                            .get_export(&mut caller, &export_name)
                            .unwrap()
                            .into_func()
                            .unwrap()
                            .call(&mut caller, params)
                            .map_err(|error| error.downcast::<Trap>().unwrap())?;

                        // Copy the return values into the output slice.
                        for (result, command_result) in
                            results.iter_mut().zip(command_results.into_vec())
                        {
                            *result = command_result;
                        }

                        Ok(())
                    },
                );
                let key = self.import_key(module_name, Some(export.name()));
                self.insert(key, Definition::Extern(func.into()))?;
            } else if export.name() == "memory" && export.ty().memory().is_some() {
                // Allow an exported "memory" memory for now.
            } else if export.name() == "__indirect_function_table" && export.ty().table().is_some()
            {
                // Allow an exported "__indirect_function_table" table for now.
            } else if export.name() == "table" && export.ty().table().is_some() {
                // Allow an exported "table" table for now.
            } else if export.name() == "__data_end" && export.ty().global().is_some() {
                // Allow an exported "__data_end" memory for compatibility with toolchains
                // which use --export-dynamic, which unfortunately doesn't work the way
                // we want it to.
                warn!("command module exporting '__data_end' is deprecated");
            } else if export.name() == "__heap_base" && export.ty().global().is_some() {
                // Allow an exported "__data_end" memory for compatibility with toolchains
                // which use --export-dynamic, which unfortunately doesn't work the way
                // we want it to.
                warn!("command module exporting '__heap_base' is deprecated");
            } else if export.name() == "__dso_handle" && export.ty().global().is_some() {
                // Allow an exported "__dso_handle" memory for compatibility with toolchains
                // which use --export-dynamic, which unfortunately doesn't work the way
                // we want it to.
                warn!("command module exporting '__dso_handle' is deprecated")
            } else if export.name() == "__rtti_base" && export.ty().global().is_some() {
                // Allow an exported "__rtti_base" memory for compatibility with
                // AssemblyScript.
                warn!("command module exporting '__rtti_base' is deprecated; pass `--runtime half` to the AssemblyScript compiler");
            } else if !self.allow_unknown_exports {
                bail!("command export '{}' is not a function", export.name());
            }
        }

        Ok(self)
    }

    /// Aliases one module's name as another.
    ///
    /// This method will alias all currently defined under `module` to also be
    /// defined under the name `as_module` too.
    ///
    /// # Errors
    ///
    /// Returns an error if any shadowing violations happen while defining new
    /// items.
    pub fn alias(&mut self, module: &str, as_module: &str) -> Result<()> {
        let module = self.intern_str(module);
        let as_module = self.intern_str(as_module);
        let items = self
            .map
            .iter()
            .filter(|(key, _def)| key.module == module)
            .map(|(key, def)| (key.name, def.clone()))
            .collect::<Vec<_>>();
        for (name, item) in items {
            self.insert(
                ImportKey {
                    module: as_module,
                    name,
                },
                item,
            )?;
        }
        Ok(())
    }

    fn insert(&mut self, key: ImportKey, item: Definition<T>) -> Result<()> {
        match self.map.entry(key) {
            Entry::Occupied(_) if !self.allow_shadowing => {
                let module = &self.strings[key.module];
                let desc = match self.strings.get(key.name) {
                    Some(name) => format!("{}::{}", module, name),
                    None => module.to_string(),
                };
                bail!("import of `{}` defined twice", desc)
            }
            Entry::Occupied(mut o) => {
                o.insert(item);
            }
            Entry::Vacant(v) => {
                v.insert(item);
            }
        }
        Ok(())
    }

    fn import_key(&mut self, module: &str, name: Option<&str>) -> ImportKey {
        ImportKey {
            module: self.intern_str(module),
            name: name
                .map(|name| self.intern_str(name))
                .unwrap_or(usize::max_value()),
        }
    }

    fn intern_str(&mut self, string: &str) -> usize {
        if let Some(idx) = self.string2idx.get(string) {
            return *idx;
        }
        let string: Arc<str> = string.into();
        let idx = self.strings.len();
        self.strings.push(string.clone());
        self.string2idx.insert(string, idx);
        idx
    }

    /// Attempts to instantiate the `module` provided.
    ///
    /// This method will attempt to assemble a list of imports that correspond
    /// to the imports required by the [`Module`] provided. This list
    /// of imports is then passed to [`Instance::new`] to continue the
    /// instantiation process.
    ///
    /// Each import of `module` will be looked up in this [`Linker`] and must
    /// have previously been defined. If it was previously defined with an
    /// incorrect signature or if it was not previously defined then an error
    /// will be returned because the import can not be satisfied.
    ///
    /// Per the WebAssembly spec, instantiation includes running the module's
    /// start function, if it has one (not to be confused with the `_start`
    /// function, which is not run).
    ///
    /// # Errors
    ///
    /// This method can fail because an import may not be found, or because
    /// instantiation itself may fail. For information on instantiation
    /// failures see [`Instance::new`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use wasmtime::*;
    /// # fn main() -> anyhow::Result<()> {
    /// # let store = Store::default();
    /// let mut linker = Linker::new(&store);
    /// linker.func("host", "double", |x: i32| x * 2)?;
    ///
    /// let wat = r#"
    ///     (module
    ///         (import "host" "double" (func (param i32) (result i32)))
    ///     )
    /// "#;
    /// let module = Module::new(store.engine(), wat)?;
    /// linker.instantiate(&module)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn instantiate(
        &self,
        mut store: impl AsContextMut<Data = T>,
        module: &Module,
    ) -> Result<Instance> {
        let imports = self.compute_imports(&mut store, module)?;
        Instance::new(store, module, &imports)
    }

    /// Attempts to instantiate the `module` provided. This is the same as [`Linker::instantiate`],
    /// except for async `Store`s.
    #[cfg(feature = "async")]
    #[cfg_attr(nightlydoc, doc(cfg(feature = "async")))]
    pub async fn instantiate_async(
        &self,
        mut store: impl AsContextMut<Data = T>,
        module: &Module,
    ) -> Result<Instance>
    where
        T: Send,
    {
        let imports = self.compute_imports(&mut store, module)?;
        Instance::new_async(store, module, &imports).await
    }

    fn compute_imports(
        &self,
        mut store: impl AsContextMut<Data = T>,
        module: &Module,
    ) -> Result<Vec<Extern>> {
        module
            .imports()
            .map(|import| {
                self.get(&mut store, &import)
                    .ok_or_else(|| self.link_error(&import))
            })
            .collect()
    }

    fn link_error(&self, import: &ImportType) -> Error {
        let desc = match import.name() {
            Some(name) => format!("{}::{}", import.module(), name),
            None => import.module().to_string(),
        };
        anyhow!("unknown import: `{}` has not been defined", desc)
    }

    /// Returns an iterator over all items defined in this `Linker`, in arbitrary order.
    ///
    /// The iterator returned will yield 3-tuples where the first two elements
    /// are the module name and item name for the external item, and the third
    /// item is the item itself that is defined.
    ///
    /// Note that multiple `Extern` items may be defined for the same
    /// module/name pair.
    pub fn iter<'a: 'p, 'p>(
        &'a self,
        mut store: impl AsContextMut<Data = T> + 'p,
    ) -> impl Iterator<Item = (&str, &str, Extern)> + 'p {
        self.map.iter().map(move |(key, item)| {
            (
                &*self.strings[key.module],
                &*self.strings[key.name],
                item.to_extern(&mut store),
            )
        })
    }

    /// Looks up a value in this `Linker` which matches the `import` type
    /// provided.
    ///
    /// Returns `None` if no match was found.
    pub fn get(
        &self,
        mut store: impl AsContextMut<Data = T>,
        import: &ImportType,
    ) -> Option<Extern> {
        if let Some(ext) = self.get_extern(&mut store, import) {
            return Some(ext);
        }

        match import.ty() {
            ExternType::Instance(t) => {
                // This is a key location where the module linking proposal is
                // implemented. This logic allows single-level imports of an instance to
                // get satisfied by multiple definitions of items within this `Linker`.
                //
                // The instance being import is iterated over to load the names from
                // this `Linker` (recursively calling `get`). If anything isn't defined
                // we return `None` since the entire value isn't defined. Otherwise when
                // all values are loaded it's assembled into an `Instance` and
                // returned`.
                //
                // Note that this isn't exactly the speediest implementation in the
                // world. Ideally we would pre-create the `Instance` instead of creating
                // it each time a module is instantiated. For now though while the
                // module linking proposal is under development this should hopefully
                // suffice.
                if import.name().is_none() {
                    let mut builder = InstanceBuilder::new();
                    for export in t.exports() {
                        let item =
                            self.get_extern(&mut store, &export.as_import(import.module()))?;
                        builder.insert(export.name(), item);
                    }
                    Some(builder.finish(&mut store.as_context_mut().opaque()).into())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Returns all items defined for the `module` and `name` pair.
    ///
    /// This may return an empty iterator, but it may also return multiple items
    /// if the module/name have been defined twice.
    pub fn get_by_name<'a: 'p, 'p>(
        &'a self,
        mut store: impl AsContextMut<Data = T> + 'p,
        module: &'p str,
        name: Option<&'p str>,
    ) -> impl Iterator<Item = Extern> + 'p {
        self.map
            .iter()
            .filter(move |(key, _item)| {
                &*self.strings[key.module] == module
                    && self.strings.get(key.name).map(|s| &**s) == name
            })
            .map(move |(_, item)| item.to_extern(&mut store))
    }

    /// Returns the single item defined for the `module` and `name` pair.
    ///
    /// Unlike the similar [`Linker::get_by_name`] method this function returns
    /// a single `Extern` item. If the `module` and `name` pair isn't defined
    /// in this linker then an error is returned. If more than one value exists
    /// for the `module` and `name` pairs, then an error is returned as well.
    pub fn get_one_by_name(
        &self,
        store: impl AsContextMut<Data = T>,
        module: &str,
        name: Option<&str>,
    ) -> Result<Extern> {
        let err_msg = || match name {
            Some(name) => format!("named `{}` in `{}`", name, module),
            None => format!("named `{}`", module),
        };
        let mut items = self.get_by_name(store, module, name);
        let ret = items
            .next()
            .ok_or_else(|| anyhow!("no item {}", err_msg()))?;
        if items.next().is_some() {
            bail!("too many items {}", err_msg());
        }
        Ok(ret.clone())
    }

    /// Returns the "default export" of a module.
    ///
    /// An export with an empty string is considered to be a "default export".
    /// "_start" is also recognized for compatibility.
    pub fn get_default(
        &self,
        mut store: impl AsContextMut<Data = T>,
        module: &str,
    ) -> Result<Func> {
        let mut items = self.get_by_name(&mut store, module, Some(""));
        if let Some(external) = items.next() {
            if items.next().is_some() {
                bail!("too many items named `` in `{}`", module);
            }
            if let Extern::Func(func) = external {
                return Ok(func.clone());
            }
            bail!("default export in '{}' is not a function", module);
        }
        drop(items);

        // For compatibility, also recognize "_start".
        let mut items = self.get_by_name(&mut store, module, Some("_start"));
        if let Some(external) = items.next() {
            if items.next().is_some() {
                bail!("too many items named `_start` in `{}`", module);
            }
            if let Extern::Func(func) = external {
                return Ok(func.clone());
            }
            bail!("`_start` in '{}' is not a function", module);
        }
        drop(items);

        // Otherwise return a no-op function.
        Ok(Func::wrap(store, || {}))
    }

    fn get_extern(
        &self,
        store: impl AsContextMut<Data = T>,
        import: &ImportType,
    ) -> Option<Extern> {
        let key = ImportKey {
            module: *self.string2idx.get(import.module())?,
            name: match import.name() {
                Some(name) => *self.string2idx.get(name)?,
                None => usize::max_value(),
            },
        };
        Some(self.map.get(&key)?.to_extern(store))
    }
}

impl<T> Default for Linker<T> {
    fn default() -> Linker<T> {
        Linker::new(&Engine::default())
    }
}

impl<T> Definition<T> {
    fn to_extern(&self, mut store: impl AsContextMut<Data = T>) -> Extern {
        match self {
            Definition::Extern(e) => e.clone(),

            // Note the unsafety here is due to calling `to_func`. The
            // requirement here is that the `T` that was originally used to
            // create the `HostFunc` matches the `T` on the store. This is done
            // with the `T` on `Definition` (and `Linker`) as well as the
            // `Data=T` bound above.
            Definition::HostFunc { func, .. } => unsafe {
                func.to_func(&mut store.as_context_mut().opaque()).into()
            },
        }
    }
}

impl<T> Clone for Definition<T> {
    fn clone(&self) -> Definition<T> {
        match self {
            Definition::Extern(e) => Definition::Extern(e.clone()),
            Definition::HostFunc { func, t } => Definition::HostFunc {
                func: func.clone(),
                t: *t,
            },
        }
    }
}

/// Modules can be interpreted either as Commands or Reactors.
enum ModuleKind {
    /// The instance is a Command, meaning an instance is created for each
    /// exported function and lives for the duration of the function call.
    Command,

    /// The instance is a Reactor, meaning one instance is created which
    /// may live across multiple calls.
    Reactor,
}

impl ModuleKind {
    /// Determine whether the given module is a Command or a Reactor.
    fn categorize(module: &Module) -> Result<ModuleKind> {
        let command_start = module.get_export("_start");
        let reactor_start = module.get_export("_initialize");
        match (command_start, reactor_start) {
            (Some(command_start), None) => {
                if let Some(_) = command_start.func() {
                    Ok(ModuleKind::Command)
                } else {
                    bail!("`_start` must be a function")
                }
            }
            (None, Some(reactor_start)) => {
                if let Some(_) = reactor_start.func() {
                    Ok(ModuleKind::Reactor)
                } else {
                    bail!("`_initialize` must be a function")
                }
            }
            (None, None) => {
                // Module declares neither of the recognized functions, so treat
                // it as a reactor with no initialization function.
                Ok(ModuleKind::Reactor)
            }
            (Some(_), Some(_)) => {
                // Module declares itself to be both a Command and a Reactor.
                bail!("Program cannot be both a Command and a Reactor")
            }
        }
    }
}
