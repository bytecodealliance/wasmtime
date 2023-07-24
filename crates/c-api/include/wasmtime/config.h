/**
 * \file wasmtime/config.h
 *
 * \brief Wasmtime-specific extensions to #wasm_config_t
 */

#ifndef WASMTIME_CONFIG_H
#define WASMTIME_CONFIG_H

#include <wasm.h>
#include <wasmtime/error.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * \brief Specifier for how Wasmtime will compile code, values are in
 * #wasmtime_strategy_enum
 */
typedef uint8_t wasmtime_strategy_t;

/**
 * \brief Different ways that Wasmtime can compile WebAssembly
 *
 * The default value is #WASMTIME_STRATEGY_AUTO.
 */
enum wasmtime_strategy_enum { // Strategy
  /// Automatically picks the compilation backend, currently always defaulting
  /// to Cranelift.
  WASMTIME_STRATEGY_AUTO,

  /// Indicates that Wasmtime will unconditionally use Cranelift to compile
  /// WebAssembly code.
  WASMTIME_STRATEGY_CRANELIFT,
};

/**
 * \brief Specifier of what optimization level to use for generated JIT code.
 *
 * See #wasmtime_opt_level_enum for possible values.
 */
typedef uint8_t wasmtime_opt_level_t;

/**
 * \brief Different ways Wasmtime can optimize generated code.
 *
 * The default value is #WASMTIME_OPT_LEVEL_SPEED.
 */
enum wasmtime_opt_level_enum { // OptLevel
  /// Generated code will not be optimized at all.
  WASMTIME_OPT_LEVEL_NONE,
  /// Generated code will be optimized purely for speed.
  WASMTIME_OPT_LEVEL_SPEED,
  /// Generated code will be optimized, but some speed optimizations are
  /// disabled if they cause the generated code to be significantly larger.
  WASMTIME_OPT_LEVEL_SPEED_AND_SIZE,
};

/**
 * \brief Different ways wasmtime can enable profiling JIT code.
 *
 * See #wasmtime_profiling_strategy_enum for possible values.
 */
typedef uint8_t wasmtime_profiling_strategy_t;

/**
 * \brief Different ways to profile JIT code.
 *
 * The default is #WASMTIME_PROFILING_STRATEGY_NONE.
 */
enum wasmtime_profiling_strategy_enum { // ProfilingStrategy
  /// No profiling is enabled at runtime.
  WASMTIME_PROFILING_STRATEGY_NONE,
  /// Linux's "jitdump" support in `perf` is enabled and when Wasmtime is run
  /// under `perf` necessary calls will be made to profile generated JIT code.
  WASMTIME_PROFILING_STRATEGY_JITDUMP,
  /// Support for VTune will be enabled and the VTune runtime will be informed,
  /// at runtime, about JIT code.
  ///
  /// Note that this isn't always enabled at build time.
  WASMTIME_PROFILING_STRATEGY_VTUNE,
  /// Linux's simple "perfmap" support in `perf` is enabled and when Wasmtime is
  /// run under `perf` necessary calls will be made to profile generated JIT
  /// code.
  WASMTIME_PROFILING_STRATEGY_PERFMAP,
};

#define WASMTIME_CONFIG_PROP(ret, name, ty) \
    WASM_API_EXTERN ret wasmtime_config_##name##_set(wasm_config_t*, ty);

/**
 * \brief Configures whether DWARF debug information is constructed at runtime
 * to describe JIT code.
 *
 * This setting is `false` by default. When enabled it will attempt to inform
 * native debuggers about DWARF debugging information for JIT code to more
 * easily debug compiled WebAssembly via native debuggers. This can also
 * sometimes improve the quality of output when profiling is enabled.
 */
WASMTIME_CONFIG_PROP(void, debug_info, bool)

/**
 * \brief Whether or not fuel is enabled for generated code.
 *
 * This setting is `false` by default. When enabled it will enable fuel counting
 * meaning that fuel will be consumed every time a wasm instruction is executed,
 * and trap when reaching zero.
 */
WASMTIME_CONFIG_PROP(void, consume_fuel, bool)

/**
 * \brief Whether or not epoch-based interruption is enabled for generated code.
 *
 * This setting is `false` by default. When enabled wasm code will check the
 * current epoch periodically and abort if the current epoch is beyond a
 * store-configured limit.
 *
 * Note that when this setting is enabled all stores will immediately trap and
 * need to have their epoch deadline otherwise configured with
 * #wasmtime_context_set_epoch_deadline.
 *
 * Note that the current epoch is engine-local and can be incremented with
 * #wasmtime_engine_increment_epoch.
 */
