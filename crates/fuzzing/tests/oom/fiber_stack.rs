#![cfg(arc_try_new)]

use wasmtime::{
    Config, Engine, InstanceAllocationStrategy, Linker, Module, PoolingAllocationConfig, Result,
    Store,
};
use wasmtime_fuzzing::oom::OomTest;

#[tokio::test]
async fn pooling_allocator_fiber_stack_slot_leak_on_oom() -> Result<()> {
    let mut pool = PoolingAllocationConfig::default();
    pool.total_stacks(1);
    pool.total_memories(10);
    pool.total_tables(10);
    pool.total_core_instances(10);

    let mut config = Config::new();
    config.concurrency_support(false);
    config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, r#"(module (func (export "f")))"#)?;
    let linker = Linker::<()>::new(&engine);
    let instance_pre = linker.instantiate_pre(&module)?;

    OomTest::new()
        .allow_alloc_after_oom(true)
        .test_async(|| async {
            let mut store = Store::try_new(&engine, ())?;
            let instance = instance_pre.instantiate_async(&mut store).await?;
            let f = instance.get_typed_func::<(), ()>(&mut store, "f")?;
            f.call_async(&mut store, ()).await?;
            Ok(())
        })
        .await
}
