/**
 * The component model
 *
 * Wasmtime APIs for interacting with WebAssembly Component Model.
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

/**
 * \brief The tag part of #wasmtime_component_val_t or
 * #wasmtime_component_type_t
 *
 * Specifies what variant is populated in #wasmtime_component_val_payload_t
 * or #wasmtime_component_type_payload_t.
 *
 * Note that resources are currently not supported.
 */
typedef uint8_t wasmtime_component_kind_t;

/// Value of #wasmtime_component_kind_t indicating a boolean
#define WASMTIME_COMPONENT_KIND_BOOL 0
/// Value of #wasmtime_component_kind_t indicating a signed 8-bit integer
#define WASMTIME_COMPONENT_KIND_S8 1
/// Value of #wasmtime_component_kind_t indicating an unsigned 8-bit integer
#define WASMTIME_COMPONENT_KIND_U8 2
/// Value of #wasmtime_component_kind_t indicating a signed 16-bit integer
#define WASMTIME_COMPONENT_KIND_S16 3
/// Value of #wasmtime_component_kind_t indicating an unsigned 16-bit integer
#define WASMTIME_COMPONENT_KIND_U16 4
/// Value of #wasmtime_component_kind_t indicating a signed 32-bit integer
#define WASMTIME_COMPONENT_KIND_S32 5
/// Value of #wasmtime_component_kind_t indicating an unsigned 32-bit integer
#define WASMTIME_COMPONENT_KIND_U32 6
/// Value of #wasmtime_component_kind_t indicating a signed 64-bit integer
#define WASMTIME_COMPONENT_KIND_S64 7
/// Value of #wasmtime_component_kind_t indicating an unsigned 64-bit integer
#define WASMTIME_COMPONENT_KIND_U64 8
/// Value of #wasmtime_component_kind_t indicating a 32-bit floating point
/// number
#define WASMTIME_COMPONENT_KIND_F32 9
/// Value of #wasmtime_component_kind_t indicating a 64-bit floating point
/// number
#define WASMTIME_COMPONENT_KIND_F64 10
/// Value of #wasmtime_component_kind_t indicating a unicode character
#define WASMTIME_COMPONENT_KIND_CHAR 11
/// Value of #wasmtime_component_kind_t indicating a unicode string
#define WASMTIME_COMPONENT_KIND_STRING 12
/// Value of #wasmtime_component_kind_t indicating a list
#define WASMTIME_COMPONENT_KIND_LIST 13
/// Value of #wasmtime_component_kind_t indicating a record
#define WASMTIME_COMPONENT_KIND_RECORD 14
/// Value of #wasmtime_component_kind_t indicating a tuple
#define WASMTIME_COMPONENT_KIND_TUPLE 15
/// Value of #wasmtime_component_kind_t indicating a variant
#define WASMTIME_COMPONENT_KIND_VARIANT 16
/// Value of #wasmtime_component_kind_t indicating an enum
#define WASMTIME_COMPONENT_KIND_ENUM 17
/// Value of #wasmtime_component_kind_t indicating an option
#define WASMTIME_COMPONENT_KIND_OPTION 18
/// Value of #wasmtime_component_kind_t indicating a result
#define WASMTIME_COMPONENT_KIND_RESULT 19
/// Value of #wasmtime_component_kind_t indicating a set of flags
#define WASMTIME_COMPONENT_KIND_FLAGS 20

// forward declarations
typedef struct wasmtime_component_val_t wasmtime_component_val_t;
typedef struct wasmtime_component_val_record_field_t
    wasmtime_component_val_record_field_t;
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

/// A vector of values.
WASMTIME_COMPONENT_DECLARE_VEC(val_vec, wasmtime_component_val_t);

/// A tuple of named fields.
WASMTIME_COMPONENT_DECLARE_VEC(val_record,
                               wasmtime_component_val_record_field_t);

/// A variable sized bitset.
WASMTIME_COMPONENT_DECLARE_VEC(val_flags, uint32_t);
WASMTIME_COMPONENT_DECLARE_VEC_NEW(val_flags, uint32_t);

