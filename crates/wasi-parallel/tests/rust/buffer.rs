//! This test checks that wasi-parallel's `read_buffer`/`write_buffer` work as
//! expected on a CPU. This is intended to be compiled to Wasm by `build.rs`,
//! but to run it directly:
//!
//! ```
//! rustc tests/rust/buffer.rs --target wasm32-wasi
//! RUST_BACKTRACE=1 wasmtime run --wasi-modules=experimental-wasi-parallel ./buffer.wasm
//! ```
#[allow(dead_code)]
mod wasi_parallel;

use wasi_parallel::{BufferAccessKind, DeviceKind};

fn main() -> Result<(), wasi_parallel::ParErrno> {
    let source = [0xFF; 1024];
    let destination = [0x00; 1024];
    assert!(source != destination);

    let device = wasi_parallel::get_device(DeviceKind::Cpu)?;
    let source_buffer =
        wasi_parallel::create_buffer(&device, source.len() as u32, BufferAccessKind::Read)?;
    wasi_parallel::write_buffer(&source, &source_buffer)?;
    wasi_parallel::read_buffer(&source_buffer, &destination)?;

    assert_eq!(source, destination);
    Ok(())
}
