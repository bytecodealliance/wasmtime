//! Implements a `wasi-parallel` device that solely executes sequentially.

use super::wasm_memory_buffer::as_pointer_and_length;
use super::{wasm_memory_buffer::WasmMemoryBuffer, Buffer, Device};
use crate::context::Kernel;
use crate::witx::types::{BufferAccessKind, DeviceKind};
use anyhow::{Context, Result};

pub struct SequentialDevice;

impl SequentialDevice {
    pub fn new() -> Box<dyn Device> {
        Box::new(Self)
    }
}

impl Device for SequentialDevice {
    fn kind(&self) -> DeviceKind {
        DeviceKind::Sequential
    }

    fn name(&self) -> String {
        "sequential implementation".into()
    }

    fn create_buffer(&self, size: i32, access: BufferAccessKind) -> Box<dyn Buffer> {
        Box::new(WasmMemoryBuffer::new(size as u32, access))
    }

    fn parallelize(
        &mut self,
        kernel: Kernel,
        num_iterations: i32,
        block_size: i32,
        in_buffers: Vec<&Box<dyn Buffer>>,
        out_buffers: Vec<&Box<dyn Buffer>>,
    ) -> Result<()> {
        // JIT-compile and instantiate the parallel kernel.
        let module = wasmtime::Module::new(kernel.engine(), kernel.module())
            .context("unable to compile kernel module")?;
        let mut store = wasmtime::Store::new(kernel.engine(), ());
        let imports = vec![kernel.memory().clone().into()];
        let instance = wasmtime::Instance::new(&mut store, &module, &imports)
            .context("failed to construct kernel instance")?;

        // Setup the buffer pointers.
        let buffers =
            as_pointer_and_length(in_buffers.into_iter().chain(out_buffers.into_iter())).unwrap();

        let kernel_fn = instance
            .get_func(&mut store, Kernel::NAME)
            .expect("failed to find kernel function");

        // Run each iteration of the parallel kernel sequentially.
        for iteration_id in 0..num_iterations {
            log::debug!("executing iteration {}", iteration_id);

            // Setup the parameters for the parallel execution.
            let mut params = vec![
                iteration_id.into(),
                num_iterations.into(),
                block_size.into(),
            ];
            params.extend_from_slice(&buffers);

            // Call the `kernel` function.
            kernel_fn
                .call(&mut store, &params[..], &mut [])
                .expect("failed to run kernel")
        }

        Ok(())
    }
}
