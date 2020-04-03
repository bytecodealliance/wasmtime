/*
Example of compiling, instantiating, and linking two WebAssembly modules
together.

You can compile and run this example on Linux with:

   cargo build --release -p wasmtime
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

static void print_trap(wasm_trap_t *trap);
static void read_wat_file(wasm_engine_t *engine, wasm_byte_vec_t *bytes, const char *file);

int main() {
  int ret = 0;
  // Set up our context
  wasm_engine_t *engine = wasm_engine_new();
  assert(engine != NULL);
  wasm_store_t *store = wasm_store_new(engine);
  assert(store != NULL);

  wasm_byte_vec_t linking1_wasm, linking2_wasm;
  read_wat_file(engine, &linking1_wasm, "examples/linking1.wat");
  read_wat_file(engine, &linking2_wasm, "examples/linking2.wat");

  // Compile our two modules
  wasm_module_t *linking1_module = wasm_module_new(store, &linking1_wasm);
  assert(linking1_module != NULL);
  wasm_module_t *linking2_module = wasm_module_new(store, &linking2_wasm);
  assert(linking2_module != NULL);
  wasm_byte_vec_delete(&linking1_wasm);
  wasm_byte_vec_delete(&linking2_wasm);

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
  if (wasi == NULL) {
    print_trap(trap);
    exit(1);
  }

  // Create our linker which will be linking our modules together, and then add
  // our WASI instance to it.
  wasmtime_linker_t *linker = wasmtime_linker_new(store);
  bool ok = wasmtime_linker_define_wasi(linker, wasi);
  assert(ok);

  // Instantiate `linking2` with our linker.
  wasm_instance_t *linking2 = wasmtime_linker_instantiate(linker, linking2_module, &trap);
  if (linking2 == NULL) {
    if (trap == NULL) {
      printf("> failed to link!\n");
    } else {
      print_trap(trap);
    }
    exit(1);
  }

  // Register our new `linking2` instance with the linker
  wasm_name_t linking2_name;
  linking2_name.data = "linking2";
  linking2_name.size = strlen(linking2_name.data);
  ok = wasmtime_linker_define_instance(linker, &linking2_name, linking2);
  assert(ok);

  // Instantiate `linking1` with the linker now that `linking2` is defined
  wasm_instance_t *linking1 = wasmtime_linker_instantiate(linker, linking1_module, &trap);
  if (linking1 == NULL) {
    if (trap == NULL) {
      printf("> failed to link!\n");
    } else {
      print_trap(trap);
    }
    exit(1);
  }

  // Lookup our `run` export function
  wasm_extern_vec_t linking1_externs;
  wasm_instance_exports(linking1, &linking1_externs);
  assert(linking1_externs.size == 1);
  wasm_func_t *run = wasm_extern_as_func(linking1_externs.data[0]);
  assert(run != NULL);
  trap = wasm_func_call(run, NULL, NULL);
  if (trap != NULL) {
    print_trap(trap);
    exit(1);
  }

  // Clean up after ourselves at this point
  wasm_extern_vec_delete(&linking1_externs);
  wasm_instance_delete(linking1);
  wasm_instance_delete(linking2);
  wasmtime_linker_delete(linker);
  wasm_module_delete(linking1_module);
  wasm_module_delete(linking2_module);
  wasm_store_delete(store);
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
  wasm_byte_vec_t error;
  if (wasmtime_wat2wasm(&wat, bytes, &error) == 0) {
    fprintf(stderr, "failed to parse wat %.*s\n", (int) error.size, error.data);
    exit(1);
  }
  wasm_byte_vec_delete(&wat);
}

static void print_trap(wasm_trap_t *trap) {
  assert(trap != NULL);
  wasm_message_t message;
  wasm_trap_message(trap, &message);
  fprintf(stderr, "failed to instantiate module %.*s\n", (int) message.size, message.data);
  wasm_byte_vec_delete(&message);
  wasm_trap_delete(trap);
}
