use crate::p2;
use crate::{NamedId, WasiCtxNamedView};
use std::marker;
use std::pin::Pin;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite, empty};
use wasmtime::component::{HasData, ResourceTable};
use wasmtime_wasi_io::streams::{InputStream, OutputStream, StreamError};

mod empty;
mod file;
mod locked_async;
mod mem;
mod stdout;
mod worker_thread_stdin;

pub use self::file::{InputFile, OutputFile};
pub use self::locked_async::{AsyncStdinStream, AsyncStdoutStream};

/// Convert a host `io::Error` into a `StreamError`, matching the error-code
/// recovery that wasip1 performs via `filesystem::ErrorCode::from`.
///
/// * `BrokenPipe` is mapped to `StreamError::Closed` so that downstream
///   consumers (e.g. wasi-libc) can recover `EPIPE` rather than falling back
///   to a generic `EIO`.
///
/// * All other errors (including `IsADirectory`, permission errors, etc.) are
///   preserved as `LastOperationFailed` with the original `std::io::Error`
///   intact. This allows guests to recover the specific error code via the
///   `wasi:filesystem/types#filesystem-error-code` function, which downcasts
///   the error back to `std::io::Error` and maps it through
///   `ErrorCode::from`.
fn stream_error_from(e: std::io::Error) -> StreamError {
    if e.kind() == std::io::ErrorKind::BrokenPipe {
        StreamError::Closed
    } else {
        StreamError::LastOperationFailed(e.into())
    }
}

// Convenience reexport for stdio types so tokio doesn't have to be imported
// itself.
#[doc(no_inline)]
pub use tokio::io::{Stderr, Stdin, Stdout, stderr, stdin, stdout};

/// A helper struct which implements [`HasData`] for the `wasi:cli` APIs.
///
/// This can be useful when directly calling `add_to_linker` functions directly,
/// such as [`wasmtime_wasi::p2::bindings::cli::environment::add_to_linker`] as
/// the `D` type parameter. See [`HasData`] for more information about the type
/// parameter's purpose.
///
/// When using this type you can skip the [`WasiCliView`] trait, for
/// example.
///
/// [`wasmtime_wasi::p2::bindings::cli::environment::add_to_linker`]: crate::p2::bindings::cli::environment::add_to_linker
///
/// # Examples
///
/// ```
/// use wasmtime::component::{Linker, ResourceTable};
/// use wasmtime::{Engine, Result};
/// use wasmtime_wasi::cli::*;
///
/// struct MyStoreState {
///     table: ResourceTable,
///     cli: WasiCliCtx,
/// }
///
/// fn main() -> Result<()> {
///     let engine = Engine::default();
///     let mut linker = Linker::new(&engine);
///
///     wasmtime_wasi::p2::bindings::cli::environment::add_to_linker::<MyStoreState, WasiCli>(
///         &mut linker,
///         |state| WasiCliCtxView {
///             table: &mut state.table,
///             ctx: &mut state.cli,
///         },
///     )?;
///     Ok(())
/// }
/// ```
pub struct WasiCli;

impl HasData for WasiCli {
    type Data<'a> = WasiCliCtxView<'a>;
}

/// Provides a "view" of `wasi:cli`-related context used to implement host
/// traits.
pub trait WasiCliView: Send {
    fn cli(&mut self) -> WasiCliCtxView<'_>;
}

pub struct WasiCliCtxView<'a> {
    pub ctx: &'a mut WasiCliCtx,
    pub table: &'a mut ResourceTable,
}

pub struct WasiCliCtx {
    pub(crate) environment: Vec<(String, String)>,
    pub(crate) arguments: Vec<String>,
    pub(crate) initial_cwd: Option<String>,
    pub(crate) stdin: Box<dyn StdinStream>,
    pub(crate) stdout: Box<dyn StdoutStream>,
    pub(crate) stderr: Box<dyn StdoutStream>,
}

impl Default for WasiCliCtx {
    fn default() -> WasiCliCtx {
        WasiCliCtx {
            environment: Vec::new(),
            arguments: Vec::new(),
            initial_cwd: None,
            stdin: Box::new(empty()),
            stdout: Box::new(empty()),
            stderr: Box::new(empty()),
        }
    }
}

pub trait IsTerminal {
    /// Returns whether this stream is backed by a TTY.
    fn is_terminal(&self) -> bool;
}

