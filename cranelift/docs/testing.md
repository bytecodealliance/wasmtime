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

File tests are `*.clif` files in the `filetests/` directory
hierarchy. Each file has a header describing what to test followed by a number
of input functions in the :doc:`Cranelift textual intermediate representation
<ir>`:

.. productionlist::
    test_file     : test_header `function_list`
    test_header   : test_commands (`isa_specs` | `settings`)
    test_commands : test_command { test_command }
    test_command  : "test" test_name { option } "\n"

The available test commands are described below.

Many test commands only make sense in the context of a target instruction set
architecture. These tests require one or more ISA specifications in the test
header:

.. productionlist::
    isa_specs     : { [`settings`] isa_spec }
    isa_spec      : "isa" isa_name { `option` } "\n"

The options given on the `isa` line modify the ISA-specific settings defined in
`cranelift-codegen/meta-python/isa/*/settings.py`.

All types of tests allow shared Cranelift settings to be modified:

.. productionlist::
    settings      : { setting }
    setting       : "set" { option } "\n"
    option        : flag | setting "=" value

The shared settings available for all target ISAs are defined in
`cranelift-codegen/meta-python/base/settings.py`.

The `set` lines apply settings cumulatively:

```
    test legalizer
    set opt_level=best
    set is_pic=1
    isa riscv64
    set is_pic=0
    isa riscv32 supports_m=false

    function %foo() {}
```

This example will run the legalizer test twice. Both runs will have
`opt_level=best`, but they will have different `is_pic` settings. The 32-bit
run will also have the RISC-V specific flag `supports_m` disabled.

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
See the `documentation <https://docs.rs/filecheck/>`_ for details of its syntax.

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
directives in the test file. LLVM's :command:`FileCheck` command has a
`CHECK-LABEL:` directive to help separate the output from different functions.
Cranelift's tests don't need this.

### `test cat`

This is one of the simplest file tests, used for testing the conversion to and
from textual IR. The `test cat` command simply parses each function and
converts it back to text again. The text of each function is then matched
against the associated filecheck directives.

Example:

```
    function %r1() -> i32, f32 {
    ebb1:
        v10 = iconst.i32 3
        v20 = f32const 0.0
        return v10, v20
    }
    ; sameln: function %r1() -> i32, f32 {
    ; nextln: ebb0:
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
        ebb0(v0: i32):
            jump ebb1       ; error: terminator
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
filecheck over the result. See also the :command:`clif-util print-cfg`
command:

```
    ; For testing cfg generation. This code is nonsense.
    test print-cfg
    test verifier

    function %nonsense(i32, i32) -> f32 {
    ; check: digraph %nonsense {
    ; regex: I=\binst\d+\b
    ; check: label="{ebb0 | <$(BRZ=$I)>brz ebb2 | <$(JUMP=$I)>jump ebb1}"]

    ebb0(v0: i32, v1: i32):
        brz v1, ebb2            ; unordered: ebb0:$BRZ -> ebb2
        v2 = iconst.i32 0
        jump ebb1(v2)           ; unordered: ebb0:$JUMP -> ebb1

    ebb1(v5: i32):
        return v0

    ebb2:
        v100 = f32const 0.0
        return v100
    }
```

### `test domtree`

Compute the dominator tree of each function and validate it against the
`dominates:` annotations::

```
    test domtree

    function %test(i32) {
        ebb0(v0: i32):
            jump ebb1     ; dominates: ebb1
        ebb1:
            brz v0, ebb3  ; dominates: ebb3
            jump ebb2     ; dominates: ebb2
        ebb2:
            jump ebb3
        ebb3:
            return
    }
