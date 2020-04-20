use crate::handle::{Handle, HandleRights};
use crate::sys::stdio::{Stdio, StdioExt};
use crate::wasi::{types, RightsExt};
use std::cell::Cell;
use std::fs::File;
use std::io;
use std::mem::ManuallyDrop;
use std::os::unix::prelude::{AsRawFd, FromRawFd, RawFd};

impl AsRawFd for Stdio {
    fn as_raw_fd(&self) -> RawFd {
        match self {
            Self::In { .. } => io::stdin().as_raw_fd(),
            Self::Out { .. } => io::stdout().as_raw_fd(),
            Self::Err { .. } => io::stderr().as_raw_fd(),
        }
    }
}

impl StdioExt for Stdio {
    fn stdin() -> io::Result<Box<dyn Handle>> {
        let file = unsafe { File::from_raw_fd(io::stdin().as_raw_fd()) };
        let file = ManuallyDrop::new(file);
        let rights = get_rights(&file)?;
        let rights = Cell::new(rights);
        Ok(Box::new(Self::In { rights }))
    }
    fn stdout() -> io::Result<Box<dyn Handle>> {
        let file = unsafe { File::from_raw_fd(io::stdin().as_raw_fd()) };
        let file = ManuallyDrop::new(file);
        let rights = get_rights(&file)?;
        let rights = Cell::new(rights);
        Ok(Box::new(Self::Out { rights }))
    }
    fn stderr() -> io::Result<Box<dyn Handle>> {
        let file = unsafe { File::from_raw_fd(io::stdin().as_raw_fd()) };
        let file = ManuallyDrop::new(file);
        let rights = get_rights(&file)?;
        let rights = Cell::new(rights);
        Ok(Box::new(Self::Err { rights }))
    }
}

fn get_rights(file: &File) -> io::Result<HandleRights> {
    use yanix::file::isatty;
    let (base, inheriting) = {
        if unsafe { isatty(file.as_raw_fd())? } {
            (types::Rights::tty_base(), types::Rights::tty_base())
        } else {
            (
                types::Rights::character_device_base(),
                types::Rights::character_device_inheriting(),
            )
        }
    };
    Ok(HandleRights::new(base, inheriting))
}
