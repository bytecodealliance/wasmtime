//! # Wasmtime's embedding API
//!
//! Wasmtime is a WebAssembly engine for JIT-compiled or ahead-of-time compiled
//! WebAssembly modules and components. More information about the Wasmtime
//! project as a whole can be found [in the documentation
//! book](https://docs.wasmtime.dev) whereas this documentation mostly focuses
//! on the API reference of the `wasmtime` crate itself.
//!
//! This crate contains an API used to interact with [WebAssembly modules] or
//! [WebAssembly components]. For example you can compile WebAssembly, create
//! instances, call functions, etc. As an embedder of WebAssembly you can also
//! provide guests functionality from the host by creating host-defined
//! functions, memories, globals, etc, which can do things that WebAssembly
//! cannot (such as print to the screen).
//!
//! [WebAssembly modules]: https://webassembly.github.io/spec
//! [WebAssembly components]: https://component-model.bytecodealliance.org
//!
//! The `wasmtime` crate is designed to be safe, efficient, and ergonomic.
//! This enables executing WebAssembly without the embedder needing to use
//! `unsafe` code, meaning that you're guaranteed there is no undefined behavior
//! or segfaults in either the WebAssembly guest or the host itself.
//!
//! The `wasmtime` crate can roughly be thought of as being split into two
//! halves:
//!
//! * One half of the crate is similar to the [JS WebAssembly
//!   API](https://developer.mozilla.org/en-US/docs/WebAssembly) as well as the
//!   [proposed C API](https://github.com/webassembly/wasm-c-api) and is
//!   intended for working with [WebAssembly modules]. This API resides in the
//!   root of the `wasmtime` crate's namespace, for example
//!   [`wasmtime::Module`](`Module`).
//!
//! * The second half of the crate is for use with the [WebAssembly Component
//!   Model]. The implementation of the component model is present in
//!   [`wasmtime::component`](`component`) and roughly mirrors the structure for
//!   core WebAssembly, for example [`component::Func`] mirrors [`Func`].
//!
//! [WebAssembly Component Model]: https://component-model.bytecodealliance.org
//!
//! An example of using Wasmtime to run a core WebAssembly module looks like:
//!
//! ```
//! use wasmtime::*;
//!
//! fn main() -> wasmtime::Result<()> {
//!     let engine = Engine::default();
//!
//!     // Modules can be compiled through either the text or binary format
//!     let wat = r#"
//!         (module
//!             (import "host" "host_func" (func $host_hello (param i32)))
//!
//!             (func (export "hello")
//!                 i32.const 3
//!                 call $host_hello)
//!         )
//!     "#;
//!     let module = Module::new(&engine, wat)?;
//!
//!     // Host functionality can be arbitrary Rust functions and is provided
//!     // to guests through a `Linker`.
//!     let mut linker = Linker::new(&engine);
//!     linker.func_wrap("host", "host_func", |caller: Caller<'_, u32>, param: i32| {
//!         println!("Got {} from WebAssembly", param);
//!         println!("my host state is: {}", caller.data());
//!     })?;
//!
//!     // All wasm objects operate within the context of a "store". Each
//!     // `Store` has a type parameter to store host-specific data, which in
//!     // this case we're using `4` for.
//!     let mut store: Store<u32> = Store::new(&engine, 4);
//!
//!     // Instantiation of a module requires specifying its imports and then
//!     // afterwards we can fetch exports by name, as well as asserting the
//!     // type signature of the function with `get_typed_func`.
//!     let instance = linker.instantiate(&mut store, &module)?;
//!     let hello = instance.get_typed_func::<(), ()>(&mut store, "hello")?;
//!
//!     // And finally we can call the wasm!
//!     hello.call(&mut store, ())?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Core Concepts
//!
//! There are a number of core types and concepts that are important to be aware
//! of when using the `wasmtime` crate:
//!
//! * [`Engine`] - a global compilation and runtime environment for WebAssembly.
//!   An [`Engine`] is an object that can be shared concurrently across threads
//!   and is created with a [`Config`] with many knobs for configuring
//!   behavior. Compiling or executing any WebAssembly requires first
//!   configuring and creating an [`Engine`]. All [`Module`]s and
//!   [`Component`](component::Component)s belong to an [`Engine`], and
//!   typically there's one [`Engine`] per process.
//!
//! * [`Store`] - container for all information related to WebAssembly objects
//!   such as functions, instances, memories, etc. A [`Store<T>`][`Store`]
//!   allows customization of the `T` to store arbitrary host data within a
//!   [`Store`]. This host data can be accessed through host functions via the
//!   [`Caller`] function parameter in host-defined functions. A [`Store`] is
//!   required for all WebAssembly operations, such as calling a wasm function.
//!   The [`Store`] is passed in as a "context" to methods like [`Func::call`].
//!   Dropping a [`Store`] will deallocate all memory associated with
//!   WebAssembly objects within the [`Store`]. A [`Store`] is cheap to create
//!   and destroy and does not GC objects such as unused instances internally,
//!   so it's intended to be short-lived (or no longer than the instances it
//!   contains).
//!
//! * [`Linker`] (or [`component::Linker`]) - host functions are defined within
//!   a linker to provide them a string-based name which can be looked up when
//!   instantiating a WebAssembly module or component. Linkers are traditionally
//!   populated at startup and then reused for all future instantiations of all
//!   instances, assuming the set of host functions does not change over time.
//!   Host functions are `Fn(..) + Send + Sync` and typically do not close over
//!   mutable state. Instead it's recommended to store mutable state in the `T`
//!   of [`Store<T>`] which is accessed through [`Caller<'_,
//!   T>`](crate::Caller) provided to host functions.
//!
//! * [`Module`] (or [`Component`](component::Component)) - a compiled
//!   WebAssembly module or component. These structures contain compiled
//!   executable code from a WebAssembly binary which is ready to execute after
//!   being instantiated. These are expensive to create as they require
//!   compilation of the input WebAssembly. Modules and components are safe to
//!   share across threads, however. Modules and components can additionally be
//!   [serialized into a list of bytes](crate::Module::serialize) to later be
//!   [deserialized](crate::Module::deserialize) quickly. This enables JIT-style
//!   compilation through constructors such as [`Module::new`] and AOT-style
//!   compilation by having the compilation process use [`Module::serialize`]
//!   and the execution process use [`Module::deserialize`].
//!
//! * [`Instance`] (or [`component::Instance`]) - an instantiated WebAssembly
//!   module or component. An instance is where you can actually acquire a
//!   [`Func`] (or [`component::Func`]) from, for example, to call.
//!
//! * [`Func`] (or [`component::Func`]) - a WebAssembly function. This can be
//!   acquired as the export of an [`Instance`] to call WebAssembly functions,
//!   or it can be created via functions like [`Func::wrap`] to wrap
//!   host-defined functionality and give it to WebAssembly. Functions also have
//!   typed views as [`TypedFunc`] or [`component::TypedFunc`] for a more
//!   efficient calling convention.
//!
//! * [`Table`], [`Global`], [`Memory`], [`component::Resource`] - other
//!   WebAssembly objects which can either be defined on the host or in wasm
//!   itself (via instances). These all have various ways of being interacted
//!   with like [`Func`].
//!
//! All "store-connected" types such as [`Func`], [`Memory`], etc, require the
//! store to be passed in as a context to each method. Methods in wasmtime
//! frequently have their first parameter as either [`impl
//! AsContext`][`AsContext`] or [`impl AsContextMut`][`AsContextMut`]. These
//! traits are implemented for a variety of types, allowing you to, for example,
//! pass the following types into methods:
//!
//! * `&Store<T>`
//! * `&mut Store<T>`
//! * `&Caller<'_, T>`
//! * `&mut Caller<'_, T>`
//! * `StoreContext<'_, T>`
//! * `StoreContextMut<'_, T>`
//!
//! A [`Store`] is the sole owner of all WebAssembly internals. Types like
//! [`Func`] point within the [`Store`] and require the [`Store`] to be provided
//! to actually access the internals of the WebAssembly function, for instance.
//!
//! ## WASI
//!
//! The `wasmtime` crate does not natively provide support for WASI, but you can
//! use the [`wasmtime-wasi`] crate for that purpose. With [`wasmtime-wasi`] all
//! WASI functions can be added to a [`Linker`] and then used to instantiate
//! WASI-using modules. For more information see the [WASI example in the
//! documentation](https://docs.wasmtime.dev/examples-rust-wasi.html).
//!
//! [`wasmtime-wasi`]: https://crates.io/crates/wasmtime-wasi
//!
//! ## Async
//!
//! Wasmtime supports executing WebAssembly guests through Rust-level `async`
//! functions. This enables Wasmtime to block the guest without blocking the
//! host, interrupt infinite loops or long-running CPU-bound guests, and
//! integrate with Rust host functions that are themselves `async`.
//!
//! Many functions in the embedding API have a sync variant and an async
//! variant, for example [`Func::call`] and [`Func::call_async`]. Embedders
//! may decide which is most appropriate for their use case, but if certain
//! features of Wasmtime are configured then `*_async` variants of functions are
//! required. If any of these features are used, for example, then `*_async`
//! must be used:
//!
//! * Async core wasm host functions, for example via [`Linker::func_wrap_async`]
//! * Async component host functions, for example via [`component::LinkerInstance::func_wrap_async`]
//! * Async resource limiters, via [`Store::limiter_async`]
//! * Async yields with fuel via [`Store::fuel_async_yield_interval`]
//! * Async yields via epochs via [`Store::epoch_deadline_async_yield_and_update`]
//!
//! This is not an exhaustive list, but if any of these configurations/APIs are
//! used then all `*_async` APIs must be used in Wasmtime. If synchronous APIs
//! are used instead they will return an error.
//!
//! #### Asynchronous Wasm
//!
//! Core WebAssembly and synchronous WIT functions (e.g. WASIp2-and-prior)
//! require that all imported functions appear synchronous from the perspective
//! of the guest. Host functions which perform I/O and block, however, are often
//! defined with `async` in Rust. Wasmtime's async support bridges this gap with
//! asynchronous wasm execution.
//!
//! When using `*_async` APIs to execute WebAssembly Wasmtime will always
//! represent its computation as a [`Future`]. The `poll` method of the futures
//! returned by Wasmtime will perform the actual work of calling the
//! WebAssembly. Wasmtime won't manage its own thread pools or similar, that's
//! left up to the embedder.
//!
//! To implement futures in a way that WebAssembly sees asynchronous host
//! functions as synchronous, all async Wasmtime futures will execute on a
//! separately allocated native stack from the thread otherwise executing
//! Wasmtime. This separate native stack can then be switched to and from.
//! Using this whenever an `async` host function returns a future that
//! resolves to `Pending` we switch away from the temporary stack back to
//! the main stack and propagate the `Pending` status.
//!
//! #### Execution in `poll`
//!
//! The [`Future::poll`] method is the main driving force behind Rust's futures.
//! That method's own documentation states "an implementation of `poll` should
//! strive to return quickly, and should not block". This, however, can be at
//! odds with executing WebAssembly code as part of the `poll` method itself. If
//! your WebAssembly is untrusted then this could allow the `poll` method to
//! take arbitrarily long in the worst case, likely blocking all other
//! asynchronous tasks.
//!
//! To remedy this situation you have a few possible ways to solve this:
//!
//! * The most efficient solution is to enable
//!   [`Config::epoch_interruption`] in conjunction with
//!   [`crate::Store::epoch_deadline_async_yield_and_update`]. Coupled with
//!   periodic calls to [`crate::Engine::increment_epoch`] this will cause
//!   executing WebAssembly to periodically yield back according to the
//!   epoch configuration settings. This enables [`Future::poll`] to take at
//!   most a certain amount of time according to epoch configuration
//!   settings and when increments happen. The benefit of this approach is
//!   that the instrumentation in compiled code is quite lightweight, but a
//!   downside can be that the scheduling is somewhat nondeterministic since
//!   increments are usually timer-based which are not always deterministic.
//!
//!   Note that to prevent infinite execution of wasm it's recommended to
//!   place a timeout on the entire future representing executing wasm code
//!   and the periodic yields with epochs should ensure that when the
//!   timeout is reached it's appropriately recognized.
//!
//! * Alternatively you can enable the
//!   [`Config::consume_fuel`](crate::Config::consume_fuel) method as well
//!   as [`crate::Store::fuel_async_yield_interval`] When doing so this will
//!   configure Wasmtime futures to yield periodically while they're
//!   executing WebAssembly code. After consuming the specified amount of
//!   fuel wasm futures will return `Poll::Pending` from their `poll`
//!   method, and will get automatically re-polled later. This enables the
//!   `Future::poll` method to take roughly a fixed amount of time since
//!   fuel is guaranteed to get consumed while wasm is executing. Unlike
//!   epoch-based preemption this is deterministic since wasm always
//!   consumes a fixed amount of fuel per-operation. The downside of this
//!   approach, however, is that the compiled code instrumentation is
//!   significantly more expensive than epoch checks.
//!
//!   Note that to prevent infinite execution of wasm it's recommended to
//!   place a timeout on the entire future representing executing wasm code
//!   and the periodic yields with epochs should ensure that when the
//!   timeout is reached it's appropriately recognized.
//!
//! In all cases special care needs to be taken when integrating
//! asynchronous wasm into your application. You should carefully plan where
//! WebAssembly will execute and what compute resources will be allotted to
//! it. If Wasmtime doesn't support exactly what you'd like just yet, please
//! feel free to open an issue!
//!
//! ## Crate Features
//!
//! The `wasmtime` crate comes with a number of compile-time features that can
//! be used to customize what features it supports. Some of these features are
//! just internal details, but some affect the public API of the `wasmtime`
//! crate. Wasmtime APIs gated behind a Cargo feature should be indicated as
//! such in the documentation.
//!
//! * `runtime` - Enabled by default, this feature enables executing
//!   WebAssembly modules and components. If a compiler is not available (such
//!   as `cranelift`) then [`Module::deserialize`] must be used, for example, to
//!   provide an ahead-of-time compiled artifact to execute WebAssembly.
//!
//! * `cranelift` - Enabled by default, this features enables using Cranelift at
//!   runtime to compile a WebAssembly module to native code. This feature is
//!   required to process and compile new WebAssembly modules and components.
//!
//! * `cache` - Enabled by default, this feature adds support for wasmtime to
//!   perform internal caching of modules in a global location. This must still
//!   be enabled explicitly through [`Config::cache`].
//!
//! * `wat` - Enabled by default, this feature adds support for accepting the
//!   text format of WebAssembly in [`Module::new`] and
//!   [`Component::new`](component::Component::new). The text format will be
//!   automatically recognized and translated to binary when compiling a
//!   module.
//!
//! * `parallel-compilation` - Enabled by default, this feature enables support
//!   for compiling functions in parallel with `rayon`.
//!
//! * `async` - Enabled by default, this feature enables APIs and runtime
//!   support for defining asynchronous host functions and calling WebAssembly
//!   asynchronously. For more information see [async documentation](#async)
//!
//! * `profiling` - Enabled by default, this feature compiles in support for
//!   profiling guest code via a number of possible strategies. See
//!   [`Config::profiler`] for more information.
//!
//! * `all-arch` - Not enabled by default. This feature compiles in support for
//!   all architectures for both the JIT compiler and the `wasmtime compile` CLI
//!   command. This can be combined with [`Config::target`] to precompile
//!   modules for a different platform than the host.
//!
//! * `pooling-allocator` - Enabled by default, this feature adds support for
//!   [`PoolingAllocationConfig`] to pass to [`Config::allocation_strategy`].
//!   The pooling allocator can enable efficient reuse of resources for
//!   high-concurrency and high-instantiation-count scenarios.
//!
//! * `demangle` - Enabled by default, this will affect how backtraces are
//!   printed and whether symbol names from WebAssembly are attempted to be
//!   demangled. Rust and C++ demanglings are currently supported.
//!
//! * `coredump` - Enabled by default, this will provide support for generating
//!   a core dump when a trap happens. This can be configured via
//!   [`Config::coredump_on_trap`].
//!
//! * `addr2line` - Enabled by default, this feature configures whether traps
//!   will attempt to parse DWARF debug information and convert WebAssembly
//!   addresses to source filenames and line numbers.
//!
//! * `debug-builtins` - Enabled by default, this feature includes some built-in
//!   debugging utilities and symbols for native debuggers such as GDB and LLDB
//!   to attach to the process Wasmtime is used within. The intrinsics provided
//!   will enable debugging guest code compiled to WebAssembly. This must also
//!   be enabled via [`Config::debug_info`] as well for guests.
//!
//! * `component-model` - Enabled by default, this enables support for the
//!   [`wasmtime::component`](component) API for working with components.
//!
//! * `gc` - Enabled by default, this enables support for a number of
//!   WebAssembly proposals such as `reference-types`, `function-references`,
//!   and `gc`. Note that the implementation of the `gc` proposal itself is not
//!   yet complete at this time.
//!
//! * `threads` - Enabled by default, this enables compile-time support for the
//!   WebAssembly `threads` proposal, notably shared memories.
//!
//! * `call-hook` - Disabled by default, this enables support for the
//!   [`Store::call_hook`] API. This incurs a small overhead on all
//!   entries/exits from WebAssembly and may want to be disabled by some
//!   embedders.
//!
//! * `memory-protection-keys` - Disabled by default, this enables support for
//!   the [`PoolingAllocationConfig::memory_protection_keys`] API. This feature
//!   currently only works on x64 Linux and can enable compacting the virtual
//!   memory allocation for linear memories in the pooling allocator. This comes
//!   with the same overhead as the `call-hook` feature where entries/exits into
//!   WebAssembly will have more overhead than before.
//!
//! * `signals-based-traps` - Enabled by default, this enables support for using
//!   host signal handlers to implement WebAssembly traps. For example virtual
//!   memory is used to catch out-of-bounds accesses in WebAssembly that result
//!   in segfaults. This is implicitly enabled by the `std` feature and is the
//!   best way to get high-performance WebAssembly.
//!
//! More crate features can be found in the [manifest] of Wasmtime itself for
//! seeing what can be enabled and disabled.
//!
//! [manifest]: https://github.com/bytecodealliance/wasmtime/blob/main/crates/wasmtime/Cargo.toml

