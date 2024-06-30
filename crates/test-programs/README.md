# test-programs

This crate contains many different binaries which are built into Wasm test
programs for other crates in the Wasmtime workspace.

Each file in `./src/bin/` is built into a distinct test binary. By convention,
the prefix before the first underscore in each binary is the name of the test
suite: `api`, `cli`, `http`, `nn`, `piped`, `preview1`, and `preview2`.

## Test suites

### api

This suite is used for exercising `wasmtime`'s Rust APIs. Each test program in
this suite has particular behavior which interacts with the tests themselves.

These test programs are executed in `/crates/wasi/tests/api.rs`.

### cli

This suite is used for exercising the `wasmtime-cli` crate, which produces the
`wasmtime` native executable. Each test program in this suite has particular
behavior which interacts with the commands and arguments passed to the
wasmtime CLI. Some use the `wasmtime run` command and others the `wasmtime
serve` command.

These test programs are executed in `/tests/all/cli_tests.rs`.

### http

This test suite is used for exercising the `wasmtime-wasi-http` crate. Each
test program in this suite is a component which exports the wasi-cli command
world's run interface (i.e. its entrypoint is defined by `fn main() {...}`),
and is given the cli command world imports as well as the wasi-http
outgoing-handler and types imports - in short, this is a basic executable
which can make http requests using wasi-http.

Each test program executor spawns a fresh tokio runtime on a unique local
socket which responds to the requests made by a single test program. This
socket address is passed to the test program using the `HTTP_SERVER`
environment variable.

These test programs are executed in `/crates/wasi-http/tests/all/sync.rs`
and `/crates/wasi-http/tests/all/async_.rs`, which test the synchronous and
async rust interfaces of the `wasmtime` and `wasmtime-wasi-http` crates,
respectively.

### nn

This test suite is used for exercising the `wasmtime-wasi-nn` crate. It only
tests the witx ABI of the wasi-nn proposal.

These test programs are executed in `crates/wasi-nn/tests/all.rs`.

### piped

This suite is used for exercising the wasmtime-cli and, in particular,
examining the behavior when the standard output of one instance is piped to
the input of another instance. The test programs in this suite are passed
the environment variable `PIPED_SIDE` with the value `PRODUCER` or `CONSUMER`
to select the behavior of producing output to stdout, or consuming input
on stdin.

These test programs are executed in `/tests/all/cli_tests.rs`, which creates
two separate `wasmtime` host processes, with the consumer process stdin file
descriptor set to the producer process stdout file descriptor. The test
harness asserts that both processes exit successfully.

### preview1

This suite is used for exercising the Wasi Preview 1 (aka Wasi 0.1) interface,
and, additionally, provides test coverage of Wasi Preview 2 (aka Wasi 0.2)
when used with the component adapter, as described below.

Some of these tests require the stdin stream provided is always empty and
closed, others require that the stdin stream is open and pending forever. The
test execution provides this by inheriting the host process stdio in some test
program invocations, and providing the default null stdio in others.

All of these tests require an empty directory be provided as a preopen, and
the path of that preopen passed as the first argument.

The Wasmtime tree has three distinct ways to implement the Wasi 0.1 interface,
which should all provide identical functionality:
* The Wasm Module is executed against the host implementation of Wasi 0.1
  provided by the `wasi-common` crate. The test programs are executed
  in `/crates/wasi-common/tests/all/{sync,async_}.rs` against the synchronous
  and async Rust interfaces of the `wasmtime` and `wasi-common` crate.
* The Wasm Module is executed against the host implementation of Wasi 0.1
  provided by the `wasmtime-wasi` crate. This Wasi 0.1 interface is implemented
  using the Wasi 0.2 implementation defined in that same crate. The test
  programs are executed in `/crates/wasi/tests/all/preview1.rs` against the
  synchronous Rust interfaces of the `wasmtime` and `wasmtime-wasi` crates.
* The Wasm Component created by composing the Module with the
  `wasi-preview1-component-adapter`, which provides a pure WebAssembly
  implementation of the Wasi 0.1 import functions in terms of the Wasi 0.2
  import functions. When run in the host implementation of Wasi 0.2 provided
  by the `wasmtime-wasi` crate, this mode of execution exercises the Wasi 0.2
  interfaces, and ensures that a faithful implementation of Wasi 0.1 can be
  provided in terms of them, without needing any support for Wasi 0.1 in the
  host. The test programs are executed in
  `crates/wasi/tests/all/{sync,async_}.rs` against the synchronous and async
  Rust interfaces of the `wasmtime` and `wasmtime-wasi` crates.

### preview2

This suite is used for exercising the portions of Wasi Preview 2 (aka Wasi
0.2) which are not covered by the Wasi 0.1 + component adapter, as described
in the preview1 suite above.

These test programs use `wit-bindgen` to generate bindings to Wasi 0.2
(invoked at the root of the `test-programs` crate), and then invoke those
bindings directly in each test. The modules created from these test programs
still have imports of Wasi 0.1 thanks to the Rust `std` library, so those
imports are implemented by the same `wasi-preview1-component-adapter` as in
the preview1 suite. (When Rust stabilizes the `wasm32-wasip2` target, this
will finally get to be removed!) However, the tests themselves should be
exercising the Wasi 0.2 interfaces directly - in some cases, such as the
socket tests, its exercising functionality that Wasi 0.1 lacked.

These tests are executed much like the component adapted preview1 test suite
is, in `crates/wasi/tests/all/{sync,async_}.rs`.

## Building and executing these test programs

These tests are built and consumed by way of the `test-programs-artifacts`
crate. `test-programs-artifacts` has a `build.rs` which compiles this crate
and translates some of the resulting Wasm Modules to Wasm Components. It then
exposes the filesystem paths of these binaries as string constants, and as
macros which iterate over the complete set of binary paths in a given test
suite.

The filesystem path to each Wasm Module built by `test-programs-artifacts`
is provided by a `pub const <TESTNAME>: &'static str = ...;` at the top
level of that crate, where `<TESTNAME>` is the file stem of the source file
in `./src/bin`, in shouty snake case (all caps and underscores).

The filesystem path to each Wasm Component built by `test-programs-artifacts`
is provided by a `pub const <TESTNAME>_COMPONENT: &'static str = ...;` at the top
level of that crate, where `<TESTNAME>` is the file stem of the source file
in `./src/bin`, in shouty snake case (all caps and underscores), and then the
suffix `_COMPONENT` is added to the identifier to distinguish it from the
module.

Wasm Components are not built for the `nn` test suite. All other components
are created, using the `wasi-preview1-component-adapter` to translate Wasi 0.1
module imports to Wasi 0.2 component imports.

Each test suite is given an iterator macro in `test-programs-artifacts` named
`foreach_<suitename>`.

The iterator macro is passed the identifier of another macro which is passed
the path of each test in the suite. You could do something very clever with
this if you desire, but as of this writing, all use sites in the workspace
pass it a macro `assert_test_exists` which emits a `use <testname> as _;`
statement. This ensures that, in the rust mod where `foreach_<suitename>` is
invoked, a `fn <testname>() {...}` is defined, which in turn makes sure that,
if a wasmtime developer adds a new test program, they also add a test which
executes it in the appropriate location.

By convention, the expectation is that each mod that invokes
`foreach_<suitename>!(assert_test_exists);` also defines something like:

```
#[test]
fn <testname>() { 
    // do something which executes <TESTNAME> or
    // <TESTNAME>_COMPONENT here.
}
```
