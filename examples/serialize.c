/*
Example of instantiating of the WebAssembly module and invoking its exported
function.

You can compile and run this example on Linux with:

   cargo build --release -p wasmtime-c-api
   cc examples/hello.c \
       -I crates/c-api/include \
       -I crates/c-api/wasm-c-api/include \
       target/release/libwasmtime.a \
       -lpthread -ldl -lm \
       -o hello
   ./hello

Note that on Windows and macOS the command will be similar, but you'll need
to tweak the `-lpthread` and such annotations as well as the name of the
`libwasmtime.a` file on Windows.
*/

#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <wasm.h>
#include <wasmtime.h>

static void exit_with_error(const char *message, wasmtime_error_t *error, wasm_trap_t *trap);

static wasm_trap_t* hello_callback(
    void *env,
    wasmtime_caller_t *caller,
    const wasmtime_val_t* args,
    size_t nargs,
    wasmtime_val_t* results,
    size_t nresults) {
  printf("Calling back...\n");
  printf("> Hello World!\n");
  return NULL;
}

int serialize(wasm_byte_vec_t* buffer) {
  // Set up our compilation context. Note that we could also work with a
  // `wasm_config_t` here to configure what feature are enabled and various
  // compilation settings.
  printf("Initializing...\n");
  wasm_engine_t *engine = wasm_engine_new();
  assert(engine != NULL);

  // Read our input file, which in this case is a wasm text file.
  FILE* file = fopen("examples/hello.wat", "r");
  assert(file != NULL);
  fseek(file, 0L, SEEK_END);
  size_t file_size = ftell(file);
  fseek(file, 0L, SEEK_SET);
  wasm_byte_vec_t wat;
  wasm_byte_vec_new_uninitialized(&wat, file_size);
  assert(fread(wat.data, file_size, 1, file) == 1);
  fclose(file);

  // Parse the wat into the binary wasm format
  wasm_byte_vec_t wasm;
  wasmtime_error_t *error = wasmtime_wat2wasm(wat.data, wat.size, &wasm);
  if (error != NULL)
    exit_with_error("failed to parse wat", error, NULL);
  wasm_byte_vec_delete(&wat);

  // Now that we've got our binary webassembly we can compile our module
  // and serialize into buffer.
  printf("Compiling and serializing module...\n");
  wasmtime_module_t *module = NULL;
  error = wasmtime_module_new(engine, (uint8_t*)wasm.data, wasm.size, &module);
  wasm_byte_vec_delete(&wasm);
  if (error != NULL)
    exit_with_error("failed to compile module", error, NULL);
  error = wasmtime_module_serialize(module, buffer);
  wasmtime_module_delete(module);
  if (error != NULL)
    exit_with_error("failed to serialize module", error, NULL);

  printf("Serialized.\n");

  wasm_engine_delete(engine);
  return 0;
}

int deserialize(wasm_byte_vec_t* buffer) {
  // Set up our compilation context. Note that we could also work with a
  // `wasm_config_t` here to configure what feature are enabled and various
  // compilation settings.
  printf("Initializing...\n");
  wasm_engine_t *engine = wasm_engine_new();
  assert(engine != NULL);

  // With an engine we can create a *store* which is a long-lived group of wasm
  // modules.
  wasmtime_store_t *store = wasmtime_store_new(engine, NULL, NULL);
  assert(store != NULL);
  wasmtime_context_t *context = wasmtime_store_context(store);

  // Deserialize compiled module.
  printf("Deserialize module...\n");
  wasmtime_module_t *module = NULL;
  wasmtime_error_t *error = wasmtime_module_deserialize(engine, (uint8_t*) buffer->data, buffer->size, &module);
  if (error != NULL)
    exit_with_error("failed to compile module", error, NULL);

  // Next up we need to create the function that the wasm module imports. Here
  // we'll be hooking up a thunk function to the `hello_callback` native
  // function above.
  printf("Creating callback...\n");
  wasm_functype_t *hello_ty = wasm_functype_new_0_0();
  wasmtime_func_t hello;
  wasmtime_func_new(context, hello_ty, hello_callback, NULL, NULL, &hello);

  // With our callback function we can now instantiate the compiled module,
  // giving us an instance we can then execute exports from. Note that
  // instantiation can trap due to execution of the `start` function, so we need
  // to handle that here too.
  printf("Instantiating module...\n");
  wasm_trap_t *trap = NULL;
  wasmtime_instance_t instance;
  wasmtime_extern_t imports[1];
  imports[0].kind = WASMTIME_EXTERN_FUNC;
  imports[0].of.func = hello;
  error = wasmtime_instance_new(context, module, imports, 1, &instance, &trap);
  if (error != NULL || trap != NULL)
    exit_with_error("failed to instantiate", error, trap);
  wasmtime_module_delete(module);

  // Lookup our `run` export function
  wasmtime_extern_t run;
  bool ok = wasmtime_instance_export_get(context, &instance, "run", 3, &run);
  assert(ok);
  assert(run.kind == WASMTIME_EXTERN_FUNC);

  // And call it!
  printf("Calling export...\n");
  error = wasmtime_func_call(context, &run.of.func, NULL, 0, NULL, 0, &trap);
  if (error != NULL || trap != NULL)
    exit_with_error("failed to call function", error, trap);

  // Clean up after ourselves at this point
  printf("All finished!\n");

  wasmtime_store_delete(store);
  wasm_engine_delete(engine);
  return 0;
}

int main() {
  wasm_byte_vec_t buffer;
  if (serialize(&buffer)) {
    return 1;
  }
  if (deserialize(&buffer)) {
    return 1;
  }
  wasm_byte_vec_delete(&buffer);
  return 0;
}

static void exit_with_error(const char *message, wasmtime_error_t *error, wasm_trap_t *trap) {
  fprintf(stderr, "error: %s\n", message);
  wasm_byte_vec_t error_message;
  if (error != NULL) {
    wasmtime_error_message(error, &error_message);
    wasmtime_error_delete(error);
  } else {
    wasm_trap_message(trap, &error_message);
    wasm_trap_delete(trap);
  }
  fprintf(stderr, "%.*s\n", (int) error_message.size, error_message.data);
  wasm_byte_vec_delete(&error_message);
  exit(1);
}
