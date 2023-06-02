use crate::preview2::{
    wasi::poll::poll::{self, Pollable},
    WasiView,
};
use std::future::Future;
use std::pin::Pin;

pub struct HostPollable(Pin<Box<dyn Future<Output = ()>>>);

#[async_trait::async_trait]
impl<T: WasiView> poll::Host for T {
    async fn drop_pollable(&mut self, pollable: Pollable) -> anyhow::Result<()> {
        self.table_mut().delete::<HostPollable>(pollable)?;
        Ok(())
    }

    async fn poll_oneoff(&mut self, futures: Vec<Pollable>) -> anyhow::Result<Vec<u8>> {
        // Convert `futures` into `Poll` subscriptions.
        let len = futures.len();

        // Convert the results into a list of `u8` to return.
        let mut results = vec![0_u8; len];
        if todo!() {
            let index: usize = todo!();
            results[index] = u8::from(true);
        }
        Ok(results)
    }
}
