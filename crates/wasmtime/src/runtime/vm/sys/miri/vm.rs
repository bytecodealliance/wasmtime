use std::fs::File;
use std::io;
use std::sync::Arc;

pub unsafe fn expose_existing_mapping(ptr: *mut u8, len: usize) -> io::Result<()> {
    std::ptr::write_bytes(ptr, 0u8, len);
    Ok(())
}

pub unsafe fn hide_existing_mapping(ptr: *mut u8, len: usize) -> io::Result<()> {
    std::ptr::write_bytes(ptr, 0, len);
    Ok(())
}

pub unsafe fn erase_existing_mapping(ptr: *mut u8, len: usize) -> io::Result<()> {
    std::ptr::write_bytes(ptr, 0, len);
    Ok(())
}

#[cfg(feature = "pooling-allocator")]
pub unsafe fn commit_table_pages(ptr: *mut u8, len: usize) -> io::Result<()> {
    std::ptr::write_bytes(ptr, 0, len);
    Ok(())
}

#[cfg(feature = "pooling-allocator")]
pub unsafe fn decommit_table_pages(ptr: *mut u8, len: usize) -> io::Result<()> {
    std::ptr::write_bytes(ptr, 0, len);
    Ok(())
}

pub fn get_page_size() -> usize {
    4096
}

pub fn supports_madvise_dontneed() -> bool {
    false
}

pub unsafe fn madvise_dontneed(_ptr: *mut u8, _len: usize) -> io::Result<()> {
    unreachable!()
}

#[derive(PartialEq, Debug)]
pub enum MemoryImageSource {}

impl MemoryImageSource {
    pub fn from_file(_file: &Arc<File>) -> Option<MemoryImageSource> {
        None
    }

    pub fn from_data(_data: &[u8]) -> io::Result<Option<MemoryImageSource>> {
        Ok(None)
    }

    pub unsafe fn map_at(&self, _base: *mut u8, _len: usize, _offset: u64) -> io::Result<()> {
        match *self {}
    }

    pub unsafe fn remap_as_zeros_at(&self, _base: *mut u8, _len: usize) -> io::Result<()> {
        match *self {}
    }
}
