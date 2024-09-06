# Testing

This section describes how to run Wasmtime's tests and add new tests.

Before continuing, make sure you can [build
Wasmtime](./contributing-building.md) successfully. Can't run the tests if you
can't build it!

## Installing `wasm32` Targets

To compile the tests, you'll need the `wasm32-wasip1` and
`wasm32-unknown-unknown` targets installed, which, assuming you're using
[rustup.rs](https://rustup.rs) to manage your Rust versions, can be done as
follows:

```shell
rustup target add wasm32-wasip1 wasm32-unknown-unknown
```

## Running Tests

Depending on what you're modifying there's a few commands you may be the most
interested:

* `cargo test` - used to run the `tests/*` folder at the top-level. This tests
  the CLI and contains most tests for the `wasmtime` crate itself. This will
  also run all spec tests. Note that this does not run all tests in the
  repository, but it's generally a good starting point.
* `cargo test -p cranelift-tools` - used if you're working on Cranelift and this
  will run all the tests at `cranelift/filetests/filetests`. You can also,
  within the `cranelift` folder, run `cargo run test ./filetests` to run these
  tests.
* `cargo test -p wasmtime-wasi` - this will run all WASI tests for the
  `wasmtime-wasi` crate.

At this time not all of the crates in the Wasmtime workspace can be tested, so
running all tests is a little non-standard. To match what CI does and run all
tests you'll need to execute

```shell
./ci/run-tests.sh
```

## Testing a Specific Crate

You can test a particular Wasmtime crate with `cargo test -p
wasmtime-whatever`. For example, to test the `wasmtime-environ` crate, execute
this command:

```shell
cargo test -p wasmtime-environ
```

Alternatively, you can `cd` into the crate's directory, and run `cargo test`
there, without needing to supply the `-p` flag:

```shell
cd crates/environ/
cargo test
```

## Running the Wasm Spec Tests

The spec testsuite itself is in a git submodule, so make sure you've
checked it out and initialized its submodule:

```shell
git submodule update --init
```

When the submodule is checked out, Wasmtime runs the Wasm spec testsuite as part
of testing the `wasmtime-cli` crate at the crate root, meaning in the root of
the repository you can execute:

```shell
cargo test --test wast
```

You can pass an additional CLI argument to act as a filter on which tests to
run. For example to only run the spec tests themselves (excluding handwritten
Wasmtime-specific tests) and only in Cranelift you can run:

```shell
cargo test --test wast Cranelift/tests/spec
```

Note that in general spec tests are executed regardless of whether they pass
or not. In `tests/wast.rs` there's a `should_fail` function which indicates the
expected result of the test. When adding new spec tests or implementing features
this function will need to be updated as tests change from failing to passing.

## Running WASI Integration Tests

WASI integration tests can be run separately from all other tests which
can be useful when working on the `wasmtime-wasi` crate. This can be done by
executing this command:

```shell
cargo test -p wasmtime-wasi
```

Similarly if you're testing HTTP-related functionality you can execute:

```shell
cargo test -p wasmtime-wasi-http
```

Note that these tests will compile programs in `crates/test-programs` to run.

## Adding New Tests

### Adding Rust's `#[test]`-Style Tests

For very "unit-y" tests, we add `test` modules in the same `.rs` file as the
code that is being tested. These `test` modules are configured to only get
compiled during testing with `#[cfg(test)]`.

```rust
// some code...

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn some_test_for_that_code() {
        // ...
    }
}
```

If you're writing a unit test and a `test` module doesn't already exist, you can
create one.

For more "integration-y" tests, each crate supports a separate `tests` directory
within the crate, and put the tests inside there. Most integration tests in
Wasmtime are located in the root `tests/*.rs` location, notably
`tests/all/*.rs`. This tests much of the `wasmtime` crate for example and
facilitates `cargo test` at the repository root running most tests.

Some tests make more sense to live per-crate, though.  For example, many WASI
tests are at `crates/wasi/tests/*.rs`. For adding a test feel free to add it
wherever feels best, there's not really a strong reason to put it in one place
over another. While it's easiest to add to existing tests it's ok to add a new
`tests` directory with tests too.

### Adding Specification-Style Wast Tests

We use the spec testsuite as-is and without custom patches or a forked
version via a submodule at `tests/spec_testsuite`. This probably isn't what you
want to modify when adding a new Wasmtime test!

When you have a Wasmtime-specific test that you'd like to write in Wast and use
the Wast-style assertions, you can add it to our "misc testsuite". The misc
testsuite uses the same syntax and assertions as the spec testsuite, but lives
in `tests/misc_testsuite`. Feel free to add new tests to existing
`tests/misc_testsuite/*.wast` files or create new ones as needed. These tests
are run from the crate root:

```shell
cargo test --test wast
```

If you have a new test that you think really belongs in the spec testsuite, make
sure it makes sense for every Wasm implementation to run your test (i.e. it
isn't Wasmtime-specific) and send a pull request
[upstream](https://github.com/WebAssembly/spec). Once it is accepted in the
upstream repo, it'll make its way to the test-specific mirror at
[WebAssembly/testsuite](https://github.com/WebAssembly/testsuite) and then we
can update our git submodule and we'll start running the new tests.

### Adding WASI Integration Tests

When you have a WASI-specific test program that you'd like to include as a
test case to run against our WASI implementation, you can add it to our
`test-programs` crate. In particular, you should drop a main-style Rust source
file into `crates/test-programs/src/bin/PREFIX_some_new_test.rs`. Here the
`PREFIX` indicates what test suite it's going to run as. For example
`preview2_*` tests are run as part of `wasmtime-wasi` crate tests. The `cli_*`
tests are run as part of `tests/all/cli_tests.rs`. It's probably easiest to use
a preexisting prefix. The `some_new_test` name is arbitrary and is selected as
appropriate by you.

One a test file is added you'll need to add some code to execute the tests as
well. For example if you added a new test
`crates/test-programs/src/bin/cli_my_test.rs` then you'll need to add a new
function to `tests/all/cli_tests.rs` such as:

```rust
#[test]
fn my_test() {
    // ...
}
```

The path to the compiled WebAssembly of your test case will be available in a
Rust-level `const` named `CLI_MY_TEST`. There is also a component version at
`CLI_MY_TEST_COMPONENT`. These are used to pass as arguments to
`wasmtime`-the-CLI for example or you can use `Module::from_file`.

When in doubt feel free to copy existing tests and then modify them to suit your
needs.