/// A trait used to represent the standard input to a guest program.
///
/// Note that there are many built-in implementations of this trait for various
/// types such as [`tokio::io::Stdin`], [`tokio::io::Empty`], and
/// [`p2::pipe::MemoryInputPipe`].
pub trait StdinStream: IsTerminal + Send {
    /// Creates a fresh stream which is reading stdin.
    ///
    /// Note that the returned stream must share state with all other streams
    /// previously created. Guests may create multiple handles to the same stdin
    /// and they should all be synchronized in their progress through the
    /// program's input.
    ///
    /// Note that this means that if one handle becomes ready for reading they
    /// all become ready for reading. Subsequently if one is read from it may
    /// mean that all the others are no longer ready for reading. This is
    /// basically a consequence of the way the WIT APIs are designed today.
    fn async_stream(&self) -> Box<dyn AsyncRead + Send + Sync>;

    /// Same as [`Self::async_stream`] except that a WASIp2 [`InputStream`] is
    /// returned.
    ///
    /// Note that this has a default implementation which uses
    /// [`p2::pipe::AsyncReadStream`] as an adapter, but this can be overridden
    /// if there's a more specialized implementation available.
    fn p2_stream(&self) -> Box<dyn InputStream> {
        Box::new(p2::pipe::AsyncReadStream::new(Pin::from(
            self.async_stream(),
        )))
    }
}

/// Similar to [`StdinStream`], except for output.
///
/// This is used both for a guest stdin and a guest stdout.
///
/// Note that there are many built-in implementations of this trait for various
/// types such as [`tokio::io::Stdout`], [`tokio::io::Empty`], and
/// [`p2::pipe::MemoryOutputPipe`].
pub trait StdoutStream: IsTerminal + Send {
    /// Returns a fresh new stream which can write to this output stream.
    ///
    /// Note that all output streams should output to the same logical source.
    /// This means that it's possible for each independent stream to acquire a
    /// separate "permit" to write and then act on that permit. Note that
    /// additionally at this time once a permit is "acquired" there's no way to
    /// release it, for example you can wait for readiness and then never
    /// actually write in WASI. This means that acquisition of a permit for one
    /// stream cannot discount the size of a permit another stream could
    /// obtain.
    ///
    /// Implementations must be able to handle this
    fn async_stream(&self) -> Box<dyn AsyncWrite + Send + Sync>;

    /// Same as [`Self::async_stream`] except that a WASIp2 [`OutputStream`] is
    /// returned.
    ///
    /// Note that this has a default implementation which uses
    /// [`p2::pipe::AsyncWriteStream`] as an adapter, but this can be overridden
    /// if there's a more specialized implementation available.
    fn p2_stream(&self) -> Box<dyn OutputStream> {
        Box::new(p2::pipe::AsyncWriteStream::new(
            8192, // FIXME: extract this to a constant.
            Pin::from(self.async_stream()),
        ))
    }
}

// Forward `&T => T`
impl<T: ?Sized + IsTerminal> IsTerminal for &T {
    fn is_terminal(&self) -> bool {
        T::is_terminal(self)
    }
}
impl<T: ?Sized + StdinStream + Sync> StdinStream for &T {
    fn p2_stream(&self) -> Box<dyn InputStream> {
        T::p2_stream(self)
    }
    fn async_stream(&self) -> Box<dyn AsyncRead + Send + Sync> {
        T::async_stream(self)
    }
}
impl<T: ?Sized + StdoutStream + Sync> StdoutStream for &T {
    fn p2_stream(&self) -> Box<dyn OutputStream> {
        T::p2_stream(self)
    }
    fn async_stream(&self) -> Box<dyn AsyncWrite + Send + Sync> {
        T::async_stream(self)
    }
}

// Forward `&mut T => T`
impl<T: ?Sized + IsTerminal> IsTerminal for &mut T {
    fn is_terminal(&self) -> bool {
        T::is_terminal(self)
    }
}
impl<T: ?Sized + StdinStream + Sync> StdinStream for &mut T {
    fn p2_stream(&self) -> Box<dyn InputStream> {
        T::p2_stream(self)
    }
    fn async_stream(&self) -> Box<dyn AsyncRead + Send + Sync> {
        T::async_stream(self)
    }
}
impl<T: ?Sized + StdoutStream + Sync> StdoutStream for &mut T {
    fn p2_stream(&self) -> Box<dyn OutputStream> {
        T::p2_stream(self)
    }
    fn async_stream(&self) -> Box<dyn AsyncWrite + Send + Sync> {
        T::async_stream(self)
    }
}

