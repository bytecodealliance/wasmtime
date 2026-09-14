use crate::p2::bindings::random::{insecure, insecure_seed, random};
use crate::random::WasiRandomCtx;
use rand::RngExt;
use wasmtime::bail;

impl random::Host for WasiRandomCtx {
    fn get_random_bytes(&mut self, len: u64) -> wasmtime::Result<Vec<u8>> {
        if len > self.max_size {
            bail!("requested len {len:?} exceeds limit {}", self.max_size);
        }
        Ok((&mut self.random)
            .random_iter()
            .take(len as usize)
            .collect())
    }

    fn get_random_u64(&mut self) -> wasmtime::Result<u64> {
        Ok(self.random.random())
    }
}

impl insecure::Host for WasiRandomCtx {
    fn get_insecure_random_bytes(&mut self, len: u64) -> wasmtime::Result<Vec<u8>> {
        if len > self.max_size {
            bail!("requested len {len:?} exceeds limit {}", self.max_size);
        }
        Ok((&mut self.insecure_random)
            .random_iter()
            .take(len as usize)
            .collect())
    }

    fn get_insecure_random_u64(&mut self) -> wasmtime::Result<u64> {
        Ok(self.insecure_random.random())
    }
}

impl insecure_seed::Host for WasiRandomCtx {
    fn insecure_seed(&mut self) -> wasmtime::Result<(u64, u64)> {
        let seed: u128 = self.insecure_random_seed;
        Ok((seed as u64, (seed >> 64) as u64))
    }
}

mod named {
    use crate::p2::bindings::named_imports::wasi::random::{insecure, insecure_seed, random};
    use crate::random::WasiRandomNamedView;
    use crate::{NamedId, WasiCtxNamedView};

    impl<T> random::Host for WasiCtxNamedView<'_, T>
    where
        T: WasiRandomNamedView,
    {
        fn get_random_bytes(&mut self, id: NamedId, len: u64) -> wasmtime::Result<Vec<u8>> {
            super::random::Host::get_random_bytes(self.0.random(id), len)
        }

        fn get_random_u64(&mut self, id: NamedId) -> wasmtime::Result<u64> {
            super::random::Host::get_random_u64(self.0.random(id))
        }
    }

    impl<T> insecure::Host for WasiCtxNamedView<'_, T>
    where
        T: WasiRandomNamedView,
    {
        fn get_insecure_random_bytes(
            &mut self,
            id: NamedId,
            len: u64,
        ) -> wasmtime::Result<Vec<u8>> {
            super::insecure::Host::get_insecure_random_bytes(self.0.random(id), len)
        }

        fn get_insecure_random_u64(&mut self, id: NamedId) -> wasmtime::Result<u64> {
            super::insecure::Host::get_insecure_random_u64(self.0.random(id))
        }
    }

    impl<T> insecure_seed::Host for WasiCtxNamedView<'_, T>
    where
        T: WasiRandomNamedView,
    {
        fn insecure_seed(&mut self, id: NamedId) -> wasmtime::Result<(u64, u64)> {
            super::insecure_seed::Host::insecure_seed(self.0.random(id))
        }
    }
}