WASMTIME_CONFIG_PROP(void, epoch_interruption, bool)

/**
 * \brief Configures the maximum amount of stack space available, in bytes,
 * for executing WebAssembly code.
 *
 * WebAssembly has well-defined semantics on stack overflow. This is
 * intended to be a knob which can help configure how much stack space
 * wasm execution is allowed to consume. Note that the number here is not
 * super-precise, but rather wasm will take at most "pretty close to this
 * much" stack space.
 *
 * If a wasm call (or series of nested wasm calls) take more stack space
 * than the `size` specified then a stack overflow trap will be raised.
 *
 * Caveat: this knob only limits the stack space consumed by wasm code.
 * More importantly, it does not ensure that this much stack space is
 * available on the calling thread stack. Exhausting the thread stack
 * typically leads to an **abort** of the process.
 *
 * Here are some examples of how that could happen:
 *
 * - Let's assume this option is set to 2 MiB and then a thread that has
 *   a stack with 512 KiB left.
 *
 *   If wasm code consumes more than 512 KiB then the process will be aborted.
 *
 * - Assuming the same conditions, but this time wasm code does not consume
 *   any stack but calls into a host function. The host function consumes
 *   more than 512 KiB of stack space. The process will be aborted.
 *
 * There's another gotcha related to recursive calling into wasm: the stack
 * space consumed by a host function is counted towards this limit. The
 * host functions are not prevented from consuming more than this limit.
 * However, if the host function that used more than this limit and called
 * back into wasm, then the execution will trap immediatelly because of
 * stack overflow.
 *
 * When the `async` feature is enabled, this value cannot exceed the
 * `async_stack_size` option. Be careful not to set this value too close
 * to `async_stack_size` as doing so may limit how much stack space
 * is available for host functions.
 *
 * By default this option is 512 KiB.
 */
WASMTIME_CONFIG_PROP(void, max_wasm_stack, size_t)

/**
 * \brief Configures the size of the stacks, in bytes, used for asynchronous
 * execution.
 *
 * This setting configures the size of the stacks that are allocated for
 * asynchronous execution. The value cannot be less than `max_wasm_stack`.
 *
 * The amount of stack space guaranteed for host functions is
 * `async_stack_size - max_wasm_stack`, so take care not to set these two
 * values close to one another; doing so may cause host functions to overflow
 * the stack and abort the process.
 *
 * By default this option is 2 MiB.
 */
WASMTIME_CONFIG_PROP(void, async_stack_size, size_t)

/**
 * \brief Configures whether the WebAssembly threading proposal is enabled.
 *
 * This setting is `false` by default.
 *
 * Note that threads are largely unimplemented in Wasmtime at this time.
 */
WASMTIME_CONFIG_PROP(void, wasm_threads, bool)

/**
 * \brief Configures whether the WebAssembly reference types proposal is
 * enabled.
 *
 * This setting is `false` by default.
 */
WASMTIME_CONFIG_PROP(void, wasm_reference_types, bool)

/**
 * \brief Configures whether the WebAssembly SIMD proposal is
 * enabled.
 *
 * This setting is `false` by default.
 */
WASMTIME_CONFIG_PROP(void, wasm_simd, bool)

/**
 * \brief Configures whether the WebAssembly relaxed SIMD proposal is
 * enabled.
 *
 * This setting is `false` by default.
 */
WASMTIME_CONFIG_PROP(void, wasm_relaxed_simd, bool)

/**
 * \brief Configures whether the WebAssembly relaxed SIMD proposal is
 * in deterministic mode.
 *
 * This setting is `false` by default.
 */
WASMTIME_CONFIG_PROP(void, wasm_relaxed_simd_deterministic, bool)

/**
 * \brief Configures whether the WebAssembly bulk memory proposal is
 * enabled.
 *
 * This setting is `false` by default.
 */
WASMTIME_CONFIG_PROP(void, wasm_bulk_memory, bool)

/**
 * \brief Configures whether the WebAssembly multi value proposal is
 * enabled.
 *
 * This setting is `true` by default.
 */
WASMTIME_CONFIG_PROP(void, wasm_multi_value, bool)

