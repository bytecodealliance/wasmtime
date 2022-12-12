use crate::rust::{to_rust_ident, RustGenerator, TypeMode};
use crate::types::{TypeInfo, Types};
use heck::*;
use std::collections::BTreeMap;
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

#[derive(Default)]
struct Wasmtime {
    src: Source,
    opts: Opts,
    imports: Vec<String>,
    exports: Exports,
}

#[derive(Default)]
struct Exports {
    fields: BTreeMap<String, (String, String)>,
    funcs: Vec<String>,
}

#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
pub struct Opts {
    /// Whether or not `rustfmt` is executed to format generated code.
    #[cfg_attr(feature = "clap", arg(long))]
    pub rustfmt: bool,

    /// Whether or not to emit `tracing` macro calls on function entry/exit.
    #[cfg_attr(feature = "clap", arg(long))]
    pub tracing: bool,

    /// Whether or not to use async rust functions and traits.
    #[cfg_attr(feature = "clap", arg(long = "async"))]
    pub async_: bool,

    /// For a given wit interface and type name, generate a "trappable error type"
    /// of the following Rust type name
    #[cfg_attr(feature = "clap", arg(long = "trappable_error_type"), clap(value_name="INTERFACE:TYPE=RUSTTYPE", value_parser = parse_trappable_error))]
    pub trappable_error_type: Vec<(String, String, String)>,
}

impl Opts {
    pub fn generate(&self, world: &World) -> String {
        let mut r = Wasmtime::default();
        r.opts = self.clone();
        r.generate(world)
    }
}

#[cfg(feature = "clap")]
// Argument looks like `INTERFACE:TYPE=RUSTTYPE`
fn parse_trappable_error(s: &str) -> Result<(String, String, String)> {
    let (interface, after_colon) = s
        .split_once(':')
        .ok_or_else(|| anyhow!("expected `:` separator"))?;
    let (ty, rustty) = after_colon
        .split_once('=')
        .ok_or_else(|| anyhow!("expected `=` separator"))?;
    Ok((interface, ty, rustty))
}

impl Wasmtime {
    fn generate(&mut self, world: &World) -> String {
        for (name, import) in world.imports.iter() {
            self.import(name, import);
        }
        for (name, export) in world.exports.iter() {
            self.export(name, export);
        }
        if let Some(iface) = &world.default {
            self.export_default(&world.name, iface);
        }
        self.finish(world)
    }

    fn import(&mut self, name: &str, iface: &Interface) {
        let mut gen = InterfaceGenerator::new(self, iface, TypeMode::Owned);
        gen.types();
        gen.generate_trappable_error_types();
        gen.generate_add_to_linker(name);

        let snake = name.to_snake_case();
        let module = &gen.src[..];

        uwriteln!(
            self.src,
            "
                #[allow(clippy::all)]
                pub mod {snake} {{
                    #[allow(unused_imports)]
                    use wasmtime::component::__internal::anyhow;

                    {module}
                }}
            "
        );

        self.imports.push(snake);
    }