/// A vector of types
WASMTIME_COMPONENT_DECLARE_VEC(type_vec, wasmtime_component_type_t);

/// A vector of field types
WASMTIME_COMPONENT_DECLARE_VEC(type_field_vec, wasmtime_component_type_field_t);

/// A vector of strings
WASMTIME_COMPONENT_DECLARE_VEC(string_vec, wasm_name_t);
WASMTIME_COMPONENT_DECLARE_VEC_NEW(string_vec, wasm_name_t);

#undef WASMTIME_COMPONENT_DECLARE_VEC

/// Representation of a variant value
typedef struct wasmtime_component_val_variant_t {
  /// Discriminant indicating the index of the variant case of this value
  uint32_t discriminant;
  /// #wasmtime_component_val_t value of the variant case of this value if it
  /// has one
  wasmtime_component_val_t *val;
} wasmtime_component_val_variant_t;

/**
 * \brief Representation of a result value
 *
 * A result is an either type holding an ok value or an error
 */
typedef struct wasmtime_component_val_result_t {
  /// Value of the result, either of the ok type or of the error type, depending
  /// on #error
  wasmtime_component_val_t *val;
  /// Discriminant indicating if this result is an ok value or an error
  bool error;
} wasmtime_component_val_result_t;

/// Representation of an enum value
typedef struct wasmtime_component_val_enum_t {
  /// Discriminant indicating the index of this value in the enum
  uint32_t discriminant;
} wasmtime_component_val_enum_t;

/**
 * \brief Container for different kind of component model value data
 *
 * Setting dynamic data to one of this field means that the payload now owns it
 */
typedef union wasmtime_component_val_payload_t {
  /// Field used if #wasmtime_component_val_t::kind is
  /// #WASMTIME_COMPONENT_KIND_BOOLEAN
  bool boolean;
  /// Field used if #wasmtime_component_val_t::kind is
  /// #WASMTIME_COMPONENT_KIND_S8
  int8_t s8;
  /// Field used if #wasmtime_component_val_t::kind is
  /// #WASMTIME_COMPONENT_KIND_U8
  uint8_t u8;
  /// Field used if #wasmtime_component_val_t::kind is
  /// #WASMTIME_COMPONENT_KIND_S16
  int16_t s16;
  /// Field used if #wasmtime_component_val_t::kind is
  /// #WASMTIME_COMPONENT_KIND_U16
  uint16_t u16;
  /// Field used if #wasmtime_component_val_t::kind is
  /// #WASMTIME_COMPONENT_KIND_S32
  int32_t s32;
  /// Field used if #wasmtime_component_val_t::kind is
  /// #WASMTIME_COMPONENT_KIND_U32
  uint32_t u32;
  /// Field used if #wasmtime_component_val_t::kind is
  /// #WASMTIME_COMPONENT_KIND_S64
  int64_t s64;
  /// Field used if #wasmtime_component_val_t::kind is
  /// #WASMTIME_COMPONENT_KIND_U64
  uint64_t u64;
  /// Field used if #wasmtime_component_val_t::kind is
  /// #WASMTIME_COMPONENT_KIND_F32
  float f32;
  /// Field used if #wasmtime_component_val_t::kind is
  /// #WASMTIME_COMPONENT_KIND_F64
  double f64;
  /// Field used if #wasmtime_component_val_t::kind is
  /// #WASMTIME_COMPONENT_KIND_CHARACTER
  uint8_t character;
  /// Field used if #wasmtime_component_val_t::kind is
  /// #WASMTIME_COMPONENT_KIND_STRING
  wasm_name_t string;
  /// Field used if #wasmtime_component_val_t::kind is
  /// #WASMTIME_COMPONENT_KIND_LIST
  wasmtime_component_val_vec_t list;
  /// Field used if #wasmtime_component_val_t::kind is
  /// #WASMTIME_COMPONENT_KIND_RECORD
  wasmtime_component_val_record_t record;
  /// Field used if #wasmtime_component_val_t::kind is
  /// #WASMTIME_COMPONENT_KIND_TUPLE
  wasmtime_component_val_vec_t tuple;
  /// Field used if #wasmtime_component_val_t::kind is
  /// #WASMTIME_COMPONENT_KIND_VARIANT
  wasmtime_component_val_variant_t variant;
  /// Field used if #wasmtime_component_val_t::kind is
  /// #WASMTIME_COMPONENT_KIND_ENUMERATION
  wasmtime_component_val_enum_t enumeration;
  /// Field used if #wasmtime_component_val_t::kind is
  /// #WASMTIME_COMPONENT_KIND_OPTION
  wasmtime_component_val_t *option;
  /// Field used if #wasmtime_component_val_t::kind is
  /// #WASMTIME_COMPONENT_KIND_RESULT
  wasmtime_component_val_result_t result;
  /// Field used if #wasmtime_component_val_t::kind is
  /// #WASMTIME_COMPONENT_KIND_FLAGS
  wasmtime_component_val_flags_t flags;
} wasmtime_component_val_payload_t;

