use crate::old::snapshot_0::fdentry::FdEntry;
use crate::old::snapshot_0::{wasi, Error, Result};
use std::borrow::Borrow;
use std::collections::HashMap;
use std::env;
use std::ffi::{CString, OsString};
use std::fs::File;
use std::path::{Path, PathBuf};

enum PendingFdEntry {
    Thunk(fn() -> Result<FdEntry>),
    File(File),
}

impl std::fmt::Debug for PendingFdEntry {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Thunk(f) => write!(
                fmt,
                "PendingFdEntry::Thunk({:p})",
                f as *const fn() -> Result<FdEntry>
            ),
            Self::File(f) => write!(fmt, "PendingFdEntry::File({:?})", f),
        }
    }
}

#[derive(Debug, Eq, Hash, PartialEq)]
enum PendingCString {
    Bytes(Vec<u8>),
    OsString(OsString),
}

impl From<Vec<u8>> for PendingCString {
    fn from(bytes: Vec<u8>) -> Self {
        Self::Bytes(bytes)
    }
}

impl From<OsString> for PendingCString {
    fn from(s: OsString) -> Self {
        Self::OsString(s)
    }
}

impl PendingCString {
    fn into_string(self) -> Result<String> {
        match self {
            Self::Bytes(v) => String::from_utf8(v).map_err(|_| Error::EILSEQ),
            Self::OsString(s) => s.into_string().map_err(|_| Error::EILSEQ),
        }
    }

    /// Create a `CString` containing valid UTF-8, or fail with `Error::EILSEQ`.
    fn into_utf8_cstring(self) -> Result<CString> {
        self.into_string()
            .and_then(|s| CString::new(s).map_err(|_| Error::EILSEQ))
    }
}

/// A builder allowing customizable construction of `WasiCtx` instances.
pub struct WasiCtxBuilder {
    fds: HashMap<wasi::__wasi_fd_t, PendingFdEntry>,
    preopens: Vec<(PathBuf, File)>,
    args: Vec<PendingCString>,
    env: HashMap<PendingCString, PendingCString>,
}

impl WasiCtxBuilder {
    /// Builder for a new `WasiCtx`.
    pub fn new() -> Self {
        let mut builder = Self {
            fds: HashMap::new(),
            preopens: Vec::new(),
            args: vec![],
            env: HashMap::new(),
        };

        builder.fds.insert(0, PendingFdEntry::Thunk(FdEntry::null));
        builder.fds.insert(1, PendingFdEntry::Thunk(FdEntry::null));
        builder.fds.insert(2, PendingFdEntry::Thunk(FdEntry::null));

        builder
    }

    /// Add arguments to the command-line arguments list.
    ///
    /// Arguments must be valid UTF-8 with no NUL bytes, or else `WasiCtxBuilder::build()` will fail
    /// with `Error::EILSEQ`.
    pub fn args<S: AsRef<[u8]>>(mut self, args: impl IntoIterator<Item = S>) -> Self {
        self.args = args
            .into_iter()
            .map(|arg| arg.as_ref().to_vec().into())
            .collect();
        self
    }

    /// Add an argument to the command-line arguments list.
    ///
    /// Arguments must be valid UTF-8 with no NUL bytes, or else `WasiCtxBuilder::build()` will fail
    /// with `Error::EILSEQ`.
    pub fn arg<S: AsRef<[u8]>>(mut self, arg: S) -> Self {
        self.args.push(arg.as_ref().to_vec().into());
        self
    }

    /// Inherit the command-line arguments from the host process.
    ///
    /// If any arguments from the host process contain invalid UTF-8, `WasiCtxBuilder::build()` will
    /// fail with `Error::EILSEQ`.
    pub fn inherit_args(mut self) -> Self {
        self.args = env::args_os().map(PendingCString::OsString).collect();
        self
    }

    /// Inherit the stdin, stdout, and stderr streams from the host process.
    pub fn inherit_stdio(mut self) -> Self {
        self.fds
            .insert(0, PendingFdEntry::Thunk(FdEntry::duplicate_stdin));
        self.fds
            .insert(1, PendingFdEntry::Thunk(FdEntry::duplicate_stdout));
        self.fds
            .insert(2, PendingFdEntry::Thunk(FdEntry::duplicate_stderr));
        self
    }

    /// Inherit the environment variables from the host process.
    ///
    /// If any environment variables from the host process contain invalid Unicode (UTF-16 for
    /// Windows, UTF-8 for other platforms), `WasiCtxBuilder::build()` will fail with
    /// `Error::EILSEQ`.
    pub fn inherit_env(mut self) -> Self {
        self.env = std::env::vars_os()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
        self
    }

