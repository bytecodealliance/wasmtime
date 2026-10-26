//! The module for the Wasmtime CLI commands.

#[cfg(any(feature = "run", feature = "wizer"))]
mod run;
#[cfg(any(feature = "run", feature = "wizer"))]
pub use self::run::*;

#[cfg(feature = "serve")]
mod serve;
#[cfg(feature = "serve")]
pub use self::serve::*;

#[cfg(feature = "explore")]
mod explore;
#[cfg(feature = "explore")]
pub use self::explore::*;

#[cfg(feature = "wast")]
mod wast;
#[cfg(feature = "wast")]
pub use self::wast::*;

#[cfg(feature = "cache")]
mod config;
#[cfg(feature = "cache")]
pub use self::config::*;

#[cfg(feature = "compile")]
mod compile;
#[cfg(feature = "compile")]
pub use self::compile::*;

#[cfg(feature = "cranelift")]
mod settings;
#[cfg(feature = "cranelift")]
pub use self::settings::*;

#[cfg(feature = "objdump")]
mod objdump;
#[cfg(feature = "objdump")]
pub use self::objdump::*;

#[cfg(all(feature = "hot-blocks", target_os = "linux"))]
mod hot_blocks;
#[cfg(all(feature = "hot-blocks", target_os = "linux"))]
pub use self::hot_blocks::*;

#[cfg(feature = "wizer")]
mod wizer;
#[cfg(feature = "wizer")]
pub use self::wizer::*;

#[cfg(all(
    feature = "coredump",
    any(feature = "run", feature = "wizer", feature = "serve")
))]
fn write_core_dump(
    store: &mut impl wasmtime::AsContextMut,
    err: &wasmtime::Error,
    name: &str,
    path: &str,
) -> wasmtime::Result<()> {
    use std::fs::File;
    use std::io::Write;
    use wasmtime::error::Context as _;

    let core_dump = err
        .downcast_ref::<wasmtime::WasmCoreDump>()
        .expect("should have been configured to capture core dumps");
    let core_dump = core_dump.serialize(store, name);
    let mut core_dump_file =
        File::create(path).with_context(|| format!("failed to create file at `{path}`"))?;
    core_dump_file
        .write_all(&core_dump)
        .with_context(|| format!("failed to write core dump file at `{path}`"))?;
    Ok(())
}
