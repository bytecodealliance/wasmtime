// WASI C API

#ifndef WASI_H
#define WASI_H

#include "wasm.h"

#ifndef WASI_API_EXTERN
#ifdef _WIN32
#define WASI_API_EXTERN __declspec(dllimport)
#else
#define WASI_API_EXTERN
#endif
#endif

#ifdef __cplusplus
extern "C" {
#endif

#define own

#define WASI_DECLARE_OWN(name) \
  typedef struct wasi_##name##_t wasi_##name##_t; \
  WASI_API_EXTERN void wasi_##name##_delete(own wasi_##name##_t*);

// WASI config

WASI_DECLARE_OWN(config)

WASI_API_EXTERN own wasi_config_t* wasi_config_new();

WASI_API_EXTERN void wasi_config_set_argv(wasi_config_t* config, int argc, const char* argv[]);
WASI_API_EXTERN void wasi_config_inherit_argv(wasi_config_t* config);

WASI_API_EXTERN void wasi_config_set_env(wasi_config_t* config, int envc, const char* names[], const char* values[]);
WASI_API_EXTERN void wasi_config_inherit_env(wasi_config_t* config);

WASI_API_EXTERN bool wasi_config_set_stdin_file(wasi_config_t* config, const char* path);
WASI_API_EXTERN void wasi_config_inherit_stdin(wasi_config_t* config);

WASI_API_EXTERN bool wasi_config_set_stdout_file(wasi_config_t* config, const char* path);
WASI_API_EXTERN void wasi_config_inherit_stdout(wasi_config_t* config);

WASI_API_EXTERN bool wasi_config_set_stderr_file(wasi_config_t* config, const char* path);
WASI_API_EXTERN void wasi_config_inherit_stderr(wasi_config_t* config);

WASI_API_EXTERN bool wasi_config_preopen_dir(wasi_config_t* config, const char* path, const char* guest_path);

// WASI instance

WASI_DECLARE_OWN(instance)

WASI_API_EXTERN own wasi_instance_t* wasi_instance_new(
  wasm_store_t* store,
  own wasi_config_t* config,
  own wasm_trap_t** trap
);

WASI_API_EXTERN const wasm_extern_t* wasi_instance_bind_import(
  const wasi_instance_t* instance,
  const wasm_importtype_t* import
);

#undef own

#ifdef __cplusplus
}  // extern "C"
#endif

#endif  // #ifdef WASI_H