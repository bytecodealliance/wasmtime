#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <inttypes.h>

#include <wasm.h>
#include "wasmtime.h"

#define own

static void exit_with_error(const char *message, wasmtime_error_t *error, wasm_trap_t *trap);

int main(int argc, const char* argv[]) {
  // Configuring engine to support generating of DWARF info.
  // lldb can be used to attach to the program and observe
  // original fib-wasm.c source code and variables.
  wasm_config_t* config = wasm_config_new();
  wasmtime_config_debug_info_set(config, true);

  // Initialize.
  printf("Initializing...\n");
  wasm_engine_t* engine = wasm_engine_new_with_config(config);
  wasm_store_t* store = wasm_store_new(engine);

  // Load binary.
  printf("Loading binary...\n");
  FILE* file = fopen("target/wasm32-unknown-unknown/debug/fib.wasm", "rb");
  if (!file) {
    printf("> Error opening module!\n");
    return 1;
  }
  fseek(file, 0L, SEEK_END);
  size_t file_size = ftell(file);
  fseek(file, 0L, SEEK_SET);
  wasm_byte_vec_t binary;
  wasm_byte_vec_new_uninitialized(&binary, file_size);
  if (fread(binary.data, file_size, 1, file) != 1) {
    printf("> Error reading module!\n");
    return 1;
  }
  fclose(file);

  // Compile.
  printf("Compiling module...\n");
  wasm_module_t *module = NULL;
  wasmtime_error_t* error = wasmtime_module_new(store, &binary, &module);
  if (!module)
    exit_with_error("failed to compile module", error, NULL);
  wasm_byte_vec_delete(&binary);

  // Figure out which export is the `fib` export
  wasm_exporttype_vec_t module_exports;
  wasm_module_exports(module, &module_exports);
  int fib_idx = -1;
  for (int i = 0; i < module_exports.size; i++) {
    const wasm_name_t *name = wasm_exporttype_name(module_exports.data[i]);
    if (name->size != 3)
      continue;
    if (strncmp("fib", name->data, 3) != 0)
      continue;
    fib_idx = i;
    break;
  }
  wasm_exporttype_vec_delete(&module_exports);
  if (fib_idx == -1) {
    printf("> Error finding `fib` export!\n");
    return 1;
  }

  // Instantiate.
  printf("Instantiating module...\n");
  wasm_instance_t* instance = NULL;
  wasm_trap_t *trap = NULL;
  error = wasmtime_instance_new(store, module, NULL, 0, &instance, &trap);
  if (error != NULL || trap != NULL)
    exit_with_error("failed to instantiate", error, trap);
  wasm_module_delete(module);

  // Extract export.
  printf("Extracting export...\n");
  own wasm_extern_vec_t exports;
  wasm_instance_exports(instance, &exports);
  if (exports.size == 0) {
    printf("> Error accessing exports!\n");
    return 1;
  }
  // Getting second export (first is memory).
  wasm_func_t* run_func = wasm_extern_as_func(exports.data[fib_idx]);
  if (run_func == NULL) {
    printf("> Error accessing export!\n");
    return 1;
  }

  wasm_instance_delete(instance);

  // Call.
  printf("Calling fib...\n");
  wasm_val_t params[1] = { {.kind = WASM_I32, .of = {.i32 = 6}} };
  wasm_val_t results[1];
  error = wasmtime_func_call(run_func, params, 1, results, 1, &trap);
  if (error != NULL || trap != NULL)
    exit_with_error("failed to call function", error, trap);

  wasm_extern_vec_delete(&exports);

  printf("> fib(6) = %d\n", results[0].of.i32);

  // Shut down.
  printf("Shutting down...\n");
  wasm_store_delete(store);
  wasm_engine_delete(engine);

  // All done.
  printf("Done.\n");
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
