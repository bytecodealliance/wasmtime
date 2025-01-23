# Using Pulley

On architectures such as x86\_64 or aarch64 Wasmtime will by default use the
Cranelift compiler to translate WebAssembly to native machine code and execute
it. Cranelift does not support all architectures, however, for example i686
(32-bit Intel machines) is not supported at this time. To help execute
WebAssembly on these architectures Wasmtime comes with an interpreter called
Pulley.

Pulley is a bytecode interpreter originally proposed [in an RFC][rfc] which is
intended to primarily be portable. Pulley is a loose backronym for "Portable,
Universal, Low-Level Execution strategY" but mostly just a theme on
machines/tools (Cranelift, Winch, Pulley, ...). Pulley is a distinct target and
execution environment for Wasmtime.

## Enabling Pulley

The Pulley interpreter is enabled via one of two means:

1. On architectures which have Cranelift support, Pulley must be enabled via the
   `pulley` crate feature of the `wasmtime` crate. This feature is otherwise
   off-by-default.

2. On architectures which do NOT have Cranelift support, Pulley is already
   enabled by default. This means that Wasmtime can execute WebAssembly by
   default on any platform, it'll just be faster on Cranelift-supported
   platforms.

For platforms in category (2) there is no opt-in necessary to execute Pulley as
that's already the default target. Platforms in category (1), such as
`x86_64-unknown-linux-gnu`, may still want to execute Pulley to run tests,
evaluate the implementation, benchmark, etc.

To force execution of Pulley on any platform the `pulley` crate feature of
the `wasmtime` crate must be enabled in addition to configuring a target.
Specifying a target is done with the `--target` CLI option to the `wasmtime`
executable, the [`Config::target`] method in Rust, or the
[`wasmtime_config_target_set`] C API. The target string for pulley must be one
of:

[`Config::target`]: https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html#method.target
[`wasmtime_config_target_set`]: https://docs.wasmtime.dev/c-api/config_8h.html#ae68a2737ba1680e75cddb6ede08d682a

* `pulley32` - for 32-bit little-endian hosts
* `pulley32be` - for 32-bit big-endian hosts
* `pulley64` - for 64-bit little-endian hosts
* `pulley64be` - for 64-bit big-endian hosts

The Pulley target string must match the environment that the Pulley Bytecode
will be executing in. Some examples of Pulley targets are:

| Host target                | Pulley target |
|----------------------------|---------------|
| `x86_64-unknown-linux-gnu` | `pulley64`    |
| `i686-unknown-linux-gnu`   | `pulley32`    |
| `s390x-unknown-linux-gnu`  | `pulley64be`  |

Wasmtime will return an error trying to load bytecode compiled for the wrong
Pulley target. When Pulley is the default target for a particular host then the
correct Pulley target will be selected automatically. Specifying the Pulley
target may still be necessary when cross-compiling from one platform to another,
however.

## Using Pulley

Using Pulley in Wasmtime requires no further configuration beyond specifying the
target for Pulley. Once that is done all of the Wasmtime crate's Rust APIs or C
API work as usual. For example when specifying `wasmtime run --target pulley64`
on the CLI this will execute all WebAssembly in the interpreter rather than via
Cranelift.

Pulley at this time has the same feature parity for WebAssembly as Cranelift
does. This means that all WebAssembly proposals and features supported by
Wasmtime are supported by Pulley.

If you notice anything awry, however, please feel free to file an issue.

## Impact of using Pulley

Pulley is an interpreter for its own bytecode format. While the design of Pulley
is optimized for speed you should still expect a ~10x order-of-magnitude
slowdown relative to native code or Cranelift. This means that Pulley is likely
not suitable for compute-intensive tasks that must run in as little time as
possible.

The primary goal of Pulley is to enable using and embedding Wasmtime across a
variety of platforms simultaneously. The same API/interface is used to interact
with the runtime and loading WebAssembly module regardless of the host
architecture.

Pulley bytecode is produced by the Cranelift compiler today in a similar manner
to native platforms. Pulley is not designed for quickly loading WebAssembly
modules as Cranelift is an optimizing compiler. Compiling WebAssembly to Pulley
bytecode should be expected to take about the same time as compiling to native
platforms.

## High-level Design of Pulley

This section is not necessary for users of Pulley but for those interested this
is a description of the high-level design of Pulley. The Pulley virtual machine
consists of:

* 32 "X" integer registers each of which are 64-bits large. (`XReg`)
* 32 "F" float registers each of which are 64-bits large. (`FReg`)
* 32 "V" vector registers each of which are 128-bits large. (`VReg`)
* A dynamically allocated "stack" on the host's heap.
* A frame pointer register.
* A link register to store the return address for the current function.

This state lives in [`MachineState`] which is in turned stored in a [`Vm`].
Pulley's source code lives in `pulley/` in the Wasmtime repository.

Pulley's bytecode is defined in `pulley/src/lib.rs` with a combination of the
`for_each_op!` and `for_each_extended_op!` macros. Opcode numbers and opcode
layout are defined by the structure of these macros. The macros are used to
"derive" encoding/decoding/traits/etc used throughout the `pulley_interpreter`
crate.

Pulley opcodes are a single discriminator byte followed by any immediates.
Immediates are not aligned and require unaligned loads/stores to work with them.
Pulley has more than 256 opcodes, however, which is where "extended" opcodes
come into play. The final Pulley opcode is reserved to indicate that an extended
opcode is being used. Extended opcodes follow this initial discriminator with a
16-bit integer which further indicates which extended opcode is being used. This
design is intended to allow common operations to be encoded more compactly while
less common operations can still be packed in effectively without limit.

