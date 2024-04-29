# Architecture of Wasmtime

This document is intended to give an overview of the implementation of Wasmtime.
This will explain the purposes of the various `wasmtime-*` crates that the main
`wasmtime` crate depends on. For even more detailed information it's recommended
to review the code itself and find the comments contained within.

## The `wasmtime` crate

The main entry point for Wasmtime is the `wasmtime` crate itself. Wasmtime is
designed such that the `wasmtime` crate is nearly a 100% safe API (safe in the
Rust sense) modulo some small and well-documented functions as to why they're
`unsafe`. The `wasmtime` crate provides features and access to WebAssembly
primitives and functionality, such as compiling modules, instantiating them,
calling functions, etc.

At this time the `wasmtime` crate is the first crate that is intended to be
consumed by users. First in this sense means that everything `wasmtime` depends
on is thought of as an internal dependency. We publish crates to crates.io but
put very little effort into having a "nice" API for internal crates or worrying
about breakage between versions of internal crates. This primarily means that
all the other crates discussed here are considered internal dependencies of
Wasmtime and don't show up in the public API of Wasmtime at all. To use some
Cargo terminology, all the `wasmtime-*` crates that `wasmtime` depends on are
"private" dependencies.

Additionally at this time the safe/unsafe boundary between Wasmtime's internal
crates is not the most well-defined. There are methods that should be marked
`unsafe` which aren't, and `unsafe` methods do not have exhaustive documentation
as to why they are `unsafe`. This is an ongoing matter of improvement, however,
where the goal is to have safe methods be actually safe in the Rust sense,
as well as having documentation for `unsafe` methods which clearly lists why
they are `unsafe`.

## Important concepts

To preface discussion of more nitty-gritty internals, it's important to have a
few concepts in the back of your head. These are some of the important types and
their implications in Wasmtime:

* `wasmtime::Engine` - this is a global compilation context which is sort of the
  "root context". An `Engine` is typically created once per program and is
  expected to be shared across many threads (internally it's atomically
  reference counted). Each `Engine` stores configuration values and other
  cross-thread data such as type interning for `Module` instances. The main
  thing to remember for `Engine` is that any mutation of its internals typically
  involves acquiring a lock, whereas for `Store` below no locks are necessary.

* `wasmtime::Store` - this is the concept of a "store" in WebAssembly. While
  there's also a formal definition to go off of, it can be thought of as a bag
  of related WebAssembly objects. This includes instances, globals, memories,
  tables, etc. A `Store` does not implement any form of garbage collection of
  the internal items (there is a `gc` function but that's just for `externref`
  values). This means that once you create an `Instance` or a `Table` the memory
  is not actually released until the `Store` itself is deallocated. A `Store` is
  sort of a "context" used for almost all wasm operations. `Store` also contains
  instance handles which recursively refer back to the `Store`, leading to a
  good bit of aliasing of pointers within the `Store`. The important thing for
  now, though, is to know that `Store` is a unit of isolation. WebAssembly
  objects are always entirely contained within a `Store`, and at this time
  nothing can cross between stores (except scalars if you manually hook it up).
  In other words, wasm objects from different stores cannot interact with each
  other. A `Store` cannot be used simultaneously from multiple threads (almost
  all operations require `&mut self`).

