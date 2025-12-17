/// \file wasmtime/component/types/val.h

#ifndef WASMTIME_COMPONENT_TYPES_VAL_H
#define WASMTIME_COMPONENT_TYPES_VAL_H

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_COMPONENT_MODEL

#include <stdint.h>
#include <wasmtime/component/types/resource.h>

#ifdef __cplusplus
extern "C" {
#endif

struct wasmtime_component_valtype_t;

// ----------- lists -----------------------------------------------------------

/// \brief Opaque type representing a component list type.
typedef struct wasmtime_component_list_type wasmtime_component_list_type_t;

/// \brief Clones a component list type.
///
/// The returned pointer must be deallocated with
/// `wasmtime_component_list_type_delete`.
WASM_API_EXTERN wasmtime_component_list_type_t *
wasmtime_component_list_type_clone(const wasmtime_component_list_type_t *ty);

/// \brief Compares two component list types for equality.
WASM_API_EXTERN bool
wasmtime_component_list_type_equal(const wasmtime_component_list_type_t *a,
                                   const wasmtime_component_list_type_t *b);

/// \brief Deallocates a component list type.
WASM_API_EXTERN void
wasmtime_component_list_type_delete(wasmtime_component_list_type_t *ptr);

/// \brief Returns the element type of a component list type.
///
/// The returned type must be deallocated with
/// `wasmtime_component_valtype_delete`.
WASM_API_EXTERN void wasmtime_component_list_type_element(
    const wasmtime_component_list_type_t *ty,
    struct wasmtime_component_valtype_t *type_ret);

// ----------- records ---------------------------------------------------------

/// \brief Opaque type representing a component record type.
typedef struct wasmtime_component_record_type wasmtime_component_record_type_t;

/// \brief Clones a component record type.
///
/// The returned pointer must be deallocated with
/// `wasmtime_component_record_type_delete`.
WASM_API_EXTERN wasmtime_component_record_type_t *
wasmtime_component_record_type_clone(
    const wasmtime_component_record_type_t *ty);

/// \brief Compares two component record types for equality.
WASM_API_EXTERN bool
wasmtime_component_record_type_equal(const wasmtime_component_record_type_t *a,
                                     const wasmtime_component_record_type_t *b);

/// \brief Deallocates a component record type.
WASM_API_EXTERN void
wasmtime_component_record_type_delete(wasmtime_component_record_type_t *ptr);

/// \brief Returns the number of fields in a component record type.
WASM_API_EXTERN size_t wasmtime_component_record_type_field_count(
    const wasmtime_component_record_type_t *ty);

/// \brief Returns the nth field in a component record type.
///
/// The returned type must be deallocated with
/// `wasmtime_component_valtype_delete`.
WASM_API_EXTERN bool wasmtime_component_record_type_field_nth(
    const wasmtime_component_record_type_t *ty, size_t nth,
    const char **name_ret, size_t *name_len_ret,
    struct wasmtime_component_valtype_t *type_ret);

// ----------- tuples  ---------------------------------------------------------

/// \brief Opaque type representing a component tuple type.
typedef struct wasmtime_component_tuple_type wasmtime_component_tuple_type_t;

/// \brief Clones a component tuple type.
///
/// The returned pointer must be deallocated with
/// `wasmtime_component_tuple_type_delete`.
WASM_API_EXTERN wasmtime_component_tuple_type_t *
wasmtime_component_tuple_type_clone(const wasmtime_component_tuple_type_t *ty);

/// \brief Compares two component tuple types for equality.
WASM_API_EXTERN bool
wasmtime_component_tuple_type_equal(const wasmtime_component_tuple_type_t *a,
                                    const wasmtime_component_tuple_type_t *b);

/// \brief Deallocates a component tuple type.
WASM_API_EXTERN void
wasmtime_component_tuple_type_delete(wasmtime_component_tuple_type_t *ptr);

/// \brief Returns the number of types in a component tuple type.
WASM_API_EXTERN size_t wasmtime_component_tuple_type_types_count(
    const wasmtime_component_tuple_type_t *ty);

/// \brief Returns the nth type in a component tuple type.
///
/// The returned type must be deallocated with
/// `wasmtime_component_valtype_delete`.
WASM_API_EXTERN bool wasmtime_component_tuple_type_types_nth(
    const wasmtime_component_tuple_type_t *ty, size_t nth,
    struct wasmtime_component_valtype_t *type_ret);

// ----------- variants --------------------------------------------------------

/// \brief Opaque type representing a component variant type.
typedef struct wasmtime_component_variant_type
    wasmtime_component_variant_type_t;

/// \brief Clones a component variant type.
///
/// The returned pointer must be deallocated with
/// `wasmtime_component_variant_type_delete`.
WASM_API_EXTERN wasmtime_component_variant_type_t *
wasmtime_component_variant_type_clone(
    const wasmtime_component_variant_type_t *ty);

/// \brief Compares two component variant types for equality.
WASM_API_EXTERN bool wasmtime_component_variant_type_equal(
    const wasmtime_component_variant_type_t *a,
    const wasmtime_component_variant_type_t *b);

/// \brief Deallocates a component variant type.
WASM_API_EXTERN void
wasmtime_component_variant_type_delete(wasmtime_component_variant_type_t *ptr);

/// \brief Returns the number of cases in a component variant type.
WASM_API_EXTERN size_t wasmtime_component_variant_type_case_count(
    const wasmtime_component_variant_type_t *ty);

/// \brief Returns the nth case in a component variant type.
/// The returned payload type must be deallocated with
/// `wasmtime_component_valtype_delete`.
WASM_API_EXTERN bool wasmtime_component_variant_type_case_nth(
    const wasmtime_component_variant_type_t *ty, size_t nth,
    const char **name_ret, size_t *name_len_ret, bool *has_payload_ret,
    struct wasmtime_component_valtype_t *payload_ret);

// ----------- enums -----------------------------------------------------------

/// \brief Opaque type representing a component enum type.
typedef struct wasmtime_component_enum_type wasmtime_component_enum_type_t;

/// \brief Clones a component enum type.
///
/// The returned pointer must be deallocated with
/// `wasmtime_component_enum_type_delete`.
WASM_API_EXTERN wasmtime_component_enum_type_t *
wasmtime_component_enum_type_clone(const wasmtime_component_enum_type_t *ty);

/// \brief Compares two component enum types for equality.
WASM_API_EXTERN bool
wasmtime_component_enum_type_equal(const wasmtime_component_enum_type_t *a,
                                   const wasmtime_component_enum_type_t *b);

/// \brief Deallocates a component enum type.
WASM_API_EXTERN void
wasmtime_component_enum_type_delete(wasmtime_component_enum_type_t *ptr);

/// \brief Returns the number of names in a component enum type.
WASM_API_EXTERN size_t wasmtime_component_enum_type_names_count(
    const wasmtime_component_enum_type_t *ty);

/// \brief Returns the nth name in a component enum type.
WASM_API_EXTERN bool
wasmtime_component_enum_type_names_nth(const wasmtime_component_enum_type_t *ty,
                                       size_t nth, const char **name_ret,
                                       size_t *name_len_ret);

// ----------- options ---------------------------------------------------------

/// \brief Opaque type representing a component option type.
typedef struct wasmtime_component_option_type wasmtime_component_option_type_t;

/// \brief Clones a component option type.
///
/// The returned pointer must be deallocated with
/// `wasmtime_component_option_type_delete`.
WASM_API_EXTERN wasmtime_component_option_type_t *
wasmtime_component_option_type_clone(
    const wasmtime_component_option_type_t *ty);

/// \brief Compares two component option types for equality.
WASM_API_EXTERN bool
wasmtime_component_option_type_equal(const wasmtime_component_option_type_t *a,
                                     const wasmtime_component_option_type_t *b);

/// \brief Deallocates a component option type.
WASM_API_EXTERN void
wasmtime_component_option_type_delete(wasmtime_component_option_type_t *ptr);

/// \brief Returns the inner type of a component option type.
///
/// The returned type must be deallocated with
/// `wasmtime_component_valtype_delete`.
WASM_API_EXTERN void wasmtime_component_option_type_ty(
    const wasmtime_component_option_type_t *ty,
    struct wasmtime_component_valtype_t *type_ret);

// ----------- results ---------------------------------------------------------

/// \brief Opaque type representing a component result type.
typedef struct wasmtime_component_result_type wasmtime_component_result_type_t;

/// \brief Clones a component result type.
///
/// The returned pointer must be deallocated with
/// `wasmtime_component_result_type_delete`.
WASM_API_EXTERN wasmtime_component_result_type_t *
wasmtime_component_result_type_clone(
    const wasmtime_component_result_type_t *ty);

/// \brief Compares two component result types for equality.
WASM_API_EXTERN bool
wasmtime_component_result_type_equal(const wasmtime_component_result_type_t *a,
                                     const wasmtime_component_result_type_t *b);

/// \brief Deallocates a component result type.
WASM_API_EXTERN void
wasmtime_component_result_type_delete(wasmtime_component_result_type_t *ptr);

/// \brief Returns the `ok` type of a component result type.
/// The returned type must be deallocated with
/// `wasmtime_component_valtype_delete`.
WASM_API_EXTERN bool wasmtime_component_result_type_ok(
    const wasmtime_component_result_type_t *ty,
    struct wasmtime_component_valtype_t *type_ret);

/// \brief Returns the `err` type of a component result type.
/// The returned type must be deallocated with
/// `wasmtime_component_valtype_delete`.
WASM_API_EXTERN bool wasmtime_component_result_type_err(
    const wasmtime_component_result_type_t *ty,
    struct wasmtime_component_valtype_t *type_ret);

// ----------- flags -----------------------------------------------------------

/// \brief Opaque type representing a component flags type.
typedef struct wasmtime_component_flags_type wasmtime_component_flags_type_t;

/// \brief Clones a component flags type.
///
/// The returned pointer must be deallocated with
/// `wasmtime_component_flags_type_delete`.
WASM_API_EXTERN wasmtime_component_flags_type_t *
wasmtime_component_flags_type_clone(const wasmtime_component_flags_type_t *ty);

/// \brief Compares two component flags types for equality.
WASM_API_EXTERN bool
wasmtime_component_flags_type_equal(const wasmtime_component_flags_type_t *a,
                                    const wasmtime_component_flags_type_t *b);

/// \brief Deallocates a component flags type.
WASM_API_EXTERN void
wasmtime_component_flags_type_delete(wasmtime_component_flags_type_t *ptr);

/// \brief Returns the number of names in a component flags type.
WASM_API_EXTERN size_t wasmtime_component_flags_type_names_count(
    const wasmtime_component_flags_type_t *ty);

/// \brief Returns the nth name in a component flags type.
WASM_API_EXTERN bool wasmtime_component_flags_type_names_nth(
    const wasmtime_component_flags_type_t *ty, size_t nth,
    const char **name_ret, size_t *name_len_ret);

// ----------- futures ---------------------------------------------------------

/// \brief Opaque type representing a component future type.
typedef struct wasmtime_component_future_type wasmtime_component_future_type_t;

/// \brief Clones a component future type.
///
/// The returned pointer must be deallocated with
/// `wasmtime_component_future_type_delete`.
WASM_API_EXTERN wasmtime_component_future_type_t *
wasmtime_component_future_type_clone(
    const wasmtime_component_future_type_t *ty);

/// \brief Compares two component future types for equality.
WASM_API_EXTERN bool
wasmtime_component_future_type_equal(const wasmtime_component_future_type_t *a,
                                     const wasmtime_component_future_type_t *b);

/// \brief Deallocates a component future type.
WASM_API_EXTERN void
wasmtime_component_future_type_delete(wasmtime_component_future_type_t *ptr);

/// \brief Returns the inner type of a component future type.
///
/// The returned type must be deallocated with
/// `wasmtime_component_valtype_delete`.
WASM_API_EXTERN bool wasmtime_component_future_type_ty(
    const wasmtime_component_future_type_t *ty,
    struct wasmtime_component_valtype_t *type_ret);

// ----------- streams ---------------------------------------------------------

/// \brief Opaque type representing a component stream type.
typedef struct wasmtime_component_stream_type wasmtime_component_stream_type_t;

/// \brief Clones a component stream type.
///
/// The returned pointer must be deallocated with
/// `wasmtime_component_stream_type_delete`.
WASM_API_EXTERN wasmtime_component_stream_type_t *
wasmtime_component_stream_type_clone(
    const wasmtime_component_stream_type_t *ty);

/// \brief Compares two component stream types for equality.
WASM_API_EXTERN bool
wasmtime_component_stream_type_equal(const wasmtime_component_stream_type_t *a,
                                     const wasmtime_component_stream_type_t *b);

/// \brief Deallocates a component stream type.
WASM_API_EXTERN void
wasmtime_component_stream_type_delete(wasmtime_component_stream_type_t *ptr);

/// \brief Returns the inner type of a component stream type.
///
/// The returned type must be deallocated with
/// `wasmtime_component_valtype_delete`.
WASM_API_EXTERN bool wasmtime_component_stream_type_ty(
    const wasmtime_component_stream_type_t *ty,
    struct wasmtime_component_valtype_t *type_ret);

// ----------- valtype ---------------------------------------------------------

/// \brief Value of #wasmtime_component_valtype_kind_t meaning that
/// #wasmtime_component_valtype_t is a `bool` WIT type.
#define WASMTIME_COMPONENT_VALTYPE_BOOL 0
/// \brief Value of #wasmtime_component_valtype_kind_t meaning that
/// #wasmtime_component_valtype_t is a `s8` WIT type.
#define WASMTIME_COMPONENT_VALTYPE_S8 1
/// \brief Value of #wasmtime_component_valtype_kind_t meaning that
/// #wasmtime_component_valtype_t is a `s16` WIT type.
#define WASMTIME_COMPONENT_VALTYPE_S16 2
/// \brief Value of #wasmtime_component_valtype_kind_t meaning that
/// #wasmtime_component_valtype_t is a `s32` WIT type.
#define WASMTIME_COMPONENT_VALTYPE_S32 3
/// \brief Value of #wasmtime_component_valtype_kind_t meaning that
/// #wasmtime_component_valtype_t is a `s64` WIT type.
#define WASMTIME_COMPONENT_VALTYPE_S64 4
/// \brief Value of #wasmtime_component_valtype_kind_t meaning that
/// #wasmtime_component_valtype_t is a `u8` WIT type.
#define WASMTIME_COMPONENT_VALTYPE_U8 5
/// \brief Value of #wasmtime_component_valtype_kind_t meaning that
/// #wasmtime_component_valtype_t is a `u16` WIT type.
#define WASMTIME_COMPONENT_VALTYPE_U16 6
/// \brief Value of #wasmtime_component_valtype_kind_t meaning that
/// #wasmtime_component_valtype_t is a `u32` WIT type.
#define WASMTIME_COMPONENT_VALTYPE_U32 7
/// \brief Value of #wasmtime_component_valtype_kind_t meaning that
/// #wasmtime_component_valtype_t is a `u64` WIT type.
#define WASMTIME_COMPONENT_VALTYPE_U64 8
/// \brief Value of #wasmtime_component_valtype_kind_t meaning that
/// #wasmtime_component_valtype_t is a `f32` WIT type.
#define WASMTIME_COMPONENT_VALTYPE_F32 9
/// \brief Value of #wasmtime_component_valtype_kind_t meaning that
/// #wasmtime_component_valtype_t is a `f64` WIT type.
#define WASMTIME_COMPONENT_VALTYPE_F64 10
/// \brief Value of #wasmtime_component_valtype_kind_t meaning that
/// #wasmtime_component_valtype_t is a `char` WIT type.
#define WASMTIME_COMPONENT_VALTYPE_CHAR 11
/// \brief Value of #wasmtime_component_valtype_kind_t meaning that
/// #wasmtime_component_valtype_t is a `string` WIT type.
#define WASMTIME_COMPONENT_VALTYPE_STRING 12
/// \brief Value of #wasmtime_component_valtype_kind_t meaning that
/// #wasmtime_component_valtype_t is a `list` WIT type.
#define WASMTIME_COMPONENT_VALTYPE_LIST 13
/// \brief Value of #wasmtime_component_valtype_kind_t meaning that
/// #wasmtime_component_valtype_t is a `record` WIT type.
#define WASMTIME_COMPONENT_VALTYPE_RECORD 14
/// \brief Value of #wasmtime_component_valtype_kind_t meaning that
/// #wasmtime_component_valtype_t is a `tuple` WIT type.
#define WASMTIME_COMPONENT_VALTYPE_TUPLE 15
/// \brief Value of #wasmtime_component_valtype_kind_t meaning that
/// #wasmtime_component_valtype_t is a `variant` WIT type.
#define WASMTIME_COMPONENT_VALTYPE_VARIANT 16
/// \brief Value of #wasmtime_component_valtype_kind_t meaning that
/// #wasmtime_component_valtype_t is a `enum` WIT type.
#define WASMTIME_COMPONENT_VALTYPE_ENUM 17
/// \brief Value of #wasmtime_component_valtype_kind_t meaning that
/// #wasmtime_component_valtype_t is a `option` WIT type.
#define WASMTIME_COMPONENT_VALTYPE_OPTION 18
/// \brief Value of #wasmtime_component_valtype_kind_t meaning that
/// #wasmtime_component_valtype_t is a `result` WIT type.
#define WASMTIME_COMPONENT_VALTYPE_RESULT 19
/// \brief Value of #wasmtime_component_valtype_kind_t meaning that
/// #wasmtime_component_valtype_t is a `flags` WIT type.
#define WASMTIME_COMPONENT_VALTYPE_FLAGS 20
/// \brief Value of #wasmtime_component_valtype_kind_t meaning that
/// #wasmtime_component_valtype_t is a resource `own` WIT type.
#define WASMTIME_COMPONENT_VALTYPE_OWN 21
/// \brief Value of #wasmtime_component_valtype_kind_t meaning that
/// #wasmtime_component_valtype_t is a resource `borrow` WIT type.
#define WASMTIME_COMPONENT_VALTYPE_BORROW 22
/// \brief Value of #wasmtime_component_valtype_kind_t meaning that
/// #wasmtime_component_valtype_t is a `future` WIT type.
#define WASMTIME_COMPONENT_VALTYPE_FUTURE 23
/// \brief Value of #wasmtime_component_valtype_kind_t meaning that
/// #wasmtime_component_valtype_t is a `stream` WIT type.
#define WASMTIME_COMPONENT_VALTYPE_STREAM 24
/// \brief Value of #wasmtime_component_valtype_kind_t meaning that
/// #wasmtime_component_valtype_t is an `error context` WIT type.
#define WASMTIME_COMPONENT_VALTYPE_ERROR_CONTEXT 25

/// \brief Discriminant used in #wasmtime_component_valtype_t::kind
typedef uint8_t wasmtime_component_valtype_kind_t;

/// \brief Represents a single value type in the component model.
typedef union wasmtime_component_valtype_union {
  /// Field used if #wasmtime_component_valtype_t::kind is
  /// #WASMTIME_COMPONENT_VALTYPE_LIST
  wasmtime_component_list_type_t *list;
  /// Field used if #wasmtime_component_valtype_t::kind is
  /// #WASMTIME_COMPONENT_VALTYPE_RECORD
  wasmtime_component_record_type_t *record;
  /// Field used if #wasmtime_component_valtype_t::kind is
  /// #WASMTIME_COMPONENT_VALTYPE_TUPLE
  wasmtime_component_tuple_type_t *tuple;
  /// Field used if #wasmtime_component_valtype_t::kind is
  /// #WASMTIME_COMPONENT_VALTYPE_VARIANT
  wasmtime_component_variant_type_t *variant;
  /// Field used if #wasmtime_component_valtype_t::kind is
  /// #WASMTIME_COMPONENT_VALTYPE_ENUM
  wasmtime_component_enum_type_t *enum_;
  /// Field used if #wasmtime_component_valtype_t::kind is
  /// #WASMTIME_COMPONENT_VALTYPE_OPTION
  wasmtime_component_option_type_t *option;
  /// Field used if #wasmtime_component_valtype_t::kind is
  /// #WASMTIME_COMPONENT_VALTYPE_RESULT
  wasmtime_component_result_type_t *result;
  /// Field used if #wasmtime_component_valtype_t::kind is
  /// #WASMTIME_COMPONENT_VALTYPE_FLAGS
  wasmtime_component_flags_type_t *flags;
  /// Field used if #wasmtime_component_valtype_t::kind is
  /// #WASMTIME_COMPONENT_VALTYPE_OWN
  wasmtime_component_resource_type_t *own;
  /// Field used if #wasmtime_component_valtype_t::kind is
  /// #WASMTIME_COMPONENT_VALTYPE_BORROW
  wasmtime_component_resource_type_t *borrow;
  /// Field used if #wasmtime_component_valtype_t::kind is
  /// #WASMTIME_COMPONENT_VALTYPE_FUTURE
  wasmtime_component_future_type_t *future;
  /// Field used if #wasmtime_component_valtype_t::kind is
  /// #WASMTIME_COMPONENT_VALTYPE_STREAM
  wasmtime_component_stream_type_t *stream;
} wasmtime_component_valtype_union_t;

/// \brief Represents a single value type in the component model.
typedef struct wasmtime_component_valtype_t {
  /// The type discriminant for the `of` union.
  wasmtime_component_valtype_kind_t kind;
  /// The actual type.
  wasmtime_component_valtype_union_t of;
} wasmtime_component_valtype_t;

/// \brief Clones a component value type.
///
/// The returned pointer must be deallocated with
/// `wasmtime_component_valtype_delete`.
WASM_API_EXTERN void
wasmtime_component_valtype_clone(const wasmtime_component_valtype_t *ty,
                                 wasmtime_component_valtype_t *out);

/// \brief Compares two component value types for equality.
WASM_API_EXTERN bool
wasmtime_component_valtype_equal(const wasmtime_component_valtype_t *a,
                                 const wasmtime_component_valtype_t *b);

/// \brief Deallocates a component value type.
WASM_API_EXTERN void
wasmtime_component_valtype_delete(wasmtime_component_valtype_t *ptr);

#ifdef __cplusplus
}
#endif

#endif // WASMTIME_FEATURE_COMPONENT_MODEL

#endif // WASMTIME_COMPONENT_TYPES_VAL_H
