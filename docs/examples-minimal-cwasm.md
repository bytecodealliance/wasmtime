# Building a minimal `*.cwasm`

In addition to building a [minimal embedding] embedders may also want to
minimize the size of their `*.cwasm` they're compiling as well.  These size of a
`*.cwasm` affects the in-memory size of a compiled wasm module on a device, for
example, and thus minimizing that can lead to freeing up resources to use
elsewhere.

As with building a [minimal embedding] wasmtime is by default not optimized for
this use case, so some knobs will need to be turned to enable this. The first
step to building a minimal `*.cwasm` is building a minimal wasm itself. To that
extent many of the instructions on [minimal embedding] about recompiling code
with smaller options apply here too. This example will walk through compiling a
Rust "hello world" program and optimizing the size of the output `*.cwasm`.

The source code we have here is:

```rust
fn main() {
    println!("Hello, world!");
}
```

The defaults are:

```shell-session
$ rustc foo.rs --target wasm32-wasip2
$ wasmtime compile foo.wasm
$ ls -lh foo.wasm foo.cwasm
-rw-rw-r-- 1 alex alex 284K Jun 12 16:58 foo.cwasm
-rw-rw-r-- 1 alex alex 2.5M Jun 12 16:58 foo.wasm
```

While this looks like Wasmtime was able to shave off ~2M of data here what's
actually happening is that the Rust compiler is preserving DWARF debug info by
default. Wasmtime strips guest-DWARF information by default, however, so the
first step to minimizing a wasm is to strip out unnecessary custom sections
like this:

```shell-session
$ rustc foo.rs --target wasm32-wasip2 -Cstrip=debuginfo
$ wasmtime compile foo.wasm
$ ls -lh foo.wasm foo.cwasm
-rw-rw-r-- 1 alex alex 284K Jun 12 16:59 foo.cwasm
-rw-rw-r-- 1 alex alex  78K Jun 12 16:59 foo.wasm
```

Here we can see that compiled `*.cwasm` files are often larger than their
corresponding `*.wasm` file. This is expected and generally always going to be
the case. First though let's apply many learnings from a [minimal embedding] and
[general purpose Rust advice for compiling minimal binaries][min-sized-rust] to
shrink the size of this wasm module. Note that here we're compiling with rustc
manually, but for Cargo or other projects it'll look similar.

[min-sized-rust]: https://github.com/johnthagen/min-sized-rust

```shell-session
$ rustc foo.rs --target wasm32-wasip2 -Copt-level=s -Clto -Ccodegen-units=1 -Cstrip=debuginfo
$ wasmtime compile foo.wasm
$ ls -lh foo.wasm foo.cwasm
-rw-rw-r-- 1 alex alex 219K Jun 12 17:02 foo.cwasm
-rw-rw-r-- 1 alex alex  64K Jun 12 17:02 foo.wasm
```

Optimizations, LTO, reducing codegen units, etc, all reduce the size of this
input `*.wasm` file by ~14k in this case. This additionally reflects a general
trend where `*.cwasm` is proportional to the size of the input `*.wasm`, so it
shrunk appropriately as well. At this point we'll assume that the input `*.wasm`
is as small as can be and shift to Wasmtime-specific optimizations.

Like with the documentation of a [minimal embedding] the trend here is that by
removing features of Wasmtime or the compiled artifact you'll be able to shrink
the output. First what we can do is to disable Wasmtime's "address maps":

```shell-session
$ rustc foo.rs --target wasm32-wasip2 -Copt-level=s -Clto -Ccodegen-units=1 -Cstrip=debuginfo
$ wasmtime compile foo.wasm -Daddress-map=n
$ ls -lh foo.wasm foo.cwasm
-rw-rw-r-- 1 alex alex 167K Jun 12 17:04 foo.cwasm
-rw-rw-r-- 1 alex alex  64K Jun 12 17:04 foo.wasm
```

Address maps are used by Wasmtime to generate a backtrace that refers to
WebAssembly program counters information in the output file. These counters can
be coupled with in-WebAssembly DWARF to generate backtraces with filenames and
line numbers pointing back to the source. For this use case though this can all
be safely stripped out as we won't be using it.

The next optimization is to disable debug symbols in Wasmtime:

