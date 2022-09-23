//! This module provides Rust bindings to the wasi-parallel API as implemented
//! in this crate. It was generated at
//! https://alexcrichton.github.io/witx-bindgen using the WITX below and altered
//! slightly since Wiggle's generated signatures are slightly different than
//! witx-bindgen's:
//!
//! ```
//! resource Device
//! enum DeviceKind {
//!     Cpu,
//!     DiscreteGpu,
//!     IntegratedGpu
//! }
//! resource Buffer
//! enum BufferAccessKind {
//!     Read,
//!     Write,
//!     ReadWrite,
//! }
//! enum ParErrno {
//!     Success
//! }
//! get_device: function(hint: DeviceKind) -> expected<Device, ParErrno>
//! create_buffer: function(device: Device, size: u32, buffer_access_kind: BufferAccessKind) -> expected<Buffer, ParErrno>
//! write_buffer: function(source: list<u8>, destination: Buffer) -> expected<_, ParErrno>
//! read_buffer: function(source: Buffer, destination: list<u8>) -> expected<_, ParErrno>
//! parallel_for: function(device: Device, kernel: list<u8>, num_iterations: u32, block_size: u32, in_buffers: list<Buffer>, out_buffers: list<Buffer>) -> expected<_, ParErrno>
//! ```

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DeviceKind {
    Cpu,
    DiscreteGpu,
    IntegratedGpu,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BufferAccessKind {
    Read,
    Write,
    ReadWrite,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ParErrno {
    Success,
    Error(i32),
}

#[derive(Debug)]
#[repr(transparent)]
pub struct Device(i32);
impl Device {
    pub unsafe fn from_raw(raw: i32) -> Self {
        Self(raw)
    }

    pub fn into_raw(self) -> i32 {
        let ret = self.0;
        core::mem::forget(self);
        return ret;
    }

    pub fn as_raw(&self) -> i32 {
        self.0
    }
}
// Removed `impl Drop for Device { ... }`

#[derive(Debug)]
#[repr(transparent)]
pub struct Buffer(i32);
impl Buffer {
    pub unsafe fn from_raw(raw: i32) -> Self {
        Self(raw)
    }

    pub fn into_raw(self) -> i32 {
        let ret = self.0;
        core::mem::forget(self);
        return ret;
    }

    pub fn as_raw(&self) -> i32 {
        self.0
    }
}
// Removed `impl Drop for Buffer { ... }`

pub fn get_device(hint: DeviceKind) -> Result<Device, ParErrno> {
    unsafe {
        let ptr0 = RET_AREA.as_mut_ptr() as i32;
        #[link(wasm_import_module = "wasi_ephemeral_parallel")]
        extern "C" {
            #[cfg_attr(target_arch = "wasm32", link_name = "get_device")]
            #[cfg_attr(not(target_arch = "wasm32"), link_name = "input_get_device")]
            fn witx_import(_: i32, _: i32) -> i32;
        }
        let result = witx_import(hint as i32, ptr0);
        match result {
            0 => Ok(Device(*(ptr0 as *const i32))),
            _ => Err(ParErrno::Error(result)),
        }
    }
}

pub fn create_buffer(
    device: &Device,
    size: u32,
    buffer_access_kind: BufferAccessKind,
) -> Result<Buffer, ParErrno> {
    unsafe {
        let ptr1 = RET_AREA.as_mut_ptr() as i32;
        #[link(wasm_import_module = "wasi_ephemeral_parallel")]
        extern "C" {
            #[cfg_attr(target_arch = "wasm32", link_name = "create_buffer")]
            #[cfg_attr(not(target_arch = "wasm32"), link_name = "input_create_buffer")]
            fn witx_import(_: i32, _: i32, _: i32, _: i32) -> i32;
        }
        let result = witx_import(device.0, size as i32, buffer_access_kind as i32, ptr1);
        match result {
            0 => Ok(Buffer(*(ptr1 as *const i32))),
            _ => Err(ParErrno::Error(result)),
        }
    }
}

pub fn write_buffer(source: &[u8], destination: &Buffer) -> Result<(), ParErrno> {
    unsafe {
        let vec2 = source;
        let ptr2 = vec2.as_ptr() as i32;
        let len2 = vec2.len() as i32;
        #[link(wasm_import_module = "wasi_ephemeral_parallel")]
        extern "C" {
            #[cfg_attr(target_arch = "wasm32", link_name = "write_buffer")]
            #[cfg_attr(not(target_arch = "wasm32"), link_name = "input_write_buffer")]
            fn witx_import(_: i32, _: i32, _: i32) -> i32;
        }
        let result = witx_import(ptr2, len2, destination.0);
        match result {
            0 => Ok(()),
            _ => Err(ParErrno::Error(result)),
        }
    }
}

pub fn read_buffer(source: &Buffer, destination: &[u8]) -> Result<(), ParErrno> {
    unsafe {
        let vec4 = destination;
        let ptr4 = vec4.as_ptr() as i32;
        let len4 = vec4.len() as i32;
        #[link(wasm_import_module = "wasi_ephemeral_parallel")]
        extern "C" {
            #[cfg_attr(target_arch = "wasm32", link_name = "read_buffer")]
            #[cfg_attr(not(target_arch = "wasm32"), link_name = "input_read_buffer")]
            fn witx_import(_: i32, _: i32, _: i32) -> i32;
        }
        let result = witx_import(source.0, ptr4, len4);
        match result {
            0 => Ok(()),
            _ => Err(ParErrno::Error(result)),
        }
    }
}

pub fn parallel_for(
    kernel: u32,
    num_threads: u32,
    block_size: u32,
    in_buffers: &[&Buffer],
    out_buffers: &[&Buffer],
) -> Result<(), ParErrno> {
    unsafe {
        let vec6 = in_buffers;
        let len6 = vec6.len() as i32;
        let layout6 = core::alloc::Layout::from_size_align_unchecked(vec6.len() * 4, 4);
        let result6 = std::alloc::alloc(layout6);
        if result6.is_null() {
            std::alloc::handle_alloc_error(layout6);
        }
        for (i, e) in vec6.into_iter().enumerate() {
            let base = result6 as i32 + (i as i32) * 4;
            {
                *((base + 0) as *mut i32) = e.0;
            }
        }
        let vec7 = out_buffers;
        let len7 = vec7.len() as i32;
        let layout7 = core::alloc::Layout::from_size_align_unchecked(vec7.len() * 4, 4);
        let result7 = std::alloc::alloc(layout7);
        if result7.is_null() {
            std::alloc::handle_alloc_error(layout7);
        }
        for (i, e) in vec7.into_iter().enumerate() {
            let base = result7 as i32 + (i as i32) * 4;
            {
                *((base + 0) as *mut i32) = e.0;
            }
        }
        #[link(wasm_import_module = "wasi_ephemeral_parallel")]
        extern "C" {
            #[cfg_attr(target_arch = "wasm32", link_name = "parallel_for")]
            #[cfg_attr(not(target_arch = "wasm32"), link_name = "input_parallel_for")]
            fn witx_import(_: i32, _: i32, _: i32, _: i32, _: i32, _: i32, _: i32) -> i32;
        }
        let result = witx_import(
            kernel as i32,
            num_threads as i32,
            block_size as i32,
            result6 as i32,
            len6,
            result7 as i32,
            len7,
        );
        std::alloc::dealloc(result6, layout6);
        std::alloc::dealloc(result7, layout7);
        match result {
            0 => Ok(()),
            _ => Err(ParErrno::Error(result)),
        }
    }
}

static mut RET_AREA: [i64; 2] = [0; 2];
