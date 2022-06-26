use anyhow::{Context, Result};

fn decommit(addr: *mut u8, len: usize, protect: bool) -> Result<()> {
    if len == 0 {
        return Ok(());
    }

    // By creating a new mapping at the same location, this will discard the
    // mapping for the pages in the given range.
    // The new mapping will be to the CoW zero page, so this effectively
    // zeroes the pages.
    unsafe {
        rustix::mm::mmap_anonymous(
            addr as _,
            len,
            if protect {
                rustix::mm::ProtFlags::empty()
            } else {
                rustix::mm::ProtFlags::READ | rustix::mm::ProtFlags::WRITE
            },
            rustix::mm::MapFlags::PRIVATE | rustix::mm::MapFlags::FIXED,
        )
        .context("mmap failed to remap pages: {}")?;
    }

    Ok(())
}

pub fn commit_memory_pages(addr: *mut u8, len: usize) -> Result<()> {
    if len == 0 {
        return Ok(());
    }

    // Just change the protection level to READ|WRITE
    unsafe {
        region::protect(addr, len, region::Protection::READ_WRITE)
            .context("failed to make linear memory pages read/write")
    }
}

pub fn decommit_memory_pages(addr: *mut u8, len: usize) -> Result<()> {
    decommit(addr, len, true)
}

pub fn commit_table_pages(_addr: *mut u8, _len: usize) -> Result<()> {
    // A no-op as table pages remain READ|WRITE
    Ok(())
}

pub fn decommit_table_pages(addr: *mut u8, len: usize) -> Result<()> {
    decommit(addr, len, false)
}

#[cfg(feature = "async")]
pub fn commit_stack_pages(_addr: *mut u8, _len: usize) -> Result<()> {
    // A no-op as stack pages remain READ|WRITE
    Ok(())
}

#[cfg(feature = "async")]
pub fn decommit_stack_pages(addr: *mut u8, len: usize) -> Result<()> {
    decommit(addr, len, false)
}
