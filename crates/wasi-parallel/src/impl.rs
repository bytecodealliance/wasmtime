//! This module translates the calls from the Wasm program (running in a Wasm
//! engine) into something usable by the `wasi-parallel` context (running in the
//! Wasm host). Wiggle performs most of the conversions using `from_witx!` in
//! `witx.rs` but the special nature of `parallel_exec` (it can call back into a
//! function in the Wasm module) involves a manual implementation of this glue
//! code elsewhere.

use crate::witx::types::{
    Buffer, BufferAccessKind, BufferData, BufferSize, DeviceKind, ParallelDevice,
};
use crate::witx::wasi_ephemeral_parallel::WasiEphemeralParallel;
use crate::{WasiParallel, WasiParallelError};

impl WasiEphemeralParallel for WasiParallel {
    fn get_device(&mut self, hint: DeviceKind) -> Result<ParallelDevice, WasiParallelError> {
        let id = self.ctx.borrow().get_device(hint)?;
        Ok(ParallelDevice::from(id))
    }

    fn create_buffer(
        &mut self,
        device: ParallelDevice,
        size: BufferSize,
        kind: BufferAccessKind,
    ) -> Result<Buffer, super::WasiParallelError> {
        let id = self
            .ctx
            .borrow_mut()
            .create_buffer(device.into(), size as i32, kind)?;
        Ok(Buffer::from(id))
    }

    fn write_buffer<'a>(
        &mut self,
        data: &BufferData<'a>,
        buffer: Buffer,
    ) -> Result<(), super::WasiParallelError> {
        let mut ctx = self.ctx.borrow_mut();
        let buffer = ctx.get_buffer_mut(buffer.into())?;
        buffer.write(*data)?;
        Ok(())
    }

    fn read_buffer<'a>(
        &mut self,
        buffer: Buffer,
        data: &BufferData<'a>,
    ) -> Result<(), super::WasiParallelError> {
        let ctx = self.ctx.borrow_mut();
        let buffer = ctx.get_buffer(buffer.into())?;
        buffer.read(*data)
    }

    // Note: `parallel_exec` is manually linked in `lib.rs`.
}
