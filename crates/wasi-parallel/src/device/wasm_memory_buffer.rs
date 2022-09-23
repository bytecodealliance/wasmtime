//! Contains a reference to a slice of Wasm memory.
//!
use super::Buffer;
use crate::witx::types::BufferAccessKind;
use anyhow::{anyhow, bail, Context, Result};
use std::{any::Any, convert::TryInto, vec};
use wasmtime::Val;
use wiggle::GuestPtr;

/// This kind of buffer is designed to live exclusively in Wasm memory--it
/// contains the information necessary for reading and writing to Wasm memory.
/// Its lifecycle involves:
/// - The buffer is created by `create_buffer`; this associates a buffer ID with
///   a device ID, but the buffer has no knowledge of what data it may contain,
///   only its length.
/// - The buffer may then be written to by `write_buffer`; at this point the
///   buffer will record the its offset within the Wasm memory.
/// - When used by `parallel_exec`, this buffer will simply pass its offset and
///   length to the parallel kernel, where it will be mutated by a Wasm
///   function.
/// - The buffer contents may "read" from one section of the Wasm memory to
///   another.
pub struct WasmMemoryBuffer {
    offset: Option<u32>,
    length: u32,
    access: BufferAccessKind,
}

impl WasmMemoryBuffer {
    pub fn new(size: u32, access: BufferAccessKind) -> Self {
        Self {
            offset: None,
            length: size,
            access,
        }
    }
}

impl Buffer for WasmMemoryBuffer {
    fn len(&self) -> u32 {
        self.length
    }

    fn access(&self) -> BufferAccessKind {
        self.access
    }

    /// Does not copy data: simply checks that the lengths of the buffer and
    /// guest slice match and then records the starting location of the guest
    /// pointer. This will require some re-thinking once multiple memories are
    /// possible (TODO).
    fn write(&mut self, slice: GuestPtr<[u8]>) -> Result<()> {
        if slice.len() == self.len() {
            self.offset = Some(slice.offset_base());
            Ok(())
        } else {
            Err(anyhow!(
                "The slice to write did not match the buffer size: {} != {}",
                slice.len(),
                self.len(),
            ))
        }
    }

    /// This implementation of `read` will attempt to copy the device data, held
    /// in Wasm memory, to another location in Wasm memory. Currently it will
    /// fail if the slices are overlapping (TODO). At some point, this should
    /// also see if the `read` is from and to the same slice and avoid the copy
    /// entirely (TODO).
    fn read(&self, slice: GuestPtr<[u8]>) -> Result<()> {
        debug_assert_eq!(slice.len(), self.len());
        let mem = slice.mem().base();
        let mem = unsafe { std::slice::from_raw_parts_mut(mem.0, mem.1 as usize) };
        copy_within_a_slice(
            mem,
            self.offset.unwrap() as usize,
            slice.offset_base() as usize,
            slice.len() as usize,
        );
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }
}

/// This helper copies one sub-slice to another within a mutable slice. It will
/// panic if the slices are overlapping.
fn copy_within_a_slice<T: Clone>(v: &mut [T], from: usize, to: usize, len: usize) {
    if from == to {
        // Do nothing.
    } else if from > to {
        let (dst, src) = v.split_at_mut(from);
        dst[to..to + len].clone_from_slice(&src[..len]);
    } else {
        let (src, dst) = v.split_at_mut(to);
        dst[..len].clone_from_slice(&src[from..from + len]);
    }
}

/// Convert an iterator of [`WasmMemoryBuffer`] into their corresponding
/// WebAssembly pointer address and length.
pub fn as_pointer_and_length<'a, I>(buffers: I) -> Result<Vec<Val>>
where
    I: Iterator<Item = &'a Box<dyn Buffer>>,
{
    let mut results = vec![];
    for b in buffers {
        if let Some(b_) = b.as_any().downcast_ref::<WasmMemoryBuffer>() {
            let len = b
                .len()
                .try_into()
                .context("the buffer length is too large for an i32")?;
            if let Some(offset) = b_.offset {
                results.push(Val::I32(offset.try_into().unwrap()));
                results.push(Val::I32(len));
            } else {
                bail!("the buffer has not been written to: {:?}", b);
                // TODO there must be a way to set up the buffer without writing
                // to it; e.g., for write buffers that only the device touches.
            }
        } else {
            bail!("the buffer is invalid; any buffer used by the CPU should be castable to a pointer + length");
        }
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_before() {
        let mut buffer = [0u8; 1024];
        buffer[42] = 42;
        copy_within_a_slice(&mut buffer, 42, 41, 1);
        assert_eq!(buffer[41], 42);
    }

    #[test]
    fn copy_same_location() {
        let mut buffer = [0u8; 1024];
        buffer[42] = 42;
        copy_within_a_slice(&mut buffer, 42, 42, 1);
        assert_eq!(buffer[42], 42);
    }

    #[test]
    fn copy_after() {
        let mut buffer = [0u8; 1024];
        buffer[42] = 42;
        copy_within_a_slice(&mut buffer, 42, 43, 1);
        assert_eq!(buffer[43], 42);
    }

    #[test]
    #[should_panic]
    fn copy_overlapping() {
        let mut buffer = [0u8; 1024];
        copy_within_a_slice(&mut buffer, 42, 43, 2);
    }
}
