# Debugging Cranelift Code Generation

This guide describes strategies for debugging issues in Cranelift's code
generation — wrong output, compiler panics, miscompilations, and register
allocation failures.

## Printing IR at various pipeline stages

### Print CLIF input

To see the CLIF IR that a function starts with, use `clif-util cat`:

```
cargo run -p cranelift-tools -- cat path/to/function.clif
```

To print CLIF at key stages during normal compilation, enable trace logging:

```
RUST_LOG=cranelift_codegen=trace cargo test ...
```

### Print optimized CLIF

Use a `test optimize` filetest (see [Testing Cranelift](testing.md)) to observe
the IR after mid-end optimization passes:

```
test optimize
target x86_64

function %my_func(i32, i32) -> i32 {
...
}
; check: ...
```

Run with:

```
cargo run -p cranelift-tools -- test path/to/test.clif
```

### Print VCode (machine instructions before regalloc)

Enable the `trace` log level for `cranelift_codegen::machinst`:


```
RUST_LOG=cranelift_codegen::machinst=trace cargo test ...
```

This prints the VCode (virtual-register machine instructions) after lowering
but before register allocation.

### Print the final compiled output

Use a `test compile` filetest with filecheck directives to inspect the
instruction sequence after register allocation:

```
test compile
target aarch64

function %add(i32, i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    return v2
}
; check: add w0, w0, w1
```

## Minimizing a failing test case

When a miscompilation or panic occurs in Wasmtime or another consumer, it
helps enormously to reduce the input to a small `.clif` file.

### Manual minimization

1. Dump the CLIF IR of the failing function. In Wasmtime, set
   `WASMTIME_LOG=cranelift_codegen=trace` and look for the function's CLIF
   output in stderr.

2. Save the function to a `.clif` file and run:
   ```
   cargo run -p cranelift-tools -- test path/to/test.clif
   ```

3. Manually simplify the function: remove instructions, replace values with
   constants, shrink the number of blocks, etc. After each change, confirm the
   bug is still reproducible.

### Fuzzer-assisted minimization

If the bug was found via fuzzing (`cranelift/fuzzgen`), the fuzzer's corpus
entry can be minimized using `cargo-fuzz tmin`. See the
[fuzzgen README](../fuzzgen/README.md) for details.

## Diffing output against a reference

When debugging a miscompilation (the code compiles but produces wrong results),
it can be useful to compare the output of two different configurations.

### Compare optimized vs. unoptimized

Compile with `opt_level=none` and `opt_level=speed` and compare the outputs:

```
test run
set opt_level=none
target x86_64

function %foo(i32) -> i32 { ... }
; run: %foo(42) == 42
```

```
test run
set opt_level=speed
target x86_64

function %foo(i32) -> i32 { ... }
; run: %foo(42) == 42
```

### Compare two backends

Use `test run` with different `target` lines to run the same function on
different ISAs and compare results. The interpreter (`test interpret`) provides
a reference implementation that does not go through native code generation.

### Using `clif-util compile --disasm`

```
cargo run -p cranelift-tools -- compile --disasm --target aarch64 path/to/test.clif
```

This prints the generated machine code as both hex and disassembly.

## Debugging register allocation issues

Register allocation errors from regalloc2 are reported with a dump of the
VCode. Key things to check:

- Is `get_operands` correct? Every register read by an instruction must be
  listed as a `reg_use`, every register written must be listed as `reg_def`.
  A missing `reg_use` can cause a use-before-def error; a missing `reg_def`
  can cause wrong-register assignment.
- Are block parameters (`block_params_succ` / `block_params`) correct?
- Does the terminator instruction list its target blocks correctly?

Enable the regalloc checker for additional verification:

```
set regalloc_checker=true
```

This adds a post-allocation correctness check that verifies every register
use is preceded by a definition or block-parameter assignment.

Enable verbose regalloc logs:

```
RUST_LOG=regalloc2=debug cargo test ...
```

## Using "optimization fuel" / chaos mode

Cranelift supports a `ControlPlane` mechanism that can randomize certain
compilation decisions (such as block layout). This is useful for finding
latent bugs that only manifest with a particular ordering.

Chaos mode is enabled via a Cargo feature. Build with the `chaos` feature to
activate it:

```
cargo test -p cranelift-filetests --features cranelift-control/chaos
```

When a miscompilation only appears under chaos mode, run tests repeatedly or
use a fuzzer seed to reproduce the specific ordering that triggers the bug.

## Debugging ISLE rule selection

### See which rule fired

Set `RUST_LOG=cranelift_codegen::machinst::isle=trace` to see which ISLE rules
are selected during lowering.

### Inspect generated ISLE code

To see the Rust code that ISLE generated:

```
ISLE_SOURCE_DIR=$(pwd)/isle-out cargo check -p cranelift-codegen
```

The generated code is placed in `./isle-out/`. You can set breakpoints in it or
add `eprintln!` statements to trace rule execution.

### ISLE error reporting

For pretty-printed ISLE type errors with source context, build with:

```
cargo build -p cranelift-codegen --features isle-errors
```

## Common panics and their causes

| Panic message | Likely cause |
|---|---|
| `no rule for opcode X in lower` | Backend missing a lowering rule for that CLIF opcode. Add an ISLE rule or return a more helpful error. |
| `register class mismatch` | `get_operands` returned a wrong `RegClass` for a register, or the instruction is being given a register of the wrong class. |
| `assertion failed: is_term` | A basic block's last instruction is not a terminator (branch or return). |
| `MachBuffer: island required` | An instruction's branch or constant reference is out of range and the `MachBuffer` island logic did not insert an island. Increase island frequency or check `worst_case_size`. |
| `regalloc2: live range crosses call without clobber` | A value is live across a call but the register assigned to it is not callee-saved. Check that the ABI's callee-saved register list is correct. |

## Useful environment variables

| Variable | Effect |
|---|---|
| `RUST_LOG=cranelift_codegen=debug` | Enable debug-level tracing in cranelift-codegen |
| `RUST_LOG=regalloc2=debug` | Enable regalloc2 verbose logging |
| `CRANELIFT_FILETESTS_THREADS=1` | Run filetests single-threaded (for cleaner output) |
| `ISLE_SOURCE_DIR=<path>` | Write ISLE-generated Rust to `<path>` instead of `target/` |