* `wasmtime::runtime::vm::InstanceHandle` - this is the low-level representation of a
  WebAssembly instance. At the same time this is also used as the representation
  for all host-defined objects. For example if you call `wasmtime::Memory::new`
  it'll create an `InstanceHandle` under the hood. This is a very `unsafe` type
  that should probably have all of its functions marked `unsafe` or otherwise
  have more strict guarantees documented about it, but it's an internal type
  that we don't put much thought into for public consumption at this time. An
  `InstanceHandle` doesn't know how to deallocate itself and relies on the
  caller to manage its memory. Currently this is either allocated on-demand
  (with `malloc`) or in a pooling fashion (using the pooling allocator). The
  `deallocate` method is different in these two paths (as well as the
  `allocate` method).

  An `InstanceHandle` is laid out in memory with some Rust-owned values first
  capturing the dynamic state of memories/tables/etc. Most of these fields are
  unused for host-defined objects that serve one purpose (e.g. a
  `wasmtime::Table::new`), but for an instantiated WebAssembly module these
  fields will have more information. After an `InstanceHandle` in memory is a
  `VMContext`, which will be discussed next. `InstanceHandle` values are the
  main internal runtime representation and what the `crate::runtime::vm` code
  works with. The `wasmtime::Store` holds onto all these `InstanceHandle` values
  and deallocates them at the appropriate time. From the runtime perspective it
  simplifies things so the graph of wasm modules communicating to each other is
  reduced to simply `InstanceHandle` values all talking to themselves.

* `crate::runtime::vm::VMContext` - this is a raw pointer, within an allocation of
  an `InstanceHandle`, that is passed around in JIT code. A `VMContext` does not
  have a structure defined in Rust (it's a 0-sized structure) because its
  contents are dynamically determined based on the `VMOffsets`, or the source
  wasm module it came from. Each `InstanceHandle` has a "shape" of a `VMContext`
  corresponding with it. For example a `VMContext` stores all values of
  WebAssembly globals, but if a wasm module has no globals then the size of this
  array will be 0 and it won't be allocated. The intention of a `VMContext` is
  to be an efficient in-memory representation of all wasm module state that JIT
  code may access. The layout of `VMContext` is dynamically determined by a
  module and JIT code is specialized for this one structure. This means that the
  structure is efficiently accessed by JIT code, but less efficiently accessed
  by native host code. A non-exhaustive list of purposes of the `VMContext` is
  to:

  * Store WebAssembly instance state such as global values, pointers to tables,
    pointers to memory, and pointers to other JIT functions.
  * Separate wasm imports and local state. Imported values have pointers stored
    to their actual values, and local state has the state defined inline.
  * Hold a pointer to the stack limit at which point JIT code will trigger a
    stack overflow.
  * Hold a pointer to a `VMExternRefActivationsTable` for fast-path insertion of
    `externref` values into the table.
  * Hold a pointer to a `*mut dyn crate::runtime::vm::Store` so store-level
    operations can be performed in libcalls.

  A comment about the layout of a `VMContext` can be found in the `vmoffsets.rs`
  file.

* `wasmtime::Module` - this is the representation of a compiled WebAssembly
  module. At this time Wasmtime always assumes that a wasm module is always
  compiled to native JIT code. `Module` holds the results of said compilation,
  and currently Cranelift can be used for compiling. It is a goal of
  Wasmtime to support other modes of representing modules but those are not
  implemented today just yet, only Cranelift is implemented and supported.

* `wasmtime_environ::Module` - this is a descriptor of a wasm module's type and
  structure without holding any actual JIT code. An instance of this type is
  created very early on in the compilation process, and it is not modified when
  functions themselves are actually compiled. This holds the internal type
  representation and state about functions, globals, etc. In a sense this can be
  thought of as the result of validation or typechecking a wasm module, although
  it doesn't have information such as the types of each opcode or minute
  function-level details like that.

## Compiling a module

With a high-level overview and some background information of types, this will
next walk through the steps taken to compile a WebAssembly module. The main
entry point for this is the `wasmtime::Module::from_binary` API. There are a
number of other entry points that deal with surface-level details like
translation from text-to-binary, loading from the filesystem, etc.

Compilation is roughly broken down into a few phases:

1. First compilation walks over the WebAssembly module validating everything
   except function bodies. This synchronous pass over a wasm module creates a
   `wasmtime_environ::Module` instance and additionally prepares for function
   compilation. Note that with the module linking proposal one input module may
   end up creating a number of output modules to process. Each module is
   processed independently and all further steps are parallelized on a
   per-module basis. Note that parsing and validation of the WebAssembly module
   happens with the `wasmparser` crate. Validation is interleaved with parsing,
   validating parsed values before using them.

2. Next all functions within a module are validated and compiled in parallel.
   No inter-procedural analysis is done and each function is compiled as its
   own little island of code at this time. This is the point where the meat of
   Cranelift is invoked on a per-function basis.

3. The compilation results at this point are all woven into a
   `wasmtime_jit::CompilationArtifacts` structure. This holds module information
   (`wasmtime_environ::Module`), compiled JIT code (stored as an ELF image), and
   miscellaneous other information about functions such as platform-agnostic
   unwinding information, per-function trap tables (indicating which JIT
   instructions can trap and what the trap means), per-function address maps
   (mapping from JIT addresses back to wasm offsets), and debug information
   (parsed from DWARF information in the wasm module). These results are inert
   and can't actually be executed, but they're appropriate at this point to
   serialize to disk or begin the next phase...

4. The final step is to actually place all code into a form that's ready to get
   executed. This starts from the `CompilationArtifacts` of the previous step.
   Here a new memory mapping is allocated and the JIT code is copied into this
   memory mapping. This memory mapping is then switched from read/write to
   read/execute so it's actually executable JIT code at this point. This is
   where various hooks like loading debuginfo, informing JIT profilers of new
   code, etc, all happens. At this point a `wasmtime_jit::CompiledModule` is
   produced and this is itself wrapped up in a `wasmtime::Module`. At this
   point the module is ready to be instantiated.

A `wasmtime::Module` is an atomically-reference-counted object where upon
instantiation into a `Store`, the `Store` will hold a strong reference to the
internals of the module. This means that all instances of a `wasmtime::Module`
share the same compiled code. Additionally a `wasmtime::Module` is one of the
few objects that lives outside of a `wasmtime::Store`. This means that
`wasmtime::Module`'s reference counting is its own form of memory management.

Note that the property of sharing a module's compiled code across all
instantiations has interesting implications on what the compiled code can
assume. For example Wasmtime implements a form of type interning, but the
interned types happen at a few different levels. Within a module we deduplicate
function types, but across modules in a `Store` types need to be represented
with the same value. This means that if the same module is instantiated into
many stores its same function type may take on many values, so the compiled
code can't assume a particular value for a function type. (more on type
information later). The general gist though is that compiled code leans
relatively heavily on the `VMContext` for contextual input because the JIT code
is intended to be so widely reusable.