#![deny(missing_docs)]
#![doc(test(attr(deny(warnings))))]
#![doc(test(attr(allow(dead_code, unused_variables, unused_mut))))]
#![cfg_attr(docsrs, feature(doc_cfg))]
// NB: this list is currently being burned down to remove all features listed
// here to get warnings in all configurations of Wasmtime.
#![cfg_attr(
    any(not(feature = "runtime"), not(feature = "std")),
    expect(dead_code, unused_imports, reason = "list not burned down yet")
)]
// Allow broken links when the default features is disabled because most of our
// documentation is written for the "one build" of the `main` branch which has
// most features enabled. This will present warnings in stripped-down doc builds
// and will prevent the doc build from failing.
#![cfg_attr(feature = "default", warn(rustdoc::broken_intra_doc_links))]
#![no_std]
// Wasmtime liberally uses #[cfg]'d definitions of structures to uninhabited
// types to reduce the total amount of #[cfg], but rustc warns that much usage
// of these structures, rightfully, leads to unreachable code. This unreachable
// code is only conditional, however, so it's generally just annoying to deal
// with. Disable the `unreachable_code` lint in situations like this when some
// major features are disabled. If all the features are enabled, though, we
// still want to get warned about this.
#![cfg_attr(
    any(not(feature = "threads"), not(feature = "gc",)),
    allow(unreachable_code, reason = "see comment")
)]

