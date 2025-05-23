/// \file wasmtime/component/val.h

#ifndef WASMTIME_COMPONENT_VAL_H
#define WASMTIME_COMPONENT_VAL_H

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_COMPONENT_MODEL

#ifdef __cplusplus
extern "C" {
#endif

/// \brief Discriminant used in #wasmtime_component_val_t::kind
typedef uint8_t wasmtime_component_valkind_t;

/// \brief Value of #wasmtime_component_valkind_t meaning that
/// #wasmtime_component_val_t is a bool
#define WASMTIME_COMPONENT_BOOL 0
/// \brief Value of #wasmtime_component_valkind_t meaning that
/// #wasmtime_component_val_t is a s8
#define WASMTIME_COMPONENT_S8 1
/// \brief Value of #wasmtime_component_valkind_t meaning that
/// #wasmtime_component_val_t is a u8
#define WASMTIME_COMPONENT_U8 2
/// \brief Value of #wasmtime_component_valkind_t meaning that
/// #wasmtime_component_val_t is a s16
#define WASMTIME_COMPONENT_S16 3
/// \brief Value of #wasmtime_component_valkind_t meaning that
/// #wasmtime_component_val_t is a u16
#define WASMTIME_COMPONENT_U16 4
/// \brief Value of #wasmtime_component_valkind_t meaning that
/// #wasmtime_component_val_t is a s32
#define WASMTIME_COMPONENT_S32 5
/// \brief Value of #wasmtime_component_valkind_t meaning that
/// #wasmtime_component_val_t is a u32
#define WASMTIME_COMPONENT_U32 6
/// \brief Value of #wasmtime_component_valkind_t meaning that
/// #wasmtime_component_val_t is a s64
#define WASMTIME_COMPONENT_S64 7
/// \brief Value of #wasmtime_component_valkind_t meaning that
/// #wasmtime_component_val_t is a u64
#define WASMTIME_COMPONENT_U64 8
/// \brief Value of #wasmtime_component_valkind_t meaning that
/// #wasmtime_component_val_t is a f32
#define WASMTIME_COMPONENT_F32 9
/// \brief Value of #wasmtime_component_valkind_t meaning that
/// #wasmtime_component_val_t is a f64
#define WASMTIME_COMPONENT_F64 10
/// \brief Value of #wasmtime_component_valkind_t meaning that
/// #wasmtime_component_val_t is a char
#define WASMTIME_COMPONENT_CHAR 11
/// \brief Value of #wasmtime_component_valkind_t meaning that
/// #wasmtime_component_val_t is a string
#define WASMTIME_COMPONENT_STRING 12
/// \brief Value of #wasmtime_component_valkind_t meaning that
/// #wasmtime_component_val_t is a list
#define WASMTIME_COMPONENT_LIST 13
/// \brief Value of #wasmtime_component_valkind_t meaning that
/// #wasmtime_component_val_t is a record
#define WASMTIME_COMPONENT_RECORD 14

struct wasmtime_component_val;
struct wasmtime_component_valrecord_entry;

#define DECLARE_VEC(name, type)                                                \
  /** \brief A vec of a type */                                                \
  typedef struct name {                                                        \
    /** Length of the vec */                                                   \
    size_t size;                                                               \
    /** Pointer to the elements */                                             \
    type *data;                                                                \
  } name##_t;                                                                  \
                                                                               \
  /** \brief Create vec from \p ptr and \p size */                             \
  WASM_API_EXTERN void name##_new(name##_t *out, size_t size, type *ptr);      \
  /** \brief Create an empty vec */                                            \
  WASM_API_EXTERN void name##_new_empty(name##_t *out);                        \
  /** \brief Create a vec with length \p size */                               \
  WASM_API_EXTERN void name##_new_uninit(name##_t *out, size_t size);          \
  /** \brief Copy \p src to \p dst */                                          \
  WASM_API_EXTERN void name##_copy(name##_t *dst, const name##_t *src);        \
  /** \brief Delete \p value */                                                \
  WASM_API_EXTERN void name##_delete(name##_t *value);

DECLARE_VEC(wasmtime_component_vallist, struct wasmtime_component_val)
DECLARE_VEC(wasmtime_component_valrecord,
            struct wasmtime_component_valrecord_entry)

#undef DECLARE_VEC

/// \brief Represents possible runtime values which a component function can
/// either consume or produce
typedef union {
  /// Field used if #wasmtime_component_val_t::kind is #WASMTIME_COMPONENT_BOOL
  bool boolean;
  /// Field used if #wasmtime_component_val_t::kind is #WASMTIME_COMPONENT_S8
  int8_t s8;
  /// Field used if #wasmtime_component_val_t::kind is #WASMTIME_COMPONENT_U8
  uint8_t u8;
  /// Field used if #wasmtime_component_val_t::kind is #WASMTIME_COMPONENT_S16
  int16_t s16;
  /// Field used if #wasmtime_component_val_t::kind is #WASMTIME_COMPONENT_U16
  uint16_t u16;
  /// Field used if #wasmtime_component_val_t::kind is #WASMTIME_COMPONENT_S32
  int32_t s32;
  /// Field used if #wasmtime_component_val_t::kind is #WASMTIME_COMPONENT_U32
  uint32_t u32;
  /// Field used if #wasmtime_component_val_t::kind is #WASMTIME_COMPONENT_S64
  int64_t s64;
  /// Field used if #wasmtime_component_val_t::kind is #WASMTIME_COMPONENT_U64
  uint64_t u64;
  /// Field used if #wasmtime_component_val_t::kind is #WASMTIME_COMPONENT_F32
  float32_t f32;
  /// Field used if #wasmtime_component_val_t::kind is #WASMTIME_COMPONENT_F64
  float64_t f64;
  /// Field used if #wasmtime_component_val_t::kind is #WASMTIME_COMPONENT_CHAR
  uint32_t character;
  /// Field used if #wasmtime_component_val_t::kind is
  /// #WASMTIME_COMPONENT_STRING
  wasm_name_t string;
  /// Field used if #wasmtime_component_val_t::kind is #WASMTIME_COMPONENT_LIST
  wasmtime_component_vallist_t list;
  /// Field used if #wasmtime_component_val_t::kind is
  /// #WASMTIME_COMPONENT_RECORD
  wasmtime_component_valrecord_t record;
} wasmtime_component_valunion_t;

/// \brief Represents possible runtime values which a component function can
/// either consume or produce
typedef struct wasmtime_component_val {
  /// The type discriminant
  wasmtime_component_valkind_t kind;
  /// Value of type \ref kind
  wasmtime_component_valunion_t of;
} wasmtime_component_val_t;

/// \brief A pair of a name and a value that represents one entry in a value
/// with kind #WASMTIME_COMPONENT_RECORD
typedef struct wasmtime_component_valrecord_entry {
  /// The name of this entry
  wasm_name_t name;
  /// The value of this entry
  wasmtime_component_val_t val;
} wasmtime_component_valrecord_entry_t;

/// \brief Calls the destructor on \p value deallocating any owned memory
WASM_API_EXTERN void
wasmtime_component_val_delete(wasmtime_component_val_t *value);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_FEATURE_COMPONENT_MODEL

#endif // WASMTIME_COMPONENT_VAL_H
