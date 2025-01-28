# Conditional Compilation in Wasmtime

The `wasmtime` crate and workspace is both quite configurable in terms of
runtime configuration (e.g. `Config::*`) and compile-time configuration (Cargo
features). Wasmtime also wants to take advantage of native hardware features on
specific CPUs and operating systems to implement optimizations for executing
WebAssembly. This overall leads to the state where the source code for Wasmtime
has quite a lot of `#[cfg]` directives and is trying to wrangle the
combinatorial explosion of:

1. All possible CPU architectures that Wasmtime (or Rust) supports.
2. All possible operating systems that Wasmtime (or Rust) supports.
3. All possible feature combinations of the `wasmtime` crate.

Like any open source project one of the goals of Wasmtime is to have readable
and understandable code and to that effect we ideally don't have `#[cfg]`
everywhere throughout the codebase in confusing combinations. The goal of this
document is to explain the various guidelines we have for conditional
compilation in Rust and some recommended styles for working with `#[cfg]` in a
maintainable and scalable manner.

## Rust's `#[cfg]` attribute

If you haven't worked with Rust much before or if you'd like a refresher, Rust's
main ability to handle conditional compilation is the `#[cfg]` attribute. This
is semantically and structurally different from `#ifdef` in C/C++ and gives rise
to alternative patterns which look quite different as well.

