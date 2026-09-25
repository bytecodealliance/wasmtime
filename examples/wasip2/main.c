/*
Example of instantiating a WebAssembly component which uses WASIp2 imports.

You can compile and run this example on Linux with:

   cargo build --target wasm32-wasip2 -p example-wasi-wasm
   cargo build --release -p wasmtime-c-api
   cc examples/wasip2/main.c \
       -I crates/c-api/include \
       target/release/libwasmtime.a \
       -lpthread -ldl -lm \
       -o wasip2
   ./wasip2

Note that on Windows and macOS the command will be similar, but you'll need
to tweak the `-lpthread` and such annotations.

You can also build and run this example using cmake:

  cargo build --target wasm32-wasip2 -p example-wasi-wasm
  cmake -B build -S examples
  cmake --build build --target wasmtime-wasip2
  ./build/wasmtime-wasip2
*/

#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <wasi.h>
#include <wasm.h>
#include <wasmtime.h>

static void exit_with_error(const char *message, wasmtime_error_t *error,
                            wasm_trap_t *trap);

int main() {
  // Set up our context
  wasm_engine_t *engine = wasm_engine_new();
  assert(engine != NULL);
  wasmtime_store_t *store = wasmtime_store_new(engine, NULL, NULL);
  assert(store != NULL);
  wasmtime_context_t *context = wasmtime_store_context(store);

  // Create a component linker with WASIp2 functions defined
  wasmtime_component_linker_t *linker = wasmtime_component_linker_new(engine);
  wasmtime_error_t *error = wasmtime_component_linker_add_wasip2(linker);
  if (error != NULL)
    exit_with_error("failed to link wasi", error, NULL);

  wasm_byte_vec_t wasm;
  // Load our input file to parse it next
  FILE *file = fopen("target/wasm32-wasip2/debug/wasi.wasm", "rb");
  if (!file) {
    printf("> Error loading file!\n");
    exit(1);
  }
  fseek(file, 0L, SEEK_END);
  size_t file_size = ftell(file);
  wasm_byte_vec_new_uninitialized(&wasm, file_size);
  fseek(file, 0L, SEEK_SET);
  if (fread(wasm.data, file_size, 1, file) != 1) {
    printf("> Error loading component!\n");
    exit(1);
  }
  fclose(file);

  // Compile our component
  wasmtime_component_t *component = NULL;
  error = wasmtime_component_new(engine, (uint8_t *)wasm.data, wasm.size,
                                 &component);
  if (!component)
    exit_with_error("failed to compile component", error, NULL);
  wasm_byte_vec_delete(&wasm);

  // Create a WASI context and put it in the store; all instances in the store
  // share this context. `wasi_config_t` provides a number of ways to configure
  // what the target program will have access to.
  wasi_config_t *wasi_config = wasi_config_new();
  assert(wasi_config);
  wasi_config_inherit_argv(wasi_config);
  wasi_config_inherit_env(wasi_config);
  wasi_config_inherit_stdin(wasi_config);
  wasi_config_inherit_stdout(wasi_config);
  wasi_config_inherit_stderr(wasi_config);
  error = wasmtime_context_set_wasi(context, wasi_config);
  if (error != NULL)
    exit_with_error("failed to instantiate WASI", error, NULL);

  // Instantiate the component
  wasmtime_component_instance_t instance;
  error = wasmtime_component_linker_instantiate(linker, context, component,
                                                &instance);
  if (error != NULL)
    exit_with_error("failed to instantiate component", error, NULL);

  // Look up the `run` function in the exported `wasi:cli/run@0.2.0` interface.
  // First look up the interface itself, then the function within it.
  const char *interface_name = "wasi:cli/run@0.2.0";
  wasmtime_component_export_index_t *interface_idx =
      wasmtime_component_instance_get_export_index(
          &instance, context, NULL, interface_name, strlen(interface_name));
  if (interface_idx == NULL) {
    fprintf(stderr, "error: cannot find `%s` interface\n", interface_name);
    exit(1);
  }
  wasmtime_component_export_index_t *func_idx =
      wasmtime_component_instance_get_export_index(&instance, context,
                                                   interface_idx, "run", 3);
  if (func_idx == NULL) {
    fprintf(stderr, "error: cannot find `run` function in `%s` interface\n",
            interface_name);
    exit(1);
  }
  wasmtime_component_func_t func;
  bool found =
      wasmtime_component_instance_get_func(&instance, context, func_idx, &func);
  assert(found);
  wasmtime_component_export_index_delete(func_idx);
  wasmtime_component_export_index_delete(interface_idx);

  // Run it. The `run` function takes no arguments and returns a single
  // `result<(), ()>` value indicating whether the program succeeded.
  wasmtime_component_val_t result;
  error = wasmtime_component_func_call(&func, context, NULL, 0, &result, 1);
  if (error != NULL)
    exit_with_error("error calling `run`", error, NULL);
  assert(result.kind == WASMTIME_COMPONENT_RESULT);
  bool ok = result.of.result.is_ok;
  wasmtime_component_val_delete(&result);
  if (!ok) {
    fprintf(stderr, "error: program returned an error\n");
    exit(1);
  }

  // Clean up after ourselves at this point
  wasmtime_component_linker_delete(linker);
  wasmtime_component_delete(component);
  wasmtime_store_delete(store);
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
