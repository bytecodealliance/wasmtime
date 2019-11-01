use crate::fdentry::FdEntry;
use crate::sys::dev_null;
use crate::{wasi, Error, Result};
use std::borrow::Borrow;
use std::collections::HashMap;
use std::env;
use std::ffi::CString;
use std::fs::File;
use std::path::{Path, PathBuf};

/// A builder allowing customizable construction of `WasiCtx` instances.
pub struct WasiCtxBuilder {
    fds: HashMap<wasi::__wasi_fd_t, FdEntry>,
    preopens: Vec<(PathBuf, File)>,
    args: Vec<CString>,
    env: HashMap<CString, CString>,
}

impl WasiCtxBuilder {
    /// Builder for a new `WasiCtx`.
    pub fn new() -> Result<Self> {
        let mut builder = Self {
            fds: HashMap::new(),
            preopens: Vec::new(),
            args: vec![],
            env: HashMap::new(),
        };

        builder.fds.insert(0, FdEntry::from(dev_null()?)?);
        builder.fds.insert(1, FdEntry::from(dev_null()?)?);
        builder.fds.insert(2, FdEntry::from(dev_null()?)?);

        Ok(builder)
    }

    /// Add arguments to the command-line arguments list.
    pub fn args<S: AsRef<str>>(mut self, args: impl Iterator<Item = S>) -> Result<Self> {
        let args: Result<Vec<CString>> = args
            .map(|arg| CString::new(arg.as_ref()).map_err(|_| Error::ENOTCAPABLE))
            .collect();
        self.args = args?;
        Ok(self)
    }

    /// Add an argument to the command-line arguments list.
    pub fn arg(mut self, arg: &str) -> Result<Self> {
        self.args
            .push(CString::new(arg).map_err(|_| Error::ENOTCAPABLE)?);
        Ok(self)
    }

    /// Inherit the command-line arguments from the host process.
    pub fn inherit_args(self) -> Result<Self> {
        self.args(env::args())
    }

    /// Inherit the stdin, stdout, and stderr streams from the host process.
    pub fn inherit_stdio(mut self) -> Result<Self> {
        self.fds.insert(0, FdEntry::duplicate_stdin()?);
        self.fds.insert(1, FdEntry::duplicate_stdout()?);
        self.fds.insert(2, FdEntry::duplicate_stderr()?);
        Ok(self)
    }

    /// Inherit the environment variables from the host process.
    pub fn inherit_env(self) -> Result<Self> {
        self.envs(std::env::vars())
    }

    /// Add an entry to the environment.
    pub fn env<S: AsRef<str>>(mut self, k: S, v: S) -> Result<Self> {
        self.env.insert(
            CString::new(k.as_ref()).map_err(|_| Error::ENOTCAPABLE)?,
            CString::new(v.as_ref()).map_err(|_| Error::ENOTCAPABLE)?,
        );
        Ok(self)
    }

    /// Add entries to the environment.
    pub fn envs<S: AsRef<str>, T: Borrow<(S, S)>>(
        mut self,
        envs: impl Iterator<Item = T>,
    ) -> Result<Self> {
        let env: Result<HashMap<CString, CString>> = envs
            .map(|t| {
                let (k, v) = t.borrow();
                let k = CString::new(k.as_ref()).map_err(|_| Error::ENOTCAPABLE);
                let v = CString::new(v.as_ref()).map_err(|_| Error::ENOTCAPABLE);
                match (k, v) {
                    (Ok(k), Ok(v)) => Ok((k, v)),
                    _ => Err(Error::ENOTCAPABLE),
                }
            })
            .collect();
        self.env = env?;
        Ok(self)
    }

    /// Provide a File to use as stdin
    pub fn stdin(mut self, file: File) -> Result<Self> {
        self.fds.insert(0, FdEntry::from(file)?);
        Ok(self)
    }

    /// Provide a File to use as stdout
    pub fn stdout(mut self, file: File) -> Result<Self> {
        self.fds.insert(1, FdEntry::from(file)?);
        Ok(self)
    }

    /// Provide a File to use as stderr
    pub fn stderr(mut self, file: File) -> Result<Self> {
        self.fds.insert(2, FdEntry::from(file)?);
        Ok(self)
    }

    /// Add a preopened directory.
    pub fn preopened_dir<P: AsRef<Path>>(mut self, dir: File, guest_path: P) -> Self {
        self.preopens.push((guest_path.as_ref().to_owned(), dir));
        self
    }

