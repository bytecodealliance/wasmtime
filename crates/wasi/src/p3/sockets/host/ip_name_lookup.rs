use crate::p3::bindings::sockets::ip_name_lookup::{ErrorCode, Host, HostWithStore};
use crate::p3::bindings::sockets::types;
use crate::p3::sockets::WasiSockets;
use crate::sockets::WasiSocketsCtxView;
use crate::sockets::ip_name_lookup::resolve_addresses;
use wasmtime::component::Accessor;

impl<U> HostWithStore<U> for WasiSockets {
    async fn resolve_addresses(
        store: &Accessor<U, Self>,
        name: String,
    ) -> wasmtime::Result<Result<Vec<types::IpAddress>, ErrorCode>> {
        store
            .with(|mut view| view.get().resolve_addresses(name))
            .await
    }
}

impl Host for WasiSocketsCtxView<'_> {}

impl WasiSocketsCtxView<'_> {
    fn resolve_addresses(
        &self,
        name: String,
    ) -> impl Future<Output = wasmtime::Result<Result<Vec<types::IpAddress>, ErrorCode>>> + use<>
    {
        let fut = resolve_addresses(self.ctx, name);
        async move {
            Ok(match fut.await {
                Ok(addrs) => Ok(addrs.into_iter().map(|addr| addr.into()).collect()),
                Err(err) => Err(err.into()),
            })
        }
    }
}

mod named {
    use crate::p3::bindings::named_imports::wasi::sockets::ip_name_lookup::{Host, HostWithStore};
    use crate::p3::bindings::sockets::ip_name_lookup::ErrorCode;
    use crate::p3::bindings::sockets::types;
    use crate::sockets::{WasiSocketsNamed, WasiSocketsNamedView};
    use crate::{NamedId, WasiCtxNamedView};
    use wasmtime::component::Accessor;

    impl<T, U> HostWithStore<U> for WasiSocketsNamed<T>
    where
        T: WasiSocketsNamedView,
    {
        async fn resolve_addresses(
            store: &Accessor<U, Self>,
            id: NamedId,
            name: String,
        ) -> wasmtime::Result<Result<Vec<types::IpAddress>, ErrorCode>> {
            store
                .with(|mut view| view.get().0.sockets(id).resolve_addresses(name))
                .await
        }
    }

    impl<T> Host for WasiCtxNamedView<'_, T> where T: WasiSocketsNamedView {}
}
