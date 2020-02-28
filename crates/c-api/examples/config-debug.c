#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <inttypes.h>

#include <wasm.h>
#include "wasmtime.h"

#define own

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
  FILE* file = fopen("fib-wasm.wasm", "r");
  if (!file) {
    printf("> Error loading module!\n");
    return 1;
  }
  fseek(file, 0L, SEEK_END);
  size_t file_size = ftell(file);
  fseek(file, 0L, SEEK_SET);
  wasm_byte_vec_t binary;
  wasm_byte_vec_new_uninitialized(&binary, file_size);
  if (fread(binary.data, file_size, 1, file) != 1) {
    printf("> Error loading module!\n");
    return 1;
  }
  fclose(file);

  // Compile.
  printf("Compiling module...\n");
  own wasm_module_t* module = wasm_module_new(store, &binary);
  if (!module) {
    printf("> Error compiling module!\n");
    return 1;
  }

  wasm_byte_vec_delete(&binary);

  // Instantiate.
  printf("Instantiating module...\n");
  own wasm_instance_t* instance =
    wasm_instance_new(store, module, NULL, NULL);
  if (!instance) {
    printf("> Error instantiating module!\n");
    return 1;
  }

  // Extract export.
  printf("Extracting export...\n");
  own wasm_extern_vec_t exports;
  wasm_instance_exports(instance, &exports);
  if (exports.size == 0) {
    printf("> Error accessing exports!\n");
    return 1;
  }
  // Getting second export (first is memory).
  const wasm_func_t* run_func = wasm_extern_as_func(exports.data[1]);
  if (run_func == NULL) {
    printf("> Error accessing export!\n");
    return 1;
  }

  wasm_module_delete(module);
  wasm_instance_delete(instance);

  // Call.
  printf("Calling fib...\n");
  wasm_val_t params[1] = { {.kind = WASM_I32, .of = {.i32 = 6}} };
  wasm_val_t results[1];
  if (wasm_func_call(run_func, params, results)) {
    printf("> Error calling function!\n");
    return 1;
  }

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
