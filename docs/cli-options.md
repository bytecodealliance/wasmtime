# CLI Options for `wasmtime`

The `wasmtime` CLI is organized into a few subcommands. If no subcommand is
provided it'll assume `run`, which is to execute a wasm file. The subcommands
supported by `wasmtime` are:

## `help`

This is a general subcommand used to print help information to the terminal. You
can execute any number of the following:

```console
wasmtime help
wasmtime --help
wasmtime -h
wasmtime help run
wasmtime run -h
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

```console
wasmtime run foo.wasm
wasmtime foo.wasm
```

Note that the `wasmtime` CLI can take both a binary WebAssembly file (`*.wasm`)
as well as the text format for WebAssembly (`*.wat`):

```console
wasmtime foo.wat
```

#### Running WebAssembly CLI programs

WebAssembly modules or components can behave like a CLI program which means
they're intended to look like a normal OS executable with a `main` function and
run once to completion. This is the default mode of running a wasm provided to
Wasmtime.

For core WebAssembly modules this means that the function exported as an empty
string, or the `_start` export, is invoked. For WebAssembly components this
means that the `wasi:cli/run` interface is executed.

For both core modules and components, CLI arguments are passed via WASI. Core
modules receive arguments via WASIp1 APIs and components receive arguments via
WASIp2 or later APIs. Arguments, flags, etc., are passed to the WebAssembly file
after the file itself. For example,

```console
wasmtime foo.wasm --bar baz
```

Will pass `["foo.wasm", "--bar", "baz"]` as the list of arguments to the module.
Note that flags for Wasmtime must be passed before the WebAssembly file, not
afterwards. For example,

```console
wasmtime foo.wasm --dir .
```

Will pass `--dir .` to the `foo.wasm` program, not Wasmtime. If you want to
mount the current directory you instead need to invoke

```console
wasmtime --dir . foo.wasm
```

All Wasmtime options must come before the WebAssembly file provided. All
arguments afterwards are passed to the WebAssembly file itself.

#### Running Custom Module exports

If you're not running a "command" but want to run a specific export of a
WebAssembly core module you can use the `--invoke` argument:

```console
wasmtime run --invoke initialize foo.wasm
```

This will invoke the `initialize` export of the `foo.wasm` module.

When invoking a WebAssembly function arguments to the function itself are parsed
from CLI arguments. For example an `i32` argument to a WebAssembly module is
parsed as a CLI argument for the module:

```console
wasmtime run --invoke add add.wasm 1 2
```

Note though that this syntax is unstable at this time and may change in the
future. If you'd like to rely on this please open an issue, otherwise we request
that you please don't rely on the exact output here.

#### Running Custom Component exports

Like core modules Wasmtime supports invoking arbitrary component exports.
Components can export typed interfaces defined by [the component
model](https://component-model.bytecodealliance.org/design/components.html). The
`--invoke` argument is supported to skip calling `wasi:cli/run` and invoke a
specific typed export instead. Arguments are passed with
[WAVE](https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wasm-wave#readme)
,a human-oriented text encoding of Wasm Component Model values. For example:

```console
wasmtime run --invoke 'initialize()' foo.wasm
```

You will notice that (when using WAVE) the exported function's name and exported
function's parentheses are both enclosed in one set of single quotes, i.e.
`'initialize()'`. This treats the exported function as a single argument in a
Unix shell and prevents issues with shell interpretation and signifies function
invocation (as apposed to the function name just being referenced). Using WAVE
(when calling exported functions of Wasm components) helps to distinguish
function calls from other kinds of string arguments. Below are some more
examples:

If your function takes a string argument, you surround the string argument in
double quotes:

```console
wasmtime run --invoke 'initialize("hello")' foo.wasm
```

And each individual argument within the parentheses is separated by a comma:

```console
wasmtime run --invoke 'initialize("Pi", 3.14)' foo.wasm
wasmtime run --invoke 'add(1, 2)' foo.wasm
```

**Please note:** If you enclose your whole function call using double quotes,
your string argument will require its double quotes to be escaped (escaping
quotes is more complicated and harder to read and therefore not ideal). For
example:

```bash
wasmtime run - invoke "initialize(\"hello\")" foo.wasm
```

## `serve`

The `serve` subcommand runs a WebAssembly component in the `wasi:http/proxy`
world via the WASI HTTP API, which is available since Wasmtime 18.0.0. The goal
of this world is to support sending and receiving HTTP requests.

The `serve` command takes one positional argument which is the name of the
component to run:

```console
wasmtime serve foo.wasm
```

Furthermore, an address can be specified via:

```console
wasmtime serve --addr=0.0.0.0:8081 foo.wasm
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

```console
wasmtime wast foo.wast
```

## `config`

This subcommand is used to control and edit local Wasmtime configuration
settings. The primary purpose of this currently is to configure [how Wasmtime's
code caching works](./cli-cache.md). You can create a new configuration file for
you to edit with:

```console
wasmtime config new
```

And that'll print out the path to the file you can edit.

## `compile`

This subcommand is used to Ahead-Of-Time (AOT) compile a WebAssembly module to produce
a "compiled wasm" (.cwasm) file.

The `wasmtime run` subcommand can then be used to run a AOT-compiled WebAssembly module:

```console
wasmtime compile foo.wasm
wasmtime foo.cwasm
```

AOT-compiled modules can be run from hosts that are compatible with the target
environment of the AOT-completed module.

## `settings`

This subcommand is used to print the available Cranelift settings for a given target.

When run without options, it will print the settings for the host target and also
display what Cranelift settings are inferred for the host:

```console
wasmtime settings
```

## `explore`

This subcommand can be used to explore a `*.cwasm` file and see how it connects
to the original wasm file in a web browser. This will compile an input wasm
file and emit an HTML file that can be opened in a web browser:

```console
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

```console
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

```console
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

```console
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
```console
wasmtime compile --config config.toml
```
would be the same as
```console
wasmtime compile --optimize opt-level=0
```
assuming the TOML file is called `config.toml`. Of course you can put as many
key-value pairs as you want in the TOML file.

Options on the CLI take precedent over options specified in a configuration
file, meaning they're allowed to shadow configuration values in a TOML
configuration file.
