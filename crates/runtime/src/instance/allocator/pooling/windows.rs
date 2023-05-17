use anyhow::{bail, Result};
use windows_sys::Win32::System::Memory::*;

pub fn commit(addr: *mut u8, len: usize) -> Result<()> {
    if len == 0 {
        return Ok(());
    }

    // Memory needs to be committed, so don't use the `region` crate
    if unsafe { VirtualAlloc(addr as _, len, MEM_COMMIT, PAGE_READWRITE).is_null() } {
        bail!("failed to commit memory as read/write");
    }

    Ok(())
}

pub fn decommit(addr: *mut u8, len: usize) -> Result<()> {
    if len == 0 {
        return Ok(());
    }

    if unsafe { VirtualFree(addr as _, len, MEM_DECOMMIT) } == 0 {
        bail!(
            "failed to decommit memory pages: {}",
            std::io::Error::last_os_error()
        );
    }

    Ok(())
}

pub fn commit_table_pages(addr: *mut u8, len: usize) -> Result<()> {
    commit(addr, len)
}

pub fn decommit_table_pages(addr: *mut u8, len: usize) -> Result<()> {
    decommit(addr, len)
}
