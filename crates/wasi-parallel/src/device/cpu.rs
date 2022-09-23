//! Implement a `wasi-parallel` device using all available cores on the CPU.

use super::wasm_memory_buffer::{as_pointer_and_length, WasmMemoryBuffer};
use super::{Buffer, Device};
use crate::context::Kernel;
use crate::witx::types::{BufferAccessKind, DeviceKind};
use anyhow::Result;
use std::convert::TryInto;

pub struct CpuDevice {
    pool: scoped_threadpool::Pool,
}

impl CpuDevice {
    pub fn new() -> Box<dyn Device> {
        let pool = scoped_threadpool::Pool::new(num_cpus::get().try_into().unwrap());
        Box::new(Self { pool })
    }
}

impl Device for CpuDevice {
    fn kind(&self) -> DeviceKind {
        DeviceKind::Cpu
    }

    fn name(&self) -> String {
        "thread pool implementation".into() // TODO retrieve CPU name from system.
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
        self.pool.scoped(|scoped| {
            let module = wasmtime::Module::new(kernel.engine(), kernel.module())
                .expect("unable to compile module");

            // Setup the buffer pointers.
            let buffers =
                as_pointer_and_length(in_buffers.into_iter().chain(out_buffers.into_iter()))
                    .unwrap();

            for iteration_id in 0..num_iterations {
                let engine = kernel.engine().clone();
                let module = module.clone();
                let memory = kernel.memory().clone();
                let buffers = buffers.clone();
                scoped.execute(move || {
                    // Instantiate again in a new thread.
                    let mut store = wasmtime::Store::new(&engine, ());
                    let imports = vec![memory.clone().into()];
                    let instance = wasmtime::Instance::new(&mut store, &module, &imports)
                        .expect("failed to construct thread instance");

                    // Setup the parameters for the parallel execution.
                    let mut params = vec![
                        iteration_id.into(),
                        num_iterations.into(),
                        block_size.into(),
                    ];
                    params.extend_from_slice(&buffers);

                    // Call the `kernel` function.
                    log::debug!("executing iteration {}", iteration_id);
                    let kernel_fn = instance
                        .get_func(&mut store, Kernel::NAME)
                        .expect("failed to find kernel function");
                    kernel_fn
                        .call(&mut store, &params[..], &mut [])
                        .expect("failed to run kernel")
                });
            }
        });
        Ok(())
    }
}
