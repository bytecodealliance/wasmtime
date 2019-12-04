# Testing

This section describes how to run Wasmtime's tests and add new tests.

Before continuing, make sure you can [build
Wasmtime](./contributing-building.html) successfully. Can't run the tests if you
can't build it!

## Running All Tests

To run all of Wasmtime's tests, execute this command:

```shell
cargo test --all
```

You can also exclude a particular crate from testing with `--exclude`. For
example, if you want to avoid testing the `wastime-fuzzing` crate — which
requires that `libclang` is installed on your system, and for some reason maybe
you don't have it — you can run:

```shell
cargo test --all --exclude wasmtime-fuzzing
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
of testing the `wasmtime-cli` crate:

```shell
cargo test -p wasmtime-cli
```

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

For more "integration-y" tests, we create a `tests` directory within the crate,
and put the tests inside there. For example, there are various code
cache-related tests at `crates/environ/tests/cache_*.rs`. Always feel free to
add a `tests` directory to a crate, if you want to add a new test and there
aren't any existing tests.

### Adding Specification-Style Wast Tests

We use the spec testsuite as-is and without custom patches or a forked
version. If you have a new test that you want to add to it, make sure it makes
sense for every Wasm implementation to run your test (i.e. it isn't
Wasmtime-specific) and send a pull request
[upstream](https://github.com/WebAssembly/testsuite/). Once it is accepted in
the upstream repo, we can update our git submodule and we'll start running the
new tests.

Alternatively, if you have a Wasmtime-specific test that you'd like to write in
Wast and use the Wast-style assertions, you can add it to our "misc
testsuite". The misc testsuite uses the same syntax and assertions as the spec
testsuite, but lives in `tests/misc_testsuite`. Feel free to add new tests to
existing `tests/misc_testsuite/*.wast` files or create new ones as needed. These
tests are run as part of the `wasmtime-cli` crate.
