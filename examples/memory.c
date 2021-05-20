/*
Example of instantiating of the WebAssembly module and invoking its exported
function.

You can compile and run this example on Linux with:

   cargo build --release -p wasmtime-c-api
   cc examples/memory.c \
       -I crates/c-api/include \
       -I crates/c-api/wasm-c-api/include \
       target/release/libwasmtime.a \
       -lpthread -ldl -lm \
       -o memory
   ./memory

Note that on Windows and macOS the command will be similar, but you'll need
to tweak the `-lpthread` and such annotations.

Also note that this example was taken from
https://github.com/WebAssembly/wasm-c-api/blob/master/example/memory.c
originally
*/

#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <wasm.h>
#include <wasmtime.h>

static void exit_with_error(const char *message, wasmtime_error_t *error, wasm_trap_t *trap);

void check(bool success) {
  if (!success) {
    printf("> Error, expected success\n");
    exit(1);
  }
}

void check_call(wasmtime_context_t *store,
                wasmtime_func_t *func,
                const wasmtime_val_t* args,
                size_t nargs,
                int32_t expected) {
  wasmtime_val_t results[1];
  wasm_trap_t *trap = NULL;
  wasmtime_error_t *error = wasmtime_func_call(
      store, func, args, nargs, results, 1, &trap
  );
  if (error != NULL || trap != NULL)
    exit_with_error("failed to call function", error, trap);
  if (results[0].of.i32 != expected) {
    printf("> Error on result\n");
    exit(1);
  }
}

void check_call0(wasmtime_context_t *store, wasmtime_func_t *func, int32_t expected) {
  check_call(store, func, NULL, 0, expected);
}

void check_call1(wasmtime_context_t *store, wasmtime_func_t *func, int32_t arg, int32_t expected) {
  wasmtime_val_t args[1];
  args[0].kind = WASMTIME_I32;
  args[0].of.i32 = arg;
  check_call(store, func, args, 1, expected);
}

void check_call2(wasmtime_context_t *store, wasmtime_func_t *func, int32_t arg1, int32_t arg2, int32_t expected) {
  wasmtime_val_t args[2];
  args[0].kind = WASMTIME_I32;
  args[0].of.i32 = arg1;
  args[1].kind = WASMTIME_I32;
  args[1].of.i32 = arg2;
  check_call(store, func, args, 2, expected);
}

void check_ok(wasmtime_context_t *store, wasmtime_func_t *func, const wasmtime_val_t* args, size_t nargs) {
  wasm_trap_t *trap = NULL;
  wasmtime_error_t *error = wasmtime_func_call(store, func, args, nargs, NULL, 0, &trap);
  if (error != NULL || trap != NULL)
    exit_with_error("failed to call function", error, trap);
}

void check_ok2(wasmtime_context_t *store, wasmtime_func_t *func, int32_t arg1, int32_t arg2) {
  wasmtime_val_t args[2];
  args[0].kind = WASMTIME_I32;
  args[0].of.i32 = arg1;
  args[1].kind = WASMTIME_I32;
  args[1].of.i32 = arg2;
  check_ok(store, func, args, 2);
}

void check_trap(wasmtime_context_t *store,
                wasmtime_func_t *func,
                const wasmtime_val_t *args,
                size_t nargs,
                size_t num_results) {
  assert(num_results <= 1);
  wasmtime_val_t results[1];
  wasm_trap_t *trap = NULL;
  wasmtime_error_t *error = wasmtime_func_call(store, func, args, nargs, results, num_results, &trap);
  if (error != NULL)
    exit_with_error("failed to call function", error, NULL);
  if (trap == NULL) {
    printf("> Error on result, expected trap\n");
    exit(1);
  }
  wasm_trap_delete(trap);
}

void check_trap1(wasmtime_context_t *store, wasmtime_func_t *func, int32_t arg) {
  wasmtime_val_t args[1];
  args[0].kind = WASMTIME_I32;
  args[0].of.i32 = arg;
  check_trap(store, func, args, 1, 1);
}

void check_trap2(wasmtime_context_t *store, wasmtime_func_t *func, int32_t arg1, int32_t arg2) {
  wasmtime_val_t args[2];
  args[0].kind = WASMTIME_I32;
  args[0].of.i32 = arg1;
  args[1].kind = WASMTIME_I32;
  args[1].of.i32 = arg2;
  check_trap(store, func, args, 2, 0);
}

