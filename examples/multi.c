/*
Example of instantiating of the WebAssembly module and invoking its exported
function.

You can compile and run this example on Linux with:

   cargo build --release -p wasmtime-c-api
   cc examples/multi.c \
       -I crates/c-api/include \
       -I crates/c-api/wasm-c-api/include \
       target/release/libwasmtime.a \
       -lpthread -ldl -lm \
       -o multi
   ./multi

Note that on Windows and macOS the command will be similar, but you'll need
to tweak the `-lpthread` and such annotations.

Also note that this example was taken from
https://github.com/WebAssembly/wasm-c-api/blob/master/example/multi.c
originally
*/

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <inttypes.h>
#include <wasm.h>
#include <wasmtime.h>

static void exit_with_error(const char *message, wasmtime_error_t *error, wasm_trap_t *trap);

// A function to be called from Wasm code.
wasm_trap_t* callback(
  const wasm_val_vec_t* args, wasm_val_vec_t* results
) {
  printf("Calling back...\n");
  printf("> %"PRIu32" %"PRIu64"\n", args->data[0].of.i32, args->data[1].of.i64);
  printf("\n");

  wasm_val_copy(&results->data[0], &args->data[1]);
  wasm_val_copy(&results->data[1], &args->data[0]);
  return NULL;
}


// A function closure.
wasm_trap_t* closure_callback(
  void* env, const wasm_val_t args[], wasm_val_t results[]
) {
  int i = *(int*)env;
  printf("Calling back closure...\n");
  printf("> %d\n", i);

  results[0].kind = WASM_I32;
  results[0].of.i32 = (int32_t)i;
  return NULL;
}


int main(int argc, const char* argv[]) {
  // Initialize.
  printf("Initializing...\n");
  wasm_engine_t* engine = wasm_engine_new();
  wasm_store_t* store = wasm_store_new(engine);

  // Load our input file to parse it next
  FILE* file = fopen("examples/multi.wat", "r");
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
  wasm_byte_vec_t binary;
  wasmtime_error_t *error = wasmtime_wat2wasm(&wat, &binary);
  if (error != NULL)
    exit_with_error("failed to parse wat", error, NULL);
  wasm_byte_vec_delete(&wat);

  // Compile.
  printf("Compiling module...\n");
  wasm_module_t* module = NULL;
  error = wasmtime_module_new(engine, &binary, &module);
  if (error)
    exit_with_error("failed to compile module", error, NULL);

  wasm_byte_vec_delete(&binary);

  // Create external print functions.
  printf("Creating callback...\n");
  wasm_functype_t* callback_type = wasm_functype_new_2_2(
      wasm_valtype_new_i32(),
      wasm_valtype_new_i64(),
      wasm_valtype_new_i64(),
      wasm_valtype_new_i32()
  );
  wasm_func_t* callback_func =
    wasm_func_new(store, callback_type, callback);

  wasm_functype_delete(callback_type);

  // Instantiate.
  printf("Instantiating module...\n");
  wasm_extern_t* imports[] = {wasm_func_as_extern(callback_func)};
  wasm_extern_vec_t imports_vec = WASM_ARRAY_VEC(imports);
  wasm_instance_t* instance = NULL;
  wasm_trap_t* trap = NULL;
  error = wasmtime_instance_new(store, module, &imports_vec, &instance, &trap);
  if (!instance)
    exit_with_error("failed to instantiate", error, trap);

  wasm_func_delete(callback_func);

  // Extract export.
  printf("Extracting export...\n");
  wasm_extern_vec_t exports;
  wasm_instance_exports(instance, &exports);
  if (exports.size == 0) {
    printf("> Error accessing exports!\n");
    return 1;
  }
  wasm_func_t* run_func = wasm_extern_as_func(exports.data[0]);
  if (run_func == NULL) {
    printf("> Error accessing export!\n");
    return 1;
  }

  wasm_module_delete(module);
  wasm_instance_delete(instance);

  // Call.
  printf("Calling export...\n");
  wasm_val_t args[2] = { WASM_I32_VAL(1), WASM_I64_VAL(2) };
  wasm_val_t results[2];
  wasm_val_vec_t args_vec = WASM_ARRAY_VEC(args);
  wasm_val_vec_t results_vec = WASM_ARRAY_VEC(results);
  error = wasmtime_func_call(run_func, &args_vec, &results_vec, &trap);
  if (error != NULL || trap != NULL)
    exit_with_error("failed to call run", error, trap);

  wasm_extern_vec_delete(&exports);

  // Print result.
  printf("Printing result...\n");
  printf("> %"PRIu64" %"PRIu32"\n",
    results[0].of.i64, results[1].of.i32);

  assert(results[0].kind == WASM_I64);
  assert(results[0].of.i64 == 2);
  assert(results[1].kind == WASM_I32);
  assert(results[1].of.i32 == 1);

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
    wasmtime_error_delete(error);
  } else {
    wasm_trap_message(trap, &error_message);
    wasm_trap_delete(trap);
  }
  fprintf(stderr, "%.*s\n", (int) error_message.size, error_message.data);
  wasm_byte_vec_delete(&error_message);
  exit(1);
}
