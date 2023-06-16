//! Virtual pipes.
//!
//! These types provide easy implementations of `WasiFile` that mimic much of the behavior of Unix
//! pipes. These are particularly helpful for redirecting WASI stdio handles to destinations other
//! than OS files.
//!
//! Some convenience constructors are included for common backing types like `Vec<u8>` and `String`,
//! but the virtual pipes can be instantiated with any `Read` or `Write` type.
//!
use crate::preview2::{HostInputStream, HostOutputStream, HostPollable};
use anyhow::{anyhow, Error};
use std::any::Any;
use std::convert::TryInto;
use std::io::{self, Read, Write};
use std::sync::{Arc, RwLock};
use system_interface::io::ReadReady;

/// A virtual pipe read end.
///
/// This reads from a source that implements the [`Read`] trait. It
/// also requires the [`ReadReady`] trait, which is implemented for many
/// popular `Read`-implementing types and is easy to implemented for new
/// types.
///
/// A variety of `From` impls are provided so that common pipe types are
/// easy to create. For example:
///
/// ```
/// use wasmtime_wasi::preview2::{pipe::ReadPipe, WasiCtx};
/// let stdin = ReadPipe::from("hello from stdin!");
/// let builder = WasiCtx::builder().set_stdin(stdin);
/// ```
#[derive(Debug)]
pub struct ReadPipe<R: Read + ReadReady> {
    notify: Arc<tokio::sync::Notify>,
    reader: Arc<RwLock<R>>,
}

impl<R: Read + ReadReady> Clone for ReadPipe<R> {
    fn clone(&self) -> Self {
        Self {
            notify: self.notify.clone(),
            reader: self.reader.clone(),
        }
    }
}

impl<R: Read + ReadReady> ReadPipe<R> {
    /// Create a new pipe from a `Read` type.
    ///
    /// All `Handle` read operations delegate to reading from this underlying reader.
    pub fn new(r: R) -> Self {
        Self::from_shared(Arc::new(RwLock::new(r)))
    }

    /// Create a new pipe from a shareable `Read` type.
    ///
    /// All `Handle` read operations delegate to reading from this underlying reader.
    pub fn from_shared(reader: Arc<RwLock<R>>) -> Self {
        Self {
            // TODO(elliottt): should the shared notify be an argument as well?
            notify: Arc::new(tokio::sync::Notify::new()),
            reader,
        }
    }

    /// Try to convert this `ReadPipe<R>` back to the underlying `R` type.
    ///
    /// This will fail with `Err(self)` if multiple references to the underlying `R` exist.
    pub fn try_into_inner(mut self) -> Result<R, Self> {
        match Arc::try_unwrap(self.reader) {
            Ok(rc) => Ok(RwLock::into_inner(rc).unwrap()),
            Err(reader) => {
                self.reader = reader;
                Err(self)
            }
        }
    }
    fn borrow(&self) -> std::sync::RwLockWriteGuard<R> {
        RwLock::write(&self.reader).unwrap()
    }
}

impl From<Vec<u8>> for ReadPipe<io::Cursor<Vec<u8>>> {
    fn from(r: Vec<u8>) -> Self {
        Self::new(io::Cursor::new(r))
    }
}

impl From<&[u8]> for ReadPipe<io::Cursor<Vec<u8>>> {
    fn from(r: &[u8]) -> Self {
        Self::from(r.to_vec())
    }
}

impl From<String> for ReadPipe<io::Cursor<String>> {
    fn from(r: String) -> Self {
        Self::new(io::Cursor::new(r))
    }
}

impl From<&str> for ReadPipe<io::Cursor<String>> {
    fn from(r: &str) -> Self {
        Self::from(r.to_string())
    }
}

