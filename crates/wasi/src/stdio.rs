use crate::bindings::cli::{
    stderr, stdin, stdout, terminal_input, terminal_output, terminal_stderr, terminal_stdin,
    terminal_stdout,
};
use crate::bindings::io::streams;
use crate::pipe;
use crate::{
    HostInputStream, HostOutputStream, StreamError, StreamResult, Subscribe, WasiImpl, WasiView,
};
use bytes::Bytes;
use std::future::Future;
use std::io::IsTerminal;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use wasmtime::component::Resource;

/// A trait used to represent the standard input to a guest program.
///
/// This is used to implement various WASI APIs via the method implementations
/// below.
///
/// Built-in implementations are provided for [`Stdin`],
/// [`pipe::MemoryInputPipe`], and [`pipe::ClosedInputStream`].
pub trait StdinStream: Send {
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
    fn stream(&self) -> Box<dyn HostInputStream>;

    /// Returns whether this stream is backed by a TTY.
    fn isatty(&self) -> bool;
}

impl StdinStream for pipe::MemoryInputPipe {
    fn stream(&self) -> Box<dyn HostInputStream> {
        Box::new(self.clone())
    }

    fn isatty(&self) -> bool {
        false
    }
}

impl StdinStream for pipe::ClosedInputStream {
    fn stream(&self) -> Box<dyn HostInputStream> {
        Box::new(self.clone())
    }

    fn isatty(&self) -> bool {
        false
    }
}

/// An impl of [`StdinStream`] built on top of [`crate::pipe::AsyncReadStream`].
pub struct AsyncStdinStream(Arc<Mutex<crate::pipe::AsyncReadStream>>);

impl AsyncStdinStream {
    pub fn new(s: crate::pipe::AsyncReadStream) -> Self {
        Self(Arc::new(Mutex::new(s)))
    }
}

impl StdinStream for AsyncStdinStream {
    fn stream(&self) -> Box<dyn HostInputStream> {
        Box::new(Self(self.0.clone()))
    }
    fn isatty(&self) -> bool {
        false
    }
}

impl HostInputStream for AsyncStdinStream {
    fn read(&mut self, size: usize) -> Result<bytes::Bytes, crate::StreamError> {
        self.0.lock().unwrap().read(size)
    }
    fn skip(&mut self, size: usize) -> Result<usize, crate::StreamError> {
        self.0.lock().unwrap().skip(size)
    }
}

impl Subscribe for AsyncStdinStream {
    fn ready<'a, 'b>(&'a mut self) -> Pin<Box<dyn Future<Output = ()> + Send + 'b>>
    where
        Self: 'b,
        'a: 'b,
    {
        struct F(AsyncStdinStream);
        impl Future for F {
            type Output = ();
            fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
                let mut inner = self.0 .0.lock().unwrap();
                let mut fut = inner.ready();
                fut.as_mut().poll(cx)
            }
        }
        Box::pin(F(Self(self.0.clone())))
    }
}

mod worker_thread_stdin;
pub use self::worker_thread_stdin::{stdin, Stdin};

/// Similar to [`StdinStream`], except for output.
pub trait StdoutStream: Send {
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
    fn stream(&self) -> Box<dyn HostOutputStream>;

    /// Returns whether this stream is backed by a TTY.
    fn isatty(&self) -> bool;
}

impl StdoutStream for pipe::MemoryOutputPipe {
    fn stream(&self) -> Box<dyn HostOutputStream> {
        Box::new(self.clone())
    }

    fn isatty(&self) -> bool {
        false
    }
}

impl StdoutStream for pipe::SinkOutputStream {
    fn stream(&self) -> Box<dyn HostOutputStream> {
        Box::new(self.clone())
    }

    fn isatty(&self) -> bool {
        false
    }
}

impl StdoutStream for pipe::ClosedOutputStream {
    fn stream(&self) -> Box<dyn HostOutputStream> {
        Box::new(self.clone())
    }

    fn isatty(&self) -> bool {
        false
    }
}

/// This implementation will yield output streams that block on writes, and
/// output directly to a file. If truly async output is required, [`AsyncStdoutStream`]
/// should be used instead.
pub struct OutputFile {
    file: Arc<std::fs::File>,
}

impl OutputFile {
    pub fn new(file: std::fs::File) -> Self {
        Self {
            file: Arc::new(file),
        }
    }
}

impl StdoutStream for OutputFile {
    fn stream(&self) -> Box<dyn HostOutputStream> {
        Box::new(OutputFileStream {
            file: Arc::clone(&self.file),
        })
    }

    fn isatty(&self) -> bool {
        false
    }
}

struct OutputFileStream {
    file: Arc<std::fs::File>,
}