Pulley opcode assignment happens through the order of the `for_each_op!` macro
which means that it's not portable across multiple versions of Wasmtime.

The interpreter is an implementation of the [`OpVisitor`] and
[`ExtendedOpVisitor`] traits. This is located at `pulley/src/interp.rs`. Notably
this means that there's a method-per-opcode and is how the interpreter is
implemented.

The interpreter loop itself is implemented in one of two ways:

1. A "match loop" which is a Rust `loop { ... }` which internally uses the
   [`Decode`] trait on each opcode. This is not literally modeled as but
   compiles down to something that looks like `loop { match .. { ... } }`. This
   interpreter loop is located at `pulley/src/interp/match_loop.rs`.

2. A "tail loop" were each opcode handler is a Rust function. Control flow
   between opcodes continues with tail-calls and exiting the interpreter is done
   by returning from the function. Tail calls are not available in stable Rust
   so this interpreter loop is not used by default. It can be enabled, though,
   with `RUSTFLAGS=--cfg=pulley_assume_llvm_makes_tail_calls` to rely on LLVM's
   tail-call-optimization pass to implement the loop.

The "match loop" is the default interpreter loop as it's portable and works on
stable Rust. The "tail loop" is thought to probably perform better than the
"match loop" but it's not available on stable Rust (`become` in Rust is an
unfinished nightly feature at this time) or portable (tail-call-optimization
doesn't happen the same in LLVM on all architectures).

### Inspecting Pulley Bytecode

When compiling to native the `*.cwasm` produced by `wasmtime compile` can be
inspected with `objdump -S`, but this doesn't work with Pulley. A small example
in the `pulley_interpreter` crate suffices for doing this though. You can
inspect compiled Pulley bytecode from the Wasmtime repository with:

```sh
$ cargo run compile --target pulley64 foo.wat
$ cargo run -p pulley-interpreter --all-features --example objdump foo.cwasm
0x000000: <wasm[0]::function[20]>:
       0: 9f 10 00 08 00                     push_frame_save 16, x19
       5: 40 13 00                           xmov x19, x0
       8: 03 13 13 3f cb 89 00               call2 x19, x19, 0x89cb3f    // target = 0x89cb47
       f: 03 13 13 8c ab 84 00               call2 x19, x19, 0x84ab8c    // target = 0x84ab9b
      16: 03 13 13 5b 12 00 00               call2 x19, x19, 0x125b    // target = 0x1271
      1d: 03 13 13 9f 12 00 00               call2 x19, x19, 0x129f    // target = 0x12bc
      24: 03 13 13 e0 45 00 00               call2 x19, x19, 0x45e0    // target = 0x4604
...
```

The output is intended to look somewhat similar to `objdump` but otherwise
mainly provides the ability to inspect opcode selection, see the encoded bytes,
etc.

### Profiling Pulley

Profiling the Pulley interpreter can be done with native profiler such as `perf`
but this has a few downsides:

* When profiling the "match loop" it's not clear what machine code corresponds
  to which Pulley opcode. Most of the time all the samples are just in the one
  big "run" function.

* When profiling with the "tail loop" you can see hot opcodes much more clearly,
  but it can be difficult to understand why a particular opcode was chosen.

It can sometimes be more beneficial to see time spent per Pulley opcode itself
in the context of the all Pulley opcodes. In a similar manner as you can look at
instruction-level profiling in `perf` it can be useful to look at opcode-level
profiling of Pulley.

Pulley has limited support for opcode-level profiling. This is off-by-default as
it has a performance hit for the interpreter. To collect a profile with the
`wasmtime` CLI you'll have to build from source and enable the `profile-pulley`
feature:

```sh
$ cargo run --features profile-pulley --release run --profile pulley --target pulley64 foo.wat
```

This will compile an optimized `wasmtime` executable with the `profile-pulley`
Cargo feature enabled. The `--profile pulley` flag can then be passed to the
`wasmtime` CLI to enable the profiler at runtime.

The command will emit a `pulley-$pid.data` file which contains raw data about
Pulley opcodes and samples taken. To view this file you can use:

```sh
$ cargo run -p pulley-interpreter --example profiler-html --all-features ./pulley-$pid.data
```

This will load the `pulley-*.data` file, parse it, collate the results, and
display the hottest functions. The hottest function is emitted last and
instructions are annotated with the `%` of samples taken that were executing at
that instruction.

Some more information can be found in [the PR that implemented Pulley profiling
support][profile-pr]

[`OpVisitor`]: https://docs.rs/pulley-interpreter/latest/pulley_interpreter/decode/trait.OpVisitor.html
[`MachineState`]: https://docs.rs/pulley-interpreter/latest/pulley_interpreter/interp/struct.MachineState.html
[`Vm`]: https://docs.rs/pulley-interpreter/latest/pulley_interpreter/interp/struct.Vm.html
[rfc]: https://github.com/bytecodealliance/rfcs/blob/main/accepted/pulley.md
[`ExtendedOpVisitor`]: https://docs.rs/pulley-interpreter/latest/pulley_interpreter/decode/trait.ExtendedOpVisitor.html
[`Decode`]: https://docs.rs/pulley-interpreter/latest/pulley_interpreter/decode/trait.Decode.html
[profile-pr]: https://github.com/bytecodealliance/wasmtime/pull/10034
