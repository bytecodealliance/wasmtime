/*
Example of instantiating of the WebAssembly module and invoking its exported
function.

You can compile and run this example on Linux with:

   cargo build --release -p wasmtime
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

static void print_trap(wasm_trap_t *trap);

static wasm_trap_t* hello_callback(const wasm_val_t args[], wasm_val_t results[]) {
  printf("Calling back...\n");
  printf("> Hello World!\n");
  return NULL;
}

int main() {
  int ret = 0;
  // Set up our compilation context. Note that we could also work with a
  // `wasm_config_t` here to configure what feature are enabled and various
  // compilation settings.
  printf("Initializing...\n");
  wasm_engine_t *engine = wasm_engine_new();
  assert(engine != NULL);

  // With an engine we can create a *store* which is a long-lived group of wasm
  // modules.
  wasm_store_t *store = wasm_store_new(engine);
  assert(store != NULL);

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
  wasm_byte_vec_t wasm, error;
  if (wasmtime_wat2wasm(engine, &wat, &wasm, &error) == 0) {
    fprintf(stderr, "failed to parse wat %.*s\n", (int) error.size, error.data);
    goto free_store;
  }
  wasm_byte_vec_delete(&wat);

  // Now that we've got our binary webassembly we can compile our module.
  printf("Compiling module...\n");
  wasm_module_t *module = wasm_module_new(store, &wasm);
  wasm_byte_vec_delete(&wasm);
  assert(module != NULL);

  // Next up we need to create the function that the wasm module imports. Here
  // we'll be hooking up a thunk function to the `hello_callback` native
  // function above.
  printf("Creating callback...\n");
  wasm_functype_t *hello_ty = wasm_functype_new_0_0();
  wasm_func_t *hello = wasm_func_new(store, hello_ty, hello_callback);

  // With our callback function we can now instantiate the compiled module,
  // giving us an instance we can then execute exports from. Note that
  // instantiation can trap due to execution of the `start` function, so we need
  // to handle that here too.
  printf("Instantiating module...\n");
  wasm_trap_t *trap = NULL;
  const wasm_extern_t *imports[] = { wasm_func_as_extern(hello) };
  wasm_instance_t *instance = wasm_instance_new(store, module, imports, &trap);
  if (instance == NULL) {
    print_trap(trap);
    goto free_module;
  }

  // Lookup our `run` export function
  printf("Extracting export...\n");
  wasm_extern_vec_t externs;
  wasm_instance_exports(instance, &externs);
  assert(externs.size == 1);
  wasm_func_t *run = wasm_extern_as_func(externs.data[0]);
  assert(run != NULL);

  // And call it!
  printf("Calling export...\n");
  trap = wasm_func_call(run, NULL, NULL);
  if (trap != NULL) {
    print_trap(trap);
    goto free_instance;
  }

  // Clean up after ourselves at this point
  printf("All finished!\n");
  ret = 0;

free_instance:
  wasm_extern_vec_delete(&externs);
  wasm_instance_delete(instance);
free_module:
  wasm_module_delete(module);
free_store:
  wasm_store_delete(store);
  wasm_engine_delete(engine);
  return ret;
}

static void print_trap(wasm_trap_t *trap) {
  assert(trap != NULL);
  wasm_message_t message;
  wasm_trap_message(trap, &message);
  fprintf(stderr, "failed to instantiate module %.*s\n", (int) message.size, message.data);
  wasm_byte_vec_delete(&message);
  wasm_trap_delete(trap);
}