### Trampolines

An important aspect to also cover for compilation is the creation of
trampolines. Trampolines in this case refer to code executed by Wasmtime to
enter WebAssembly code. The host may not always have prior knowledge about the
signature of the WebAssembly function that it wants to call. Wasmtime JIT code
is compiled with native ABIs (e.g. params/results in registers according to
System V on Unix), which means that a Wasmtime embedding doesn't have an easy
way to enter JIT code.

This problem is what the trampolines compiled into a module solve, which is to
provide a function with a known ABI that will call into a function with a
specific other type signature/ABI. Wasmtime collects all the exported functions
of a module and creates a set of their type signatures. Note that exported in
this context actually means "possibly exported" which includes things like
insertion into a global/function table, conversion to a `funcref`, etc. A
trampoline is generated for each of these type signatures and stored along with
the JIT code for the rest of the module.

These trampolines are then used with the `wasmtime::Func::call` API where in
that specific case because we don't know the ABI of the target function the
trampoline (with a known ABI) is used and has all the parameters/results passed
through the stack.

Another point of note is that trampolines are not deduplicated at this time.
Each compiled module contains its own set of trampolines, and if two compiled
modules have the same types then they'll have different copies of the same
trampoline.

### Type Interning and `VMSharedSignatureIndex`

One important point to talk about with compilation is the
`VMSharedSignatureIndex` type and how it's used. The `call_indirect` opcode in
wasm compares an actual function's signature against the function signature of
the instruction, trapping if the signatures mismatch. This is implemented in
Wasmtime as an integer comparison, and the comparison happens on a
`VMSharedSignatureIndex` value. This index is an intern'd representation of a
function type.

