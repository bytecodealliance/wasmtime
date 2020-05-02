use crate::handle::{Handle, HandleRights};
use crate::sys::stdio::{Stderr, StderrExt, Stdin, StdinExt, Stdout, StdoutExt};
use crate::wasi::{types, RightsExt};
use std::cell::Cell;
use std::io;
use std::os::windows::prelude::{AsRawHandle, RawHandle};

impl AsRawHandle for Stdin {
    fn as_raw_handle(&self) -> RawHandle {
        io::stdin().as_raw_handle()
    }
}

impl AsRawHandle for Stdout {
    fn as_raw_handle(&self) -> RawHandle {
        io::stdout().as_raw_handle()
    }
}

impl AsRawHandle for Stderr {
    fn as_raw_handle(&self) -> RawHandle {
        io::stderr().as_raw_handle()
    }
}

impl StdinExt for Stdin {
    fn stdin() -> io::Result<Box<dyn Handle>> {
        let rights = get_rights()?;
        let rights = Cell::new(rights);
        Ok(Box::new(Self { rights }))
    }
}

impl StdoutExt for Stdout {
    fn stdout() -> io::Result<Box<dyn Handle>> {
        let rights = get_rights()?;
        let rights = Cell::new(rights);
        Ok(Box::new(Self { rights }))
    }
}

impl StderrExt for Stderr {
    fn stderr() -> io::Result<Box<dyn Handle>> {
        let rights = get_rights()?;
        let rights = Cell::new(rights);
        Ok(Box::new(Self { rights }))
    }
}

fn get_rights() -> io::Result<HandleRights> {
    let rights = HandleRights::new(types::Rights::tty_base(), types::Rights::tty_base());
    Ok(rights)
}
