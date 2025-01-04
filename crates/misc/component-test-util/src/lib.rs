use anyhow::Result;
use arbitrary::Arbitrary;
use std::mem::MaybeUninit;
use wasmtime::component::__internal::{
    CanonicalAbiInfo, InstanceType, InterfaceType, LiftContext, LowerContext,
};
use wasmtime::component::{ComponentNamedList, ComponentType, Func, Lift, Lower, TypedFunc, Val};
use wasmtime::{AsContextMut, Config, Engine};

pub trait TypedFuncExt<P, R> {
    fn call_and_post_return<T: Send>(
        &self,
        store: impl AsContextMut<Data = T>,
        params: P,
    ) -> Result<R>;
}

impl<P, R> TypedFuncExt<P, R> for TypedFunc<P, R>
where
    P: ComponentNamedList + Lower,
    R: ComponentNamedList + Lift + Send + Sync + 'static,
{
    fn call_and_post_return<T: Send>(
        &self,
        mut store: impl AsContextMut<Data = T>,
        params: P,
    ) -> Result<R> {
        let result = self.call(&mut store, params)?;
        self.post_return(&mut store)?;
        Ok(result)
    }
}

pub trait FuncExt {
    fn call_and_post_return<T: Send>(
        &self,
        store: impl AsContextMut<Data = T>,
        params: &[Val],
        results: &mut [Val],
    ) -> Result<()>;
}

impl FuncExt for Func {
    fn call_and_post_return<T: Send>(
        &self,
        mut store: impl AsContextMut<Data = T>,
        params: &[Val],
        results: &mut [Val],
    ) -> Result<()> {
        self.call(&mut store, params, results)?;
        self.post_return(&mut store)?;
        Ok(())
    }
}

pub fn config() -> Config {
    drop(env_logger::try_init());

    let mut config = Config::new();
    config.wasm_component_model(true);

    // When `WASMTIME_TEST_NO_HOG_MEMORY` is set it means we're in qemu. The
    // component model tests create a disproportionate number of instances so
    // try to cut down on virtual memory usage by avoiding 4G reservations.
    if std::env::var("WASMTIME_TEST_NO_HOG_MEMORY").is_ok() {
        config.memory_reservation(0);
        config.memory_guard_size(0);
    }
    config
}

pub fn engine() -> Engine {
    Engine::new(&config()).unwrap()
}

pub fn async_engine() -> Engine {
    let mut config = config();
    config.async_support(true);
    Engine::new(&config).unwrap()
}

/// Newtype wrapper for `f32` whose `PartialEq` impl considers NaNs equal to each other.
#[derive(Copy, Clone, Debug, Arbitrary)]
pub struct Float32(pub f32);

/// Newtype wrapper for `f64` whose `PartialEq` impl considers NaNs equal to each other.
#[derive(Copy, Clone, Debug, Arbitrary)]
pub struct Float64(pub f64);

macro_rules! forward_impls {
    ($($a:ty => $b:ty,)*) => ($(
        unsafe impl ComponentType for $a {
            type Lower = <$b as ComponentType>::Lower;

            const ABI: CanonicalAbiInfo = <$b as ComponentType>::ABI;

            #[inline]
            fn typecheck(ty: &InterfaceType, types: &InstanceType<'_>) -> Result<()> {
                <$b as ComponentType>::typecheck(ty, types)
            }
        }

        unsafe impl Lower for $a {
            fn lower<U>(
                &self,
                cx: &mut LowerContext<'_, U>,
                ty: InterfaceType,
                dst: &mut MaybeUninit<Self::Lower>,
            ) -> Result<()> {
                <$b as Lower>::lower(&self.0, cx, ty, dst)
            }

            fn store<U>(&self, cx: &mut LowerContext<'_, U>, ty: InterfaceType, offset: usize) -> Result<()> {
                <$b as Lower>::store(&self.0, cx, ty, offset)
            }
        }

        unsafe impl Lift for $a {
            fn lift(cx: &mut LiftContext<'_>, ty: InterfaceType, src: &Self::Lower) -> Result<Self> {
                Ok(Self(<$b as Lift>::lift(cx, ty, src)?))
            }

            fn load(cx: &mut LiftContext<'_>, ty: InterfaceType, bytes: &[u8]) -> Result<Self> {
                Ok(Self(<$b as Lift>::load(cx, ty, bytes)?))
            }
        }

        impl PartialEq for $a {
            fn eq(&self, other: &Self) -> bool {
                self.0 == other.0 || (self.0.is_nan() && other.0.is_nan())
            }
        }
    )*)
}

forward_impls! {
    Float32 => f32,
    Float64 => f64,
}

/// Helper method to apply `wast_config` to `config`.
pub fn apply_wast_config(config: &mut Config, wast_config: &wasmtime_wast_util::WastConfig) {
    use wasmtime_environ::TripleExt;
    use wasmtime_wast_util::{Collector, Compiler};

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
pub fn apply_test_config(config: &mut Config, test_config: &wasmtime_wast_util::TestConfig) {
    let wasmtime_wast_util::TestConfig {
        memory64,
        custom_page_sizes,
        multi_memory,
        threads,
        gc,
        function_references,
        relaxed_simd,
        reference_types,
        tail_call,
        extended_const,
        wide_arithmetic,
        component_model_more_flags,
        component_model_async,
        nan_canonicalization,
        simd,

        hogs_memory: _,
        gc_types: _,
    } = *test_config;
    // Note that all of these proposals/features are currently default-off to
    // ensure that we annotate all tests accurately with what features they
    // need, even in the future when features are stabilized.
    let memory64 = memory64.unwrap_or(false);
    let custom_page_sizes = custom_page_sizes.unwrap_or(false);
    let multi_memory = multi_memory.unwrap_or(false);
    let threads = threads.unwrap_or(false);
    let gc = gc.unwrap_or(false);
    let tail_call = tail_call.unwrap_or(false);
    let extended_const = extended_const.unwrap_or(false);
    let wide_arithmetic = wide_arithmetic.unwrap_or(false);
    let component_model_more_flags = component_model_more_flags.unwrap_or(false);
    let component_model_async = component_model_async.unwrap_or(false);
    let nan_canonicalization = nan_canonicalization.unwrap_or(false);
    let relaxed_simd = relaxed_simd.unwrap_or(false);

    // Some proposals in wasm depend on previous proposals. For example the gc
    // proposal depends on function-references which depends on reference-types.
    // To avoid needing to enable all of them at once implicitly enable
    // downstream proposals once the end proposal is enabled (e.g. when enabling
    // gc that also enables function-references and reference-types).
    let function_references = gc || function_references.unwrap_or(false);
    let reference_types = function_references || reference_types.unwrap_or(false);
    let simd = relaxed_simd || simd.unwrap_or(false);

    config
        .wasm_multi_memory(multi_memory)
        .wasm_threads(threads)
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
        .wasm_component_model_more_flags(component_model_more_flags)
        .wasm_component_model_async(component_model_async)
        .cranelift_nan_canonicalization(nan_canonicalization);
}
