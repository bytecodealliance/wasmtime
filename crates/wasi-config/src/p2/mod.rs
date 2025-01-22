//! Implementation of wasip2 version of `wasi:config` package

mod gen_ {
    wasmtime::component::bindgen!({
        path: "src/p2/wit",
        world: "wasi:config/imports",
        trappable_imports: true,
    });
}
use self::gen_::wasi::config::store as generated;

use crate::WasiConfig;

impl generated::Host for WasiConfig<'_> {
    fn get(&mut self, key: String) -> anyhow::Result<Result<Option<String>, generated::Error>> {
        Ok(Ok(self.vars.0.get(&key).map(|s| s.to_owned())))
    }

    fn get_all(&mut self) -> anyhow::Result<Result<Vec<(String, String)>, generated::Error>> {
        Ok(Ok(self
            .vars
            .0
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()))
    }
}

/// Add all the `wasi-config` world's interfaces to a [`wasmtime::component::Linker`].
pub fn add_to_linker<T>(
    l: &mut wasmtime::component::Linker<T>,
    f: impl Fn(&mut T) -> WasiConfig<'_> + Send + Sync + Copy + 'static,
) -> anyhow::Result<()> {
    generated::add_to_linker_get_host(l, f)?;
    Ok(())
}
