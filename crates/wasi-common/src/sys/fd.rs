use crate::handle::{Fstflags, Timestamp};
use crate::{Error, Result};
use filetime::{set_file_handle_times, FileTime};
use std::fs::File;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub(crate) use super::sys_impl::fd::*;

pub(crate) fn filestat_set_times(
    file: &File,
    st_atim: Timestamp,
    st_mtim: Timestamp,
    fst_flags: Fstflags,
) -> Result<()> {
    let set_atim = fst_flags.contains(&Fstflags::ATIM);
    let set_atim_now = fst_flags.contains(&Fstflags::ATIM_NOW);
    let set_mtim = fst_flags.contains(&Fstflags::MTIM);
    let set_mtim_now = fst_flags.contains(&Fstflags::MTIM_NOW);

    if (set_atim && set_atim_now) || (set_mtim && set_mtim_now) {
        return Err(Error::Inval);
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

    set_file_handle_times(file, atim, mtim)?;
    Ok(())
}
