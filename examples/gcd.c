/*
Example of instantiating of the WebAssembly module and invoking its exported
function.

You can compile and run this example on Linux with:

   cargo build --release -p wasmtime
   cc examples/gcd.c \
       -I crates/c-api/include \
       -I crates/c-api/wasm-c-api/include \
       target/release/libwasmtime.a \
       -lpthread -ldl -lm \
       -o gcd
   ./gcd

Note that on Windows and macOS the command will be similar, but you'll need
to tweak the `-lpthread` and such annotations.
*/

#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <wasm.h>
#include <wasmtime.h>

static void exit_with_error(const char *message, wasmtime_error_t *error, wasm_trap_t *trap);

int main() {
  int ret = 0;
  // Set up our context
  wasm_engine_t *engine = wasm_engine_new();
  assert(engine != NULL);
  wasm_store_t *store = wasm_store_new(engine);
  assert(store != NULL);

  // Load our input file to parse it next
  FILE* file = fopen("examples/gcd.wat", "r");
  if (!file) {
    printf("> Error loading file!\n");
    return 1;
  }
  fseek(file, 0L, SEEK_END);
  size_t file_size = ftell(file);
  fseek(file, 0L, SEEK_SET);
  wasm_byte_vec_t wat;
  wasm_byte_vec_new_uninitialized(&wat, file_size);
  if (fread(wat.data, file_size, 1, file) != 1) {
    printf("> Error loading module!\n");
    return 1;
  }
  fclose(file);

  // Parse the wat into the binary wasm format
  wasm_byte_vec_t wasm;
  wasmtime_error_t *error = wasmtime_wat2wasm(&wat, &wasm);
  if (error != NULL)
    exit_with_error("failed to parse wat", error, NULL);
  wasm_byte_vec_delete(&wat);

  // Compile and instantiate our module
  wasm_module_t *module = NULL;
  error = wasmtime_module_new(store, &wasm, &module);
  if (module == NULL)
    exit_with_error("failed to compile module", error, NULL);
  wasm_byte_vec_delete(&wasm);
  wasm_trap_t *trap = NULL;
  wasm_instance_t *instance = NULL;
  error = wasmtime_instance_new(store, module, NULL, 0, &instance, &trap);
  if (instance == NULL)
    exit_with_error("failed to instantiate", error, trap);

  // Lookup our `gcd` export function
  wasm_extern_vec_t externs;
  wasm_instance_exports(instance, &externs);
  assert(externs.size == 1);
  wasm_func_t *gcd = wasm_extern_as_func(externs.data[0]);
  assert(gcd != NULL);

  // And call it!
  int a = 6;
  int b = 27;
  wasm_val_t params[2];
  wasm_val_t results[1];
  params[0].kind = WASM_I32;
  params[0].of.i32 = a;
  params[1].kind = WASM_I32;
  params[1].of.i32 = b;
  error = wasmtime_func_call(gcd, params, 2, results, 1, &trap);
  if (error != NULL || trap != NULL)
    exit_with_error("failed to call gcd", error, trap);
  assert(results[0].kind == WASM_I32);

  printf("gcd(%d, %d) = %d\n", a, b, results[0].of.i32);

  // Clean up after ourselves at this point
  ret = 0;

  wasm_extern_vec_delete(&externs);
  wasm_instance_delete(instance);
  wasm_module_delete(module);
  wasm_store_delete(store);
  wasm_engine_delete(engine);
  return ret;
}

static void exit_with_error(const char *message, wasmtime_error_t *error, wasm_trap_t *trap) {
  fprintf(stderr, "error: %s\n", message);
  wasm_byte_vec_t error_message;
  if (error != NULL) {
    wasmtime_error_message(error, &error_message);
  } else {
    wasm_trap_message(trap, &error_message);
  }
  fprintf(stderr, "%.*s\n", (int) error_message.size, error_message.data);
  wasm_byte_vec_delete(&error_message);
  exit(1);
}
