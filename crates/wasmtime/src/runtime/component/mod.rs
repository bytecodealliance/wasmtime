//! # Embedding API for the Component Model
//!
//! This module contains the embedding API for the [Component Model] in
//! Wasmtime. This module requires the `component-model` feature to be enabled,
//! which is enabled by default. The embedding API here is mirrored after the
//! core wasm embedding API at the crate root and is intended to have the same
//! look-and-feel while handling concepts of the component model.
//!
//! [Component Model]: https://component-model.bytecodealliance.org
//!
//! The component model is a broad topic which can't be explained here fully, so
//! it's recommended to read over individual items' documentation to see more
//! about the capabilities of the embedding API. At a high-level, however,
//! perhaps the most interesting items in this module are:
//!
//! * [`Component`] - a compiled component ready to be instantiated. Similar to
//!   a [`Module`](crate::Module) for core wasm.
//!
//! * [`Linker`] - a component-style location for defining host functions. This
//!   is not the same as [`wasmtime::Linker`](crate::Linker) for core wasm
//!   modules.
//!
//! * [`bindgen!`] - a macro to generate Rust bindings for a [WIT] [world]. This
//!   maps all WIT types into Rust automatically and generates traits for
//!   embedders to implement.
//!
//! [WIT]: https://component-model.bytecodealliance.org/design/wit.html
//! [world]: https://component-model.bytecodealliance.org/design/worlds.html
//!
//! Embedders of the component model will typically start by defining their API
//! in [WIT]. This describes what will be available to guests and what needs to
//! be provided to the embedder by the guest. This [`world`][world] that was
//! created is then fed into [`bindgen!`] to generate types and traits for the
//! embedder to use. The embedder then implements these traits, adds
//! functionality via the generated `add_to_linker` method (see [`bindgen!`] for
//! more info), and then instantiates/executes a component.
//!
//! It's recommended to read over the [documentation for the Component
//! Model][Component Model] to get an overview about how to build components
//! from various languages.
//!
//! ## Example Usage
//!
//! Imagine you have the following WIT package definition in a file called world.wit
//! along with a component (my_component.wasm) that targets `my-world`:
//!
//! ```text,ignore
//! package component:my-package;
//!
//! world my-world {
//!     import name: func() -> string;
//!     export greet: func() -> string;
//! }
//! ```
//!
//! You can instantiate and call the component like so:
//!
//! ```
//! fn main() -> wasmtime::Result<()> {
//!     #   if true { return Ok(()) }
//!     // Instantiate the engine and store
//!     let engine = wasmtime::Engine::default();
//!     let mut store = wasmtime::Store::new(&engine, ());
//!
//!     // Load the component from disk
//!     let bytes = std::fs::read("my_component.wasm")?;
//!     let component = wasmtime::component::Component::new(&engine, bytes)?;
//!
//!     // Configure the linker
//!     let mut linker = wasmtime::component::Linker::new(&engine);
//!     // The component expects one import `name` that
//!     // takes no params and returns a string
//!     linker
//!         .root()
//!         .func_wrap("name", |_store, _params: ()| {
//!             Ok((String::from("Alice"),))
//!         })?;
//!
//!     // Instantiate the component
//!     let instance = linker.instantiate(&mut store, &component)?;
//!
//!     // Call the `greet` function
//!     let func = instance.get_func(&mut store, "greet").expect("greet export not found");
//!     let mut result = [wasmtime::component::Val::String("".into())];
//!     func.call(&mut store, &[], &mut result)?;
//!
//!     // This should print out `Greeting: [String("Hello, Alice!")]`
//!     println!("Greeting: {:?}", result);
//!
//!     Ok(())
//! }
//! ```
//!
//! Manually configuring the linker and calling untyped component exports is
//! a bit tedious and error prone. The [`bindgen!`] macro can be used to
//! generate bindings eliminating much of this boilerplate.
//!
//! See the docs for [`bindgen!`] for more information on how to use it.

