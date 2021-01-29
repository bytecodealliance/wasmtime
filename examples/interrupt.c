/*
Example of instantiating of the WebAssembly module and invoking its exported
function.

You can compile and run this example on Linux with:

   cargo build --release -p wasmtime-c-api
   cc examples/interrupt.c \
       -I crates/c-api/include \
       -I crates/c-api/wasm-c-api/include \
       target/release/libwasmtime.a \
       -lpthread -ldl -lm \
       -o interrupt
   ./interrupt

Note that on Windows and macOS the command will be similar, but you'll need
to tweak the `-lpthread` and such annotations as well as the name of the
`libwasmtime.a` file on Windows.
*/

#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <wasm.h>
#include <wasmtime.h>

#ifdef _WIN32
static void spawn_interrupt(wasmtime_interrupt_handle_t *handle) {
  wasmtime_interrupt_handle_interrupt(handle);
  wasmtime_interrupt_handle_delete(handle);
}
#else
#include <pthread.h>
#include <time.h>

static void* helper(void *_handle) {
  wasmtime_interrupt_handle_t *handle = _handle;
  struct timespec sleep_dur;
  sleep_dur.tv_sec = 1;
  sleep_dur.tv_nsec = 0;
  nanosleep(&sleep_dur, NULL);
  printf("Sending an interrupt\n");
  wasmtime_interrupt_handle_interrupt(handle);
  wasmtime_interrupt_handle_delete(handle);
  return 0;
}

static void spawn_interrupt(wasmtime_interrupt_handle_t *handle) {
  pthread_t child;
  int rc = pthread_create(&child, NULL, helper, handle);
  assert(rc == 0);
}
#endif

static void exit_with_error(const char *message, wasmtime_error_t *error, wasm_trap_t *trap);

int main() {
  // Create a `wasm_store_t` with interrupts enabled
  wasm_config_t *config = wasm_config_new();
  assert(config != NULL);
  wasmtime_config_interruptable_set(config, true);
  wasm_engine_t *engine = wasm_engine_new_with_config(config);
  assert(engine != NULL);
  wasm_store_t *store = wasm_store_new(engine);
  assert(store != NULL);

  // Create our interrupt handle we'll use later
  wasmtime_interrupt_handle_t *handle = wasmtime_interrupt_handle_new(store);
  assert(handle != NULL);

  // Read our input file, which in this case is a wasm text file.
  FILE* file = fopen("examples/interrupt.wat", "r");
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
  wasm_module_t *module = NULL;
  wasm_trap_t *trap = NULL;
  wasm_instance_t *instance = NULL;
  wasm_extern_vec_t imports = WASM_EMPTY_VEC;
  error = wasmtime_module_new(engine, &wasm, &module);
  wasm_byte_vec_delete(&wasm);
  if (error != NULL)
    exit_with_error("failed to compile module", error, NULL);
  error = wasmtime_instance_new(store, module, &imports, &instance, &trap);
  if (instance == NULL)
    exit_with_error("failed to instantiate", error, trap);

  // Lookup our `run` export function
  wasm_extern_vec_t externs;
  wasm_instance_exports(instance, &externs);
  assert(externs.size == 1);
  wasm_func_t *run = wasm_extern_as_func(externs.data[0]);
  assert(run != NULL);

  // Spawn a thread to send us an interrupt after a period of time.
  spawn_interrupt(handle);

  // And call it!
  printf("Entering infinite loop...\n");
  wasm_val_vec_t args_vec = WASM_EMPTY_VEC;
  wasm_val_vec_t results_vec = WASM_EMPTY_VEC;
  error = wasmtime_func_call(run, &args_vec, &results_vec, &trap);
  assert(error == NULL);
  assert(trap != NULL);
  printf("Got a trap!...\n");

  // `trap` can be inspected here to see the trap message has an interrupt in it

  wasm_trap_delete(trap);
  wasm_extern_vec_delete(&externs);
  wasm_instance_delete(instance);
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