```shell-session
$ rustc foo.rs --target wasm32-wasip2 -Copt-level=s -Clto -Ccodegen-units=1 -Cstrip=debuginfo
$ wasmtime compile foo.wasm -Daddress-map=n -Dsymbols=n
$ ls -lh foo.wasm foo.cwasm
-rw-rw-r-- 1 alex alex 143K Jun 12 17:06 foo.cwasm
-rw-rw-r-- 1 alex alex  64K Jun 12 17:06 foo.wasm
```

Wasmtime's `.cwasm` artifacts are designed to integrate with the system's native
profiler and other developer tools like `wasmtime objdump` by default, but this
information isn't needed to actually run the program and is safe to remove.

The final optimization is noticing that the original wasm's [`name` custom
section][name section] is actually still present. This section generally survives stripping
because of how useful it is for debugging, but for the absolutely minimal size
it can be stripped away:

[name section]: https://webassembly.github.io/spec/core/appendix/custom.html#name-section

```shell-session
$ rustc foo.rs --target wasm32-wasip2 -Copt-level=s -Clto -Ccodegen-units=1 -Cstrip=debuginfo
$ wasm-tools strip -a foo.wasm -o foo.wasm
$ wasmtime compile foo.wasm -Daddress-map=n -Dsymbols=n
$ ls -lh foo.wasm foo.cwasm
-rw-rw-r-- 1 alex alex 132K Jun 12 17:08 foo.cwasm
-rw-rw-r-- 1 alex alex  50K Jun 12 17:08 foo.wasm
```

The final output has virtually no debugging information in it for when anything
goes wrong, so all you'll get are function indices and not much else.

The next optimization is when runtime performance of this module will start
being affected. Wasmtime precomputes an image of linear memory for
initialization and page-aligns it, but this page-alignment and precomputation
can add fair amount of empty space in the output file. This can be disabled to
avoid CoW initialization and instead manually initialize all linear memories:

```shell-session
$ rustc foo.rs --target wasm32-wasip2 -Copt-level=s -Clto -Ccodegen-units=1 -Cstrip=debuginfo
$ wasm-tools strip -a foo.wasm -o foo.wasm
$ wasmtime compile foo.wasm -Daddress-map=n -Dsymbols=n -Omemory-init-cow=n
$ ls -lh foo.wasm foo.cwasm
-rw-rw-r-- 1 alex alex 129K Jun 12 17:11 foo.cwasm
-rw-rw-r-- 1 alex alex  50K Jun 12 17:11 foo.wasm
```

The final optimization is that Wasmtime's interpreter, Pulley, can sometimes
have smaller output than native machine output. This is another hit on runtime
performance, but for the sake of example:

```shell-session
$ rustc foo.rs --target wasm32-wasip2 -Copt-level=s -Clto -Ccodegen-units=1 -Cstrip=debuginfo
$ wasm-tools strip -a foo.wasm -o foo.wasm
$ wasmtime compile foo.wasm -Daddress-map=n -Dsymbols=n -Omemory-init-cow=n --target pulley64
$ ls -lh foo.wasm foo.cwasm
-rw-rw-r-- 1 alex alex 90K Jun 12 17:12 foo.cwasm
-rw-rw-r-- 1 alex alex 50K Jun 12 17:12 foo.wasm
```

At this time this is the smallest binary that Wasmtime can generate. If this is
not small enough for you please feel free to file an issue and Wasmtime
maintainers can help debug if there's any more low-hanging fruit to pick.

## Minimizing `*.cwasm`: Summary

The steps you'll want to use when minimizing `*.cwasm` size are:

* Minimize the size of the input `*.wasm`.
  * Compile with optimizations.
  * Strip debug info.
  * Strip the `name` section.
  * Apply language-specific optimizations like LTO, codegen units, rebuilding
    Rust's libstd, etc.
* Pass `-Daddress-map=n` to disable the ability to generate backtraces with wasm
  pc's in the backtrace.
* Pass `-Dsymbols=n` to diasble symbols used for debugging/profiling in the
  output artifact.
* Pass `-Omemory-init-cow=n` to disable page-aligned data sections and
  precomputation of a memory image that may have holes in it.
* Pass `--target pulley64` to leverage "macro opcodes" in Pulley to compress
  instructions a bit further.

And, failing that, feel free to file an issue!

[minimal embedding]: ./examples-minimal.md