// rustdoc appears to lie about a warning above, so squelch it for now.
#![allow(rustdoc::redundant_explicit_links)]

mod component;
#[cfg(feature = "component-model-async")]
pub(crate) mod concurrent;
mod func;
mod instance;
mod linker;
mod matching;
mod resource_table;
mod resources;
mod storage;
mod store;
pub mod types;
mod values;
pub use self::component::{Component, ComponentExportIndex};
#[cfg(feature = "component-model-async")]
pub use self::concurrent::{ErrorContext, FutureReader, Promise, PromisesUnordered, StreamReader};
pub use self::func::{
    ComponentNamedList, ComponentType, Func, Lift, Lower, TypedFunc, WasmList, WasmStr,
};
pub use self::instance::{Instance, InstanceExportLookup, InstancePre};
pub use self::linker::{Linker, LinkerInstance};
pub use self::resource_table::{ResourceTable, ResourceTableError};
pub use self::resources::{Resource, ResourceAny};
pub use self::types::{ResourceType, Type};
pub use self::values::Val;

pub(crate) use self::resources::HostResourceData;

// Re-export wasm_wave crate so the compatible version of this dep doesn't have to be
// tracked separately from wasmtime.
#[cfg(feature = "wave")]
pub use wasm_wave;

// These items are used by `#[derive(ComponentType, Lift, Lower)]`, but they are not part of
// Wasmtime's API stability guarantees
#[doc(hidden)]
pub mod __internal {
    pub use super::func::{
        bad_type_info, format_flags, lower_payload, typecheck_enum, typecheck_flags,
        typecheck_record, typecheck_variant, ComponentVariant, LiftContext, LowerContext, Options,
    };
    pub use super::matching::InstanceType;
    pub use crate::map_maybe_uninit;
    pub use crate::store::StoreOpaque;
    pub use crate::MaybeUninitExt;
    pub use alloc::boxed::Box;
    pub use alloc::string::String;
    pub use alloc::vec::Vec;
    pub use anyhow;
    pub use core::mem::transmute;
    #[cfg(feature = "async")]
    pub use trait_variant::make as trait_variant_make;
    pub use wasmtime_environ;
    pub use wasmtime_environ::component::{CanonicalAbiInfo, ComponentTypes, InterfaceType};
}

pub(crate) use self::store::ComponentStoreData;

