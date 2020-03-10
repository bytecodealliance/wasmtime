# Embedding Wasmtime in C

This document shows an example of how to embed Wasmtime using the [C API](https://github.com/WebAssembly/wasm-c-api) to execute a simple wasm program.

## Hello, world!

An excellent example of C API usage can be found at https://github.com/WebAssembly/wasm-c-api/blob/master/example/hello.c

### WebAssembly code

```wat
(module
  (func $hello (import "" "hello"))
  (func (export "run") (call $hello))
)
```

### C code

The wasmtime engine and its storage has to be initialized, before the WebAssembly modules can be loaded:

```C
  wasm_engine_t* engine = wasm_engine_new();
  wasm_store_t* store = wasm_store_new(engine);
```

The WebAssembly module bytecode can be loaded into memory and has to be provided as `wasm_byte_vec_t` to be compiled:

```C
  own wasm_module_t* module = wasm_module_new(store, &binary);
  if (!module) {
    printf("> Error compiling module!\n");
    return 1;
  }
```

This WebAssembly module requires the external import, provided by host, see `(func $hello (import "" "hello"))` code in the WAT file.

```C
own wasm_trap_t* hello_callback(
  const wasm_val_t args[], wasm_val_t results[]
) {
  printf("Calling back...\n");
  printf("> Hello World!\n");
  return NULL;
}
```

```C
  own wasm_functype_t* hello_type = wasm_functype_new_0_0();
  own wasm_func_t* hello_func =
    wasm_func_new(store, hello_type, hello_callback);
```

The WebAssembly instance can be created now.

```C
  const wasm_extern_t* imports[] = { wasm_func_as_extern(hello_func) };
  own wasm_instance_t* instance =
    wasm_instance_new(store, module, imports, NULL);
  if (!instance) {
    printf("> Error instantiating module!\n");
    return 1;
  }
```

Notice that the host function was provided during `wasm_instance_new`, and number of elements, order and their types in the `imports` array have to match WebAssembly module imports.

The WebAssembly module exports a function: `(func (export "run") ...`. The export can be call looked up via the `instance` exports.

```C
  own wasm_extern_vec_t exports;
  wasm_instance_exports(instance, &exports);
  const wasm_func_t* run_func = wasm_extern_as_func(exports.data[0]);
```

The exported `run_func` function can be called:

```C
  if (wasm_func_call(run_func, NULL, NULL)) {
    printf("> Error calling function!\n");
    return 1;
  }
```

The WebAssembly code calls the "hello", which will print "Hello World!".


## Get and build the hello example from the wasm-c-api

The example uses Clang for Linux, though GCC or other OS may work too.

```bash
# Create folder
mkdir hello_c && cd hello_c

# Get and untar wasmtime C API library
wget https://github.com/bytecodealliance/wasmtime/releases/download/v0.12.0/wasmtime-v0.12.0-x86_64-linux-c-api.tar.xz
tar xvf wasmtime-v0.12.0-x86_64-linux-c-api.tar.xz
export WASMTIME=wasmtime-v0.12.0-x86_64-linux-c-api

# Get hello.wasm and hello.c from https://github.com/WebAssembly/wasm-c-api
wget https://github.com/WebAssembly/wasm-c-api/raw/master/example/hello.wasm
wget https://github.com/WebAssembly/wasm-c-api/raw/master/example/hello.c
cat hello.c

# Build hello example
clang hello.c -o hello $WASMTIME/lib/libwasmtime.a -I$WASMTIME/include/
  
# Run
./hello
```
