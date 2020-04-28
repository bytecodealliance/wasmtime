use super::{fd, AsFile};
use crate::handle::{Handle, HandleRights};
use crate::sandboxed_tty_writer::SandboxedTTYWriter;
use crate::wasi::types::{self, Filetype};
use crate::wasi::{Errno, Result};
use std::any::Any;
use std::cell::Cell;
use std::io::{self, Read, Write};

pub(crate) trait StdioExt: Sized {
    /// Create `Stdio` from `io::stdin`.
    fn stdin() -> io::Result<Box<dyn Handle>>;
    /// Create `Stdio` from `io::stdout`.
    fn stdout() -> io::Result<Box<dyn Handle>>;
    /// Create `Stdio` from `io::stderr`.
    fn stderr() -> io::Result<Box<dyn Handle>>;
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) enum Stdio {
    In { rights: Cell<HandleRights> },
    Out { rights: Cell<HandleRights> },
    Err { rights: Cell<HandleRights> },
}

impl Handle for Stdio {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn try_clone(&self) -> io::Result<Box<dyn Handle>> {
        Ok(Box::new(self.clone()))
    }
    fn get_file_type(&self) -> Filetype {
        Filetype::CharacterDevice
    }
    fn get_rights(&self) -> HandleRights {
        match self {
            Self::In { rights } => rights.get(),
            Self::Out { rights } => rights.get(),
            Self::Err { rights } => rights.get(),
        }
    }
    fn set_rights(&self, new_rights: HandleRights) {
        match self {
            Self::In { rights } => rights.set(new_rights),
            Self::Out { rights } => rights.set(new_rights),
            Self::Err { rights } => rights.set(new_rights),
        }
    }
    // FdOps
    fn fdstat_get(&self) -> Result<types::Fdflags> {
        fd::fdstat_get(&*self.as_file()?)
    }
    fn fdstat_set_flags(&self, fdflags: types::Fdflags) -> Result<()> {
        if let Some(_) = fd::fdstat_set_flags(&*self.as_file()?, fdflags)? {
            // OK, this means we should somehow update the underlying os handle,
            // and we can't do that with `std::io::std{in, out, err}`, so we'll
            // panic for now.
            panic!("Tried updating Fdflags on Stdio handle by re-opening as file!");
        }
        Ok(())
    }
    fn read_vectored(&self, iovs: &mut [io::IoSliceMut]) -> Result<usize> {
        let nread = match self {
            Self::In { .. } => io::stdin().read_vectored(iovs)?,
            _ => return Err(Errno::Badf),
        };
        Ok(nread)
    }
    fn write_vectored(&self, iovs: &[io::IoSlice]) -> Result<usize> {
        let nwritten = match self {
            Self::In { .. } => return Err(Errno::Badf),
            Self::Out { .. } => {
                // lock for the duration of the scope
                let stdout = io::stdout();
                let mut stdout = stdout.lock();
                let nwritten = if self.is_tty() {
                    SandboxedTTYWriter::new(&mut stdout).write_vectored(&iovs)?
                } else {
                    stdout.write_vectored(&iovs)?
                };
                stdout.flush()?;
                nwritten
            }
            // Always sanitize stderr, even if it's not directly connected to a tty,
            // because stderr is meant for diagnostics rather than binary output,
            // and may be redirected to a file which could end up being displayed
            // on a tty later.
            Self::Err { .. } => SandboxedTTYWriter::new(&mut io::stderr()).write_vectored(&iovs)?,
        };
        Ok(nwritten)
    }
}