// Forward `Box<T> => T`
impl<T: ?Sized + IsTerminal> IsTerminal for Box<T> {
    fn is_terminal(&self) -> bool {
        T::is_terminal(self)
    }
}
impl<T: ?Sized + StdinStream + Sync> StdinStream for Box<T> {
    fn p2_stream(&self) -> Box<dyn InputStream> {
        T::p2_stream(self)
    }
    fn async_stream(&self) -> Box<dyn AsyncRead + Send + Sync> {
        T::async_stream(self)
    }
}
impl<T: ?Sized + StdoutStream + Sync> StdoutStream for Box<T> {
    fn p2_stream(&self) -> Box<dyn OutputStream> {
        T::p2_stream(self)
    }
    fn async_stream(&self) -> Box<dyn AsyncWrite + Send + Sync> {
        T::async_stream(self)
    }
}

// Forward `Arc<T> => T`
impl<T: ?Sized + IsTerminal> IsTerminal for Arc<T> {
    fn is_terminal(&self) -> bool {
        T::is_terminal(self)
    }
}
impl<T: ?Sized + StdinStream + Sync> StdinStream for Arc<T> {
    fn p2_stream(&self) -> Box<dyn InputStream> {
        T::p2_stream(self)
    }
    fn async_stream(&self) -> Box<dyn AsyncRead + Send + Sync> {
        T::async_stream(self)
    }
}
impl<T: ?Sized + StdoutStream + Sync> StdoutStream for Arc<T> {
    fn p2_stream(&self) -> Box<dyn OutputStream> {
        T::p2_stream(self)
    }
    fn async_stream(&self) -> Box<dyn AsyncWrite + Send + Sync> {
        T::async_stream(self)
    }
}

/// A helper struct which implements [`HasData`] for the `wasi:cli` APIs when
/// used in combination with named imports.
///
/// This structure is similar in purpose to [`WasiCli`] and is used
/// when using the [`named_imports`] module for `wasi:cli`. This structure
/// serves as the `D` type parameter for `add_to_linker` functions.
///
/// [`named_imports`]: crate::p3::bindings::named_imports::wasi::cli
///
/// # Meaning of the `T` parameter
///
/// Here the `T` must be something that implements [`WasiCliNamedView`]. The
/// corresponding `Data` for this type is [`WasiCtxNamedView`] which internally
/// will contain `&mut T`.
///
/// Effectively you're going to implement [`WasiCliNamedView`] for something in
/// your embedding, and that's the `T` you'll fill in here.
///
/// # Examples
///
/// ```
/// use wasmtime::component::{Linker, Component, ResourceTable};
/// use wasmtime::{Engine, Result};
/// use wasmtime_wasi::{NamedId, WasiCtxNamedView};
/// use wasmtime_wasi::cli::*;
/// use wasmtime_wasi::p2::bindings::named_imports;
/// use std::collections::HashMap;
///
/// struct MyStoreState {
///     table: ResourceTable,
///     states: HashMap<NamedId, WasiCliCtx>,
/// }
///
/// fn main() -> Result<()> {
///     let engine = Engine::default();
///     let mut linker = Linker::new(&engine);
///     let component = Component::new(&engine, "(component)")?;
///     let mut name_map = HashMap::new();
///
///     named_imports::wasi::cli::environment::add_to_linker::<MyStoreState, WasiCliNamed<MyStoreState>>(
///         &mut linker,
///         &component,
///         |name| {
///             let len = name_map.len();
///             Ok(NamedId(*name_map.entry(name.to_string()).or_insert(len)))
///         },
///         |state| WasiCtxNamedView(state),
///     )?;
///     Ok(())
/// }
///
/// impl WasiCliNamedView for MyStoreState {
///     fn cli(&mut self, id: NamedId) -> WasiCliCtxView<'_> {
///         let ctx = self.states.get_mut(&id).expect("state for id");
///         WasiCliCtxView {
///             table: &mut self.table,
///             ctx,
///         }
///     }
/// }
/// ```
pub struct WasiCliNamed<T>(marker::PhantomData<fn() -> T>);

impl<T> HasData for WasiCliNamed<T>
where
    T: WasiCliNamedView,
{
    type Data<'a> = WasiCtxNamedView<'a, T>;
}

