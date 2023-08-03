use crate::rust::{to_rust_ident, to_rust_upper_camel_case, RustGenerator, TypeMode};
use crate::types::{TypeInfo, Types};
use anyhow::{anyhow, bail, Context};
use heck::*;
use indexmap::IndexMap;
use std::collections::{BTreeMap, HashMap};
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

struct InterfaceName {
    /// True when this interface name has been remapped through the use of `with` in the `bindgen!`
    /// macro invocation.
    remapped: bool,

    /// The string name for this interface.
    path: String,
}

#[derive(Default)]
struct Wasmtime {
    src: Source,
    opts: Opts,
    import_interfaces: BTreeMap<Option<PackageName>, Vec<ImportInterface>>,
    import_functions: Vec<ImportFunction>,
    exports: Exports,
    types: Types,
    sizes: SizeAlign,
    interface_names: HashMap<InterfaceId, InterfaceName>,
    with_name_counter: usize,
}

struct ImportInterface {
    snake: String,
    module: String,
}
struct ImportFunction {
    add_to_linker: String,
    sig: String,
}

#[derive(Default)]
struct Exports {
    fields: BTreeMap<String, (String, String)>,
    modules: BTreeMap<Option<PackageName>, Vec<String>>,
    funcs: Vec<String>,
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

    /// Whether or not to use async rust functions and traits.
    pub async_: bool,

    /// A list of "trappable errors" which are used to replace the `E` in
    /// `result<T, E>` found in WIT.
    pub trappable_error_type: Vec<TrappableError>,

    /// Whether to generate owning or borrowing type definitions.
    pub ownership: Ownership,

    /// Whether or not to generate code for only the interfaces of this wit file or not.
    pub only_interfaces: bool,

    /// Remapping of interface names to rust module names.
    /// TODO: is there a better type to use for the value of this map?
    pub with: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct TrappableError {
    /// The package and interface that define the error type being mapped.
    pub wit_package_path: String,

    /// The name of the error type in WIT that is being mapped.
    pub wit_type_name: String,

    /// The name, in Rust, of the error type to generate.
    pub rust_type_name: String,
}

impl Opts {
    pub fn generate(&self, resolve: &Resolve, world: WorldId) -> String {
        let mut r = Wasmtime::default();
        r.sizes.fill(resolve);
        r.opts = self.clone();
        r.generate(resolve, world)
    }
}

impl Wasmtime {
    fn name_interface(&mut self, resolve: &Resolve, id: InterfaceId, name: &WorldKey) -> bool {
        let with_name = resolve.name_world_key(name);
        let entry = if let Some(remapped_path) = self.opts.with.get(&with_name) {
            let name = format!("__with_name{}", self.with_name_counter);
            self.with_name_counter += 1;
            uwriteln!(self.src, "use {remapped_path} as {name};");
            InterfaceName {
                remapped: true,
                path: name,
            }
        } else {
            let path = match name {
                WorldKey::Name(name) => name.to_snake_case(),
                WorldKey::Interface(_) => {
                    let iface = &resolve.interfaces[id];
                    let pkgname = &resolve.packages[iface.package.unwrap()].name;
                    format!(
                        "{}::{}::{}",
                        pkgname.namespace.to_snake_case(),
                        pkgname.name.to_snake_case(),
                        iface.name.as_ref().unwrap().to_snake_case()
                    )
                }
            };
            InterfaceName {
                remapped: false,
                path,
            }
        };

        let remapped = entry.remapped;
        self.interface_names.insert(id, entry);

        remapped
    }

    fn generate(&mut self, resolve: &Resolve, id: WorldId) -> String {
        self.types.analyze(resolve, id);
        let world = &resolve.worlds[id];
        for (name, import) in world.imports.iter() {
            if !self.opts.only_interfaces || matches!(import, WorldItem::Interface(_)) {
                self.import(resolve, name, import);
            }
        }
        for (name, export) in world.exports.iter() {
            if !self.opts.only_interfaces || matches!(export, WorldItem::Interface(_)) {
                self.export(resolve, name, export);
            }
        }
        self.finish(resolve, id)
    }

