use crate::cli::{IsTerminal, StdoutStream};
use crate::p2;
use bytes::Bytes;
use std::io::{self, Write};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::AsyncWrite;
use wasmtime_wasi_io::streams::OutputStream;

// Implementation for tokio::io::Stdout
impl IsTerminal for tokio::io::Stdout {
    fn is_terminal(&self) -> bool {
        std::io::stdout().is_terminal()
    }
}
impl StdoutStream for tokio::io::Stdout {
    fn p2_stream(&self) -> Box<dyn OutputStream> {
        Box::new(StdioOutputStream::Stdout)
    }
    fn async_stream(&self) -> Box<dyn AsyncWrite + Send + Sync> {
        Box::new(StdioOutputStream::Stdout)
    }
}

// Implementation for std::io::Stdout
impl IsTerminal for std::io::Stdout {
    fn is_terminal(&self) -> bool {
        std::io::IsTerminal::is_terminal(self)
    }
}
impl StdoutStream for std::io::Stdout {
    fn p2_stream(&self) -> Box<dyn OutputStream> {
        Box::new(StdioOutputStream::Stdout)
    }
    fn async_stream(&self) -> Box<dyn AsyncWrite + Send + Sync> {
        Box::new(StdioOutputStream::Stdout)
    }
}

// Implementation for tokio::io::Stderr
impl IsTerminal for tokio::io::Stderr {
    fn is_terminal(&self) -> bool {
        std::io::stderr().is_terminal()
    }
}
impl StdoutStream for tokio::io::Stderr {
    fn p2_stream(&self) -> Box<dyn OutputStream> {
        Box::new(StdioOutputStream::Stderr)
    }
    fn async_stream(&self) -> Box<dyn AsyncWrite + Send + Sync> {
        Box::new(StdioOutputStream::Stderr)
    }
}

// Implementation for std::io::Stderr
impl IsTerminal for std::io::Stderr {
    fn is_terminal(&self) -> bool {
        std::io::IsTerminal::is_terminal(self)
    }
}
impl StdoutStream for std::io::Stderr {
    fn p2_stream(&self) -> Box<dyn OutputStream> {
        Box::new(StdioOutputStream::Stderr)
    }
    fn async_stream(&self) -> Box<dyn AsyncWrite + Send + Sync> {
        Box::new(StdioOutputStream::Stderr)
    }
}

enum StdioOutputStream {
    Stdout,
    Stderr,
}

impl OutputStream for StdioOutputStream {
    fn write(&mut self, bytes: Bytes) -> p2::StreamResult<()> {
        match self {
            StdioOutputStream::Stdout => std::io::stdout().write_all(&bytes),
            StdioOutputStream::Stderr => std::io::stderr().write_all(&bytes),
        }
        .map_err(|e| p2::StreamError::LastOperationFailed(anyhow::anyhow!(e)))
    }

    fn flush(&mut self) -> p2::StreamResult<()> {
        match self {
            StdioOutputStream::Stdout => std::io::stdout().flush(),
            StdioOutputStream::Stderr => std::io::stderr().flush(),
        }
        .map_err(|e| p2::StreamError::LastOperationFailed(anyhow::anyhow!(e)))
    }

    fn check_write(&mut self) -> p2::StreamResult<usize> {
        Ok(1024 * 1024)
    }
}

impl AsyncWrite for StdioOutputStream {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Poll::Ready(match *self {
            StdioOutputStream::Stdout => std::io::stdout().write(buf),
            StdioOutputStream::Stderr => std::io::stderr().write(buf),
        })
    }
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(match *self {
            StdioOutputStream::Stdout => std::io::stdout().flush(),
            StdioOutputStream::Stderr => std::io::stderr().flush(),
        })
    }
    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[async_trait::async_trait]
impl p2::Pollable for StdioOutputStream {
    async fn ready(&mut self) {}
}
