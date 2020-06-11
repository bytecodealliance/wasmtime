// The reason we have a separate Stdio wrappers is to correctly facilitate redirects on Windows.
// To elaborate further, in POSIX, we can get a stdio handle by opening a specific fd {0,1,2}.
// On Windows however, we need to issue a syscall that's separate from standard Windows "open"
// to get a console handle, and this is GetStdHandle. This is exactly what Rust does and what
// is wrapped inside their Stdio object in the libstd. We wrap it here as well because of this
// nuance on Windows:
//
//      The standard handles of a process may be redirected by a call to SetStdHandle, in which
//      case GetStdHandle returns the redirected handle.
//
// The MSDN also says this however:
//
//      If the standard handles have been redirected, you can specify the CONIN$ value in a call
//      to the CreateFile function to get a handle to a console's input buffer. Similarly, you
//      can specify the CONOUT$ value to get a handle to a console's active screen buffer.
//
// TODO it might worth re-investigating the suitability of this type on Windows.

use super::{fd, AsFile};
use crate::handle::{Handle, HandleRights};
use crate::sandboxed_tty_writer::SandboxedTTYWriter;
use crate::wasi::types::{self, Filetype};
use crate::wasi::{Errno, Result, RightsExt};
use std::any::Any;
use std::cell::Cell;
use std::convert::TryInto;
use std::io::{self, Read, Write};

pub(crate) trait StdinExt: Sized {
    /// Create `Stdin` from `io::stdin`.
    fn stdin() -> io::Result<Box<dyn Handle>>;
}

#[derive(Debug, Clone)]
pub(crate) struct Stdin {
    pub(crate) file_type: Filetype,
    pub(crate) rights: Cell<HandleRights>,
}

impl Handle for Stdin {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn try_clone(&self) -> io::Result<Box<dyn Handle>> {
        Ok(Box::new(self.clone()))
    }
    fn get_file_type(&self) -> Filetype {
        self.file_type
    }
    fn get_rights(&self) -> HandleRights {
        self.rights.get()
    }
    fn set_rights(&self, new_rights: HandleRights) {
        self.rights.set(new_rights)
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
        let nread = io::stdin().read_vectored(iovs)?;
        Ok(nread)
    }
}

pub(crate) trait StdoutExt: Sized {
    /// Create `Stdout` from `io::stdout`.
    fn stdout() -> io::Result<Box<dyn Handle>>;
}

#[derive(Debug, Clone)]
pub(crate) struct Stdout {
    pub(crate) file_type: Filetype,
    pub(crate) rights: Cell<HandleRights>,
}

impl Handle for Stdout {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn try_clone(&self) -> io::Result<Box<dyn Handle>> {
        Ok(Box::new(self.clone()))
    }
    fn get_file_type(&self) -> Filetype {
        self.file_type
    }
    fn get_rights(&self) -> HandleRights {
        self.rights.get()
    }
    fn set_rights(&self, new_rights: HandleRights) {
        self.rights.set(new_rights)
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
    fn write_vectored(&self, iovs: &[io::IoSlice]) -> Result<usize> {
        // lock for the duration of the scope
        let stdout = io::stdout();
        let mut stdout = stdout.lock();
        let nwritten = if self.is_tty() {
            SandboxedTTYWriter::new(&mut stdout).write_vectored(&iovs)?
        } else {
            stdout.write_vectored(iovs)?
        };
        stdout.flush()?;
        Ok(nwritten)
    }
}

pub(crate) trait StderrExt: Sized {
    /// Create `Stderr` from `io::stderr`.
    fn stderr() -> io::Result<Box<dyn Handle>>;
}

#[derive(Debug, Clone)]
pub(crate) struct Stderr {
    pub(crate) file_type: Filetype,
    pub(crate) rights: Cell<HandleRights>,
}

impl Handle for Stderr {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn try_clone(&self) -> io::Result<Box<dyn Handle>> {
        Ok(Box::new(self.clone()))
    }
    fn get_file_type(&self) -> Filetype {
        self.file_type
    }
    fn get_rights(&self) -> HandleRights {
        self.rights.get()
    }
    fn set_rights(&self, new_rights: HandleRights) {
        self.rights.set(new_rights)
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
    fn write_vectored(&self, iovs: &[io::IoSlice]) -> Result<usize> {
        // Always sanitize stderr, even if it's not directly connected to a tty,
        // because stderr is meant for diagnostics rather than binary output,
        // and may be redirected to a file which could end up being displayed
        // on a tty later.
        let nwritten = SandboxedTTYWriter::new(&mut io::stderr()).write_vectored(&iovs)?;
        Ok(nwritten)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct NullDevice {
    pub(crate) rights: Cell<HandleRights>,
    pub(crate) fd_flags: Cell<types::Fdflags>,
}

impl NullDevice {
    pub(crate) fn new() -> Self {
        let rights = HandleRights::new(
            types::Rights::character_device_base(),
            types::Rights::character_device_inheriting(),
        );
        let rights = Cell::new(rights);
        let fd_flags = types::Fdflags::empty();
        let fd_flags = Cell::new(fd_flags);
        Self { rights, fd_flags }
    }
}

impl Handle for NullDevice {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn try_clone(&self) -> io::Result<Box<dyn Handle>> {
        Ok(Box::new(self.clone()))
    }
    fn get_file_type(&self) -> types::Filetype {
        types::Filetype::CharacterDevice
    }
    fn get_rights(&self) -> HandleRights {
        self.rights.get()
    }
    fn set_rights(&self, rights: HandleRights) {
        self.rights.set(rights)
    }
    // FdOps
    fn fdstat_get(&self) -> Result<types::Fdflags> {
        Ok(self.fd_flags.get())
    }
    fn fdstat_set_flags(&self, fdflags: types::Fdflags) -> Result<()> {
        self.fd_flags.set(fdflags);
        Ok(())
    }
    fn read_vectored(&self, _iovs: &mut [io::IoSliceMut]) -> Result<usize> {
        Ok(0)
    }
    fn write_vectored(&self, iovs: &[io::IoSlice]) -> Result<usize> {
        let mut total_len = 0u32;
        for iov in iovs {
            let len: types::Size = iov.len().try_into()?;
            total_len = total_len.checked_add(len).ok_or(Errno::Overflow)?;
        }
        Ok(total_len as usize)
    }
}
