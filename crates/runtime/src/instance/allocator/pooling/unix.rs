use anyhow::{bail, Context, Result};

fn decommit(addr: *mut u8, len: usize, protect: bool) -> Result<()> {
    if len == 0 {
        return Ok(());
    }

    if unsafe {
        libc::mmap(
            addr as _,
            len,
            if protect {
                libc::PROT_NONE
            } else {
                libc::PROT_READ | libc::PROT_WRITE
            },
            libc::MAP_PRIVATE | libc::MAP_ANON | libc::MAP_FIXED,
            -1,
            0,
        ) as *mut u8
    } != addr
    {
        bail!(
            "mmap failed to remap pages: {}",
            std::io::Error::last_os_error()
        );
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

pub fn commit_stack_pages(_addr: *mut u8, _len: usize) -> Result<()> {
    // A no-op as stack pages remain READ|WRITE
    Ok(())
}

pub fn decommit_stack_pages(addr: *mut u8, len: usize) -> Result<()> {
    decommit(addr, len, false)
}
