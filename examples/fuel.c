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
  wasm_store_t *store = wasm_store_new(engine);
  assert(store != NULL);
  error = wasmtime_store_add_fuel(store, 10000);
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
  error = wasmtime_wat2wasm(&wat, &wasm);
  if (error != NULL)
    exit_with_error("failed to parse wat", error, NULL);
  wasm_byte_vec_delete(&wat);

  // Compile and instantiate our module
  wasm_module_t *module = NULL;
  error = wasmtime_module_new(engine, &wasm, &module);
  if (module == NULL)
    exit_with_error("failed to compile module", error, NULL);
  wasm_byte_vec_delete(&wasm);
  wasm_trap_t *trap = NULL;
  wasm_instance_t *instance = NULL;
  wasm_extern_vec_t imports = WASM_EMPTY_VEC;
  error = wasmtime_instance_new(store, module, &imports, &instance, &trap);
  if (instance == NULL)
    exit_with_error("failed to instantiate", error, trap);

  // Lookup our `fibonacci` export function
  wasm_extern_vec_t externs;
  wasm_instance_exports(instance, &externs);
  assert(externs.size == 1);
  wasm_func_t *fibonacci = wasm_extern_as_func(externs.data[0]);
  assert(fibonacci != NULL);

  // Call it repeatedly until it fails
  for (int n = 1; ; n++) {
    uint64_t fuel_before;
    wasmtime_store_fuel_consumed(store, &fuel_before);
    wasm_val_t params[1] = { WASM_I32_VAL(n) };
    wasm_val_t results[1];
    wasm_val_vec_t params_vec = WASM_ARRAY_VEC(params);
    wasm_val_vec_t results_vec = WASM_ARRAY_VEC(results);
    error = wasmtime_func_call(fibonacci, &params_vec, &results_vec, &trap);
    if (error != NULL || trap != NULL) {
      printf("Exhausted fuel computing fib(%d)\n", n);
      break;
    }

    uint64_t fuel_after;
    wasmtime_store_fuel_consumed(store, &fuel_after);
    assert(results[0].kind == WASM_I32);
    printf("fib(%d) = %d [consumed %lld fuel]\n", n, results[0].of.i32, fuel_after - fuel_before);

    error = wasmtime_store_add_fuel(store, fuel_after - fuel_before);
    if (error != NULL)
      exit_with_error("failed to add fuel", error, NULL);
  }

  // Clean up after ourselves at this point
  wasm_extern_vec_delete(&externs);
  wasm_instance_delete(instance);
  wasm_module_delete(module);
  wasm_store_delete(store);
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
