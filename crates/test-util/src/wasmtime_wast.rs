//! Translation of test configuration in [`crate::wast`] into Wasmtime
//! configuration.
//!
//! Everything is expressed as [`CommonOptions`], the same options the
//! `wasmtime` CLI accepts. Harnesses follow the same steps:
//!
//! 1. Start from [`Compiler::common_options`], or their own options.
//! 2. Make their own adjustments, such as memory limits.
//! 3. Apply the test with [`TestConfig::apply`], last so the test has the
//!    final say.
//! 4. Create a [`wasmtime::Config`] with [`CommonOptions::config`] and
//!    configure each [`wasmtime::Store`] with
//!    [`CommonOptions::configure_store`].

use crate::wast::{Compiler, TestConfig};
use wasmtime::{Result, bail, format_err};
use wasmtime_cli_flags::CommonOptions;

impl TestConfig {
    /// Applies this test's configuration on top of `opts`.
    ///
    /// This sets the test's WebAssembly features and then applies its own
    /// `flags`, which override anything already in `opts`.
    ///
    /// Returns an error if the flags set a WebAssembly feature, which must use
    /// the boolean options of `TestConfig` instead, or an option that the
    /// harness owns because it's described by [`crate::wast::WastConfig`]: the compiler,
    /// the target, or the pooling allocator.
    pub fn apply(&self, opts: &mut CommonOptions) -> Result<()> {
        self.apply_features(opts);
        let mut flags = self.flags()?;
        opts.merge(&mut flags)?;

        // Reject `flags` that are supposed to be configured elsewhere in the
        // harness for testing wasts and such. This ensures that `should_fail`
        // calculations have all relevant info to make a decision since `flags`
        // aren't factored in at all there.
        if let Some(name) = flags.wasm_features_specified().first() {
            bail!("`flags` cannot set the wasm feature `-W {name}`, use `;;!` feature options");
        }
        if flags.codegen.compiler.is_some() {
            bail!("`flags` cannot set `-C compiler`, it's chosen by the harness");
        }
        if flags.target.is_some() {
            bail!("`flags` cannot set `--target`, it's chosen by the harness");
        }
        if flags.opts.pooling_allocator.is_some() {
            bail!("`flags` cannot set `-O pooling-allocator`, it's chosen by the harness");
        }
        Ok(())
    }

    /// Sets the WebAssembly features in `opts` to those of this test.
    ///
    /// Every feature is set explicitly, and features the test doesn't list
    /// are disabled. This ensures tests are annotated with the features they
    /// need, even after a feature becomes enabled by default. Features that
    /// enabled features depend on, such as `function_references` for `gc`,
    /// are enabled too.
    fn apply_features(&self, opts: &mut CommonOptions) {
        let TestConfig {
            bulk_memory,
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
            compact_imports,
            component_model_async,
            component_model_more_async_builtins,
            component_model_async_stackful,
            component_model_threading,
            component_model_error_context,
            component_model_gc,
            component_model_map,
            component_model_memory64,
            component_model_fixed_length_lists,
            component_model_implements,
            component_model_canonical_names,
            component_model_accessors,
            simd,
            exceptions,
            legacy_exceptions,
            stack_switching,
            // Not implemented in Wasmtime yet.
            custom_descriptors: _,

            // Not features.
            hogs_memory: _,
            spec_test: _,
            flags: _,
        } = *self;

        let on = |feature: Option<bool>| feature.unwrap_or(false);

        // Enable the features that enabled features depend on, so tests only
        // need to list the feature they're testing.
        let gc = on(gc);
        let function_references = gc || on(function_references);
        let reference_types =
            function_references || on(component_model_async) || on(reference_types);
        let relaxed_simd = on(relaxed_simd);
        let simd = relaxed_simd || on(simd);
        let stack_switching = on(stack_switching);
        let exceptions = stack_switching || on(exceptions);

        let w = &mut opts.wasm;
        w.bulk_memory = Some(on(bulk_memory));
        w.memory64 = Some(on(memory64));
        w.custom_page_sizes = Some(on(custom_page_sizes));
        w.multi_memory = Some(on(multi_memory));
        w.threads = Some(on(threads));
        w.shared_everything_threads = Some(on(shared_everything_threads));
        w.gc = Some(gc);
        w.function_references = Some(function_references);
        w.relaxed_simd = Some(relaxed_simd);
        w.reference_types = Some(reference_types);
        w.tail_call = Some(on(tail_call));
        w.extended_const = Some(on(extended_const));
        w.wide_arithmetic = Some(on(wide_arithmetic));
        w.compact_imports = Some(on(compact_imports));
        w.component_model_async = Some(on(component_model_async));
        w.component_model_more_async_builtins = Some(on(component_model_more_async_builtins));
        w.component_model_async_stackful = Some(on(component_model_async_stackful));
        w.component_model_threading = Some(on(component_model_threading));
        w.component_model_error_context = Some(on(component_model_error_context));
        w.component_model_gc = Some(on(component_model_gc));
        w.component_model_map = Some(on(component_model_map));
        w.component_model_memory64 = Some(on(component_model_memory64));
        w.component_model_fixed_length_lists = Some(on(component_model_fixed_length_lists));
        w.component_model_implements = Some(on(component_model_implements));
        w.component_model_canonical_names = Some(on(component_model_canonical_names));
        w.component_model_accessors = Some(on(component_model_accessors));
        w.simd = Some(simd);
        w.exceptions = Some(exceptions);
        w.legacy_exceptions = Some(on(legacy_exceptions));
        w.stack_switching = Some(stack_switching);

        // Multi-value is part of the baseline that all tests assume.
        w.multi_value = Some(true);

        // GC infrastructure in Wasmtime is needed for any GC reference.
        w.gc_support = Some(gc || exceptions || reference_types);
    }

    /// Parses this test's `flags`. Options that the flags don't specify are
    /// unset.
    fn flags(&self) -> Result<CommonOptions> {
        use clap::Parser;

        let mut args = vec!["wasmtime"];
        if let Some(flags) = &self.flags {
            args.extend(flags.to_vec());
        }
        let mut flags = CommonOptions::try_parse_from(&args)
            .map_err(|e| format_err!("failed to parse `flags`: {e}"))?;
        // Tests are configured by their `flags`, not the environment.
        flags.ignore_env = true;
        Ok(flags)
    }
}

impl Compiler {
    /// Returns the options to run tests with this compiler.
    pub fn common_options(&self) -> CommonOptions {
        use wasmtime_environ::TripleExt;

        let mut opts = CommonOptions::new();
        // Tests shouldn't read or write the global compilation cache.
        opts.codegen.cache = Some(false);

        opts.codegen.compiler = Some(match self {
            Compiler::CraneliftNative | Compiler::CraneliftPulley => wasmtime::Strategy::Cranelift,
            Compiler::Winch => wasmtime::Strategy::Winch,
        });
        if let Compiler::CraneliftPulley = self {
            opts.target = Some(target_lexicon::Triple::pulley_host().to_string());
        }
        opts
    }
}
