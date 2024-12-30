#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <wasm.h>
#include <wasmtime.h>

#ifdef WASMTIME_TEST_ONLY
// These are the declarations provided from GDB documentation, used to validate
// that we actually added some DWARF info:
// https://sourceware.org/gdb/current/onlinedocs/gdb.html/Declarations.html#Declarations
//
// NOTE: These are not required in your code, rather they are used for wasmtime
// testing only.
typedef enum {
  JIT_NOACTION = 0,
  JIT_REGISTER_FN,
  JIT_UNREGISTER_FN
} jit_actions_t;

struct jit_code_entry {
  struct jit_code_entry *next_entry;
  struct jit_code_entry *prev_entry;
  const char *symfile_addr;
  uint64_t symfile_size;
};

struct jit_descriptor {
  uint32_t version;
  /* This type should be jit_actions_t, but we use uint32_t
     to be explicit about the bitwidth.  */
  uint32_t action_flag;
  struct jit_code_entry *relevant_entry;
  struct jit_code_entry *first_entry;
};

/*
 * Import the descriptor, defined elsewhere in wasmtime
 */
extern struct jit_descriptor __jit_debug_descriptor;
#endif

#define own

static void exit_with_error(const char *message, wasmtime_error_t *error,
                            wasm_trap_t *trap);

int main(int argc, const char *argv[]) {
  // Configuring engine to support generating of DWARF info.
  // lldb can be used to attach to the program and observe
  // original fib-wasm.c source code and variables.
  wasm_config_t *config = wasm_config_new();
  wasmtime_config_debug_info_set(config, true);
  wasmtime_config_cranelift_opt_level_set(config, WASMTIME_OPT_LEVEL_NONE);

  // Initialize.
  printf("Initializing...\n");
  wasm_engine_t *engine = wasm_engine_new_with_config(config);
  wasmtime_store_t *store = wasmtime_store_new(engine, NULL, NULL);
  wasmtime_context_t *context = wasmtime_store_context(store);

#ifdef WASMTIME_TEST_ONLY
  // NOTE: This validation is for wasmtime testing and should not be included in
  // your code.
  if (__jit_debug_descriptor.first_entry != NULL) {
    fprintf(stderr, "FAIL: JIT descriptor is already initialized\n");
    return 1;
  }
#endif

  // Load binary.
  printf("Loading binary...\n");
  FILE *file = fopen("target/wasm32-unknown-unknown/debug/fib.wasm", "rb");
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
  wasmtime_module_t *module = NULL;
  wasmtime_error_t *error =
      wasmtime_module_new(engine, (uint8_t *)binary.data, binary.size, &module);
  if (!module)
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

#ifdef WASMTIME_TEST_ONLY
  // NOTE: This validation is for wasmtime testing and should not be included in
  // your code.
  if (__jit_debug_descriptor.first_entry == NULL) {
    fprintf(stderr, "FAIL: JIT descriptor is NOT initialized\n");
    return 1;
  }
#endif

  // Extract export.
  wasmtime_extern_t fib;
  bool ok = wasmtime_instance_export_get(context, &instance, "fib", 3, &fib);
  assert(ok);

  // Call.
  printf("Calling fib...\n");
  wasmtime_val_t params[1];
  params[0].kind = WASMTIME_I32;
  params[0].of.i32 = 6;
  wasmtime_val_t results[1];
  error =
      wasmtime_func_call(context, &fib.of.func, params, 1, results, 1, &trap);
  if (error != NULL || trap != NULL)
    exit_with_error("failed to call function", error, trap);

  assert(results[0].kind == WASMTIME_I32);
  printf("> fib(6) = %d\n", results[0].of.i32);

  // Shut down.
  printf("Shutting down...\n");
  wasmtime_store_delete(store);
  wasm_engine_delete(engine);

  // All done.
  printf("Done.\n");
  return 0;
}

static void exit_with_error(const char *message, wasmtime_error_t *error,
                            wasm_trap_t *trap) {
  fprintf(stderr, "error: %s\n", message);
  wasm_byte_vec_t error_message;
  if (error != NULL) {
    wasmtime_error_message(error, &error_message);
  } else {
    wasm_trap_message(trap, &error_message);
  }
  fprintf(stderr, "%.*s\n", (int)error_message.size, error_message.data);
  wasm_byte_vec_delete(&error_message);
  exit(1);
}
