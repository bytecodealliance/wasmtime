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

File tests are `*.clif` files in the `file tests/` directory
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
    target riscv64
    set is_pic=0
    target riscv32 supports_m=false

    function %foo() {}
```

This example will run the legalizer test twice. Both runs will have
`opt_level=best`, but they will have different `is_pic` settings. The 32-bit
run will also have the RISC-V specific flag `supports_m` disabled.

The file tests are run automatically as part of `cargo test`, and they can
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
filecheck over the result. See also the :command:`clif-util print-cfg`
command:

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
`dominates:` annotations::

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

### `test simple_preopt`

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
 `() -> i*` signature, only the `run` directive is required, with no
 invocation or comparison (see `%baz` below);  a non zero value is
 interpreted as a successful test execution, whereas a zero value is
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
    function %baz() -> i8 {
    block0:
        v0 = iconst.i8 1
        return v0
    }
    ; run
```
