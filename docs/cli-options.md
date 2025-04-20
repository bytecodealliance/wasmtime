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
functionality, but at this time, if your module imports anything else, it will
fail instantiation.

The `run` command takes one positional argument, which is the name of the module to run:

```sh
$ wasmtime run foo.wasm
$ wasmtime foo.wasm
```

Note that the `wasmtime` CLI can take both a binary WebAssembly file (`*.wasm`)
as well as the text format for WebAssembly (`*.wat`):

```sh
$ wasmtime foo.wat
```

**Wasm Modules**

A Wasm **module** exports raw functions directly. The `run` command accepts an optional `--invoke` argument, which is the name of an exported raw function (of the module) to run:

```sh
$ wasmtime run --invoke initialize foo.wasm
```

**Wasm Components**

A Wasm **component** uses typed interfaces defined by [the component model](https://component-model.bytecodealliance.org/design/components.html). The `run` command also accepts the optional `--invoke` argument for calling an exported function of a **component**. However, the calling of an exported function of a component uses [WAVE](https://github.com/bytecodealliance/wasm-tools/tree/a56e8d3d2a0b754e0465c668f8e4b68bad97590f/crates/wasm-wave#readme)(a human-oriented text encoding of Wasm Component Model values). For example:

```sh
$ wasmtime run --invoke 'initialize()' foo.wasm
```

You will notice that (when using WAVE) the exported function's name and exported function's parentheses are both enclosed in one set of single quotes, i.e. `'initialize()'`. This treats the exported function as a single argument, prevents issues with shell interpretation and signifies function invocation (as apposed to the function name just being referenced). Using WAVE (when calling exported functions of Wasm components) helps to distinguish function calls from other kinds of string arguments. Below are some more examples:

If your function takes a string argument, you surround the string argument in double quotes:

```sh
$ wasmtime run --invoke 'initialize("hello")' foo.wasm
```

And each individual argument within the parentheses is separated by a comma:

```sh
$ wasmtime run --invoke 'initialize("Pi", 3.14)' foo.wasm
$ wasmtime run --invoke 'add(1, 2)' foo.wasm
```

**Please note:** If you enclose your whole function call using double quotes, your string argument will require its double quotes to be escaped (escaping quotes is more complicated and harder to read and therefore not ideal). For example:
```bash
wasmtime run - invoke "initialize(\"hello\")" foo.wasm
```

## `serve`

The `serve` subcommand runs a WebAssembly component in the `wasi:http/proxy`
world via the WASI HTTP API, which is available since Wasmtime 18.0.0. The goal
of this world is to support sending and receiving HTTP requests.

The `serve` command takes one positional argument which is the name of the
component to run:

```sh
$ wasmtime serve foo.wasm
```

Furthermore, an address can be specified via:

```sh
$ wasmtime serve --addr=0.0.0.0:8081 foo.wasm
```

At the time of writing, the `wasi:http/proxy` world is still experimental and
requires setup of some `wit` dependencies. For more information, see
the [hello-wasi-http](https://github.com/sunfishcode/hello-wasi-http/) example.

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

## `explore`

This subcommand can be used to explore a `*.cwasm` file and see how it connects
to the original wasm file in a web browser. This will compile an input wasm
file and emit an HTML file that can be opened in a web browser:

```sh
$ wasmtime explore foo.wasm
Exploration written to foo.explore.html
```

The output HTML file can be used to compare what WebAssembly instruction
compiles to what native instruction. Compilation options can be passed to
`wasmtime explore` to see the effect of compilation options on generated code.

## `objdump`

Primarily intended as a debugging utility the `objdump` subcommand can be used
to explore a `*.cwasm` file locally on your terminal. This is roughly modeled
after native `objdump` binaries themselves:

```sh
$ wasmtime objdump foo.cwasm
wasm[0]::function[0]:
            stp     x29, x30, [sp, #-0x10]!
            mov     x29, sp
            ldr     x5, [x2, #0x50]
            lsl     w6, w4, #2
            ldr     w2, [x5, w6, uxtw]
            ldp     x29, x30, [sp], #0x10
            ret
```

You can also pass various options to configure and annotate the output:

```sh
$ wasmtime objdump foo.cwasm --addresses --bytes --addrma
00000000 wasm[0]::function[0]:
         0: fd 7b bf a9                  stp     x29, x30, [sp, #-0x10]!
         4: fd 03 00 91                  mov     x29, sp
         8: 45 28 40 f9                  ldr     x5, [x2, #0x50]
                                          ╰─╼ addrmap: 0x23
         c: 86 74 1e 53                  lsl     w6, w4, #2
                                          ╰─╼ addrmap: 0x22
        10: a2 48 66 b8                  ldr     w2, [x5, w6, uxtw]
                                          ╰─╼ addrmap: 0x23
        14: fd 7b c1 a8                  ldp     x29, x30, [sp], #0x10
                                          ╰─╼ addrmap: 0x26
        18: c0 03 5f d6                  ret
```

# Additional options
Many of the above subcommands also take additional options. For example,
- run
- serve
- compile
- explore
- wast

are all subcommands which can take additional CLI options of the format

```sh
Options:
  -O, --optimize <KEY[=VAL[,..]]>
          Optimization and tuning related options for wasm performance, `-O help` to see all

  -C, --codegen <KEY[=VAL[,..]]>
          Codegen-related configuration options, `-C help` to see all

  -D, --debug <KEY[=VAL[,..]]>
          Debug-related configuration options, `-D help` to see all

  -W, --wasm <KEY[=VAL[,..]]>
          Options for configuring semantic execution of WebAssembly, `-W help` to see all

  -S, --wasi <KEY[=VAL[,..]]>
          Options for configuring WASI and its proposals, `-S help` to see all
```

For example, adding `--optimize opt-level=0` to a `wasmtime compile` subcommand
will turn off most optimizations for the generated code.

## CLI options using TOML file
Most key-value options that can be provided using the `--optimize`, `--codegen`,
`--debug`, `--wasm`, and `--wasi` flags can also be provided using a TOML
file using the `--config <FILE>` cli flag, by putting the key-value inside a TOML
table with the same name.

For example, with a TOML file like this
```toml
[optimize]
opt-level = 0
```
the command
```sh
$ wasmtime compile --config config.toml
```
would be the same as
```sh
$ wasmtime compile --optimize opt-level=0
```
assuming the TOML file is called `config.toml`. Of course you can put as many
key-value pairs as you want in the TOML file.

Options on the CLI take precedent over options specified in a configuration
file, meaning they're allowed to shadow configuration values in a TOML
configuration file.
