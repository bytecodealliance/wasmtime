/*
Example of instantiating a WebAssembly which uses WASI imports.

You can compile and run this example on Linux with:

   cargo build --release -p wasmtime-c-api
   cc examples/wasi/main.c \
       -I crates/c-api/include \
       -I crates/c-api/wasm-c-api/include \
       target/release/libwasmtime.a \
       -lpthread -ldl -lm \
       -o wasi
   ./wasi

Note that on Windows and macOS the command will be similar, but you'll need
to tweak the `-lpthread` and such annotations.
*/

#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <wasm.h>
#include <wasi.h>
#include <wasmtime.h>

#define MIN(a, b) ((a) < (b) ? (a) : (b))

static void exit_with_error(const char *message, wasmtime_error_t *error, wasm_trap_t *trap);

int main() {
  int ret = 0;
  // Set up our context
  wasm_engine_t *engine = wasm_engine_new();
  assert(engine != NULL);
  wasm_store_t *store = wasm_store_new(engine);
  assert(store != NULL);

  wasm_byte_vec_t wasm;
  // Load our input file to parse it next
  FILE* file = fopen("target/wasm32-wasi/debug/wasi.wasm", "rb");
  if (!file) {
    printf("> Error loading file!\n");
    exit(1);
  }
  fseek(file, 0L, SEEK_END);
  size_t file_size = ftell(file);
  wasm_byte_vec_new_uninitialized(&wasm, file_size);
  fseek(file, 0L, SEEK_SET);
  if (fread(wasm.data, file_size, 1, file) != 1) {
    printf("> Error loading module!\n");
    exit(1);
  }
  fclose(file);

  // Compile our modules
  wasm_module_t *module = NULL;
  wasmtime_error_t *error = wasmtime_module_new(engine, &wasm, &module);
  if (!module)
    exit_with_error("failed to compile module", error, NULL);
  wasm_byte_vec_delete(&wasm);

  // Instantiate wasi
  wasi_config_t *wasi_config = wasi_config_new();
  assert(wasi_config);
  wasi_config_inherit_argv(wasi_config);
  wasi_config_inherit_env(wasi_config);
  wasi_config_inherit_stdin(wasi_config);
  wasi_config_inherit_stdout(wasi_config);
  wasi_config_inherit_stderr(wasi_config);
  wasm_trap_t *trap = NULL;
  wasi_instance_t *wasi = wasi_instance_new(store, "wasi_snapshot_preview1", wasi_config, &trap);
  if (wasi == NULL)
    exit_with_error("failed to instantiate WASI", NULL, trap);

  wasmtime_linker_t *linker = wasmtime_linker_new(store);
  error = wasmtime_linker_define_wasi(linker, wasi);
  if (error != NULL)
    exit_with_error("failed to link wasi", error, NULL);

  // Instantiate the module
  wasm_name_t empty;
  wasm_name_new_from_string(&empty, "");
  wasm_instance_t *instance = NULL;
  error = wasmtime_linker_module(linker, &empty, module);
  if (error != NULL)
    exit_with_error("failed to instantiate module", error, NULL);

  // Run it.
  wasm_func_t* func;
  wasmtime_linker_get_default(linker, &empty, &func);
  if (error != NULL)
    exit_with_error("failed to locate default export for module", error, NULL);

  wasm_val_vec_t args_vec = WASM_EMPTY_VEC;
  wasm_val_vec_t results_vec = WASM_EMPTY_VEC;
  error = wasmtime_func_call(func, &args_vec, &results_vec, &trap);
  if (error != NULL)
    exit_with_error("error calling default export", error, trap);

  // Clean up after ourselves at this point
  wasm_name_delete(&empty);
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
    wasmtime_error_delete(error);
  } else {
    wasm_trap_message(trap, &error_message);
    wasm_trap_delete(trap);
  }
  fprintf(stderr, "%.*s\n", (int) error_message.size, error_message.data);
  wasm_byte_vec_delete(&error_message);
  exit(1);
}