    fn export(&mut self, name: &str, iface: &Interface) {
        let mut gen = InterfaceGenerator::new(self, iface, TypeMode::AllBorrowed("'a"));
        gen.types();
        gen.generate_trappable_error_types();

        let camel = name.to_upper_camel_case();
        uwriteln!(gen.src, "pub struct {camel} {{");
        for func in iface.functions.iter() {
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
                ) -> anyhow::Result<{camel}> {{
            "
        );
        let fields = gen.extract_typed_functions();
        for (name, getter) in fields.iter() {
            uwriteln!(gen.src, "let {name} = {getter};");
        }
        uwriteln!(gen.src, "Ok({camel} {{");
        for (name, _) in fields.iter() {
            uwriteln!(gen.src, "{name},");
        }
        uwriteln!(gen.src, "}})");
        uwriteln!(gen.src, "}}");
        for func in iface.functions.iter() {
            gen.define_rust_guest_export(Some(name), func);
        }
        uwriteln!(gen.src, "}}");

        let snake = name.to_snake_case();
        let module = &gen.src[..];

        uwriteln!(
            self.src,
            "
                #[allow(clippy::all)]
                pub mod {snake} {{
                    #[allow(unused_imports)]
                    use wasmtime::component::__internal::anyhow;

                    {module}
                }}
            "
        );

        let getter = format!(
            "\
                {snake}::{camel}::new(
                    &mut __exports.instance(\"{name}\")
                        .ok_or_else(|| anyhow::anyhow!(\"exported instance `{name}` not present\"))?
                )?\
            "
        );
        let prev = self
            .exports
            .fields
            .insert(snake.clone(), (format!("{snake}::{camel}"), getter));
        assert!(prev.is_none());
        self.exports.funcs.push(format!(
            "
                pub fn {snake}(&self) -> &{snake}::{camel} {{
                    &self.{snake}
                }}
            "
        ));
    }

    fn export_default(&mut self, _name: &str, iface: &Interface) {
        let mut gen = InterfaceGenerator::new(self, iface, TypeMode::AllBorrowed("'a"));
        gen.types();
        gen.generate_trappable_error_types();
        let fields = gen.extract_typed_functions();
        for (name, getter) in fields {
            let prev = gen
                .gen
                .exports
                .fields
                .insert(name, ("wasmtime::component::Func".to_string(), getter));
            assert!(prev.is_none());
        }

        for func in iface.functions.iter() {
            let prev = mem::take(&mut gen.src);
            gen.define_rust_guest_export(None, func);
            let func = mem::replace(&mut gen.src, prev);
            gen.gen.exports.funcs.push(func.to_string());
        }

        let src = gen.src;
        self.src.push_str(&src);
    }

    fn finish(&mut self, world: &World) -> String {
        let camel = world.name.to_upper_camel_case();
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

        uwriteln!(self.src, "const _: () = {{");
        uwriteln!(self.src, "use wasmtime::component::__internal::anyhow;");

        uwriteln!(
            self.src,
            "
                impl {camel} {{
                    /// Instantiates the provided `module` using the specified
                    /// parameters, wrapping up the result in a structure that
                    /// translates between wasm and the host.
                    pub {async_} fn instantiate{async__}<T {send}>(
                        mut store: impl wasmtime::AsContextMut<Data = T>,
                        component: &wasmtime::component::Component,
                        linker: &wasmtime::component::Linker<T>,
                    ) -> anyhow::Result<(Self, wasmtime::component::Instance)> {{
                        let instance = linker.instantiate{async__}(&mut store, component){await_}?;
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
                    ) -> anyhow::Result<Self> {{
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
}

impl Wasmtime {
    fn trappable_error_types<'a>(
        &'a self,
        iface: &'a Interface,
    ) -> impl Iterator<Item = (&String, &TypeId, &String)> + 'a {
        println!("TRAPPABLE ERROR TYPES {:?}", self.opts.trappable_error_type);
        println!("{:?}", iface.name);

        println!(
            "{:?}",
            self.opts
                .trappable_error_type
                .iter()
                .filter(|(interface_name, _, _)| iface.name == *interface_name)
                .collect::<Vec<_>>()
        );
        println!(
            "{:?}",
            self.opts
                .trappable_error_type
                .iter()
                .filter(|(interface_name, _, _)| iface.name == *interface_name)
                .map(|(_, tn, _)| (tn, iface.type_lookup.get(tn)))
                .collect::<Vec<_>>()
        );

        self.opts
            .trappable_error_type
            .iter()
            .filter(|(interface_name, _, _)| iface.name == *interface_name)
            .filter_map(|(_, wit_typename, rust_typename)| {
                let wit_type = iface.type_lookup.get(wit_typename)?;
                Some((wit_typename, wit_type, rust_typename))
            })
    }
}

struct InterfaceGenerator<'a> {
    src: Source,
    gen: &'a mut Wasmtime,
    iface: &'a Interface,
    default_param_mode: TypeMode,
    types: Types,
}

impl<'a> InterfaceGenerator<'a> {
    fn new(
        gen: &'a mut Wasmtime,
        iface: &'a Interface,
        default_param_mode: TypeMode,
    ) -> InterfaceGenerator<'a> {
        let mut types = Types::default();
        types.analyze(iface);
        InterfaceGenerator {
            src: Source::default(),
            gen,
            iface,
            types,
            default_param_mode,
        }
    }

    fn types(&mut self) {
        for (id, ty) in self.iface.types.iter() {
            let name = match &ty.name {
                Some(name) => name,
                None => continue,
            };
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
            }
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
        }
    }

    fn type_flags(&mut self, _id: TypeId, name: &str, flags: &Flags, docs: &Docs) {
        self.rustdoc(docs);
        self.src.push_str("wasmtime::component::flags!(\n");
        self.src
            .push_str(&format!("{} {{\n", name.to_upper_camel_case()));
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
        }
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
            let name = name.to_upper_camel_case();

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
        }
    }

    fn type_enum(&mut self, id: TypeId, name: &str, enum_: &Enum, docs: &Docs) {
        let info = self.info(id);

        let name = name.to_upper_camel_case();
        self.rustdoc(docs);
        self.push_str("#[derive(wasmtime::component::ComponentType)]\n");
        self.push_str("#[derive(wasmtime::component::Lift)]\n");
        self.push_str("#[derive(wasmtime::component::Lower)]\n");
        self.push_str("#[component(enum)]\n");
        self.push_str("#[derive(Clone, Copy, PartialEq, Eq)]\n");
        self.push_str(&format!("pub enum {} {{\n", name.to_upper_camel_case()));
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

    fn special_case_trappable_error(&self, results: &Results) -> Option<(Result_, String)> {
        // We fillin a special trappable error type in the case when a function has just one
        // result, which is itself a `result<a, e>`, and the `e` is *not* a primitive
        // (i.e. defined in std) type, and matches the typename given by the user.
        let mut i = results.iter_types();
        if i.len() == 1 {
            match i.next().unwrap() {
                Type::Id(id) => match &self.iface.types[*id].kind {
                    TypeDefKind::Result(r) => match r.err {
                        Some(Type::Id(error_typeid)) => self
                            .gen
                            .trappable_error_types(&self.iface)
                            .find(|(_, wit_error_typeid, _)| error_typeid == **wit_error_typeid)
                            .map(|(_, _, rust_errortype)| (r.clone(), rust_errortype.clone())),
                        _ => None,
                    },
                    _ => None,
                },
                _ => None,
            }
        } else {
            None
        }
    }

    fn generate_add_to_linker(&mut self, name: &str) {
        let camel = name.to_upper_camel_case();

        if self.gen.opts.async_ {
            uwriteln!(self.src, "#[wasmtime::component::__internal::async_trait]")
        }
        // Generate the `pub trait` which represents the host functionality for
        // this import.
        uwriteln!(self.src, "pub trait {camel}: Sized {{");
        for func in self.iface.functions.iter() {
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

            if let Some((r, error_typename)) = self.special_case_trappable_error(&func.results) {
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
                self.push_str(&error_typename);
                self.push_str(">");
            } else {
                // All other functions get their return values wrapped in an anyhow::Result.
                // Returning the anyhow::Error case can be used to trap.
                self.push_str("anyhow::Result<");
                self.print_result_ty(&func.results, TypeMode::Owned);
                self.push_str(">");
            }

            self.push_str(";\n");
        }
        uwriteln!(self.src, "}}");

        let where_clause = if self.gen.opts.async_ {
            format!("T: Send, U: {camel} + Send")
        } else {
            format!("U: {camel}")
        };
        uwriteln!(
            self.src,
            "
                pub fn add_to_linker<T, U>(
                    linker: &mut wasmtime::component::Linker<T>,
                    get: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
                ) -> anyhow::Result<()>
                    where {where_clause},
                {{
            "
        );
        uwriteln!(self.src, "let mut inst = linker.instance(\"{name}\")?;");
        for func in self.iface.functions.iter() {
            uwrite!(
                self.src,
                "inst.{}(\"{}\", ",
                if self.gen.opts.async_ {
                    "func_wrap_async"
                } else {
                    "func_wrap"
                },
                func.name
            );
            self.generate_guest_import_closure(func);
            uwriteln!(self.src, ")?;")
        }
        uwriteln!(self.src, "Ok(())");
        uwriteln!(self.src, "}}");
    }

    fn generate_guest_import_closure(&mut self, func: &Function) {
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
            self.src.push_str(&format!(
                "
                   let span = tracing::span!(
                       tracing::Level::TRACE,
                       \"wit-bindgen guest import\",
                       module = \"{}\",
                       function = \"{}\",
                   );
                   let _enter = span.enter();
               ",
                self.iface.name, func.name,
            ));
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

    fn extract_typed_functions(&mut self) -> Vec<(String, String)> {
        let prev = mem::take(&mut self.src);
        let mut ret = Vec::new();
        for func in self.iface.functions.iter() {
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

            ret.push((snake, mem::take(&mut self.src).to_string()));
        }
        self.src = prev;
        return ret;
    }

    fn define_rust_guest_export(&mut self, ns: Option<&str>, func: &Function) {
        let (async_, async__, await_) = if self.gen.opts.async_ {
            ("async", "_async", ".await")
        } else {
            ("", "", "")
        };

        self.rustdoc(&func.docs);
        uwrite!(
            self.src,
            "pub {async_} fn {}<S: wasmtime::AsContextMut>(&self, mut store: S, ",
            func.name.to_snake_case(),
        );
        for (i, param) in func.params.iter().enumerate() {
            uwrite!(self.src, "arg{}: ", i);
            self.print_ty(&param.1, TypeMode::AllBorrowed("'_"));
            self.push_str(",");
        }
        self.src.push_str(") -> anyhow::Result<");
        self.print_result_ty(&func.results, TypeMode::Owned);

        if self.gen.opts.async_ {
            self.src
                .push_str("> where <S as wasmtime::AsContext>::Data: Send {\n");
        } else {
            self.src.push_str("> {\n");
        }

        if self.gen.opts.tracing {
            self.src.push_str(&format!(
                "
                   let span = tracing::span!(
                       tracing::Level::TRACE,
                       \"wit-bindgen guest export\",
                       module = \"{}\",
                       function = \"{}\",
                   );
                   let _enter = span.enter();
               ",
                ns.unwrap_or("default"),
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

    fn generate_trappable_error_types(&mut self) {
        for (wit_typename, wit_type, trappable_type) in self.gen.trappable_error_types(&self.iface)
        {
            let info = self.info(*wit_type);
            if self.lifetime_for(&info, TypeMode::Owned).is_some() {
                panic!(
                    "type {:?} in interface {:?} is not 'static",
                    wit_typename, self.iface.name
                )
            }
            let abi_type = self.param_name(*wit_type);

            println!(
                "TRAPPABLE TYPE {}::{wit_typename}({abi_type}): {trappable_type}",
                self.iface.name
            );
            uwriteln!(
                self.src,
                "
                #[derive(Debug)]
                pub struct {trappable_type} {{
                    inner: anyhow::Error,
                }}
                impl std::fmt::Display for {trappable_type} {{
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{
                        write!(f, \"{{}}\", self.inner)
                    }}
                }}
                impl std::error::Error for {trappable_type} {{
                    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {{
                        self.inner.source()
                    }}
                }}
                impl {trappable_type} {{
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
                impl From<{abi_type}> for {trappable_type} {{
                    fn from(abi: {abi_type}) -> {trappable_type} {{
                        {trappable_type} {{ inner: anyhow::Error::from(abi) }}
                    }}
                }}
           "
            );
        }
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
    fn iface(&self) -> &'a Interface {
        self.iface
    }

    fn default_param_mode(&self) -> TypeMode {
        self.default_param_mode
    }

    fn push_str(&mut self, s: &str) {
        self.src.push_str(s);
    }

    fn info(&self, ty: TypeId) -> TypeInfo {
        self.types.get(ty)
    }
}