#[async_trait::async_trait]
impl<R: Read + ReadReady + Any + Send + Sync> HostInputStream for ReadPipe<R> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<(u64, bool), Error> {
        match self.borrow().read(buf) {
            Ok(0) => Ok((0, true)),
            Ok(n) => Ok((n.try_into()?, false)),
            Err(e) if e.kind() == io::ErrorKind::Interrupted => Ok((0, false)),
            Err(e) => Err(e.into()),
        }
    }

    async fn skip(&mut self, nelem: u64) -> Result<(u64, bool), Error> {
        let num = io::copy(
            &mut io::Read::take(&mut *self.borrow(), nelem),
            &mut io::sink(),
        )?;
        Ok((num, num < nelem))
    }

    fn pollable(&self) -> HostPollable {
        // This is a standalone function because RwLockReadGuard does not implement Send -- calling
        // `reader.read()` from within the async closure below is just not possible.
        fn ready<T: Read + ReadReady + Any + Send + Sync>(reader: &RwLock<T>) -> bool {
            if let Ok(g) = reader.read() {
                if let Ok(n) = g.num_ready_bytes() {
                    return n > 0;
                }
            }

            // If either read or num_ready_bytes raised an error, we want to consider the pipe
            // ready for reading.
            true
        }

        let notify = Arc::clone(&self.notify);
        let reader = Arc::clone(&self.reader);
        HostPollable::new(move || {
            // TODO(elliottt): is it possible to avoid these clones? They're needed because `Arc`
            // isn't copy, and we need to move values into the async closure.
            let notify = Arc::clone(&notify);
            let reader = Arc::clone(&reader);
            Box::pin(async move {
                {
                    let reader = reader.clone();
                    let sender = notify.clone();
                    tokio::spawn(async move {
                        while !ready(&reader) {
                            tokio::task::yield_now().await;
                        }

                        sender.notify_one();
                    });
                }

                notify.notified().await;

                let g = match reader.read() {
                    Ok(g) => g,
                    Err(_) => return Err(anyhow!("pipe has been poisoned")),
                };

                match g.num_ready_bytes() {
                    Ok(_) => Ok(()),
                    Err(e) => Err(anyhow!(e)),
                }
            })
        })
    }
}

/// A virtual pipe write end.
///
/// ```no_run
/// use wasmtime_wasi::preview2::{pipe::WritePipe, WasiCtx, Table};
/// let mut table = Table::new();
/// let stdout = WritePipe::new_in_memory();
/// let mut ctx = WasiCtx::builder().set_stdout(stdout.clone()).build(&mut table).unwrap();
/// // use ctx and table in an instance, then make sure it is dropped:
/// drop(ctx);
/// drop(table);
/// let contents: Vec<u8> = stdout.try_into_inner().expect("sole remaining reference to WritePipe").into_inner();
/// println!("contents of stdout: {:?}", contents);
/// ```
#[derive(Debug)]
pub struct WritePipe<W: Write> {
    writer: Arc<RwLock<W>>,
}

impl<W: Write> Clone for WritePipe<W> {
    fn clone(&self) -> Self {
        Self {
            writer: self.writer.clone(),
        }
    }
}

impl<W: Write> WritePipe<W> {
    /// Create a new pipe from a `Write` type.
    ///
    /// All `Handle` write operations delegate to writing to this underlying writer.
    pub fn new(w: W) -> Self {
        Self::from_shared(Arc::new(RwLock::new(w)))
    }

    /// Create a new pipe from a shareable `Write` type.
    ///
    /// All `Handle` write operations delegate to writing to this underlying writer.
    pub fn from_shared(writer: Arc<RwLock<W>>) -> Self {
        Self { writer }
    }

    /// Try to convert this `WritePipe<W>` back to the underlying `W` type.
    ///
    /// This will fail with `Err(self)` if multiple references to the underlying `W` exist.
    pub fn try_into_inner(mut self) -> Result<W, Self> {
        match Arc::try_unwrap(self.writer) {
            Ok(rc) => Ok(RwLock::into_inner(rc).unwrap()),
            Err(writer) => {
                self.writer = writer;
                Err(self)
            }
        }
    }

    fn borrow(&self) -> std::sync::RwLockWriteGuard<W> {
        RwLock::write(&self.writer).unwrap()
    }
}

impl WritePipe<io::Cursor<Vec<u8>>> {
    /// Create a new writable virtual pipe backed by a `Vec<u8>` buffer.
    pub fn new_in_memory() -> Self {
        Self::new(io::Cursor::new(vec![]))
    }
}

#[async_trait::async_trait]
// TODO: can we remove the `Any` constraint here?
impl<W: Write + Any + Send + Sync> HostOutputStream for WritePipe<W> {
    async fn write(&mut self, buf: &[u8]) -> Result<u64, Error> {
        let n = self.borrow().write(buf)?;
        Ok(n.try_into()?)
    }
    async fn write_zeroes(&mut self, nelem: u64) -> Result<u64, Error> {
        let num = io::copy(
            &mut io::Read::take(io::repeat(0), nelem),
            &mut *self.borrow(),
        )?;
        Ok(num)
    }

    fn pollable(&self) -> HostPollable {
        // NOTE: as we only really know that W is Write, there's no way to determine what space is
        // available for writing. Thus we indicate that there's space available by returning
        // immediately.
        HostPollable::new(|| Box::pin(async { Ok(()) }))
    }
}