/**
 * \brief Configures whether the WebAssembly multi-memory proposal is
 * enabled.
 *
 * This setting is `false` by default.
 */
WASMTIME_CONFIG_PROP(void, wasm_multi_memory, bool)

/**
 * \brief Configures whether the WebAssembly memory64 proposal is
 * enabled.
 *
 * This setting is `false` by default.
 */
WASMTIME_CONFIG_PROP(void, wasm_memory64, bool)

/**
 * \brief Configures how JIT code will be compiled.
 *
 * This setting is #WASMTIME_STRATEGY_AUTO by default.
 */
WASMTIME_CONFIG_PROP(void, strategy, wasmtime_strategy_t)

/**
 * \brief Configure whether wasmtime should compile a module using multiple threads.
 *
 * For more information see the Rust documentation at
 * https://docs.wasmtime.dev/api/wasmtime/struct.Config.html#method.parallel_compilation.
 */
WASMTIME_CONFIG_PROP(void, parallel_compilation, bool)

/**
 * \brief Configures whether Cranelift's debug verifier is enabled.
 *
 * This setting in `false` by default.
 *
 * When cranelift is used for compilation this enables expensive debug checks
 * within Cranelift itself to verify it's correct.
 */
WASMTIME_CONFIG_PROP(void, cranelift_debug_verifier, bool)

/**
 * \brief Configures whether Cranelift should perform a NaN-canonicalization pass.
 *
 * When Cranelift is used as a code generation backend this will configure
 * it to replace NaNs with a single canonical value. This is useful for users
 * requiring entirely deterministic WebAssembly computation.
 * 
 * This is not required by the WebAssembly spec, so it is not enabled by default.
 * 
 * The default value for this is `false`
 */
WASMTIME_CONFIG_PROP(void, cranelift_nan_canonicalization, bool)
  
/**
 * \brief Configures Cranelift's optimization level for JIT code.
 *
 * This setting in #WASMTIME_OPT_LEVEL_SPEED by default.
 */
WASMTIME_CONFIG_PROP(void, cranelift_opt_level, wasmtime_opt_level_t)

/**
 * \brief Configures the profiling strategy used for JIT code.
 *
 * This setting in #WASMTIME_PROFILING_STRATEGY_NONE by default.
 */
WASMTIME_CONFIG_PROP(void, profiler, wasmtime_profiling_strategy_t)

/**
 * \brief Configures the “static” style of memory to always be used.
 *
 * This setting is `false` by default.
 *
 * For more information see the Rust documentation at
 * https://bytecodealliance.github.io/wasmtime/api/wasmtime/struct.Config.html#method.static_memory_forced.
 */
WASMTIME_CONFIG_PROP(void, static_memory_forced, bool)

/**
 * \brief Configures the maximum size for memory to be considered "static"
 *
 * For more information see the Rust documentation at
 * https://bytecodealliance.github.io/wasmtime/api/wasmtime/struct.Config.html#method.static_memory_maximum_size.
 */
WASMTIME_CONFIG_PROP(void, static_memory_maximum_size, uint64_t)

/**
 * \brief Configures the guard region size for "static" memory.
 *
 * For more information see the Rust documentation at
 * https://bytecodealliance.github.io/wasmtime/api/wasmtime/struct.Config.html#method.static_memory_guard_size.
 */
WASMTIME_CONFIG_PROP(void, static_memory_guard_size, uint64_t)

/**
 * \brief Configures the guard region size for "dynamic" memory.
 *
 * For more information see the Rust documentation at
 * https://bytecodealliance.github.io/wasmtime/api/wasmtime/struct.Config.html#method.dynamic_memory_guard_size.
 */
WASMTIME_CONFIG_PROP(void, dynamic_memory_guard_size, uint64_t)

/**
 * \brief Enables Wasmtime's cache and loads configuration from the specified
 * path.
 *
 * By default the Wasmtime compilation cache is disabled. The configuration path
 * here can be `NULL` to use the default settings, and otherwise the argument
 * here must be a file on the filesystem with TOML configuration -
 * https://bytecodealliance.github.io/wasmtime/cli-cache.html.
 *
 * An error is returned if the cache configuration could not be loaded or if the
 * cache could not be enabled.
 */
WASM_API_EXTERN wasmtime_error_t* wasmtime_config_cache_config_load(wasm_config_t*, const char*);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif // WASMTIME_CONFIG_H