/**
 * \brief Representation of a component model value
 *
 * Many kind of values own data, so those need to be properly dropped in
 * wasmtime, and therefore should not live on the stack.
 */
typedef struct wasmtime_component_val_t {
  /// Discriminant indicating which field of #payload is valid
  wasmtime_component_kind_t kind;
  /// Container for the value data
  wasmtime_component_val_payload_t payload;
} wasmtime_component_val_t;

WASMTIME_COMPONENT_DECLARE_VEC_NEW(val_vec, wasmtime_component_val_t);

/// Representation of record field.
typedef struct wasmtime_component_val_record_field_t {
  /// Name of the field
  wasm_name_t name;
  /// Value of the field
  wasmtime_component_val_t val;
} wasmtime_component_val_record_field_t;

WASMTIME_COMPONENT_DECLARE_VEC_NEW(val_record,
                                   wasmtime_component_val_record_field_t);

/**
 * \brief Sets the value of a flag within within a
 #wasmtime_component_val_flags_t.
 *
 * If this bit set is too small to hold a value at `index` it will be resized.
 *
 * \param flags the #wasmtime_component_val_flags_t to modify
 * \param index the index of the flag to modify
 * \param enabled the value to set the flag to
 */
void wasmtime_component_val_flags_set(wasmtime_component_val_flags_t *flags,
                                      uint32_t index, bool enabled);

/**
 * \brief Tests the value of a flag within within a
 #wasmtime_component_val_flags_t.
 *
 * If this bit set is too small to hold a value at `index` it will be resized.
 *
 * \param flags the #wasmtime_component_val_flags_t to test
 * \param index the index of the flag to test
 *
 * \return true if the flag is set, else false
 */
bool wasmtime_component_val_flags_test(
    const wasmtime_component_val_flags_t *flags, uint32_t index);

/**
 * \brief Creates a new #wasmtime_component_val_t
 *
 * This is usually used to create inner values, typically as part of an option
 * or result. In this case, ownership is given out to the outer value.
 *
 * In case where a top level #wasmtime_component_val_t is created (for example
 * to be passed directly to #wasmtime_component_func_call), then it should be
 * deleted with #wasmtime_component_val_delete
 *
 * \return a pointer to the newly created #wasmtime_component_val_t
 */
wasmtime_component_val_t *wasmtime_component_val_new();

/**
 * \brief Deletes a #wasmtime_component_val_t previously created by
 * #wasmtime_component_val_new
 *
 * This should not be called if the value has been given out as part of an outer
 * #wasmtime_component_val_t
 *
 * \param val the #wasmtime_component_val_t to delete
 */
void wasmtime_component_val_delete(wasmtime_component_val_t *val);

/// Representation of a field in a record type or a case in a variant type
typedef struct wasmtime_component_type_field_t {
  /// Name of the record field or variant case
  wasm_name_t name;
  /// Type of the record field or variant case (may be null for variant case)
  wasmtime_component_type_t *ty;
} wasmtime_component_type_field_t;

WASMTIME_COMPONENT_DECLARE_VEC_NEW(type_field_vec,
                                   wasmtime_component_type_field_t);