int main(int argc, const char* argv[]) {
  // Initialize.
  printf("Initializing...\n");
  wasm_engine_t* engine = wasm_engine_new();
  wasmtime_store_t* store = wasmtime_store_new(engine, NULL, NULL);
  wasmtime_context_t *context = wasmtime_store_context(store);

  // Load our input file to parse it next
  FILE* file = fopen("examples/memory.wat", "r");
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

  // Instantiate.
  printf("Instantiating module...\n");
  wasmtime_instance_t instance;
  wasm_trap_t *trap = NULL;
  error = wasmtime_instance_new(context, module, NULL, 0, &instance, &trap);
  if (error != NULL || trap != NULL)
    exit_with_error("failed to instantiate", error, trap);
  wasmtime_module_delete(module);

  // Extract export.
  printf("Extracting exports...\n");
  wasmtime_memory_t memory;
  wasmtime_func_t size_func, load_func, store_func;
  wasmtime_extern_t item;
  bool ok;
  ok = wasmtime_instance_export_get(context, &instance, "memory", strlen("memory"), &item);
  assert(ok && item.kind == WASMTIME_EXTERN_MEMORY);
  memory = item.of.memory;
  ok = wasmtime_instance_export_get(context, &instance, "size", strlen("size"), &item);
  assert(ok && item.kind == WASMTIME_EXTERN_FUNC);
  size_func = item.of.func;
  ok = wasmtime_instance_export_get(context, &instance, "load", strlen("load"), &item);
  assert(ok && item.kind == WASMTIME_EXTERN_FUNC);
  load_func = item.of.func;
  ok = wasmtime_instance_export_get(context, &instance, "store", strlen("store"), &item);
  assert(ok && item.kind == WASMTIME_EXTERN_FUNC);
  store_func = item.of.func;

  // Check initial memory.
  printf("Checking memory...\n");
  check(wasmtime_memory_size(context, &memory) == 2);
  check(wasmtime_memory_data_size(context, &memory) == 0x20000);
  check(wasmtime_memory_data(context, &memory)[0] == 0);
  check(wasmtime_memory_data(context, &memory)[0x1000] == 1);
  check(wasmtime_memory_data(context, &memory)[0x1003] == 4);

  check_call0(context, &size_func, 2);
  check_call1(context, &load_func, 0, 0);
  check_call1(context, &load_func, 0x1000, 1);
  check_call1(context, &load_func, 0x1003, 4);
  check_call1(context, &load_func, 0x1ffff, 0);
  check_trap1(context, &load_func, 0x20000);

  // Mutate memory.
  printf("Mutating memory...\n");
  wasmtime_memory_data(context, &memory)[0x1003] = 5;
  check_ok2(context, &store_func, 0x1002, 6);
  check_trap2(context, &store_func, 0x20000, 0);

  check(wasmtime_memory_data(context, &memory)[0x1002] == 6);
  check(wasmtime_memory_data(context, &memory)[0x1003] == 5);
  check_call1(context, &load_func, 0x1002, 6);
  check_call1(context, &load_func, 0x1003, 5);

  // Grow memory.
  printf("Growing memory...\n");
  uint32_t old_size;
  error = wasmtime_memory_grow(context, &memory, 1, &old_size);
  if (error != NULL)
    exit_with_error("failed to grow memory", error, trap);
  check(wasmtime_memory_size(context, &memory) == 3);
  check(wasmtime_memory_data_size(context, &memory) == 0x30000);

  check_call1(context, &load_func, 0x20000, 0);
  check_ok2(context, &store_func, 0x20000, 0);
  check_trap1(context, &load_func, 0x30000);
  check_trap2(context, &store_func, 0x30000, 0);

  error = wasmtime_memory_grow(context, &memory, 1, &old_size);
  assert(error != NULL);
  wasmtime_error_delete(error);
  error = wasmtime_memory_grow(context, &memory, 0, &old_size);
  if (error != NULL)
    exit_with_error("failed to grow memory", error, trap);

  // Create stand-alone memory.
  printf("Creating stand-alone memory...\n");
  wasm_limits_t limits = {5, 5};
  wasm_memorytype_t* memorytype = wasm_memorytype_new(&limits);
  wasmtime_memory_t memory2;
  error = wasmtime_memory_new(context, memorytype, &memory2);
  if (error != NULL)
    exit_with_error("failed to create memory", error, trap);
  wasm_memorytype_delete(memorytype);
  check(wasmtime_memory_size(context, &memory2) == 5);

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
