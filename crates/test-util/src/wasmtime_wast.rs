use crate::wast;
use wasmtime::Config;

/// Helper method to apply `wast_config` to `config`.
pub fn apply_wast_config(config: &mut Config, wast_config: &wast::WastConfig) {
    use wasmtime_environ::TripleExt;
    use wast::{Collector, Compiler};

    config.strategy(match wast_config.compiler {
        Compiler::CraneliftNative | Compiler::CraneliftPulley => wasmtime::Strategy::Cranelift,
        Compiler::Winch => wasmtime::Strategy::Winch,
    });
    if let Compiler::CraneliftPulley = wast_config.compiler {
        config
            .target(&target_lexicon::Triple::pulley_host().to_string())
            .unwrap();
    }
    config.collector(match wast_config.collector {
        Collector::Auto => wasmtime::Collector::Auto,
        Collector::Null => wasmtime::Collector::Null,
        Collector::DeferredReferenceCounting => wasmtime::Collector::DeferredReferenceCounting,
    });
}

/// Helper method to apply `test_config` to `config`.
pub fn apply_test_config(config: &mut Config, test_config: &wast::TestConfig) {
    let wast::TestConfig {
        memory64,
        custom_page_sizes,
        multi_memory,
        threads,
        shared_everything_threads,
        gc,
        function_references,
        relaxed_simd,
        reference_types,
        tail_call,
        extended_const,
        wide_arithmetic,
        component_model_async,
        component_model_async_builtins,
        component_model_async_stackful,
        component_model_error_context,
        component_model_gc,
        nan_canonicalization,
        simd,
        exceptions,
        legacy_exceptions,
        stack_switching,

        hogs_memory: _,
        gc_types: _,
        spec_test: _,
    } = *test_config;
    // Note that all of these proposals/features are currently default-off to
    // ensure that we annotate all tests accurately with what features they
    // need, even in the future when features are stabilized.
    let memory64 = memory64.unwrap_or(false);
    let custom_page_sizes = custom_page_sizes.unwrap_or(false);
    let multi_memory = multi_memory.unwrap_or(false);
    let threads = threads.unwrap_or(false);
    let shared_everything_threads = shared_everything_threads.unwrap_or(false);
    let gc = gc.unwrap_or(false);
    let tail_call = tail_call.unwrap_or(false);
    let extended_const = extended_const.unwrap_or(false);
    let wide_arithmetic = wide_arithmetic.unwrap_or(false);
    let component_model_async = component_model_async.unwrap_or(false);
    let component_model_async_builtins = component_model_async_builtins.unwrap_or(false);
    let component_model_async_stackful = component_model_async_stackful.unwrap_or(false);
    let component_model_error_context = component_model_error_context.unwrap_or(false);
    let component_model_gc = component_model_gc.unwrap_or(false);
    let nan_canonicalization = nan_canonicalization.unwrap_or(false);
    let relaxed_simd = relaxed_simd.unwrap_or(false);
    let legacy_exceptions = legacy_exceptions.unwrap_or(false);
    let stack_switching = stack_switching.unwrap_or(false);

    // Some proposals in wasm depend on previous proposals. For example the gc
    // proposal depends on function-references which depends on reference-types.
    // To avoid needing to enable all of them at once implicitly enable
    // downstream proposals once the end proposal is enabled (e.g. when enabling
    // gc that also enables function-references and reference-types).
    let function_references = gc || function_references.unwrap_or(false);
    let reference_types = function_references || reference_types.unwrap_or(false);
    let simd = relaxed_simd || simd.unwrap_or(false);

    let exceptions = stack_switching || exceptions.unwrap_or(false);

    config
        .wasm_multi_memory(multi_memory)
        .wasm_threads(threads)
        .wasm_shared_everything_threads(shared_everything_threads)
        .wasm_memory64(memory64)
        .wasm_function_references(function_references)
        .wasm_gc(gc)
        .wasm_reference_types(reference_types)
        .wasm_relaxed_simd(relaxed_simd)
        .wasm_simd(simd)
        .wasm_tail_call(tail_call)
        .wasm_custom_page_sizes(custom_page_sizes)
        .wasm_extended_const(extended_const)
        .wasm_wide_arithmetic(wide_arithmetic)
        .wasm_component_model_async(component_model_async)
        .wasm_component_model_async_builtins(component_model_async_builtins)
        .wasm_component_model_async_stackful(component_model_async_stackful)
        .wasm_component_model_error_context(component_model_error_context)
        .wasm_component_model_gc(component_model_gc)
        .wasm_exceptions(exceptions)
        .wasm_stack_switching(stack_switching)
        .cranelift_nan_canonicalization(nan_canonicalization);
    #[expect(deprecated, reason = "forwarding legacy-exceptions")]
    config.wasm_legacy_exceptions(legacy_exceptions);
}
