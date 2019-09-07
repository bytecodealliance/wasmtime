use crate::sys::fdentry_impl;
use crate::{host, Error, Result};

use std::mem::ManuallyDrop;
use std::path::PathBuf;
use std::{fs, io};

#[derive(Debug)]
pub enum Descriptor {
    File(fs::File),
    Stdin,
    Stdout,
    Stderr,
}

impl Descriptor {
    pub fn as_file(&self) -> Result<&fs::File> {
        match self {
            Descriptor::File(f) => Ok(f),
            _ => Err(Error::EBADF),
        }
    }

    pub fn is_file(&self) -> bool {
        match self {
            Descriptor::File(_) => true,
            _ => false,
        }
    }

    pub fn is_stdin(&self) -> bool {
        match self {
            Descriptor::Stdin => true,
            _ => false,
        }
    }

    pub fn is_stdout(&self) -> bool {
        match self {
            Descriptor::Stdout => true,
            _ => false,
        }
    }

    pub fn is_stderr(&self) -> bool {
        match self {
            Descriptor::Stderr => true,
            _ => false,
        }
    }
}

#[derive(Debug)]
pub struct FdObject {
    pub file_type: host::__wasi_filetype_t,
    pub descriptor: ManuallyDrop<Descriptor>,
    pub needs_close: bool,
    // TODO: directories
}

#[derive(Debug)]
pub struct FdEntry {
    pub fd_object: FdObject,
    pub rights_base: host::__wasi_rights_t,
    pub rights_inheriting: host::__wasi_rights_t,
    pub preopen_path: Option<PathBuf>,
}

impl Drop for FdObject {
    fn drop(&mut self) {
        if self.needs_close {
            unsafe { ManuallyDrop::drop(&mut self.descriptor) };
        }
    }
}

impl FdEntry {
    pub fn from(file: fs::File) -> Result<Self> {
        fdentry_impl::determine_type_and_access_rights(&file).map(
            |(file_type, rights_base, rights_inheriting)| Self {
                fd_object: FdObject {
                    file_type,
                    descriptor: ManuallyDrop::new(Descriptor::File(file)),
                    needs_close: true,
                },
                rights_base,
                rights_inheriting,
                preopen_path: None,
            },
        )
    }

    pub fn duplicate(file: &fs::File) -> Result<Self> {
        Self::from(file.try_clone()?)
    }

    pub fn duplicate_stdin() -> Result<Self> {
        fdentry_impl::determine_type_and_access_rights(&io::stdin()).map(
            |(file_type, rights_base, rights_inheriting)| Self {
                fd_object: FdObject {
                    file_type,
                    descriptor: ManuallyDrop::new(Descriptor::Stdin),
                    needs_close: true,
                },
                rights_base,
                rights_inheriting,
                preopen_path: None,
            },
        )
    }

    pub fn duplicate_stdout() -> Result<Self> {
        fdentry_impl::determine_type_and_access_rights(&io::stdout()).map(
            |(file_type, rights_base, rights_inheriting)| Self {
                fd_object: FdObject {
                    file_type,
                    descriptor: ManuallyDrop::new(Descriptor::Stdout),
                    needs_close: true,
                },
                rights_base,
                rights_inheriting,
                preopen_path: None,
            },
        )
    }

    pub fn duplicate_stderr() -> Result<Self> {
        fdentry_impl::determine_type_and_access_rights(&io::stderr()).map(
            |(file_type, rights_base, rights_inheriting)| Self {
                fd_object: FdObject {
                    file_type,
                    descriptor: ManuallyDrop::new(Descriptor::Stderr),
                    needs_close: true,
                },
                rights_base,
                rights_inheriting,
                preopen_path: None,
            },
        )
    }
}