The scope of interning for `VMSharedSignatureIndex` happens at the
`wasmtime::Engine` level. Modules are compiled into an `Engine`. Insertion of a
`Module` into an `Engine` will assign a `VMSharedSignatureIndex` to all of the
types found within the module.

The `VMSharedSignatureIndex` values for a module are local to that one
instantiation of a `Module` (and they may change on each insertion of a
`Module` into a different `Engine`). These are used during the instantiation
process by the runtime to assign a type ID effectively to all functions for
imports and such.

## Instantiating a module

Once a module has been compiled it's typically then instantiated to actually
get access to the exports and call wasm code. Instantiation always happens
within a `wasmtime::Store` and the created instance (plus all exports) are tied
to the `Store`.

Instantiation itself (`crates/wasmtime/src/instance.rs`) may look complicated,
but this is primarily due to the implementation of the Module Linking proposal.
The rough flow of instantiation looks like:

1. First all imports are type-checked. The provided list of imports is
   cross-referenced with the list of imports recorded in the
   `wasmtime_environ::Module` and all types are verified to line up and match
   (according to the core wasm specification's definition of type matching).

2. Each `wasmtime_environ::Module` has a list of initializers that need to be
   completed before instantiation is finished. For MVP wasm this only involves
   loading the import into the correct index array, but for module linking this
   could involve instantiating other modules, handling `alias` fields, etc. In
   any case the result of this step is a `crate::runtime::vm::Imports` array
   which has the values for all imported items into the wasm module. Note that
   in this case an import is typically some sort of raw pointer to the actual
   state plus the `VMContext` of the instance that was imported from. The final
   result of this step is an `InstanceAllocationRequest`, which is then
   submitted to the configured instance allocator, either on-demand or pooling.

3. The `InstanceHandle` corresponding to this instance is allocated. How this
   is allocated depends on the strategy (malloc for on-demand, slab allocation
   for pooling). In addition to initialization of the fields of `InstanceHandle`
   this also initializes all the fields of the `VMContext` for this handle
   (which as mentioned above is adjacent to the `InstanceHandle` allocation
   after it in memory). This does not process any data segments, element
   segments, or the `start` function at this time.

4. At this point the `InstanceHandle` is stored within the `Store`. This is
   the "point of no return" where the handle must be kept alive for the same
   lifetime as the `Store` itself. If an initialization step fails then the
   instance may still have had its functions, for example, inserted into an
   imported table via an element segment. This means that even if we fail to
   initialize this instance its state could still be visible to other
   instances/objects so we need to keep it alive regardless.

5. The final step is performing wasm-defined instantiation. This involves
   processing element segments, data segments, the `start` function, etc. Most
   of this is just translating from Wasmtime's internal representation to the
   specification's required behavior.

Another part worth pointing out for instantiating a module is that a
`ModuleRegistry` is maintained within a `Store` of all instantiated modules
into the store. The purpose of this registry is to retain a strong reference to
items in the module needed to run instances. This includes the JIT code
primarily but also has information such as the `VMSharedSignatureIndex`
registration, metadata about function addresses and such, etc. Much of this
data is stored into a `GLOBAL_MODULES` map for later access during traps.

## Traps

Once instances have been created and wasm starts running most things are fairly
standard. Trampolines are used to enter wasm (or we can enter with a known ABI
if using `wasmtime::TypedFunc`) and JIT code generally does what it does to
execute wasm. An important aspect of the implementation to cover, however, is
traps.

Wasmtime today implements traps with `longjmp` and `setjmp`. The `setjmp`
function cannot be defined in Rust (even unsafely --
(https://github.com/rust-lang/rfcs/issues/2625) so the
`crates/wasmtime/src/runtime/vm/helpers.c` file actually calls
setjmp/longjmp. Note that in general the operation of `longjmp` is not safe to
execute in Rust because it skips stack-based destructors, so after `setjmp` when
we call back into Rust to execute wasm we need to be careful in Wasmtime to not
have any significant destructors on the stack once wasm is called.

Traps can happen from a few different sources:

* Explicit traps - these can happen when a host call returns a trap, for
  example. These bottom out in `raise_user_trap` or `raise_lib_trap`, both of
  which immediately call `longjmp` to go back to the wasm starting point. Note
  that these, like when calling wasm, have to have callers be very careful to
  not have any destructors on the stack.

* Signals - this is the main vector for trap. Basically we use segfault and
  illegal instructions to implement traps in wasm code itself. Segfaults arise
  when linear memory accesses go out of bounds and illegal instructions are how
  the wasm `unreachable` instruction is implemented. In both of these cases
  Wasmtime installs a platform-specific signal handler to catch the signal,
  inspect the state of the signal, and then handle it. Note that Wasmtime tries
  to only catch signals that happen from JIT code itself as to not accidentally
  cover up other bugs. Exiting a signal handler happens via `longjmp` to get
  back to the original wasm call-site.

The general idea is that Wasmtime has very tight control over the stack frames
of wasm (naturally via Cranelift) and also very tight control over the code that
executes just before we enter wasm (aka before the `setjmp`) and just after we
reenter back into wasm (aka frames before a possible `longjmp`).

The signal handler for Wasmtime uses the `GLOBAL_MODULES` map populated during
instantiation to determine whether a program counter that triggered a signal is
indeed a valid wasm trap. This should be true except for cases where the host
program has another bug that triggered the signal.

A final note worth mentioning is that Wasmtime uses the Rust `backtrace` crate
to capture a stack trace when a wasm exception occurs. This forces Wasmtime to
generate native platform-specific unwinding information to correctly unwind the
stack and generate a stack trace for wasm code. This does have other benefits as
well such as improving generic sampling profilers when used with Wasmtime.

## Linear Memory

Linear memory in Wasmtime is implemented effectively with `mmap` (or the
platform equivalent thereof), but there are some subtle nuances that are worth
pointing out here too. The implementation of linear memory is relatively
configurable which gives rise to a number of situations that both the runtime
and generated code need to handle.

First there are a number of properties about linear memory which can be
configured:

* `wasmtime::Config::static_memory_maximum_size`
* `wasmtime::Config::static_memory_guard_size`
* `wasmtime::Config::dynamic_memory_guard_size`
* `wasmtime::Config::guard_before_linear_memory`

The methods on `Config` have a good bit of documentation to go over some
nitty-gritty, but the general gist is that Wasmtime has two modes of memory:
static and dynamic. Static memories represent an address space reservation that
never moves and pages are committed to represent memory growth. Dynamic
memories represent allocations where the committed portion exactly matches the
wasm memory's size and growth happens by allocating a bigger chunk of memory.

The guard size configuration indicates the size of the guard region that
happens after linear memory. This guard size affects whether generated JIT code
emits bounds checks or not. Bounds checks are elided if out-of-bounds addresses
provably encounter the guard pages.

The `guard_before_linear_memory` configuration additionally places guard pages
in front of linear memory as well as after linear memory (the same size on both
ends). This is only used to protect against possible Cranelift bugs and
otherwise serves no purpose.

The defaults for Wasmtime on 64-bit platforms are:

* 4GB static maximum size meaning all 32-bit memories are static and 64-bit
  memories are dynamic.
* 2GB static guard size meaning all loads/stores with less than 2GB offset
  don't need bounds checks with 32-bit memories.
* Guard pages before linear memory are enabled.

Altogether this means that 32-bit linear memories result in an 8GB virtual
address space reservation by default in Wasmtime. With the pooling allocator
where we know that linear memories are contiguous this results in a 6GB
reservation per memory because the guard region after one memory is the guard
region before the next.

Note that 64-bit memories (the memory64 proposal for WebAssembly) can be
configured to be static but will never be able to elide bounds checks at this
time. This configuration is possible through the `static_memory_forced`
configuration option. Additionally note that support for 64-bit memories in
Wasmtime is functional but not yet tuned at this time so there's probably still
some performance work and better defaults to manage.

## Tables and `externref`

WebAssembly tables contain reference types, currently either `funcref` or
`externref`. A `funcref` in Wasmtime is represented as `*mut
VMCallerCheckedFuncRef` and an `externref` is represented as `VMExternRef`
(which is internally `*mut VMExternData`). Tables are consequently represented
as vectors of pointers.  Table storage memory management by default goes through
Rust's `Vec` which uses `malloc` and friends for memory. With the pooling
allocator this uses preallocated memory for storage.

As mentioned previously `Store` has no form of internal garbage
collection for wasm objects themselves so a `funcref` table in wasm is pretty
simple in that there's no lifetime management of any of the pointers stored
within, they're simply assumed to be valid for as long as the table is in use.

For tables of `externref` the story is more complicated. The `VMExternRef` is a
version of `Arc<dyn Any>` but specialized in Wasmtime so JIT code knows where
the offset of the reference count field to directly manipulate it is.
Furthermore tables of `externref` values need to manage the reference count
field themselves, since the pointer stored in the table is required to have a
strong reference count allocated to it.

## GC and `externref`

Wasmtime implements the `externref` type of WebAssembly with an
atomically-reference-counted pointer. Note that the atomic part is not needed
by wasm itself but rather from the Rust embedding environment where it must be
safe to send `ExternRef` values to other threads. Wasmtime also does not
come with a cycle collector so cycles of host-allocated `ExternRef` objects
will leak.

Despite reference counting, though, a `Store::gc` method exists. This is an
implementation detail of how reference counts are managed while wasm code is
executing. Instead of managing the reference count of an `externref` value
individually as it moves around on the stack Wasmtime implements "deferred
reference counting" where there's an overly conservative list of `ExternRef`
values that may be in use, and periodically a GC is performed to make this
overly conservative list a precise one. This leverages the stack map support
of Cranelift plus the backtracing support of `backtrace` to determine live
roots on the stack. The `Store::gc` method forces the
possibly-overly-conservative list to become a precise list of `externref`
values that are actively in use on the stack.

## Index of crates

The main Wasmtime internal crates are:

* `wasmtime` - the safe public API of `wasmtime`.
  * `wasmtime::runtime::vm` - low-level runtime implementation of Wasmtime. This
    is where `VMContext` and `InstanceHandle` live. This module used to be a
    crate, but has since been folding into `wasmtime`.
* `wasmtime-environ` - low-level compilation support. This is where translation
  of the `Module` and its environment happens, although no compilation actually
  happens in this crate (although it defines an interface for compilers). The
  results of this crate are handed off to other crates for actual compilation.
* `wasmtime-cranelift` - implementation of function-level compilation using
  Cranelift.

Note that at this time Cranelift is a required dependency of wasmtime. Most of
the types exported from `wasmtime-environ` use cranelift types in their API. One
day it's a goal, though, to remove the required cranelift dependency and have
`wasmtime-environ` be a relatively standalone crate.

In addition to the above crates there are some other miscellaneous crates that
`wasmtime` depends on:

* `wasmtime-cache` - optional dependency to manage default caches on the
  filesystem. This is enabled in the CLI by default but not enabled in the
  `wasmtime` crate by default.
* `wasmtime-fiber` - implementation of stack-switching used by `async` support
  in Wasmtime
* `wasmtime-debug` - implementation of mapping wasm dwarf debug information to
  native dwarf debug information.
* `wasmtime-profiling` - implementation of hooking up generated JIT code to
  standard profiling runtimes.
* `wasmtime-obj` - implementation of creating an ELF image from compiled
  functions.
