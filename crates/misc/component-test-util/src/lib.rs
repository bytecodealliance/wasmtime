use anyhow::Result;
use arbitrary::Arbitrary;
use std::mem::MaybeUninit;
use wasmtime::component::__internal::{
    CanonicalAbiInfo, ComponentTypes, InterfaceType, Memory, MemoryMut, Options, StoreOpaque,
};
use wasmtime::component::{ComponentNamedList, ComponentType, Func, Lift, Lower, TypedFunc, Val};
use wasmtime::{AsContextMut, Config, Engine, StoreContextMut};

pub trait TypedFuncExt<P, R> {
    fn call_and_post_return(&self, store: impl AsContextMut, params: P) -> Result<R>;
}

impl<P, R> TypedFuncExt<P, R> for TypedFunc<P, R>
where
    P: ComponentNamedList + Lower,
    R: ComponentNamedList + Lift,
{
    fn call_and_post_return(&self, mut store: impl AsContextMut, params: P) -> Result<R> {
        let result = self.call(&mut store, params)?;
        self.post_return(&mut store)?;
        Ok(result)
    }
}

pub trait FuncExt {
    fn call_and_post_return(
        &self,
        store: impl AsContextMut,
        params: &[Val],
        results: &mut [Val],
    ) -> Result<()>;
}

impl FuncExt for Func {
    fn call_and_post_return(
        &self,
        mut store: impl AsContextMut,
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
        config.static_memory_maximum_size(0);
        config.dynamic_memory_guard_size(0);
    }
    config
}

pub fn engine() -> Engine {
    Engine::new(&config()).unwrap()
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
            fn typecheck(ty: &InterfaceType, types: &ComponentTypes) -> Result<()> {
                <$b as ComponentType>::typecheck(ty, types)
            }
        }

        unsafe impl Lower for $a {
            fn lower<U>(
                &self,
                store: &mut StoreContextMut<U>,
                options: &Options,
                dst: &mut MaybeUninit<Self::Lower>,
            ) -> Result<()> {
                <$b as Lower>::lower(&self.0, store, options, dst)
            }

            fn store<U>(&self, memory: &mut MemoryMut<'_, U>, offset: usize) -> Result<()> {
                <$b as Lower>::store(&self.0, memory, offset)
            }
        }

        unsafe impl Lift for $a {
            fn lift(store: &StoreOpaque, options: &Options, src: &Self::Lower) -> Result<Self> {
                Ok(Self(<$b as Lift>::lift(store, options, src)?))
            }

            fn load(memory: &Memory<'_>, bytes: &[u8]) -> Result<Self> {
                Ok(Self(<$b as Lift>::load(memory, bytes)?))
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
