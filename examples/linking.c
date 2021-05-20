/*
Example of compiling, instantiating, and linking two WebAssembly modules
together.

You can compile and run this example on Linux with:

   cargo build --release -p wasmtime-c-api
   cc examples/linking.c \
       -I crates/c-api/include \
       -I crates/c-api/wasm-c-api/include \
       target/release/libwasmtime.a \
       -lpthread -ldl -lm \
       -o linking
   ./linking

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
static void read_wat_file(wasm_engine_t *engine, wasm_byte_vec_t *bytes, const char *file);

int main() {
  // Set up our context
  wasm_engine_t *engine = wasm_engine_new();
  assert(engine != NULL);
  wasmtime_store_t *store = wasmtime_store_new(engine, NULL, NULL);
  assert(store != NULL);
  wasmtime_context_t *context = wasmtime_store_context(store);

  wasm_byte_vec_t linking1_wasm, linking2_wasm;
  read_wat_file(engine, &linking1_wasm, "examples/linking1.wat");
  read_wat_file(engine, &linking2_wasm, "examples/linking2.wat");

  // Compile our two modules
  wasmtime_error_t *error;
  wasmtime_module_t *linking1_module = NULL;
  wasmtime_module_t *linking2_module = NULL;
  error = wasmtime_module_new(engine, (uint8_t*) linking1_wasm.data, linking1_wasm.size, &linking1_module);
  if (error != NULL)
    exit_with_error("failed to compile linking1", error, NULL);
  error = wasmtime_module_new(engine, (uint8_t*) linking2_wasm.data, linking2_wasm.size, &linking2_module);
  if (error != NULL)
    exit_with_error("failed to compile linking2", error, NULL);
  wasm_byte_vec_delete(&linking1_wasm);
  wasm_byte_vec_delete(&linking2_wasm);

  // Configure WASI and store it within our `wasmtime_store_t`
  wasi_config_t *wasi_config = wasi_config_new();
  assert(wasi_config);
  wasi_config_inherit_argv(wasi_config);
  wasi_config_inherit_env(wasi_config);
  wasi_config_inherit_stdin(wasi_config);
  wasi_config_inherit_stdout(wasi_config);
  wasi_config_inherit_stderr(wasi_config);
  wasm_trap_t *trap = NULL;
  error = wasmtime_context_set_wasi(context, wasi_config);
  if (error != NULL)
    exit_with_error("failed to instantiate wasi", NULL, trap);

  // Create our linker which will be linking our modules together, and then add
  // our WASI instance to it.
  wasmtime_linker_t *linker = wasmtime_linker_new(engine);
  error = wasmtime_linker_define_wasi(linker);
  if (error != NULL)
    exit_with_error("failed to link wasi", error, NULL);

  // Instantiate `linking2` with our linker.
  wasmtime_instance_t linking2;
  error = wasmtime_linker_instantiate(linker, context, linking2_module, &linking2, &trap);
  if (error != NULL || trap != NULL)
    exit_with_error("failed to instantiate linking2", error, trap);

  // Register our new `linking2` instance with the linker
  error = wasmtime_linker_define_instance(linker, context, "linking2", strlen("linking2"), &linking2);
  if (error != NULL)
    exit_with_error("failed to link linking2", error, NULL);

  // Instantiate `linking1` with the linker now that `linking2` is defined
  wasmtime_instance_t linking1;
  error = wasmtime_linker_instantiate(linker, context, linking1_module, &linking1, &trap);
  if (error != NULL || trap != NULL)
    exit_with_error("failed to instantiate linking1", error, trap);

  // Lookup our `run` export function
  wasmtime_extern_t run;
  bool ok = wasmtime_instance_export_get(context, &linking1, "run", 3, &run);
  assert(ok);
  assert(run.kind == WASMTIME_EXTERN_FUNC);
  error = wasmtime_func_call(context, &run.of.func, NULL, 0, NULL, 0, &trap);
  if (error != NULL || trap != NULL)
    exit_with_error("failed to call run", error, trap);

  // Clean up after ourselves at this point
  wasmtime_linker_delete(linker);
  wasmtime_module_delete(linking1_module);
  wasmtime_module_delete(linking2_module);
  wasmtime_store_delete(store);
  wasm_engine_delete(engine);
  return 0;
}

static void read_wat_file(
  wasm_engine_t *engine,
  wasm_byte_vec_t *bytes,
  const char *filename
) {
  wasm_byte_vec_t wat;
  // Load our input file to parse it next
  FILE* file = fopen(filename, "r");
  if (!file) {
    printf("> Error loading file!\n");
    exit(1);
  }
  fseek(file, 0L, SEEK_END);
  size_t file_size = ftell(file);
  wasm_byte_vec_new_uninitialized(&wat, file_size);
  fseek(file, 0L, SEEK_SET);
  if (fread(wat.data, file_size, 1, file) != 1) {
    printf("> Error loading module!\n");
    exit(1);
  }
  fclose(file);

  // Parse the wat into the binary wasm format
  wasmtime_error_t *error = wasmtime_wat2wasm(wat.data, wat.size, bytes);
  if (error != NULL)
    exit_with_error("failed to parse wat", error, NULL);
  wasm_byte_vec_delete(&wat);
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
