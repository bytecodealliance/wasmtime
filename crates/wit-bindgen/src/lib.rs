use crate::rust::{to_rust_ident, to_rust_upper_camel_case, RustGenerator, TypeMode};
use crate::types::{TypeInfo, Types};
use anyhow::bail;
use heck::*;
use indexmap::{IndexMap, IndexSet};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt::Write as _;
use std::io::{Read, Write};
use std::mem;
use std::process::{Command, Stdio};
use wit_parser::*;

macro_rules! uwrite {
    ($dst:expr, $($arg:tt)*) => {
        write!($dst, $($arg)*).unwrap()
    };
}

macro_rules! uwriteln {
    ($dst:expr, $($arg:tt)*) => {
        writeln!($dst, $($arg)*).unwrap()
    };
}

mod rust;
mod source;
mod types;
use source::Source;

#[derive(Clone)]
enum InterfaceName {
    /// This interface was remapped using `with` to some other Rust code.
    Remapped {
        /// This is the `::`-separated string which is the path to the mapped
        /// item relative to the root of the `bindgen!` macro invocation.
        ///
        /// This path currently starts with `__with_name$N` and will then
        /// optionally have `::` projections through to the actual item
        /// depending on how `with` was configured.
        name_at_root: String,

        /// This is currently only used for exports and is the relative path to
        /// where this mapped name would be located if `with` were not
        /// specified. Basically it's the same as the `Path` variant of this
        /// enum if the mapping weren't present.
        local_path: Vec<String>,
    },

    /// This interface is generated in the module hierarchy specified.
    ///
    /// The path listed here is the path, from the root of the `bindgen!` macro,
    /// to where this interface is generated.
    Path(Vec<String>),
}

#[derive(Default)]
struct Wasmtime {
    src: Source,
    opts: Opts,
    /// A list of all interfaces which were imported by this world.
    ///
    /// The first two values identify the interface; the third is the contents of the
    /// module that this interface generated. The fourth value is the name of the
    /// interface as also present in `self.interface_names`.
    import_interfaces: Vec<(WorldKey, InterfaceId, String, InterfaceName)>,
    import_functions: Vec<ImportFunction>,
    exports: Exports,
    types: Types,
    sizes: SizeAlign,
    interface_names: HashMap<InterfaceId, InterfaceName>,
    interface_last_seen_as_import: HashMap<InterfaceId, bool>,
    trappable_errors: IndexMap<TypeId, String>,
    // Track the with options that were used. Remapped interfaces provided via `with`
    // are required to be used.
    used_with_opts: HashSet<String>,
    // Track the imports that matched the `trappable_imports` spec.
    used_trappable_imports_opts: HashSet<String>,
    world_link_options: LinkOptionsBuilder,
    interface_link_options: HashMap<InterfaceId, LinkOptionsBuilder>,
}

struct ImportFunction {
    func: Function,
    add_to_linker: String,
    sig: Option<String>,
}

#[derive(Default)]
struct Exports {
    fields: BTreeMap<String, ExportField>,
    modules: Vec<(InterfaceId, String, InterfaceName)>,
    funcs: Vec<String>,
}

struct ExportField {
    ty: String,
    ty_index: String,
    load: String,
    get_index_from_component: String,
    get_index_from_instance: String,
}

#[derive(Default, Debug, Clone, Copy)]
pub enum Ownership {
    /// Generated types will be composed entirely of owning fields, regardless
    /// of whether they are used as parameters to guest exports or not.
    #[default]
    Owning,

    /// Generated types used as parameters to guest exports will be "deeply
    /// borrowing", i.e. contain references rather than owned values when
    /// applicable.
    Borrowing {
        /// Whether or not to generate "duplicate" type definitions for a single
        /// WIT type if necessary, for example if it's used as both an import
        /// and an export, or if it's used both as a parameter to an export and
        /// a return value from an export.
        duplicate_if_necessary: bool,
    },
}

#[derive(Default, Debug, Clone)]
pub struct Opts {
    /// Whether or not `rustfmt` is executed to format generated code.
    pub rustfmt: bool,

    /// Whether or not to emit `tracing` macro calls on function entry/exit.
    pub tracing: bool,

    /// Whether or not `tracing` macro calls should included argument and
    /// return values which contain dynamically-sized `list` values.
    pub verbose_tracing: bool,

    /// Whether or not to use async rust functions and traits.
    pub async_: AsyncConfig,

    /// Whether or not to use `func_wrap_concurrent` when generating code for
    /// async imports.
    ///
    /// Unlike `func_wrap_async`, `func_wrap_concurrent` allows host functions
    /// to suspend without monopolizing the `Store`, meaning other guest tasks
    /// can make progress concurrently.
    pub concurrent_imports: bool,

    /// Whether or not to use `call_concurrent` when generating code for
    /// async exports.
    ///
    /// Unlike `call_async`, `call_concurrent` allows the caller to make
    /// multiple concurrent calls on the same component instance.
    pub concurrent_exports: bool,

    /// A list of "trappable errors" which are used to replace the `E` in
    /// `result<T, E>` found in WIT.
    pub trappable_error_type: Vec<TrappableError>,

    /// Whether to generate owning or borrowing type definitions.
    pub ownership: Ownership,

    /// Whether or not to generate code for only the interfaces of this wit file or not.
    pub only_interfaces: bool,

    /// Configuration of which imports are allowed to generate a trap.
    pub trappable_imports: TrappableImports,

    /// Remapping of interface names to rust module names.
    /// TODO: is there a better type to use for the value of this map?
    pub with: HashMap<String, String>,

    /// Additional derive attributes to add to generated types. If using in a CLI, this flag can be
    /// specified multiple times to add multiple attributes.
    ///
    /// These derive attributes will be added to any generated structs or enums
    pub additional_derive_attributes: Vec<String>,

    /// Evaluate to a string literal containing the generated code rather than the generated tokens
    /// themselves. Mostly useful for Wasmtime internal debugging and development.
    pub stringify: bool,

    /// Temporary option to skip `impl<T: Trait> Trait for &mut T` for the
    /// `wasmtime-wasi` crate while that's given a chance to update its b
    /// indings.
    pub skip_mut_forwarding_impls: bool,

    /// Indicates that the `T` in `Store<T>` should be send even if async is not
    /// enabled.
    ///
    /// This is helpful when sync bindings depend on generated functions from
    /// async bindings as is the case with WASI in-tree.
    pub require_store_data_send: bool,

    /// Path to the `wasmtime` crate if it's not the default path.
    pub wasmtime_crate: Option<String>,

    /// If true, write the generated bindings to a file for better error
    /// messages from `rustc`.
    ///
    /// This can also be toggled via the `WASMTIME_DEBUG_BINDGEN` environment
    /// variable, but that will affect _all_ `bindgen!` macro invocations (and
    /// can sometimes lead to one invocation ovewriting another in unpredictable
    /// ways), whereas this option lets you specify it on a case-by-case basis.
    pub debug: bool,
}

#[derive(Debug, Clone)]
pub struct TrappableError {
    /// Full path to the error, such as `wasi:io/streams/error`.
    pub wit_path: String,

    /// The name, in Rust, of the error type to generate.
    pub rust_type_name: String,
}

/// Which imports should be generated as async functions.
///
/// The imports should be declared in the following format:
/// - Regular functions: `"{function-name}"`
/// - Resource methods: `"[method]{resource-name}.{method-name}"`
/// - Resource destructors: `"[drop]{resource-name}"`
///
/// Examples:
/// - Regular function: `"get-environment"`
/// - Resource method: `"[method]input-stream.read"`
/// - Resource destructor: `"[drop]input-stream"`
#[derive(Default, Debug, Clone)]
pub enum AsyncConfig {
    /// No functions are `async`.
    #[default]
    None,
    /// All generated functions should be `async`.
    All,
    /// These imported functions should not be async, but everything else is.
    AllExceptImports(HashSet<String>),
    /// These functions are the only imports that are async, all other imports
    /// are sync.
    ///
    /// Note that all exports are still async in this situation.
    OnlyImports(HashSet<String>),
}

pub enum CallStyle {
    Sync,
    Async,
    Concurrent,
}

#[derive(Default, Debug, Clone)]
pub enum TrappableImports {
    /// No imports are allowed to trap.
    #[default]
    None,
    /// All imports may trap.
    All,
    /// Only the specified set of functions may trap.
    Only(HashSet<String>),
}

impl TrappableImports {
    fn can_trap(&self, f: &Function) -> bool {
        match self {
            TrappableImports::None => false,
            TrappableImports::All => true,
            TrappableImports::Only(set) => set.contains(&f.name),
        }
    }
}

impl Opts {
    pub fn generate(&self, resolve: &Resolve, world: WorldId) -> anyhow::Result<String> {
        // TODO: Should we refine this test to inspect only types reachable from
        // the specified world?
        if !cfg!(feature = "component-model-async")
            && resolve.types.iter().any(|(_, ty)| {
                matches!(
                    ty.kind,
                    TypeDefKind::Future(_) | TypeDefKind::Stream(_) | TypeDefKind::ErrorContext
                )
            })
        {
            anyhow::bail!(
                "must enable `component-model-async` feature when using WIT files \
                 containing future, stream, or error types"
            );
        }

        let mut r = Wasmtime::default();
        r.sizes.fill(resolve);
        r.opts = self.clone();
        r.populate_world_and_interface_options(resolve, world);
        r.generate(resolve, world)
    }

    fn is_store_data_send(&self) -> bool {
        matches!(self.call_style(), CallStyle::Async | CallStyle::Concurrent)
            || self.require_store_data_send
    }

    pub fn import_call_style(&self, qualifier: Option<&str>, f: &str) -> CallStyle {
        let matched = |names: &HashSet<String>| {
            names.contains(f)
                || qualifier
                    .map(|v| names.contains(&format!("{v}#{f}")))
                    .unwrap_or(false)
        };

        match &self.async_ {
            AsyncConfig::AllExceptImports(names) if matched(names) => CallStyle::Sync,
            AsyncConfig::OnlyImports(names) if !matched(names) => CallStyle::Sync,
            _ => self.call_style(),
        }
    }

    pub fn drop_call_style(&self, qualifier: Option<&str>, r: &str) -> CallStyle {
        self.import_call_style(qualifier, &format!("[drop]{r}"))
    }

    pub fn call_style(&self) -> CallStyle {
        match &self.async_ {
            AsyncConfig::None => CallStyle::Sync,

            AsyncConfig::All | AsyncConfig::AllExceptImports(_) | AsyncConfig::OnlyImports(_) => {
                if self.concurrent_imports {
                    CallStyle::Concurrent
                } else {
                    CallStyle::Async
                }
            }
        }
    }
}

impl Wasmtime {
    fn populate_world_and_interface_options(&mut self, resolve: &Resolve, world: WorldId) {
        self.world_link_options.add_world(resolve, &world);

        for (_, import) in resolve.worlds[world].imports.iter() {
            match import {
                WorldItem::Interface { id, .. } => {
                    let mut o = LinkOptionsBuilder::default();
                    o.add_interface(resolve, id);
                    self.interface_link_options.insert(*id, o);
                }
                WorldItem::Function(_) | WorldItem::Type(_) => {}
            }
        }
    }
    fn name_interface(
        &mut self,
        resolve: &Resolve,
        id: InterfaceId,
        name: &WorldKey,
        is_export: bool,
    ) -> bool {
        let mut path = Vec::new();
        if is_export {
            path.push("exports".to_string());
        }
        match name {
            WorldKey::Name(name) => {
                path.push(name.to_snake_case());
            }
            WorldKey::Interface(_) => {
                let iface = &resolve.interfaces[id];
                let pkgname = &resolve.packages[iface.package.unwrap()].name;
                path.push(pkgname.namespace.to_snake_case());
                path.push(self.name_package_module(resolve, iface.package.unwrap()));
                path.push(to_rust_ident(iface.name.as_ref().unwrap()));
            }
        }
        let entry = if let Some(name_at_root) = self.lookup_replacement(resolve, name, None) {
            InterfaceName::Remapped {
                name_at_root,
                local_path: path,
            }
        } else {
            InterfaceName::Path(path)
        };

        let remapped = matches!(entry, InterfaceName::Remapped { .. });
        self.interface_names.insert(id, entry);
        remapped
    }