#[cfg(feature = "std")]
#[macro_use]
extern crate std;
extern crate alloc;

// Internal `use` statement which isn't used in this module but enable
// `use crate::prelude::*;` everywhere else within this crate, for example.
use wasmtime_environ::prelude;

// FIXME(#12069) should transition to OOM-handling versions of these collections
// for all internal usage instead of using abort-on-OOM versions. Once that's
// done this can be removed and the collections should be directly imported from
// `wasmtime_environ::collections::*`.
use wasmtime_environ::collections::oom_abort::{hash_map, hash_set};

/// A helper macro to safely map `MaybeUninit<T>` to `MaybeUninit<U>` where `U`
/// is a field projection within `T`.
///
/// This is intended to be invoked as:
///
/// ```ignore
/// struct MyType {
///     field: u32,
/// }
///
/// let initial: &mut MaybeUninit<MyType> = ...;
/// let field: &mut MaybeUninit<u32> = map_maybe_uninit!(initial.field);
/// ```
///
/// Note that array accesses are also supported:
///
/// ```ignore
///
/// let initial: &mut MaybeUninit<[u32; 2]> = ...;
/// let element: &mut MaybeUninit<u32> = map_maybe_uninit!(initial[1]);
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! map_maybe_uninit {
    ($maybe_uninit:ident $($field:tt)*) => ({
        #[allow(unused_unsafe, reason = "macro-generated code")]
        {
            unsafe {
                use $crate::MaybeUninitExt;

                let m: &mut core::mem::MaybeUninit<_> = $maybe_uninit;
                // Note the usage of `&raw` here which is an attempt to "stay
                // safe" here where we never accidentally create `&mut T` where `T` is
                // actually uninitialized, hopefully appeasing the Rust unsafe
                // guidelines gods.
                m.map(|p| &raw mut (*p)$($field)*)
            }
        }
    })
}

