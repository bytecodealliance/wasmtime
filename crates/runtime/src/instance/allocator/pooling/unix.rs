use crate::Mmap;

pub unsafe fn make_accessible(addr: *mut u8, len: usize) -> bool {
    region::protect(addr, len, region::Protection::READ_WRITE).is_ok()
}

pub unsafe fn decommit(addr: *mut u8, len: usize) {
    assert_eq!(
        libc::mmap(
            addr as _,
            len,
            libc::PROT_NONE,
            libc::MAP_PRIVATE | libc::MAP_ANON | libc::MAP_FIXED,
            -1,
            0,
        ) as *mut u8,
        addr,
        "mmap failed to remap pages: {}",
        std::io::Error::last_os_error()
    );
}

pub fn create_memory_map(accessible_size: usize, mapping_size: usize) -> Result<Mmap, String> {
    Mmap::accessible_reserved(accessible_size, mapping_size)
        .map_err(|e| format!("failed to allocate pool memory: {}", e))
}
