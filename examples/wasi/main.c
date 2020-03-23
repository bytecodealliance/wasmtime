/*
Example of instantiating a WebAssembly which uses WASI imports.

You can compile and run this example on Linux with:

   cargo build --release -p wasmtime
   cc examples/wasi.c \
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
  wasm_module_t *module = wasm_module_new(store, &wasm);
  assert(module != NULL);
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
  wasi_instance_t *wasi = wasi_instance_new(store, wasi_config, &trap);
  if (wasi == NULL) {
    print_trap(trap);
    exit(1);
  }

  // Create import list for our module using wasi
  wasm_importtype_vec_t import_types;
  wasm_module_imports(module, &import_types);
  const wasm_extern_t **imports = calloc(import_types.size, sizeof(void*));
  assert(imports);
  for (int i = 0; i < import_types.size; i++) {
    const wasm_extern_t *binding = wasi_instance_bind_import(wasi, import_types.data[i]);
    if (binding != NULL) {
      imports[i] = binding;
    } else {
      printf("> Failed to satisfy import\n");
      exit(1);
    }
  }
  wasm_importtype_vec_delete(&import_types);

  // Instantiate the module
  wasm_instance_t *instance = wasm_instance_new(store, module, imports, &trap);
  if (instance == NULL) {
    print_trap(trap);
    exit(1);
  }
  free(imports);

  // Lookup our `_start` export function
  wasm_extern_vec_t externs;
  wasm_instance_exports(instance, &externs);
  wasm_exporttype_vec_t exports;
  wasm_module_exports(module, &exports);
  wasm_extern_t *start_extern = NULL;
  for (int i = 0; i < exports.size; i++) {
    const wasm_name_t *name = wasm_exporttype_name(exports.data[i]);
    if (strncmp(name->data, "_start", name->size) == 0)
      start_extern = externs.data[i];
  }
  assert(start_extern);
  wasm_func_t *start = wasm_extern_as_func(start_extern);
  assert(start != NULL);
  trap = wasm_func_call(start, NULL, NULL);
  if (trap != NULL) {
    print_trap(trap);
    exit(1);
  }

  // Clean up after ourselves at this point
  wasm_exporttype_vec_delete(&exports);
  wasm_extern_vec_delete(&externs);
  wasm_instance_delete(instance);
  wasm_module_delete(module);
  wasm_store_delete(store);
  wasm_engine_delete(engine);
  return 0;
}

static void print_trap(wasm_trap_t *trap) {
  assert(trap != NULL);
  wasm_message_t message;
  wasm_trap_message(trap, &message);
  fprintf(stderr, "failed to instantiate module %.*s\n", (int) message.size, message.data);
  wasm_byte_vec_delete(&message);
  wasm_trap_delete(trap);
}