#[doc(hidden)]
pub trait MaybeUninitExt<T> {
    /// Maps `MaybeUninit<T>` to `MaybeUninit<U>` using the closure provided.
    ///
    /// # Safety
    ///
    /// Requires that `*mut U` is a field projection from `*mut T`. Use
    /// `map_maybe_uninit!` above instead.
    unsafe fn map<U>(&mut self, f: impl FnOnce(*mut T) -> *mut U)
    -> &mut core::mem::MaybeUninit<U>;
}

impl<T> MaybeUninitExt<T> for core::mem::MaybeUninit<T> {
    unsafe fn map<U>(
        &mut self,
        f: impl FnOnce(*mut T) -> *mut U,
    ) -> &mut core::mem::MaybeUninit<U> {
        let new_ptr = f(self.as_mut_ptr());
        // SAFETY: the memory layout of these two types are the same, and
        // asserting that it's a safe reference with the same lifetime as `self`
        // is a requirement of this function itself.
        unsafe { core::mem::transmute::<*mut U, &mut core::mem::MaybeUninit<U>>(new_ptr) }
    }
}

#[cfg(feature = "runtime")]
mod runtime;
#[cfg(feature = "runtime")]
pub use runtime::*;

#[cfg(any(feature = "cranelift", feature = "winch"))]
mod compile;
#[cfg(any(feature = "cranelift", feature = "winch"))]
pub use compile::{CodeBuilder, CodeHint};

