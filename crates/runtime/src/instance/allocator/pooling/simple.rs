//! A simple index allocator.
//!
//! This index allocator doesn't do any module affinity or anything like that,
//! however it is built on top of the `ModuleAffinityIndexAllocator` to save
//! code (and code size).

use super::module_affinity::{ModuleAffinityIndexAllocator, SlotId};

#[derive(Debug)]
pub struct SimpleIndexAllocator(ModuleAffinityIndexAllocator);

impl SimpleIndexAllocator {
    pub fn new(max_instances: u32) -> Self {
        SimpleIndexAllocator(ModuleAffinityIndexAllocator::new(max_instances, 0))
    }

    pub fn alloc(&self) -> Option<SlotId> {
        self.0.alloc(None)
    }

    pub(crate) fn free(&self, index: SlotId) {
        self.0.free(index);
    }
}
