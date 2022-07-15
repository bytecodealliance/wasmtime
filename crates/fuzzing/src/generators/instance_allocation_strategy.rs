use arbitrary::Arbitrary;

use super::InstanceLimits;

/// Configuration for `wasmtime::InstanceAllocationStrategy`.
#[derive(Arbitrary, Clone, Debug, Eq, PartialEq, Hash)]
pub enum InstanceAllocationStrategy {
    /// Use the on-demand instance allocation strategy.
    OnDemand,
    /// Use the pooling instance allocation strategy.
    Pooling {
        /// The pooling strategy to use.
        strategy: PoolingAllocationStrategy,
        /// The instance limits.
        instance_limits: InstanceLimits,
    },
}

impl InstanceAllocationStrategy {
    /// Convert this generated strategy a Wasmtime strategy.
    pub fn to_wasmtime(&self) -> wasmtime::InstanceAllocationStrategy {
        match self {
            InstanceAllocationStrategy::OnDemand => wasmtime::InstanceAllocationStrategy::OnDemand,
            InstanceAllocationStrategy::Pooling {
                strategy,
                instance_limits,
            } => wasmtime::InstanceAllocationStrategy::Pooling {
                strategy: strategy.to_wasmtime(),
                instance_limits: instance_limits.to_wasmtime(),
            },
        }
    }
}

/// Configuration for `wasmtime::PoolingAllocationStrategy`.
#[derive(Arbitrary, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PoolingAllocationStrategy {
    /// Use next available instance slot.
    NextAvailable,
    /// Use random instance slot.
    Random,
    /// Use an affinity-based strategy.
    ReuseAffinity,
}

impl PoolingAllocationStrategy {
    fn to_wasmtime(&self) -> wasmtime::PoolingAllocationStrategy {
        match self {
            PoolingAllocationStrategy::NextAvailable => {
                wasmtime::PoolingAllocationStrategy::NextAvailable
            }
            PoolingAllocationStrategy::Random => wasmtime::PoolingAllocationStrategy::Random,
            PoolingAllocationStrategy::ReuseAffinity => {
                wasmtime::PoolingAllocationStrategy::ReuseAffinity
            }
        }
    }
}