/// Generate bindings for a [WIT world].
///
/// [WIT world]: https://component-model.bytecodealliance.org/design/worlds.html
/// [WIT package]: https://component-model.bytecodealliance.org/design/packages.html
///
/// This macro ingests a [WIT world] and will generate all the necessary
/// bindings for instantiating components that ascribe to the `world`. This
/// provides a higher-level representation of working with a component than the
/// raw [`Instance`] type which must be manually-type-checked and manually have
/// its imports provided via the [`Linker`] type.
///
/// # Examples
///
/// Examples for this macro can be found in the [`bindgen_examples`] module
/// documentation. That module has a submodule-per-example which includes the
/// source code, with WIT, used to generate the structures along with the
/// generated code itself in documentation.
///
/// # Debugging and Exploring
///
/// If you need to debug the output of `bindgen!` you can try using the
/// `WASMTIME_DEBUG_BINDGEN=1` environment variable. This will write the
/// generated code to a file on disk so rustc can produce better error messages
/// against the actual generated source instead of the macro invocation itself.
/// This additionally can enable opening up the generated code in an editor and
/// exploring it (through an error message).
///
/// The generated bindings can additionally be explored with `cargo doc` to see
/// what's generated. It's also recommended to browse the [`bindgen_examples`]
/// for example generated structures and example generated code.
///
/// # Syntax
///
/// This procedural macro accepts a few different syntaxes. The primary purpose
/// of this macro is to locate a WIT package, parse it, and then extract a
/// `world` from the parsed package. There are then codegen-specific options to
/// the bindings themselves which can additionally be specified.
///
/// Usage of this macro looks like:
///
/// ```rust
/// # macro_rules! bindgen { ($($t:tt)*) => () }
/// // Parse the `wit/` folder adjacent to this crate's `Cargo.toml` and look
/// // for a single `world` in it. There must be exactly one for this to
/// // succeed.
/// bindgen!();
///
/// // Parse the `wit/` folder adjacent to this crate's `Cargo.toml` and look
/// // for the world `foo` contained in it.
/// bindgen!("foo");
///
/// // Parse the folder `other/wit/folder` adjacent to `Cargo.toml`.
/// bindgen!(in "other/wit/folder");
/// bindgen!("foo" in "other/wit/folder");
///
/// // Parse the file `foo.wit` as a single-file WIT package with no
/// // dependencies.
/// bindgen!("foo" in "foo.wit");
///
/// // Specify a suite of options to the bindings generation, documented below
/// bindgen!({
///     world: "foo",
///     path: "other/path/to/wit",
///     // ...
/// });
/// ```
///
/// # Options Reference
///
/// This is an example listing of all options that this macro supports along
/// with documentation for each option and example syntax for each option.
///
/// ```rust
/// # macro_rules! bindgen { ($($t:tt)*) => () }
/// bindgen!({
///     world: "foo", // not needed if `path` has one `world`
///
///     // same as in `bindgen!(in "other/wit/folder")
///     path: "other/wit/folder",
///
///     // Instead of `path` the WIT document can be provided inline if
///     // desired.
///     inline: "
///         package my:inline;
///
///         world foo {
///             // ...
///         }
///     ",
///
///     // Add calls to `tracing::span!` before each import or export is called
///     // to log most arguments and return values. By default values
///     // containing lists are excluded; enable `verbose_tracing` to include
///     // them.
///     //
///     // This option defaults to `false`.
///     tracing: true,
///
///     // Include all arguments and return values in the tracing output,
///     // including values containing lists, which may be very large.
///     //
///     // This option defaults to `false`.
///     verbose_tracing: false,
///
///     // Imports will be async functions and exports
///     // are also invoked as async functions. Requires `Config::async_support`
///     // to be `true`.
///     //
///     // Note that this is only async for the host as the guest will still
///     // appear as if it's invoking blocking functions.
///     //
///     // This option defaults to `false`.
///     async: true,
///
///     // Alternative mode of async configuration where this still implies
///     // async instantiation happens, for example, but more control is
///     // provided over which imports are async and which aren't.
///     //
///     // Note that in this mode all exports are still async.
///     async: {
///         // All imports are async except for functions with these names
///         except_imports: ["foo", "bar"],
///
///         // All imports are synchronous except for functions with these names
///         //
///         // Note that this key cannot be specified with `except_imports`,
///         // only one or the other is accepted.
///         only_imports: ["foo", "bar"],
///     },
///
///     // This option is used to indicate whether imports can trap.
///     //
///     // Imports that may trap have their return types wrapped in
///     // `wasmtime::Result<T>` where the `Err` variant indicates that a
///     // trap will be raised in the guest.
///     //
///     // By default imports cannot trap and the return value is the return
///     // value from the WIT bindings itself. This value can be set to `true`
///     // to indicate that any import can trap. This value can also be set to
///     // an array-of-strings to indicate that only a set list of imports
///     // can trap.
///     trappable_imports: false,             // no imports can trap (default)
///     // trappable_imports: true,           // all imports can trap
///     // trappable_imports: ["foo", "bar"], // only these can trap
///
///     // This can be used to translate WIT return values of the form
///     // `result<T, error-type>` into `Result<T, RustErrorType>` in Rust.
///     // Users must define `RustErrorType` and the `Host` trait for the
///     // interface which defines `error-type` will have a method
///     // called `convert_error_type` which converts `RustErrorType`
///     // into `wasmtime::Result<ErrorType>`. This conversion can either
///     // return the raw WIT error (`ErrorType` here) or a trap.
///     //
///     // By default this option is not specified. This option only takes
///     // effect when `trappable_imports` is set for some imports.
///     trappable_error_type: {
///         "wasi:io/streams/stream-error" => RustErrorType,
///     },
///
///     // All generated bindgen types are "owned" meaning types like `String`
///     // are used instead of `&str`, for example. This is the default and
///     // ensures that the same type used in both imports and exports uses the
///     // same generated type.
///     ownership: Owning,
///
///     // Alternative to `Owning` above where borrowed types attempt to be used
///     // instead. The `duplicate_if_necessary` configures whether duplicate
///     // Rust types will be generated for the same WIT type if necessary, for
///     // example when a type is used both as an import and an export.
///     ownership: Borrowing {
///         duplicate_if_necessary: true
///     },
///
///     // Restrict the code generated to what's needed for the interface
///     // imports in the inlined WIT document fragment.
///     interfaces: "
///         import wasi:cli/command;
///     ",
///
///     // Remap imported interfaces or resources to types defined in Rust
///     // elsewhere. Using this option will prevent any code from being
///     // generated for interfaces mentioned here. Resources named here will
///     // not have a type generated to represent the resource.
///     //
///     // Interfaces mapped with this option should be previously generated
///     // with an invocation of this macro. Resources need to be mapped to a
///     // Rust type name.
///     with: {
///         // This can be used to indicate that entire interfaces have
///         // bindings generated elsewhere with a path pointing to the
///         // bindinges-generated module.
///         "wasi:random/random": wasmtime_wasi::bindings::random::random,
///
///         // Similarly entire packages can also be specified.
///         "wasi:cli": wasmtime_wasi::bindings::cli,
///
///         // Or, if applicable, entire namespaces can additionally be mapped.
///         "wasi": wasmtime_wasi::bindings,
///
///         // Versions are supported if multiple versions are in play:
///         "wasi:http/types@0.2.0": wasmtime_wasi_http::bindings::http::types,
///         "wasi:http@0.2.0": wasmtime_wasi_http::bindings::http,
///
///         // The `with` key can also be used to specify the `T` used in
///         // import bindings of `Resource<T>`. This can be done to configure
///         // which typed resource shows up in generated bindings and can be
///         // useful when working with the typed methods of `ResourceTable`.
///         "wasi:filesystem/types/descriptor": MyDescriptorType,
///     },
///
///     // Additional derive attributes to include on generated types (structs or enums).
///     //
///     // These are deduplicated and attached in a deterministic order.
///     additional_derives: [
///         Hash,
///         serde::Deserialize,
///         serde::Serialize,
///     ],
///
///     // An niche configuration option to require that the `T` in `Store<T>`
///     // is always `Send` in the generated bindings. Typically not needed
///     // but if synchronous bindings depend on asynchronous bindings using
///     // the `with` key then this may be required.
///     require_store_data_send: false,
///
///     // If the `wasmtime` crate is depended on at a nonstandard location
///     // or is renamed then this is the path to the root of the `wasmtime`
///     // crate. Much of the generated code needs to refer to `wasmtime` so
///     // this should be used if the `wasmtime` name is not wasmtime itself.
///     //
///     // By default this is `wasmtime`.
///     wasmtime_crate: path::to::wasmtime,
///
///     // This is an in-source alternative to using `WASMTIME_DEBUG_BINDGEN`.
///     //
///     // Note that if this option is specified then the compiler will always
///     // recompile your bindings. Cargo records the start time of when rustc
///     // is spawned by this will write a file during compilation. To Cargo
///     // that looks like a file was modified after `rustc` was spawned,
///     // so Cargo will always think your project is "dirty" and thus always
///     // recompile it. Recompiling will then overwrite the file again,
///     // starting the cycle anew. This is only recommended for debugging.
///     //
///     // This option defaults to false.
///     include_generated_code_from_file: false,
/// });
/// ```
pub use wasmtime_component_macro::bindgen;

