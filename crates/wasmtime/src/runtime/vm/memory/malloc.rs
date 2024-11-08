//! Support for implementing the [`RuntimeLinearMemory`] trait in terms of a
//! platform memory allocation primitive (e.g. `malloc`)
//!
//! Note that memory is allocated here using `Vec::try_reserve` to explicitly
//! handle memory allocation failures.

use crate::prelude::*;
use crate::runtime::vm::memory::RuntimeLinearMemory;
use crate::runtime::vm::SendSyncPtr;
use core::mem;
use core::ptr::NonNull;
use wasmtime_environ::Tunables;

#[repr(C, align(16))]
#[derive(Copy, Clone)]
pub struct Align16(u128);

/// An instance of linear memory backed by the default system allocator.
pub struct MallocMemory {
    storage: Vec<Align16>,
    base_ptr: SendSyncPtr<u8>,
    byte_len: usize,
}

impl MallocMemory {
    pub fn new(
        _ty: &wasmtime_environ::Memory,
        tunables: &Tunables,
        minimum: usize,
    ) -> Result<Self> {
        if tunables.memory_guard_size > 0 {
            bail!("malloc memory is only compatible if guard pages aren't used");
        }
        if tunables.memory_reservation > 0 {
            bail!("malloc memory is only compatible with no ahead-of-time memory reservation");
        }

        let byte_size = minimum
            .checked_add(
                tunables
                    .memory_reservation_for_growth
                    .try_into()
                    .err2anyhow()?,
            )
            .context("memory allocation size too large")?;

        let element_len = byte_size_to_element_len(byte_size);
        let mut storage = Vec::new();
        storage.try_reserve(element_len).err2anyhow()?;
        storage.resize(element_len, Align16(0));
        Ok(MallocMemory {
            base_ptr: SendSyncPtr::new(NonNull::new(storage.as_mut_ptr()).unwrap()).cast(),
            storage,
            byte_len: minimum,
        })
    }
}

impl RuntimeLinearMemory for MallocMemory {
    fn byte_size(&self) -> usize {
        self.byte_len
    }

    fn byte_capacity(&self) -> usize {
        self.storage.capacity() * mem::size_of::<Align16>()
    }

    fn grow_to(&mut self, new_size: usize) -> Result<()> {
        let new_element_len = byte_size_to_element_len(new_size);
        if new_element_len > self.storage.len() {
            self.storage
                .try_reserve(new_element_len - self.storage.len())
                .err2anyhow()?;
            self.storage.resize(new_element_len, Align16(0));
            self.base_ptr =
                SendSyncPtr::new(NonNull::new(self.storage.as_mut_ptr()).unwrap()).cast();
        }
        self.byte_len = new_size;
        Ok(())
    }

    fn base_ptr(&self) -> *mut u8 {
        self.base_ptr.as_ptr()
    }
}

fn byte_size_to_element_len(byte_size: usize) -> usize {
    let align = mem::align_of::<Align16>();

    // Round up the requested byte size to the size of each vector element.
    let byte_size_rounded_up =
        byte_size.checked_add(align - 1).unwrap_or(usize::MAX) & !(align - 1);

    // Next divide this aligned size by the size of each element to get the
    // element length of our vector.
    byte_size_rounded_up / align
}
