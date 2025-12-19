use wasmtime::component::Component;
use wasmtime::{Config, Engine, Module, Result};

#[test]
#[cfg_attr(miri, ignore)]
fn perfmap() -> Result<()> {
    let mut config = Config::new();
    config.profiler(wasmtime::ProfilingStrategy::PerfMap);
    let engine = Engine::new(&config)?;

    Module::new(&engine, "(module (func))")?;
    Component::new(&engine, "(component)")?;
    Component::new(&engine, "(component (core module (func)))")?;
    Component::new(
        &engine,
        "(component
            (core module (func))
            (core module (func))
        )",
    )?;

    Ok(())
}