/// A trait used to look up a specific `wasi:cli` context for a named import.
///
/// This trait is used in conjunction with the [`named_imports`] bindings
/// generated for all WASI interfaces. The purpose of this trait is for
/// embedders to define how a [`NamedId`] maps to a particular `wasi:cli`
/// context, here returned as [`WasiCliCtxView`]. Embedders are responsible
/// for assigning meaning to [`NamedId`] values themselves. These IDs are
/// assigned when [`add_named_to_linker`] is called, for example, as the
/// `lookup` argument to that function.
///
/// When using [`add_named_to_linker`] it's sufficient to implement this trait
/// for the `T` in `Store<T>`. You can also instead implement the
/// [`WasiNamedView`] trait for `T` which implies an implementation of this
/// trait.
///
/// When using `add_to_linker` in the generated `bindings::named_imports`
/// module then values implementing this live within the `T` of `Store<T>`, and
/// be temporarily referenced in [`WasiCtxNamedView`] where internally that'll
/// hold `WasiCtxNamedView(&mut your_type)`.
///
/// [`named_imports`]: crate::p3::bindings::named_imports
/// [`add_named_to_linker`]: crate::p3::cli::add_named_to_linker
/// [`WasiNamedView`]: crate::WasiNamedView
///
/// # Examples
///
/// ```
/// use wasmtime::component::{Linker, Component, ResourceTable};
/// use wasmtime::{Engine, Result};
/// use wasmtime_wasi::{NamedId, WasiCtxNamedView};
/// use wasmtime_wasi::cli::*;
/// use wasmtime_wasi::p2::bindings::named_imports;
/// use std::collections::HashMap;
///
/// struct MyStoreState {
///     table: ResourceTable,
///     states: HashMap<NamedId, WasiCliCtx>,
/// }
///
/// fn main() -> Result<()> {
///     let engine = Engine::default();
///     let mut linker = Linker::new(&engine);
///     let component = Component::new(&engine, "(component)")?;
///     let mut name_map = HashMap::new();
///
///     wasmtime_wasi::p3::cli::add_named_to_linker::<MyStoreState>(
///         &mut linker,
///         &component,
///         |_, name| {
///             let len = name_map.len();
///             Ok(NamedId(*name_map.entry(name.to_string()).or_insert(len)))
///         },
///     )?;
///     Ok(())
/// }
///
/// impl WasiCliNamedView for MyStoreState {
///     fn cli(&mut self, id: NamedId) -> WasiCliCtxView<'_> {
///         let ctx = self.states.get_mut(&id).expect("state for id");
///         WasiCliCtxView {
///             table: &mut self.table,
///             ctx,
///         }
///     }
/// }
/// ```
pub trait WasiCliNamedView: Send + 'static {
    /// Looks up the [`WasiCliCtxView`] for the given [`NamedId`].
    ///
    /// This method will resolve the `id` specified to a specific CLI context
    /// that is available to be used. Note that this method is specifically
    /// infallible meaning that a CLI context must be returned and this cannot
    /// generate a trap or panic or similar.
    ///
    /// Embedders are responsible for allocating [`NamedId`] and assigning
    /// meaning to ids. When a `Linker` is populated embedders will have the
    /// ability to generate a `NamedId` for all imports found, and then that
    /// embedder-allocated id is then passed back here when the corresponding
    /// imported function is invoked.
    ///
    /// Note that the [`ResourceTable`] referenced in the returned
    /// [`WasiCliCtxView`] need not be unique. It's ok to use the same
    /// [`ResourceTable`] for all imports. This is not a guest-visible
    /// abstraction and just helps the host allocate and manage state.
    fn cli(&mut self, id: NamedId) -> WasiCliCtxView<'_>;
}

#[cfg(test)]
mod test {
    use crate::cli::{AsyncStdoutStream, StdinStream, StdoutStream};
    use crate::p2::{self, OutputStream};
    use bytes::Bytes;
    use tokio::io::AsyncReadExt;
    use wasmtime::Result;

    #[test]
    fn memory_stdin_stream() {
        // A StdinStream has the property that there are multiple
        // InputStreams created, using the stream() method which are each
        // views on the same shared state underneath. Consuming input on one
        // stream results in consuming that input on all streams.
        //
        // The simplest way to measure this is to check if the MemoryInputPipe
        // impl of StdinStream follows this property.

        let pipe =
            p2::pipe::MemoryInputPipe::new("the quick brown fox jumped over the three lazy dogs");

        let mut view1 = pipe.p2_stream();
        let mut view2 = pipe.p2_stream();

        let read1 = view1.read(10).expect("read first 10 bytes");
        assert_eq!(read1, "the quick ".as_bytes(), "first 10 bytes");
        let read2 = view2.read(10).expect("read second 10 bytes");
        assert_eq!(read2, "brown fox ".as_bytes(), "second 10 bytes");
        let read3 = view1.read(10).expect("read third 10 bytes");
        assert_eq!(read3, "jumped ove".as_bytes(), "third 10 bytes");
        let read4 = view2.read(10).expect("read fourth 10 bytes");
        assert_eq!(read4, "r the thre".as_bytes(), "fourth 10 bytes");
    }

