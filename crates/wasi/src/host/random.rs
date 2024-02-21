use crate::bindings::random::{insecure, insecure_seed, random};
use crate::WasiView;
use cap_rand::{distributions::Standard, Rng};

impl<T: WasiView> random::Host for T {
    fn get_random_bytes(&mut self, len: u64) -> anyhow::Result<Vec<u8>> {
        Ok((&mut self.ctx().random)
            .sample_iter(Standard)
            .take(len as usize)
            .collect())
    }

    fn get_random_u64(&mut self) -> anyhow::Result<u64> {
        Ok(self.ctx().random.sample(Standard))
    }
}

impl<T: WasiView> insecure::Host for T {
    fn get_insecure_random_bytes(&mut self, len: u64) -> anyhow::Result<Vec<u8>> {
        Ok((&mut self.ctx().insecure_random)
            .sample_iter(Standard)
            .take(len as usize)
            .collect())
    }

    fn get_insecure_random_u64(&mut self) -> anyhow::Result<u64> {
        Ok(self.ctx().insecure_random.sample(Standard))
    }
}

impl<T: WasiView> insecure_seed::Host for T {
    fn insecure_seed(&mut self) -> anyhow::Result<(u64, u64)> {
        let seed: u128 = self.ctx().insecure_random_seed;
        Ok((seed as u64, (seed >> 64) as u64))
    }
}
