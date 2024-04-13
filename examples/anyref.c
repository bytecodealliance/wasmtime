/*
Example of using `anyref` values.

You can compile and run this example on Linux with:

   cargo build --release -p wasmtime-c-api
   cc examples/anyref.c \
       -I crates/c-api/include \
       -I crates/c-api/wasm-c-api/include \
       target/release/libwasmtime.a \
       -lpthread -ldl -lm \
       -o anyref
   ./anyref

Note that on Windows and macOS the command will be similar, but you'll need
to tweak the `-lpthread` and such annotations as well as the name of the
`libwasmtime.a` file on Windows.

You can also build using cmake:

mkdir build && cd build && cmake .. && \
  cmake --build . --target wasmtime-anyref
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
  // Create a new configuration with Wasm GC enabled.
  printf("Initializing...\n");
  wasm_config_t *config = wasm_config_new();
  assert(config != NULL);
  wasmtime_config_wasm_reference_types_set(config, true);
  wasmtime_config_wasm_function_references_set(config, true);
  wasmtime_config_wasm_gc_set(config, true);

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
  FILE *file = fopen("examples/anyref.wat", "r");
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

  printf("Creating new `anyref` from i31...\n");

  // Create a new `anyref` value from an i31 (`i31ref` is a subtype of
  // `anyref`).
  wasmtime_anyref_t *anyref = wasmtime_anyref_from_i31(context, 1234);
  assert(anyref != NULL);

  // The `anyref`'s inner i31 value should be 1234.
  uint32_t i31val = 0;
  bool is_i31 = wasmtime_anyref_i31_get_u(context, anyref, &i31val);
  assert(is_i31);
  assert(i31val == 1234);

  printf("Touching `anyref` table...\n");

  wasmtime_extern_t item;

  // Lookup the `table` export.
  ok = wasmtime_instance_export_get(context, &instance, "table",
                                    strlen("table"), &item);
  assert(ok);
  assert(item.kind == WASMTIME_EXTERN_TABLE);

  // Set `table[3]` to our `anyref`.
  wasmtime_val_t anyref_val;
  anyref_val.kind = WASMTIME_ANYREF;
  anyref_val.of.anyref = anyref;
  error = wasmtime_table_set(context, &item.of.table, 3, &anyref_val);
  if (error != NULL)
    exit_with_error("failed to set table", error, NULL);

  // `table[3]` should now be our `anyref`.
  wasmtime_val_t elem;
  ok = wasmtime_table_get(context, &item.of.table, 3, &elem);
  assert(ok);
  assert(elem.kind == WASMTIME_ANYREF);
  is_i31 = false;
  i31val = 0;
  is_i31 = wasmtime_anyref_i31_get_u(context, elem.of.anyref, &i31val);
  assert(is_i31);
  assert(i31val == 1234);
  wasmtime_val_delete(context, &elem);

  printf("Touching `anyref` global...\n");

  // Lookup the `global` export.
  ok = wasmtime_instance_export_get(context, &instance, "global",
                                    strlen("global"), &item);
  assert(ok);
  assert(item.kind == WASMTIME_EXTERN_GLOBAL);

  // Set the global to our `anyref`.
  error = wasmtime_global_set(context, &item.of.global, &anyref_val);
  if (error != NULL)
    exit_with_error("failed to set global", error, NULL);

  // Get the global, and it should return our `anyref` again.
  wasmtime_val_t global_val;
  wasmtime_global_get(context, &item.of.global, &global_val);
  assert(global_val.kind == WASMTIME_ANYREF);
  is_i31 = false;
  i31val = 0;
  is_i31 = wasmtime_anyref_i31_get_u(context, global_val.of.anyref, &i31val);
  assert(is_i31);
  assert(i31val == 1234);
  wasmtime_val_delete(context, &global_val);

  printf("Passing `anyref` into func...\n");

  // Lookup the `take_anyref` export.
  ok = wasmtime_instance_export_get(context, &instance, "take_anyref",
                                    strlen("take_anyref"), &item);
  assert(ok);
  assert(item.kind == WASMTIME_EXTERN_FUNC);

  // And call it!
  error = wasmtime_func_call(context, &item.of.func, &anyref_val, 1, NULL, 0,
                             &trap);
  if (error != NULL || trap != NULL)
    exit_with_error("failed to call function", error, trap);

  printf("Getting `anyref` from func...\n");

  // Lookup the `return_anyref` export.
  ok = wasmtime_instance_export_get(context, &instance, "return_anyref",
                                    strlen("return_anyref"), &item);
  assert(ok);
  assert(item.kind == WASMTIME_EXTERN_FUNC);

  // And call it!
  wasmtime_val_t results[1];
  error =
      wasmtime_func_call(context, &item.of.func, NULL, 0, results, 1, &trap);
  if (error != NULL || trap != NULL)
    exit_with_error("failed to call function", error, trap);

  // `return_anyfunc` returns an `i31ref` that wraps `42`.
  assert(results[0].kind == WASMTIME_ANYREF);
  is_i31 = false;
  i31val = 0;
  is_i31 = wasmtime_anyref_i31_get_u(context, results[0].of.anyref, &i31val);
  assert(is_i31);
  assert(i31val == 42);
  wasmtime_val_delete(context, &results[0]);
  wasmtime_val_delete(context, &anyref_val);

  // We can GC any now-unused references to our anyref that the store is
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
