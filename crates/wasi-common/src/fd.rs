use crate::entry::Descriptor;
use crate::sys;
use crate::wasi::{types, Errno, Result};
use filetime::{set_file_handle_times, FileTime};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub(crate) use sys::fd::*;

pub(crate) fn filestat_set_times_impl(
    file: &Descriptor,
    st_atim: types::Timestamp,
    st_mtim: types::Timestamp,
    fst_flags: types::Fstflags,
) -> Result<()> {
    let set_atim = fst_flags.contains(&types::Fstflags::ATIM);
    let set_atim_now = fst_flags.contains(&types::Fstflags::ATIM_NOW);
    let set_mtim = fst_flags.contains(&types::Fstflags::MTIM);
    let set_mtim_now = fst_flags.contains(&types::Fstflags::MTIM_NOW);

    if (set_atim && set_atim_now) || (set_mtim && set_mtim_now) {
        return Err(Errno::Inval);
    }
    let atim = if set_atim {
        let time = UNIX_EPOCH + Duration::from_nanos(st_atim);
        Some(FileTime::from_system_time(time))
    } else if set_atim_now {
        let time = SystemTime::now();
        Some(FileTime::from_system_time(time))
    } else {
        None
    };

    let mtim = if set_mtim {
        let time = UNIX_EPOCH + Duration::from_nanos(st_mtim);
        Some(FileTime::from_system_time(time))
    } else if set_mtim_now {
        let time = SystemTime::now();
        Some(FileTime::from_system_time(time))
    } else {
        None
    };
    match file {
        Descriptor::OsHandle(fd) => set_file_handle_times(fd, atim, mtim).map_err(Into::into),
        Descriptor::VirtualFile(virt) => virt.filestat_set_times(atim, mtim),
        _ => {
            unreachable!(
                "implementation error: fd should have been checked to not be a stream already"
            );
        }
    }
}
