use futures::TryFutureExt;

use crate::{
    p2::bindings::sockets::network::ErrorCode,
    sockets::{MaybeReady, UdpSocket as P3Socket, noop_cx},
};
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

/// A UDP socket + associated p2 bookkeeping.
pub struct UdpSocket {
    pub(crate) inner: Arc<Mutex<P3Socket>>,
    pub(crate) in_progress_operation: Option<AsyncOperation>,
}
impl UdpSocket {
    pub(crate) fn new(inner: P3Socket) -> Self {
        Self {
            inner: Arc::new(Mutex::new(inner)),
            in_progress_operation: None,
        }
    }
    pub(crate) fn get_mut(&mut self) -> Option<&mut P3Socket> {
        Arc::get_mut(&mut self.inner)?.get_mut().ok()
    }
    pub(crate) fn lock(&self) -> std::sync::MutexGuard<'_, P3Socket> {
        self.inner.lock().expect("other thread panicked")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AsyncOperation {
    Bind,
}

pub struct IncomingDatagramStream {
    pub(crate) inner: Arc<Mutex<P3Socket>>,
    pub(crate) connected_addr: Option<SocketAddr>,
    pub(crate) current_recv: Option<MaybeReady<Result<(Vec<u8>, SocketAddr), ErrorCode>>>,
}
impl IncomingDatagramStream {
    pub(crate) fn new(inner: Arc<Mutex<P3Socket>>) -> Self {
        let connected_addr = inner.lock().unwrap().remote_address().ok();
        Self {
            inner,
            connected_addr,
            current_recv: None,
        }
    }
    pub(crate) fn poll_recv_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<()> {
        if self.current_recv.is_none() {
            let connected_addr = self.connected_addr;
            let inner = self.inner.clone();
            let recv = MaybeReady::poll_or_spawn(async move {
                loop {
                    let fut = inner.lock().unwrap().recv();
                    let (data, addr) = fut.await?;

                    // Only process the packet if it matches the expected remote
                    // address (if connected). Under normal circumstances, the
                    // OS should already do this filtering for us. However,
                    // nothing in POSIX guarantees this behavior, especially
                    // after (re)connecting a socket with already queued packets
                    // from a different peer. Case in point: on Linux the
                    // filtering happens when the packet is received from the
                    // network, *not* when the packet is delivered to the
                    // application as part of `recvfrom`.
                    if let Some(connected_addr) = connected_addr
                        && connected_addr != addr
                    {
                        continue;
                    }

                    return Ok((data, addr));
                }
            });
            self.current_recv = Some(recv);
        }

        self.current_recv
            .as_mut()
            .unwrap()
            .poll_ready(cx)
            .map(|_| ())
    }

    pub(crate) fn try_recv(&mut self) -> Result<(Vec<u8>, SocketAddr), ErrorCode> {
        let noop_cx = &mut noop_cx();
        if self.poll_recv_ready(noop_cx).is_pending() {
            return Err(ErrorCode::WouldBlock);
        }

        self.current_recv.take().unwrap().unwrap_ready()
    }
}

pub struct OutgoingDatagramStream {
    pub(crate) inner: Arc<Mutex<P3Socket>>,
    /// Number of datagrams permitted by most recent `check-send` call.
    pub(crate) check_send_permit_count: usize,
    pub(crate) prev_send: Option<MaybeReady<Result<(), ErrorCode>>>,
}
impl OutgoingDatagramStream {
    pub(crate) fn new(inner: Arc<Mutex<P3Socket>>) -> Self {
        Self {
            inner,
            check_send_permit_count: 0,
            prev_send: None,
        }
    }
    pub(crate) fn poll_send_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<()> {
        match &mut self.prev_send {
            Some(send) => send.poll_ready(cx).map(|_| ()),
            None => std::task::Poll::Ready(()),
        }
    }

    pub(crate) fn try_send(
        &mut self,
        data: Vec<u8>,
        addr: Option<std::net::SocketAddr>,
    ) -> Result<(), ErrorCode> {
        if let Some(send) = &mut self.prev_send {
            if !send.poll_ready(&mut noop_cx()).is_ready() {
                return Err(ErrorCode::WouldBlock);
            }

            let result = self.prev_send.take().unwrap().unwrap_ready();
            if let Err(e) = result {
                return Err(e);
            }
        }

        debug_assert!(self.prev_send.is_none());

        let mut send = MaybeReady::poll_or_spawn(
            self.inner
                .lock()
                .unwrap()
                .send(data, addr)
                .map_err(|e| e.into()),
        );
        if send.poll_ready(&mut noop_cx()).is_ready() {
            send.unwrap_ready()
        } else {
            self.prev_send = Some(send);
            Ok(())
        }
    }
}