/// Derive macro to generate implementations of the [`ComponentType`] trait.
///
/// This derive macro can be applied to `struct` and `enum` definitions and is
/// used to bind either a `record`, `enum`, or `variant` in the component model.
///
/// Note you might be looking for [`bindgen!`] rather than this macro as that
/// will generate the entire type for you rather than just a trait
/// implementation.
///
/// This macro supports a `#[component]` attribute which is used to customize
/// how the type is bound to the component model. A top-level `#[component]`
/// attribute is required to specify either `record`, `enum`, or `variant`.
///
/// ## Records
///
/// `record`s in the component model correspond to `struct`s in Rust. An example
/// is:
///
/// ```rust
/// use wasmtime::component::ComponentType;
///
/// #[derive(ComponentType)]
/// #[component(record)]
/// struct Color {
///     r: u8,
///     g: u8,
///     b: u8,
/// }
/// ```
///
/// which corresponds to the WIT type:
///
/// ```wit
/// record color {
///     r: u8,
///     g: u8,
///     b: u8,
/// }
/// ```
///
/// Note that the name `Color` here does not need to match the name in WIT.
/// That's purely used as a name in Rust of what to refer to. The field names
/// must match that in WIT, however. Field names can be customized with the
/// `#[component]` attribute though.
///
/// ```rust
/// use wasmtime::component::ComponentType;
///
/// #[derive(ComponentType)]
/// #[component(record)]
/// struct VerboseColor {
///     #[component(name = "r")]
///     red: u8,
///     #[component(name = "g")]
///     green: u8,
///     #[component(name = "b")]
///     blue: u8,
/// }
/// ```
///
/// Also note that field ordering is significant at this time and must match
/// WIT.
///
/// ## Variants
///
/// `variant`s in the component model correspond to a subset of shapes of a Rust
/// `enum`. Variants in the component model have a single optional payload type
/// which means that not all Rust `enum`s correspond to component model
/// `variant`s. An example variant is:
///
/// ```rust
/// use wasmtime::component::ComponentType;
///
/// #[derive(ComponentType)]
/// #[component(variant)]
/// enum Filter {
///     #[component(name = "none")]
///     None,
///     #[component(name = "all")]
///     All,
///     #[component(name = "some")]
///     Some(Vec<String>),
/// }
/// ```
///
/// which corresponds to the WIT type:
///
/// ```wit
/// variant filter {
///     none,
///     all,
///     some(list<string>),
/// }
/// ```
///
/// The `variant` style of derive allows an optional payload on Rust `enum`
/// variants but it must be a single unnamed field. Variants of the form `Foo(T,
/// U)` or `Foo { name: T }` are not supported at this time.
///
/// Note that the order of variants in Rust must match the order of variants in
/// WIT. Additionally it's likely that `#[component(name = "...")]` is required
/// on all Rust `enum` variants because the name currently defaults to the Rust
/// name which is typically UpperCamelCase whereas WIT uses kebab-case.
///
/// ## Enums
///
/// `enum`s in the component model correspond to C-like `enum`s in Rust. Note
/// that a component model `enum` does not allow any payloads so the Rust `enum`
/// must additionally have no payloads.
///
/// ```rust
/// use wasmtime::component::ComponentType;
///
/// #[derive(ComponentType)]
/// #[component(enum)]
/// #[repr(u8)]
/// enum Setting {
///     #[component(name = "yes")]
///     Yes,
///     #[component(name = "no")]
///     No,
///     #[component(name = "auto")]
///     Auto,
/// }
/// ```
///
/// which corresponds to the WIT type:
///
/// ```wit
/// enum setting {
///     yes,
///     no,
///     auto,
/// }
/// ```
///
/// Note that the order of variants in Rust must match the order of variants in
/// WIT. Additionally it's likely that `#[component(name = "...")]` is required
/// on all Rust `enum` variants because the name currently defaults to the Rust
/// name which is typically UpperCamelCase whereas WIT uses kebab-case.
pub use wasmtime_component_macro::ComponentType;

