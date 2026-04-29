/**
 * \file wasmtime/types/val.h
 *
 * Wasmtime-specific extensions for `wasm_valtype_t` and structures for working
 * with the full breadth of value types in the wasm type system.
 */

#ifndef WASMTIME_TYPES_VAL_H
#define WASMTIME_TYPES_VAL_H

#include <wasm.h>
#include <wasmtime/types/arrayref.h>
#include <wasmtime/types/exnref.h>
#include <wasmtime/types/structref.h>

#ifdef __cplusplus
extern "C" {
#endif

/// \brief Returns a new value type representing the wasm `v128` type.
WASM_API_EXTERN wasm_valtype_t *wasmtime_wasm_valtype_v128(void);

/// \brief Returns whether `a` is logically equal to `b`.
WASM_API_EXTERN bool wasmtime_wasm_valtype_equal(const wasm_valtype_t *a,
                                                 const wasm_valtype_t *b);

/// \brief Discriminant located in `wasmtime_heaptype_t.kind`
typedef uint8_t wasmtime_heaptype_kind_t;

/// \brief Heap type for the abstract `extern` type.
#define WASMTIME_HEAPTYPE_KIND_EXTERN 0
/// \brief Heap type for the abstract `noextern` type.
#define WASMTIME_HEAPTYPE_KIND_NOEXTERN 1
/// \brief Heap type for the abstract `func` type.
#define WASMTIME_HEAPTYPE_KIND_FUNC 2
/// \brief Heap type for a concrete function type.
#define WASMTIME_HEAPTYPE_KIND_CONCRETE_FUNC 3
/// \brief Heap type for the abstract `nofunc` type.
#define WASMTIME_HEAPTYPE_KIND_NOFUNC 4
/// \brief Heap type for the abstract `any` type.
#define WASMTIME_HEAPTYPE_KIND_ANY 5
/// \brief Heap type for the abstract `none` type.
#define WASMTIME_HEAPTYPE_KIND_NONE 6
/// \brief Heap type for the abstract `eq` type.
#define WASMTIME_HEAPTYPE_KIND_EQ 7
/// \brief Heap type for the abstract `i31` type.
#define WASMTIME_HEAPTYPE_KIND_I31 8
/// \brief Heap type for the abstract `array` type.
#define WASMTIME_HEAPTYPE_KIND_ARRAY 9
/// \brief Heap type for a concrete array type.
#define WASMTIME_HEAPTYPE_KIND_CONCRETE_ARRAY 10
/// \brief Heap type for the abstract `struct` type.
#define WASMTIME_HEAPTYPE_KIND_STRUCT 11
/// \brief Heap type for a concrete struct type.
#define WASMTIME_HEAPTYPE_KIND_CONCRETE_STRUCT 12
/// \brief Heap type for the abstract `exn` type.
#define WASMTIME_HEAPTYPE_KIND_EXN 13
/// \brief Heap type for a concrete exception type.
#define WASMTIME_HEAPTYPE_KIND_CONCRETE_EXN 14
/// \brief Heap type for the abstract `noexn` type.
#define WASMTIME_HEAPTYPE_KIND_NOEXN 15

/// \brief Payload of the `wasmtime_heaptype_t` union.
typedef union wasmtime_heaptype_union {
  /// \brief Used with `WASMTIME_HEAPTYPE_KIND_CONCRETE_FUNC`.
  wasm_functype_t *concrete_func;
  /// \brief Used with `WASMTIME_HEAPTYPE_KIND_CONCRETE_ARRAY`.
  wasmtime_array_type_t *concrete_array;
  /// \brief Used with `WASMTIME_HEAPTYPE_KIND_CONCRETE_STRUCT`.
  wasmtime_struct_type_t *concrete_struct;
  /// \brief Used with `WASMTIME_HEAPTYPE_KIND_CONCRETE_EXN`.
  wasmtime_exn_type_t *concrete_exn;
} wasmtime_heaptype_union_t;

/// \brief A WebAssembly heap type.
typedef struct wasmtime_heaptype {
  /// \brief Discriminant of which heap type this is, and may indicate fields of
  /// `of` to use.
  wasmtime_heaptype_kind_t kind;
  /// \brief Payload of this heap type, with fields indicated by `kind`.
  wasmtime_heaptype_union_t of;
} wasmtime_heaptype_t;

/// \brief Clones `ty` into `out`.
WASM_API_EXTERN void wasmtime_heaptype_clone(const wasmtime_heaptype_t *ty,
                                             wasmtime_heaptype_t *out);

/// \brief Deletes any payload of `ty`, if applicable.
///
/// Only necessary to call for concrete types.
WASM_API_EXTERN void wasmtime_heaptype_delete(wasmtime_heaptype_t *ty);

/// \brief A WebAssembly reference type.
typedef struct wasmtime_reftype {
  /// \brief Whether this reference type is nullable.
  bool nullable;
  /// \brief The heap type of this reference type.
  wasmtime_heaptype_t heaptype;
} wasmtime_reftype_t;

/// \brief Clones `ty` into `out`.
WASM_API_EXTERN void wasmtime_reftype_clone(const wasmtime_reftype_t *ty,
                                            wasmtime_reftype_t *out);

/// \brief Deletes any payload of `ty`, if applicable.
///
/// Only necessary if `ty->heaptype` is concrete.
WASM_API_EXTERN void wasmtime_reftype_delete(wasmtime_reftype_t *ty);

/// \brief Discriminant located in `wasmtime_valtype_t.kind`
typedef uint8_t wasmtime_valtype_kind_t;

/// \brief The WebAssembly `i32` type.
#define WASMTIME_VALTYPE_KIND_I32 0
/// \brief The WebAssembly `i64` type.
#define WASMTIME_VALTYPE_KIND_I64 1
/// \brief The WebAssembly `f32` type.
#define WASMTIME_VALTYPE_KIND_F32 2
/// \brief The WebAssembly `f64` type.
#define WASMTIME_VALTYPE_KIND_F64 3
/// \brief The WebAssembly `v128` type.
#define WASMTIME_VALTYPE_KIND_V128 4
/// \brief A WebAssembly reference type.
#define WASMTIME_VALTYPE_KIND_REF 5

/// \brief A WebAssembly value type.
///
/// Note that this is a parallel representation to `wasm_valtype_t` which
/// is intended to support the entire breadth of WebAssembly that wasmtime
/// supports.
typedef struct wasmtime_valtype {
  /// \brief Discriminant of which value type this is.
  wasmtime_valtype_kind_t kind;
  /// \brief Payload of this value type, only used with
  /// `WASMTIME_VALTYPE_KIND_REF`.
  wasmtime_reftype_t reftype;
} wasmtime_valtype_t;

/// \brief Creates a new type in `out` from the type in `ty`.
WASM_API_EXTERN void wasmtime_valtype_new(const wasm_valtype_t *ty,
                                          wasmtime_valtype_t *out);

/// \brief Clones `ty` into `out`.
WASM_API_EXTERN void wasmtime_valtype_clone(const wasmtime_valtype_t *ty,
                                            wasmtime_valtype_t *out);

/// \brief Deletes any payload of `ty`, if applicable.
///
/// Only necessary when `ty` is a concrete reference type.
WASM_API_EXTERN void wasmtime_valtype_delete(wasmtime_valtype_t *ty);

/// \brief Converts `ty` into a `wasm_valtype_t` and returns a pointer to it.
///
/// The caller must deallocate the returned value.
WASM_API_EXTERN wasm_valtype_t *
wasmtime_valtype_to_wasm(const wasm_engine_t *engine,
                         const wasmtime_valtype_t *ty);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_TYPES_VAL_H
