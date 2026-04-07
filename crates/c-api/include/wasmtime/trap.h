/**
 * \file wasmtime/trap.h
 *
 * Wasmtime APIs for interacting with traps and extensions to #wasm_trap_t.
 */

#ifndef WASMTIME_TRAP_H
#define WASMTIME_TRAP_H

#include <wasm.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * \brief Code of an instruction trap.
 *
 * See #wasmtime_trap_code_enum for possible values.
 */
typedef uint8_t wasmtime_trap_code_t;

/**
 * \brief Trap codes for instruction traps.
 */
enum wasmtime_trap_code_enum {
  /// The current stack space was exhausted.
  WASMTIME_TRAP_CODE_STACK_OVERFLOW = 0,
  /// An out-of-bounds memory access.
  WASMTIME_TRAP_CODE_MEMORY_OUT_OF_BOUNDS = 1,
  /// A wasm atomic operation was presented with a not-naturally-aligned
  /// linear-memory address.
  WASMTIME_TRAP_CODE_HEAP_MISALIGNED = 2,
  /// An out-of-bounds access to a table.
  WASMTIME_TRAP_CODE_TABLE_OUT_OF_BOUNDS = 3,
  /// Indirect call to a null table entry.
  WASMTIME_TRAP_CODE_INDIRECT_CALL_TO_NULL = 4,
  /// Signature mismatch on indirect call.
  WASMTIME_TRAP_CODE_BAD_SIGNATURE = 5,
  /// An integer arithmetic operation caused an overflow.
  WASMTIME_TRAP_CODE_INTEGER_OVERFLOW = 6,
  /// An integer division by zero.
  WASMTIME_TRAP_CODE_INTEGER_DIVISION_BY_ZERO = 7,
  /// Failed float-to-int conversion.
  WASMTIME_TRAP_CODE_BAD_CONVERSION_TO_INTEGER = 8,
  /// Code that was supposed to have been unreachable was reached.
  WASMTIME_TRAP_CODE_UNREACHABLE_CODE_REACHED = 9,
  /// Execution has potentially run too long and may be interrupted.
  WASMTIME_TRAP_CODE_INTERRUPT = 10,
  /// Execution has run out of the configured fuel amount.
  WASMTIME_TRAP_CODE_OUT_OF_FUEL = 11,
  /// Used to indicate that a trap was raised by atomic wait operations on non
  /// shared memory.
  WASMTIME_TRAP_CODE_ATOMIC_WAIT_NON_SHARED_MEMORY = 12,
  /// Call to a null reference.
  WASMTIME_TRAP_CODE_NULL_REFERENCE = 13,
  /// Attempt to access beyond the bounds of an array.
  WASMTIME_TRAP_CODE_ARRAY_OUT_OF_BOUNDS = 14,
  /// Attempted an allocation that was too large to succeed.
  WASMTIME_TRAP_CODE_ALLOCATION_TOO_LARGE = 15,
  /// Attempted to cast a reference to a type that it is not an instance of.
  WASMTIME_TRAP_CODE_CAST_FAILURE = 16,
  /// When the `component-model` feature is enabled this trap represents a
  /// scenario where one component tried to call another component but it
  /// would have violated the reentrance rules of the component model,
  /// triggering a trap instead.
  WASMTIME_TRAP_CODE_CANNOT_ENTER_COMPONENT = 17,
  /// Async-lifted export failed to produce a result by calling `task.return`
  /// before returning `STATUS_DONE` and/or after all host tasks completed.
  WASMTIME_TRAP_CODE_NO_ASYNC_RESULT = 18,
  /// We are suspending to a tag for which there is no active handler.
  WASMTIME_TRAP_CODE_UNHANDLED_TAG = 19,
  /// Attempt to resume a continuation twice.
  WASMTIME_TRAP_CODE_CONTINUATION_ALREADY_CONSUMED = 20,
  /// A Pulley opcode was executed at runtime when the opcode was disabled at
  /// compile time.
  WASMTIME_TRAP_CODE_DISABLED_OPCODE = 21,
  /// Async event loop deadlocked; i.e. it cannot make further progress given
  /// that all host tasks have completed and any/all host-owned stream/future
  /// handles have been dropped.
  WASMTIME_TRAP_CODE_ASYNC_DEADLOCK = 22,
  /// When the `component-model` feature is enabled this trap represents a
  /// scenario where a component instance tried to call an import or intrinsic
  /// when it wasn't allowed to, e.g. from a post-return function.
  WASMTIME_TRAP_CODE_CANNOT_LEAVE_COMPONENT = 23,
  /// A synchronous task attempted to make a potentially blocking call prior
  /// to returning.
  WASMTIME_TRAP_CODE_CANNOT_BLOCK_SYNC_TASK = 24,
  /// A component tried to lift a `char` with an invalid bit pattern.
  WASMTIME_TRAP_CODE_INVALID_CHAR = 25,
  /// Debug assertion generated for a fused adapter regarding the expected
  /// completion of a string encoding operation.
  WASMTIME_TRAP_CODE_DEBUG_ASSERT_STRING_ENCODING_FINISHED = 26,
  /// Debug assertion generated for a fused adapter regarding a string
  /// encoding operation.
  WASMTIME_TRAP_CODE_DEBUG_ASSERT_EQUAL_CODE_UNITS = 27,
  /// Debug assertion generated for a fused adapter regarding the alignment of
  /// a pointer.
  WASMTIME_TRAP_CODE_DEBUG_ASSERT_POINTER_ALIGNED = 28,
  /// Debug assertion generated for a fused adapter regarding the upper bits
  /// of a 64-bit value.
  WASMTIME_TRAP_CODE_DEBUG_ASSERT_UPPER_BITS_UNSET = 29,
  /// A component tried to lift or lower a string past the end of its memory.
  WASMTIME_TRAP_CODE_STRING_OUT_OF_BOUNDS = 30,
  /// A component tried to lift or lower a list past the end of its memory.
  WASMTIME_TRAP_CODE_LIST_OUT_OF_BOUNDS = 31,
  /// A component used an invalid discriminant when lowering a variant value.
  WASMTIME_TRAP_CODE_INVALID_DISCRIMINANT = 32,
  /// A component passed an unaligned pointer when lifting or lowering a
  /// value.
  WASMTIME_TRAP_CODE_UNALIGNED_POINTER = 33,
  /// `task.cancel` invoked in an invalid way.
  WASMTIME_TRAP_CODE_TASK_CANCEL_NOT_CANCELLED = 34,
  /// `task.cancel` or `task.return` called too many times
  WASMTIME_TRAP_CODE_TASK_CANCEL_OR_RETURN_TWICE = 35,
  /// `subtask.cancel` invoked after it already finished.
  WASMTIME_TRAP_CODE_SUBTASK_CANCEL_AFTER_TERMINAL = 36,
  /// `task.return` invoked with an invalid type.
  WASMTIME_TRAP_CODE_TASK_RETURN_INVALID = 37,
  /// `waitable-set.drop` invoked on a waitable set with waiters.
  WASMTIME_TRAP_CODE_WAITABLE_SET_DROP_HAS_WAITERS = 38,
  /// `subtask.drop` invoked on a subtask that hasn't resolved yet.
  WASMTIME_TRAP_CODE_SUBTASK_DROP_NOT_RESOLVED = 39,
  /// `thread.new-indirect` invoked with a function that has an invalid type.
  WASMTIME_TRAP_CODE_THREAD_NEW_INDIRECT_INVALID_TYPE = 40,
  /// `thread.new-indirect` invoked with an uninitialized function reference.
  WASMTIME_TRAP_CODE_THREAD_NEW_INDIRECT_UNINITIALIZED = 41,
  /// Backpressure-related intrinsics overflowed the built-in counter.
  WASMTIME_TRAP_CODE_BACKPRESSURE_OVERFLOW = 42,
  /// Invalid code returned from `callback` of `async`-lifted function.
  WASMTIME_TRAP_CODE_UNSUPPORTED_CALLBACK_CODE = 43,
  /// Cannot resume a thread which is not suspended.
  WASMTIME_TRAP_CODE_CANNOT_RESUME_THREAD = 44,
  /// Cannot issue a read/write on a future/stream while there is a
  /// pending operation already.
  WASMTIME_TRAP_CODE_CONCURRENT_FUTURE_STREAM_OP = 45,
  /// A reference count (for e.g. an `error-context`) overflowed.
  WASMTIME_TRAP_CODE_REFERENCE_COUNT_OVERFLOW = 46,
};