/// A derive macro for generating implementations of the [`Lift`] trait.
///
/// This macro will likely be applied in conjunction with the
/// [`#[derive(ComponentType)]`](macro@ComponentType) macro along the lines
/// of `#[derive(ComponentType, Lift)]`. This trait enables reading values from
/// WebAssembly.
///
/// Note you might be looking for [`bindgen!`] rather than this macro as that
/// will generate the entire type for you rather than just a trait
/// implementation.
///
/// At this time this derive macro has no configuration.
///
/// ## Examples
///
/// ```rust
/// use wasmtime::component::{ComponentType, Lift};
///
/// #[derive(ComponentType, Lift)]
/// #[component(record)]
/// struct Color {
///     r: u8,
///     g: u8,
///     b: u8,
/// }
/// ```
pub use wasmtime_component_macro::Lift;

/// A derive macro for generating implementations of the [`Lower`] trait.
///
/// This macro will likely be applied in conjunction with the
/// [`#[derive(ComponentType)]`](macro@ComponentType) macro along the lines
/// of `#[derive(ComponentType, Lower)]`. This trait enables passing values to
/// WebAssembly.
///
/// Note you might be looking for [`bindgen!`] rather than this macro as that
/// will generate the entire type for you rather than just a trait
/// implementation.
///
/// At this time this derive macro has no configuration.
///
/// ## Examples
///
/// ```rust
/// use wasmtime::component::{ComponentType, Lower};
///
/// #[derive(ComponentType, Lower)]
/// #[component(record)]
/// struct Color {
///     r: u8,
///     g: u8,
///     b: u8,
/// }
/// ```
pub use wasmtime_component_macro::Lower;

