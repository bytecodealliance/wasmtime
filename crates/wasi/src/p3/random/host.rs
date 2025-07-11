use cap_rand::Rng;
use cap_rand::distributions::Standard;

use crate::p3::bindings::random::{insecure, insecure_seed, random};
use crate::p3::random::{WasiRandomImpl, WasiRandomView};

impl<T> random::Host for WasiRandomImpl<T>
where
    T: WasiRandomView,
{
    fn get_random_bytes(&mut self, len: u64) -> wasmtime::Result<Vec<u8>> {
        Ok((&mut self.random().random)
            .sample_iter(Standard)
            .take(len as usize)
            .collect())
    }

    fn get_random_u64(&mut self) -> wasmtime::Result<u64> {
        Ok(self.random().random.sample(Standard))
    }
}

impl<T> insecure::Host for WasiRandomImpl<T>
where
    T: WasiRandomView,
{
    fn get_insecure_random_bytes(&mut self, len: u64) -> wasmtime::Result<Vec<u8>> {
        Ok((&mut self.random().insecure_random)
            .sample_iter(Standard)
            .take(len as usize)
            .collect())
    }

    fn get_insecure_random_u64(&mut self) -> wasmtime::Result<u64> {
        Ok(self.random().insecure_random.sample(Standard))
    }
}

impl<T> insecure_seed::Host for WasiRandomImpl<T>
where
    T: WasiRandomView,
{
    fn insecure_seed(&mut self) -> wasmtime::Result<(u64, u64)> {
        let seed: u128 = self.random().insecure_random_seed;
        Ok((seed as u64, (seed >> 64) as u64))
    }
}
