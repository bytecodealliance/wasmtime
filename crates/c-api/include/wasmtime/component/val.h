/// \file wasmtime/component/val.h

#ifndef WASMTIME_COMPONENT_VAL_H
#define WASMTIME_COMPONENT_VAL_H

#include <wasmtime/conf.h>

#ifdef WASMTIME_FEATURE_COMPONENT_MODEL

#include <stdint.h>
#include <wasm.h>
#include <wasmtime/component/types/resource.h>
#include <wasmtime/store.h>

#ifdef __cplusplus
extern "C" {
#endif

/// \brief Represents a component resource which can be either guest-owned or
/// host-owned.
///
/// This type is an opaque type used to represent any component model resource.
/// Internally this tracks information about ownership, type, etc. Values of
/// this type have dynamic ownership guarantees associated with them. Notably
/// from a component-model perspective values of this type must either be
/// converted to a host resource with `wasmtime_component_resource_any_to_host`
/// or dropped via `wasmtime_component_resource_any_drop`. This is required to
/// handle various metadata tracking appropriately, and if this is not done
/// then the resource will be leaked into the store and a trap may be raised.
///
/// Note that this type also has dynamic memory allocations associated with it
/// and users must call `wasmtime_component_resource_any_delete` to deallocate
/// the host-side resources. This destructor can be called in an RAII fashion
/// and will only clean up memory, not metadata related to the resource.
/// It is required to call `wasmtime_component_resource_any_delete` to prevent
/// leaking memory on the host. It's highly recommended to call
/// `wasmtime_component_resource_any_drop` to avoid leaking memory in a
/// long-lived store, but if this is forgotten then deallocating the store will
/// deallocate all memory still.
typedef struct wasmtime_component_resource_any
    wasmtime_component_resource_any_t;

/// \brief Gets the type of a component resource.
///
/// Returns an owned `wasmtime_component_resource_type_t` which represents the
/// type of this resource.
///
/// The pointer returned from this function must be deallocated with
/// `wasmtime_component_resource_type_delete`.
WASM_API_EXTERN
wasmtime_component_resource_type_t *wasmtime_component_resource_any_type(
    const wasmtime_component_resource_any_t *resource);

/// \brief Clones a component resource.
///
/// Creates a new owned copy of a component resource. Note that the returned
/// resource still logically refers to the same resource as before, but this
/// can be convenient from an API perspective. Calls to
/// `wasmtime_component_resource_any_drop` need only happen
/// once-per-logical-resource, not once-per-handle-to-the-resource. Note though
/// that calls to `wasmtime_component_resource_any_delete` must happen
/// once-per-handle-to-the-resource.
///
/// The pointer returned from this function must be deallocated with
/// `wasmtime_component_resource_any_delete`.
WASM_API_EXTERN
wasmtime_component_resource_any_t *wasmtime_component_resource_any_clone(
    const wasmtime_component_resource_any_t *resource);

/// \brief Returns whether this resource is an `own`, or a `borrow` in the
/// component model.
WASM_API_EXTERN
bool wasmtime_component_resource_any_owned(
    const wasmtime_component_resource_any_t *resource);

/// \brief Drops a component resource.
///
/// This function is required to be called per "logical resource" to clean up
/// any borrow-tracking state in the store, for example. Additionally this may
/// invoke WebAssembly if it's a guest-owned resource with a destructor
/// associated with it.
///
/// This operation is not to be confused with
/// `wasmtime_component_resource_any_delete` which deallocates host-related
/// memory for this resource. After `wasmtime_component_resource_any_drop` is
/// called it's still required to call
/// `wasmtime_component_resource_any_delete`.
WASM_API_EXTERN
wasmtime_error_t *wasmtime_component_resource_any_drop(
    wasmtime_context_t *ctx, const wasmtime_component_resource_any_t *resource);

/// \brief Deallocates a component resource.
///
/// This function deallocates any host-side memory associated with this
/// resource. This function does not perform any component-model related
/// cleanup, and `wasmtime_component_resource_any_drop` is required for that.
WASM_API_EXTERN
void wasmtime_component_resource_any_delete(
    wasmtime_component_resource_any_t *resource);

/// \brief Represents a host-defined component resource.
///
/// This structure is similar to `wasmtime_component_resource_any_t` except
/// that it unconditionally represents an embedder-defined resource via this
/// API. Host resources have a "rep" which is a 32-bit integer whose meaning
/// is defined by the host. This "rep" is trusted in the sense that the guest
/// cannot forge this so the embedder is the only one that can view this.
///
/// Host resources also have a 32-bit type whose meaning is also defined by the
/// host and has no meaning internally. This is used to distinguish different
/// types of resources from one another.
///
/// Also note that unlike `wasmtime_component_resource_any_t` host resources
/// do not have a "drop" operation. It's up to the host to define what it means
/// to drop an owned resource and handle that appropriately.
typedef struct wasmtime_component_resource_host
    wasmtime_component_resource_host_t;

/// \brief Creates a new host-defined component resource.
///
/// This function creates a new host-defined component resource with the
/// provided parameters. The `owned` parameter indicates whether this resource
/// is an `own` or a `borrow` in the component model. The `rep` and `ty`
/// parameters are 32-bit integers which only have meaning to the embedder and
/// are plumbed through with this resource.
///
/// The pointer returned from this function must be deallocated with
/// `wasmtime_component_resource_host_delete`.
WASM_API_EXTERN
wasmtime_component_resource_host_t *
wasmtime_component_resource_host_new(bool owned, uint32_t rep, uint32_t ty);

/// \brief Clones a host-defined component resource.
///
/// Creates a new owned copy of a host-defined component resource. Note that the
/// returned resource still logically refers to the same resource as before,
/// but this can be convenient from an API perspective.
///
/// The pointer returned from this function must be deallocated with
/// `wasmtime_component_resource_host_delete`.
WASM_API_EXTERN
wasmtime_component_resource_host_t *wasmtime_component_resource_host_clone(
    const wasmtime_component_resource_host_t *resource);

/// \brief Gets the "rep" of a host-defined component resource.
///
/// Returns the 32-bit integer "rep" associated with this resource. This is a
/// trusted value that guests cannot forge.
WASM_API_EXTERN
uint32_t wasmtime_component_resource_host_rep(
    const wasmtime_component_resource_host_t *resource);

/// \brief Gets the "type" of a host-defined component resource.
///
/// Returns the 32-bit integer "type" associated with this resource. This is a
/// trusted value that guests cannot forge.
WASM_API_EXTERN
uint32_t wasmtime_component_resource_host_type(
    const wasmtime_component_resource_host_t *resource);

/// \brief Returns whether this host-defined resource is an `own` or a `borrow`
/// in the component model.
WASM_API_EXTERN
bool wasmtime_component_resource_host_owned(
    const wasmtime_component_resource_host_t *resource);

/// \brief Deallocates a host-defined component resource.
///
/// This function deallocates any host-side memory associated with this
/// resource.
WASM_API_EXTERN
void wasmtime_component_resource_host_delete(
    wasmtime_component_resource_host_t *resource);

/// \brief Attempts to convert a `wasmtime_component_resource_any_t` into a
/// `wasmtime_component_resource_host_t`.
///
/// This function will attempt to convert the provided `resource` into a
/// host-defined resource. If the resource is indeed host-defined then a new
/// owned `wasmtime_component_resource_host_t` is returned via `ret`. If the
/// resource is guest-defined then an error is returned and `ret` is not
/// modified.
///
/// If no error is returned then the pointer written to `ret` must be
/// deallocated with `wasmtime_component_resource_host_delete`.
WASM_API_EXTERN
wasmtime_error_t *wasmtime_component_resource_any_to_host(
    wasmtime_context_t *ctx, const wasmtime_component_resource_any_t *resource,
    wasmtime_component_resource_host_t **ret);

/// \brief Same as `wasmtime_component_resource_any_to_host` except for
/// converting the other way around.
///
/// This can fail in some edge-case scenarios but typically does not fail.
WASM_API_EXTERN
wasmtime_error_t *wasmtime_component_resource_host_to_any(
    wasmtime_context_t *ctx, const wasmtime_component_resource_host_t *resource,
    wasmtime_component_resource_any_t **ret);

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
/// \brief Value of #wasmtime_component_valkind_t meaning that
/// #wasmtime_component_val_t is a tuple
#define WASMTIME_COMPONENT_TUPLE 15
/// \brief Value of #wasmtime_component_valkind_t meaning that
/// #wasmtime_component_val_t is a variant
#define WASMTIME_COMPONENT_VARIANT 16
/// \brief Value of #wasmtime_component_valkind_t meaning that
/// #wasmtime_component_val_t is a enum
#define WASMTIME_COMPONENT_ENUM 17
/// \brief Value of #wasmtime_component_valkind_t meaning that
/// #wasmtime_component_val_t is a option
#define WASMTIME_COMPONENT_OPTION 18
/// \brief Value of #wasmtime_component_valkind_t meaning that
/// #wasmtime_component_val_t is a result
#define WASMTIME_COMPONENT_RESULT 19
/// \brief Value of #wasmtime_component_valkind_t meaning that
/// #wasmtime_component_val_t is flags
#define WASMTIME_COMPONENT_FLAGS 20
/// \brief Value of #wasmtime_component_valkind_t meaning that
/// #wasmtime_component_val_t is a resource
#define WASMTIME_COMPONENT_RESOURCE 21

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
  WASM_API_EXTERN void name##_new(name##_t *out, size_t size,                  \
                                  const type *ptr);                            \
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
DECLARE_VEC(wasmtime_component_valtuple, struct wasmtime_component_val)
DECLARE_VEC(wasmtime_component_valflags, wasm_name_t)

