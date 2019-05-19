use crate::host;

use crate::sys::dev_null;
use crate::sys::fdmap::{FdEntry, FdMap};

use failure::{bail, format_err, Error};
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::fs::File;
use std::io::{stderr, stdin, stdout};
use std::path::{Path, PathBuf};

pub struct WasiCtxBuilder {
    fds: FdMap,
    preopens: HashMap<PathBuf, File>,
    args: Vec<CString>,
    env: HashMap<CString, CString>,
}

impl WasiCtxBuilder {
    /// Builder for a new `WasiCtx`.
    pub fn new() -> Self {
        let mut builder = Self {
            fds: FdMap::new(),
            preopens: HashMap::new(),
            args: vec![],
            env: HashMap::new(),
        };

        builder.fds.insert_fd_entry_at(0, FdEntry::from_file(dev_null()));
        builder.fds.insert_fd_entry_at(1, FdEntry::from_file(dev_null()));
        builder.fds.insert_fd_entry_at(2, FdEntry::from_file(dev_null()));

        builder
    }

    pub fn args(mut self, args: &[&str]) -> Self {
        self.args = args
            .into_iter()
            .map(|arg| CString::new(*arg).expect("argument can be converted to a CString"))
            .collect();
        self
    }

    pub fn arg(mut self, arg: &str) -> Self {
        self.args
            .push(CString::new(arg).expect("argument can be converted to a CString"));
        self
    }

    pub fn c_args<S: AsRef<CStr>>(mut self, args: &[S]) -> Self {
        self.args = args
            .into_iter()
            .map(|arg| arg.as_ref().to_owned())
            .collect();
        self
    }

    pub fn c_arg<S: AsRef<CStr>>(mut self, arg: S) -> Self {
        self.args.push(arg.as_ref().to_owned());
        self
    }

    pub fn inherit_stdio(mut self) -> Self {
        self.fds.insert_fd_entry_at(0, FdEntry::duplicate(&stdin()));
        self.fds.insert_fd_entry_at(1, FdEntry::duplicate(&stdout()));
        self.fds.insert_fd_entry_at(2, FdEntry::duplicate(&stderr()));
        self
    }

    pub fn inherit_env(mut self) -> Self {
        self.env = std::env::vars()
            .map(|(k, v)| {
                // TODO: handle errors, and possibly assert that the key is valid per POSIX
                (
                    CString::new(k).expect("environment key can be converted to a CString"),
                    CString::new(v).expect("environment value can be converted to a CString"),
                )
            })
            .collect();
        self
    }

    pub fn env(mut self, k: &str, v: &str) -> Self {
        self.env.insert(
            // TODO: handle errors, and possibly assert that the key is valid per POSIX
            CString::new(k).expect("environment key can be converted to a CString"),
            CString::new(v).expect("environment value can be converted to a CString"),
        );
        self
    }

    pub fn c_env<S, T>(mut self, k: S, v: T) -> Self
    where
        S: AsRef<CStr>,
        T: AsRef<CStr>,
    {
        self.env
            .insert(k.as_ref().to_owned(), v.as_ref().to_owned());
        self
    }

    pub fn preopened_dir<P: AsRef<Path>>(mut self, dir: File, guest_path: P) -> Self {
        self.preopens.insert(guest_path.as_ref().to_owned(), dir);
        self
    }

    pub fn build(mut self) -> Result<WasiCtx, Error> {
        for (guest_path, dir) in self.preopens {
            if !dir.metadata()?.is_dir() {
                bail!("preopened file is not a directory");
            }
            let mut fe = FdEntry::from_file(dir);
            fe.preopen_path = Some(guest_path);
            self.fds
                .insert_fd_entry(fe)
                .map_err(|_| format_err!("not enough file handles"))?;
        }

        let env = self
            .env
            .into_iter()
            .map(|(k, v)| {
                let mut pair = k.into_bytes();
                pair.extend_from_slice(b"=");
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
    pub fds: FdMap,
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
    pub fn new(args: &[&str]) -> Self {
        WasiCtxBuilder::new()
            .args(args)
            .inherit_stdio()
            .inherit_env()
            .build()
            .expect("default options don't fail")
    }

    pub fn get_fd_entry(
        &self,
        fd: host::__wasi_fd_t,
        rights_base: host::__wasi_rights_t,
        rights_inheriting: host::__wasi_rights_t,
    ) -> Result<&FdEntry, host::__wasi_errno_t> {
        self.fds.get_fd_entry(fd, rights_base, rights_inheriting)
    }

    pub fn insert_fd_entry(
        &mut self,
        fe: FdEntry,
    ) -> Result<host::__wasi_fd_t, host::__wasi_errno_t> {
        self.fds.insert_fd_entry(fe)
    }
}
