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
pub use self::component::Component;
pub use self::func::{
    ComponentNamedList, ComponentType, Func, Lift, Lower, TypedFunc, WasmList, WasmStr,
};
pub use self::instance::{ExportInstance, Exports, Instance, InstancePre};
pub use self::linker::{Linker, LinkerInstance};
pub use self::resource_table::{ResourceTable, ResourceTableError};
pub use self::resources::{Resource, ResourceAny};
pub use self::types::{ResourceType, Type};
pub use self::values::Val;

pub(crate) use self::resources::HostResourceData;

// These items are expected to be used by an eventual
// `#[derive(ComponentType)]`, they are not part of Wasmtime's API stability
// guarantees
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
    #[cfg(feature = "async")]
    pub use async_trait::async_trait;
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
/// The most basic usage of this macro is:
///
/// ```rust,ignore
/// wasmtime::component::bindgen!();
/// ```
///
/// This will parse your projects [WIT package] in a `wit` directory adjacent to
/// your crate's `Cargo.toml`. All of the `*.wit` files in that directory are
/// parsed and then the single `world` found will be used for bindings.
///
/// For example if your project contained:
///
/// ```text,ignore
/// // wit/my-component.wit
///
/// package my:project;
///
/// world hello-world {
///     import name: func() -> string;
///     export greet: func();
/// }
/// ```
///
/// Then you can interact with the generated bindings like so:
///
/// ```rust
/// use wasmtime::component::*;
/// use wasmtime::{Config, Engine, Store};
///
/// # const _: () = { macro_rules! bindgen { () => () }
/// bindgen!();
/// # };
/// # bindgen!({
/// #   inline: r#"
/// #       package my:project;
/// #       world hello-world {
/// #           import name: func() -> string;
/// #           export greet: func();
/// #       }
/// #   "#,
/// # });
///
/// struct MyState {
///     name: String,
/// }
///
/// // Imports into the world, like the `name` import for this world, are
/// // satisfied through traits.
/// impl HelloWorldImports for MyState {
///     // Note the `Result` return value here where `Ok` is returned back to
///     // the component and `Err` will raise a trap.
///     fn name(&mut self) -> String {
///         self.name.clone()
///     }
/// }
///
/// fn main() -> wasmtime::Result<()> {
/// #   if true { return Ok(()) }
///     // Configure an `Engine` and compile the `Component` that is being run for
///     // the application.
///     let mut config = Config::new();
///     config.wasm_component_model(true);
///     let engine = Engine::new(&config)?;
///     let component = Component::from_file(&engine, "./your-component.wasm")?;
///
///     // Instantiation of bindings always happens through a `Linker`.
///     // Configuration of the linker is done through a generated `add_to_linker`
///     // method on the bindings structure.
///     //
///     // Note that the closure provided here is a projection from `T` in
///     // `Store<T>` to `&mut U` where `U` implements the `HelloWorldImports`
///     // trait. In this case the `T`, `MyState`, is stored directly in the
///     // structure so no projection is necessary here.
///     let mut linker = Linker::new(&engine);
///     HelloWorld::add_to_linker(&mut linker, |state: &mut MyState| state)?;
///
///     // As with the core wasm API of Wasmtime instantiation occurs within a
///     // `Store`. The bindings structure contains an `instantiate` method which
///     // takes the store, component, and linker. This returns the `bindings`
///     // structure which is an instance of `HelloWorld` and supports typed access
///     // to the exports of the component.
///     let mut store = Store::new(
///         &engine,
///         MyState {
///             name: "me".to_string(),
///         },
///     );
///     let (bindings, _) = HelloWorld::instantiate(&mut store, &component, &linker)?;
///
///     // Here our `greet` function doesn't take any parameters for the component,
///     // but in the Wasmtime embedding API the first argument is always a `Store`.
///     bindings.call_greet(&mut store)?;
///     Ok(())
/// }
/// ```
///
/// The function signatures within generated traits and on generated exports
/// match the component-model signatures as specified in the WIT `world` item.
/// Note that WIT also has support for importing and exports interfaces within
/// worlds, which can be bound here as well:
///
/// For example this WIT input
///
/// ```text,ignore
/// // wit/my-component.wit
///
/// package my:project;
///
/// interface host {
///     gen-random-integer: func() -> u32;
///     sha256: func(bytes: list<u8>) -> string;
/// }
///
/// world hello-world {
///     import host;
///
///     export demo: interface {
///         run: func();
///     }
/// }
/// ```
///
/// Then you can interact with the generated bindings like so:
///
/// ```rust
/// use wasmtime::component::*;
/// use wasmtime::{Config, Engine, Store};
/// use my::project::host::Host;
///
/// # const _: () = { macro_rules! bindgen { () => () }
/// bindgen!();
/// # };
/// # bindgen!({
/// #   inline: r#"
/// # package my:project;
/// #
/// # interface host {
/// #     gen-random-integer: func() -> u32;
/// #     sha256: func(bytes: list<u8>) -> string;
/// # }
/// #
/// # world hello-world {
/// #     import host;
/// #
/// #     export demo: interface {
/// #         run: func();
/// #     }
/// # }
/// #   "#,
/// # });
///
/// struct MyState {
///     // ...
/// }
///
/// // Note that the trait here is per-interface and within a submodule now.
/// impl Host for MyState {
///     fn gen_random_integer(&mut self) -> u32 {
/// #       panic!();
/// #       #[cfg(FALSE)]
///         rand::thread_rng().gen()
///     }
///
///     fn sha256(&mut self, bytes: Vec<u8>) -> String {
///         // ...
/// #       panic!()
///     }
/// }
///
/// fn main() -> wasmtime::Result<()> {
/// #   if true { return Ok(()) }
///     let mut config = Config::new();
///     config.wasm_component_model(true);
///     let engine = Engine::new(&config)?;
///     let component = Component::from_file(&engine, "./your-component.wasm")?;
///
///     let mut linker = Linker::new(&engine);
///     HelloWorld::add_to_linker(&mut linker, |state: &mut MyState| state)?;
///
///     let mut store = Store::new(
///         &engine,
///         MyState { /* ... */ },
///     );
///     let (bindings, _) = HelloWorld::instantiate(&mut store, &component, &linker)?;
///
///     // Note that the `demo` method returns a `&Demo` through which we can
///     // run the methods on that interface.
///     bindings.demo().call_run(&mut store)?;
///     Ok(())
/// }
/// ```
///
/// The generated bindings can additionally be explored more fully with `cargo
/// doc` to see what types and traits and such are generated.
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
/// ```rust,ignore
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
/// ```
///
/// A more configured version of invoking this macro looks like:
///
/// ```rust,ignore
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
///     // to log arguments and return values.
///     //
///     // This option defaults to `false`.
///     tracing: true,
///
///     // Imports will be async functions through #[async_trait] and exports
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
///     // A list of WIT "features" to enable when parsing the WIT document that
///     // this bindgen macro matches. WIT features are all disabled by default
///     // and must be opted-in-to if source level features are used.
///     //
///     // This option defaults to an empty array.
///     features: ["foo", "bar", "baz"],
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
