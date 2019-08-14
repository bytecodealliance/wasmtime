use crate::fdentry::FdEntry;
use crate::sys::{dev_null, errno_from_ioerror};
use crate::{host, Result};
use std::borrow::Borrow;
use std::collections::HashMap;
use std::ffi::CString;
use std::fs::File;
use std::path::{Path, PathBuf};

pub struct WasiCtxBuilder {
    fds: HashMap<host::__wasi_fd_t, FdEntry>,
    preopens: HashMap<PathBuf, File>,
    args: Vec<CString>,
    env: HashMap<CString, CString>,
}

impl WasiCtxBuilder {
    /// Builder for a new `WasiCtx`.
    pub fn new() -> Result<Self> {
        let mut builder = Self {
            fds: HashMap::new(),
            preopens: HashMap::new(),
            args: vec![],
            env: HashMap::new(),
        };

        builder.fds.insert(0, FdEntry::from(dev_null()?)?);
        builder.fds.insert(1, FdEntry::from(dev_null()?)?);
        builder.fds.insert(2, FdEntry::from(dev_null()?)?);

        Ok(builder)
    }

    pub fn args<S: AsRef<str>>(mut self, args: impl Iterator<Item = S>) -> Result<Self> {
        let args: Result<Vec<CString>> = args
            .map(|arg| CString::new(arg.as_ref()).map_err(|_| host::__WASI_ENOTCAPABLE))
            .collect();
        self.args = args?;
        Ok(self)
    }

    pub fn arg(mut self, arg: &str) -> Result<Self> {
        self.args
            .push(CString::new(arg).map_err(|_| host::__WASI_ENOTCAPABLE)?);
        Ok(self)
    }

    pub fn inherit_stdio(mut self) -> Result<Self> {
        self.fds.insert(0, FdEntry::duplicate_stdin()?);
        self.fds.insert(1, FdEntry::duplicate_stdout()?);
        self.fds.insert(2, FdEntry::duplicate_stderr()?);
        Ok(self)
    }

    pub fn inherit_env(self) -> Result<Self> {
        self.envs(std::env::vars())
    }

    pub fn env<S: AsRef<str>>(mut self, k: S, v: S) -> Result<Self> {
        self.env.insert(
            CString::new(k.as_ref()).map_err(|_| host::__WASI_ENOTCAPABLE)?,
            CString::new(v.as_ref()).map_err(|_| host::__WASI_ENOTCAPABLE)?,
        );
        Ok(self)
    }

    pub fn envs<S: AsRef<str>, T: Borrow<(S, S)>>(
        mut self,
        envs: impl Iterator<Item = T>,
    ) -> Result<Self> {
        let env: Result<HashMap<CString, CString>> = envs
            .map(|t| {
                let (k, v) = t.borrow();
                let k = CString::new(k.as_ref()).map_err(|_| host::__WASI_ENOTCAPABLE);
                let v = CString::new(v.as_ref()).map_err(|_| host::__WASI_ENOTCAPABLE);
                match (k, v) {
                    (Ok(k), Ok(v)) => Ok((k, v)),
                    _ => Err(host::__WASI_ENOTCAPABLE),
                }
            })
            .collect();
        self.env = env?;
        Ok(self)
    }

    pub fn preopened_dir<P: AsRef<Path>>(mut self, dir: File, guest_path: P) -> Self {
        self.preopens.insert(guest_path.as_ref().to_owned(), dir);
        self
    }

    pub fn build(mut self) -> Result<WasiCtx> {
        // startup code starts looking at fd 3 for preopens
        let mut preopen_fd = 3;
        for (guest_path, dir) in self.preopens {
            if !dir.metadata().map_err(errno_from_ioerror)?.is_dir() {
                return Err(host::__WASI_EBADF);
            }

            while self.fds.contains_key(&preopen_fd) {
                preopen_fd = preopen_fd.checked_add(1).ok_or(host::__WASI_ENFILE)?;
            }
            let mut fe = FdEntry::from(dir)?;
            fe.preopen_path = Some(guest_path);
            self.fds.insert(preopen_fd, fe);
            preopen_fd += 1;
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
    pub fds: HashMap<host::__wasi_fd_t, FdEntry>,
    pub args: Vec<CString>,
    pub env: Vec<CString>,
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

    pub fn contains_fd_entry(&self, fd: host::__wasi_fd_t) -> bool {
        self.fds.contains_key(&fd)
    }

    pub fn get_fd_entry(
        &self,
        fd: host::__wasi_fd_t,
        rights_base: host::__wasi_rights_t,
        rights_inheriting: host::__wasi_rights_t,
    ) -> Result<&FdEntry> {
        if let Some(fe) = self.fds.get(&fd) {
            Self::validate_rights(fe, rights_base, rights_inheriting).and(Ok(fe))
        } else {
            Err(host::__WASI_EBADF)
        }
    }

    pub fn get_fd_entry_mut(
        &mut self,
        fd: host::__wasi_fd_t,
        rights_base: host::__wasi_rights_t,
        rights_inheriting: host::__wasi_rights_t,
    ) -> Result<&mut FdEntry> {
        if let Some(fe) = self.fds.get_mut(&fd) {
            Self::validate_rights(fe, rights_base, rights_inheriting).and(Ok(fe))
        } else {
            Err(host::__WASI_EBADF)
        }
    }

    fn validate_rights(
        fe: &FdEntry,
        rights_base: host::__wasi_rights_t,
        rights_inheriting: host::__wasi_rights_t,
    ) -> Result<()> {
        if !fe.rights_base & rights_base != 0 || !fe.rights_inheriting & rights_inheriting != 0 {
            Err(host::__WASI_ENOTCAPABLE)
        } else {
            Ok(())
        }
    }

    pub fn insert_fd_entry(&mut self, fe: FdEntry) -> Result<host::__wasi_fd_t> {
        // never insert where stdio handles usually are
        let mut fd = 3;
        while self.fds.contains_key(&fd) {
            if let Some(next_fd) = fd.checked_add(1) {
                fd = next_fd;
            } else {
                return Err(host::__WASI_EMFILE);
            }
        }
        self.fds.insert(fd, fe);
        Ok(fd)
    }
}
