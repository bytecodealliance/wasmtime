/*
Example of instantiating of the WebAssembly module and invoking its exported
function.

You can compile and run this example on Linux with:

   cargo build --release -p wasmtime-c-api
   cc examples/fuel.c \
       -I crates/c-api/include \
       -I crates/c-api/wasm-c-api/include \
       target/release/libwasmtime.a \
       -lpthread -ldl -lm \
       -o fuel
   ./fuel

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
  wasmtime_error_t *error = NULL;

  wasm_config_t *config = wasm_config_new();
  assert(config != NULL);
  wasmtime_config_consume_fuel_set(config, true);

  // Create an *engine*, which is a compilation context, with our configured options.
  wasm_engine_t *engine = wasm_engine_new_with_config(config);
  assert(engine != NULL);
  wasmtime_store_t *store = wasmtime_store_new(engine, NULL, NULL);
  assert(store != NULL);
  wasmtime_context_t *context = wasmtime_store_context(store);

  error = wasmtime_context_add_fuel(context, 10000);
  if (error != NULL)
    exit_with_error("failed to add fuel", error, NULL);

  // Load our input file to parse it next
  FILE* file = fopen("examples/fuel.wat", "r");
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
  error = wasmtime_wat2wasm(wat.data, wat.size, &wasm);
  if (error != NULL)
    exit_with_error("failed to parse wat", error, NULL);
  wasm_byte_vec_delete(&wat);

  // Compile and instantiate our module
  wasmtime_module_t *module = NULL;
  error = wasmtime_module_new(engine, (uint8_t*) wasm.data, wasm.size, &module);
  if (module == NULL)
    exit_with_error("failed to compile module", error, NULL);
  wasm_byte_vec_delete(&wasm);

  wasm_trap_t *trap = NULL;
  wasmtime_instance_t instance;
  error = wasmtime_instance_new(context, module, NULL, 0, &instance, &trap);
  if (error != NULL || trap != NULL)
    exit_with_error("failed to instantiate", error, trap);

  // Lookup our `fibonacci` export function
  wasmtime_extern_t fib;
  bool ok = wasmtime_instance_export_get(context, &instance, "fibonacci", strlen("fibonacci"), &fib);
  assert(ok);
  assert(fib.kind == WASMTIME_EXTERN_FUNC);

  // Call it repeatedly until it fails
  for (int n = 1; ; n++) {
    uint64_t fuel_before;
    wasmtime_context_fuel_consumed(context, &fuel_before);
    wasmtime_val_t params[1];
    params[0].kind = WASMTIME_I32;
    params[0].of.i32 = n;
    wasmtime_val_t results[1];
    error = wasmtime_func_call(context, &fib.of.func, params, 1, results, 1, &trap);
    if (error != NULL || trap != NULL) {
      printf("Exhausted fuel computing fib(%d)\n", n);
      break;
    }

    uint64_t fuel_after;
    wasmtime_context_fuel_consumed(context, &fuel_after);
    assert(results[0].kind == WASMTIME_I32);
    printf("fib(%d) = %d [consumed %lld fuel]\n", n, results[0].of.i32, fuel_after - fuel_before);

    error = wasmtime_context_add_fuel(context, fuel_after - fuel_before);
    if (error != NULL)
      exit_with_error("failed to add fuel", error, NULL);
  }

  // Clean up after ourselves at this point
  wasmtime_module_delete(module);
  wasmtime_store_delete(store);
  wasm_engine_delete(engine);
  return 0;
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
