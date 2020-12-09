use crate::Mmap;
use winapi::um::memoryapi::{VirtualAlloc, VirtualFree};
use winapi::um::winnt::{MEM_COMMIT, MEM_DECOMMIT, PAGE_READWRITE};

pub unsafe fn make_accessible(addr: *mut u8, len: usize) -> bool {
    // This doesn't use the `region` crate because the memory needs to be committed
    !VirtualAlloc(addr as _, len, MEM_COMMIT, PAGE_READWRITE).is_null()
}

pub unsafe fn decommit(addr: *mut u8, len: usize) {
    assert!(
        VirtualFree(addr as _, len, MEM_DECOMMIT) != 0,
        "failed to decommit memory pages: {}",
        std::io::Error::last_os_error()
    );
}

pub fn create_memory_map(accessible_size: usize, mapping_size: usize) -> Result<Mmap, String> {
    Mmap::accessible_reserved(accessible_size, mapping_size)
        .map_err(|e| format!("failed to allocate pool memory: {}", e))
}