/// A macro to generate a Rust type corresponding to WIT `flags`
///
/// This macro generates a type that implements the [`ComponentType`], [`Lift`],
/// and [`Lower`] traits. The generated Rust type corresponds to the `flags`
/// type in WIT.
///
/// Example usage of this looks like:
///
/// ```rust
/// use wasmtime::component::flags;
///
/// flags! {
///     Permissions {
///         #[component(name = "read")]
///         const READ;
///         #[component(name = "write")]
///         const WRITE;
///         #[component(name = "execute")]
///         const EXECUTE;
///     }
/// }
///
/// fn validate_permissions(permissions: &mut Permissions) {
///     if permissions.contains(Permissions::EXECUTE | Permissions::WRITE) {
///         panic!("cannot enable both writable and executable at the same time");
///     }
///
///     if permissions.contains(Permissions::READ) {
///         panic!("permissions must at least contain read");
///     }
/// }
/// ```
///
/// which corresponds to the WIT type:
///
/// ```wit
/// flags permissions {
///     read,
///     write,
///     execute,
/// }
/// ```
///
/// This generates a structure which is similar to/inspired by the [`bitflags`
/// crate](https://crates.io/crates/bitflags). The `Permissions` structure
/// generated implements the [`PartialEq`], [`Eq`], [`Debug`], [`BitOr`],
/// [`BitOrAssign`], [`BitAnd`], [`BitAndAssign`], [`BitXor`], [`BitXorAssign`],
/// and [`Not`] traits - in addition to the Wasmtime-specific component ones
/// [`ComponentType`], [`Lift`], and [`Lower`].
///
/// [`BitOr`]: std::ops::BitOr
/// [`BitOrAssign`]: std::ops::BitOrAssign
/// [`BitAnd`]: std::ops::BitAnd
/// [`BitAndAssign`]: std::ops::BitAndAssign
/// [`BitXor`]: std::ops::BitXor
/// [`BitXorAssign`]: std::ops::BitXorAssign
/// [`Not`]: std::ops::Not
pub use wasmtime_component_macro::flags;

#[cfg(any(docsrs, test, doctest))]
pub mod bindgen_examples;

// NB: needed for the links in the docs above to work in all `cargo doc`
// configurations and avoid errors.
#[cfg(not(any(docsrs, test, doctest)))]
#[doc(hidden)]
pub mod bindgen_examples {}
