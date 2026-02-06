use arbitrary::Arbitrary;
use std::mem::MaybeUninit;
use wasmtime::component::__internal::{
    CanonicalAbiInfo, InstanceType, InterfaceType, LiftContext, LowerContext,
};
use wasmtime::component::{ComponentType, Lift, Lower};
use wasmtime::{Config, Engine};
use wasmtime_environ::prelude::*;

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

pub fn map_config() -> Config {
    let mut config = config();
    config.wasm_component_model_map(true);
    config
}

pub fn map_engine() -> Engine {
    Engine::new(&map_config()).unwrap()
}

pub fn async_engine() -> Engine {
    Engine::default()
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
            fn linear_lower_to_flat<U>(
                &self,
                cx: &mut LowerContext<'_, U>,
                ty: InterfaceType,
                dst: &mut MaybeUninit<Self::Lower>,
            ) -> Result<()> {
                <$b as Lower>::linear_lower_to_flat(&self.0, cx, ty, dst)
            }

            fn linear_lower_to_memory<U>(&self, cx: &mut LowerContext<'_, U>, ty: InterfaceType, offset: usize) -> Result<()> {
                <$b as Lower>::linear_lower_to_memory(&self.0, cx, ty, offset)
            }
        }

        unsafe impl Lift for $a {
            fn linear_lift_from_flat(cx: &mut LiftContext<'_>, ty: InterfaceType, src: &Self::Lower) -> Result<Self> {
                Ok(Self(<$b as Lift>::linear_lift_from_flat(cx, ty, src)?))
            }

            fn linear_lift_from_memory(cx: &mut LiftContext<'_>, ty: InterfaceType, bytes: &[u8]) -> Result<Self> {
                Ok(Self(<$b as Lift>::linear_lift_from_memory(cx, ty, bytes)?))
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
