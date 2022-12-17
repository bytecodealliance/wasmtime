use cap_rand::{distributions::Standard, Rng};

use crate::{wasi_random, WasiCtx};

#[async_trait::async_trait]
impl wasi_random::WasiRandom for WasiCtx {
    async fn getrandom(&mut self, len: u32) -> anyhow::Result<Vec<u8>> {
        Ok((&mut self.random)
            .sample_iter(Standard)
            .take(len as usize)
            .collect())
    }
}
