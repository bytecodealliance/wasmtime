/**
 * The component model
 *
 * TODO: Write some more documentation here like in the Rust API.
 *
 */

#ifndef WASMTIME_COMPONENT_H
#define WASMTIME_COMPONENT_H

#include <wasm.h>
#include <wasmtime/config.h>
#include <wasmtime/error.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * \brief Whether or not to enable support for the component model in
 * Wasmtime.
 *
 * For more information see the Rust documentation at
 * https://docs.wasmtime.dev/api/wasmtime/struct.Config.html#method.wasm_component_model
 */
WASMTIME_CONFIG_PROP(void, component_model, bool)

// The tag part of wasmtime_component_val_t that specifies what variant is
// populated in wasmtime_component_val_payload_t.
typedef uint8_t wasmtime_component_kind_t;

#define WASMTIME_COMPONENT_KIND_BOOL 0
#define WASMTIME_COMPONENT_KIND_S8 1
#define WASMTIME_COMPONENT_KIND_U8 2
#define WASMTIME_COMPONENT_KIND_S16 3
#define WASMTIME_COMPONENT_KIND_U16 4
#define WASMTIME_COMPONENT_KIND_S32 5
#define WASMTIME_COMPONENT_KIND_U32 6
#define WASMTIME_COMPONENT_KIND_S64 7
#define WASMTIME_COMPONENT_KIND_U64 8
#define WASMTIME_COMPONENT_KIND_F32 9
#define WASMTIME_COMPONENT_KIND_F64 10
#define WASMTIME_COMPONENT_KIND_CHAR 11
#define WASMTIME_COMPONENT_KIND_STRING 12
#define WASMTIME_COMPONENT_KIND_LIST 13
#define WASMTIME_COMPONENT_KIND_RECORD 14
#define WASMTIME_COMPONENT_KIND_TUPLE 15
#define WASMTIME_COMPONENT_KIND_VARIANT 16
#define WASMTIME_COMPONENT_KIND_ENUM 17
#define WASMTIME_COMPONENT_KIND_OPTION 18
#define WASMTIME_COMPONENT_KIND_RESULT 19
#define WASMTIME_COMPONENT_KIND_FLAGS 20

typedef struct wasmtime_component_val_t wasmtime_component_val_t;
typedef struct wasmtime_component_val_record_field_t wasmtime_component_val_record_field_t;
typedef struct wasmtime_component_type_t wasmtime_component_type_t;
typedef struct wasmtime_component_type_field_t wasmtime_component_type_field_t;

#define WASMTIME_COMPONENT_DECLARE_VEC(name, element)                          \
  typedef struct wasmtime_component_##name##_t {                               \
    size_t size;                                                               \
    element *data;                                                             \
  } wasmtime_component_##name##_t;                                             \
                                                                               \
  WASM_API_EXTERN void wasmtime_component_##name##_new_empty(                  \
      wasmtime_component_##name##_t *out);                                     \
  WASM_API_EXTERN void wasmtime_component_##name##_new_uninitialized(          \
      wasmtime_component_##name##_t *out, size_t);                             \
  WASM_API_EXTERN void wasmtime_component_##name##_copy(                       \
      wasmtime_component_##name##_t *out,                                      \
      const wasmtime_component_##name##_t *);                                  \
  WASM_API_EXTERN void wasmtime_component_##name##_delete(                     \
      wasmtime_component_##name##_t *);

// in C, an array type needs a complete element type, we need to defer xxx_new
#define WASMTIME_COMPONENT_DECLARE_VEC_NEW(name, element)                      \
  WASM_API_EXTERN void wasmtime_component_##name##_new(                        \
      wasmtime_component_##name##_t *out, size_t, element const[]);

/// \brief A vector of values.
WASMTIME_COMPONENT_DECLARE_VEC(val_vec, wasmtime_component_val_t);

/// \brief A tuple of named fields.
WASMTIME_COMPONENT_DECLARE_VEC(val_record, wasmtime_component_val_record_field_t);

/// \brief A variable sized bitset.
WASMTIME_COMPONENT_DECLARE_VEC(val_flags, uint32_t);
WASMTIME_COMPONENT_DECLARE_VEC_NEW(val_flags, uint32_t);

/// \brief A vector of types
WASMTIME_COMPONENT_DECLARE_VEC(type_vec, wasmtime_component_type_t);

/// \brief A vector of field types
WASMTIME_COMPONENT_DECLARE_VEC(type_field_vec, wasmtime_component_type_field_t);

/// \brief A vector of strings
WASMTIME_COMPONENT_DECLARE_VEC(string_vec, wasm_name_t);
WASMTIME_COMPONENT_DECLARE_VEC_NEW(string_vec, wasm_name_t);

#undef WASMTIME_COMPONENT_DECLARE_VEC

// A variant contains the discriminant index and an optional value that is held.
typedef struct wasmtime_component_val_variant_t {
  uint32_t discriminant;
  wasmtime_component_val_t *val;
} wasmtime_component_val_variant_t;

// A result is an either type holding a value and a bit if is it an ok or error
// variant.
typedef struct wasmtime_component_val_result_t {
  wasmtime_component_val_t *val;
  bool error;
} wasmtime_component_val_result_t;

// Which value within an enumeration is selected.
typedef struct wasmtime_component_val_enum_t {
  uint32_t discriminant;
} wasmtime_component_val_enum_t;

