/**
 * \file wasm.h
 *
 * Embedding API for WebAssembly.
 *
 * This API is defined by the upstream wasm-c-api proposal at
 * https://github.com/WebAssembly/wasm-c-api. That proposal is in flux but
 * Wasmtime intends to be active in its development.
 *
 * The documentation for this header file is currently defined in the Wasmtime
 * project, not in the upstream header file. Some behavior here may be
 * Wasmtime-specific and may not be portable to other engines implementing the
 * same C API. Also note that not all functionality from the upstream C API is
 * implemented in Wasmtime. We strive to provide all symbols as to not generate
 * link errors but some functions are unimplemented and will abort the process
 * if called.
 *
 * ### Memory Management
 *
 * Memory management in the wasm C API is intended to be relatively simple. Each
 * individual object is reference counted unless otherwise noted. You can delete
 * any object at any time after you no longer need it. Deletion of an object
 * does not imply that the memory will be deallocated at that time. If another
 * object still internally references the original object then the memory will
 * still be alive.
 *
 * For example you can delete a #wasm_engine_t after you create a #wasm_store_t
 * with #wasm_store_new. The engine, however, is still referenced by the
 * #wasm_store_t so it will not be deallocated. In essence by calling
 * #wasm_engine_delete you're release your own strong reference on the
 * #wasm_engine_t, but that's it.
 *
 * Additionally APIs like #wasm_memory_copy do not actually copy the underlying
 * data. Instead they only increment the reference count and return a new
 * object. You'll need to still call #wasm_memory_delete (or the corresponding
 * `*_delete` function) for each copy of an object you acquire.
 *
 * ### Thread Safety
 *
 * All objects are not thread safe unless otherwise noted. This means that all
 * objects, once created, cannot be used simultaneously on more than one thread.
 *
 * In general the split happens at the #wasm_store_t layer. Everything within a
 * #wasm_store_t is pinned to that store, and that store is pinned to a thread.
 * Objects like #wasm_config_t and #wasm_engine_t, however, are safe to share
 * across threads.
 *
 * Note that if you move a #wasm_store_t between threads this is ok so long as
 * you move *all* references within that store to the new thread. If any
 * reference lingers on the previous thread then that is unsafe.
 */

/**
 * \typedef byte_t
 * \headerfile wasm.h
 * \brief A type definition for a number that occupies a single byte of data.
 */

/**
 * \typedef wasm_byte_t
 * \headerfile wasm.h
 * \brief A type definition for a number that occupies a single byte of data.
 */

/**
 * \typedef float32_t
 * \headerfile wasm.h
 * \brief A type definition for a 32-bit float.
 */

/**
 * \typedef float64_t
 * \headerfile wasm.h
 * \brief A type definition for a 64-bit float.
 */

/**
 * \typedef wasm_name_t
 * \headerfile wasm.h
 * \brief Convenience for hinting that an argument only accepts utf-8 input.
 */

/**
 * \typedef wasm_config_t
 * \headerfile wasm.h
 * \brief Convenience alias for #wasm_config_t
 */

/**
 * \struct wasm_config_t
 * \headerfile wasm.h
 * \brief Global engine configuration
 *
 * This structure represents global configuration used when constructing a
 * #wasm_engine_t. There are now functions to modify this from wasm.h but the
 * wasmtime.h header provides a number of Wasmtime-specific functions to
 * tweak configuration options.
 *
 * This object is created with #wasm_config_new.
 *
 * Configuration is safe to share between threads. Typically you'll create a
 * config object and immediately pass it into #wasm_engine_new_with_config,
 * however.
 */

/**
 * \fn own wasm_config_t *wasm_config_new(void);
 * \brief Creates a new empty configuration object.
 *
 * The object returned is owned by the caller and will need to be deleted with
 * #wasm_config_delete. May return `NULL` if a configuration object could not be
 * allocated.
 */

/**
 * \fn void wasm_config_delete(own wasm_config_t*);
 * \brief Deletes a configuration object.
 */

/**
 * \typedef wasm_engine_t
 * \headerfile wasm.h
 * \brief Convenience alias for #wasm_engine_t
 */

/**
 * \struct wasm_engine_t
 * \headerfile wasm.h
 * \brief Typically global object to create #wasm_store_t from.
 *
 * An engine is typically global in a program and contains all the configuration
 * necessary for compiling wasm code. From an engine you'll typically create a
 * #wasm_store_t. Engines are created with #wasm_engine_new or
 * #wasm_engine_new_with_config.
 *
 * An engine is safe to share between threads. Multiple stores can be created
 * within the same engine with each store living on a separate thread. Typically
 * you'll create one #wasm_engine_t for the lifetime of your program.
 */

