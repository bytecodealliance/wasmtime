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
  void *env,
  wasmtime_caller_t *caller,
  const wasmtime_val_t* args,
  size_t nargs,
  wasmtime_val_t* results,
  size_t nresults
) {
  printf("Calling back...\n");
  printf("> %"PRIu32" %"PRIu64"\n", args[0].of.i32, args[1].of.i64);
  printf("\n");

  results[0] = args[1];
  results[1] = args[0];
  return NULL;
}


// A function closure.
wasm_trap_t* closure_callback(
  void* env,
  wasmtime_caller_t *caller,
  const wasmtime_val_t* args,
  size_t nargs,
  wasmtime_val_t* results,
  size_t nresults
) {
  int i = *(int*)env;
  printf("Calling back closure...\n");
  printf("> %d\n", i);

  results[0].kind = WASMTIME_I32;
  results[0].of.i32 = (int32_t)i;
  return NULL;
}


int main(int argc, const char* argv[]) {
  // Initialize.
  printf("Initializing...\n");
  wasm_engine_t* engine = wasm_engine_new();
  wasmtime_store_t* store = wasmtime_store_new(engine, NULL, NULL);
  wasmtime_context_t *context = wasmtime_store_context(store);

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
  wasmtime_error_t *error = wasmtime_wat2wasm(wat.data, wat.size, &binary);
  if (error != NULL)
    exit_with_error("failed to parse wat", error, NULL);
  wasm_byte_vec_delete(&wat);

  // Compile.
  printf("Compiling module...\n");
  wasmtime_module_t* module = NULL;
  error = wasmtime_module_new(engine, (uint8_t*) binary.data, binary.size, &module);
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
  wasmtime_func_t callback_func;
  wasmtime_func_new(context, callback_type, callback, NULL, NULL, &callback_func);
  wasm_functype_delete(callback_type);

  // Instantiate.
  printf("Instantiating module...\n");
  wasmtime_extern_t imports[1];
  imports[0].kind = WASMTIME_EXTERN_FUNC;
  imports[0].of.func = callback_func;
  wasmtime_instance_t instance;
  wasm_trap_t* trap = NULL;
  error = wasmtime_instance_new(context, module, imports, 1, &instance, &trap);
  if (error != NULL || trap != NULL)
    exit_with_error("failed to instantiate", error, trap);
  wasmtime_module_delete(module);

  // Extract export.
  printf("Extracting export...\n");
  wasmtime_extern_t run;
  bool ok = wasmtime_instance_export_get(context, &instance, "g", 1, &run);
  assert(ok);
  assert(run.kind == WASMTIME_EXTERN_FUNC);

  // Call.
  printf("Calling export...\n");
  wasmtime_val_t args[2];
  args[0].kind = WASMTIME_I32;
  args[0].of.i32 = 1;
  args[1].kind = WASMTIME_I64;
  args[1].of.i64 = 2;
  wasmtime_val_t results[2];
  error = wasmtime_func_call(context, &run.of.func, args, 2, results, 2, &trap);
  if (error != NULL || trap != NULL)
    exit_with_error("failed to call run", error, trap);

  // Print result.
  printf("Printing result...\n");
  printf("> %"PRIu64" %"PRIu32"\n",
    results[0].of.i64, results[1].of.i32);

  assert(results[0].kind == WASMTIME_I64);
  assert(results[0].of.i64 == 2);
  assert(results[1].kind == WASMTIME_I32);
  assert(results[1].of.i32 == 1);

  // Shut down.
  printf("Shutting down...\n");
  wasmtime_store_delete(store);
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
