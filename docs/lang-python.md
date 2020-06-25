# Using WebAssembly from Python

Wasmtime [is available on PyPI](https://pypi.org/project/wasmtime/) and can be
used programmatically or as a python module loader, which allows almost any
WebAssembly module to be used as a python module. This guide will go over adding
Wasmtime to your project, and some provided examples of what can be done with
WebAssembly modules.

Make sure you've got Python 3.5 or newer installed locally, and we can get
started!

## Getting started and simple example

First, copy this example WebAssembly text module into your project. It exports a
function for calculating the greatest common denominator of two numbers.

```wat
{{#include ../examples/gcd.wat}}
```

Next, install the [Wasmtime package](https://pypi.org/project/wasmtime/) from
PyPi. It can be installed as a dependency through Pip or related tools such as
Pipenv.

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
import wasmtime.loader
import gcd

print("gcd(27, 6) =", gcd.gcd(27, 6))
```

This script should output

```bash
gcd(27, 6) = 3
```

If this is the output you see, congrats! You've successfully ran your first
WebAssembly code in python!

You can also alternatively use the [`wasmtime` package's
API](https://bytecodealliance.github.io/wasmtime-py/):

```python
from wasmtime import Store, Module, Instance

store = Store()
module = Module.from_file(store, 'gcd.wat')
instance = Instance(module, [])
gcd = instance.get_export('gcd')
print("gcd(27, 6) =", gcd(27, 6))
```

## More examples and contributing

The `wasmtime` Python package currently [lives in its own repository outside of
`wasmtime`](https://github.com/bytecodealliance/wasmtime-py) and has a [number
of other more advanced
examples](https://github.com/bytecodealliance/wasmtime-py/tree/main/examples)
as well. Feel free to browse those, but if you find anything missing don't
hesitate to [open an
issue](https://github.com/bytecodealliance/wasmtime-py/issues/new) and let us
know if you have any questions!
