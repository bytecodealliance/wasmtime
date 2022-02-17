# CLI Options for `wasmtime`

The `wasmtime` CLI is organized into a few subcommands. If no subcommand is
provided it'll assume `run`, which is to execute a wasm file. The subcommands
supported by `wasmtime` are:

## `help`

This is a general subcommand used to print help information to the terminal. You
can execute any number of the following:

```sh
$ wasmtime help
$ wasmtime --help
$ wasmtime -h
$ wasmtime help run
$ wasmtime run -h
```

When in doubt, try running the `help` command to learn more about functionality!

## `run`

This is the `wasmtime` CLI's main subcommand, and it's also the default if no
other subcommand is provided. The `run` command will execute a WebAssembly
module. This means that the module will be compiled to native code,
instantiated, and then optionally have an export executed.

The `wasmtime` CLI will automatically hook up any WASI-related imported
functionality, but at this time if your module imports anything else it will
fail instantiation.

The `run` command takes one positional argument which is the name of the module
to run:

```sh
$ wasmtime run foo.wasm
$ wasmtime foo.wasm
```

Note that the `wasmtime` CLI can take both a binary WebAssembly file (`*.wasm`)
as well as the text format for WebAssembly (`*.wat`):

```sh
$ wasmtime foo.wat
```

The `run` command accepts an optional `invoke` argument which is the name of
an exported function of the module to run.

```sh
$ wasmtime run foo.wasm --invoke initialize
```

## `wast`

The `wast` command executes a `*.wast` file which is the test format for the
official WebAssembly spec test suite. This subcommand will execute the script
file which has a number of directives supported to instantiate modules, link
tests, etc.

Executing this looks like:

```sh
$ wasmtime wast foo.wast
```

## `config`

This subcommand is used to control and edit local Wasmtime configuration
settings. The primary purpose of this currently is to configure [how Wasmtime's
code caching works](./cli-cache.md). You can create a new configuration file for
you to edit with:

```sh
$ wasmtime config new
```

And that'll print out the path to the file you can edit.

## `compile`

This subcommand is used to Ahead-Of-Time (AOT) compile a WebAssembly module to produce
a "compiled wasm" (.cwasm) file.

The `wasmtime run` subcommand can then be used to run a AOT-compiled WebAssembly module:

```sh
$ wasmtime compile foo.wasm
$ wasmtime foo.cwasm
```

AOT-compiled modules can be run from hosts that are compatible with the target
environment of the AOT-completed module.

## `settings`

This subcommand is used to print the available Cranelift settings for a given target.

When run without options, it will print the settings for the host target and also
display what Cranelift settings are inferred for the host:

```sh
$ wasmtime settings
```
