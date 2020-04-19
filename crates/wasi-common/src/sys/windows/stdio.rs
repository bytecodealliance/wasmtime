use crate::handle::{Handle, HandleRights};
use crate::sys::stdio::{Stdio, StdioExt};
use crate::wasi::{types, RightsExt};
use std::cell::Cell;
use std::io;
use std::os::windows::prelude::{AsRawHandle, RawHandle};

impl AsRawHandle for Stdio {
    fn as_raw_handle(&self) -> RawHandle {
        match self {
            Self::In { .. } => io::stdin().as_raw_handle(),
            Self::Out { .. } => io::stdout().as_raw_handle(),
            Self::Err { .. } => io::stderr().as_raw_handle(),
        }
    }
}

impl StdioExt for Stdio {
    fn stdin() -> io::Result<Box<dyn Handle>> {
        let rights = get_rights()?;
        let rights = Cell::new(rights);
        Ok(Box::new(Self::In { rights }))
    }
    fn stdout() -> io::Result<Box<dyn Handle>> {
        let rights = get_rights()?;
        let rights = Cell::new(rights);
        Ok(Box::new(Self::Out { rights }))
    }
    fn stderr() -> io::Result<Box<dyn Handle>> {
        let rights = get_rights()?;
        let rights = Cell::new(rights);
        Ok(Box::new(Self::Err { rights }))
    }
}

fn get_rights() -> io::Result<HandleRights> {
    let rights = HandleRights::new(types::Rights::tty_base(), types::Rights::tty_base());
    Ok(rights)
}
