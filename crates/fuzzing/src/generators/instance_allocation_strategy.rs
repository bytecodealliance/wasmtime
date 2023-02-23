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
    pub fn to_wasmtime(&self) -> wasmtime::InstanceAllocationStrategy {
        match self {
            InstanceAllocationStrategy::OnDemand => wasmtime::InstanceAllocationStrategy::OnDemand,
            InstanceAllocationStrategy::Pooling(pooling) => {
                wasmtime::InstanceAllocationStrategy::Pooling(pooling.to_wasmtime())
            }
        }
    }
}