    /// Add an entry to the environment.
    ///
    /// Environment variable keys and values must be valid UTF-8 with no NUL bytes, or else
    /// `WasiCtxBuilder::build()` will fail with `Error::EILSEQ`.
    pub fn env<S: AsRef<[u8]>>(mut self, k: S, v: S) -> Self {
        self.env
            .insert(k.as_ref().to_vec().into(), v.as_ref().to_vec().into());
        self
    }

    /// Add entries to the environment.
    ///
    /// Environment variable keys and values must be valid UTF-8 with no NUL bytes, or else
    /// `WasiCtxBuilder::build()` will fail with `Error::EILSEQ`.
    pub fn envs<S: AsRef<[u8]>, T: Borrow<(S, S)>>(
        mut self,
        envs: impl IntoIterator<Item = T>,
    ) -> Self {
        self.env = envs
            .into_iter()
            .map(|t| {
                let (k, v) = t.borrow();
                (k.as_ref().to_vec().into(), v.as_ref().to_vec().into())
            })
            .collect();
        self
    }

    /// Provide a File to use as stdin
    pub fn stdin(mut self, file: File) -> Self {
        self.fds.insert(0, PendingFdEntry::File(file));
        self
    }

    /// Provide a File to use as stdout
    pub fn stdout(mut self, file: File) -> Self {
        self.fds.insert(1, PendingFdEntry::File(file));
        self
    }

    /// Provide a File to use as stderr
    pub fn stderr(mut self, file: File) -> Self {
        self.fds.insert(2, PendingFdEntry::File(file));
        self
    }

    /// Add a preopened directory.
    pub fn preopened_dir<P: AsRef<Path>>(mut self, dir: File, guest_path: P) -> Self {
        self.preopens.push((guest_path.as_ref().to_owned(), dir));
        self
    }

    /// Build a `WasiCtx`, consuming this `WasiCtxBuilder`.
    ///
    /// If any of the arguments or environment variables in this builder cannot be converted into
    /// `CString`s, either due to NUL bytes or Unicode conversions, this returns `Error::EILSEQ`.
    pub fn build(self) -> Result<WasiCtx> {
        // Process arguments and environment variables into `CString`s, failing quickly if they
        // contain any NUL bytes, or if conversion from `OsString` fails.
        let args = self
            .args
            .into_iter()
            .map(|arg| arg.into_utf8_cstring())
            .collect::<Result<Vec<CString>>>()?;

        let env = self
            .env
            .into_iter()
            .map(|(k, v)| {
                k.into_string().and_then(|mut pair| {
                    v.into_string().and_then(|v| {
                        pair.push('=');
                        pair.push_str(v.as_str());
                        // We have valid UTF-8, but the keys and values have not yet been checked
                        // for NULs, so we do a final check here.
                        CString::new(pair).map_err(|_| Error::EILSEQ)
                    })
                })
            })
            .collect::<Result<Vec<CString>>>()?;

        let mut fds: HashMap<wasi::__wasi_fd_t, FdEntry> = HashMap::new();
        // Populate the non-preopen fds.
        for (fd, pending) in self.fds {
            log::debug!("WasiCtx inserting ({:?}, {:?})", fd, pending);
            match pending {
                PendingFdEntry::Thunk(f) => {
                    fds.insert(fd, f()?);
                }
                PendingFdEntry::File(f) => {
                    fds.insert(fd, FdEntry::from(f)?);
                }
            }
        }
        // Then add the preopen fds. Startup code in the guest starts looking at fd 3 for preopens,
        // so we start from there. This variable is initially 2, though, because the loop
        // immediately does the increment and check for overflow.
        let mut preopen_fd: wasi::__wasi_fd_t = 2;
        for (guest_path, dir) in self.preopens {
            // We do the increment at the beginning of the loop body, so that we don't overflow
            // unnecessarily if we have exactly the maximum number of file descriptors.
            preopen_fd = preopen_fd.checked_add(1).ok_or(Error::ENFILE)?;

            if !dir.metadata()?.is_dir() {
                return Err(Error::EBADF);
            }

            // We don't currently allow setting file descriptors other than 0-2, but this will avoid
            // collisions if we restore that functionality in the future.
            while fds.contains_key(&preopen_fd) {
                preopen_fd = preopen_fd.checked_add(1).ok_or(Error::ENFILE)?;
            }
            let mut fe = FdEntry::from(dir)?;
            fe.preopen_path = Some(guest_path);
            log::debug!("WasiCtx inserting ({:?}, {:?})", preopen_fd, fe);
            fds.insert(preopen_fd, fe);
            log::debug!("WasiCtx fds = {:?}", fds);
        }

        Ok(WasiCtx { args, env, fds })
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
    pub fn new<S: AsRef<[u8]>>(args: impl IntoIterator<Item = S>) -> Result<Self> {
        WasiCtxBuilder::new()
            .args(args)
            .inherit_stdio()
            .inherit_env()
            .build()
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
        // Never insert where stdio handles are expected to be.
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