/// Representation of a result type
typedef struct wasmtime_component_type_result_t {
  /// Type of the ok value (if there is one)
  wasmtime_component_type_t *ok_ty;
  /// Type of the error value (if there is one)
  wasmtime_component_type_t *err_ty;
} wasmtime_component_type_result_t;

/// Container for different kind of component model type data
typedef union wasmtime_component_type_payload_t {
  /// Field used if #wasmtime_component_type_t::kind is
  /// #WASMTIME_COMPONENT_KIND_LIST
  wasmtime_component_type_t *list;
  /// Field used if #wasmtime_component_type_t::kind is
  /// #WASMTIME_COMPONENT_KIND_RECORD
  wasmtime_component_type_field_vec_t record;
  /// Field used if #wasmtime_component_type_t::kind is
  /// #WASMTIME_COMPONENT_KIND_TUPLE
  wasmtime_component_type_vec_t tuple;
  /// Field used if #wasmtime_component_type_t::kind is
  /// #WASMTIME_COMPONENT_KIND_VARIANT
  wasmtime_component_type_field_vec_t variant;
  /// Field used if #wasmtime_component_type_t::kind is
  /// #WASMTIME_COMPONENT_KIND_ENUM
  wasmtime_component_string_vec_t enumeration;
  /// Field used if #wasmtime_component_type_t::kind is
  /// #WASMTIME_COMPONENT_KIND_OPTION
  wasmtime_component_type_t *option;
  /// Field used if #wasmtime_component_type_t::kind is
  /// #WASMTIME_COMPONENT_KIND_RESULT
  wasmtime_component_type_result_t result;
  /// Field used if #wasmtime_component_type_t::kind is
  /// #WASMTIME_COMPONENT_KIND_FLAGS
  wasmtime_component_string_vec_t flags;
} wasmtime_component_type_payload_t;

/**
 * \brief Representation of a component model type
 *
 * Many kind of types own data, so those need to be properly dropped in
 * wasmtime, and therefore should not live on the stack.
 */
typedef struct wasmtime_component_type_t {
  /// Discriminant indicating what kind of type it is, and which field of
  /// #payload is valid, if any
  wasmtime_component_kind_t kind;
  /// Container for the type data, if any
  wasmtime_component_type_payload_t payload;
} wasmtime_component_type_t;

WASMTIME_COMPONENT_DECLARE_VEC_NEW(type_vec, wasmtime_component_type_t);

#undef WASMTIME_COMPONENT_DECLARE_VEC_NEW

/**
 * \brief Creates a new #wasmtime_component_type_t
 *
 * This is usually used to create inner types, typically as part of an option or
 * result. In this case, ownership is given out to the outer value.
 *
 * In case where a top level #wasmtime_component_type_t is created (for example
 * to be passed directly to #wasmtime_component_linker_define_func), then it
 * should be deleted with #wasmtime_component_type_delete
 *
 * \return a pointer to the newly created #wasmtime_component_type_t
 */
wasmtime_component_type_t *wasmtime_component_type_new();

/**
 * \brief Deletes a #wasmtime_component_type_t previously created by
 * #wasmtime_component_type_new
 *
 * This should not be called if the type has been given out as part of an outer
 * #wasmtime_component_type_t
 *
 * \param val the #wasmtime_component_type_t to delete
 */
void wasmtime_component_type_delete(wasmtime_component_type_t *ty);

/// Representation of a component in the component model.
typedef struct wasmtime_component_t wasmtime_component_t;

/**
 * \brief Compiles a WebAssembly binary into a #wasmtime_component_t
 *
 * This function will compile a WebAssembly binary into an owned
 #wasmtime_component_t.
 *
 * It requires a component binary, such as what is produced by Rust `cargo
 component` tooling.
 *
 * This function does not take ownership of any of its arguments, but the
 * returned error and component are owned by the caller.

 * \param engine the #wasm_engine_t that will create the component
 * \param buf the address of the buffer containing the WebAssembly binary
 * \param len the length of the buffer containing the WebAssembly binary
 * \param component_out on success, contains the address of the created
 *        component
 *
 * \return NULL on success, else a #wasmtime_error_t describing the error
 */
