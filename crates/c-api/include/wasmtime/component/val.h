#ifndef WASMTIME_COMPONENT_VAL_H
#define WASMTIME_COMPONENT_VAL_H

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_COMPONENT_MODEL

#ifdef __cplusplus
extern "C" {
#endif

typedef uint8_t wasmtime_component_valkind_t;

#define WASMTIME_COMPONENT_BOOL 0
#define WASMTIME_COMPONENT_S8 1
#define WASMTIME_COMPONENT_U8 2
#define WASMTIME_COMPONENT_S16 3
#define WASMTIME_COMPONENT_U16 4
#define WASMTIME_COMPONENT_S32 5
#define WASMTIME_COMPONENT_U32 6
#define WASMTIME_COMPONENT_S64 7
#define WASMTIME_COMPONENT_U64 8
#define WASMTIME_COMPONENT_F32 9
#define WASMTIME_COMPONENT_F64 10
#define WASMTIME_COMPONENT_CHAR 11
#define WASMTIME_COMPONENT_STRING 12
#define WASMTIME_COMPONENT_LIST 13
#define WASMTIME_COMPONENT_RECORD 14

struct wasmtime_component_val;

typedef struct {
  struct wasmtime_component_val *ptr;
  size_t len;
} wasmtime_component_vallist_t;

struct wasmtime_component_valrecord_entry;

typedef struct {
  struct wasmtime_component_valrecord_entry *ptr;
  size_t len;
} wasmtime_component_valrecord_t;

typedef union {
  bool boolean;
  int8_t s8;
  uint8_t u8;
  int16_t s16;
  uint16_t u16;
  int32_t s32;
  uint32_t u32;
  int64_t s64;
  uint64_t u64;
  float32_t f32;
  float64_t f64;
  uint32_t character;
  wasm_name_t string;
  wasmtime_component_vallist_t list;
  wasmtime_component_valrecord_t record;
} wasmtime_component_valunion_t;

typedef struct wasmtime_component_val {
  wasmtime_component_valkind_t kind;
  wasmtime_component_valunion_t of;
} wasmtime_component_val_t;

typedef struct wasmtime_component_valrecord_entry {
  wasm_name_t name;
  wasmtime_component_val_t val;
} wasmtime_component_valrecord_entry_t;

WASM_API_EXTERN void
wasmtime_component_val_delete(wasmtime_component_val_t *value);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_FEATURE_COMPONENT_MODEL

#endif // WASMTIME_COMPONENT_VAL_H