    fn import(&mut self, resolve: &Resolve, name: &WorldKey, item: &WorldItem) {
        let mut gen = InterfaceGenerator::new(self, resolve);
        match item {
            WorldItem::Function(func) => {
                gen.generate_function_trait_sig(func);
                let sig = mem::take(&mut gen.src).into();
                gen.generate_add_function_to_linker(TypeOwner::None, func, "linker");
                let add_to_linker = gen.src.into();
                self.import_functions
                    .push(ImportFunction { sig, add_to_linker });
            }
            WorldItem::Interface(id) => {
                if gen.gen.name_interface(resolve, *id, name) {
                    return;
                }
                gen.current_interface = Some((*id, name, false));
                gen.types(*id);
                let key_name = resolve.name_world_key(name);
                gen.generate_add_to_linker(*id, &key_name);

                let module = &gen.src[..];

                let snake = match name {
                    WorldKey::Name(s) => s.to_snake_case(),
                    WorldKey::Interface(id) => resolve.interfaces[*id]
                        .name
                        .as_ref()
                        .unwrap()
                        .to_snake_case(),
                };
                let module = format!(
                    "
                        #[allow(clippy::all)]
                        pub mod {snake} {{
                            #[allow(unused_imports)]
                            use wasmtime::component::__internal::anyhow;

                            {module}
                        }}
                    "
                );
                let pkg = resolve.interfaces[*id].package.unwrap();
                let pkgname = match name {
                    WorldKey::Name(_) => None,
                    WorldKey::Interface(_) => Some(resolve.packages[pkg].name.clone()),
                };
                self.import_interfaces
                    .entry(pkgname)
                    .or_insert(Vec::new())
                    .push(ImportInterface { snake, module });
            }
            WorldItem::Type(ty) => {
                let name = match name {
                    WorldKey::Name(name) => name,
                    WorldKey::Interface(_) => unreachable!(),
                };
                gen.define_type(name, *ty);
                let body = mem::take(&mut gen.src);
                self.src.push_str(&body);
            }
        };
    }

