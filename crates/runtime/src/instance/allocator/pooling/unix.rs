use anyhow::{Context, Result};

fn decommit(addr: *mut u8, len: usize) -> Result<()> {
    if len == 0 {
        return Ok(());
    }

    unsafe {
        cfg_if::cfg_if! {
            if #[cfg(target_os = "linux")] {
                use rustix::mm::{madvise, Advice};

                // On Linux, this is enough to cause the kernel to initialize
                // the pages to 0 on next access
                madvise(addr as _, len, Advice::LinuxDontNeed)
                    .context("madvise failed to decommit: {}")?;
            } else {
                use rustix::mm::{mmap_anonymous, ProtFlags, MapFlags};

                // By creating a new mapping at the same location, this will
                // discard the mapping for the pages in the given range.
                // The new mapping will be to the CoW zero page, so this
                // effectively zeroes the pages.
                mmap_anonymous(
                    addr as _,
                    len,
                    ProtFlags::READ | ProtFlags::WRITE,
                    MapFlags::PRIVATE | MapFlags::FIXED,
                )
                .context("mmap failed to remap pages: {}")?;
            }
        }
    }

    Ok(())
}

pub fn commit_table_pages(_addr: *mut u8, _len: usize) -> Result<()> {
    // A no-op as table pages remain READ|WRITE
    Ok(())
}

pub fn decommit_table_pages(addr: *mut u8, len: usize) -> Result<()> {
    decommit(addr, len)
}

#[cfg(feature = "async")]
pub fn commit_stack_pages(_addr: *mut u8, _len: usize) -> Result<()> {
    // A no-op as stack pages remain READ|WRITE
    Ok(())
}

#[cfg(feature = "async")]
pub fn reset_stack_pages_to_zero(addr: *mut u8, len: usize) -> Result<()> {
    decommit(addr, len)
}