#undef DECLARE_VEC

/// Represents a variant type
typedef struct {
  /// The discriminant of the variant
  wasm_name_t discriminant;
  /// The payload of the variant
  struct wasmtime_component_val *val;
} wasmtime_component_valvariant_t;

/// Represents a result type
typedef struct {
  /// The discriminant of the result
  bool is_ok;
  /// The 'ok' value if #wasmtime_component_valresult_t::is_ok is `true`, else
  /// the 'err' value
  struct wasmtime_component_val *val;
} wasmtime_component_valresult_t;

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
  /// Field used if #wasmtime_component_val_t::kind is #WASMTIME_COMPONENT_TUPLE
  wasmtime_component_valtuple_t tuple;
  /// Field used if #wasmtime_component_val_t::kind is
  /// #WASMTIME_COMPONENT_VARIANT
  wasmtime_component_valvariant_t variant;
  /// Field used if #wasmtime_component_val_t::kind is #WASMTIME_COMPONENT_ENUM
  wasm_name_t enumeration;
  /// Field used if #wasmtime_component_val_t::kind is
  /// #WASMTIME_COMPONENT_OPTION
  struct wasmtime_component_val *option;
  /// Field used if #wasmtime_component_val_t::kind is
  /// #WASMTIME_COMPONENT_RESULT
  wasmtime_component_valresult_t result;
  /// Field used if #wasmtime_component_val_t::kind is #WASMTIME_COMPONENT_FLAGS
  wasmtime_component_valflags_t flags;
  /// Field used if #wasmtime_component_val_t::kind is
  /// #WASMTIME_COMPONENT_RESOURCE
  wasmtime_component_resource_any_t *resource;
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