wasmtime_error_t *
wasmtime_component_from_binary(const wasm_engine_t *engine, const uint8_t *buf,
                               size_t len,
                               wasmtime_component_t **component_out);

/**
 * \brief Deletes a #wasmtime_component_t created by
 * #wasmtime_component_from_binary
 *
 * \param component the component to delete
 */
void wasmtime_component_delete(wasmtime_component_t *component);

/**
 * \brief Representation of a component linker
 *
 * This type corresponds to a `wasmtime::component::Linker`.
 *
 * Due to the interaction between `wasmtime::component::Linker` and
 * `wasmtime::component::LinkerInstance`, the latter being more of a builder,
 * it is expected to first define the host functions through calls to
 * #wasmtime_component_linker_define_func, then call
 * #wasmtime_component_linker_build to create and populate the root
 * `wasmtime::component::LinkerInstance` and it's descendants (if any).
 */
typedef struct wasmtime_component_linker_t wasmtime_component_linker_t;

/**
 * \brief Creates a new #wasmtime_component_linker_t for the specified engine.
 *
 * \param engine the compilation environment and configuration
 *
 * \return a pointer to the newly created #wasmtime_component_linker_t
 */
wasmtime_component_linker_t *
wasmtime_component_linker_new(const wasm_engine_t *engine);

/**
 * \brief Deletes a #wasmtime_component_linker_t created by
 * #wasmtime_component_linker_new
 *
 * \param linker the #wasmtime_component_linker_t to delete
 */
void wasmtime_component_linker_delete(wasmtime_component_linker_t *linker);

/// Representation of a component instance
typedef struct wasmtime_component_instance_t wasmtime_component_instance_t;

// declaration from store.h
typedef struct wasmtime_context wasmtime_context_t;

/**
 * \brief Callback signature for #wasmtime_component_linker_define_func.
 *
 * This is the function signature for host functions that can be made accessible
 * to WebAssembly components. The arguments to this function are:
 *
 * \param env a user-provided argument passed to
 *        #wasmtime_component_linker_define_func
 * \param context a #wasmtime_context_t, the context of this call
 * \param args the arguments provided to this function invocation
 * \param nargs how many arguments are provided
 * \param results where to write the results of this function
 * \param nresults how many results must be produced
 *
 * Callbacks are guaranteed to get called with the right types of arguments, but
 * they must produce the correct number and types of results. Failure to do so
 * will cause traps to get raised on the wasm side.
 *
 * This callback can optionally return a #wasm_trap_t indicating that a trap
 * should be raised in WebAssembly. It's expected that in this case the caller
 * relinquishes ownership of the trap and it is passed back to the engine.
 */
typedef wasm_trap_t *(*wasmtime_component_func_callback_t)(
    void *env, wasmtime_context_t *context,
    const wasmtime_component_val_t *args, size_t nargs,
    wasmtime_component_val_t *results, size_t nresults);

/**
 * \brief Defines a host function in a linker
 *
 * Must be done before calling #wasmtime_component_linker_build
 *
 * Does not take ownership of the #wasmtime_component_type_t arguments.
 *
 * \param linker the #wasmtime_component_linker_t in which the function should
 *        be defined
 * \param path the dot-separated path of the package where the function is
 *        defined
 * \param path_len the byte length of `path`
 * \param name the name of the function
 * \param name_len the byte length of `name`
 * \param params_types_buf a pointer to an array of #wasmtime_component_type_t
 *        describing the function's parameters
 * \param params_types_len the length of `params_types_buf`
 * \param outputs_types_buf a pointer to an array of #wasmtime_component_type_t
 *        describing the function's outputs
 * \param outputs_types_len the length of `outputs_types_buf`
 * \param cb a pointer to the actual functions, a
 *        #wasmtime_component_func_callback_t
 * \param data the host-provided data to provide as the first argument to the
 *        callback
 * \param finalizer an optional finalizer for the `data` argument.
 *
 * \return wasmtime_error_t* on success `NULL` is returned, otherwise an error
 *         is returned which describes why the definition failed.
 */