typedef union wasmtime_component_val_payload_t {
  bool boolean;
  int8_t s8;
  uint8_t u8;
  int16_t s16;
  uint16_t u16;
  int32_t s32;
  uint32_t u32;
  int64_t s64;
  uint64_t u64;
  float f32;
  double f64;
  uint8_t character;
  wasm_name_t string;
  wasmtime_component_val_vec_t list;
  wasmtime_component_val_record_t record;
  wasmtime_component_val_vec_t tuple;
  wasmtime_component_val_variant_t variant;
  wasmtime_component_val_enum_t enumeration;
  wasmtime_component_val_t *option;
  wasmtime_component_val_result_t result;
  wasmtime_component_val_flags_t flags;
} wasmtime_component_val_payload_t;

// The tagged union for a value within the component model.
typedef struct wasmtime_component_val_t {
  wasmtime_component_kind_t kind;
  wasmtime_component_val_payload_t payload;
} wasmtime_component_val_t;

WASMTIME_COMPONENT_DECLARE_VEC_NEW(val_vec, wasmtime_component_val_t);

// A record is a series of named fields, which are values with a string name.
typedef struct wasmtime_component_val_record_field_t {
  wasm_name_t name;
  wasmtime_component_val_t val;
} wasmtime_component_val_record_field_t;

WASMTIME_COMPONENT_DECLARE_VEC_NEW(val_record, wasmtime_component_val_record_field_t);

// Set a value within this bitset.
//
// If this bit set is too small to hold a value at `index` it will be resized.
void wasmtime_component_val_flags_set(wasmtime_component_val_flags_t *flags, uint32_t index, bool enabled);

// Test if this bitset holds a value at `index`.
bool wasmtime_component_val_flags_test(const wasmtime_component_val_flags_t* flags, uint32_t index);

wasmtime_component_val_t* wasmtime_component_val_new();

void wasmtime_component_val_delete(wasmtime_component_val_t* val);

typedef struct wasmtime_component_type_field_t {
  wasm_name_t name;
  wasmtime_component_type_t* ty;
} wasmtime_component_type_field_t;

WASMTIME_COMPONENT_DECLARE_VEC_NEW(type_field_vec, wasmtime_component_type_field_t);

typedef struct wasmtime_component_type_result_t {
  wasmtime_component_type_t* ok_ty;
  wasmtime_component_type_t* err_ty;
} wasmtime_component_type_result_t;

typedef union wasmtime_component_type_payload_t
{
  wasmtime_component_type_t* list;
  wasmtime_component_type_field_vec_t record;
  wasmtime_component_type_vec_t tuple;
  wasmtime_component_type_field_vec_t variant;
  wasmtime_component_string_vec_t enumeration;
  wasmtime_component_type_t* option;
  wasmtime_component_type_result_t result;
  wasmtime_component_string_vec_t flags;
} wasmtime_component_type_payload_t;

typedef struct wasmtime_component_type_t {
  wasmtime_component_kind_t kind;
  wasmtime_component_type_payload_t payload;
} wasmtime_component_type_t;

WASMTIME_COMPONENT_DECLARE_VEC_NEW(type_vec, wasmtime_component_type_t);

#undef WASMTIME_COMPONENT_DECLARE_VEC_NEW

wasmtime_component_type_t* wasmtime_component_type_new();

void wasmtime_component_type_delete(wasmtime_component_type_t* ty);

typedef struct wasmtime_component_t wasmtime_component_t;

wasmtime_error_t *
wasmtime_component_from_binary(const wasm_engine_t *engine, const uint8_t *buf, size_t len,
                               wasmtime_component_t **component_out);

void wasmtime_component_delete(wasmtime_component_t *component);

typedef struct wasmtime_component_linker_t wasmtime_component_linker_t;

wasmtime_component_linker_t *wasmtime_component_linker_new(const wasm_engine_t *engine);

void wasmtime_component_linker_delete(wasmtime_component_linker_t *linker);

typedef struct wasmtime_component_instance_t wasmtime_component_instance_t;

// declaration from store.h
typedef struct wasmtime_context wasmtime_context_t;

typedef wasm_trap_t *(*wasmtime_component_func_callback_t)(
    void *env, wasmtime_context_t *context, const wasmtime_component_val_t *args,
    size_t nargs, wasmtime_component_val_t *results, size_t nresults);

wasmtime_error_t *wasmtime_component_linker_define_func(
    wasmtime_component_linker_t *linker, const char *path, size_t path_len,
    const char *name, size_t name_len,
    wasmtime_component_type_t* params_types_buf, size_t params_types_len,
    wasmtime_component_type_t* outputs_types_buf, size_t outputs_types_len,
    wasmtime_component_func_callback_t cb, void *data, void (*finalizer)(void *));

wasmtime_error_t *wasmtime_component_linker_build(wasmtime_component_linker_t *linker);

wasmtime_error_t *wasmtime_component_linker_instantiate(
    const wasmtime_component_linker_t *linker, wasmtime_context_t *context,
    const wasmtime_component_t *component, wasmtime_component_instance_t **instance_out);

typedef struct wasmtime_component_func_t wasmtime_component_func_t;

bool wasmtime_component_instance_get_func(
    const wasmtime_component_instance_t *instance, wasmtime_context_t *context,
    const char *name, size_t name_len, wasmtime_component_func_t **item_out);

wasmtime_error_t *wasmtime_component_func_call(
    const wasmtime_component_func_t *func, wasmtime_context_t *context,
    const wasmtime_component_val_t *params, size_t params_len,
    wasmtime_component_val_t *results, size_t results_len,
    wasm_trap_t **trap_out);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_COMPONENT_H
