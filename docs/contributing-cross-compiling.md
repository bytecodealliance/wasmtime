# Cross Compiling

When contributing to Wasmtime and Cranelift you may run into issues that only
reproduce on a different architecture from your development machine. Luckily,
`cargo` makes cross compilation and running tests under [QEMU] pretty easy.

[QEMU]: https://www.qemu.org/

This guide will assume you are on an x86-64 with Ubuntu/Debian as your OS. The
basic approach (with commands, paths, and package names appropriately tweaked)
applies to other Linux distributions as well.

On Windows you can install build tools for AArch64 Windows, but targeting
platforms like Linux or macOS is not easy. While toolchains exist for targeting
non-Windows platforms you'll have to hunt yourself to find the right one.

On macOS you can install, through Xcode, toolchains for iOS but the main
`x86_64-apple-darwin` is really the only easy target to install. You'll need to
hunt for toolchains if you want to compile for Linux or Windows.

## Install Rust Targets

First, use `rustup` to install Rust targets for the other architectures that
Wasmtime and Cranelift support:

```shell
$ rustup target add \
    s390x-unknown-linux-gnu \
    riscv64gc-unknown-linux-gnu \
    aarch64-unknown-linux-gnu
```

## Install GCC Cross-Compilation Toolchains

Next, you'll need to install a `gcc` for each cross-compilation target to serve
as a linker for `rustc`.

```shell
$ sudo apt install \
    gcc-s390x-linux-gnu \
    gcc-riscv64-linux-gnu \
    gcc-aarch64-linux-gnu
```

## Install `qemu`

You will also need to install `qemu` to emulate the cross-compilation targets.

```shell
$ sudo apt install qemu-user
```

## Configure Cargo

The final bit to get out of the way is to configure `cargo` to use the
appropriate `gcc` and `qemu` when cross-compiling and running tests for other
architectures.

Add this to `.cargo/config.toml` in the Wasmtime repository (or create that file
if none already exists).

```toml
[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"
runner = "qemu-aarch64 -L /usr/aarch64-linux-gnu -E LD_LIBRARY_PATH=/usr/aarch64-linux-gnu/lib -E WASMTIME_TEST_NO_HOG_MEMORY=1"

[target.riscv64gc-unknown-linux-gnu]
linker = "riscv64-linux-gnu-gcc"
runner = "qemu-riscv64 -L /usr/riscv64-linux-gnu -E LD_LIBRARY_PATH=/usr/riscv64-linux-gnu/lib -E WASMTIME_TEST_NO_HOG_MEMORY=1"

[target.s390x-unknown-linux-gnu]
linker = "s390x-linux-gnu-gcc"
runner = "qemu-s390x -L /usr/s390x-linux-gnu -E LD_LIBRARY_PATH=/usr/s390x-linux-gnu/lib -E WASMTIME_TEST_NO_HOG_MEMORY=1"
```

## Cross-Compile Tests and Run Them!

Now you can use `cargo build`, `cargo run`, and `cargo test` as you normally
would for any crate inside the Wasmtime repository, just add the appropriate
`--target` flag!

A few examples:

* Build the `wasmtime` binary for `aarch64`:

  ```shell
  $ cargo build --target aarch64-unknown-linux-gnu
  ```

* Run the tests under `riscv` emulation:

  ```shell
  $ cargo test --target riscv64gc-unknown-linux-gnu
  ```

* Run the `wasmtime` binary under `s390x` emulation:

  ```shell
  $ cargo run --target s390x-unknown-linux-gnu -- compile example.wasm
  ```
