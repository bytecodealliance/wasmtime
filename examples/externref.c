/*
Example of using `externref` values.

You can compile and run this example on Linux with:

   cargo build --release -p wasmtime
   cc examples/externref.c \
       -I crates/c-api/include \
       -I crates/c-api/wasm-c-api/include \
       target/release/libwasmtime.a \
       -lpthread -ldl -lm \
       -o externref
   ./externref

Note that on Windows and macOS the command will be similar, but you'll need
to tweak the `-lpthread` and such annotations as well as the name of the
`libwasmtime.a` file on Windows.
*/

#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <wasm.h>
#include <wasmtime.h>

static void exit_with_error(const char *message, wasmtime_error_t *error, wasm_trap_t *trap);

int main() {
  int ret = 0;
  bool ok = true;
  // Create a new configuration with Wasm reference types enabled.
  printf("Initializing...\n");
  wasm_config_t *config = wasm_config_new();
  assert(config != NULL);
  wasmtime_config_wasm_reference_types_set(config, true);

  // Create an *engine*, which is a compilation context, with our configured
  // options.
  wasm_engine_t *engine = wasm_engine_new_with_config(config);
  assert(engine != NULL);

  // With an engine we can create a *store* which is a long-lived group of wasm
  // modules.
  wasm_store_t *store = wasm_store_new(engine);
  assert(store != NULL);

  // Read our input file, which in this case is a wasm text file.
  FILE* file = fopen("examples/externref.wat", "r");
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
  wasmtime_error_t *error = wasmtime_wat2wasm(&wat, &wasm);
  if (error != NULL)
    exit_with_error("failed to parse wat", error, NULL);
  wasm_byte_vec_delete(&wat);

  // Now that we've got our binary webassembly we can compile our module.
  printf("Compiling module...\n");
  wasm_module_t *module = NULL;
  error = wasmtime_module_new(engine, &wasm, &module);
  wasm_byte_vec_delete(&wasm);
  if (error != NULL)
    exit_with_error("failed to compile module", error, NULL);

  // Instantiate the module.
  printf("Instantiating module...\n");
  wasm_trap_t *trap = NULL;
  wasm_instance_t *instance = NULL;
  error = wasmtime_instance_new(store, module, NULL, 0, &instance, &trap);
  if (instance == NULL)
    exit_with_error("failed to instantiate", error, trap);

  printf("Creating new `externref`...\n");

  // Create a new `externref` value.
  wasm_val_t externref;
  wasmtime_externref_new("Hello, World!", &externref);
  assert(externref.kind == WASM_ANYREF);

  // The `externref`'s wrapped data should be the string "Hello, World!".
  void* data = NULL;
  ok = wasmtime_externref_data(&externref, &data);
  assert(ok);
  assert(strcmp((char*)data, "Hello, World!") == 0);

  printf("Touching `externref` table...\n");

  // Lookup the `table` export.
  wasm_extern_vec_t externs;
  wasm_instance_exports(instance, &externs);
  assert(externs.size == 3);
  wasm_table_t *table = wasm_extern_as_table(externs.data[0]);
  assert(table != NULL);

  // Set `table[3]` to our `externref`.
  wasm_val_t elem;
  wasm_val_copy(&elem, &externref);
  assert(elem.kind == WASM_ANYREF);
  ok = wasm_table_set(table, 3, elem.of.ref);
  assert(ok);

  // `table[3]` should now be our `externref`.
  wasm_ref_delete(elem.of.ref);
  elem.of.ref = wasm_table_get(table, 3);
  assert(elem.of.ref != NULL);
  assert(wasm_ref_same(elem.of.ref, externref.of.ref));

  printf("Touching `externref` global...\n");

  // Lookup the `global` export.
  wasm_global_t *global = wasm_extern_as_global(externs.data[1]);
  assert(global != NULL);

  // Set the global to our `externref`.
  wasm_global_set(global, &externref);

  // Get the global, and it should return our `externref` again.
  wasm_val_t global_val;
  wasm_global_get(global, &global_val);
  assert(global_val.kind == WASM_ANYREF);
  assert(wasm_ref_same(global_val.of.ref, externref.of.ref));

  printf("Calling `externref` func...\n");

  // Lookup the `func` export.
  wasm_func_t *func = wasm_extern_as_func(externs.data[2]);
  assert(func != NULL);

  // And call it!
  wasm_val_t args[1];
  wasm_val_copy(&args[0], &externref);
  wasm_val_t results[1];
  error = wasmtime_func_call(func, args, 1, results, 1, &trap);
  if (error != NULL || trap != NULL)
    exit_with_error("failed to call function", error, trap);

  // `func` returns the same reference we gave it, so `results[0]` should be our
  // `externref`.
  assert(results[0].kind == WASM_ANYREF);
  assert(wasm_ref_same(results[0].of.ref, externref.of.ref));

  // Clean up after ourselves at this point
  printf("All finished!\n");
  ret = 0;

  wasm_val_delete(&results[0]);
  wasm_val_delete(&args[0]);
  wasm_val_delete(&global_val);
  wasm_val_delete(&elem);
  wasm_extern_vec_delete(&externs);
  wasm_val_delete(&externref);
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
    wasmtime_error_delete(error);
  } else {
    wasm_trap_message(trap, &error_message);
    wasm_trap_delete(trap);
  }
  fprintf(stderr, "%.*s\n", (int) error_message.size, error_message.data);
  wasm_byte_vec_delete(&error_message);
  exit(1);
}
