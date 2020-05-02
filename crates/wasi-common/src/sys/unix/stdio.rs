use crate::handle::{Handle, HandleRights};
use crate::sys::stdio::{Stderr, StderrExt, Stdin, StdinExt, Stdout, StdoutExt};
use crate::wasi::{types, RightsExt};
use std::cell::Cell;
use std::fs::File;
use std::io;
use std::mem::ManuallyDrop;
use std::os::unix::prelude::{AsRawFd, FromRawFd, RawFd};

impl AsRawFd for Stdin {
    fn as_raw_fd(&self) -> RawFd {
        io::stdin().as_raw_fd()
    }
}

impl AsRawFd for Stdout {
    fn as_raw_fd(&self) -> RawFd {
        io::stdout().as_raw_fd()
    }
}

impl AsRawFd for Stderr {
    fn as_raw_fd(&self) -> RawFd {
        io::stderr().as_raw_fd()
    }
}

impl StdinExt for Stdin {
    fn stdin() -> io::Result<Box<dyn Handle>> {
        let file = unsafe { File::from_raw_fd(io::stdin().as_raw_fd()) };
        let file = ManuallyDrop::new(file);
        let rights = get_rights(&file)?;
        let rights = Cell::new(rights);
        Ok(Box::new(Self { rights }))
    }
}

impl StdoutExt for Stdout {
    fn stdout() -> io::Result<Box<dyn Handle>> {
        let file = unsafe { File::from_raw_fd(io::stdout().as_raw_fd()) };
        let file = ManuallyDrop::new(file);
        let rights = get_rights(&file)?;
        let rights = Cell::new(rights);
        Ok(Box::new(Self { rights }))
    }
}

impl StderrExt for Stderr {
    fn stderr() -> io::Result<Box<dyn Handle>> {
        let file = unsafe { File::from_raw_fd(io::stderr().as_raw_fd()) };
        let file = ManuallyDrop::new(file);
        let rights = get_rights(&file)?;
        let rights = Cell::new(rights);
        Ok(Box::new(Self { rights }))
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
