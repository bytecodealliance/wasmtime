//! In-progress implementation of the WebAssembly component model
//!
//! This module is a work-in-progress and currently represents an incomplete and
//! probably buggy implementation of the component model.

#![cfg_attr(nightlydoc, doc(cfg(feature = "component-model")))]

mod component;
mod func;
mod instance;
mod linker;
mod matching;
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
pub use self::resources::{Resource, ResourceAny};
pub use self::types::{ResourceType, Type};
pub use self::values::{
    Enum, Flags, List, OptionVal, Record, ResultVal, Tuple, Union, Val, Variant,
};
pub use wasmtime_component_macro::{flags, ComponentType, Lift, Lower};

// These items are expected to be used by an eventual
// `#[derive(ComponentType)]`, they are not part of Wasmtime's API stability
// guarantees
#[doc(hidden)]
pub mod __internal {
    pub use super::func::{
        bad_type_info, format_flags, lower_payload, typecheck_enum, typecheck_flags,
        typecheck_record, typecheck_union, typecheck_variant, ComponentVariant, LiftContext,
        LowerContext, MaybeUninitExt, Options,
    };
    pub use super::matching::InstanceType;
    pub use crate::map_maybe_uninit;
    pub use crate::store::StoreOpaque;
    pub use anyhow;
    #[cfg(feature = "async")]
    pub use async_trait::async_trait;
    pub use wasmtime_environ;
    pub use wasmtime_environ::component::{CanonicalAbiInfo, ComponentTypes, InterfaceType};
}

pub(crate) use self::store::ComponentStoreData;

/// Generate bindings for a WIT package.
///
/// This macro ingests a [WIT package] and will generate all the necessary
/// bindings for instantiating and invoking a particular `world` in the
/// package. A `world` in a WIT package is a description of imports and exports
/// for a component. This provides a higher-level representation of working with
/// a component than the raw [`Instance`] type which must be manually-type-check
/// and manually have its imports provided via the [`Linker`] type.
///
/// The most basic usage of this macro is:
///
/// ```rust,ignore
/// wasmtime::component::bindgen!("my-component");
/// ```
///
/// This will parse your projects WIT package in a `wit` directory adjacent to
/// your crate's `Cargo.toml`. All of the `*.wit` files in that directory are
/// parsed and then the `default world` will be looked up within
/// `my-component.wit`. This world is then used as the basis for generating
/// bindings.
///
/// For example if your project contained:
///
/// ```text,ignore
/// // wit/my-component.wit
///
/// package my:project
///
/// world hello-world {
///     import name: func() -> string
///     export greet: func()
/// }
/// ```
///
/// Then you can interact with the generated bindings like so:
///
/// ```rust,ignore
/// use wasmtime::component::*;
/// use wasmtime::{Config, Engine, Store};
///
/// bindgen!();
///
/// struct MyState {
///     name: String,
/// }
///
/// // Imports into the world, like the `name` import for this world, are satisfied
/// // through traits.
/// impl HelloWorldImports for MyState {
///     // Note the `Result` return value here where `Ok` is returned back to
///     // the component and `Err` will raise a trap.
///     fn name(&mut self) -> wasmtime::Result<String> {
///         Ok(self.name.clone())
///     }
/// }
///
/// fn main() -> wasmtime::Result<()> {
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
/// package my:project
///
/// interface host {
///     gen-random-integer: func() -> u32
///     sha256: func(bytes: list<u8>) -> string
/// }
///
/// default world hello-world {
///     import host
///
///     export demo: interface {
///         run: func()
///     }
/// }
/// ```
///
/// Then you can interact with the generated bindings like so:
///
/// ```rust,ignore
/// use wasmtime::component::*;
/// use wasmtime::{Config, Engine, Store};
/// use my::project::host::Host;
///
/// bindgen!();
///
/// struct MyState {
///     // ...
/// }
///
/// // Note that the trait here is per-interface and within a submodule now.
/// impl Host for MyState {
///     fn gen_random_integer(&mut self) -> wasmtime::Result<u32> {
///         Ok(rand::thread_rng().gen())
///     }
///
///     fn sha256(&mut self, bytes: Vec<u8>) -> wasmtime::Result<String> {
///         // ...
///     }
/// }
///
/// fn main() -> wasmtime::Result<()> {
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
/// Basic usage of this macro looks like:
///
/// ```rust,ignore
/// // Parse the `wit/` folder adjacent to this crate's `Cargo.toml` and look
/// // for a `default world` in its documents. There must be exactly one
/// // `default world` for this to succeed.
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
///         package my:inline
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
///     // This can be used to translate WIT return values of the form
///     // `result<T, error-type>` into `Result<T, RustErrorType>` in Rust.
///     // The `RustErrorType` structure will have an automatically generated
///     // implementation of `From<ErrorType> for RustErrorType`. The
///     // `RustErrorType` additionally can also represent a trap to
///     // conveniently flatten all errors into one container.
///     //
///     // By default this option is not specified.
///     trappable_error_type: {
///         interface::ErrorType: RustErrorType,
///     },
///
///     // Restrict the code generated to what's needed for the interface
///     // imports in the inlined WIT document fragment.
///     interfaces: "
///         import package.foo
///     ",
///
///     // Remap interface names to module names, imported from elsewhere.
///     // Using this option will prevent any code from being generated
///     // for the names mentioned in the mapping, assuming instead that the
///     // names mentioned come from a previous use of the `bindgen!` macro
///     // with `only_interfaces: true`.
///     with: {
///         "a": somewhere::else::a,
///     },
/// });
/// ```
///
/// [WIT package]: https://github.com/WebAssembly/component-model/blob/main/design/mvp/WIT.md
pub use wasmtime_component_macro::bindgen;
