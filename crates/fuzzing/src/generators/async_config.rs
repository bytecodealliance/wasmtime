use arbitrary::{Arbitrary, Unstructured};
use std::time::Duration;

/// Configuration for async support within a store.
///
/// Note that the `Arbitrary` implementation for this type always returns
/// `Disabled` because this is something that is statically chosen if the fuzzer
/// has support for async.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum AsyncConfig {
    /// No async support enabled.
    Disabled,
    /// Async support is enabled and cooperative yielding is done with fuel.
    YieldWithFuel(u64),
    /// Async support is enabled and cooperative yielding is done with epochs.
    YieldWithEpochs {
        /// Duration between epoch ticks.
        dur: Duration,
        /// Number of ticks between yields.
        ticks: u64,
    },
}

impl AsyncConfig {
    /// Applies this async configuration to the `wasmtime::Config` provided to
    /// ensure it's ready to execute with the resulting modules.
    pub fn configure(&self, config: &mut wasmtime::Config) {
        match self {
            AsyncConfig::Disabled => {}
            AsyncConfig::YieldWithFuel(_) => {
                config.async_support(true).consume_fuel(true);
            }
            AsyncConfig::YieldWithEpochs { .. } => {
                config.async_support(true).epoch_interruption(true);
            }
        }
    }
}

impl<'a> Arbitrary<'a> for AsyncConfig {
    fn arbitrary(_: &mut Unstructured<'a>) -> arbitrary::Result<AsyncConfig> {
        Ok(AsyncConfig::Disabled)
    }
}