wasmtime_error_t *wasmtime_component_linker_define_func(
    wasmtime_component_linker_t *linker, const char *path, size_t path_len,
    const char *name, size_t name_len,
    wasmtime_component_type_t *params_types_buf, size_t params_types_len,
    wasmtime_component_type_t *outputs_types_buf, size_t outputs_types_len,
    wasmtime_component_func_callback_t cb, void *data,
    void (*finalizer)(void *));

/**
 * \brief Builds the linker, providing the host functions defined by calls to
 * #wasmtime_component_linker_define_func
 *
 * \param linker the #wasmtime_component_linker_t to build
 *
 * \return wasmtime_error_t* On success `NULL` is returned, otherwise an error
 *         is returned which describes why the build failed.
 */
wasmtime_error_t *
wasmtime_component_linker_build(wasmtime_component_linker_t *linker);

/**
 * \brief Instantiates a component instance in a given #wasmtime_context_t
 *
 * \param linker a #wasmtime_component_linker_t that will help provide host
 *        functions
 * \param context the #wasmtime_context_t in which the instance should be
 *        created
 * \param component the #wasmtime_component_t to instantiate
 * \param instance_out on success, the instantiated
 *        #wasmtime_component_instance_t
 *
 * \return wasmtime_error_t* on success `NULL` is returned, otherwise an error
 *         is returned which describes why the build failed.
 */
wasmtime_error_t *wasmtime_component_linker_instantiate(
    const wasmtime_component_linker_t *linker, wasmtime_context_t *context,
    const wasmtime_component_t *component,
    wasmtime_component_instance_t **instance_out);

/// Representation of an exported function in Wasmtime component model.
typedef struct wasmtime_component_func_t wasmtime_component_func_t;

/**
 * \brief Looks for an exported function in the given component instance
 *
 * \param instance the #wasmtime_component_instance_t in which the function
 *        should be looked for
 * \param context the #wasmtime_context_t that contains `instance`
 * \param name the name of function to look for
 * \param name_len the byte length of `name`
 * \param item_out the wasmtime_component_func_t that was found, if any
 *
 * \return true if the function was found, else false
 */
bool wasmtime_component_instance_get_func(
    const wasmtime_component_instance_t *instance, wasmtime_context_t *context,
    const char *name, size_t name_len, wasmtime_component_func_t **item_out);

/**
 * \brief Calls an exported function of a component
 *
 * It is the responsibility of the caller to make sure that `params` has the
 * expected length, and the correct types, else the call will error out.
 * `results` must have the expected length, but the values will be written with
 * the correct types.
 *
 * This can fail in two ways : either a non-NULL #wasmtime_error_t is returned,
 * for example if the parameters are incorrect (and `trap_out` will be NULL), or
 * the call may trap, in which case NULL is returned, but `trap_out` will be
 * non-NULL.
 *
 * Does not take ownership of #wasmtime_component_val_t arguments. Gives
 * ownership of #wasmtime_component_val_t results. As such, if those are
 * data-owning values, they should be created and deleted through this api,
 * either directly with #wasmtime_component_val_new, or through a
 * #wasmtime_component_val_vec_t, using #wasmtime_component_val_vec_new and
 * #wasmtime_component_val_vec_delete
 *
 * \param func the function to call, typically found with
 *        #wasmtime_component_instance_get_func
 * \param context the #wasmtime_context_t that contains `func`
 * \param params the parameters of `func`, as an array of
 *        #wasmtime_component_val_t
 * \param params_len the length of `params`
 * \param results the results of `func`, as an array of
 *        #wasmtime_component_val_t that will be written
 * \param results_len the length of `results`
 * \param trap_out NULL if the call completed successfully or couldn't be made,
 *        otherwise the trap that was raised
 *
 * \return wasmtime_error_t* NULL on success or a description of the error
 *         calling the function
 */
wasmtime_error_t *wasmtime_component_func_call(
    const wasmtime_component_func_t *func, wasmtime_context_t *context,
    const wasmtime_component_val_t *params, size_t params_len,
    wasmtime_component_val_t *results, size_t results_len,
    wasm_trap_t **trap_out);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_COMPONENT_H