mod config;
mod engine;
mod profiling_agent;

pub use crate::config::*;
pub use crate::engine::*;

#[cfg(feature = "std")]
mod sync_std;
#[cfg(feature = "std")]
use sync_std as sync;

mod sync_nostd;
#[cfg(not(feature = "std"))]
use sync_nostd as sync;

#[doc(inline)]
pub use wasmtime_environ::error;

#[cfg(feature = "anyhow")]
pub use wasmtime_environ::ToWasmtimeResult;

// Only for use in `bindgen!`-generated code.
#[doc(hidden)]
#[cfg(feature = "anyhow")]
pub use wasmtime_environ::anyhow;

pub use self::error::{Error, Result, bail, ensure, format_err};

/// A re-exported instance of Wasmtime's `wasmparser` dependency.
///
/// This may be useful for embedders that also use `wasmparser`
/// directly: it allows embedders to ensure that they are using the same
/// version as Wasmtime, both to eliminate redundant dependencies on
/// multiple versions of the library, and to ensure compatibility in
/// validation and feature support.
///
/// Note that this re-export is *not subject to semver*: we reserve the
/// right to make patch releases of Wasmtime that bump the version of
/// wasmparser used, and hence the version re-exported, in
/// semver-incompatible ways. This is the tradeoff that the embedder
/// needs to opt into: in order to stay exactly in sync with an internal
/// detail of Wasmtime, the cost is visibility into potential internal
/// version changes.
#[cfg(feature = "reexport-wasmparser")]
pub use wasmparser;

fn _assert_send_and_sync<T: Send + Sync>() {}

fn _assertions_lib() {
    _assert_send_and_sync::<Engine>();
    _assert_send_and_sync::<Config>();
}

#[cfg(feature = "runtime")]
#[doc(hidden)]
pub mod _internal {
    // Exported just for the CLI.
    pub use crate::runtime::vm::MmapVec;
}
