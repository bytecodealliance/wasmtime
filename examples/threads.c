/*
Example of instantiating of the WebAssembly module and invoking its exported
function in a separate thread.

You can build using cmake:

mkdir build && cd build && cmake .. && cmake --build . --target wasmtime-threads
*/

#ifndef _WIN32

#include <inttypes.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <wasm.h>
#include <wasmtime.h>

#define own

static void exit_with_error(const char *message, wasmtime_error_t *error,
                            wasm_trap_t *trap);

const int N_THREADS = 10;
const int N_REPS = 3;

#if defined(__linux__)
#define _GNU_SOURCE
#include <sys/syscall.h>
uint64_t get_thread_id() { return (uint64_t)syscall(SYS_gettid); }

#elif defined(__APPLE__)
#include <pthread.h>
uint64_t get_thread_id() {
  uint64_t tid;
  pthread_threadid_np(NULL, &tid);
  return tid;
}

#endif

// A function to be called from Wasm code.
own wasm_trap_t *callback(const wasm_val_vec_t *args, wasm_val_vec_t *results) {
  printf("> Thread %lu running\n", (uint64_t)get_thread_id());
  return NULL;
}

typedef struct {
  wasm_engine_t *engine;
  wasm_shared_module_t *module;
  int id;
} thread_args;

void *run(void *args_abs) {
  thread_args *args = (thread_args *)args_abs;

  // Rereate store and module.
  own wasm_store_t *store = wasm_store_new(args->engine);
  own wasm_module_t *module = wasm_module_obtain(store, args->module);

  // Run the example N times.
  for (int i = 0; i < N_REPS; ++i) {
    usleep(100000);

    // Create imports.
    own wasm_functype_t *func_type = wasm_functype_new_0_0();
    own wasm_func_t *func = wasm_func_new(store, func_type, callback);
    wasm_functype_delete(func_type);

    // Instantiate.
    wasm_extern_t *imports[] = {
        wasm_func_as_extern(func),
    };
    wasm_extern_vec_t imports_vec = WASM_ARRAY_VEC(imports);
    own wasm_instance_t *instance =
        wasm_instance_new(store, module, &imports_vec, NULL);
    if (!instance) {
      printf("> Error instantiating module!\n");
      return NULL;
    }

    wasm_func_delete(func);

    // Extract export.
    own wasm_extern_vec_t exports;
    wasm_instance_exports(instance, &exports);
    if (exports.size == 0) {
      printf("> Error accessing exports!\n");
      return NULL;
    }
    const wasm_func_t *run_func = wasm_extern_as_func(exports.data[0]);
    if (run_func == NULL) {
      printf("> Error accessing export!\n");
      return NULL;
    }

    wasm_instance_delete(instance);

    // Call.
    wasm_val_vec_t args_vec = WASM_EMPTY_VEC;
    wasm_val_vec_t results_vec = WASM_EMPTY_VEC;
    if (wasm_func_call(run_func, &args_vec, &results_vec)) {
      printf("> Error calling function!\n");
      return NULL;
    }

    wasm_extern_vec_delete(&exports);
  }

  wasm_module_delete(module);
  wasm_store_delete(store);

  free(args_abs);

  return NULL;
}

int main(int argc, const char *argv[]) {
  // Initialize.
  wasm_engine_t *engine = wasm_engine_new();

  // Load our input file to parse it next
  FILE *file = fopen("examples/threads.wat", "r");
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

  // Compile and share.
  own wasm_store_t *store = wasm_store_new(engine);
  own wasm_module_t *module = wasm_module_new(store, &binary);
  if (!module) {
    printf("> Error compiling module!\n");
    return 1;
  }

  wasm_byte_vec_delete(&binary);

  own wasm_shared_module_t *shared = wasm_module_share(module);

  wasm_module_delete(module);
  wasm_store_delete(store);

  // Spawn threads.
  pthread_t threads[N_THREADS];
  for (int i = 0; i < N_THREADS; i++) {
    thread_args *args = malloc(sizeof(thread_args));
    args->engine = engine;
    args->module = shared;
    printf("Initializing thread %d...\n", i);

    // Guarantee at least 2MB of stack to allow running Cranelift in debug mode
    // on CI.
    pthread_attr_t attrs;
    pthread_attr_init(&attrs);
    pthread_attr_setstacksize(&attrs, 2 << 20);
    pthread_create(&threads[i], &attrs, &run, args);
    pthread_attr_destroy(&attrs);
  }

  for (int i = 0; i < N_THREADS; i++) {
    printf("Waiting for thread: %d\n", i);
    pthread_join(threads[i], NULL);
  }

  wasm_shared_module_delete(shared);
  wasm_engine_delete(engine);

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

#else
// TODO implement example for Windows
int main(int argc, const char *argv[]) { return 0; }
#endif // _WIN32
