# Testing Cranelift

Cranelift is tested at multiple levels of abstraction and integration. When
possible, Rust unit tests are used to verify single functions and types. When
testing the interaction between compiler passes, file-level tests are
appropriate.

## Rust tests

Rust and Cargo have good support for testing. Cranelift uses unit tests, doc
tests, and integration tests where appropriate. The
[Rust By Example page on Testing] is a great illustration on how to write
each of these forms of test.

[Rust By Example page on Testing]: https://doc.rust-lang.org/rust-by-example/testing.html

## File tests

Compilers work with large data structures representing programs, and it quickly
gets unwieldy to generate test data programmatically. File-level tests make it
easier to provide substantial input functions for the compiler tests.

File tests are `*.clif` files in the `filetests/` directory hierarchy. Each
file has a header describing what to test followed by a number of input
functions in the [Cranelift textual intermediate representation](ir.md):

```
test_file     ::= test_header function_list
test_header   ::= test_commands (isa_specs | settings)
test_commands ::= test_command { test_command }
test_command  ::= "test" test_name { option } "\n"
```

The available test commands are described below.

Many test commands only make sense in the context of a target instruction set
architecture. These tests require one or more ISA specifications in the test
header:

```
isa_specs ::= { [settings] isa_spec }
isa_spec  ::= "isa" isa_name { option } "\n"
```

The options given on the `isa` line modify ISA-specific settings.

All types of tests allow shared Cranelift settings to be modified:

```
settings ::= { setting }
setting  ::= "set" { option } "\n"
option   ::= flag | setting "=" value
```

The `set` lines apply settings cumulatively:

```
test compile
set opt_level=best
set is_pic=1
target riscv64
set is_pic=0
target riscv64 has_m=false

function %foo() {}
```

This example will run the compile test twice. Both runs will have
`opt_level=best`, but they will have different `is_pic` settings. The second
run will also have the RISC-V specific flag `has_m` disabled.

The filetests are run automatically as part of `cargo test`, and they can
also be run manually with the `clif-util test` command.

By default, the test runner will spawn a thread pool with as many threads as
there are logical CPUs. You can explicitly control how many threads are spawned
via the `CRANELIFT_FILETESTS_THREADS` environment variable. For example, to
limit the test runner to a single thread, use:

```
$ CRANELIFT_FILETESTS_THREADS=1 clif-util test path/to/file.clif
```

### Filecheck

