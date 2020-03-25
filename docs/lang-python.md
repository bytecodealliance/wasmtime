# Using WebAssembly from Python

Wasmtime can be used as a python module loader, which allows almost any
WebAssembly module to be used as a python module. This guide will go over adding
Wasmtime to your project, and some provided examples of what can be done with
WebAssembly modules.

## Prerequisites

To follow this guide, you'll need

 - Python 3.6 or newer
 - The [WebAssembly binary toolkit](https://github.com/WebAssembly/wabt/releases)
 - The rust toolchain installer [rustup](https://rustup.rs/)

## Getting started and simple example

First, copy this example WebAssembly text module into your project. It exports a
function for calculating the greatest common denominator of two numbers.

```wat
{{#include ../examples/gcd.wat}}
```

Before we can do anything with this module, we need to convert it to the
WebAssembly binary format. We can do this with the command line tools provided
by the WebAssembly binary toolkit

```bash
wat2wasm gcd.wat
```

This will create the binary form of the gcd module `gcd.wasm`, we'll use this
module in the following steps.

Next, install the Wasmtime module loader, which is provided as a [python package](https://pypi.org/project/wasmtime/)
on PyPi. It can be installed as a dependency through Pip or related tools such
as Pipenv.

```bash
pip install wasmtime
```

Or

```bash
pipenv install wasmtime
```

After you have Wasmtime installed and you've imported `wasmtime`, you can import
WebAssembly modules in your project like any other python module.

```python
{{#include ../crates/misc/py/examples/gcd/run.py}}
```

This script should output

```bash
gcd(27, 6) = 3
```

If this is the output you see, congrats! You've successfully ran your first
WebAssembly code in python!

## Host interaction and memory

In the first example, we called a function exported by a WebAssembly
module. Depeding on what you need to accomplish, WebAssembly modules can also
call functions from other modules and python itself. This is done through the
module imports mechanism, which allows other modules and the host environment to
provide functions, globals, and memory spaces. The following example will show
you how to use module imports and work with module linear memory.

> Note: At the moment, the Wasmtime python module can only import functions and
> memories.

To show how we can use functions from the host, take a look at this rust code

```rust
{{#include ../crates/misc/py/examples/import/demo.rs}}
```

We have a `test` function which calls `callback`. Since it's wrapped in `extern "C"`,
this function will be dynamically linked. The Wasmtime module does this linking
automatically by importing any needed modules at runtime. If we compile this
example without any extra linker options, the result module will import
`callback` from a module called `env`, so we need to provide an implementation of
`callback` inside an `env.py` module.

```python
{{#include ../crates/misc/py/examples/import/env.py}}
```

The module provides `callback` with a pointer to a string message. We use this
to index into the demo module's memory, extract the message bytes and print them
as a string. Every WebAssembly module exports its main linear memory as "memory"
by default, so it's accessible as `demo.memory` in python. We wrap the memory
into a `memoryview` so we can safely access the values inside.

Before we move on, note the type annotations on `callback`. These are necessary
for representing your function as something callable in WebAssembly, since
WebAssembly functions only operate on 32 and 64 bit floats and integers. When
defining functions for use by WebAssembly modules, make sure the parameters and
return value are annotated appropriately as any of `'i32'`, `'i64'`, `'f32'`, or
`'f64'`.

Before we can use `demo.rs` we need to compile it

```bash
rustup run nightly rustc --target=wasm32-unknown-unknown --crate-type=cdylib demo.rs
```

We can then use it like this

```python
{{#include ../crates/misc/py/examples/import/run.py}}
```

The script should print `Hello, world!` and exit.