    fn export(&mut self, resolve: &Resolve, name: &WorldKey, item: &WorldItem) {
        let mut gen = InterfaceGenerator::new(self, resolve);
        let (field, ty, getter) = match item {
            WorldItem::Function(func) => {
                gen.define_rust_guest_export(resolve, None, func);
                let body = mem::take(&mut gen.src).into();
                let (_name, getter) = gen.extract_typed_function(func);
                assert!(gen.src.is_empty());
                self.exports.funcs.push(body);
                (
                    func.name.to_snake_case(),
                    "wasmtime::component::Func".to_string(),
                    getter,
                )
            }
            WorldItem::Type(_) => unreachable!(),
            WorldItem::Interface(id) => {
                gen.gen.name_interface(resolve, *id, name);
                gen.current_interface = Some((*id, name, true));
                gen.types(*id);
                let iface = &resolve.interfaces[*id];
                let iface_name = match name {
                    WorldKey::Name(name) => name,
                    WorldKey::Interface(_) => iface.name.as_ref().unwrap(),
                };
                let camel = to_rust_upper_camel_case(iface_name);
                uwriteln!(gen.src, "pub struct {camel} {{");
                for (_, func) in iface.functions.iter() {
                    uwriteln!(
                        gen.src,
                        "{}: wasmtime::component::Func,",
                        func.name.to_snake_case()
                    );
                }
                uwriteln!(gen.src, "}}");

                uwriteln!(gen.src, "impl {camel} {{");
                uwrite!(
                    gen.src,
                    "
                        pub fn new(
                            __exports: &mut wasmtime::component::ExportInstance<'_, '_>,
                        ) -> wasmtime::Result<{camel}> {{
                    "
                );
                let mut fields = Vec::new();
                for (_, func) in iface.functions.iter() {
                    let (name, getter) = gen.extract_typed_function(func);
                    uwriteln!(gen.src, "let {name} = {getter};");
                    fields.push(name);
                }
                uwriteln!(gen.src, "Ok({camel} {{");
                for name in fields {
                    uwriteln!(gen.src, "{name},");
                }
                uwriteln!(gen.src, "}})");
                uwriteln!(gen.src, "}}");
                for (_, func) in iface.functions.iter() {
                    gen.define_rust_guest_export(resolve, Some(name), func);
                }
                uwriteln!(gen.src, "}}");

                let module = &gen.src[..];
                let snake = iface_name.to_snake_case();

                let module = format!(
                    "
                        #[allow(clippy::all)]
                        pub mod {snake} {{
                            #[allow(unused_imports)]
                            use wasmtime::component::__internal::anyhow;

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
                    .entry(pkgname.clone())
                    .or_insert(Vec::new())
                    .push(module);

                let name = resolve.name_world_key(name);
                let (path, method_name) = match pkgname {
                    Some(pkgname) => (
                        format!(
                            "exports::{}::{}::{snake}::{camel}",
                            pkgname.namespace.to_snake_case(),
                            pkgname.name.to_snake_case(),
                        ),
                        format!(
                            "{}_{}_{snake}",
                            pkgname.namespace.to_snake_case(),
                            pkgname.name.to_snake_case()
                        ),
                    ),
                    None => (format!("exports::{snake}::{camel}"), snake.clone()),
                };
                let getter = format!(
                    "\
                        {path}::new(
                            &mut __exports.instance(\"{name}\")
                                .ok_or_else(|| anyhow::anyhow!(\"exported instance `{name}` not present\"))?
                        )?\
                    "
                );
                let field = format!("interface{}", self.exports.fields.len());
                self.exports.funcs.push(format!(
                    "
                        pub fn {method_name}(&self) -> &{path} {{
                            &self.{field}
                        }}
                    ",
                ));
                (field, path, getter)
            }
        };
        let prev = self.exports.fields.insert(field, (ty, getter));
        assert!(prev.is_none());
    }

    fn build_struct(&mut self, resolve: &Resolve, world: WorldId) {
        let camel = to_rust_upper_camel_case(&resolve.worlds[world].name);
        uwriteln!(self.src, "pub struct {camel} {{");
        for (name, (ty, _)) in self.exports.fields.iter() {
            uwriteln!(self.src, "{name}: {ty},");
        }
        self.src.push_str("}\n");

        let (async_, async__, send, await_) = if self.opts.async_ {
            ("async", "_async", ":Send", ".await")
        } else {
            ("", "", "", "")
        };

        self.toplevel_import_trait(resolve, world);

        uwriteln!(self.src, "const _: () = {{");
        uwriteln!(self.src, "use wasmtime::component::__internal::anyhow;");

        uwriteln!(self.src, "impl {camel} {{");
        self.toplevel_add_to_linker(resolve, world);
        uwriteln!(
            self.src,
            "
                /// Instantiates the provided `module` using the specified
                /// parameters, wrapping up the result in a structure that
                /// translates between wasm and the host.
                pub {async_} fn instantiate{async__}<T {send}>(
                    mut store: impl wasmtime::AsContextMut<Data = T>,
                    component: &wasmtime::component::Component,
                    linker: &wasmtime::component::Linker<T>,
                ) -> wasmtime::Result<(Self, wasmtime::component::Instance)> {{
                    let instance = linker.instantiate{async__}(&mut store, component){await_}?;
                    Ok((Self::new(store, &instance)?, instance))
                }}

                /// Instantiates a pre-instantiated module using the specified
                /// parameters, wrapping up the result in a structure that
                /// translates between wasm and the host.
                pub {async_} fn instantiate_pre<T {send}>(
                    mut store: impl wasmtime::AsContextMut<Data = T>,
                    instance_pre: &wasmtime::component::InstancePre<T>,
                ) -> wasmtime::Result<(Self, wasmtime::component::Instance)> {{
                    let instance = instance_pre.instantiate{async__}(&mut store){await_}?;
                    Ok((Self::new(store, &instance)?, instance))
                }}

                /// Low-level creation wrapper for wrapping up the exports
                /// of the `instance` provided in this structure of wasm
                /// exports.
                ///
                /// This function will extract exports from the `instance`
                /// defined within `store` and wrap them all up in the
                /// returned structure which can be used to interact with
                /// the wasm module.
                pub fn new(
                    mut store: impl wasmtime::AsContextMut,
                    instance: &wasmtime::component::Instance,
                ) -> wasmtime::Result<Self> {{
                    let mut store = store.as_context_mut();
                    let mut exports = instance.exports(&mut store);
                    let mut __exports = exports.root();
            ",
        );
        for (name, (_, get)) in self.exports.fields.iter() {
            uwriteln!(self.src, "let {name} = {get};");
        }
        uwriteln!(self.src, "Ok({camel} {{");
        for (name, _) in self.exports.fields.iter() {
            uwriteln!(self.src, "{name},");
        }
        uwriteln!(self.src, "}})");
        uwriteln!(self.src, "}}"); // close `fn new`

        for func in self.exports.funcs.iter() {
            self.src.push_str(func);
        }

        uwriteln!(self.src, "}}"); // close `impl {camel}`

        uwriteln!(self.src, "}};"); // close `const _: () = ...
    }

    fn finish(&mut self, resolve: &Resolve, world: WorldId) -> String {
        if !self.opts.only_interfaces {
            self.build_struct(resolve, world)
        }

        let imports = mem::take(&mut self.import_interfaces);
        self.emit_modules(
            &imports
                .into_iter()
                .map(|(k, v)| (k, v.into_iter().map(|m| m.module).collect()))
                .collect(),
        );
        if !self.exports.modules.is_empty() {
            uwriteln!(self.src, "pub mod exports {{");
            let exports = mem::take(&mut self.exports.modules);
            self.emit_modules(&exports);
            uwriteln!(self.src, "}}");
        }

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

        src.into()
    }

    fn emit_modules(&mut self, modules: &BTreeMap<Option<PackageName>, Vec<String>>) {
        let mut map = BTreeMap::new();
        for (pkg, modules) in modules {
            match pkg {
                Some(pkg) => {
                    let prev = map
                        .entry(&pkg.namespace)
                        .or_insert(BTreeMap::new())
                        .insert(&pkg.name, modules);
                    assert!(prev.is_none());
                }
                None => {
                    for module in modules {
                        uwriteln!(self.src, "{module}");
                    }
                }
            }
        }
        for (ns, pkgs) in map {
            uwriteln!(self.src, "pub mod {} {{", ns.to_snake_case());
            for (pkg, modules) in pkgs {
                uwriteln!(self.src, "pub mod {} {{", pkg.to_snake_case());
                for module in modules {
                    uwriteln!(self.src, "{module}");
                }
                uwriteln!(self.src, "}}");
            }
            uwriteln!(self.src, "}}");
        }
    }
}

impl Wasmtime {
    fn toplevel_import_trait(&mut self, resolve: &Resolve, world: WorldId) {
        if self.import_functions.is_empty() {
            return;
        }

        let world_camel = to_rust_upper_camel_case(&resolve.worlds[world].name);
        if self.opts.async_ {
            uwriteln!(self.src, "#[wasmtime::component::__internal::async_trait]")
        }
        uwriteln!(self.src, "pub trait {world_camel}Imports {{");
        for f in self.import_functions.iter() {
            self.src.push_str(&f.sig);
            self.src.push_str("\n");
        }
        uwriteln!(self.src, "}}");
    }

    fn toplevel_add_to_linker(&mut self, resolve: &Resolve, world: WorldId) {
        if self.import_interfaces.is_empty() && self.import_functions.is_empty() {
            return;
        }
        let mut interfaces = Vec::new();
        for (pkg, imports) in self.import_interfaces.iter() {
            for import in imports {
                let mut path = String::new();
                if let Some(pkg) = pkg {
                    path.push_str(&pkg.namespace.to_snake_case());
                    path.push_str("::");
                    path.push_str(&pkg.name.to_snake_case());
                    path.push_str("::");
                }
                path.push_str(&import.snake);
                interfaces.push(path)
            }
        }

        uwrite!(
            self.src,
            "
                pub fn add_to_linker<T, U>(
                    linker: &mut wasmtime::component::Linker<T>,
                    get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
                ) -> wasmtime::Result<()>
                    where U: \
            "
        );
        let world_camel = to_rust_upper_camel_case(&resolve.worlds[world].name);
        let world_trait = format!("{world_camel}Imports");
        for (i, name) in interfaces
            .iter()
            .map(|n| format!("{n}::Host"))
            .chain(if self.import_functions.is_empty() {
                None
            } else {
                Some(world_trait.clone())
            })
            .enumerate()
        {
            if i > 0 {
                self.src.push_str(" + ");
            }
            self.src.push_str(&name);
        }
        let maybe_send = if self.opts.async_ {
            " + Send, T: Send"
        } else {
            ""
        };
        self.src.push_str(maybe_send);
        self.src.push_str(",\n{\n");
        for name in interfaces.iter() {
            uwriteln!(self.src, "{name}::add_to_linker(linker, get)?;");
        }
        if !self.import_functions.is_empty() {
            uwriteln!(self.src, "Self::add_root_to_linker(linker, get)?;");
        }
        uwriteln!(self.src, "Ok(())\n}}");
        if self.import_functions.is_empty() {
            return;
        }

        uwrite!(
            self.src,
            "
                pub fn add_root_to_linker<T, U>(
                    linker: &mut wasmtime::component::Linker<T>,
                    get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
                ) -> wasmtime::Result<()>
                    where U: {world_trait}{maybe_send}
                {{
                    let mut linker = linker.root();
            ",
        );
        for f in self.import_functions.iter() {
            self.src.push_str(&f.add_to_linker);
            self.src.push_str("\n");
        }
        uwriteln!(self.src, "Ok(())\n}}");
    }
}

fn resolve_type_in_package(
    resolve: &Resolve,
    package_path: &str,
    type_name: &str,
) -> anyhow::Result<TypeId> {
    // foo:bar/baz

    let (namespace, rest) = package_path
        .split_once(':')
        .ok_or_else(|| anyhow!("Invalid package path: missing package identifier"))?;

    let (package_name, iface_name) = rest
        .split_once('/')
        .ok_or_else(|| anyhow!("Invalid package path: missing namespace separator"))?;

    // TODO: we should handle version annotations
    if package_name.contains('@') {
        bail!("Invalid package path: version parsing is not currently handled");
    }

    let packages = Vec::from_iter(
        resolve
            .package_names
            .iter()
            .filter(|(pname, _)| pname.namespace == namespace && pname.name == package_name),
    );

    if packages.len() != 1 {
        if packages.is_empty() {
            bail!("No package named `{}`", namespace);
        } else {
            // Getting here is a bug, parsing version identifiers would disambiguate the intended
            // package.
            bail!(
                "Multiple packages named `{}` found ({:?})",
                namespace,
                packages
            );
        }
    }

    let (_, &package_id) = packages[0];
    let package = &resolve.packages[package_id];

    let (_, &iface_id) = package
        .interfaces
        .iter()
        .find(|(name, _)| name.as_str() == iface_name)
        .ok_or_else(|| {
            anyhow!(
                "Unknown interface `{}` in package `{}`",
                iface_name,
                package_path
            )
        })?;

    let iface = &resolve.interfaces[iface_id];

    let (_, &type_id) = iface
        .types
        .iter()
        .find(|(n, _)| n.as_str() == type_name)
        .ok_or_else(|| {
            anyhow!(
                "No type named `{}` in package `{}`",
                package_name,
                package_path
            )
        })?;

    Ok(type_id)
}

struct InterfaceGenerator<'a> {
    src: Source,
    gen: &'a mut Wasmtime,
    resolve: &'a Resolve,
    current_interface: Option<(InterfaceId, &'a WorldKey, bool)>,

    /// A mapping of wit types to their rust type name equivalent. This is the pre-processed
    /// version of `gen.opts.trappable_error_types`, where the types have been eagerly resolved.
    trappable_errors: IndexMap<TypeId, String>,
}

impl<'a> InterfaceGenerator<'a> {
    fn new(gen: &'a mut Wasmtime, resolve: &'a Resolve) -> InterfaceGenerator<'a> {
        let trappable_errors = gen
            .opts
            .trappable_error_type
            .iter()
            .map(|te| {
                let id = resolve_type_in_package(resolve, &te.wit_package_path, &te.wit_type_name)
                    .context(format!("resolving {:?}", te))?;
                Ok((id, te.rust_type_name.clone()))
            })
            .collect::<anyhow::Result<IndexMap<_, _>>>()
            .unwrap();

        InterfaceGenerator {
            src: Source::default(),
            gen,
            resolve,
            current_interface: None,
            trappable_errors,
        }
    }

    fn types(&mut self, id: InterfaceId) {
        for (name, id) in self.resolve.interfaces[id].types.iter() {
            self.define_type(name, *id);

            if let Some(rust_name) = self.trappable_errors.get(id) {
                self.define_trappable_error_type(*id, rust_name.clone())
            }
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
            TypeDefKind::Union(u) => self.type_union(id, name, u, &ty.docs),
            TypeDefKind::List(t) => self.type_list(id, name, t, &ty.docs),
            TypeDefKind::Type(t) => self.type_alias(id, name, t, &ty.docs),
            TypeDefKind::Future(_) => todo!("generate for future"),
            TypeDefKind::Stream(_) => todo!("generate for stream"),
            TypeDefKind::Handle(_) => todo!("#6722"),
            TypeDefKind::Resource => todo!("#6722"),
            TypeDefKind::Unknown => unreachable!(),
        }
    }

    fn type_record(&mut self, id: TypeId, _name: &str, record: &Record, docs: &Docs) {
        let info = self.info(id);
        for (name, mode) in self.modes_of(id) {
            let lt = self.lifetime_for(&info, mode);
            self.rustdoc(docs);

            self.push_str("#[derive(wasmtime::component::ComponentType)]\n");
            if lt.is_none() {
                self.push_str("#[derive(wasmtime::component::Lift)]\n");
            }
            self.push_str("#[derive(wasmtime::component::Lower)]\n");
            self.push_str("#[component(record)]\n");

            if !info.has_list {
                self.push_str("#[derive(Copy, Clone)]\n");
            } else {
                self.push_str("#[derive(Clone)]\n");
            }
            self.push_str(&format!("pub struct {}", name));
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
            self.push_str(&format!("f.debug_struct(\"{}\")", name));
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
                self.push_str("impl std::error::Error for ");
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
            self.push_str(&format!("pub type {}", name));
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
        let rust_name = to_rust_upper_camel_case(name);
        self.src.push_str("wasmtime::component::flags!(\n");
        self.src.push_str(&format!("{rust_name} {{\n"));
        for flag in flags.flags.iter() {
            // TODO wasmtime-component-macro doesnt support docs for flags rn
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

    fn type_union(&mut self, id: TypeId, _name: &str, union: &Union, docs: &Docs) {
        self.print_rust_enum(
            id,
            std::iter::zip(self.union_case_names(union), &union.cases)
                .map(|(name, case)| (name, None, &case.docs, Some(&case.ty))),
            docs,
            "union",
        );
    }

    fn type_option(&mut self, id: TypeId, _name: &str, payload: &Type, docs: &Docs) {
        let info = self.info(id);

        for (name, mode) in self.modes_of(id) {
            self.rustdoc(docs);
            let lt = self.lifetime_for(&info, mode);
            self.push_str(&format!("pub type {}", name));
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
        uwriteln!(
            self.src,
            "assert!({} == <{name} as wasmtime::component::ComponentType>::SIZE32);",
            self.gen.sizes.size(&Type::Id(id)),
        );
        uwriteln!(
            self.src,
            "assert!({} == <{name} as wasmtime::component::ComponentType>::ALIGN32);",
            self.gen.sizes.align(&Type::Id(id)),
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

        for (name, mode) in self.modes_of(id) {
            let name = to_rust_upper_camel_case(&name);

            self.rustdoc(docs);
            let lt = self.lifetime_for(&info, mode);
            self.push_str("#[derive(wasmtime::component::ComponentType)]\n");
            if lt.is_none() {
                self.push_str("#[derive(wasmtime::component::Lift)]\n");
            }
            self.push_str("#[derive(wasmtime::component::Lower)]\n");
            self.push_str(&format!("#[component({})]\n", derive_component));
            if !info.has_list {
                self.push_str("#[derive(Clone, Copy)]\n");
            } else {
                self.push_str("#[derive(Clone)]\n");
            }
            self.push_str(&format!("pub enum {name}"));
            self.print_generics(lt);
            self.push_str("{\n");
            for (case_name, component_name, docs, payload) in cases.clone() {
                self.rustdoc(docs);
                if let Some(n) = component_name {
                    self.push_str(&format!("#[component(name = \"{}\")] ", n));
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
                self.push_str("write!(f, \"{:?}\", self)");
                self.push_str("}\n");
                self.push_str("}\n");
                self.push_str("\n");

                self.push_str("impl");
                self.print_generics(lt);
                self.push_str(" std::error::Error for ");
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
            self.push_str(&format!("f.debug_tuple(\"{}::{}\")", name, case_name));
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
            self.push_str(&format!("pub type {}", name));
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

        let name = to_rust_upper_camel_case(name);
        self.rustdoc(docs);
        self.push_str("#[derive(wasmtime::component::ComponentType)]\n");
        self.push_str("#[derive(wasmtime::component::Lift)]\n");
        self.push_str("#[derive(wasmtime::component::Lower)]\n");
        self.push_str("#[component(enum)]\n");
        self.push_str("#[derive(Clone, Copy, PartialEq, Eq)]\n");
        self.push_str(&format!("pub enum {} {{\n", name));
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
            self.push_str("impl std::error::Error for ");
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
            self.push_str(&format!("pub type {}", name));
            let lt = self.lifetime_for(&info, mode);
            self.print_generics(lt);
            self.push_str(" = ");
            self.print_ty(ty, mode);
            self.push_str(";\n");
            self.assert_type(id, &name);
        }
    }

    fn type_list(&mut self, id: TypeId, _name: &str, ty: &Type, docs: &Docs) {
        let info = self.info(id);
        for (name, mode) in self.modes_of(id) {
            let lt = self.lifetime_for(&info, mode);
            self.rustdoc(docs);
            self.push_str(&format!("pub type {}", name));
            self.print_generics(lt);
            self.push_str(" = ");
            self.print_list(ty, mode);
            self.push_str(";\n");
            self.assert_type(id, &name);
        }
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

    fn special_case_trappable_error(
        &self,
        results: &Results,
    ) -> Option<(&'a Result_, TypeId, String)> {
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

        let rust_type = self.trappable_errors.get(&error_typeid)?;

        Some((result, error_typeid, rust_type.clone()))
    }

    fn generate_add_to_linker(&mut self, id: InterfaceId, name: &str) {
        let iface = &self.resolve.interfaces[id];
        let owner = TypeOwner::Interface(id);

        if self.gen.opts.async_ {
            uwriteln!(self.src, "#[wasmtime::component::__internal::async_trait]")
        }
        // Generate the `pub trait` which represents the host functionality for
        // this import.
        uwriteln!(self.src, "pub trait Host {{");
        for (_, func) in iface.functions.iter() {
            self.generate_function_trait_sig(func);
        }
        uwriteln!(self.src, "}}");

        let where_clause = if self.gen.opts.async_ {
            "T: Send, U: Host + Send".to_string()
        } else {
            "U: Host".to_string()
        };
        uwriteln!(
            self.src,
            "
                pub fn add_to_linker<T, U>(
                    linker: &mut wasmtime::component::Linker<T>,
                    get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
                ) -> wasmtime::Result<()>
                    where {where_clause},
                {{
            "
        );
        uwriteln!(self.src, "let mut inst = linker.instance(\"{name}\")?;");
        for (_, func) in iface.functions.iter() {
            self.generate_add_function_to_linker(owner, func, "inst");
        }
        uwriteln!(self.src, "Ok(())");
        uwriteln!(self.src, "}}");
    }

    fn generate_add_function_to_linker(&mut self, owner: TypeOwner, func: &Function, linker: &str) {
        uwrite!(
            self.src,
            "{linker}.{}(\"{}\", ",
            if self.gen.opts.async_ {
                "func_wrap_async"
            } else {
                "func_wrap"
            },
            func.name
        );
        self.generate_guest_import_closure(owner, func);
        uwriteln!(self.src, ")?;")
    }

    fn generate_guest_import_closure(&mut self, owner: TypeOwner, func: &Function) {
        // Generate the closure that's passed to a `Linker`, the final piece of
        // codegen here.
        self.src
            .push_str("move |mut caller: wasmtime::StoreContextMut<'_, T>, (");
        for (i, _param) in func.params.iter().enumerate() {
            uwrite!(self.src, "arg{},", i);
        }
        self.src.push_str(") : (");
        for param in func.params.iter() {
            // Lift is required to be impled for this type, so we can't use
            // a borrowed type:
            self.print_ty(&param.1, TypeMode::Owned);
            self.src.push_str(", ");
        }
        self.src.push_str(") |");
        if self.gen.opts.async_ {
            self.src.push_str(" Box::new(async move { \n");
        } else {
            self.src.push_str(" { \n");
        }

        if self.gen.opts.tracing {
            uwrite!(
                self.src,
                "
                   let span = tracing::span!(
                       tracing::Level::TRACE,
                       \"wit-bindgen import\",
                       module = \"{}\",
                       function = \"{}\",
                   );
                   let _enter = span.enter();
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
            let mut event_fields = func
                .params
                .iter()
                .enumerate()
                .map(|(i, (name, _ty))| {
                    let name = to_rust_ident(&name);
                    format!("{name} = tracing::field::debug(&arg{i})")
                })
                .collect::<Vec<String>>();
            event_fields.push(format!("\"call\""));
            uwrite!(
                self.src,
                "tracing::event!(tracing::Level::TRACE, {});\n",
                event_fields.join(", ")
            );
        }

        self.src.push_str("let host = get(caller.data_mut());\n");

        uwrite!(self.src, "let r = host.{}(", func.name.to_snake_case());
        for (i, _) in func.params.iter().enumerate() {
            uwrite!(self.src, "arg{},", i);
        }
        if self.gen.opts.async_ {
            uwrite!(self.src, ").await;\n");
        } else {
            uwrite!(self.src, ");\n");
        }

        if self.gen.opts.tracing {
            uwrite!(
                self.src,
                "tracing::event!(tracing::Level::TRACE, result = tracing::field::debug(&r), \"return\");"
            );
        }

        if self.special_case_trappable_error(&func.results).is_some() {
            uwrite!(
                self.src,
                "match r {{
                    Ok(a) => Ok((Ok(a),)),
                    Err(e) => match e.downcast() {{
                        Ok(api_error) => Ok((Err(api_error),)),
                        Err(anyhow_error) => Err(anyhow_error),
                    }}
                }}"
            );
        } else if func.results.iter_types().len() == 1 {
            uwrite!(self.src, "Ok((r?,))\n");
        } else {
            uwrite!(self.src, "r\n");
        }

        if self.gen.opts.async_ {
            // Need to close Box::new and async block
            self.src.push_str("})");
        } else {
            self.src.push_str("}");
        }
    }

    fn generate_function_trait_sig(&mut self, func: &Function) {
        self.rustdoc(&func.docs);

        if self.gen.opts.async_ {
            self.push_str("async ");
        }
        self.push_str("fn ");
        self.push_str(&to_rust_ident(&func.name));
        self.push_str("(&mut self, ");
        for (name, param) in func.params.iter() {
            let name = to_rust_ident(name);
            self.push_str(&name);
            self.push_str(": ");
            self.print_ty(param, TypeMode::Owned);
            self.push_str(",");
        }
        self.push_str(")");
        self.push_str(" -> ");

        if let Some((r, error_id, error_typename)) =
            self.special_case_trappable_error(&func.results)
        {
            // Functions which have a single result `result<ok,err>` get special
            // cased to use the host_wasmtime_rust::Error<err>, making it possible
            // for them to trap or use `?` to propogate their errors
            self.push_str("Result<");
            if let Some(ok) = r.ok {
                self.print_ty(&ok, TypeMode::Owned);
            } else {
                self.push_str("()");
            }
            self.push_str(",");
            if let TypeOwner::Interface(id) = self.resolve.types[error_id].owner {
                if let Some(path) = self.path_to_interface(id) {
                    self.push_str(&path);
                    self.push_str("::");
                }
            }
            self.push_str(&error_typename);
            self.push_str(">");
        } else {
            // All other functions get their return values wrapped in an wasmtime::Result.
            // Returning the anyhow::Error case can be used to trap.
            self.push_str("wasmtime::Result<");
            self.print_result_ty(&func.results, TypeMode::Owned);
            self.push_str(">");
        }

        self.push_str(";\n");
    }

    fn extract_typed_function(&mut self, func: &Function) -> (String, String) {
        let prev = mem::take(&mut self.src);
        let snake = func.name.to_snake_case();
        uwrite!(self.src, "*__exports.typed_func::<(");
        for (_, ty) in func.params.iter() {
            self.print_ty(ty, TypeMode::AllBorrowed("'_"));
            self.push_str(", ");
        }
        self.src.push_str("), (");
        for ty in func.results.iter_types() {
            self.print_ty(ty, TypeMode::Owned);
            self.push_str(", ");
        }
        self.src.push_str(")>(\"");
        self.src.push_str(&func.name);
        self.src.push_str("\")?.func()");

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
        let (async_, async__, await_) = if self.gen.opts.async_ {
            ("async", "_async", ".await")
        } else {
            ("", "", "")
        };

        self.rustdoc(&func.docs);
        uwrite!(
            self.src,
            "pub {async_} fn call_{}<S: wasmtime::AsContextMut>(&self, mut store: S, ",
            func.name.to_snake_case(),
        );
        for (i, param) in func.params.iter().enumerate() {
            uwrite!(self.src, "arg{}: ", i);
            self.print_ty(&param.1, TypeMode::AllBorrowed("'_"));
            self.push_str(",");
        }
        self.src.push_str(") -> wasmtime::Result<");
        self.print_result_ty(&func.results, TypeMode::Owned);

        if self.gen.opts.async_ {
            self.src
                .push_str("> where <S as wasmtime::AsContext>::Data: Send {\n");
        } else {
            self.src.push_str("> {\n");
        }

        if self.gen.opts.tracing {
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
                   let _enter = span.enter();
               ",
                func.name,
            ));
        }

        self.src.push_str("let callee = unsafe {\n");
        self.src.push_str("wasmtime::component::TypedFunc::<(");
        for (_, ty) in func.params.iter() {
            self.print_ty(ty, TypeMode::AllBorrowed("'_"));
            self.push_str(", ");
        }
        self.src.push_str("), (");
        for ty in func.results.iter_types() {
            self.print_ty(ty, TypeMode::Owned);
            self.push_str(", ");
        }
        uwriteln!(
            self.src,
            ")>::new_unchecked(self.{})",
            func.name.to_snake_case()
        );
        self.src.push_str("};\n");
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
        uwriteln!(self.src, ")){await_}?;");

        uwriteln!(
            self.src,
            "callee.post_return{async__}(store.as_context_mut()){await_}?;"
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

        // End function body
        self.src.push_str("}\n");
    }

    fn define_trappable_error_type(&mut self, id: TypeId, rust_name: String) {
        let info = self.info(id);
        if self.lifetime_for(&info, TypeMode::Owned).is_some() {
            panic!("wit error for {rust_name} is not 'static")
        }
        let abi_type = self.param_name(id);

        uwriteln!(
            self.src,
            "
                #[derive(Debug)]
                pub struct {rust_name} {{
                    inner: anyhow::Error,
                }}
                impl std::fmt::Display for {rust_name} {{
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{
                        write!(f, \"{{}}\", self.inner)
                    }}
                }}
                impl std::error::Error for {rust_name} {{
                    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {{
                        self.inner.source()
                    }}
                }}
                impl {rust_name} {{
                    pub fn trap(inner: anyhow::Error) -> Self {{
                        Self {{ inner }}
                    }}
                    pub fn downcast(self) -> Result<{abi_type}, anyhow::Error> {{
                        self.inner.downcast()
                    }}
                    pub fn downcast_ref(&self) -> Option<&{abi_type}> {{
                        self.inner.downcast_ref()
                    }}
                    pub fn context(self, s: impl Into<String>) -> Self {{
                        Self {{ inner: self.inner.context(s.into()) }}
                    }}
                }}
                impl From<{abi_type}> for {rust_name} {{
                    fn from(abi: {abi_type}) -> {rust_name} {{
                        {rust_name} {{ inner: anyhow::Error::from(abi) }}
                    }}
                }}
           "
        );
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
}

impl<'a> RustGenerator<'a> for InterfaceGenerator<'a> {
    fn resolve(&self) -> &'a Resolve {
        self.resolve
    }

    fn ownership(&self) -> Ownership {
        self.gen.opts.ownership
    }

    fn path_to_interface(&self, interface: InterfaceId) -> Option<String> {
        let mut path_to_root = String::new();
        if let Some((cur, key, is_export)) = self.current_interface {
            if cur == interface {
                return None;
            }
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
        let InterfaceName { path, .. } = &self.gen.interface_names[&interface];
        path_to_root.push_str(path);
        Some(path_to_root)
    }

    fn push_str(&mut self, s: &str) {
        self.src.push_str(s);
    }

    fn info(&self, ty: TypeId) -> TypeInfo {
        self.gen.types.get(ty)
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