One of the more common conditional compilation attributes in Rust is
`#[cfg(test)]` which enables including a module or a piece of code only when
compiled with `cargo test` (or `rustc`'s `--test` flag). There are many other
directives you can put in `#[cfg]`, however, for example:

* `#[cfg(target_arch = "...")]` - this can be used to detect the architecture
  that the code was compiled for.
* `#[cfg(target_os = "...")]` - this can be used to detect the operating system
  that the code was compiled for.
* `#[cfg(feature = "...")]` - these correspond to [Cargo features][cargo] and
  are enabled when depending on crates in `Cargo.toml`.
* `#[cfg(has_foo)]` - completely custom directives can be emitted by build
  scripts such as `crates/wasmtime/build.rs`.

[cargo]: https://doc.rust-lang.org/cargo/reference/features.html

To explore built-in `#[cfg]` directives you can use `rustc --print cfg` for your
host target. This also supports `rustc --print cfg --target ...`.

Finally, `#[cfg]` directive support internal "functions" such as `all(...)`,
`any(...)`, and `not(..)`.

Attributes in Rust can be applied to syntactic items in Rust, not fragments of
lexical tokens like C/C++. This means that conditional compilation happens at
the AST level rather than the lexical level. For example:

```rust,ignore
#[cfg(foo)]
fn the_function() { /* ... */ }

#[cfg(not(foo))]
fn the_function() { /* ... */ }
```

This can additionally be applied to entire expressions in Rust too:

```rust,ignore
fn the_function() {
    #[cfg(foo)]
    {
        // ...
    }
    #[cfg(not(foo))]
    {
        // ...
    }
}
```

The Rust compiler doesn't type-check or analyze anything in
conditionally-omitted code. It is only required to be syntactically valid.

## Hazards with `#[cfg]`

Conditional compilation in any language can get hairy quickly and Rust is no
exception. The venerable "`#ifdef` soup" one might have seen in C/C++ is very
much possible to have in Rust too in the sense that it won't look the same but
it'll still taste just as bad. In that sense it's worth going over some of the
downsides of `#[cfg]` in Rust and some hazards to watch out for.

**Unused Imports**

Conditional compilation can be great for quickly excluding an entire function in
one line, but this might have ramifications if that function was the only use of
an imported type for example:

```rust,ignore
use std::ptr::NonNull; //~ WARNING: unused import when `foo` is turned off

#[cfg(foo)]
fn my_function() -> NonNull<u8> {
    // ...
}
```

**Repetitive Attributes**

Enabling a Cargo feature can add features to existing types which means it can
lead to repetitive `#[cfg]` annotations such as:

```rust,ignore
#[cfg(feature = "async")]
use std::future::Future;

impl<T> Store<T> {
    #[cfg(feature = "async")]
    async fn some_new_async_api(&mut self) {
        // ...
    }

    #[cfg(feature = "async")]
    async fn some_other_new_async_api(&mut self) {
        // ...
    }
}

#[cfg(feature = "async")]
struct SomeAsyncHelperType {
    // ...
}

#[cfg(feature = "async")]
impl SomeAsyncHelperType {
    // ...
}
```

**Boilerplate throughout an implementation**

In addition to being repetitive when defining conditionally compiled code
there's also a risk of being quite repetitive when using conditionally compiled
code as well. In its most basic form any usage of a conditionally compiled piece
of code must additionally be gated as well.

```rust
#[cfg(feature = "gc")]
fn gc() {
    // ...
}

fn call_wasm() {
    #[cfg(feature = "gc")]
    gc();

    // do the call ...

    #[cfg(feature = "gc")]
    gc();
}
```

**Interactions with ecosystem tooling**

Conditionally compiled code does not always interact well with ecosystem tooling
in Rust. An example of this is the `cfg_if!` macro where `rustfmt` is unable to
format the contents of the macro. If there are conditionally defined modules in
the macro then it means `rustfmt` won't format any modules internally in the
macro either. Not a great experience!

Here neither `gc.rs` nor `gc_disabled.rs` will be formatted by `cargo fmt`.

```rust
cfg_if::cfg_if! {
    if #[cfg(feature = "gc")] {
        mod gc;
        use gc::*;
    } else {
        mod gc_disabled;
        use gc_disabled::*;
    }
}
```

**Combinatorial explosion in testing complexity**

Each crate feature can be turned on and off. Wasmtime supports a range of
platforms and architectures. It's practically infeasible to test every single
possible combination of these. This means that inevitably there are going to be
untested configurations as well as bugs within these configurations.

## Conditional Compilation Style Guide

With some of the basics out of the way, this is intended to document the rough
current state of Wasmtime and some various principles for writing conditionally
compiled code. Much of these are meant to address some of the hazards above.
These guidelines are not always religiously followed throughout Wasmtime's
repository but PRs to improve things are always welcome!

The main takeaway is that the main goal is **to minimize the number of `#[cfg]`
attributes necessary in the repository**. Conditional compilation is required no
matter what so this number can never be zero, but that doesn't mean every other
line should have `#[cfg]` on it. Otherwise these guidelines need to be applied
with some understanding that each one is fallible. There's no always-right
answer unfortunately and style will still differ from person to person.

1. **Separate files** - try to put conditionally compiled code into separate
   files. By placing `#[cfg]` at the module level you can drastically cut down
   on annotations by removing the entire module at once. An example of this is
   that Wasmtime's internal `runtime` module is [feature gated][file-gate] at
   the top-level.

2. **Only `#[cfg]` definitions, not uses** - try to only use `#[cfg]` when a
   type or function is defined, not when it's used. Functions and types can be
   used all over the place and putting a `#[cfg]` everywhere can be quite
   annoying an brittle to maintain.

   * This isn't a problem if a use-site is already contained in a
     `#[cfg]` item, such as a module. This can be assisted by lifting `#[cfg]`
     up "as high as possible". An example of this is Wasmtime's `component`
     module which uses `#[cfg(feature = "component-model")]` at the root of all
     component-related functionality. That means that conditionally included
     dependencies used within `component::*` don't need extra `#[cfg]` annotations.

   * Another common pattern for this is to conditionally define a "dummy" shim
     interface. The real implementation would live in `foo.rs` while the dummy
     implementation would live in `foo_disabled.rs`. That means that "foo" is
     always available but the dummy implementation doesn't do anything. This
     makes heavy use of zero-sized-types (e.g. `struct Foo;`) and uninhabited
     types (e.g. `enum Foo {}`) to ensure there is no runtime overhead. An
     example of this is [`shared_memory.rs`][dummy-enabled] and
     [`shared_memory_disabled.rs`][dummy-disabled] where the disabled version
     returns an error on construction and otherwise has trivial implementations
     of each method.

3. **Off-by-default code should be trivial** - described above it's not possible
   to test Wasmtime in every possible configuration of `#[cfg]`, so to help
   reduce the risk of lurking bugs try to ensure that all off-by-default code is
   trivially correct-by-construction. For "dummy" shims described above this
   means that methods do nothing or return an error. If off-by-default code is
   nontrivial then it should have a dedicated CI job to ensure that all
   conditionally compiled parts are tested one way or another.

4. **Absolute paths are useful, but noisy** - described above it's easy to get
   into a situation where a conditionally compiled piece of code is the only
   users of a `use` statement. One easy fix is to remove the `use` and use the
   fully qualified path (e.g. `param: core::ptr::NonNull`) in the function
   instead. This reduces the `#[cfg]` to one, just the function in question, as
   opposed to one on the function and one on the `use`. Beware though that this
   can make function signatures very long very quickly, so if that ends up
   happening one of the above points may help instead.

5. **Use `#[cfg]` for anything that requires a new runtime dependency** - one of
   the primary use cases for `#[cfg]` in Wasmtime is to conditionally remove
   dependencies at runtime on pieces of functionality. For example if the
   `async` feature is disabled then stack switching is not necessary to
   implement. This is a lynchpin of Wasmtime's portability story where we don't
   guarantee all features compile on all platforms, but the "major" features
   should compile on all platforms. An example of this is that `threads`
   requires the standard library, but `runtime` does not.

6. **Don't use `#[cfg]` for compiler features** - in contrast to the previous
   point it's generally not necessary to plumb `#[cfg]` features to Wasmtime's
   integration with Cranelift. The runtime size or runtime features required to
   compile WebAssembly code is generally much larger than just running code
   itself. This means that conditionally compiled compiler features can just add
   lots of boilerplate to manage internally without much benefit. Ideally
   `#[cfg]` is only use for WebAssembly runtime features, not compilation of
   WebAssembly features.

Note that it's intentional that these guidelines are not 100% comprehensive.
Additionally they're not hard-and-fast rules in the sense that they're checked
in CI somewhere. Instead try to follow them if you can, but if you have any
questions or feel that `#[cfg]` is overwhelming feel free to reach out on Zulip
or on GitHub.

[file-gate]: https://github.com/bytecodealliance/wasmtime/blob/24620d9ff4cfd3a2a5f681181119eb8b0edaeab5/crates/wasmtime/src/lib.rs#L380-L383
[high-gate]: https://github.com/bytecodealliance/wasmtime/blob/24620d9ff4cfd3a2a5f681181119eb8b0edaeab5/crates/wasmtime/src/runtime.rs#L55-L56
[dummy-enabled]: https://github.com/bytecodealliance/wasmtime/blob/main/crates/wasmtime/src/runtime/vm/memory/shared_memory.rs
[dummy-disabled]: https://github.com/bytecodealliance/wasmtime/blob/24620d9ff4cfd3a2a5f681181119eb8b0edaeab5/crates/wasmtime/src/runtime/vm/memory/shared_memory_disabled.rs