Many of the test commands described below use *filecheck* to verify their
output. Filecheck is a Rust implementation of the LLVM tool of the same name.
See the [filecheck documentation](https://docs.rs/filecheck/) for details of
its syntax.

Comments in `.clif` files are associated with the entity they follow.
This typically means an instruction or the whole function. Those tests that
use filecheck will extract comments associated with each function (or its
entities) and scan them for filecheck directives. The test output for each
function is then matched against the filecheck directives for that function.

Comments appearing before the first function in a file apply to every function.
This is useful for defining common regular expression variables with the
`regex:` directive, for example.

Note that LLVM's file tests don't separate filecheck directives by their
associated function. It verifies the concatenated output against all filecheck
directives in the test file. LLVM's `FileCheck` command has a `CHECK-LABEL:`
directive to help separate the output from different functions. Cranelift's
tests don't need this.

### `test cat`

This is one of the simplest file tests, used for testing the conversion to and
from textual IR. The `test cat` command simply parses each function and
converts it back to text again. The text of each function is then matched
against the associated filecheck directives.

Example:

```
function %r1() -> i32, f32 {
block1:
    v10 = iconst.i32 3
    v20 = f32const 0.0
    return v10, v20
}
; sameln: function %r1() -> i32, f32 {
; nextln: block0:
; nextln:     v10 = iconst.i32 3
; nextln:     v20 = f32const 0.0
; nextln:     return v10, v20
; nextln: }
```

### `test verifier`

Run each function through the IR verifier and check that it produces the
expected error messages.

Expected error messages are indicated with an `error:` directive *on the
instruction that produces the verifier error*. Both the error message and
reported location of the error is verified:

```
test verifier

function %test(i32) {
    block0(v0: i32):
        jump block1       ; error: terminator
        return
}
```

This example test passes if the verifier fails with an error message containing
the sub-string `"terminator"` *and* the error is reported for the `jump`
instruction.

If a function contains no `error:` annotations, the test passes if the
function verifies correctly.

### `test print-cfg`

Print the control flow graph of each function as a Graphviz graph, and run
filecheck over the result. See also the `clif-util print-cfg` command:

```
; For testing cfg generation. This code is nonsense.
test print-cfg
test verifier

function %nonsense(i32, i32) -> f32 {
; check: digraph %nonsense {
; regex: I=\binst\d+\b
; check: label="{block0 | <$(BRIF=$I)>brif v1, block1(v2), block2 }"]

block0(v0: i32, v1: i32):
    v2 = iconst.i32 0
    brif v1, block1(v2), block2  ; unordered: block0:$BRIF -> block1
                                 ; unordered: block0:$BRIF -> block2

block1(v5: i32):
    return v0

block2:
    v100 = f32const 0.0
    return v100
}
```

### `test domtree`

Compute the dominator tree of each function and validate it against the
`dominates:` annotations:

```
test domtree

function %test(i32) {
    block0(v0: i32):
        jump block1              ; dominates: block1
    block1:
        brif v0, block2, block3  ; dominates: block2, block3
    block2:
        jump block3
    block3:
        return
}
```

Every reachable basic block except for the entry block has an
*immediate dominator* which is a jump or branch instruction. This test passes
if the `dominates:` annotations on the immediate dominator instructions are
both correct and complete.

This test also sends the computed CFG post-order through filecheck.

### `test optimize`

Run each function through the optimization passes (e-graph based GVN and
rewrites, alias analysis, etc.) but not lowering or register allocation. The
resulting CLIF IR is sent to filecheck.

Requires a target ISA.

Supports the `precise-output` option, which requires the filecheck directives
to be a complete and exact description of the optimized output. This is useful
for tests that need to verify the exact form of the optimized IR and can be
auto-updated by setting `CRANELIFT_TEST_BLESS=1` when running the tests.

Example:

```
test optimize
target x86_64

function %foo(i32, i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    v3 = iadd v2, v0
    return v3
}
; check: v4 = iadd v0, v0
; check: v5 = iadd v4, v1
; check: return v5
```

### `test alias-analysis`

Run each function through the GVN and alias analysis passes (redundant load
elimination), then send the resulting CLIF to filecheck. Does not perform
lowering or register allocation.

### `test compile`

Test the whole code generation pipeline.

Each function is passed through the full `Context::compile()` function
which is normally used to compile code. Filecheck directives are matched
against the final form of the Cranelift IR right before binary machine code
emission.

### `test run`

Compile and execute a function.

This test command allows several directives:
 - to print the result of running a function to stdout, add a `print`
   directive and call the preceding function with arguments (see `%foo` in
   the example below); remember to enable `--nocapture` if running these
   tests through Cargo
 - to check the result of a function, add a `run` directive and call the
   preceding function with a comparison (`==` or `!=`) (see `%bar` below)
 - for backwards compatibility, to check the result of a function with a
   `() -> i*` signature, only the `run` directive is required, with no
   invocation or comparison (see `%baz` below); a non-zero value is
   interpreted as a successful test execution, whereas a zero value is
   interpreted as a failed test.

Currently a `target` is required but is only used to indicate whether the host
platform can run the test; only the architecture is filtered. The host
platform's native target will be used to actually compile the test.

Example:

```
test run
target x86_64

; how to print the results of a function
function %foo() -> i32 {
block0:
    v0 = iconst.i32 42
    return v0
}
; print: %foo()

; how to check the results of a function
function %bar(i32) -> i32 {
block0(v0:i32):
    v1 = iadd_imm v0, 1
    return v1
}
; run: %bar(1) == 2

; legacy method of checking the results of a function
function %baz() -> i8 {
block0:
    v0 = iconst.i8 1
    return v0
}
; run
```

### `test interpret`

Interpret each function using Cranelift's interpreter rather than compiling it.
Use `run:` and `print:` directives the same way as in `test run`. This does not
require a target ISA and runs platform-independently.

### `test inline`

Inline all calls in each function, optionally followed by optimization passes,
and send the resulting CLIF to filecheck. Does not perform lowering or register
allocation.

Supports the following options:
 - `precise-output`: require filecheck directives to be a complete and exact
   description of the output
 - `optimize`: run optimization passes after inlining

### `test safepoint`

Run CFG and dominator tree computation for each function, then send the CLIF
to filecheck. Useful for verifying safepoint/stackmap-related IR.

### `test unwind`

Run each function through the full code generation pipeline and verify the
generated unwind information (DWARF `.eh_frame` entries or Windows x64 unwind
data) against filecheck directives. Requires a target ISA.
