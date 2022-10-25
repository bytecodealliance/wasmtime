mod clocks;
mod table;
pub use table::Table;

wit_bindgen_host_wasmtime_rust::generate!({
    import: "../wit/wasi-clocks.wit.md",
    import: "../wit/wasi-default-clocks.wit.md",
    default: "../wit/command.wit.md",
    name: "wasi",
});

pub fn add_to_linker<T>(
    l: &mut wasmtime::component::Linker<T>,
    f: impl (Fn(&mut T) -> &mut WasiCtx) + Copy + Send + Sync + 'static,
) -> anyhow::Result<()> {
    wasi_clocks::add_to_linker(l, f.clone())?;
    wasi_default_clocks::add_to_linker(l, f)?;
    Ok(())
}

pub struct WasiCtx {
    table: Table,
    default_monotonic: wasi_clocks::MonotonicClock,
    default_wall: wasi_clocks::WallClock,
}

impl Default for WasiCtx {
    fn default() -> WasiCtx {
        let mut table = Table::default();
        let default_monotonic = table
            .push(Box::new(clocks::MonotonicClock::default()))
            .unwrap();
        let default_wall = table.push(Box::new(clocks::WallClock::default())).unwrap();
        WasiCtx {
            table,
            default_monotonic,
            default_wall,
        }
    }
}
