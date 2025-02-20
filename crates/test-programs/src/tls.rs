use crate::wasi::clocks::monotonic_clock;
use crate::wasi::tls::types::{ClientConnection, ClientHandshake, InputStream, OutputStream};

const TIMEOUT_NS: u64 = 1_000_000_000;

impl ClientHandshake {
    pub fn blocking_finish(self) -> Result<(ClientConnection, InputStream, OutputStream), ()> {
        let future = ClientHandshake::finish(self);
        let timeout = monotonic_clock::subscribe_duration(TIMEOUT_NS * 200);
        let pollable = future.subscribe();

        loop {
            match future.get() {
                None => pollable.block_until(&timeout).expect("timed out"),
                Some(Ok(r)) => return r,
                Some(Err(e)) => {
                    eprintln!("{:?}", e);
                    unimplemented!()
                }
            }
        }
    }
}

impl ClientConnection {
    pub fn blocking_close_notify(
        &self,
        output: &OutputStream,
    ) -> Result<(), crate::wasi::io::error::Error> {
        let timeout = monotonic_clock::subscribe_duration(TIMEOUT_NS);
        let pollable = output.subscribe();

        loop {
            match self.close_notify() {
                None => pollable.block_until(&timeout).expect("timed out"),
                Some(result) => return result,
            }
        }
    }
}