#[async_trait::async_trait]
impl Subscribe for OutputFileStream {
    async fn ready(&mut self) {}
}

impl HostOutputStream for OutputFileStream {
    fn write(&mut self, bytes: Bytes) -> StreamResult<()> {
        use std::io::Write;
        self.file
            .write_all(&bytes)
            .map_err(|e| StreamError::LastOperationFailed(anyhow::anyhow!(e)))
    }

    fn flush(&mut self) -> StreamResult<()> {
        use std::io::Write;
        self.file
            .flush()
            .map_err(|e| StreamError::LastOperationFailed(anyhow::anyhow!(e)))
    }

    fn check_write(&mut self) -> StreamResult<usize> {
        Ok(1024 * 1024)
    }
}

/// This implementation will yield output streams that block on writes, as they
/// inherit the implementation directly from the rust std library. A different
/// implementation of [`StdoutStream`] will be necessary if truly async output
/// streams are required.
pub struct Stdout;

/// Returns a stream that represents the host's standard out.
///
/// Suitable for passing to
/// [`WasiCtxBuilder::stdout`](crate::WasiCtxBuilder::stdout).
pub fn stdout() -> Stdout {
    Stdout
}

impl StdoutStream for Stdout {
    fn stream(&self) -> Box<dyn HostOutputStream> {
        Box::new(OutputStream::Stdout)
    }

    fn isatty(&self) -> bool {
        std::io::stdout().is_terminal()
    }
}

/// This implementation will yield output streams that block on writes, as they
/// inherit the implementation directly from the rust std library. A different
/// implementation of [`StdoutStream`] will be necessary if truly async output
/// streams are required.
pub struct Stderr;

/// Returns a stream that represents the host's standard err.
///
/// Suitable for passing to
/// [`WasiCtxBuilder::stderr`](crate::WasiCtxBuilder::stderr).
pub fn stderr() -> Stderr {
    Stderr
}

impl StdoutStream for Stderr {
    fn stream(&self) -> Box<dyn HostOutputStream> {
        Box::new(OutputStream::Stderr)
    }

    fn isatty(&self) -> bool {
        std::io::stderr().is_terminal()
    }
}

enum OutputStream {
    Stdout,
    Stderr,
}

impl HostOutputStream for OutputStream {
    fn write(&mut self, bytes: Bytes) -> StreamResult<()> {
        use std::io::Write;
        match self {
            OutputStream::Stdout => std::io::stdout().write_all(&bytes),
            OutputStream::Stderr => std::io::stderr().write_all(&bytes),
        }
        .map_err(|e| StreamError::LastOperationFailed(anyhow::anyhow!(e)))
    }

    fn flush(&mut self) -> StreamResult<()> {
        use std::io::Write;
        match self {
            OutputStream::Stdout => std::io::stdout().flush(),
            OutputStream::Stderr => std::io::stderr().flush(),
        }
        .map_err(|e| StreamError::LastOperationFailed(anyhow::anyhow!(e)))
    }

    fn check_write(&mut self) -> StreamResult<usize> {
        Ok(1024 * 1024)
    }
}

#[async_trait::async_trait]
impl Subscribe for OutputStream {
    async fn ready(&mut self) {}
}

/// A wrapper of [`crate::pipe::AsyncWriteStream`] that implements
/// [`StdoutStream`]. Note that the [`HostOutputStream`] impl for this is not
/// correct when used for interleaved async IO.
pub struct AsyncStdoutStream(Arc<Mutex<crate::pipe::AsyncWriteStream>>);

impl AsyncStdoutStream {
    pub fn new(s: crate::pipe::AsyncWriteStream) -> Self {
        Self(Arc::new(Mutex::new(s)))
    }
}

impl StdoutStream for AsyncStdoutStream {
    fn stream(&self) -> Box<dyn HostOutputStream> {
        Box::new(Self(self.0.clone()))
    }
    fn isatty(&self) -> bool {
        false
    }
}

// This implementation is known to be bogus. All check-writes and writes are
// directed at the same underlying stream. The check-write/write protocol does
// require the size returned by a check-write to be accepted by write, even if
// other side-effects happen between those calls, and this implementation
// permits another view (created by StdoutStream::stream()) of the same
// underlying stream to accept a write which will invalidate a prior
// check-write of another view.
// Ultimately, the Std{in,out}Stream::stream() methods exist because many
// different places in a linked component (which may itself contain many
// modules) may need to access stdio without any coordination to keep those
// accesses all using pointing to the same resource. So, we allow many
// resources to be created. We have the reasonable expectation that programs
// won't attempt to interleave async IO from these disparate uses of stdio.
// If that expectation doesn't turn out to be true, and you find yourself at
// this comment to correct it: sorry about that.
impl HostOutputStream for AsyncStdoutStream {
    fn check_write(&mut self) -> Result<usize, StreamError> {
        self.0.lock().unwrap().check_write()
    }
    fn write(&mut self, bytes: Bytes) -> Result<(), StreamError> {
        self.0.lock().unwrap().write(bytes)
    }
    fn flush(&mut self) -> Result<(), StreamError> {
        self.0.lock().unwrap().flush()
    }
}