/**
 * \fn own wasm_engine_t *wasm_engine_new(void);
 * \brief Creates a new engine with the default configuration.
 *
 * The object returned is owned by the caller and will need to be deleted with
 * #wasm_engine_delete. This may return `NULL` if the engine could not be
 * allocated.
 */

/**
 * \fn own wasm_engine_t *wasm_engine_new_with_config(wasm_config_t *);
 * \brief Creates a new engine with the specified configuration.
 *
 * This function will take ownership of the configuration specified regardless
 * of the outcome of this function. You do not need to call #wasm_config_delete
 * on the argument. The object returned is owned by the caller and will need to
 * be deleted with #wasm_engine_delete. This may return `NULL` if the engine
 * could not be allocated.
 */

/**
 * \fn void wasm_engine_delete(own wasm_engine_t*);
 * \brief Deletes an engine.
 */

/**
 * \typedef wasm_store_t
 * \headerfile wasm.h
 * \brief Convenience alias for #wasm_store_t
 */

/**
 * \struct wasm_store_t
 * \headerfile wasm.h
 * \brief A collection of instances and wasm global items.
 *
 * A #wasm_store_t corresponds to the concept of an [embedding
 * store](https://webassembly.github.io/spec/core/exec/runtime.html#store)
 */

/**
 * \fn own wasm_store_t *wasm_store_new(wasm_engine_t *);
 * \brief Creates a new store within the specified engine.
 *
 * The object returned is owned by the caller and will need to be deleted with
 * #wasm_store_delete. This may return `NULL` if the store could not be
 * allocated.
 */

/**
 * \fn void wasm_store_delete(own wasm_store_t *);
 * \brief Deletes the specified store.
 */

/**
 * \struct wasm_byte_vec_t
 * \headerfile wasm.h
 * \brief A list of bytes
 *
 * Used to pass data in or pass data out of various functions.  The meaning and
 * ownership of the bytes is defined by each API that operates on this
 * datatype.
 */

/**
 * \typedef wasm_byte_vec_t
 * \headerfile wasm.h
 * \brief Convenience alias for #wasm_byte_vec_t
 */

/**
 * \fn void wasm_byte_vec_new_empty(own wasm_byte_vec_t *out);
 * \brief Initializes an empty byte vector.
 */

/**
 * \fn void wasm_byte_vec_new_uninitialized(own wasm_byte_vec_t *out, size_t);
 * \brief Initializes an byte vector with the specified capacity.
 *
 * This function will initialize the provided vector with capacity to hold the
 * specified number of bytes. The `out` parameter must previously not already be
 * initialized and after this function is called you are then responsible for
 * ensuring #wasm_byte_vec_delete is called.
 */

/**
 * \fn void wasm_byte_vec_new(own wasm_byte_vec_t *out, size_t, own wasm_byte_t const[]);
 * \brief Copies the specified data into a new byte vector.
 *
 * This function will copy the provided data into this byte vector. The byte
 * vector should not be previously initialized and the caller is responsible for
 * calling #wasm_byte_vec_delete after this function returns.
 */

/**
 * \fn void wasm_byte_vec_copy(own wasm_byte_vec_t *out, const wasm_byte_vec_t *);
 * \brief Copies one vector into a new vector.
 *
 * Copies the second argument's data into the first argument. The `out` vector
 * should not be previously initialized and after this function returns you're
 * responsible for calling #wasm_byte_vec_delete.
 */

/**
 * \fn void wasm_byte_vec_delete(own wasm_byte_vec_t *);
 * \brief Deletes a byte vector.
 *
 * This function will deallocate the data referenced by the argument provided.
 * This does not deallocate the memory holding the #wasm_byte_vec_t itself, it's
 * expected that memory is owned by the caller.
 */

/**
 * \fn own wasm_memory_t *wasm_memory_copy(const wasm_memory_t *);
 * \brief Creates a new reference to the same memory.
 *
 * The object returned is owned by the caller and will need to be deleted with
 * #wasm_memory_delete. This may return `NULL` if the new object could not be
 * allocated.
 */

/**
 * \fn void wasm_memory_delete(own wasm_memory_t*);
 * \brief Deletes a memory object.
 */