/**
 * \brief Creates a new trap with the given message.
 *
 * \param msg the message to associate with this trap
 * \param msg_len the byte length of `msg`
 *
 * The #wasm_trap_t returned is owned by the caller.
 */
WASM_API_EXTERN wasm_trap_t *wasmtime_trap_new(const char *msg, size_t msg_len);

/**
 * \brief Creates a new trap from the given trap code.
 *
 * \param code the trap code to associate with this trap
 *
 * The #wasm_trap_t returned is owned by the caller.
 */
WASM_API_EXTERN wasm_trap_t *wasmtime_trap_new_code(wasmtime_trap_code_t code);

/**
 * \brief Attempts to extract the trap code from this trap.
 *
 * Returns `true` if the trap is an instruction trap triggered while
 * executing Wasm. If `true` is returned then the trap code is returned
 * through the `code` pointer. If `false` is returned then this is not
 * an instruction trap -- traps can also be created using wasm_trap_new,
 * or occur with WASI modules exiting with a certain exit code.
 */
WASM_API_EXTERN bool wasmtime_trap_code(const wasm_trap_t *,
                                        wasmtime_trap_code_t *code);

/**
 * \brief Returns a human-readable name for this frame's function.
 *
 * This function will attempt to load a human-readable name for function this
 * frame points to. This function may return `NULL`.
 *
 * The lifetime of the returned name is the same as the #wasm_frame_t itself.
 */
WASM_API_EXTERN const wasm_name_t *
wasmtime_frame_func_name(const wasm_frame_t *);

/**
 * \brief Returns a human-readable name for this frame's module.
 *
 * This function will attempt to load a human-readable name for module this
 * frame points to. This function may return `NULL`.
 *
 * The lifetime of the returned name is the same as the #wasm_frame_t itself.
 */
WASM_API_EXTERN const wasm_name_t *
wasmtime_frame_module_name(const wasm_frame_t *);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // WASMTIME_TRAP_H
