pub(crate) mod clock;
pub(crate) mod fd;
pub(crate) mod osdir;
pub(crate) mod osfile;
pub(crate) mod osother;
pub(crate) mod stdio;

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(unix)] {
        mod unix;
        use unix as sys_impl;
        pub use unix::preopen_dir;
    } else if #[cfg(windows)] {
        mod windows;
        use windows as sys_impl;
        pub use windows::preopen_dir;
    } else {
        compile_error!("wasi-common doesn't compile for this platform yet");
    }
}

pub(crate) use sys_impl::path;
pub(crate) use sys_impl::poll;

use super::handle::Handle;
use crate::wasi::types;
use osdir::OsDir;
use osfile::OsFile;
use osother::OsOther;
use std::convert::TryFrom;
use std::fs::File;
use std::io;
use std::mem::ManuallyDrop;
use sys_impl::get_file_type;

pub(crate) trait AsFile {
    fn as_file(&self) -> ManuallyDrop<File>;
}

impl TryFrom<File> for Box<dyn Handle> {
    type Error = io::Error;

    fn try_from(file: File) -> io::Result<Self> {
        let file_type = get_file_type(&file)?;
        match file_type {
            types::Filetype::RegularFile => {
                let handle = OsFile::try_from(file)?;
                log::debug!("Created new instance of OsFile: {:?}", handle);
                Ok(Box::new(handle))
            }
            types::Filetype::Directory => {
                let handle = OsDir::try_from(file)?;
                log::debug!("Created new instance of OsDir: {:?}", handle);
                Ok(Box::new(handle))
            }
            _ => {
                let handle = OsOther::try_from(file)?;
                log::debug!("Created new instance of OsOther: {:?}", handle);
                Ok(Box::new(handle))
            }
        }
    }
}