    /// Build a `WasiCtx`, consuming this `WasiCtxBuilder`.
    pub fn build(mut self) -> Result<WasiCtx> {
        // startup code starts looking at fd 3 for preopens
        let mut preopen_fd = 3;
        for (guest_path, dir) in self.preopens {
            if !dir.metadata()?.is_dir() {
                return Err(Error::EBADF);
            }

            while self.fds.contains_key(&preopen_fd) {
                preopen_fd = preopen_fd.checked_add(1).ok_or(Error::ENFILE)?;
            }
            let mut fe = FdEntry::from(dir)?;
            fe.preopen_path = Some(guest_path);
            log::debug!("WasiCtx inserting ({:?}, {:?})", preopen_fd, fe);
            self.fds.insert(preopen_fd, fe);
            log::debug!("WasiCtx fds = {:?}", self.fds);
            preopen_fd = preopen_fd.checked_add(1).ok_or(Error::ENFILE)?;
        }

        let env = self
            .env
            .into_iter()
            .map(|(k, v)| {
                let mut pair = k.into_bytes();
                pair.push(b'=');
                pair.extend_from_slice(v.to_bytes_with_nul());
                // constructing a new CString from existing CStrings is safe
                unsafe { CString::from_vec_unchecked(pair) }
            })
            .collect();

        Ok(WasiCtx {
            fds: self.fds,
            args: self.args,
            env,
        })
    }
}

#[derive(Debug)]
pub struct WasiCtx {
    fds: HashMap<wasi::__wasi_fd_t, FdEntry>,
    pub(crate) args: Vec<CString>,
    pub(crate) env: Vec<CString>,
}

impl WasiCtx {
    /// Make a new `WasiCtx` with some default settings.
    ///
    /// - File descriptors 0, 1, and 2 inherit stdin, stdout, and stderr from the host process.
    ///
    /// - Environment variables are inherited from the host process.
    ///
    /// To override these behaviors, use `WasiCtxBuilder`.
    pub fn new<S: AsRef<str>>(args: impl Iterator<Item = S>) -> Result<Self> {
        WasiCtxBuilder::new()
            .and_then(|ctx| ctx.args(args))
            .and_then(|ctx| ctx.inherit_stdio())
            .and_then(|ctx| ctx.inherit_env())
            .and_then(|ctx| ctx.build())
    }

    /// Check if `WasiCtx` contains the specified raw WASI `fd`.
    pub(crate) unsafe fn contains_fd_entry(&self, fd: wasi::__wasi_fd_t) -> bool {
        self.fds.contains_key(&fd)
    }

    /// Get an immutable `FdEntry` corresponding to the specified raw WASI `fd`.
    pub(crate) unsafe fn get_fd_entry(&self, fd: wasi::__wasi_fd_t) -> Result<&FdEntry> {
        self.fds.get(&fd).ok_or(Error::EBADF)
    }

    /// Get a mutable `FdEntry` corresponding to the specified raw WASI `fd`.
    pub(crate) unsafe fn get_fd_entry_mut(
        &mut self,
        fd: wasi::__wasi_fd_t,
    ) -> Result<&mut FdEntry> {
        self.fds.get_mut(&fd).ok_or(Error::EBADF)
    }

    /// Insert the specified `FdEntry` into the `WasiCtx` object.
    ///
    /// The `FdEntry` will automatically get another free raw WASI `fd` assigned. Note that
    /// the two subsequent free raw WASI `fd`s do not have to be stored contiguously.
    pub(crate) fn insert_fd_entry(&mut self, fe: FdEntry) -> Result<wasi::__wasi_fd_t> {
        // never insert where stdio handles usually are
        let mut fd = 3;
        while self.fds.contains_key(&fd) {
            if let Some(next_fd) = fd.checked_add(1) {
                fd = next_fd;
            } else {
                return Err(Error::EMFILE);
            }
        }
        self.fds.insert(fd, fe);
        Ok(fd)
    }

    /// Insert the specified `FdEntry` with the specified raw WASI `fd` key into the `WasiCtx`
    /// object.
    pub(crate) fn insert_fd_entry_at(
        &mut self,
        fd: wasi::__wasi_fd_t,
        fe: FdEntry,
    ) -> Option<FdEntry> {
        self.fds.insert(fd, fe)
    }

    /// Remove `FdEntry` corresponding to the specified raw WASI `fd` from the `WasiCtx` object.
    pub(crate) fn remove_fd_entry(&mut self, fd: wasi::__wasi_fd_t) -> Result<FdEntry> {
        self.fds.remove(&fd).ok_or(Error::EBADF)
    }
}
