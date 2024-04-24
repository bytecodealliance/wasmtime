/*
Example of using `externref` values.

You can compile and run this example on Linux with:

   cargo build --release -p wasmtime-c-api
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

You can also build using cmake:

mkdir build && cd build && cmake .. && \
  cmake --build . --target wasmtime-externref
*/

#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <wasm.h>
#include <wasmtime.h>

static void exit_with_error(const char *message, wasmtime_error_t *error,
                            wasm_trap_t *trap);

int main() {
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

  // With an engine we can create a *store* which is a group of wasm instances
  // that can interact with each other.
  wasmtime_store_t *store = wasmtime_store_new(engine, NULL, NULL);
  assert(store != NULL);
  wasmtime_context_t *context = wasmtime_store_context(store);

  // Read our input file, which in this case is a wasm text file.
  FILE *file = fopen("examples/externref.wat", "r");
  assert(file != NULL);
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
  wasmtime_error_t *error = wasmtime_wat2wasm(wat.data, wat.size, &wasm);
  if (error != NULL)
    exit_with_error("failed to parse wat", error, NULL);
  wasm_byte_vec_delete(&wat);

  // Now that we've got our binary webassembly we can compile our module.
  printf("Compiling module...\n");
  wasmtime_module_t *module = NULL;
  error = wasmtime_module_new(engine, (uint8_t *)wasm.data, wasm.size, &module);
  wasm_byte_vec_delete(&wasm);
  if (error != NULL)
    exit_with_error("failed to compile module", error, NULL);

  // Instantiate the module.
  printf("Instantiating module...\n");
  wasm_trap_t *trap = NULL;
  wasmtime_instance_t instance;
  error = wasmtime_instance_new(context, module, NULL, 0, &instance, &trap);
  if (error != NULL || trap != NULL)
    exit_with_error("failed to instantiate", error, trap);

  printf("Creating new `externref`...\n");

  // Create a new `externref` value.
  //
  // Note that the NULL here is a finalizer callback, but we don't need one for
  // this example.
  wasmtime_externref_t externref;
  ok = wasmtime_externref_new(context, "Hello, World!", NULL, &externref);
  assert(ok);

  // The `externref`'s wrapped data should be the string "Hello, World!".
  void *data = wasmtime_externref_data(context, &externref);
  assert(strcmp((char *)data, "Hello, World!") == 0);

  wasmtime_extern_t item;
  wasmtime_val_t externref_val;
  externref_val.kind = WASMTIME_EXTERNREF;
  externref_val.of.externref = externref;

  // Lookup the `table` export.
  printf("Touching `externref` table...\n");
  {
    ok = wasmtime_instance_export_get(context, &instance, "table",
                                      strlen("table"), &item);
    assert(ok);
    assert(item.kind == WASMTIME_EXTERN_TABLE);

    // Set `table[3]` to our `externref`.
    error = wasmtime_table_set(context, &item.of.table, 3, &externref_val);
    if (error != NULL)
      exit_with_error("failed to set table", error, NULL);

    // `table[3]` should now be our `externref`.
    wasmtime_val_t elem;
    ok = wasmtime_table_get(context, &item.of.table, 3, &elem);
    assert(ok);
    assert(elem.kind == WASMTIME_EXTERNREF);
    assert(strcmp((char *)wasmtime_externref_data(context, &elem.of.externref),
                  "Hello, World!") == 0);
    wasmtime_val_unroot(context, &elem);
  }

  printf("Touching `externref` global...\n");

  // Lookup the `global` export.
  {
    ok = wasmtime_instance_export_get(context, &instance, "global",
                                      strlen("global"), &item);
    assert(ok);
    assert(item.kind == WASMTIME_EXTERN_GLOBAL);

    // Set the global to our `externref`.
    error = wasmtime_global_set(context, &item.of.global, &externref_val);
    if (error != NULL)
      exit_with_error("failed to set global", error, NULL);

    // Get the global, and it should return our `externref` again.
    wasmtime_val_t global_val;
    wasmtime_global_get(context, &item.of.global, &global_val);
    assert(global_val.kind == WASMTIME_EXTERNREF);
    assert(strcmp((char *)wasmtime_externref_data(context,
                                                  &global_val.of.externref),
                  "Hello, World!") == 0);
    wasmtime_val_unroot(context, &global_val);
  }

  printf("Calling `externref` func...\n");

  // Lookup the `func` export.
  {
    ok = wasmtime_instance_export_get(context, &instance, "func",
                                      strlen("func"), &item);
    assert(ok);
    assert(item.kind == WASMTIME_EXTERN_FUNC);

    // And call it!
    wasmtime_val_t results[1];
    error = wasmtime_func_call(context, &item.of.func, &externref_val, 1,
                               results, 1, &trap);
    if (error != NULL || trap != NULL)
      exit_with_error("failed to call function", error, trap);

    // `func` returns the same reference we gave it, so `results[0]` should be
    // our `externref`.
    assert(results[0].kind == WASMTIME_EXTERNREF);
    assert(strcmp((char *)wasmtime_externref_data(context,
                                                  &results[0].of.externref),
                  "Hello, World!") == 0);
    wasmtime_val_unroot(context, &results[0]);
  }
  wasmtime_val_unroot(context, &externref_val);

  // We can GC any now-unused references to our externref that the store is
  // holding.
  printf("GCing within the store...\n");
  wasmtime_context_gc(context);

  // Clean up after ourselves at this point
  printf("All finished!\n");

  wasmtime_store_delete(store);
  wasmtime_module_delete(module);
  wasm_engine_delete(engine);
  return 0;
}

static void exit_with_error(const char *message, wasmtime_error_t *error,
                            wasm_trap_t *trap) {
  fprintf(stderr, "error: %s\n", message);
  wasm_byte_vec_t error_message;
  if (error != NULL) {
    wasmtime_error_message(error, &error_message);
    wasmtime_error_delete(error);
  } else {
    wasm_trap_message(trap, &error_message);
    wasm_trap_delete(trap);
  }
  fprintf(stderr, "%.*s\n", (int)error_message.size, error_message.data);
  wasm_byte_vec_delete(&error_message);
  exit(1);
}
