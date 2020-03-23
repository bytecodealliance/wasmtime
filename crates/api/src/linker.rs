use crate::{
    Extern, ExternType, Func, FuncType, GlobalType, ImportType, Instance, IntoFunc, Module, Store,
};
use anyhow::{bail, Result};
use std::collections::hash_map::{Entry, HashMap};
use std::rc::Rc;

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
/// Names in a `Linker` can be defined twice, but only for different signatures
/// of items. This means that every item defined in a `Linker` has a unique
/// name/type pair. For example you can define two functions with the module
/// name `foo` and item name `bar`, so long as they have different function
/// signatures. Currently duplicate memories and tables are not allowed, only
/// one-per-name is allowed.
pub struct Linker {
    store: Store,
    string2idx: HashMap<Rc<str>, usize>,
    strings: Vec<Rc<str>>,
    map: HashMap<ImportKey, Extern>,
}

#[derive(Hash, PartialEq, Eq)]
struct ImportKey {
    name: usize,
    module: usize,
    kind: ImportKind,
}

#[derive(Hash, PartialEq, Eq, Debug)]
enum ImportKind {
    Func(FuncType),
    Global(GlobalType),
    Memory,
    Table,
}

impl Linker {
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
    pub fn new(store: &Store) -> Linker {
        Linker {
            store: store.clone(),
            map: HashMap::new(),
            string2idx: HashMap::new(),
            strings: Vec::new(),
        }
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
    /// of the same type as the `item` provided. For more information see the
    /// documentation on [`Linker`].
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
    /// linker.define("host", "offset", global);
    ///
    /// let wat = r#"
    ///     (module
    ///         (import "host" "offset" (global i32))
    ///         (memory 1)
    ///         (data (global.get 0) "foo")
    ///     )
    /// "#;
    /// let module = Module::new(&store, wat)?;
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
        self._define(module, name, item.into())
    }

    fn _define(&mut self, module: &str, name: &str, item: Extern) -> Result<&mut Self> {
        if !item.comes_from_same_store(&self.store) {
            bail!("all linker items must be from the same store");
        }
        self.insert(module, name, &item.ty(), item)?;
        Ok(self)
    }

    /// Convenience wrapper to define a function import.
    ///
    /// This method is a convenience wrapper around [`Linker::define`] which
    /// internally delegates to [`Func::wrap`].
    ///
    /// # Errors
    ///
    /// Returns an error if the `module` and `name` already identify a function
    /// of the same signature as `func`. For more information see the
    /// documentation on [`Linker`].
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
    /// let module = Module::new(&store, wat)?;
    /// linker.instantiate(&module)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn func<Params, Args>(
        &mut self,
        module: &str,
        name: &str,
        func: impl IntoFunc<Params, Args>,
    ) -> Result<&mut Self> {
        self._define(module, name, Func::wrap(&self.store, func).into())
    }

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
    /// example the same `module_name` was already defined), or if `instance`
    /// comes from a different [`Store`] than this [`Linker`] originally was
    /// created with.
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
    /// let module = Module::new(&store, wat)?;
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
    /// let module = Module::new(&store, wat)?;
    /// let instance = linker.instantiate(&module)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn instance(&mut self, module_name: &str, instance: &Instance) -> Result<&mut Self> {
        if !Store::same(&self.store, instance.store()) {
            bail!("all linker items must be from the same store");
        }
        for (export, item) in instance.module().exports().iter().zip(instance.exports()) {
            self.insert(module_name, export.name(), export.ty(), item.clone())?;
        }
        Ok(self)
    }

    fn insert(&mut self, module: &str, name: &str, ty: &ExternType, item: Extern) -> Result<()> {
        let key = self.import_key(module, name, ty);
        match self.map.entry(key) {
            Entry::Occupied(o) => bail!(
                "import of `{}::{}` with as {:?} defined twice",
                module,
                name,
                o.key().kind,
            ),
            Entry::Vacant(v) => {
                v.insert(item);
            }
        }
        Ok(())
    }

    fn import_key(&mut self, module: &str, name: &str, ty: &ExternType) -> ImportKey {
        ImportKey {
            module: self.intern_str(module),
            name: self.intern_str(name),
            kind: self.import_kind(ty),
        }
    }

    fn import_kind(&self, ty: &ExternType) -> ImportKind {
        match ty {
            ExternType::Func(f) => ImportKind::Func(f.clone()),
            ExternType::Global(f) => ImportKind::Global(f.clone()),
            ExternType::Memory(_) => ImportKind::Memory,
            ExternType::Table(_) => ImportKind::Table,
        }
    }

    fn intern_str(&mut self, string: &str) -> usize {
        if let Some(idx) = self.string2idx.get(string) {
            return *idx;
        }
        let string: Rc<str> = string.into();
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
    /// incorrect signature or if it was not prevoiusly defined then an error
    /// will be returned because the import can not be satisfied.
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
    /// let module = Module::new(&store, wat)?;
    /// linker.instantiate(&module)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn instantiate(&self, module: &Module) -> Result<Instance> {
        let mut imports = Vec::new();
        for import in module.imports() {
            if let Some(item) = self.import_get(import) {
                imports.push(item.clone());
                continue;
            }

            let mut options = String::new();
            for i in self.map.keys() {
                if &*self.strings[i.module] != import.module()
                    || &*self.strings[i.name] != import.name()
                {
                    continue;
                }
                options.push_str("  * ");
                options.push_str(&format!("{:?}", i.kind));
                options.push_str("\n");
            }
            if options.len() == 0 {
                bail!(
                    "import of `{}::{}` has not been defined",
                    import.module(),
                    import.name()
                )
            }

            bail!(
                "failed to find import of `{}::{}` with matching signature\n\
                 desired signature was: {:?}\n\
                 signatures available:\n\n{}",
                import.module(),
                import.name(),
                import.ty(),
                options,
            )
        }

        Instance::new(module, &imports)
    }

    fn import_get(&self, import: &ImportType) -> Option<&Extern> {
        let key = ImportKey {
            module: *self.string2idx.get(import.module())?,
            name: *self.string2idx.get(import.name())?,
            kind: self.import_kind(import.ty()),
        };
        self.map.get(&key)
    }
}
