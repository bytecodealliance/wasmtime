use super::{get_file_type, get_rights};
use crate::handle::Handle;
use crate::sys::stdio::{Stderr, StderrExt, Stdin, StdinExt, Stdout, StdoutExt};
use std::cell::Cell;
use std::fs::File;
use std::io;
use std::mem::ManuallyDrop;
use std::os::windows::prelude::{AsRawHandle, FromRawHandle, RawHandle};

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
        let file = unsafe { File::from_raw_handle(io::stdin().as_raw_handle()) };
        let file = ManuallyDrop::new(file);
        let file_type = get_file_type(&file)?;
        let rights = get_rights(&file_type)?;
        let rights = Cell::new(rights);
        Ok(Box::new(Self { file_type, rights }))
    }
}

impl StdoutExt for Stdout {
    fn stdout() -> io::Result<Box<dyn Handle>> {
        let file = unsafe { File::from_raw_handle(io::stdin().as_raw_handle()) };
        let file = ManuallyDrop::new(file);
        let file_type = get_file_type(&file)?;
        let rights = get_rights(&file_type)?;
        let rights = Cell::new(rights);
        Ok(Box::new(Self { file_type, rights }))
    }
}

impl StderrExt for Stderr {
    fn stderr() -> io::Result<Box<dyn Handle>> {
        let file = unsafe { File::from_raw_handle(io::stdin().as_raw_handle()) };
        let file = ManuallyDrop::new(file);
        let file_type = get_file_type(&file)?;
        let rights = get_rights(&file_type)?;
        let rights = Cell::new(rights);
        Ok(Box::new(Self { file_type, rights }))
    }
}