```

Every reachable extended basic block except for the entry block has an
*immediate dominator* which is a jump or branch instruction. This test passes
if the `dominates:` annotations on the immediate dominator instructions are
both correct and complete.

This test also sends the computed CFG post-order through filecheck.

### `test legalizer`

Legalize each function for the specified target ISA and run the resulting
function through filecheck. This test command can be used to validate the
encodings selected for legal instructions as well as the instruction
transformations performed by the legalizer.

### `test regalloc`

Test the register allocator.

First, each function is legalized for the specified target ISA. This is
required for register allocation since the instruction encodings provide
register class constraints to the register allocator.

Second, the register allocator is run on the function, inserting spill code and
assigning registers and stack slots to all values.

The resulting function is then run through filecheck.

### `test binemit`

Test the emission of binary machine code.

The functions must contains instructions that are annotated with both encodings
and value locations (registers or stack slots). For instructions that are
annotated with a `bin:` directive, the emitted hexadecimal machine code for
that instruction is compared to the directive:

```
    test binemit
    isa riscv

    function %int32() {
    ebb0:
        [-,%x5]             v0 = iconst.i32 1
        [-,%x6]             v1 = iconst.i32 2
        [R#0c,%x7]          v10 = iadd v0, v1       ; bin: 006283b3
        [R#200c,%x8]        v11 = isub v0, v1       ; bin: 40628433
        return
    }
```

If any instructions are unencoded (indicated with a `[-]` encoding field), they
will be encoded using the same mechanism as the legalizer uses. However,
illegal instructions for the ISA won't be expanded into other instruction
sequences. Instead the test will fail.

Value locations must be present if they are required to compute the binary
bits. Missing value locations will cause the test to crash.

### `test simple-gvn`

Test the simple GVN pass.

The simple GVN pass is run on each function, and then results are run
through filecheck.

### `test licm`

Test the LICM pass.

The LICM pass is run on each function, and then results are run
through filecheck.

### `test dce`

Test the DCE pass.

The DCE pass is run on each function, and then results are run
through filecheck.

### `test shrink`

Test the instruction shrinking pass.

The shrink pass is run on each function, and then results are run
through filecheck.

### `test preopt`

Test the preopt pass.

The preopt pass is run on each function, and then results are run
through filecheck.

### `test compile`

Test the whole code generation pipeline.

Each function is passed through the full `Context::compile()` function
which is normally used to compile code. This type of test often depends
on assertions or verifier errors, but it is also possible to use
filecheck directives which will be matched against the final form of the
Cranelift IR right before binary machine code emission.

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
 `() -> b*` signature, only the `run` directive is required, with no
 invocation or comparison (see `%baz` below);  a `true` value is
 interpreted as a successful test execution, whereas a `false` value is
 interpreted as a failed test.

Currently a `target` is required but is only used to indicate whether the host
platform can run the test and currently only the architecture is filtered. The
host platform's native target will be used to actually compile the test.

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
    function %baz() -> b1 {
    block0:
        v0 = bconst.b1 true
        return v0
    }
    ; run
```

#### Environment directives

Some tests need additional resources to be provided by the filetest infrastructure.

When any of the following directives is present the first argument of the function is *required* to be a `i64 vmctx`.
The filetest infrastructure will then pass a pointer to the environment struct via this argument.

The environment struct is essentially a list of pointers with info about the resources requested by the directives. These
pointers are always 8 bytes, and laid out sequentially in memory. Even for 32 bit machines, where we only fill the first
4 bytes of the pointer slot.

Currently, we only support requesting heaps, however this is a generic mechanism that should
be able to introduce any sort of environment support that we may need later. (e.g. tables, global values, external functions)

##### `heap` directive

The `heap` directive allows a test to request a heap to be allocated and passed to the test via the environment struct.


A sample heap annotation is the following:
```
; heap: static, size=0x1000, ptr=vmctx+0, bound=vmctx+8
```

This indicates the following:
* `static`: We have requested a non-resizable and non-movable static heap.
* `size=0x1000`: It has to have a size of 4096 bytes.
* `ptr=vmctx+0`: The pointer to the address to the start of this heap is placed at offset 0 in the `vmctx` struct
* `bound=vmctx+8`: The pointer to the address to the end of this heap is placed at offset 8 in the `vmctx` struct

The `ptr` and `bound` arguments make explicit the placement of the pointers to the start and end of the heap memory in
the environment struct. `vmctx+0` means that at offset 0 of the environment struct there will be the pointer to the start
similarly, at offset 8 the pointer to the end.


You can combine multiple heap annotations, in which case, their pointers are laid out sequentially in memory in
the order that the annotations appear in the source file.

```
; heap: static, size=0x1000, ptr=vmctx+0, bound=vmctx+8
; heap: dynamic, size=0x1000, ptr=vmctx+16, bound=vmctx+24
```

An invalid or unexpected offset will raise an error when the test is run.

See the diagram below, on how the `vmctx` struct ends up if with multiple heaps:

```
 ┌─────────────────────┐ vmctx+0
 │heap0: start address │
 ├─────────────────────┤ vmctx+8
 │heap0: end address   │
 ├─────────────────────┤ vmctx+16
 │heap1: start address │
 ├─────────────────────┤ vmctx+24
 │heap1: end address   │
 ├─────────────────────┤ vmctx+32
 │etc...               │
 └─────────────────────┘
```

With this setup, you can now use the global values to load heaps, and load / store to them.

Example:

```
function %heap_load_store(i64 vmctx, i64, i32) -> i32 {
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+0
    gv2 = load.i64 notrap aligned gv0+8
    heap0 = dynamic gv1, bound gv2, offset_guard 0, index_type i64

block0(v0: i64, v1: i64, v2: i32):
    v3 = heap_addr.i64 heap0, v1, 4
    store.i32 v2, v3
    v4 = load.i32 v3
    return v4
}
; heap: static, size=0x1000, ptr=vmctx+0, bound=vmctx+8
; run: %heap_load_store(0, 1) == 1
```


### `test interpret`

Test the CLIF interpreter

This test supports the same commands as `test run`, but runs the code in the cranelift
interpreter instead of the host machine.
