use cap_rand::{distributions::Standard, Rng};

use crate::{command, proxy, WasiCtx};

async fn get_random_bytes(ctx: &mut WasiCtx, len: u64) -> anyhow::Result<Vec<u8>> {
    Ok((&mut ctx.random)
        .sample_iter(Standard)
        .take(len as usize)
        .collect())
}

async fn get_random_u64(ctx: &mut WasiCtx) -> anyhow::Result<u64> {
    Ok(ctx.random.sample(Standard))
}

async fn insecure_random(_ctx: &mut WasiCtx) -> anyhow::Result<(u64, u64)> {
    Ok((0, 0))
}

// Implementatations of the traits for both the command and proxy worlds.
// The bodies have been pulled out into functions above to allow them to
// be shared between the two. Ideally, we should add features to the
// bindings to facilitate this kind of sharing.

#[async_trait::async_trait]
impl command::wasi::random::Host for WasiCtx {
    async fn get_random_bytes(&mut self, len: u64) -> anyhow::Result<Vec<u8>> {
        get_random_bytes(self, len).await
    }

    async fn get_random_u64(&mut self) -> anyhow::Result<u64> {
        get_random_u64(self).await
    }

    async fn insecure_random(&mut self) -> anyhow::Result<(u64, u64)> {
        insecure_random(self).await
    }
}

#[async_trait::async_trait]
impl proxy::wasi::random::Host for WasiCtx {
    async fn get_random_bytes(&mut self, len: u64) -> anyhow::Result<Vec<u8>> {
        get_random_bytes(self, len).await
    }

    async fn get_random_u64(&mut self) -> anyhow::Result<u64> {
        get_random_u64(self).await
    }

    async fn insecure_random(&mut self) -> anyhow::Result<(u64, u64)> {
        insecure_random(self).await
    }
}