    /// If the package `id` is the only package with its namespace/name combo
    /// then pass through the name unmodified. If, however, there are multiple
    /// versions of this package then the package module is going to get version
    /// information.
    fn name_package_module(&self, resolve: &Resolve, id: PackageId) -> String {
        let pkg = &resolve.packages[id];
        let versions_with_same_name = resolve
            .packages
            .iter()
            .filter_map(|(_, p)| {
                if p.name.namespace == pkg.name.namespace && p.name.name == pkg.name.name {
                    Some(&p.name.version)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let base = pkg.name.name.to_snake_case();
        if versions_with_same_name.len() == 1 {
            return base;
        }

        let version = match &pkg.name.version {
            Some(version) => version,
            // If this package didn't have a version then don't mangle its name
            // and other packages with the same name but with versions present
            // will have their names mangled.
            None => return base,
        };

        // Here there's multiple packages with the same name that differ only in
        // version, so the version needs to be mangled into the Rust module name
        // that we're generating. This in theory could look at all of
        // `versions_with_same_name` and produce a minimal diff, e.g. for 0.1.0
        // and 0.2.0 this could generate "foo1" and "foo2", but for now
        // a simpler path is chosen to generate "foo0_1_0" and "foo0_2_0".
        let version = version
            .to_string()
            .replace('.', "_")
            .replace('-', "_")
            .replace('+', "_")
            .to_snake_case();
        format!("{base}{version}")
    }

    fn generate(&mut self, resolve: &Resolve, id: WorldId) -> anyhow::Result<String> {
        self.types.analyze(resolve, id);

        self.world_link_options.write_struct(&mut self.src);

        // Resolve the `trappable_error_type` configuration values to `TypeId`
        // values. This is done by iterating over each `trappable_error_type`
        // and then locating the interface that it corresponds to as well as the
        // type within that interface.
        //
        // Note that `LookupItem::InterfaceNoPop` is used here as the full
        // hierarchical behavior of `lookup_keys` isn't used as the interface
        // must be named here.
        'outer: for (i, te) in self.opts.trappable_error_type.iter().enumerate() {
            let error_name = format!("_TrappableError{i}");
            for (id, iface) in resolve.interfaces.iter() {
                for (key, projection) in lookup_keys(
                    resolve,
                    &WorldKey::Interface(id),
                    LookupItem::InterfaceNoPop,
                ) {
                    assert!(projection.is_empty());

                    // If `wit_path` looks like `{key}/{type_name}` where
                    // `type_name` is a type within `iface` then we've found a
                    // match. Otherwise continue to the next lookup key if there
                    // is one, and failing that continue to the next interface.
                    let suffix = match te.wit_path.strip_prefix(&key) {
                        Some(s) => s,
                        None => continue,
                    };
                    let suffix = match suffix.strip_prefix('/') {
                        Some(s) => s,
                        None => continue,
                    };
                    if let Some(id) = iface.types.get(suffix) {
                        uwriteln!(self.src, "type {error_name} = {};", te.rust_type_name);
                        let prev = self.trappable_errors.insert(*id, error_name);
                        assert!(prev.is_none());
                        continue 'outer;
                    }
                }
            }

            bail!(
                "failed to locate a WIT error type corresponding to the \
                   `trappable_error_type` name `{}` provided",
                te.wit_path
            )
        }

        // Convert all entries in `with` as relative to the root of where the
        // macro itself is invoked. This emits a `pub use` to bring the name
        // into scope under an "anonymous name" which then replaces the `with`
        // map entry.
        let mut with = self.opts.with.iter_mut().collect::<Vec<_>>();
        with.sort();
        for (i, (_k, v)) in with.into_iter().enumerate() {
            let name = format!("__with_name{i}");
            uwriteln!(self.src, "#[doc(hidden)]\npub use {v} as {name};");
            *v = name;
        }

        let world = &resolve.worlds[id];
        for (name, import) in world.imports.iter() {
            if !self.opts.only_interfaces || matches!(import, WorldItem::Interface { .. }) {
                self.import(resolve, id, name, import);
            }
        }

        for (name, export) in world.exports.iter() {
            if !self.opts.only_interfaces || matches!(export, WorldItem::Interface { .. }) {
                self.export(resolve, name, export);
            }
        }
        self.finish(resolve, id)
    }

    fn import(&mut self, resolve: &Resolve, world: WorldId, name: &WorldKey, item: &WorldItem) {
        let mut generator = InterfaceGenerator::new(self, resolve);
        match item {
            WorldItem::Function(func) => {
                // Only generate a trait signature for free functions since
                // resource-related functions get their trait signatures
                // during `type_resource`.
                let sig = if let FunctionKind::Freestanding = func.kind {
                    generator.generate_function_trait_sig(func, "Data");
                    Some(mem::take(&mut generator.src).into())
                } else {
                    None
                };
                generator.generate_add_function_to_linker(TypeOwner::World(world), func, "linker");
                let add_to_linker = generator.src.into();
                self.import_functions.push(ImportFunction {
                    func: func.clone(),
                    sig,
                    add_to_linker,
                });
            }
            WorldItem::Interface { id, .. } => {
                generator
                    .generator
                    .interface_last_seen_as_import
                    .insert(*id, true);
                generator.current_interface = Some((*id, name, false));
                let snake = to_rust_ident(&match name {
                    WorldKey::Name(s) => s.to_snake_case(),
                    WorldKey::Interface(id) => resolve.interfaces[*id]
                        .name
                        .as_ref()
                        .unwrap()
                        .to_snake_case(),
                });
                let module = if generator
                    .generator
                    .name_interface(resolve, *id, name, false)
                {
                    // If this interface is remapped then that means that it was
                    // provided via the `with` key in the bindgen configuration.
                    // That means that bindings generation is skipped here. To
                    // accommodate future bindgens depending on this bindgen
                    // though we still generate a module which reexports the
                    // original module. This helps maintain the same output
                    // structure regardless of whether `with` is used.
                    let name_at_root = match &generator.generator.interface_names[id] {
                        InterfaceName::Remapped { name_at_root, .. } => name_at_root,
                        InterfaceName::Path(_) => unreachable!(),
                    };
                    let path_to_root = generator.path_to_root();
                    format!(
                        "
                            pub mod {snake} {{
                                #[allow(unused_imports)]
                                pub use {path_to_root}{name_at_root}::*;
                            }}
                        "
                    )
                } else {
                    // If this interface is not remapped then it's time to
                    // actually generate bindings here.
                    generator.generator.interface_link_options[id].write_struct(&mut generator.src);
                    generator.types(*id);
                    let key_name = resolve.name_world_key(name);
                    generator.generate_add_to_linker(*id, &key_name);

                    let module = &generator.src[..];
                    let wt = generator.generator.wasmtime_path();

                    format!(
                        "
                            #[allow(clippy::all)]
                            pub mod {snake} {{
                                #[allow(unused_imports)]
                                use {wt}::component::__internal::{{anyhow, Box}};

                                {module}
                            }}
                        "
                    )
                };
                self.import_interfaces.push((
                    name.clone(),
                    *id,
                    module,
                    self.interface_names[id].clone(),
                ));

                let interface_path = self.import_interface_path(id);
                self.interface_link_options[id]
                    .write_impl_from_world(&mut self.src, &interface_path);
            }
            WorldItem::Type(ty) => {
                let name = match name {
                    WorldKey::Name(name) => name,
                    WorldKey::Interface(_) => unreachable!(),
                };
                generator.define_type(name, *ty);
                let body = mem::take(&mut generator.src);
                self.src.push_str(&body);
            }
        };
    }

    fn export(&mut self, resolve: &Resolve, name: &WorldKey, item: &WorldItem) {
        let wt = self.wasmtime_path();
        let mut generator = InterfaceGenerator::new(self, resolve);
        let field;
        let ty;
        let ty_index;
        let load;
        let get_index_from_component;
        let get_index_from_instance;
        match item {
            WorldItem::Function(func) => {
                generator.define_rust_guest_export(resolve, None, func);
                let body = mem::take(&mut generator.src).into();
                load = generator.extract_typed_function(func).1;
                assert!(generator.src.is_empty());
                self.exports.funcs.push(body);
                ty_index = format!("{wt}::component::ComponentExportIndex");
                field = func_field_name(resolve, func);
                ty = format!("{wt}::component::Func");
                get_index_from_component = format!(
                    "_component.export_index(None, \"{}\")
                        .ok_or_else(|| anyhow::anyhow!(\"no function export `{0}` found\"))?.1",
                    func.name
                );
                get_index_from_instance = format!(
                    "_instance.get_export(&mut store, None, \"{}\")
                        .ok_or_else(|| anyhow::anyhow!(\"no function export `{0}` found\"))?",
                    func.name
                );
            }
            WorldItem::Type(_) => unreachable!(),
            WorldItem::Interface { id, .. } => {
                generator
                    .generator
                    .interface_last_seen_as_import
                    .insert(*id, false);
                generator.generator.name_interface(resolve, *id, name, true);
                generator.current_interface = Some((*id, name, true));
                generator.types(*id);
                let struct_name = "Guest";
                let iface = &resolve.interfaces[*id];
                let iface_name = match name {
                    WorldKey::Name(name) => name,
                    WorldKey::Interface(_) => iface.name.as_ref().unwrap(),
                };
                uwriteln!(generator.src, "pub struct {struct_name} {{");
                for (_, func) in iface.functions.iter() {
                    uwriteln!(
                        generator.src,
                        "{}: {wt}::component::Func,",
                        func_field_name(resolve, func)
                    );
                }
                uwriteln!(generator.src, "}}");

                uwriteln!(generator.src, "#[derive(Clone)]");
                uwriteln!(generator.src, "pub struct {struct_name}Indices {{");
                for (_, func) in iface.functions.iter() {
                    uwriteln!(
                        generator.src,
                        "{}: {wt}::component::ComponentExportIndex,",
                        func_field_name(resolve, func)
                    );
                }
                uwriteln!(generator.src, "}}");

                uwriteln!(generator.src, "impl {struct_name}Indices {{");
                let instance_name = resolve.name_world_key(name);
                uwrite!(
                    generator.src,
                    "
/// Constructor for [`{struct_name}Indices`] which takes a
/// [`Component`]({wt}::component::Component) as input and can be executed
/// before instantiation.
///
/// This constructor can be used to front-load string lookups to find exports
/// within a component.
pub fn new(
    component: &{wt}::component::Component,
) -> {wt}::Result<{struct_name}Indices> {{
    let (_, instance) = component.export_index(None, \"{instance_name}\")
        .ok_or_else(|| anyhow::anyhow!(\"no exported instance named `{instance_name}`\"))?;
    Self::_new(|name| {{
        component.export_index(Some(&instance), name)
            .map(|p| p.1)
    }})
}}

/// This constructor is similar to [`{struct_name}Indices::new`] except that it
/// performs string lookups after instantiation time.
pub fn new_instance(
    mut store: impl {wt}::AsContextMut,
    instance: &{wt}::component::Instance,
) -> {wt}::Result<{struct_name}Indices> {{
    let instance_export = instance.get_export(&mut store, None, \"{instance_name}\")
        .ok_or_else(|| anyhow::anyhow!(\"no exported instance named `{instance_name}`\"))?;
    Self::_new(|name| {{
        instance.get_export(&mut store, Some(&instance_export), name)
    }})
}}

fn _new(
    mut lookup: impl FnMut (&str) -> Option<{wt}::component::ComponentExportIndex>,
) -> {wt}::Result<{struct_name}Indices> {{
    let mut lookup = move |name| {{
        lookup(name).ok_or_else(|| {{
            anyhow::anyhow!(
                \"instance export `{instance_name}` does \\
                  not have export `{{name}}`\"
            )
        }})
    }};
    let _ = &mut lookup;
                    "
                );
                let mut fields = Vec::new();
                for (_, func) in iface.functions.iter() {
                    let name = func_field_name(resolve, func);
                    uwriteln!(generator.src, "let {name} = lookup(\"{}\")?;", func.name);
                    fields.push(name);
                }
                uwriteln!(generator.src, "Ok({struct_name}Indices {{");
                for name in fields {
                    uwriteln!(generator.src, "{name},");
                }
                uwriteln!(generator.src, "}})");
                uwriteln!(generator.src, "}}"); // end `fn _new`

                uwrite!(
                    generator.src,
                    "
                        pub fn load(
                            &self,
                            mut store: impl {wt}::AsContextMut,
                            instance: &{wt}::component::Instance,
                        ) -> {wt}::Result<{struct_name}> {{
                            let mut store = store.as_context_mut();
                            let _ = &mut store;
                            let _instance = instance;
                    "
                );
                let mut fields = Vec::new();
                for (_, func) in iface.functions.iter() {
                    let (name, getter) = generator.extract_typed_function(func);
                    uwriteln!(generator.src, "let {name} = {getter};");
                    fields.push(name);
                }
                uwriteln!(generator.src, "Ok({struct_name} {{");
                for name in fields {
                    uwriteln!(generator.src, "{name},");
                }
                uwriteln!(generator.src, "}})");
                uwriteln!(generator.src, "}}"); // end `fn new`
                uwriteln!(generator.src, "}}"); // end `impl {struct_name}Indices`

                uwriteln!(generator.src, "impl {struct_name} {{");
                let mut resource_methods = IndexMap::new();

                for (_, func) in iface.functions.iter() {
                    match func.kind {
                        FunctionKind::Freestanding => {
                            generator.define_rust_guest_export(resolve, Some(name), func);
                        }
                        FunctionKind::Method(id)
                        | FunctionKind::Constructor(id)
                        | FunctionKind::Static(id) => {
                            resource_methods.entry(id).or_insert(Vec::new()).push(func);
                        }
                    }
                }

                for (id, _) in resource_methods.iter() {
                    let name = resolve.types[*id].name.as_ref().unwrap();
                    let snake = name.to_snake_case();
                    let camel = name.to_upper_camel_case();
                    uwriteln!(
                        generator.src,
                        "pub fn {snake}(&self) -> Guest{camel}<'_> {{
                            Guest{camel} {{ funcs: self }}
                        }}"
                    );
                }

                uwriteln!(generator.src, "}}");

                for (id, methods) in resource_methods {
                    let resource_name = resolve.types[id].name.as_ref().unwrap();
                    let camel = resource_name.to_upper_camel_case();
                    uwriteln!(generator.src, "impl Guest{camel}<'_> {{");
                    for method in methods {
                        generator.define_rust_guest_export(resolve, Some(name), method);
                    }
                    uwriteln!(generator.src, "}}");
                }

                let module = &generator.src[..];
                let snake = to_rust_ident(iface_name);

                let module = format!(
                    "
                        #[allow(clippy::all)]
                        pub mod {snake} {{
                            #[allow(unused_imports)]
                            use {wt}::component::__internal::{{anyhow, Box}};

                            {module}
                        }}
                    "
                );
                let pkgname = match name {
                    WorldKey::Name(_) => None,
                    WorldKey::Interface(_) => {
                        Some(resolve.packages[iface.package.unwrap()].name.clone())
                    }
                };
                self.exports
                    .modules
                    .push((*id, module, self.interface_names[id].clone()));

                let (path, method_name) = match pkgname {
                    Some(pkgname) => (
                        format!(
                            "exports::{}::{}::{snake}::{struct_name}",
                            pkgname.namespace.to_snake_case(),
                            self.name_package_module(resolve, iface.package.unwrap()),
                        ),
                        format!(
                            "{}_{}_{snake}",
                            pkgname.namespace.to_snake_case(),
                            self.name_package_module(resolve, iface.package.unwrap())
                        ),
                    ),
                    None => (format!("exports::{snake}::{struct_name}"), snake.clone()),
                };
                field = format!("interface{}", self.exports.fields.len());
                load = format!("self.{field}.load(&mut store, &_instance)?");
                self.exports.funcs.push(format!(
                    "
                        pub fn {method_name}(&self) -> &{path} {{
                            &self.{field}
                        }}
                    ",
                ));
                ty_index = format!("{path}Indices");
                ty = path;
                get_index_from_component = format!("{ty_index}::new(_component)?");
                get_index_from_instance =
                    format!("{ty_index}::new_instance(&mut store, _instance)?");
            }
        }
        let prev = self.exports.fields.insert(
            field,
            ExportField {
                ty,
                ty_index,
                load,
                get_index_from_component,
                get_index_from_instance,
            },
        );
        assert!(prev.is_none());
    }

    fn build_world_struct(&mut self, resolve: &Resolve, world: WorldId) {
        let wt = self.wasmtime_path();
        let world_name = &resolve.worlds[world].name;
        let camel = to_rust_upper_camel_case(&world_name);
        let (async_, async__, where_clause, await_) = match self.opts.call_style() {
            CallStyle::Async => ("async", "_async", "where _T: Send", ".await"),
            CallStyle::Concurrent => ("async", "_async", "where _T: Send + 'static", ".await"),
            CallStyle::Sync => ("", "", "", ""),
        };
        uwriteln!(
            self.src,
            "
/// Auto-generated bindings for a pre-instantiated version of a
/// component which implements the world `{world_name}`.
///
/// This structure is created through [`{camel}Pre::new`] which
/// takes a [`InstancePre`]({wt}::component::InstancePre) that
/// has been created through a [`Linker`]({wt}::component::Linker).
///
/// For more information see [`{camel}`] as well.
pub struct {camel}Pre<T> {{
    instance_pre: {wt}::component::InstancePre<T>,
    indices: {camel}Indices,
}}

impl<T> Clone for {camel}Pre<T> {{
    fn clone(&self) -> Self {{
        Self {{
            instance_pre: self.instance_pre.clone(),
            indices: self.indices.clone(),
        }}
    }}
}}

impl<_T> {camel}Pre<_T> {{
    /// Creates a new copy of `{camel}Pre` bindings which can then
    /// be used to instantiate into a particular store.
    ///
    /// This method may fail if the component behind `instance_pre`
    /// does not have the required exports.
    pub fn new(instance_pre: {wt}::component::InstancePre<_T>) -> {wt}::Result<Self> {{
        let indices = {camel}Indices::new(instance_pre.component())?;
        Ok(Self {{ instance_pre, indices }})
    }}

    pub fn engine(&self) -> &{wt}::Engine {{
        self.instance_pre.engine()
    }}

    pub fn instance_pre(&self) -> &{wt}::component::InstancePre<_T> {{
        &self.instance_pre
    }}

    /// Instantiates a new instance of [`{camel}`] within the
    /// `store` provided.
    ///
    /// This function will use `self` as the pre-instantiated
    /// instance to perform instantiation. Afterwards the preloaded
    /// indices in `self` are used to lookup all exports on the
    /// resulting instance.
    pub {async_} fn instantiate{async__}(
        &self,
        mut store: impl {wt}::AsContextMut<Data = _T>,
    ) -> {wt}::Result<{camel}>
        {where_clause}
    {{
        let mut store = store.as_context_mut();
        let instance = self.instance_pre.instantiate{async__}(&mut store){await_}?;
        self.indices.load(&mut store, &instance)
    }}
}}
"
        );

        uwriteln!(
            self.src,
            "
            /// Auto-generated bindings for index of the exports of
            /// `{world_name}`.
            ///
            /// This is an implementation detail of [`{camel}Pre`] and can
            /// be constructed if needed as well.
            ///
            /// For more information see [`{camel}`] as well.
            #[derive(Clone)]
            pub struct {camel}Indices {{"
        );
        for (name, field) in self.exports.fields.iter() {
            uwriteln!(self.src, "{name}: {},", field.ty_index);
        }
        self.src.push_str("}\n");

        uwriteln!(
            self.src,
            "
                /// Auto-generated bindings for an instance a component which
                /// implements the world `{world_name}`.
                ///
                /// This structure can be created through a number of means
                /// depending on your requirements and what you have on hand:
                ///
                /// * The most convenient way is to use
                ///   [`{camel}::instantiate{async__}`] which only needs a
                ///   [`Store`], [`Component`], and [`Linker`].
                ///
                /// * Alternatively you can create a [`{camel}Pre`] ahead of
                ///   time with a [`Component`] to front-load string lookups
                ///   of exports once instead of per-instantiation. This
                ///   method then uses [`{camel}Pre::instantiate{async__}`] to
                ///   create a [`{camel}`].
                ///
                /// * If you've instantiated the instance yourself already
                ///   then you can use [`{camel}::new`].
                ///
                /// * You can also access the guts of instantiation through
                ///   [`{camel}Indices::new_instance`] followed
                ///   by [`{camel}Indices::load`] to crate an instance of this
                ///   type.
                ///
                /// These methods are all equivalent to one another and move
                /// around the tradeoff of what work is performed when.
                ///
                /// [`Store`]: {wt}::Store
                /// [`Component`]: {wt}::component::Component
                /// [`Linker`]: {wt}::component::Linker
                pub struct {camel} {{"
        );
        for (name, field) in self.exports.fields.iter() {
            uwriteln!(self.src, "{name}: {},", field.ty);
        }
        self.src.push_str("}\n");

        self.world_imports_trait(resolve, world);

        uwriteln!(self.src, "const _: () = {{");
        uwriteln!(
            self.src,
            "
                #[allow(unused_imports)]
                use {wt}::component::__internal::anyhow;
            "
        );

        uwriteln!(
            self.src,
            "impl {camel}Indices {{
                /// Creates a new copy of `{camel}Indices` bindings which can then
                /// be used to instantiate into a particular store.
                ///
                /// This method may fail if the component does not have the
                /// required exports.
                pub fn new(component: &{wt}::component::Component) -> {wt}::Result<Self> {{
                    let _component = component;
            ",
        );
        for (name, field) in self.exports.fields.iter() {
            uwriteln!(self.src, "let {name} = {};", field.get_index_from_component);
        }
        uwriteln!(self.src, "Ok({camel}Indices {{");
        for (name, _) in self.exports.fields.iter() {
            uwriteln!(self.src, "{name},");
        }
        uwriteln!(self.src, "}})");
        uwriteln!(self.src, "}}"); // close `fn new`

        uwriteln!(
            self.src,
            "
                /// Creates a new instance of [`{camel}Indices`] from an
                /// instantiated component.
                ///
                /// This method of creating a [`{camel}`] will perform string
                /// lookups for all exports when this method is called. This
                /// will only succeed if the provided instance matches the
                /// requirements of [`{camel}`].
                pub fn new_instance(
                    mut store: impl {wt}::AsContextMut,
                    instance: &{wt}::component::Instance,
                ) -> {wt}::Result<Self> {{
                    let _instance = instance;
            ",
        );
        for (name, field) in self.exports.fields.iter() {
            uwriteln!(self.src, "let {name} = {};", field.get_index_from_instance);
        }
        uwriteln!(self.src, "Ok({camel}Indices {{");
        for (name, _) in self.exports.fields.iter() {
            uwriteln!(self.src, "{name},");
        }
        uwriteln!(self.src, "}})");
        uwriteln!(self.src, "}}"); // close `fn new_instance`

        uwriteln!(
            self.src,
            "
                /// Uses the indices stored in `self` to load an instance
                /// of [`{camel}`] from the instance provided.
                ///
                /// Note that at this time this method will additionally
                /// perform type-checks of all exports.
                pub fn load(
                    &self,
                    mut store: impl {wt}::AsContextMut,
                    instance: &{wt}::component::Instance,
                ) -> {wt}::Result<{camel}> {{
                    let _instance = instance;
            ",
        );
        for (name, field) in self.exports.fields.iter() {
            uwriteln!(self.src, "let {name} = {};", field.load);
        }
        uwriteln!(self.src, "Ok({camel} {{");
        for (name, _) in self.exports.fields.iter() {
            uwriteln!(self.src, "{name},");
        }
        uwriteln!(self.src, "}})");
        uwriteln!(self.src, "}}"); // close `fn load`
        uwriteln!(self.src, "}}"); // close `impl {camel}Indices`

        uwriteln!(
            self.src,
            "impl {camel} {{
                /// Convenience wrapper around [`{camel}Pre::new`] and
                /// [`{camel}Pre::instantiate{async__}`].
                pub {async_} fn instantiate{async__}<_T>(
                    mut store: impl {wt}::AsContextMut<Data = _T>,
                    component: &{wt}::component::Component,
                    linker: &{wt}::component::Linker<_T>,
                ) -> {wt}::Result<{camel}>
                    {where_clause}
                {{
                    let pre = linker.instantiate_pre(component)?;
                    {camel}Pre::new(pre)?.instantiate{async__}(store){await_}
                }}

                /// Convenience wrapper around [`{camel}Indices::new_instance`] and
                /// [`{camel}Indices::load`].
                pub fn new(
                    mut store: impl {wt}::AsContextMut,
                    instance: &{wt}::component::Instance,
                ) -> {wt}::Result<{camel}> {{
                    let indices = {camel}Indices::new_instance(&mut store, instance)?;
                    indices.load(store, instance)
                }}
            ",
        );
        self.world_add_to_linker(resolve, world);

        for func in self.exports.funcs.iter() {
            self.src.push_str(func);
        }

        uwriteln!(self.src, "}}"); // close `impl {camel}`

        uwriteln!(self.src, "}};"); // close `const _: () = ...
    }

    fn finish(&mut self, resolve: &Resolve, world: WorldId) -> anyhow::Result<String> {
        let remapping_keys = self.opts.with.keys().cloned().collect::<HashSet<String>>();

        let mut unused_keys = remapping_keys
            .difference(&self.used_with_opts)
            .map(|s| s.as_str())
            .collect::<Vec<&str>>();

        unused_keys.sort();

        if !unused_keys.is_empty() {
            anyhow::bail!("interfaces were specified in the `with` config option but are not referenced in the target world: {unused_keys:?}");
        }

        if let TrappableImports::Only(only) = &self.opts.trappable_imports {
            let mut unused_imports = Vec::from_iter(
                only.difference(&self.used_trappable_imports_opts)
                    .map(|s| s.as_str()),
            );

            if !unused_imports.is_empty() {
                unused_imports.sort();
                anyhow::bail!("names specified in the `trappable_imports` config option but are not referenced in the target world: {unused_imports:?}");
            }
        }

        if !self.opts.only_interfaces {
            self.build_world_struct(resolve, world)
        }

        let imports = mem::take(&mut self.import_interfaces);
        self.emit_modules(
            imports
                .into_iter()
                .map(|(_, id, module, path)| (id, module, path))
                .collect(),
        );

        let exports = mem::take(&mut self.exports.modules);
        self.emit_modules(exports);

        let mut src = mem::take(&mut self.src);
        if self.opts.rustfmt {
            let mut child = Command::new("rustfmt")
                .arg("--edition=2018")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .expect("failed to spawn `rustfmt`");
            child
                .stdin
                .take()
                .unwrap()
                .write_all(src.as_bytes())
                .unwrap();
            src.as_mut_string().truncate(0);
            child
                .stdout
                .take()
                .unwrap()
                .read_to_string(src.as_mut_string())
                .unwrap();
            let status = child.wait().unwrap();
            assert!(status.success());
        }

        Ok(src.into())
    }

    fn emit_modules(&mut self, modules: Vec<(InterfaceId, String, InterfaceName)>) {
        #[derive(Default)]
        struct Module {
            submodules: BTreeMap<String, Module>,
            contents: Vec<String>,
        }
        let mut map = Module::default();
        for (_, module, name) in modules {
            let path = match name {
                InterfaceName::Remapped { local_path, .. } => local_path,
                InterfaceName::Path(path) => path,
            };
            let mut cur = &mut map;
            for name in path[..path.len() - 1].iter() {
                cur = cur
                    .submodules
                    .entry(name.clone())
                    .or_insert(Module::default());
            }
            cur.contents.push(module);
        }

        emit(&mut self.src, map);

        fn emit(me: &mut Source, module: Module) {
            for (name, submodule) in module.submodules {
                uwriteln!(me, "pub mod {name} {{");
                emit(me, submodule);
                uwriteln!(me, "}}");
            }
            for submodule in module.contents {
                uwriteln!(me, "{submodule}");
            }
        }
    }

    /// Attempts to find the `key`, possibly with the resource projection
    /// `item`, within the `with` map provided to bindings configuration.
    fn lookup_replacement(
        &mut self,
        resolve: &Resolve,
        key: &WorldKey,
        item: Option<&str>,
    ) -> Option<String> {
        let item = match item {
            Some(item) => LookupItem::Name(item),
            None => LookupItem::None,
        };

        for (lookup, mut projection) in lookup_keys(resolve, key, item) {
            if let Some(renamed) = self.opts.with.get(&lookup) {
                projection.push(renamed.clone());
                projection.reverse();
                self.used_with_opts.insert(lookup);
                return Some(projection.join("::"));
            }
        }

        None
    }

    fn wasmtime_path(&self) -> String {
        self.opts
            .wasmtime_crate
            .clone()
            .unwrap_or("wasmtime".to_string())
    }
}

enum LookupItem<'a> {
    None,
    Name(&'a str),
    InterfaceNoPop,
}

fn lookup_keys(
    resolve: &Resolve,
    key: &WorldKey,
    item: LookupItem<'_>,
) -> Vec<(String, Vec<String>)> {
    struct Name<'a> {
        prefix: Prefix,
        item: Option<&'a str>,
    }

    #[derive(Copy, Clone)]
    enum Prefix {
        Namespace(PackageId),
        UnversionedPackage(PackageId),
        VersionedPackage(PackageId),
        UnversionedInterface(InterfaceId),
        VersionedInterface(InterfaceId),
    }

    let prefix = match key {
        WorldKey::Interface(id) => Prefix::VersionedInterface(*id),

        // Non-interface-keyed names don't get the lookup logic below,
        // they're relatively uncommon so only lookup the precise key here.
        WorldKey::Name(key) => {
            let to_lookup = match item {
                LookupItem::Name(item) => format!("{key}/{item}"),
                LookupItem::None | LookupItem::InterfaceNoPop => key.to_string(),
            };
            return vec![(to_lookup, Vec::new())];
        }
    };

    // Here names are iteratively attempted as `key` + `item` is "walked to
    // its root" and each attempt is consulted in `self.opts.with`. This
    // loop will start at the leaf, the most specific path, and then walk to
    // the root, popping items, trying to find a result.
    //
    // Each time a name is "popped" the projection from the next path is
    // pushed onto `projection`. This means that if we actually find a match
    // then `projection` is a collection of namespaces that results in the
    // final replacement name.
    let (interface_required, item) = match item {
        LookupItem::None => (false, None),
        LookupItem::Name(s) => (false, Some(s)),
        LookupItem::InterfaceNoPop => (true, None),
    };
    let mut name = Name { prefix, item };
    let mut projection = Vec::new();
    let mut ret = Vec::new();
    loop {
        let lookup = name.lookup_key(resolve);
        ret.push((lookup, projection.clone()));
        if !name.pop(resolve, &mut projection) {
            break;
        }
        if interface_required {
            match name.prefix {
                Prefix::VersionedInterface(_) | Prefix::UnversionedInterface(_) => {}
                _ => break,
            }
        }
    }

    return ret;

    impl<'a> Name<'a> {
        fn lookup_key(&self, resolve: &Resolve) -> String {
            let mut s = self.prefix.lookup_key(resolve);
            if let Some(item) = self.item {
                s.push_str("/");
                s.push_str(item);
            }
            s
        }

        fn pop(&mut self, resolve: &'a Resolve, projection: &mut Vec<String>) -> bool {
            match (self.item, self.prefix) {
                // If this is a versioned resource name, try the unversioned
                // resource name next.
                (Some(_), Prefix::VersionedInterface(id)) => {
                    self.prefix = Prefix::UnversionedInterface(id);
                    true
                }
                // If this is an unversioned resource name then time to
                // ignore the resource itself and move on to the next most
                // specific item, versioned interface names.
                (Some(item), Prefix::UnversionedInterface(id)) => {
                    self.prefix = Prefix::VersionedInterface(id);
                    self.item = None;
                    projection.push(item.to_upper_camel_case());
                    true
                }
                (Some(_), _) => unreachable!(),
                (None, _) => self.prefix.pop(resolve, projection),
            }
        }
    }

    impl Prefix {
        fn lookup_key(&self, resolve: &Resolve) -> String {
            match *self {
                Prefix::Namespace(id) => resolve.packages[id].name.namespace.clone(),
                Prefix::UnversionedPackage(id) => {
                    let mut name = resolve.packages[id].name.clone();
                    name.version = None;
                    name.to_string()
                }
                Prefix::VersionedPackage(id) => resolve.packages[id].name.to_string(),
                Prefix::UnversionedInterface(id) => {
                    let id = resolve.id_of(id).unwrap();
                    match id.find('@') {
                        Some(i) => id[..i].to_string(),
                        None => id,
                    }
                }
                Prefix::VersionedInterface(id) => resolve.id_of(id).unwrap(),
            }
        }

        fn pop(&mut self, resolve: &Resolve, projection: &mut Vec<String>) -> bool {
            *self = match *self {
                // try the unversioned interface next
                Prefix::VersionedInterface(id) => Prefix::UnversionedInterface(id),
                // try this interface's versioned package next
                Prefix::UnversionedInterface(id) => {
                    let iface = &resolve.interfaces[id];
                    let name = iface.name.as_ref().unwrap();
                    projection.push(to_rust_ident(name));
                    Prefix::VersionedPackage(iface.package.unwrap())
                }
                // try the unversioned package next
                Prefix::VersionedPackage(id) => Prefix::UnversionedPackage(id),
                // try this package's namespace next
                Prefix::UnversionedPackage(id) => {
                    let name = &resolve.packages[id].name;
                    projection.push(to_rust_ident(&name.name));
                    Prefix::Namespace(id)
                }
                // nothing left to try any more
                Prefix::Namespace(_) => return false,
            };
            true
        }
    }
}

impl Wasmtime {
    fn has_world_imports_trait(&self, resolve: &Resolve, world: WorldId) -> bool {
        !self.import_functions.is_empty() || get_world_resources(resolve, world).count() > 0
    }

    fn world_imports_trait(&mut self, resolve: &Resolve, world: WorldId) {
        if !self.has_world_imports_trait(resolve, world) {
            return;
        }

        let wt = self.wasmtime_path();
        let world_camel = to_rust_upper_camel_case(&resolve.worlds[world].name);
        if let CallStyle::Async = self.opts.call_style() {
            uwriteln!(
                self.src,
                "#[{wt}::component::__internal::trait_variant_make(::core::marker::Send)]"
            )
        }
        uwrite!(self.src, "pub trait {world_camel}Imports");
        let mut supertraits = vec![];
        if let CallStyle::Async = self.opts.call_style() {
            supertraits.push("Send".to_string());
        }
        for (_, name) in get_world_resources(resolve, world) {
            supertraits.push(format!("Host{}", name.to_upper_camel_case()));
        }
        if !supertraits.is_empty() {
            uwrite!(self.src, ": {}", supertraits.join(" + "));
        }
        uwriteln!(self.src, " {{");

        let has_concurrent_function = self.import_functions.iter().any(|func| {
            matches!(func.func.kind, FunctionKind::Freestanding)
                && matches!(
                    self.opts.import_call_style(None, &func.func.name),
                    CallStyle::Concurrent
                )
        });

        if has_concurrent_function {
            self.src.push_str("type Data;\n");
        }

        for f in self.import_functions.iter() {
            if let Some(sig) = &f.sig {
                self.src.push_str(sig);
                self.src.push_str(";\n");
            }
        }
        uwriteln!(self.src, "}}");

        let get_host_bounds = if let CallStyle::Concurrent = self.opts.call_style() {
            let constraints = world_imports_concurrent_constraints(resolve, world, &self.opts);

            format!("{world_camel}Imports{}", constraints("D"))
        } else {
            format!("{world_camel}Imports")
        };

        uwriteln!(
            self.src,
            "
                pub trait {world_camel}ImportsGetHost<T, D>:
                    Fn(T) -> <Self as {world_camel}ImportsGetHost<T, D>>::Host
                        + Send
                        + Sync
                        + Copy
                        + 'static
                {{
                    type Host: {get_host_bounds};
                }}

                impl<F, T, D, O> {world_camel}ImportsGetHost<T, D> for F
                where
                    F: Fn(T) -> O + Send + Sync + Copy + 'static,
                    O: {get_host_bounds},
                {{
                    type Host = O;
                }}
            "
        );

        // Generate impl WorldImports for &mut WorldImports
        let maybe_send = if let CallStyle::Async = self.opts.call_style() {
            "+ Send"
        } else {
            ""
        };
        if !self.opts.skip_mut_forwarding_impls {
            let maybe_maybe_sized = if let CallStyle::Concurrent = self.opts.call_style() {
                ""
            } else {
                "+ ?Sized"
            };
            uwriteln!(
                self.src,
                    "impl<_T: {world_camel}Imports {maybe_maybe_sized} {maybe_send}> {world_camel}Imports for &mut _T {{"
            );
            let has_concurrent_function = self.import_functions.iter().any(|f| {
                matches!(
                    self.opts.import_call_style(None, &f.func.name),
                    CallStyle::Concurrent
                )
            });

            if has_concurrent_function {
                self.src.push_str("type Data = _T::Data;\n");
            }
            // Forward each method call to &mut T
            for f in self.import_functions.iter() {
                if let Some(sig) = &f.sig {
                    self.src.push_str(sig);
                    let call_style = self.opts.import_call_style(None, &f.func.name);
                    if let CallStyle::Concurrent = &call_style {
                        uwrite!(
                            self.src,
                            "{{ <_T as {world_camel}Imports>::{}(store,",
                            rust_function_name(&f.func)
                        );
                    } else {
                        uwrite!(
                            self.src,
                            "{{ {world_camel}Imports::{}(*self,",
                            rust_function_name(&f.func)
                        );
                    }
                    for (name, _) in f.func.params.iter() {
                        uwrite!(self.src, "{},", to_rust_ident(name));
                    }
                    uwrite!(self.src, ")");
                    if let CallStyle::Async = &call_style {
                        uwrite!(self.src, ".await");
                    }
                    uwriteln!(self.src, "}}");
                }
            }
            uwriteln!(self.src, "}}");
        }
    }

    fn import_interface_paths(&self) -> Vec<(InterfaceId, String)> {
        self.import_interfaces
            .iter()
            .map(|(_, id, _, name)| {
                let path = match name {
                    InterfaceName::Path(path) => path.join("::"),
                    InterfaceName::Remapped { name_at_root, .. } => name_at_root.clone(),
                };
                (*id, path)
            })
            .collect()
    }

    fn import_interface_path(&self, id: &InterfaceId) -> String {
        match &self.interface_names[id] {
            InterfaceName::Path(path) => path.join("::"),
            InterfaceName::Remapped { name_at_root, .. } => name_at_root.clone(),
        }
    }

    fn world_host_traits(&self, resolve: &Resolve, world: WorldId) -> Vec<String> {
        let mut traits = self
            .import_interface_paths()
            .iter()
            .map(|(_, path)| format!("{path}::Host"))
            .collect::<Vec<_>>();
        if self.has_world_imports_trait(resolve, world) {
            let world_camel = to_rust_upper_camel_case(&resolve.worlds[world].name);
            traits.push(format!("{world_camel}Imports"));
        }
        if let CallStyle::Async = self.opts.call_style() {
            traits.push("Send".to_string());
        }
        traits
    }

    fn world_add_to_linker(&mut self, resolve: &Resolve, world: WorldId) {
        let has_world_imports_trait = self.has_world_imports_trait(resolve, world);
        if self.import_interfaces.is_empty() && !has_world_imports_trait {
            return;
        }

        let (options_param, options_arg) = if self.world_link_options.has_any() {
            ("options: &LinkOptions,", ", options")
        } else {
            ("", "")
        };

        let camel = to_rust_upper_camel_case(&resolve.worlds[world].name);

        let data_bounds = if self.opts.is_store_data_send() {
            if let CallStyle::Concurrent = self.opts.call_style() {
                "T: Send + 'static,"
            } else {
                "T: Send,"
            }
        } else {
            ""
        };
        let wt = self.wasmtime_path();
        if has_world_imports_trait {
            let host_bounds = if let CallStyle::Concurrent = self.opts.call_style() {
                let constraints = world_imports_concurrent_constraints(resolve, world, &self.opts);

                format!("{camel}Imports{}", constraints("T"))
            } else {
                format!("{camel}Imports")
            };

            uwrite!(
                self.src,
                "
                    pub fn add_to_linker_imports_get_host<
                        T,
                        G: for<'a> {camel}ImportsGetHost<&'a mut T, T, Host: {host_bounds}>
                    >(
                        linker: &mut {wt}::component::Linker<T>,
                        {options_param}
                        host_getter: G,
                    ) -> {wt}::Result<()>
                        where {data_bounds}
                    {{
                        let mut linker = linker.root();
                "
            );
            let gate = FeatureGate::open(&mut self.src, &resolve.worlds[world].stability);
            for (ty, name) in get_world_resources(resolve, world) {
                Self::generate_add_resource_to_linker(
                    None,
                    &mut self.src,
                    &self.opts,
                    &wt,
                    "linker",
                    name,
                    &resolve.types[ty].stability,
                );
            }
            for f in self.import_functions.iter() {
                self.src.push_str(&f.add_to_linker);
                self.src.push_str("\n");
            }
            gate.close(&mut self.src);
            uwriteln!(self.src, "Ok(())\n}}");
        }

        let (host_bounds, data_bounds) = if let CallStyle::Concurrent = self.opts.call_style() {
            let bounds = self
                .import_interfaces
                .iter()
                .map(|(key, id, _, name)| {
                    (
                        key,
                        id,
                        match name {
                            InterfaceName::Path(path) => path.join("::"),
                            InterfaceName::Remapped { name_at_root, .. } => name_at_root.clone(),
                        },
                    )
                })
                .map(|(key, id, path)| {
                    format!(
                        " + {path}::Host{}",
                        concurrent_constraints(
                            resolve,
                            &self.opts,
                            Some(&resolve.name_world_key(key)),
                            *id
                        )("T")
                    )
                })
                .chain(if self.has_world_imports_trait(resolve, world) {
                    let world_camel = to_rust_upper_camel_case(&resolve.worlds[world].name);
                    let constraints =
                        world_imports_concurrent_constraints(resolve, world, &self.opts);
                    Some(format!(" + {world_camel}Imports{}", constraints("T")))
                } else {
                    None
                })
                .collect::<Vec<_>>()
                .concat();

            (
                format!("U: Send{bounds}"),
                format!("T: Send{bounds} + 'static,"),
            )
        } else {
            (
                format!("U: {}", self.world_host_traits(resolve, world).join(" + ")),
                data_bounds.to_string(),
            )
        };

        if !self.opts.skip_mut_forwarding_impls {
            uwriteln!(
                self.src,
                "
                    pub fn add_to_linker<T, U>(
                        linker: &mut {wt}::component::Linker<T>,
                        {options_param}
                        get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
                    ) -> {wt}::Result<()>
                        where
                            {data_bounds}
                            {host_bounds}
                    {{
                "
            );
            let gate = FeatureGate::open(&mut self.src, &resolve.worlds[world].stability);
            if has_world_imports_trait {
                uwriteln!(
                    self.src,
                    "Self::add_to_linker_imports_get_host(linker {options_arg}, get)?;"
                );
            }
            for (interface_id, path) in self.import_interface_paths() {
                let options_arg = if self.interface_link_options[&interface_id].has_any() {
                    ", &options.into()"
                } else {
                    ""
                };

                let import_stability = resolve.worlds[world]
                    .imports
                    .iter()
                    .filter_map(|(_, i)| match i {
                        WorldItem::Interface { id, stability } if *id == interface_id => {
                            Some(stability.clone())
                        }
                        _ => None,
                    })
                    .next()
                    .unwrap_or(Stability::Unknown);

                let gate = FeatureGate::open(&mut self.src, &import_stability);
                uwriteln!(
                    self.src,
                    "{path}::add_to_linker(linker {options_arg}, get)?;"
                );
                gate.close(&mut self.src);
            }
            gate.close(&mut self.src);
            uwriteln!(self.src, "Ok(())\n}}");
        }
    }

    fn generate_add_resource_to_linker(
        qualifier: Option<&str>,
        src: &mut Source,
        opts: &Opts,
        wt: &str,
        inst: &str,
        name: &str,
        stability: &Stability,
    ) {
        let gate = FeatureGate::open(src, stability);
        let camel = name.to_upper_camel_case();
        if let CallStyle::Async = opts.drop_call_style(qualifier, name) {
            uwriteln!(
                src,
                "{inst}.resource_async(
                    \"{name}\",
                    {wt}::component::ResourceType::host::<{camel}>(),
                    move |mut store, rep| {{
                        {wt}::component::__internal::Box::new(async move {{
                            Host{camel}::drop(&mut host_getter(store.data_mut()), {wt}::component::Resource::new_own(rep)).await
                        }})
                    }},
                )?;"
            )
        } else {
            uwriteln!(
                src,
                "{inst}.resource(
                    \"{name}\",
                    {wt}::component::ResourceType::host::<{camel}>(),
                    move |mut store, rep| -> {wt}::Result<()> {{
                        Host{camel}::drop(&mut host_getter(store.data_mut()), {wt}::component::Resource::new_own(rep))
                    }},
                )?;"
            )
        }
        gate.close(src);
    }
}

struct InterfaceGenerator<'a> {
    src: Source,
    generator: &'a mut Wasmtime,
    resolve: &'a Resolve,
    current_interface: Option<(InterfaceId, &'a WorldKey, bool)>,
}

impl<'a> InterfaceGenerator<'a> {
    fn new(generator: &'a mut Wasmtime, resolve: &'a Resolve) -> InterfaceGenerator<'a> {
        InterfaceGenerator {
            src: Source::default(),
            generator,
            resolve,
            current_interface: None,
        }
    }

    fn types_imported(&self) -> bool {
        match self.current_interface {
            Some((_, _, is_export)) => !is_export,
            None => true,
        }
    }

    fn types(&mut self, id: InterfaceId) {
        for (name, id) in self.resolve.interfaces[id].types.iter() {
            self.define_type(name, *id);
        }
    }

    fn define_type(&mut self, name: &str, id: TypeId) {
        let ty = &self.resolve.types[id];
        match &ty.kind {
            TypeDefKind::Record(record) => self.type_record(id, name, record, &ty.docs),
            TypeDefKind::Flags(flags) => self.type_flags(id, name, flags, &ty.docs),
            TypeDefKind::Tuple(tuple) => self.type_tuple(id, name, tuple, &ty.docs),
            TypeDefKind::Enum(enum_) => self.type_enum(id, name, enum_, &ty.docs),
            TypeDefKind::Variant(variant) => self.type_variant(id, name, variant, &ty.docs),
            TypeDefKind::Option(t) => self.type_option(id, name, t, &ty.docs),
            TypeDefKind::Result(r) => self.type_result(id, name, r, &ty.docs),
            TypeDefKind::List(t) => self.type_list(id, name, t, &ty.docs),
            TypeDefKind::Type(t) => self.type_alias(id, name, t, &ty.docs),
            TypeDefKind::Future(t) => self.type_future(id, name, t.as_ref(), &ty.docs),
            TypeDefKind::Stream(t) => self.type_stream(id, name, t.as_ref(), &ty.docs),
            TypeDefKind::ErrorContext => self.type_error_context(id, name, &ty.docs),
            TypeDefKind::Handle(handle) => self.type_handle(id, name, handle, &ty.docs),
            TypeDefKind::Resource => self.type_resource(id, name, ty, &ty.docs),
            TypeDefKind::Unknown => unreachable!(),
        }
    }

    fn type_handle(&mut self, id: TypeId, name: &str, handle: &Handle, docs: &Docs) {
        self.rustdoc(docs);
        let name = name.to_upper_camel_case();
        uwriteln!(self.src, "pub type {name} = ");
        self.print_handle(handle);
        self.push_str(";\n");
        self.assert_type(id, &name);
    }

    fn type_resource(&mut self, id: TypeId, name: &str, resource: &TypeDef, docs: &Docs) {
        let camel = name.to_upper_camel_case();
        let wt = self.generator.wasmtime_path();

        if self.types_imported() {
            self.rustdoc(docs);

            let replacement = match self.current_interface {
                Some((_, key, _)) => {
                    self.generator
                        .lookup_replacement(self.resolve, key, Some(name))
                }
                None => {
                    self.generator.used_with_opts.insert(name.into());
                    self.generator.opts.with.get(name).cloned()
                }
            };
            match replacement {
                Some(path) => {
                    uwriteln!(
                        self.src,
                        "pub use {}{path} as {camel};",
                        self.path_to_root()
                    );
                }
                None => {
                    uwriteln!(self.src, "pub enum {camel} {{}}");
                }
            }

            // Generate resource trait
            if let CallStyle::Async = self.generator.opts.call_style() {
                uwriteln!(
                    self.src,
                    "#[{wt}::component::__internal::trait_variant_make(::core::marker::Send)]"
                )
            }

            uwriteln!(self.src, "pub trait Host{camel}: Sized {{");

            let mut functions = match resource.owner {
                TypeOwner::World(id) => self.resolve.worlds[id]
                    .imports
                    .values()
                    .filter_map(|item| match item {
                        WorldItem::Function(f) => Some(f),
                        _ => None,
                    })
                    .collect(),
                TypeOwner::Interface(id) => self.resolve.interfaces[id]
                    .functions
                    .values()
                    .collect::<Vec<_>>(),
                TypeOwner::None => {
                    panic!("A resource must be owned by a world or interface");
                }
            };

            functions.retain(|func| match func.kind {
                FunctionKind::Freestanding => false,
                FunctionKind::Method(resource)
                | FunctionKind::Static(resource)
                | FunctionKind::Constructor(resource) => id == resource,
            });

            let has_concurrent_function = functions.iter().any(|func| {
                matches!(
                    self.generator
                        .opts
                        .import_call_style(self.qualifier().as_deref(), &func.name),
                    CallStyle::Concurrent
                )
            });

            if has_concurrent_function {
                uwriteln!(self.src, "type {camel}Data;");
            }

            for func in &functions {
                self.generate_function_trait_sig(func, &format!("{camel}Data"));
                self.push_str(";\n");
            }

            if let CallStyle::Async = self
                .generator
                .opts
                .drop_call_style(self.qualifier().as_deref(), name)
            {
                uwrite!(self.src, "async ");
            }
            uwrite!(
                self.src,
                "fn drop(&mut self, rep: {wt}::component::Resource<{camel}>) -> {wt}::Result<()>;"
            );

            uwriteln!(self.src, "}}");

            // Generate impl HostResource for &mut HostResource
            if !self.generator.opts.skip_mut_forwarding_impls {
                let maybe_send = if let CallStyle::Async = self.generator.opts.call_style() {
                    "+ Send"
                } else {
                    ""
                };
                let maybe_maybe_sized = if has_concurrent_function {
                    ""
                } else {
                    "+ ?Sized"
                };
                uwriteln!(
                    self.src,
                    "impl <_T: Host{camel} {maybe_maybe_sized} {maybe_send}> Host{camel} for &mut _T {{"
                );
                if has_concurrent_function {
                    uwriteln!(self.src, "type {camel}Data = _T::{camel}Data;");
                }
                for func in &functions {
                    let call_style = self
                        .generator
                        .opts
                        .import_call_style(self.qualifier().as_deref(), &func.name);
                    self.generate_function_trait_sig(func, &format!("{camel}Data"));
                    if let CallStyle::Concurrent = call_style {
                        uwrite!(
                            self.src,
                            "{{ <_T as Host{camel}>::{}(store,",
                            rust_function_name(func)
                        );
                    } else {
                        uwrite!(
                            self.src,
                            "{{ Host{camel}::{}(*self,",
                            rust_function_name(func)
                        );
                    }
                    for (name, _) in func.params.iter() {
                        uwrite!(self.src, "{},", to_rust_ident(name));
                    }
                    uwrite!(self.src, ")");
                    if let CallStyle::Async = call_style {
                        uwrite!(self.src, ".await");
                    }
                    uwriteln!(self.src, "}}");
                }
                if let CallStyle::Async = self
                    .generator
                    .opts
                    .drop_call_style(self.qualifier().as_deref(), name)
                {
                    uwriteln!(self.src, "
                        async fn drop(&mut self, rep: {wt}::component::Resource<{camel}>) -> {wt}::Result<()> {{
                            Host{camel}::drop(*self, rep).await
                        }}",
                    );
                } else {
                    uwriteln!(self.src, "
                        fn drop(&mut self, rep: {wt}::component::Resource<{camel}>) -> {wt}::Result<()> {{
                            Host{camel}::drop(*self, rep)
                        }}",
                    );
                }
                uwriteln!(self.src, "}}");
            }
        } else {
            self.rustdoc(docs);
            uwriteln!(
                self.src,
                "
                    pub type {camel} = {wt}::component::ResourceAny;

                    pub struct Guest{camel}<'a> {{
                        funcs: &'a Guest,
                    }}
                "
            );
        }
    }

    fn type_record(&mut self, id: TypeId, _name: &str, record: &Record, docs: &Docs) {
        let info = self.info(id);
        let wt = self.generator.wasmtime_path();

        // We use a BTree set to make sure we don't have any duplicates and we have a stable order
        let additional_derives: BTreeSet<String> = self
            .generator
            .opts
            .additional_derive_attributes
            .iter()
            .cloned()
            .collect();

        for (name, mode) in self.modes_of(id) {
            let lt = self.lifetime_for(&info, mode);
            self.rustdoc(docs);

            let mut derives = additional_derives.clone();

            uwriteln!(self.src, "#[derive({wt}::component::ComponentType)]");
            if lt.is_none() {
                uwriteln!(self.src, "#[derive({wt}::component::Lift)]");
            }
            uwriteln!(self.src, "#[derive({wt}::component::Lower)]");
            self.push_str("#[component(record)]\n");
            if let Some(path) = &self.generator.opts.wasmtime_crate {
                uwriteln!(self.src, "#[component(wasmtime_crate = {path})]\n");
            }

            if info.is_copy() {
                derives.extend(["Copy", "Clone"].into_iter().map(|s| s.to_string()));
            } else if info.is_clone() {
                derives.insert("Clone".to_string());
            }

            if !derives.is_empty() {
                self.push_str("#[derive(");
                self.push_str(&derives.into_iter().collect::<Vec<_>>().join(", "));
                self.push_str(")]\n")
            }

            self.push_str(&format!("pub struct {name}"));
            self.print_generics(lt);
            self.push_str(" {\n");
            for field in record.fields.iter() {
                self.rustdoc(&field.docs);
                self.push_str(&format!("#[component(name = \"{}\")]\n", field.name));
                self.push_str("pub ");
                self.push_str(&to_rust_ident(&field.name));
                self.push_str(": ");
                self.print_ty(&field.ty, mode);
                self.push_str(",\n");
            }
            self.push_str("}\n");

            self.push_str("impl");
            self.print_generics(lt);
            self.push_str(" core::fmt::Debug for ");
            self.push_str(&name);
            self.print_generics(lt);
            self.push_str(" {\n");
            self.push_str(
                "fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {\n",
            );
            self.push_str(&format!("f.debug_struct(\"{name}\")"));
            for field in record.fields.iter() {
                self.push_str(&format!(
                    ".field(\"{}\", &self.{})",
                    field.name,
                    to_rust_ident(&field.name)
                ));
            }
            self.push_str(".finish()\n");
            self.push_str("}\n");
            self.push_str("}\n");

            if info.error {
                self.push_str("impl");
                self.print_generics(lt);
                self.push_str(" core::fmt::Display for ");
                self.push_str(&name);
                self.print_generics(lt);
                self.push_str(" {\n");
                self.push_str(
                    "fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {\n",
                );
                self.push_str("write!(f, \"{:?}\", self)\n");
                self.push_str("}\n");
                self.push_str("}\n");

                self.push_str("impl core::error::Error for ");
                self.push_str(&name);
                self.push_str("{}\n");
            }
            self.assert_type(id, &name);
        }
    }

    fn type_tuple(&mut self, id: TypeId, _name: &str, tuple: &Tuple, docs: &Docs) {
        let info = self.info(id);
        for (name, mode) in self.modes_of(id) {
            let lt = self.lifetime_for(&info, mode);
            self.rustdoc(docs);
            self.push_str(&format!("pub type {name}"));
            self.print_generics(lt);
            self.push_str(" = (");
            for ty in tuple.types.iter() {
                self.print_ty(ty, mode);
                self.push_str(",");
            }
            self.push_str(");\n");
            self.assert_type(id, &name);
        }
    }

    fn type_flags(&mut self, id: TypeId, name: &str, flags: &Flags, docs: &Docs) {
        self.rustdoc(docs);
        let wt = self.generator.wasmtime_path();
        let rust_name = to_rust_upper_camel_case(name);
        uwriteln!(self.src, "{wt}::component::flags!(\n");
        self.src.push_str(&format!("{rust_name} {{\n"));
        for flag in flags.flags.iter() {
            // TODO wasmtime-component-macro doesn't support docs for flags rn
            uwrite!(
                self.src,
                "#[component(name=\"{}\")] const {};\n",
                flag.name,
                flag.name.to_shouty_snake_case()
            );
        }
        self.src.push_str("}\n");
        self.src.push_str(");\n\n");
        self.assert_type(id, &rust_name);
    }

    fn type_variant(&mut self, id: TypeId, _name: &str, variant: &Variant, docs: &Docs) {
        self.print_rust_enum(
            id,
            variant.cases.iter().map(|c| {
                (
                    c.name.to_upper_camel_case(),
                    Some(c.name.clone()),
                    &c.docs,
                    c.ty.as_ref(),
                )
            }),
            docs,
            "variant",
        );
    }

    fn type_option(&mut self, id: TypeId, _name: &str, payload: &Type, docs: &Docs) {
        let info = self.info(id);

        for (name, mode) in self.modes_of(id) {
            self.rustdoc(docs);
            let lt = self.lifetime_for(&info, mode);
            self.push_str(&format!("pub type {name}"));
            self.print_generics(lt);
            self.push_str("= Option<");
            self.print_ty(payload, mode);
            self.push_str(">;\n");
            self.assert_type(id, &name);
        }
    }

    // Emit a double-check that the wit-parser-understood size of a type agrees
    // with the Wasmtime-understood size of a type.
    fn assert_type(&mut self, id: TypeId, name: &str) {
        self.push_str("const _: () = {\n");
        let wt = self.generator.wasmtime_path();
        uwriteln!(
            self.src,
            "assert!({} == <{name} as {wt}::component::ComponentType>::SIZE32);",
            self.generator.sizes.size(&Type::Id(id)).size_wasm32(),
        );
        uwriteln!(
            self.src,
            "assert!({} == <{name} as {wt}::component::ComponentType>::ALIGN32);",
            self.generator.sizes.align(&Type::Id(id)).align_wasm32(),
        );
        self.push_str("};\n");
    }

    fn print_rust_enum<'b>(
        &mut self,
        id: TypeId,
        cases: impl IntoIterator<Item = (String, Option<String>, &'b Docs, Option<&'b Type>)> + Clone,
        docs: &Docs,
        derive_component: &str,
    ) where
        Self: Sized,
    {
        let info = self.info(id);
        let wt = self.generator.wasmtime_path();

        // We use a BTree set to make sure we don't have any duplicates and we have a stable order
        let additional_derives: BTreeSet<String> = self
            .generator
            .opts
            .additional_derive_attributes
            .iter()
            .cloned()
            .collect();

        for (name, mode) in self.modes_of(id) {
            let name = to_rust_upper_camel_case(&name);

            let mut derives = additional_derives.clone();

            self.rustdoc(docs);
            let lt = self.lifetime_for(&info, mode);
            uwriteln!(self.src, "#[derive({wt}::component::ComponentType)]");
            if lt.is_none() {
                uwriteln!(self.src, "#[derive({wt}::component::Lift)]");
            }
            uwriteln!(self.src, "#[derive({wt}::component::Lower)]");
            self.push_str(&format!("#[component({derive_component})]\n"));
            if let Some(path) = &self.generator.opts.wasmtime_crate {
                uwriteln!(self.src, "#[component(wasmtime_crate = {path})]\n");
            }
            if info.is_copy() {
                derives.extend(["Copy", "Clone"].into_iter().map(|s| s.to_string()));
            } else if info.is_clone() {
                derives.insert("Clone".to_string());
            }

            if !derives.is_empty() {
                self.push_str("#[derive(");
                self.push_str(&derives.into_iter().collect::<Vec<_>>().join(", "));
                self.push_str(")]\n")
            }

            self.push_str(&format!("pub enum {name}"));
            self.print_generics(lt);
            self.push_str("{\n");
            for (case_name, component_name, docs, payload) in cases.clone() {
                self.rustdoc(docs);
                if let Some(n) = component_name {
                    self.push_str(&format!("#[component(name = \"{n}\")] "));
                }
                self.push_str(&case_name);
                if let Some(ty) = payload {
                    self.push_str("(");
                    self.print_ty(ty, mode);
                    self.push_str(")")
                }
                self.push_str(",\n");
            }
            self.push_str("}\n");

            self.print_rust_enum_debug(
                id,
                mode,
                &name,
                cases
                    .clone()
                    .into_iter()
                    .map(|(name, _attr, _docs, ty)| (name, ty)),
            );

            if info.error {
                self.push_str("impl");
                self.print_generics(lt);
                self.push_str(" core::fmt::Display for ");
                self.push_str(&name);
                self.print_generics(lt);
                self.push_str(" {\n");
                self.push_str(
                    "fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {\n",
                );
                self.push_str("write!(f, \"{:?}\", self)\n");
                self.push_str("}\n");
                self.push_str("}\n");

                self.push_str("impl");
                self.print_generics(lt);
                self.push_str(" core::error::Error for ");
                self.push_str(&name);
                self.print_generics(lt);
                self.push_str(" {}\n");
            }

            self.assert_type(id, &name);
        }
    }

    fn print_rust_enum_debug<'b>(
        &mut self,
        id: TypeId,
        mode: TypeMode,
        name: &str,
        cases: impl IntoIterator<Item = (String, Option<&'b Type>)>,
    ) where
        Self: Sized,
    {
        let info = self.info(id);
        let lt = self.lifetime_for(&info, mode);
        self.push_str("impl");
        self.print_generics(lt);
        self.push_str(" core::fmt::Debug for ");
        self.push_str(name);
        self.print_generics(lt);
        self.push_str(" {\n");
        self.push_str("fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {\n");
        self.push_str("match self {\n");
        for (case_name, payload) in cases {
            self.push_str(name);
            self.push_str("::");
            self.push_str(&case_name);
            if payload.is_some() {
                self.push_str("(e)");
            }
            self.push_str(" => {\n");
            self.push_str(&format!("f.debug_tuple(\"{name}::{case_name}\")"));
            if payload.is_some() {
                self.push_str(".field(e)");
            }
            self.push_str(".finish()\n");
            self.push_str("}\n");
        }
        self.push_str("}\n");
        self.push_str("}\n");
        self.push_str("}\n");
    }

    fn type_result(&mut self, id: TypeId, _name: &str, result: &Result_, docs: &Docs) {
        let info = self.info(id);

        for (name, mode) in self.modes_of(id) {
            self.rustdoc(docs);
            let lt = self.lifetime_for(&info, mode);
            self.push_str(&format!("pub type {name}"));
            self.print_generics(lt);
            self.push_str("= Result<");
            self.print_optional_ty(result.ok.as_ref(), mode);
            self.push_str(",");
            self.print_optional_ty(result.err.as_ref(), mode);
            self.push_str(">;\n");
            self.assert_type(id, &name);
        }
    }

    fn type_enum(&mut self, id: TypeId, name: &str, enum_: &Enum, docs: &Docs) {
        let info = self.info(id);
        let wt = self.generator.wasmtime_path();

        // We use a BTree set to make sure we don't have any duplicates and have a stable order
        let mut derives: BTreeSet<String> = self
            .generator
            .opts
            .additional_derive_attributes
            .iter()
            .cloned()
            .collect();

        derives.extend(
            ["Clone", "Copy", "PartialEq", "Eq"]
                .into_iter()
                .map(|s| s.to_string()),
        );

        let name = to_rust_upper_camel_case(name);
        self.rustdoc(docs);
        uwriteln!(self.src, "#[derive({wt}::component::ComponentType)]");
        uwriteln!(self.src, "#[derive({wt}::component::Lift)]");
        uwriteln!(self.src, "#[derive({wt}::component::Lower)]");
        self.push_str("#[component(enum)]\n");
        if let Some(path) = &self.generator.opts.wasmtime_crate {
            uwriteln!(self.src, "#[component(wasmtime_crate = {path})]\n");
        }

        self.push_str("#[derive(");
        self.push_str(&derives.into_iter().collect::<Vec<_>>().join(", "));
        self.push_str(")]\n");

        let repr = match enum_.cases.len().ilog2() {
            0..=7 => "u8",
            8..=15 => "u16",
            _ => "u32",
        };
        uwriteln!(self.src, "#[repr({repr})]");

        self.push_str(&format!("pub enum {name} {{\n"));
        for case in enum_.cases.iter() {
            self.rustdoc(&case.docs);
            self.push_str(&format!("#[component(name = \"{}\")]", case.name));
            self.push_str(&case.name.to_upper_camel_case());
            self.push_str(",\n");
        }
        self.push_str("}\n");

        // Auto-synthesize an implementation of the standard `Error` trait for
        // error-looking types based on their name.
        if info.error {
            self.push_str("impl ");
            self.push_str(&name);
            self.push_str("{\n");

            self.push_str("pub fn name(&self) -> &'static str {\n");
            self.push_str("match self {\n");
            for case in enum_.cases.iter() {
                self.push_str(&name);
                self.push_str("::");
                self.push_str(&case.name.to_upper_camel_case());
                self.push_str(" => \"");
                self.push_str(case.name.as_str());
                self.push_str("\",\n");
            }
            self.push_str("}\n");
            self.push_str("}\n");

            self.push_str("pub fn message(&self) -> &'static str {\n");
            self.push_str("match self {\n");
            for case in enum_.cases.iter() {
                self.push_str(&name);
                self.push_str("::");
                self.push_str(&case.name.to_upper_camel_case());
                self.push_str(" => \"");
                if let Some(contents) = &case.docs.contents {
                    self.push_str(contents.trim());
                }
                self.push_str("\",\n");
            }
            self.push_str("}\n");
            self.push_str("}\n");

            self.push_str("}\n");

            self.push_str("impl core::fmt::Debug for ");
            self.push_str(&name);
            self.push_str(
                "{\nfn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {\n",
            );
            self.push_str("f.debug_struct(\"");
            self.push_str(&name);
            self.push_str("\")\n");
            self.push_str(".field(\"code\", &(*self as i32))\n");
            self.push_str(".field(\"name\", &self.name())\n");
            self.push_str(".field(\"message\", &self.message())\n");
            self.push_str(".finish()\n");
            self.push_str("}\n");
            self.push_str("}\n");

            self.push_str("impl core::fmt::Display for ");
            self.push_str(&name);
            self.push_str(
                "{\nfn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {\n",
            );
            self.push_str("write!(f, \"{} (error {})\", self.name(), *self as i32)");
            self.push_str("}\n");
            self.push_str("}\n");
            self.push_str("\n");
            self.push_str("impl core::error::Error for ");
            self.push_str(&name);
            self.push_str("{}\n");
        } else {
            self.print_rust_enum_debug(
                id,
                TypeMode::Owned,
                &name,
                enum_
                    .cases
                    .iter()
                    .map(|c| (c.name.to_upper_camel_case(), None)),
            )
        }
        self.assert_type(id, &name);
    }

    fn type_alias(&mut self, id: TypeId, _name: &str, ty: &Type, docs: &Docs) {
        let info = self.info(id);
        for (name, mode) in self.modes_of(id) {
            self.rustdoc(docs);
            self.push_str(&format!("pub type {name}"));
            let lt = self.lifetime_for(&info, mode);
            self.print_generics(lt);
            self.push_str(" = ");
            self.print_ty(ty, mode);
            self.push_str(";\n");
            let def_id = resolve_type_definition_id(self.resolve, id);
            if !matches!(self.resolve().types[def_id].kind, TypeDefKind::Resource) {
                self.assert_type(id, &name);
            }
        }
    }

    fn type_list(&mut self, id: TypeId, _name: &str, ty: &Type, docs: &Docs) {
        let info = self.info(id);
        for (name, mode) in self.modes_of(id) {
            let lt = self.lifetime_for(&info, mode);
            self.rustdoc(docs);
            self.push_str(&format!("pub type {name}"));
            self.print_generics(lt);
            self.push_str(" = ");
            self.print_list(ty, mode);
            self.push_str(";\n");
            self.assert_type(id, &name);
        }
    }

    fn type_stream(&mut self, id: TypeId, name: &str, ty: Option<&Type>, docs: &Docs) {
        self.rustdoc(docs);
        self.push_str(&format!("pub type {name}"));
        self.print_generics(None);
        self.push_str(" = ");
        self.print_stream(ty);
        self.push_str(";\n");
        self.assert_type(id, &name);
    }

    fn type_future(&mut self, id: TypeId, name: &str, ty: Option<&Type>, docs: &Docs) {
        self.rustdoc(docs);
        self.push_str(&format!("pub type {name}"));
        self.print_generics(None);
        self.push_str(" = ");
        self.print_future(ty);
        self.push_str(";\n");
        self.assert_type(id, &name);
    }

    fn type_error_context(&mut self, id: TypeId, name: &str, docs: &Docs) {
        self.rustdoc(docs);
        self.push_str(&format!("pub type {name}"));
        self.push_str(" = ");
        self.print_error_context();
        self.push_str(";\n");
        self.assert_type(id, &name);
    }

    fn print_result_ty(&mut self, results: &Results, mode: TypeMode) {
        match results {
            Results::Named(rs) => match rs.len() {
                0 => self.push_str("()"),
                1 => self.print_ty(&rs[0].1, mode),
                _ => {
                    self.push_str("(");
                    for (i, (_, ty)) in rs.iter().enumerate() {
                        if i > 0 {
                            self.push_str(", ")
                        }
                        self.print_ty(ty, mode)
                    }
                    self.push_str(")");
                }
            },
            Results::Anon(ty) => self.print_ty(ty, mode),
        }
    }

    fn print_result_ty_tuple(&mut self, results: &Results, mode: TypeMode) {
        self.push_str("(");
        match results {
            Results::Named(rs) if rs.is_empty() => self.push_str(")"),
            Results::Named(rs) => {
                for (_, ty) in rs {
                    self.print_ty(ty, mode);
                    self.push_str(", ");
                }
                self.push_str(")");
            }
            Results::Anon(ty) => {
                self.print_ty(ty, mode);
                self.push_str(",)");
            }
        }
    }

    fn special_case_trappable_error(
        &mut self,
        func: &Function,
    ) -> Option<(&'a Result_, TypeId, String)> {
        let results = &func.results;

        self.generator
            .used_trappable_imports_opts
            .insert(func.name.clone());

        // We fillin a special trappable error type in the case when a function has just one
        // result, which is itself a `result<a, e>`, and the `e` is *not* a primitive
        // (i.e. defined in std) type, and matches the typename given by the user.
        let mut i = results.iter_types();
        let id = match i.next()? {
            Type::Id(id) => id,
            _ => return None,
        };
        if i.next().is_some() {
            return None;
        }
        let result = match &self.resolve.types[*id].kind {
            TypeDefKind::Result(r) => r,
            _ => return None,
        };
        let error_typeid = match result.err? {
            Type::Id(id) => resolve_type_definition_id(&self.resolve, id),
            _ => return None,
        };

        let name = self.generator.trappable_errors.get(&error_typeid)?;

        let mut path = self.path_to_root();
        uwrite!(path, "{name}");
        Some((result, error_typeid, path))
    }

    fn generate_add_to_linker(&mut self, id: InterfaceId, name: &str) {
        let iface = &self.resolve.interfaces[id];
        let owner = TypeOwner::Interface(id);
        let wt = self.generator.wasmtime_path();

        let is_maybe_async = matches!(self.generator.opts.call_style(), CallStyle::Async);
        if is_maybe_async {
            uwriteln!(
                self.src,
                "#[{wt}::component::__internal::trait_variant_make(::core::marker::Send)]"
            )
        }
        // Generate the `pub trait` which represents the host functionality for
        // this import which additionally inherits from all resource traits
        // for this interface defined by `type_resource`.

        uwrite!(self.src, "pub trait Host");
        let mut host_supertraits = vec![];
        if is_maybe_async {
            host_supertraits.push("Send".to_string());
        }
        let mut saw_resources = false;
        for (_, name) in get_resources(self.resolve, id) {
            saw_resources = true;
            host_supertraits.push(format!("Host{}", name.to_upper_camel_case()));
        }
        if saw_resources {
            host_supertraits.push("Sized".to_string());
        }
        if !host_supertraits.is_empty() {
            uwrite!(self.src, ": {}", host_supertraits.join(" + "));
        }
        uwriteln!(self.src, " {{");

        let has_concurrent_function = iface.functions.iter().any(|(_, func)| {
            matches!(func.kind, FunctionKind::Freestanding)
                && matches!(
                    self.generator
                        .opts
                        .import_call_style(self.qualifier().as_deref(), &func.name),
                    CallStyle::Concurrent
                )
        });

        if has_concurrent_function {
            self.push_str("type Data;\n");
        }

        for (_, func) in iface.functions.iter() {
            match func.kind {
                FunctionKind::Freestanding => {}
                _ => continue,
            }
            self.generate_function_trait_sig(func, "Data");
            self.push_str(";\n");
        }

        // Generate `convert_*` functions to convert custom trappable errors
        // into the representation required by Wasmtime's component API.
        let mut required_conversion_traits = IndexSet::new();
        let mut errors_converted = IndexMap::new();
        let mut my_error_types = iface
            .types
            .iter()
            .filter(|(_, id)| self.generator.trappable_errors.contains_key(*id))
            .map(|(_, id)| *id)
            .collect::<Vec<_>>();
        my_error_types.extend(
            iface
                .functions
                .iter()
                .filter_map(|(_, func)| self.special_case_trappable_error(func))
                .map(|(_, id, _)| id),
        );
        let root = self.path_to_root();
        for err_id in my_error_types {
            let custom_name = &self.generator.trappable_errors[&err_id];
            let err = &self.resolve.types[resolve_type_definition_id(self.resolve, err_id)];
            let err_name = err.name.as_ref().unwrap();
            let err_snake = err_name.to_snake_case();
            let err_camel = err_name.to_upper_camel_case();
            let owner = match err.owner {
                TypeOwner::Interface(i) => i,
                _ => unimplemented!(),
            };
            match self.path_to_interface(owner) {
                Some(path) => {
                    required_conversion_traits.insert(format!("{path}::Host"));
                }
                None => {
                    if errors_converted.insert(err_name, err_id).is_none() {
                        uwriteln!(
                            self.src,
                            "fn convert_{err_snake}(&mut self, err: {root}{custom_name}) -> {wt}::Result<{err_camel}>;"
                        );
                    }
                }
            }
        }
        uwriteln!(self.src, "}}");

        let (data_bounds, mut host_bounds, mut get_host_bounds) =
            match self.generator.opts.call_style() {
                CallStyle::Async => (
                    "T: Send,".to_string(),
                    "Host + Send".to_string(),
                    "Host + Send".to_string(),
                ),
                CallStyle::Concurrent => {
                    let constraints = concurrent_constraints(
                        self.resolve,
                        &self.generator.opts,
                        self.qualifier().as_deref(),
                        id,
                    );

                    (
                        "T: Send + 'static,".to_string(),
                        format!("Host{} + Send", constraints("T")),
                        format!("Host{} + Send", constraints("D")),
                    )
                }
                CallStyle::Sync => (String::new(), "Host".to_string(), "Host".to_string()),
            };

        for ty in required_conversion_traits {
            uwrite!(host_bounds, " + {ty}");
            uwrite!(get_host_bounds, " + {ty}");
        }

        let (options_param, options_arg) = if self.generator.interface_link_options[&id].has_any() {
            ("options: &LinkOptions,", ", options")
        } else {
            ("", "")
        };

        uwriteln!(
            self.src,
            "
                pub trait GetHost<T, D>:
                    Fn(T) -> <Self as GetHost<T, D>>::Host
                        + Send
                        + Sync
                        + Copy
                        + 'static
                {{
                    type Host: {get_host_bounds};
                }}

                impl<F, T, D, O> GetHost<T, D> for F
                where
                    F: Fn(T) -> O + Send + Sync + Copy + 'static,
                    O: {get_host_bounds},
                {{
                    type Host = O;
                }}

                pub fn add_to_linker_get_host<T, G: for<'a> GetHost<&'a mut T, T, Host: {host_bounds}>>(
                    linker: &mut {wt}::component::Linker<T>,
                    {options_param}
                    host_getter: G,
                ) -> {wt}::Result<()>
                    where {data_bounds}
                {{
            "
        );
        let gate = FeatureGate::open(&mut self.src, &iface.stability);
        uwriteln!(self.src, "let mut inst = linker.instance(\"{name}\")?;");

        for (ty, name) in get_resources(self.resolve, id) {
            Wasmtime::generate_add_resource_to_linker(
                self.qualifier().as_deref(),
                &mut self.src,
                &self.generator.opts,
                &wt,
                "inst",
                name,
                &self.resolve.types[ty].stability,
            );
        }

        for (_, func) in iface.functions.iter() {
            self.generate_add_function_to_linker(owner, func, "inst");
        }
        gate.close(&mut self.src);
        uwriteln!(self.src, "Ok(())");
        uwriteln!(self.src, "}}");

        if !self.generator.opts.skip_mut_forwarding_impls {
            // Generate add_to_linker (with closure)
            uwriteln!(
                self.src,
                "
                pub fn add_to_linker<T, U>(
                    linker: &mut {wt}::component::Linker<T>,
                    {options_param}
                    get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
                ) -> {wt}::Result<()>
                    where
                        U: {host_bounds}, {data_bounds}
                {{
                    add_to_linker_get_host(linker {options_arg}, get)
                }}
                "
            );

            // Generate impl Host for &mut Host
            let maybe_send = if is_maybe_async { "+ Send" } else { "" };

            let maybe_maybe_sized = if has_concurrent_function {
                ""
            } else {
                "+ ?Sized"
            };

            uwriteln!(
                self.src,
                "impl<_T: Host {maybe_maybe_sized} {maybe_send}> Host for &mut _T {{"
            );

            if has_concurrent_function {
                self.push_str("type Data = _T::Data;\n");
            }

            // Forward each method call to &mut T
            for (_, func) in iface.functions.iter() {
                match func.kind {
                    FunctionKind::Freestanding => {}
                    _ => continue,
                }
                let call_style = self
                    .generator
                    .opts
                    .import_call_style(self.qualifier().as_deref(), &func.name);
                self.generate_function_trait_sig(func, "Data");
                if let CallStyle::Concurrent = call_style {
                    uwrite!(
                        self.src,
                        "{{ <_T as Host>::{}(store,",
                        rust_function_name(func)
                    );
                } else {
                    uwrite!(self.src, "{{ Host::{}(*self,", rust_function_name(func));
                }
                for (name, _) in func.params.iter() {
                    uwrite!(self.src, "{},", to_rust_ident(name));
                }
                uwrite!(self.src, ")");
                if let CallStyle::Async = call_style {
                    uwrite!(self.src, ".await");
                }
                uwriteln!(self.src, "}}");
            }
            for (err_name, err_id) in errors_converted {
                uwriteln!(
                    self.src,
                    "fn convert_{err_snake}(&mut self, err: {root}{custom_name}) -> {wt}::Result<{err_camel}> {{
                        Host::convert_{err_snake}(*self, err)
                    }}",
                    custom_name = self.generator.trappable_errors[&err_id],
                    err_snake = err_name.to_snake_case(),
                    err_camel = err_name.to_upper_camel_case(),
                );
            }
            uwriteln!(self.src, "}}");
        }
    }

    fn qualifier(&self) -> Option<String> {
        self.current_interface
            .map(|(_, key, _)| self.resolve.name_world_key(key))
    }

    fn generate_add_function_to_linker(&mut self, owner: TypeOwner, func: &Function, linker: &str) {
        let gate = FeatureGate::open(&mut self.src, &func.stability);
        uwrite!(
            self.src,
            "{linker}.{}(\"{}\", ",
            match self
                .generator
                .opts
                .import_call_style(self.qualifier().as_deref(), &func.name)
            {
                CallStyle::Sync => "func_wrap",
                CallStyle::Async => "func_wrap_async",
                CallStyle::Concurrent => "func_wrap_concurrent",
            },
            func.name
        );
        self.generate_guest_import_closure(owner, func);
        uwriteln!(self.src, ")?;");
        gate.close(&mut self.src);
    }

    fn generate_guest_import_closure(&mut self, owner: TypeOwner, func: &Function) {
        // Generate the closure that's passed to a `Linker`, the final piece of
        // codegen here.

        let wt = self.generator.wasmtime_path();
        uwrite!(
            self.src,
            "move |mut caller: {wt}::StoreContextMut<'_, T>, ("
        );
        for (i, _param) in func.params.iter().enumerate() {
            uwrite!(self.src, "arg{},", i);
        }
        self.src.push_str(") : (");

        for (_, ty) in func.params.iter() {
            // Lift is required to be implied for this type, so we can't use
            // a borrowed type:
            self.print_ty(ty, TypeMode::Owned);
            self.src.push_str(", ");
        }
        self.src.push_str(")| {\n");

        let style = self
            .generator
            .opts
            .import_call_style(self.qualifier().as_deref(), &func.name);

        if self.generator.opts.tracing {
            if let CallStyle::Async = style {
                self.src.push_str("use tracing::Instrument;\n");
            }

            uwrite!(
                self.src,
                "
                   let span = tracing::span!(
                       tracing::Level::TRACE,
                       \"wit-bindgen import\",
                       module = \"{}\",
                       function = \"{}\",
                   );
               ",
                match owner {
                    TypeOwner::Interface(id) => self.resolve.interfaces[id]
                        .name
                        .as_deref()
                        .unwrap_or("<no module>"),
                    TypeOwner::World(id) => &self.resolve.worlds[id].name,
                    TypeOwner::None => "<no owner>",
                },
                func.name,
            );
        }

        if let CallStyle::Async = &style {
            uwriteln!(
                self.src,
                " {wt}::component::__internal::Box::new(async move {{ "
            );
        } else {
            // Only directly enter the span if the function is sync. Otherwise
            // we use tracing::Instrument to ensure that the span is not entered
            // across an await point.
            if self.generator.opts.tracing {
                self.push_str("let _enter = span.enter();\n");
            }
        }

        if self.generator.opts.tracing {
            let mut event_fields = func
                .params
                .iter()
                .enumerate()
                .map(|(i, (name, ty))| {
                    let name = to_rust_ident(&name);
                    formatting_for_arg(&name, i, *ty, &self.generator.opts, &self.resolve)
                })
                .collect::<Vec<String>>();
            event_fields.push(format!("\"call\""));
            uwrite!(
                self.src,
                "tracing::event!(tracing::Level::TRACE, {});\n",
                event_fields.join(", ")
            );
        }

        self.src.push_str(if let CallStyle::Concurrent = &style {
            "let host = caller;\n"
        } else {
            "let host = &mut host_getter(caller.data_mut());\n"
        });
        let func_name = rust_function_name(func);
        let host_trait = match func.kind {
            FunctionKind::Freestanding => match owner {
                TypeOwner::World(id) => format!(
                    "{}Imports",
                    rust::to_rust_upper_camel_case(&self.resolve.worlds[id].name)
                ),
                _ => "Host".to_string(),
            },
            FunctionKind::Method(id) | FunctionKind::Static(id) | FunctionKind::Constructor(id) => {
                let resource = self.resolve.types[id]
                    .name
                    .as_ref()
                    .unwrap()
                    .to_upper_camel_case();
                format!("Host{resource}")
            }
        };

        if let CallStyle::Concurrent = &style {
            uwrite!(
                self.src,
                "let r = <G::Host as {host_trait}>::{func_name}(host, "
            );
        } else {
            uwrite!(self.src, "let r = {host_trait}::{func_name}(host, ");
        }

        for (i, _) in func.params.iter().enumerate() {
            uwrite!(self.src, "arg{},", i);
        }

        self.src.push_str(match &style {
            CallStyle::Sync | CallStyle::Concurrent => ");\n",
            CallStyle::Async => ").await;\n",
        });

        if let CallStyle::Concurrent = &style {
            self.src.push_str(
                "Box::pin(async move {
                     let fun = r.await;
                     Box::new(move |mut caller: wasmtime::StoreContextMut<'_, T>| {
                         let r = fun(caller);
                ",
            );
        }

        if self.generator.opts.tracing {
            uwrite!(
                self.src,
                "tracing::event!(tracing::Level::TRACE, {}, \"return\");",
                formatting_for_results(&func.results, &self.generator.opts, &self.resolve)
            );
        }

        if !self.generator.opts.trappable_imports.can_trap(&func) {
            if func.results.iter_types().len() == 1 {
                uwrite!(self.src, "Ok((r,))\n");
            } else {
                uwrite!(self.src, "Ok(r)\n");
            }
        } else if let Some((_, err, _)) = self.special_case_trappable_error(func) {
            let err = &self.resolve.types[resolve_type_definition_id(self.resolve, err)];
            let err_name = err.name.as_ref().unwrap();
            let owner = match err.owner {
                TypeOwner::Interface(i) => i,
                _ => unimplemented!(),
            };
            let convert_trait = match self.path_to_interface(owner) {
                Some(path) => format!("{path}::Host"),
                None => format!("Host"),
            };
            let convert = format!("{}::convert_{}", convert_trait, err_name.to_snake_case());
            uwrite!(
                self.src,
                "Ok((match r {{
                    Ok(a) => Ok(a),
                    Err(e) => Err({convert}(host, e)?),
                }},))"
            );
        } else if func.results.iter_types().len() == 1 {
            uwrite!(self.src, "Ok((r?,))\n");
        } else {
            uwrite!(self.src, "r\n");
        }

        match &style {
            CallStyle::Sync => (),
            CallStyle::Async => {
                if self.generator.opts.tracing {
                    self.src.push_str("}.instrument(span))\n");
                } else {
                    self.src.push_str("})\n");
                }
            }
            CallStyle::Concurrent => {
                let old_source = mem::take(&mut self.src);
                self.print_result_ty_tuple(&func.results, TypeMode::Owned);
                let result_type = String::from(mem::replace(&mut self.src, old_source));
                let box_fn = format!(
                    "Box<dyn FnOnce(wasmtime::StoreContextMut<'_, T>) -> \
                     wasmtime::Result<{result_type}> + Send + Sync>"
                );
                uwriteln!(
                    self.src,
                    "        }}) as {box_fn}
                         }}) as ::core::pin::Pin<Box<dyn ::core::future::Future<Output = {box_fn}> \
                               + Send + Sync + 'static>>
                    "
                );
            }
        }
        self.src.push_str("}\n");
    }

    fn generate_function_trait_sig(&mut self, func: &Function, data: &str) {
        let wt = self.generator.wasmtime_path();
        self.rustdoc(&func.docs);

        let style = self
            .generator
            .opts
            .import_call_style(self.qualifier().as_deref(), &func.name);
        if let CallStyle::Async = &style {
            self.push_str("async ");
        }
        self.push_str("fn ");
        self.push_str(&rust_function_name(func));
        self.push_str(&if let CallStyle::Concurrent = &style {
            format!("(store: wasmtime::StoreContextMut<'_, Self::{data}>, ")
        } else {
            "(&mut self, ".to_string()
        });
        for (name, param) in func.params.iter() {
            let name = to_rust_ident(name);
            self.push_str(&name);
            self.push_str(": ");
            self.print_ty(param, TypeMode::Owned);
            self.push_str(",");
        }
        self.push_str(")");
        self.push_str(" -> ");

        if let CallStyle::Concurrent = &style {
            uwrite!(self.src, "impl ::core::future::Future<Output = impl FnOnce(wasmtime::StoreContextMut<'_, Self::{data}>) -> ");
        }

        if !self.generator.opts.trappable_imports.can_trap(func) {
            self.print_result_ty(&func.results, TypeMode::Owned);
        } else if let Some((r, _id, error_typename)) = self.special_case_trappable_error(func) {
            // Functions which have a single result `result<ok,err>` get special
            // cased to use the host_wasmtime_rust::Error<err>, making it possible
            // for them to trap or use `?` to propagate their errors
            self.push_str("Result<");
            if let Some(ok) = r.ok {
                self.print_ty(&ok, TypeMode::Owned);
            } else {
                self.push_str("()");
            }
            self.push_str(",");
            self.push_str(&error_typename);
            self.push_str(">");
        } else {
            // All other functions get their return values wrapped in an wasmtime::Result.
            // Returning the anyhow::Error case can be used to trap.
            uwrite!(self.src, "{wt}::Result<");
            self.print_result_ty(&func.results, TypeMode::Owned);
            self.push_str(">");
        }

        if let CallStyle::Concurrent = &style {
            self.push_str(" + Send + Sync + 'static> + Send + Sync + 'static where Self: Sized");
        }
    }

    fn extract_typed_function(&mut self, func: &Function) -> (String, String) {
        let prev = mem::take(&mut self.src);
        let snake = func_field_name(self.resolve, func);
        uwrite!(self.src, "*_instance.get_typed_func::<(");
        for (_, ty) in func.params.iter() {
            self.print_ty(ty, TypeMode::AllBorrowed("'_"));
            self.push_str(", ");
        }
        self.src.push_str("), (");
        for ty in func.results.iter_types() {
            self.print_ty(ty, TypeMode::Owned);
            self.push_str(", ");
        }
        uwriteln!(self.src, ")>(&mut store, &self.{snake})?.func()");

        let ret = (snake, mem::take(&mut self.src).to_string());
        self.src = prev;
        ret
    }

    fn define_rust_guest_export(
        &mut self,
        resolve: &Resolve,
        ns: Option<&WorldKey>,
        func: &Function,
    ) {
        // Exports must be async if anything could be async, it's just imports
        // that get to be optionally async/sync.
        let style = self.generator.opts.call_style();
        let (async_, async__, await_, concurrent) = match &style {
            CallStyle::Async | CallStyle::Concurrent => {
                if self.generator.opts.concurrent_exports {
                    ("async", "INVALID", "INVALID", true)
                } else {
                    ("async", "_async", ".await", false)
                }
            }
            CallStyle::Sync => ("", "", "", false),
        };

        self.rustdoc(&func.docs);
        let wt = self.generator.wasmtime_path();

        uwrite!(
            self.src,
            "pub {async_} fn call_{}<S: {wt}::AsContextMut>(&self, mut store: S, ",
            func.item_name().to_snake_case(),
        );

        let param_mode = if let CallStyle::Concurrent = &style {
            TypeMode::Owned
        } else {
            TypeMode::AllBorrowed("'_")
        };

        for (i, param) in func.params.iter().enumerate() {
            uwrite!(self.src, "arg{}: ", i);
            self.print_ty(&param.1, param_mode);
            self.push_str(",");
        }

        uwrite!(self.src, ") -> {wt}::Result<");
        if concurrent {
            uwrite!(self.src, "{wt}::component::Promise<");
        }
        self.print_result_ty(&func.results, TypeMode::Owned);
        if concurrent {
            uwrite!(self.src, ">");
        }

        let maybe_static = if concurrent { " + 'static" } else { "" };

        uwrite!(
            self.src,
            "> where <S as {wt}::AsContext>::Data: Send{maybe_static} {{\n"
        );

        // TODO: support tracing concurrent calls
        if self.generator.opts.tracing && !concurrent {
            if let CallStyle::Async = &style {
                self.src.push_str("use tracing::Instrument;\n");
            }

            let ns = match ns {
                Some(key) => resolve.name_world_key(key),
                None => "default".to_string(),
            };
            self.src.push_str(&format!(
                "
                   let span = tracing::span!(
                       tracing::Level::TRACE,
                       \"wit-bindgen export\",
                       module = \"{ns}\",
                       function = \"{}\",
                   );
               ",
                func.name,
            ));

            if !matches!(&style, CallStyle::Async) {
                self.src.push_str(
                    "
                   let _enter = span.enter();
                   ",
                );
            }
        }

        self.src.push_str("let callee = unsafe {\n");
        uwrite!(self.src, "{wt}::component::TypedFunc::<(");
        for (_, ty) in func.params.iter() {
            self.print_ty(ty, param_mode);
            self.push_str(", ");
        }
        self.src.push_str("), (");
        for ty in func.results.iter_types() {
            self.print_ty(ty, TypeMode::Owned);
            self.push_str(", ");
        }
        let projection_to_func = match &func.kind {
            FunctionKind::Freestanding => "",
            _ => ".funcs",
        };
        uwriteln!(
            self.src,
            ")>::new_unchecked(self{projection_to_func}.{})",
            func_field_name(self.resolve, func),
        );
        self.src.push_str("};\n");

        if concurrent {
            uwrite!(
                self.src,
                "let promise = callee.call_concurrent(store.as_context_mut(), ("
            );
            for (i, _) in func.params.iter().enumerate() {
                uwrite!(self.src, "arg{i}, ");
            }
            self.src.push_str(")).await?;");

            if func.results.iter_types().len() == 1 {
                self.src.push_str("Ok(promise.map(|(v,)| v))\n");
            } else {
                self.src.push_str("Ok(promise)");
            }
        } else {
            self.src.push_str("let (");
            for (i, _) in func.results.iter_types().enumerate() {
                uwrite!(self.src, "ret{},", i);
            }
            uwrite!(
                self.src,
                ") = callee.call{async__}(store.as_context_mut(), ("
            );
            for (i, _) in func.params.iter().enumerate() {
                uwrite!(self.src, "arg{}, ", i);
            }

            let instrument = if matches!(&style, CallStyle::Async) && self.generator.opts.tracing {
                ".instrument(span.clone())"
            } else {
                ""
            };
            uwriteln!(self.src, ")){instrument}{await_}?;");

            let instrument = if matches!(&style, CallStyle::Async) && self.generator.opts.tracing {
                ".instrument(span)"
            } else {
                ""
            };

            uwriteln!(
                self.src,
                "callee.post_return{async__}(store.as_context_mut()){instrument}{await_}?;"
            );

            self.src.push_str("Ok(");
            if func.results.iter_types().len() == 1 {
                self.src.push_str("ret0");
            } else {
                self.src.push_str("(");
                for (i, _) in func.results.iter_types().enumerate() {
                    uwrite!(self.src, "ret{},", i);
                }
                self.src.push_str(")");
            }
            self.src.push_str(")\n");
        }

        // End function body
        self.src.push_str("}\n");
    }

    fn rustdoc(&mut self, docs: &Docs) {
        let docs = match &docs.contents {
            Some(docs) => docs,
            None => return,
        };
        for line in docs.trim().lines() {
            self.push_str("/// ");
            self.push_str(line);
            self.push_str("\n");
        }
    }

    fn path_to_root(&self) -> String {
        let mut path_to_root = String::new();
        if let Some((_, key, is_export)) = self.current_interface {
            match key {
                WorldKey::Name(_) => {
                    path_to_root.push_str("super::");
                }
                WorldKey::Interface(_) => {
                    path_to_root.push_str("super::super::super::");
                }
            }
            if is_export {
                path_to_root.push_str("super::");
            }
        }
        path_to_root
    }
}

impl<'a> RustGenerator<'a> for InterfaceGenerator<'a> {
    fn resolve(&self) -> &'a Resolve {
        self.resolve
    }

    fn ownership(&self) -> Ownership {
        self.generator.opts.ownership
    }

    fn path_to_interface(&self, interface: InterfaceId) -> Option<String> {
        if let Some((cur, _, _)) = self.current_interface {
            if cur == interface {
                return None;
            }
        }
        let mut path_to_root = self.path_to_root();
        match &self.generator.interface_names[&interface] {
            InterfaceName::Remapped { name_at_root, .. } => path_to_root.push_str(name_at_root),
            InterfaceName::Path(path) => {
                for (i, name) in path.iter().enumerate() {
                    if i > 0 {
                        path_to_root.push_str("::");
                    }
                    path_to_root.push_str(name);
                }
            }
        }
        Some(path_to_root)
    }

    fn push_str(&mut self, s: &str) {
        self.src.push_str(s);
    }

    fn info(&self, ty: TypeId) -> TypeInfo {
        self.generator.types.get(ty)
    }

    fn is_imported_interface(&self, interface: InterfaceId) -> bool {
        self.generator.interface_last_seen_as_import[&interface]
    }

    fn wasmtime_path(&self) -> String {
        self.generator.wasmtime_path()
    }
}

#[derive(Default)]
struct LinkOptionsBuilder {
    unstable_features: BTreeSet<String>,
}
impl LinkOptionsBuilder {
    fn has_any(&self) -> bool {
        !self.unstable_features.is_empty()
    }
    fn add_world(&mut self, resolve: &Resolve, id: &WorldId) {
        let world = &resolve.worlds[*id];

        self.add_stability(&world.stability);

        for (_, import) in world.imports.iter() {
            match import {
                WorldItem::Interface { id, stability } => {
                    self.add_stability(stability);
                    self.add_interface(resolve, id);
                }
                WorldItem::Function(f) => {
                    self.add_stability(&f.stability);
                }
                WorldItem::Type(t) => {
                    self.add_type(resolve, t);
                }
            }
        }
    }
    fn add_interface(&mut self, resolve: &Resolve, id: &InterfaceId) {
        let interface = &resolve.interfaces[*id];

        self.add_stability(&interface.stability);

        for (_, t) in interface.types.iter() {
            self.add_type(resolve, t);
        }
        for (_, f) in interface.functions.iter() {
            self.add_stability(&f.stability);
        }
    }
    fn add_type(&mut self, resolve: &Resolve, id: &TypeId) {
        let t = &resolve.types[*id];
        self.add_stability(&t.stability);
    }
    fn add_stability(&mut self, stability: &Stability) {
        match stability {
            Stability::Unstable { feature, .. } => {
                self.unstable_features.insert(feature.clone());
            }
            Stability::Stable { .. } | Stability::Unknown => {}
        }
    }
    fn write_struct(&self, src: &mut Source) {
        if !self.has_any() {
            return;
        }

        let mut unstable_features = self.unstable_features.iter().cloned().collect::<Vec<_>>();
        unstable_features.sort();

        uwriteln!(
            src,
            "
            /// Link-time configurations.
            #[derive(Clone, Debug, Default)]
            pub struct LinkOptions {{
            "
        );

        for feature in unstable_features.iter() {
            let feature_rust_name = feature.to_snake_case();
            uwriteln!(src, "{feature_rust_name}: bool,");
        }

        uwriteln!(src, "}}");
        uwriteln!(src, "impl LinkOptions {{");

        for feature in unstable_features.iter() {
            let feature_rust_name = feature.to_snake_case();
            uwriteln!(
                src,
                "
                /// Enable members marked as `@unstable(feature = {feature})`
                pub fn {feature_rust_name}(&mut self, enabled: bool) -> &mut Self {{
                    self.{feature_rust_name} = enabled;
                    self
                }}
            "
            );
        }

        uwriteln!(src, "}}");
    }
    fn write_impl_from_world(&self, src: &mut Source, path: &str) {
        if !self.has_any() {
            return;
        }

        let mut unstable_features = self.unstable_features.iter().cloned().collect::<Vec<_>>();
        unstable_features.sort();

        uwriteln!(
            src,
            "
            impl core::convert::From<LinkOptions> for {path}::LinkOptions {{
                fn from(src: LinkOptions) -> Self {{
                    (&src).into()
                }}
            }}

            impl core::convert::From<&LinkOptions> for {path}::LinkOptions {{
                fn from(src: &LinkOptions) -> Self {{
                    let mut dest = Self::default();
        "
        );

        for feature in unstable_features.iter() {
            let feature_rust_name = feature.to_snake_case();
            uwriteln!(src, "dest.{feature_rust_name}(src.{feature_rust_name});");
        }

        uwriteln!(
            src,
            "
                    dest
                }}
            }}
        "
        );
    }
}

struct FeatureGate {
    close: bool,
}
impl FeatureGate {
    fn open(src: &mut Source, stability: &Stability) -> FeatureGate {
        let close = if let Stability::Unstable { feature, .. } = stability {
            let feature_rust_name = feature.to_snake_case();
            uwrite!(src, "if options.{feature_rust_name} {{");
            true
        } else {
            false
        };
        Self { close }
    }

    fn close(self, src: &mut Source) {
        if self.close {
            uwriteln!(src, "}}");
        }
    }
}

/// Produce a string for tracing a function argument.
fn formatting_for_arg(
    name: &str,
    index: usize,
    ty: Type,
    opts: &Opts,
    resolve: &Resolve,
) -> String {
    if !opts.verbose_tracing && type_contains_lists(ty, resolve) {
        return format!("{name} = tracing::field::debug(\"...\")");
    }

    // Normal tracing.
    format!("{name} = tracing::field::debug(&arg{index})")
}

/// Produce a string for tracing function results.
fn formatting_for_results(results: &Results, opts: &Opts, resolve: &Resolve) -> String {
    let contains_lists = match results {
        Results::Anon(ty) => type_contains_lists(*ty, resolve),
        Results::Named(params) => params
            .iter()
            .any(|(_, ty)| type_contains_lists(*ty, resolve)),
    };

    if !opts.verbose_tracing && contains_lists {
        return format!("result = tracing::field::debug(\"...\")");
    }

    // Normal tracing.
    format!("result = tracing::field::debug(&r)")
}

/// Test whether the given type contains lists.
///
/// Here, a `string` is not considered a list.
fn type_contains_lists(ty: Type, resolve: &Resolve) -> bool {
    match ty {
        Type::Id(id) => match &resolve.types[id].kind {
            TypeDefKind::Resource
            | TypeDefKind::Unknown
            | TypeDefKind::Flags(_)
            | TypeDefKind::Handle(_)
            | TypeDefKind::Enum(_)
            | TypeDefKind::Stream(_)
            | TypeDefKind::Future(_)
            | TypeDefKind::ErrorContext => false,
            TypeDefKind::Option(ty) => type_contains_lists(*ty, resolve),
            TypeDefKind::Result(Result_ { ok, err }) => {
                option_type_contains_lists(*ok, resolve)
                    || option_type_contains_lists(*err, resolve)
            }
            TypeDefKind::Record(record) => record
                .fields
                .iter()
                .any(|field| type_contains_lists(field.ty, resolve)),
            TypeDefKind::Tuple(tuple) => tuple
                .types
                .iter()
                .any(|ty| type_contains_lists(*ty, resolve)),
            TypeDefKind::Variant(variant) => variant
                .cases
                .iter()
                .any(|case| option_type_contains_lists(case.ty, resolve)),
            TypeDefKind::Type(ty) => type_contains_lists(*ty, resolve),
            TypeDefKind::List(_) => true,
        },

        // Technically strings are lists too, but we ignore that here because
        // they're usually short.
        _ => false,
    }
}

fn option_type_contains_lists(ty: Option<Type>, resolve: &Resolve) -> bool {
    match ty {
        Some(ty) => type_contains_lists(ty, resolve),
        None => false,
    }
}

/// When an interface `use`s a type from another interface, it creates a new TypeId
/// referring to the definition TypeId. Chase this chain of references down to
/// a TypeId for type's definition.
fn resolve_type_definition_id(resolve: &Resolve, mut id: TypeId) -> TypeId {
    loop {
        match resolve.types[id].kind {
            TypeDefKind::Type(Type::Id(def_id)) => id = def_id,
            _ => return id,
        }
    }
}

fn rust_function_name(func: &Function) -> String {
    match func.kind {
        FunctionKind::Method(_) | FunctionKind::Static(_) => to_rust_ident(func.item_name()),
        FunctionKind::Constructor(_) => "new".to_string(),
        FunctionKind::Freestanding => to_rust_ident(&func.name),
    }
}

fn func_field_name(resolve: &Resolve, func: &Function) -> String {
    let mut name = String::new();
    match func.kind {
        FunctionKind::Method(id) => {
            name.push_str("method-");
            name.push_str(resolve.types[id].name.as_ref().unwrap());
            name.push_str("-");
        }
        FunctionKind::Static(id) => {
            name.push_str("static-");
            name.push_str(resolve.types[id].name.as_ref().unwrap());
            name.push_str("-");
        }
        FunctionKind::Constructor(id) => {
            name.push_str("constructor-");
            name.push_str(resolve.types[id].name.as_ref().unwrap());
            name.push_str("-");
        }
        FunctionKind::Freestanding => {}
    }
    name.push_str(func.item_name());
    name.to_snake_case()
}

fn get_resources<'a>(
    resolve: &'a Resolve,
    id: InterfaceId,
) -> impl Iterator<Item = (TypeId, &'a str)> + 'a {
    resolve.interfaces[id]
        .types
        .iter()
        .filter_map(move |(name, ty)| match &resolve.types[*ty].kind {
            TypeDefKind::Resource => Some((*ty, name.as_str())),
            _ => None,
        })
}

fn get_world_resources<'a>(
    resolve: &'a Resolve,
    id: WorldId,
) -> impl Iterator<Item = (TypeId, &'a str)> + 'a {
    resolve.worlds[id]
        .imports
        .iter()
        .filter_map(move |(name, item)| match item {
            WorldItem::Type(id) => match resolve.types[*id].kind {
                TypeDefKind::Resource => Some(match name {
                    WorldKey::Name(s) => (*id, s.as_str()),
                    WorldKey::Interface(_) => unreachable!(),
                }),
                _ => None,
            },
            _ => None,
        })
}

fn concurrent_constraints<'a>(
    resolve: &'a Resolve,
    opts: &Opts,
    qualifier: Option<&str>,
    id: InterfaceId,
) -> impl Fn(&str) -> String + use<'a> {
    let has_concurrent_function = resolve.interfaces[id].functions.iter().any(|(_, func)| {
        matches!(func.kind, FunctionKind::Freestanding)
            && matches!(
                opts.import_call_style(qualifier, &func.name),
                CallStyle::Concurrent
            )
    });

    let types = resolve.interfaces[id]
        .types
        .iter()
        .filter_map(|(name, ty)| match resolve.types[*ty].kind {
            TypeDefKind::Resource
                if resolve.interfaces[id]
                    .functions
                    .values()
                    .any(|func| match func.kind {
                        FunctionKind::Freestanding => false,
                        FunctionKind::Method(resource)
                        | FunctionKind::Static(resource)
                        | FunctionKind::Constructor(resource) => {
                            *ty == resource
                                && matches!(
                                    opts.import_call_style(qualifier, &func.name),
                                    CallStyle::Concurrent
                                )
                        }
                    }) =>
            {
                Some(format!("{}Data", name.to_upper_camel_case()))
            }
            _ => None,
        })
        .chain(has_concurrent_function.then_some("Data".to_string()))
        .collect::<Vec<_>>();

    move |v| {
        if types.is_empty() {
            String::new()
        } else {
            format!(
                "<{}>",
                types
                    .iter()
                    .map(|s| format!("{s} = {v}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }
}

fn world_imports_concurrent_constraints<'a>(
    resolve: &'a Resolve,
    world: WorldId,
    opts: &Opts,
) -> impl Fn(&str) -> String + use<'a> {
    let has_concurrent_function = resolve.worlds[world]
        .imports
        .values()
        .any(|item| match item {
            WorldItem::Function(func) => {
                matches!(func.kind, FunctionKind::Freestanding)
                    && matches!(
                        opts.import_call_style(None, &func.name),
                        CallStyle::Concurrent
                    )
            }
            WorldItem::Interface { .. } | WorldItem::Type(_) => false,
        });

    let types = resolve.worlds[world]
        .imports
        .iter()
        .filter_map(|(name, item)| match (name, item) {
            (WorldKey::Name(name), WorldItem::Type(ty)) => match resolve.types[*ty].kind {
                TypeDefKind::Resource
                    if resolve.worlds[world]
                        .imports
                        .values()
                        .any(|item| match item {
                            WorldItem::Function(func) => match func.kind {
                                FunctionKind::Freestanding => false,
                                FunctionKind::Method(resource)
                                | FunctionKind::Static(resource)
                                | FunctionKind::Constructor(resource) => {
                                    *ty == resource
                                        && matches!(
                                            opts.import_call_style(None, &func.name),
                                            CallStyle::Concurrent
                                        )
                                }
                            },
                            WorldItem::Interface { .. } | WorldItem::Type(_) => false,
                        }) =>
                {
                    Some(format!("{}Data", name.to_upper_camel_case()))
                }
                _ => None,
            },
            _ => None,
        })
        .chain(has_concurrent_function.then_some("Data".to_string()))
        .collect::<Vec<_>>();

    move |v| {
        if types.is_empty() {
            String::new()
        } else {
            format!(
                "<{}>",
                types
                    .iter()
                    .map(|s| format!("{s} = {v}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }
}
