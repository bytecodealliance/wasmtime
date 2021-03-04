use crate::Mmap;
use anyhow::{anyhow, Result};

pub unsafe fn make_accessible(addr: *mut u8, len: usize) -> bool {
    region::protect(addr, len, region::Protection::READ_WRITE).is_ok()
}

pub unsafe fn decommit(addr: *mut u8, len: usize) {
    region::protect(addr, len, region::Protection::NONE).unwrap();

    // On Linux, this is enough to cause the kernel to initialize the pages to 0 on next access
    assert_eq!(
        libc::madvise(addr as _, len, libc::MADV_DONTNEED),
        0,
        "madvise failed to mark pages as missing: {}",
        std::io::Error::last_os_error()
    );
}

pub fn create_memory_map(accessible_size: usize, mapping_size: usize) -> Result<Mmap> {
    Mmap::accessible_reserved(accessible_size, mapping_size)
        .map_err(|e| anyhow!("failed to allocate pool memory: {}", e))
}
