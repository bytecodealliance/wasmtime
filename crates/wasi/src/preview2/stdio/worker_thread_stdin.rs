use crate::preview2::{HostInputStream, StreamState};
use anyhow::{Context, Error};
use bytes::Bytes;
use tokio::sync::{mpsc, oneshot};

// wasmtime cant use std::sync::OnceLock yet because of a llvm regression in
// 1.70. when 1.71 is released, we can switch to using std here.
use once_cell::sync::OnceCell as OnceLock;

use std::sync::Mutex;

// We need a single global instance of the AsyncFd<Stdin> because creating
// this instance registers the process's stdin fd with epoll, which will
// return an error if an fd is registered more than once.
struct GlobalStdin {
    tx: mpsc::Sender<oneshot::Sender<anyhow::Result<()>>>,
    // FIXME use a Watch to check for readiness instead of sending a oneshot sender
}
static STDIN: OnceLock<Mutex<GlobalStdin>> = OnceLock::new();

fn create() -> Mutex<GlobalStdin> {
    let (tx, mut rx) = mpsc::channel::<oneshot::Sender<anyhow::Result<()>>>(1);
    std::thread::spawn(move || {
        use std::io::BufRead;
        // A client is interested in stdin's readiness.
        // Don't care about the None case - the GlobalStdin sender on the other
        // end of this pipe will live forever, because it lives inside the OnceLock.
        while let Some(msg) = rx.blocking_recv() {
            // Fill buf - can we skip this if its
            // already filled?
            // also, this could block forever and the
            // client could give up. in that case,
            // another client may want to start waiting
            let r = std::io::stdin()
                .lock()
                .fill_buf()
                .map(|_| ())
                .map_err(anyhow::Error::from);
            // tell the client stdin is ready for reading.
            // don't care if the client happens to have died.
            let _ = msg.send(r);
        }
    });

    Mutex::new(GlobalStdin { tx })
}

pub struct Stdin;
impl Stdin {
    fn get_global() -> &'static Mutex<GlobalStdin> {
        STDIN.get_or_init(|| create())
    }
}

pub fn stdin() -> Stdin {
    // This implementation still needs to be fixed, and we need better test coverage.
    // We are deferring that work to a future PR.
    // https://github.com/bytecodealliance/wasmtime/pull/6556#issuecomment-1646232646
    panic!("worker-thread based stdin is not yet implemented");
    // Stdin
}

#[async_trait::async_trait]
impl HostInputStream for Stdin {
    fn read(&mut self, size: usize) -> Result<(Bytes, StreamState), Error> {
        use std::io::Read;
        let mut buf = vec![0; size];
        // FIXME: this is actually blocking. This whole implementation is likely bogus as a result
        let nbytes = std::io::stdin().read(&mut buf)?;
        buf.truncate(nbytes);
        Ok((
            buf.into(),
            if nbytes > 0 {
                StreamState::Open
            } else {
                StreamState::Closed
            },
        ))
    }

    async fn ready(&mut self) -> Result<(), Error> {
        use mpsc::error::TrySendError;
        use std::future::Future;
        use std::pin::Pin;
        use std::task::{Context, Poll};

        // Custom Future impl takes the std mutex in each invocation of poll.
        // Required so we don't have to use a tokio mutex, which we can't take from
        // inside a sync context in Self::read.
        //
        // Take the lock, attempt to
        struct Send(Option<oneshot::Sender<anyhow::Result<()>>>);
        impl Future for Send {
            type Output = anyhow::Result<()>;
            fn poll(mut self: Pin<&mut Self>, _: &mut Context) -> Poll<Self::Output> {
                let locked = Stdin::get_global().lock().unwrap();
                let to_send = self.as_mut().0.take().expect("to_send should be some");
                match locked.tx.try_send(to_send) {
                    Ok(()) => Poll::Ready(Ok(())),
                    Err(TrySendError::Full(to_send)) => {
                        self.as_mut().0.replace(to_send);
                        Poll::Pending
                    }
                    Err(TrySendError::Closed(_)) => {
                        Poll::Ready(Err(anyhow::anyhow!("channel to GlobalStdin closed")))
                    }
                }
            }
        }

        let (result_tx, rx) = oneshot::channel::<anyhow::Result<()>>();
        Box::pin(Send(Some(result_tx)))
            .await
            .context("sending message to worker thread")?;
        rx.await.expect("channel is always alive")
    }
}
