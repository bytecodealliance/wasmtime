Python 3 extension for interface with Wasmtime/Cranelift.

# Build

First, you'll need to install some Python dependencies:

```
$ pip3 install setuptools wheel==0.31.1 setuptools-rust
```

Next you can build the extension with:

```
rustup run nightly python3 setup.py build
```

Note that a nightly version of Rust is required due to our usage of PyO3.

This will create a directory called `build/lib` which you can add to
`PYTHONPATH` in order to get `import wasmtime` working.
