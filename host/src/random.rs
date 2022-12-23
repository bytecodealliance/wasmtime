use cap_rand::{distributions::Standard, Rng};

use crate::{wasi_random, WasiCtx};

#[async_trait::async_trait]
impl wasi_random::WasiRandom for WasiCtx {
    async fn get_random_bytes(&mut self, len: u32) -> anyhow::Result<Vec<u8>> {
        Ok((&mut self.random)
            .sample_iter(Standard)
            .take(len as usize)
            .collect())
    }

    async fn get_random_u64(&mut self) -> anyhow::Result<u64> {
        Ok(self.random.sample(Standard))
    }
}
