use super::PoolingAllocationConfig;
use arbitrary::Arbitrary;

/// Configuration for `wasmtime::InstanceAllocationStrategy`.
#[derive(Arbitrary, Clone, Debug, Eq, PartialEq, Hash)]
pub enum InstanceAllocationStrategy {
    /// Use the on-demand instance allocation strategy.
    OnDemand,
    /// Use the pooling instance allocation strategy.
    Pooling(PoolingAllocationConfig),
}

impl InstanceAllocationStrategy {
    /// Convert this generated strategy a Wasmtime strategy.
    pub fn configure(&self, cfg: &mut wasmtime_cli_flags::CommonOptions) {
        match self {
            InstanceAllocationStrategy::OnDemand => {}
            InstanceAllocationStrategy::Pooling(pooling) => {
                cfg.opts.pooling_allocator = Some(true);
                pooling.configure(cfg);
            }
        }
    }
}