/// \brief Allocates a new `wasmtime_component_val_t` on the heap, initializing
/// it with the contents of `val`.
///
/// This function is intended to be used with the `variant`, `result`, and
/// `option` fields of `wasmtime_component_valunion_t`. The returned pointer
/// must be deallocated with `wasmtime_component_val_free` to deallocate the
/// heap-owned pointer. The `val` provided is "taken" meaning that its contents
/// are transferred to the returned pointer. This is not a clone operation but
/// instead is an operation used to move `val` onto a Wasmtime-defined heap
/// allocation.
///
/// Note that `wasmtime_component_val_delete` should not be used for
/// deallocating the return value because that will deallocate the contents of
/// the value but not the value pointer itself.
WASM_API_EXTERN wasmtime_component_val_t *
wasmtime_component_val_new(wasmtime_component_val_t *val);

/// \brief Deallocates the heap-allocated value at `ptr`.
///
/// This function will perform `wasmtime_component_val_delete` on `ptr` and
/// then it will deallocate `ptr` itself. This should not be used on
/// embedder-owned `ptr` storage. This function is used to clean up
/// allocations made by `wasmtime_component_val_new`.
WASM_API_EXTERN void wasmtime_component_val_free(wasmtime_component_val_t *ptr);

/// \brief Performs a deep copy of the provided `src`, storing the results into
/// `dst`.
///
/// The `dst` value must have `wasmtime_component_val_delete` run to discard
/// its contents.
WASM_API_EXTERN void
wasmtime_component_val_clone(const wasmtime_component_val_t *src,
                             wasmtime_component_val_t *dst);

/// \brief Deallocates any memory owned by `value`.
///
/// This function will look at `value->kind` and deallocate any memory if
/// necessary. For example lists will deallocate
/// `value->of.list`.
///
/// Note that this function is not to be confused with
/// `wasmtime_component_val_free` which not only deallocates the memory that
/// `value` owns but also deallocates the memory of `value` itself. This
/// function should only be used when the embedder owns the pointer `value`
/// itself.
WASM_API_EXTERN void
wasmtime_component_val_delete(wasmtime_component_val_t *value);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_FEATURE_COMPONENT_MODEL

#endif // WASMTIME_COMPONENT_VAL_H
