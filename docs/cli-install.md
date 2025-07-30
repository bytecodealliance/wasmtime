# Installing `wasmtime`

Here we'll show you how to install the `wasmtime` command line tool. Note that
this is distinct from embedding the Wasmtime project into another, for that
you'll want to consult the [embedding documentation](lang.md).

The easiest way to install the `wasmtime` CLI tool is through our installation
script. Linux and macOS users can execute the following:

```console
curl https://wasmtime.dev/install.sh -sSf | bash
```

This will download a precompiled version of `wasmtime`, place it in
`$HOME/.wasmtime`, and update your shell configuration to place the right
directory in `PATH`.

Windows users will want to visit our [releases page][releases] and can download
the MSI installer (`wasmtime-dev-x86_64-windows.msi` for example) and use that
to install.

[releases]: https://github.com/bytecodealliance/wasmtime/releases/latest

You can confirm your installation works by executing:

```shell-session
$ wasmtime -V
wasmtime 30.0.0 (ede663c2a 2025-02-19)
```

And now you're off to the races! Be sure to check out the [various CLI
options](cli-options.md) as well.

## Download Precompiled Binaries

If you'd prefer to not use an installation script, or you're perhaps
orchestrating something in CI, you can also download one of our precompiled
binaries of `wasmtime`. We have two channels of releases right now for
precompiled binaries:

1. Each tagged release will have a full set of release artifacts on the [GitHub
   releases page][releases].
2. The [`dev` release] is also continuously updated with the latest build of the
   `main` branch. If you want the latest-and-greatest and don't mind a bit of
   instability, this is the release for you.

[`dev` release]: https://github.com/bytecodealliance/wasmtime/releases/tag/dev

When downloading binaries you'll likely want one of the following archives (for
the `dev` release)

* Linux users - [`wasmtime-dev-x86_64-linux.tar.xz`]
* macOS users - [`wasmtime-dev-x86_64-macos.tar.xz`]
* Windows users - [`wasmtime-dev-x86_64-windows.zip`]

Each of these archives has a `wasmtime` binary placed inside which can be
executed normally as the CLI would.

[`wasmtime-dev-x86_64-linux.tar.xz`]: https://github.com/bytecodealliance/wasmtime/releases/download/dev/wasmtime-dev-x86_64-linux.tar.xz
[`wasmtime-dev-x86_64-macos.tar.xz`]: https://github.com/bytecodealliance/wasmtime/releases/download/dev/wasmtime-dev-x86_64-macos.tar.xz
[`wasmtime-dev-x86_64-windows.zip`]: https://github.com/bytecodealliance/wasmtime/releases/download/dev/wasmtime-dev-x86_64-windows.zip

## Install via Cargo

If you have [Rust and Cargo](https://www.rust-lang.org/tools/install) available in your system, you can build and install an official `wasmtime` release from its [crates.io](https://crates.io/crates/wasmtime-cli) source:

```console
cargo install wasmtime-cli
```

This compiles and installs `wasmtime` into your Cargo bin directory (typically `$HOME/.cargo/bin`). Make sure that directory is in your `PATH` before running `wasmtime`. For example, add the following line to `~/.bashrc` or `~/.zshrc`:

```console
export PATH="$HOME/.cargo/bin:$PATH"
```

You can also use [`binstall`](https://github.com/cargo-bins/cargo-binstall) to automatically find and install the correct `wasmtime` binary for your system, matching a candidate from [GitHub Releases](https://github.com/bytecodealliance/wasmtime/releases):  

```console
cargo binstall wasmtime-cli
```

## Compiling from Source

If you'd prefer to compile the `wasmtime` CLI from source, you'll want to
consult the [contributing documentation for building](contributing-building.md).
Be sure to use a `--release` build if you're curious to do benchmarking!
