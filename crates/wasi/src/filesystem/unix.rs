use crate::filesystem::{Advice, DescriptorFlags};
use io_lifetimes::AsFilelike;
use rustix::fs::{OFlags, fcntl_getfl, fcntl_setfl};
use rustix::io::write;
use std::fs::File;
use std::io;
use std::os::fd::AsFd;
use std::os::unix::fs::FileExt;

pub(crate) fn get_flags(file: impl AsFd) -> io::Result<DescriptorFlags> {
    let flags = fcntl_getfl(file)?;
    let mut ret = DescriptorFlags::empty();
    ret.set(
        DescriptorFlags::REQUESTED_WRITE_SYNC,
        flags.contains(OFlags::DSYNC),
    );
    ret.set(
        DescriptorFlags::FILE_INTEGRITY_SYNC,
        flags.contains(OFlags::SYNC),
    );
    #[cfg(not(any(target_vendor = "apple", target_os = "freebsd")))]
    ret.set(
        DescriptorFlags::DATA_INTEGRITY_SYNC,
        flags.contains(OFlags::RSYNC),
    );
    Ok(ret)
}

pub(crate) fn advise(file: impl AsFd, offset: u64, len: u64, advice: Advice) -> io::Result<()> {
    cfg_if::cfg_if! {
        if #[cfg(target_vendor = "apple")] {
            match advice {
                Advice::WillNeed => {
                    rustix::fs::fcntl_rdadvise(file, offset, len)?;
                }
                Advice::Normal |
                Advice::Sequential |
                Advice::Random |
                Advice::DontNeed |
                Advice::NoReuse => {}
            }
        } else if #[cfg(any(target_os = "linux", target_os = "android"))] {
            use std::num::NonZeroU64;
            let advice = match advice {
                Advice::Normal => rustix::fs::Advice::Normal,
                Advice::Sequential => rustix::fs::Advice::Sequential,
                Advice::Random => rustix::fs::Advice::Random,
                Advice::WillNeed => rustix::fs::Advice::WillNeed,
                Advice::DontNeed => rustix::fs::Advice::DontNeed,
                Advice::NoReuse => rustix::fs::Advice::NoReuse,
            };
            rustix::fs::fadvise(file, offset, NonZeroU64::new(len), advice)?;
        } else {
            // noop on other platforms
            let _ = (file, offset, len, advice);
        }
    }
    Ok(())
}

pub(crate) fn append_cursor_unspecified(file: impl AsFd, data: &[u8]) -> io::Result<usize> {
    // On Linux, use `pwritev2`.
    #[cfg(target_os = "linux")]
    {
        use rustix::io::{Errno, ReadWriteFlags, pwritev2};
        use std::io::IoSlice;

        let iovs = [IoSlice::new(data)];
        match pwritev2(&file, &iovs, 0, ReadWriteFlags::APPEND) {
            Err(Errno::NOSYS) | Err(Errno::NOTSUP) => {}
            otherwise => return Ok(otherwise?),
        }
    }

    // Otherwise use `F_SETFL` to switch the file description to append
    // mode, do the write, and switch back. This is not atomic with
    // respect to other users of the file description, but WASI isn't fully
    // threaded right now anyway.
    let old_flags = fcntl_getfl(&file)?;
    fcntl_setfl(&file, old_flags | OFlags::APPEND)?;
    let result = write(&file, data);
    fcntl_setfl(&file, old_flags).unwrap();
    Ok(result?)
}

pub(crate) fn write_at_cursor_unspecified(
    file: impl AsFd,
    data: &[u8],
    pos: u64,
) -> io::Result<usize> {
    file.as_filelike_view::<File>().write_at(data, pos)
}

pub(crate) fn read_at_cursor_unspecified(
    file: impl AsFd,
    buf: &mut [u8],
    pos: u64,
) -> io::Result<usize> {
    file.as_filelike_view::<File>().read_at(buf, pos)
}
