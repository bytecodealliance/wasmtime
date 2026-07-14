use crate::filesystem::{
    Advice, DescriptorFlags, DescriptorStat, DescriptorType, MetadataHashValue,
};
use cap_primitives::fs::{
    FileType, FileTypeExt, FollowSymlinks, Metadata, MetadataExt, OpenOptions,
};
use rustix::fs::{OFlags, fcntl_getfl, fcntl_setfl};
use rustix::io::write;
use std::fs::File;
use std::io;
use std::os::unix::fs::FileExt;
use std::path::Path;

pub use cap_primitives::fs::remove_file as remove_file_or_symlink;
pub use cap_primitives::fs::symlink;

pub(crate) fn get_flags(file: &File) -> io::Result<DescriptorFlags> {
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

pub(crate) fn advise(file: &File, offset: u64, len: u64, advice: Advice) -> io::Result<()> {
    cfg_select! {
        target_vendor = "apple" => {
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
        }
        any(target_os = "linux", target_os = "android") => {
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
        }
        _ => {
            // noop on other platforms
            let _ = (file, offset, len, advice);
        }
    }
    Ok(())
}

pub(crate) fn append_cursor_unspecified(file: &File, data: &[u8]) -> io::Result<usize> {
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

pub(crate) fn write_at_cursor_unspecified(file: &File, data: &[u8], pos: u64) -> io::Result<usize> {
    file.write_at(data, pos)
}

pub(crate) fn read_at_cursor_unspecified(
    file: &File,
    buf: &mut [u8],
    pos: u64,
) -> io::Result<usize> {
    file.read_at(buf, pos)
}

fn meta_identity(meta: &Metadata) -> (u64, u64) {
    (meta.dev(), meta.ino())
}

fn file_identity(file: &File) -> io::Result<(u64, u64)> {
    let meta = Metadata::from_file(file)?;
    Ok(meta_identity(&meta))
}

pub(crate) fn metadata_hash(file: &File) -> io::Result<MetadataHashValue> {
    Ok(MetadataHashValue::new(file_identity(file)?))
}

pub(crate) fn metadata_hash_at(
    start: &File,
    path: &Path,
    follow: FollowSymlinks,
) -> io::Result<MetadataHashValue> {
    let meta = cap_primitives::fs::stat(start, path, follow)?;
    Ok(MetadataHashValue::new(meta_identity(&meta)))
}

pub(crate) fn is_same_file(a: &File, b: &File) -> io::Result<bool> {
    Ok(file_identity(a)? == file_identity(b)?)
}

pub(crate) fn stat(f: &std::fs::File) -> io::Result<DescriptorStat> {
    let meta = Metadata::from_file(f)?;
    Ok(DescriptorStat::new(&meta, meta.nlink()))
}

pub(crate) fn stat_at(
    start: &File,
    path: &Path,
    follow: FollowSymlinks,
) -> io::Result<DescriptorStat> {
    let meta = cap_primitives::fs::stat(start, path, follow)?;
    Ok(DescriptorStat::new(&meta, meta.nlink()))
}

pub(crate) fn maybe_dir(opts: &mut OpenOptions) {
    let _ = opts;
}

pub(crate) fn descriptor_type(ft: FileType) -> DescriptorType {
    if ft.is_block_device() {
        DescriptorType::BlockDevice
    } else if ft.is_char_device() {
        DescriptorType::CharacterDevice
    } else {
        DescriptorType::Unknown
    }
}
