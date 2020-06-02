#[cfg(feature = "tracing")]
pub use tracing_crate::*;

#[cfg(not(feature = "tracing"))]
mod noop_tracing {
    // TODO fill in rest of the noop interface
    // idk how to do this because macro_rules! / #[macro_export] doesn't follow the usual module
    // visibility rules.
}
#[cfg(not(feature = "tracing"))]
pub use noop_tracing::*;