    #[tokio::test]
    async fn async_stdin_stream() {
        // A StdinStream has the property that there are multiple
        // InputStreams created, using the stream() method which are each
        // views on the same shared state underneath. Consuming input on one
        // stream results in consuming that input on all streams.
        //
        // AsyncStdinStream is a slightly more complex impl of StdinStream
        // than the MemoryInputPipe above. We can create an AsyncReadStream
        // from a file on the disk, and an AsyncStdinStream from that common
        // stream, then check that the same property holds as above.

        let dir = tempfile::tempdir().unwrap();
        let mut path = std::path::PathBuf::from(dir.path());
        path.push("file");
        std::fs::write(&path, "the quick brown fox jumped over the three lazy dogs").unwrap();

        let file = tokio::fs::File::open(&path)
            .await
            .expect("open created file");
        let stdin_stream = super::AsyncStdinStream::new(file);

        use super::StdinStream;

        let mut view1 = stdin_stream.p2_stream();
        let mut view2 = stdin_stream.p2_stream();

        view1.ready().await;

        let read1 = view1.read(10).expect("read first 10 bytes");
        assert_eq!(read1, "the quick ".as_bytes(), "first 10 bytes");
        let read2 = view2.read(10).expect("read second 10 bytes");
        assert_eq!(read2, "brown fox ".as_bytes(), "second 10 bytes");
        let read3 = view1.read(10).expect("read third 10 bytes");
        assert_eq!(read3, "jumped ove".as_bytes(), "third 10 bytes");
        let read4 = view2.read(10).expect("read fourth 10 bytes");
        assert_eq!(read4, "r the thre".as_bytes(), "fourth 10 bytes");
    }

    #[tokio::test]
    async fn async_stdout_stream_unblocks() {
        let (mut read, write) = tokio::io::duplex(32);
        let stdout = AsyncStdoutStream::new(32, write);

        let task = tokio::task::spawn(async move {
            let mut stream = stdout.p2_stream();
            blocking_write_and_flush(&mut *stream, "x".into())
                .await
                .unwrap();
        });

        let mut buf = [0; 100];
        let n = read.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"x");

        task.await.unwrap();
    }

    async fn blocking_write_and_flush(s: &mut dyn OutputStream, mut bytes: Bytes) -> Result<()> {
        while !bytes.is_empty() {
            let permit = s.write_ready().await?;
            let len = bytes.len().min(permit);
            let chunk = bytes.split_to(len);
            s.write(chunk)?;
        }

        s.flush()?;
        s.write_ready().await?;
        Ok(())
    }

    // Verify that the stdio OutputStream implementation reports a usable
    // write permit and can successfully write + flush (exercises the full
    // trait impl including the error conversion path).
    #[test]
    fn stdio_output_stream_write_flush() {
        let mut stream: Box<dyn wasmtime_wasi_io::streams::OutputStream> =
            StdoutStream::p2_stream(&std::io::stderr());

        let permit = stream.check_write().expect("check_write");
        assert!(permit > 0, "permit should be nonzero");

        // Writing empty bytes must succeed.
        stream
            .write(Bytes::new())
            .expect("writing empty bytes should succeed");

        // Flushing must succeed.
        stream.flush().expect("flush should succeed");
    }

    #[test]
    fn stream_error_from_broken_pipe_maps_to_closed() {
        use std::io;
        use wasmtime_wasi_io::streams::StreamError;

        let err = super::stream_error_from(io::Error::from(io::ErrorKind::BrokenPipe));
        assert!(matches!(err, StreamError::Closed));
    }

    #[test]
    fn stream_error_from_preserves_io_error() {
        use std::io;
        use wasmtime_wasi_io::streams::StreamError;

        let err = super::stream_error_from(io::Error::from(io::ErrorKind::IsADirectory));
        match err {
            StreamError::LastOperationFailed(e) => {
                let io_err = e.downcast::<io::Error>().expect("should downcast");
                assert_eq!(io_err.kind(), io::ErrorKind::IsADirectory);
            }
            other => panic!("expected LastOperationFailed, got: {other:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn stream_error_from_raw_os_eisdir() {
        use rustix::io::Errno;
        use std::io;
        use wasmtime_wasi_io::streams::StreamError;

        let err =
            super::stream_error_from(io::Error::from_raw_os_error(Errno::ISDIR.raw_os_error()));
        match err {
            StreamError::LastOperationFailed(e) => {
                let io_err = e.downcast::<io::Error>().expect("should downcast");
                assert_eq!(io_err.raw_os_error(), Some(Errno::ISDIR.raw_os_error()));
            }
            other => panic!("expected LastOperationFailed, got: {other:?}"),
        }
    }
}