impl Subscribe for AsyncStdoutStream {
    fn ready<'a, 'b>(&'a mut self) -> Pin<Box<dyn Future<Output = ()> + Send + 'b>>
    where
        Self: 'b,
        'a: 'b,
    {
        struct F(AsyncStdoutStream);
        impl Future for F {
            type Output = ();
            fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
                let mut inner = self.0 .0.lock().unwrap();
                let mut fut = inner.ready();
                fut.as_mut().poll(cx)
            }
        }
        Box::pin(F(Self(self.0.clone())))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsATTY {
    Yes,
    No,
}

impl<T> stdin::Host for WasiImpl<T>
where
    T: WasiView,
{
    fn get_stdin(&mut self) -> Result<Resource<streams::InputStream>, anyhow::Error> {
        let stream = self.ctx().stdin.stream();
        Ok(self.table().push(streams::InputStream::Host(stream))?)
    }
}

impl<T> stdout::Host for WasiImpl<T>
where
    T: WasiView,
{
    fn get_stdout(&mut self) -> Result<Resource<streams::OutputStream>, anyhow::Error> {
        let stream = self.ctx().stdout.stream();
        Ok(self.table().push(stream)?)
    }
}

impl<T> stderr::Host for WasiImpl<T>
where
    T: WasiView,
{
    fn get_stderr(&mut self) -> Result<Resource<streams::OutputStream>, anyhow::Error> {
        let stream = self.ctx().stderr.stream();
        Ok(self.table().push(stream)?)
    }
}

pub struct TerminalInput;
pub struct TerminalOutput;

impl<T> terminal_input::Host for WasiImpl<T> where T: WasiView {}
impl<T> terminal_input::HostTerminalInput for WasiImpl<T>
where
    T: WasiView,
{
    fn drop(&mut self, r: Resource<TerminalInput>) -> anyhow::Result<()> {
        self.table().delete(r)?;
        Ok(())
    }
}
impl<T> terminal_output::Host for WasiImpl<T> where T: WasiView {}
impl<T> terminal_output::HostTerminalOutput for WasiImpl<T>
where
    T: WasiView,
{
    fn drop(&mut self, r: Resource<TerminalOutput>) -> anyhow::Result<()> {
        self.table().delete(r)?;
        Ok(())
    }
}
impl<T> terminal_stdin::Host for WasiImpl<T>
where
    T: WasiView,
{
    fn get_terminal_stdin(&mut self) -> anyhow::Result<Option<Resource<TerminalInput>>> {
        if self.ctx().stdin.isatty() {
            let fd = self.table().push(TerminalInput)?;
            Ok(Some(fd))
        } else {
            Ok(None)
        }
    }
}
impl<T> terminal_stdout::Host for WasiImpl<T>
where
    T: WasiView,
{
    fn get_terminal_stdout(&mut self) -> anyhow::Result<Option<Resource<TerminalOutput>>> {
        if self.ctx().stdout.isatty() {
            let fd = self.table().push(TerminalOutput)?;
            Ok(Some(fd))
        } else {
            Ok(None)
        }
    }
}
impl<T> terminal_stderr::Host for WasiImpl<T>
where
    T: WasiView,
{
    fn get_terminal_stderr(&mut self) -> anyhow::Result<Option<Resource<TerminalOutput>>> {
        if self.ctx().stderr.isatty() {
            let fd = self.table().push(TerminalOutput)?;
            Ok(Some(fd))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn memory_stdin_stream() {
        // A StdinStream has the property that there are multiple
        // HostInputStreams created, using the stream() method which are each
        // views on the same shared state underneath. Consuming input on one
        // stream results in consuming that input on all streams.
        //
        // The simplest way to measure this is to check if the MemoryInputPipe
        // impl of StdinStream follows this property.

        let pipe = super::pipe::MemoryInputPipe::new(
            "the quick brown fox jumped over the three lazy dogs",
        );

        use super::StdinStream;

        let mut view1 = pipe.stream();
        let mut view2 = pipe.stream();

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
        // HostInputStreams created, using the stream() method which are each
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
        let stdin_stream = super::AsyncStdinStream::new(crate::pipe::AsyncReadStream::new(file));

        use super::StdinStream;

        let mut view1 = stdin_stream.stream();
        let mut view2 = stdin_stream.stream();

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
}
