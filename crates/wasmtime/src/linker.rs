use crate::{
    Extern, ExternType, Func, FuncType, GlobalType, ImportType, Instance, IntoFunc, Module, Store,
};
use anyhow::{anyhow, bail, Result};
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
///
/// Note that allowing duplicates by shadowing the previous definition can be
/// controlled with the [`Linker::allow_shadowing`] method as well.
pub struct Linker {
    store: Store,
    string2idx: HashMap<Rc<str>, usize>,
    strings: Vec<Rc<str>>,
    map: HashMap<ImportKey, Extern>,
    allow_shadowing: bool,
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
            allow_shadowing: false,
        }
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
    pub fn allow_shadowing(&mut self, allow: bool) -> &mut Linker {
        self.allow_shadowing = allow;
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
        self.insert(module, name, item)?;
        Ok(self)
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
    /// example the same `module_name` was already defined) and shadowing is
    /// disallowed, or if `instance` comes from a different [`Store`] than this
    /// [`Linker`] originally was created with.
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
        for export in instance.exports() {
            self.insert(module_name, export.name(), export.into_extern())?;
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
        let items = self
            .iter()
            .filter(|(m, _, _)| *m == module)
            .map(|(_, name, item)| (name.to_string(), item))
            .collect::<Vec<_>>();
        for (name, item) in items {
            self.define(as_module, &name, item)?;
        }
        Ok(())
    }

    fn insert(&mut self, module: &str, name: &str, item: Extern) -> Result<()> {
        let key = self.import_key(module, name, item.ty());
        match self.map.entry(key) {
            Entry::Occupied(o) if !self.allow_shadowing => bail!(
                "import of `{}::{}` with kind {:?} defined twice",
                module,
                name,
                o.key().kind,
            ),
            Entry::Occupied(mut o) => {
                o.insert(item);
            }
            Entry::Vacant(v) => {
                v.insert(item);
            }
        }
        Ok(())
    }

    fn import_key(&mut self, module: &str, name: &str, ty: ExternType) -> ImportKey {
        ImportKey {
            module: self.intern_str(module),
            name: self.intern_str(name),
            kind: self.import_kind(ty),
        }
    }

    fn import_kind(&self, ty: ExternType) -> ImportKind {
        match ty {
            ExternType::Func(f) => ImportKind::Func(f),
            ExternType::Global(f) => ImportKind::Global(f),
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
            if let Some(item) = self.get(&import) {
                imports.push(item);
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
                    "unknown import: `{}::{}` has not been defined",
                    import.module(),
                    import.name()
                )
            }

            bail!(
                "incompatible import type for `{}::{}` specified\n\
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

    /// Returns the [`Store`] that this linker is connected to.
    pub fn store(&self) -> &Store {
        &self.store
    }

    /// Returns an iterator over all items defined in this `Linker`.
    ///
    /// The iterator returned will yield 3-tuples where the first two elements
    /// are the module name and item name for the external item, and the third
    /// item is the item itself that is defined.
    ///
    /// Note that multiple `Extern` items may be defined for the same
    /// module/name pair.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str, Extern)> {
        self.map.iter().map(move |(key, item)| {
            (
                &*self.strings[key.module],
                &*self.strings[key.name],
                item.clone(),
            )
        })
    }

    /// Looks up a value in this `Linker` which matches the `import` type
    /// provided.
    ///
    /// Returns `None` if no match was found.
    pub fn get(&self, import: &ImportType) -> Option<Extern> {
        let key = ImportKey {
            module: *self.string2idx.get(import.module())?,
            name: *self.string2idx.get(import.name())?,
            kind: self.import_kind(import.ty()),
        };
        self.map.get(&key).cloned()
    }

    /// Returns all items defined for the `module` and `name` pair.
    ///
    /// This may return an empty iterator, but it may also return multiple items
    /// if the module/name have been defined twice.
    pub fn get_by_name<'a: 'p, 'p>(
        &'a self,
        module: &'p str,
        name: &'p str,
    ) -> impl Iterator<Item = &'a Extern> + 'p {
        self.map
            .iter()
            .filter(move |(key, _item)| {
                &*self.strings[key.module] == module && &*self.strings[key.name] == name
            })
            .map(|(_, item)| item)
    }

    /// Returns the single item defined for the `module` and `name` pair.
    ///
    /// Unlike the similar [`Linker::get_by_name`] method this function returns
    /// a single `Extern` item. If the `module` and `name` pair isn't defined
    /// in this linker then an error is returned. If more than one value exists
    /// for the `module` and `name` pairs, then an error is returned as well.
    pub fn get_one_by_name(&self, module: &str, name: &str) -> Result<Extern> {
        let mut items = self.get_by_name(module, name);
        let ret = items
            .next()
            .ok_or_else(|| anyhow!("no item named `{}` in `{}`", name, module))?;
        if items.next().is_some() {
            bail!("too many items named `{}` in `{}`", name, module);
        }
        Ok(ret.clone())
    }
}
