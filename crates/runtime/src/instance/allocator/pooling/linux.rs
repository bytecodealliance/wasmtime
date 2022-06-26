use anyhow::{Context, Result};

fn decommit(addr: *mut u8, len: usize, protect: bool) -> Result<()> {
    if len == 0 {
        return Ok(());
    }

    unsafe {
        if protect {
            region::protect(addr, len, region::Protection::NONE)
                .context("failed to protect memory pages")?;
        }

        // On Linux, this is enough to cause the kernel to initialize the pages to 0 on next access
        rustix::mm::madvise(addr as _, len, rustix::mm::Advice::LinuxDontNeed)
            .context("madvise failed to decommit: {}")?;
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
