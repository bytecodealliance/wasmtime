# Reducing Test Cases

When reporting a bug, or investing a bug report, in Wasmtime it is easier for
everyone involved when there is a test case that reproduces the bug. It is even
better when that test case is as small as possible, so that developers don't
need to wade through megabytes of unrelated Wasm that isn't necessary to
showcase the bug. The process of taking a large test case and stripping out the
unnecessary bits is called *test case reduction*.

[The `wasm-tools shrink` tool](github.com/bytecodealliance/wasm-tools) can
automatically reduce Wasm test cases when given

1. the original, unreduced test case, and
2. a predicate script to determine whether the bug reproduces on a given reduced
   test case candidate.

If the test case causes Wasmtime to segfault, the script can run Wasmtime and
check its exit code. If the test case produces a different result in Wasmtime vs
another Wasm engine, the script can run both engines and compare their
results. It is also often useful to `grep` through the candidate's WAT
disassembly to make sure that relevant features and instructions are present.

Note that there are also a few other test-case reducers that can operate on
Wasm. All of them, including `wasm-shrink`, work fairly similarly at a high
level, but often if one reducer gets stuck in a local minimum, another reducer
can pick up from there and reduce the test case further due to differences in
the details of their implementations. Therefore, if you find that `wasm-shrink`
isn't very effective on a particular test case, you can try continuing reduction
with one of the following:

* [Binaryen's `wasm-reduce`
  tool](https://github.com/WebAssembly/binaryen?tab=readme-ov-file#tools)
* [`creduce`, which can be effective at reducing Wasm test cases when
  disassembled into their `.wat` text
  format](https://github.com/csmith-project/creduce)

## Case Study: [Issue #7779](https://github.com/bytecodealliance/wasmtime/issues/7779)

A bug was reported involving the `memory.init` instruction. The attached test
case was larger than it needed to be and contained a bunch of functions and
other things that were irrelevant. A perfect use case for `wasm-tools shrink`!

First, we needed a predicate script to identify the reported buggy behavior. The
script is given the candidate test case as its first argument and must exit zero
if the candidate exhibits the bug and non-zero otherwise.

```bash
#!/usr/bin/env bash

# Propagate failure: exit non-zero if any individual command exits non-zero.
set -e

# Disassembly the candidate into WAT. Make sure the `memory.init` instruction
# is present, since the bug report is about that instruction. Additionally, make
# sure it is referencing the same data segment.
wasm-tools print $1 | grep -q 'memory.init 2'

# Make sure that the data segment in question remains unchanged, as mutating its
# length can change the semantics of the `memory.init` instruction.
wasm-tools print $1 | grep -Eq '\(data \(;2;\) \(i32\.const 32\) "\\01\\02\\03\\04\\05\\06\\07\\08\\ff"\)'

# Make sure that the `_start` function that contains the `memory.init` is still
# exported, so that running the Wasm will run the `memory.init` instruction.
wasm-tools print $1 | grep -Eq '\(export "_start" \(func 0\)\)'

# Run the testcase in Wasmtime and make sure that it traps the same way as the
# original test case.
cargo run --manifest-path ~/wasmtime/Cargo.toml -- run $1 2>&1 \
    | grep -q 'wasm trap: out of bounds memory access'
```

Note that this script is a little fuzzy! It just checks for `memory.init` and a
particular trap. That trap can correctly occur according to Wasm semantics when
`memory.init` is given certain inputs! This means we need to double check that
the reduced test case actually exhibits a real bug and its inputs haven't been
transformed into something that Wasm semantics specify should indeed
trap. Sometimes writing very precise predicate scripts is difficult, but we do
the best we can and usually it works out fine.

With the predicate script in hand, we can automatically reduce the original test
case:

```shell-session
$ wasm-tools shrink predicate.sh test-case.wasm
369 bytes (1.07% smaller)
359 bytes (3.75% smaller)
357 bytes (4.29% smaller)
354 bytes (5.09% smaller)
344 bytes (7.77% smaller)
...
118 bytes (68.36% smaller)
106 bytes (71.58% smaller)
94 bytes (74.80% smaller)
91 bytes (75.60% smaller)
90 bytes (75.87% smaller)

test-case.shrunken.wasm :: 90 bytes (75.87% smaller)
================================================================================
(module
  (type (;0;) (func))
  (func (;0;) (type 0)
    (local i32 f32 i64 f64)
    i32.const 0
    i32.const 9
    i32.const 0
    memory.init 2
  )
  (memory (;0;) 1 5)
  (export "_start" (func 0))
  (data (;0;) (i32.const 8) "")
  (data (;1;) (i32.const 16) "")
  (data (;2;) (i32.const 32) "\01\02\03\04\05\06\07\08\ff")
)
================================================================================
```

In this case, the arguments to the original `memory.init` instruction haven't
changed, and neither has the relevant data segment, so the reduced test case
should exhibit the same behavior as the original.

In the end, it was [determined that Wasmtime was behaving as
expected](https://github.com/bytecodealliance/wasmtime/issues/7779#issuecomment-1894350625),
but the presence of the reduced test case makes it much easier to make that
determination.
