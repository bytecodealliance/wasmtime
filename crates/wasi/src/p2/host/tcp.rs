use crate::p2::bindings::{
    sockets::network::{ErrorCode, IpAddressFamily, IpSocketAddress, Network},
    sockets::tcp::{self, ShutdownType},
};
use crate::p2::{Pollable, SocketResult};
use crate::sockets::{SocketAddrUse, TcpSocket, WasiSocketsCtxView};
use std::net::SocketAddr;
use wasmtime::component::Resource;
use wasmtime_wasi_io::{
    poll::DynPollable,
    streams::{DynInputStream, DynOutputStream},
};

impl tcp::Host for WasiSocketsCtxView<'_> {}

impl crate::p2::host::tcp::tcp::HostTcpSocket for WasiSocketsCtxView<'_> {
    async fn start_bind(
        &mut self,
        this: Resource<TcpSocket>,
        network: Resource<Network>,
        local_address: IpSocketAddress,
    ) -> SocketResult<()> {
        let network = self.table.get(&network)?;
        let local_address: SocketAddr = local_address.into();

        // Ensure that we're allowed to connect to this address.
        network
            .check_socket_addr(local_address, SocketAddrUse::TcpBind)
            .await?;

        // Bind to the address.
        self.table.get_mut(&this)?.start_bind(local_address)?;

        Ok(())
    }

    fn finish_bind(&mut self, this: Resource<TcpSocket>) -> SocketResult<()> {
        let socket = self.table.get_mut(&this)?;
        socket.finish_bind()?;
        Ok(())
    }

    async fn start_connect(
        &mut self,
        this: Resource<TcpSocket>,
        network: Resource<Network>,
        remote_address: IpSocketAddress,
    ) -> SocketResult<()> {
        let network = self.table.get(&network)?;
        let remote_address: SocketAddr = remote_address.into();

        // Ensure that we're allowed to connect to this address.
        network
            .check_socket_addr(remote_address, SocketAddrUse::TcpConnect)
            .await?;

        // Start connection
        let socket = self.table.get_mut(&this)?;
        let future = socket
            .start_connect(&remote_address)?
            .connect(remote_address);
        socket.set_pending_connect(future)?;

        Ok(())
    }

    fn finish_connect(
        &mut self,
        this: Resource<TcpSocket>,
    ) -> SocketResult<(Resource<DynInputStream>, Resource<DynOutputStream>)> {
        let socket = self.table.get_mut(&this)?;

        let result = socket
            .take_pending_connect()?
            .ok_or(ErrorCode::WouldBlock)?;
        socket.finish_connect(result)?;
        let (input, output) = socket.p2_streams()?;
        let input = self.table.push_child(input, &this)?;
        let output = self.table.push_child(output, &this)?;
        Ok((input, output))
    }

    fn start_listen(&mut self, this: Resource<TcpSocket>) -> SocketResult<()> {
        let socket = self.table.get_mut(&this)?;

        socket.start_listen()?;
        Ok(())
    }

    fn finish_listen(&mut self, this: Resource<TcpSocket>) -> SocketResult<()> {
        let socket = self.table.get_mut(&this)?;
        socket.finish_listen()?;
        Ok(())
    }

    fn accept(
        &mut self,
        this: Resource<TcpSocket>,
    ) -> SocketResult<(
        Resource<TcpSocket>,
        Resource<DynInputStream>,
        Resource<DynOutputStream>,
    )> {
        let socket = self.table.get_mut(&this)?;

        let mut tcp_socket = socket.accept()?.ok_or(ErrorCode::WouldBlock)?;
        let (input, output) = tcp_socket.p2_streams()?;

        let tcp_socket = self.table.push(tcp_socket)?;
        let input_stream = self.table.push_child(input, &tcp_socket)?;
        let output_stream = self.table.push_child(output, &tcp_socket)?;

        Ok((tcp_socket, input_stream, output_stream))
    }

    fn local_address(&mut self, this: Resource<TcpSocket>) -> SocketResult<IpSocketAddress> {
        let socket = self.table.get(&this)?;
        Ok(socket.local_address()?.into())
    }

    fn remote_address(&mut self, this: Resource<TcpSocket>) -> SocketResult<IpSocketAddress> {
        let socket = self.table.get(&this)?;
        Ok(socket.remote_address()?.into())
    }

    fn is_listening(&mut self, this: Resource<TcpSocket>) -> Result<bool, anyhow::Error> {
        let socket = self.table.get(&this)?;

        Ok(socket.is_listening())
    }

    fn address_family(
        &mut self,
        this: Resource<TcpSocket>,
    ) -> Result<IpAddressFamily, anyhow::Error> {
        let socket = self.table.get(&this)?;
        Ok(socket.address_family().into())
    }

    fn set_listen_backlog_size(
        &mut self,
        this: Resource<TcpSocket>,
        value: u64,
    ) -> SocketResult<()> {
        let socket = self.table.get_mut(&this)?;
        socket.set_listen_backlog_size(value)?;
        Ok(())
    }

    fn keep_alive_enabled(&mut self, this: Resource<TcpSocket>) -> SocketResult<bool> {
        let socket = self.table.get(&this)?;
        Ok(socket.keep_alive_enabled()?)
    }

    fn set_keep_alive_enabled(
        &mut self,
        this: Resource<TcpSocket>,
        value: bool,
    ) -> SocketResult<()> {
        let socket = self.table.get(&this)?;
        socket.set_keep_alive_enabled(value)?;
        Ok(())
    }

    fn keep_alive_idle_time(&mut self, this: Resource<TcpSocket>) -> SocketResult<u64> {
        let socket = self.table.get(&this)?;
        Ok(socket.keep_alive_idle_time()?)
    }

    fn set_keep_alive_idle_time(
        &mut self,
        this: Resource<TcpSocket>,
        value: u64,
    ) -> SocketResult<()> {
        let socket = self.table.get_mut(&this)?;
        socket.set_keep_alive_idle_time(value)?;
        Ok(())
    }

    fn keep_alive_interval(&mut self, this: Resource<TcpSocket>) -> SocketResult<u64> {
        let socket = self.table.get(&this)?;
        Ok(socket.keep_alive_interval()?)
    }

    fn set_keep_alive_interval(
        &mut self,
        this: Resource<TcpSocket>,
        value: u64,
    ) -> SocketResult<()> {
        let socket = self.table.get(&this)?;
        socket.set_keep_alive_interval(value)?;
        Ok(())
    }

    fn keep_alive_count(&mut self, this: Resource<TcpSocket>) -> SocketResult<u32> {
        let socket = self.table.get(&this)?;
        Ok(socket.keep_alive_count()?)
    }

    fn set_keep_alive_count(&mut self, this: Resource<TcpSocket>, value: u32) -> SocketResult<()> {
        let socket = self.table.get(&this)?;
        socket.set_keep_alive_count(value)?;
        Ok(())
    }

    fn hop_limit(&mut self, this: Resource<TcpSocket>) -> SocketResult<u8> {
        let socket = self.table.get(&this)?;
        Ok(socket.hop_limit()?)
    }

    fn set_hop_limit(&mut self, this: Resource<TcpSocket>, value: u8) -> SocketResult<()> {
        let socket = self.table.get_mut(&this)?;
        socket.set_hop_limit(value)?;
        Ok(())
    }

    fn receive_buffer_size(&mut self, this: Resource<TcpSocket>) -> SocketResult<u64> {
        let socket = self.table.get(&this)?;
        Ok(socket.receive_buffer_size()?)
    }

    fn set_receive_buffer_size(
        &mut self,
        this: Resource<TcpSocket>,
        value: u64,
    ) -> SocketResult<()> {
        let socket = self.table.get_mut(&this)?;
        socket.set_receive_buffer_size(value)?;
        Ok(())
    }

    fn send_buffer_size(&mut self, this: Resource<TcpSocket>) -> SocketResult<u64> {
        let socket = self.table.get(&this)?;
        Ok(socket.send_buffer_size()?)
    }

    fn set_send_buffer_size(&mut self, this: Resource<TcpSocket>, value: u64) -> SocketResult<()> {
        let socket = self.table.get_mut(&this)?;
        socket.set_send_buffer_size(value)?;
        Ok(())
    }

    fn subscribe(&mut self, this: Resource<TcpSocket>) -> anyhow::Result<Resource<DynPollable>> {
        wasmtime_wasi_io::poll::subscribe(self.table, this)
    }

    fn shutdown(
        &mut self,
        this: Resource<TcpSocket>,
        shutdown_type: ShutdownType,
    ) -> SocketResult<()> {
        let socket = self.table.get(&this)?;

        let how = match shutdown_type {
            ShutdownType::Receive => std::net::Shutdown::Read,
            ShutdownType::Send => std::net::Shutdown::Write,
            ShutdownType::Both => std::net::Shutdown::Both,
        };

        let state = socket.p2_streaming_state()?;
        state.shutdown(how)?;
        Ok(())
    }

    fn drop(&mut self, this: Resource<TcpSocket>) -> Result<(), anyhow::Error> {
        // As in the filesystem implementation, we assume closing a socket
        // doesn't block.
        let dropped = self.table.delete(this)?;
        drop(dropped);

        Ok(())
    }
}

#[async_trait::async_trait]
impl Pollable for TcpSocket {
    async fn ready(&mut self) {
        <TcpSocket>::ready(self).await;
    }
}

pub mod sync {
    use crate::p2::{
        SocketError,
        bindings::{
            sockets::{
                network::Network,
                tcp::{self as async_tcp, HostTcpSocket as AsyncHostTcpSocket},
            },
            sync::sockets::tcp::{
                self, Duration, HostTcpSocket, InputStream, IpAddressFamily, IpSocketAddress,
                OutputStream, Pollable, ShutdownType, TcpSocket,
            },
        },
    };
    use crate::runtime::in_tokio;
    use crate::sockets::WasiSocketsCtxView;
    use wasmtime::component::Resource;

    impl tcp::Host for WasiSocketsCtxView<'_> {}

    impl HostTcpSocket for WasiSocketsCtxView<'_> {
        fn start_bind(
            &mut self,
            self_: Resource<TcpSocket>,
            network: Resource<Network>,
            local_address: IpSocketAddress,
        ) -> Result<(), SocketError> {
            in_tokio(async {
                AsyncHostTcpSocket::start_bind(self, self_, network, local_address).await
            })
        }

        fn finish_bind(&mut self, self_: Resource<TcpSocket>) -> Result<(), SocketError> {
            AsyncHostTcpSocket::finish_bind(self, self_)
        }

        fn start_connect(
            &mut self,
            self_: Resource<TcpSocket>,
            network: Resource<Network>,
            remote_address: IpSocketAddress,
        ) -> Result<(), SocketError> {
            in_tokio(async {
                AsyncHostTcpSocket::start_connect(self, self_, network, remote_address).await
            })
        }

        fn finish_connect(
            &mut self,
            self_: Resource<TcpSocket>,
        ) -> Result<(Resource<InputStream>, Resource<OutputStream>), SocketError> {
            AsyncHostTcpSocket::finish_connect(self, self_)
        }

        fn start_listen(&mut self, self_: Resource<TcpSocket>) -> Result<(), SocketError> {
            AsyncHostTcpSocket::start_listen(self, self_)
        }

        fn finish_listen(&mut self, self_: Resource<TcpSocket>) -> Result<(), SocketError> {
            AsyncHostTcpSocket::finish_listen(self, self_)
        }

        fn accept(
            &mut self,
            self_: Resource<TcpSocket>,
        ) -> Result<
            (
                Resource<TcpSocket>,
                Resource<InputStream>,
                Resource<OutputStream>,
            ),
            SocketError,
        > {
            AsyncHostTcpSocket::accept(self, self_)
        }

        fn local_address(
            &mut self,
            self_: Resource<TcpSocket>,
        ) -> Result<IpSocketAddress, SocketError> {
            AsyncHostTcpSocket::local_address(self, self_)
        }

        fn remote_address(
            &mut self,
            self_: Resource<TcpSocket>,
        ) -> Result<IpSocketAddress, SocketError> {
            AsyncHostTcpSocket::remote_address(self, self_)
        }

        fn is_listening(&mut self, self_: Resource<TcpSocket>) -> wasmtime::Result<bool> {
            AsyncHostTcpSocket::is_listening(self, self_)
        }

        fn address_family(
            &mut self,
            self_: Resource<TcpSocket>,
        ) -> wasmtime::Result<IpAddressFamily> {
            AsyncHostTcpSocket::address_family(self, self_)
        }

        fn set_listen_backlog_size(
            &mut self,
            self_: Resource<TcpSocket>,
            value: u64,
        ) -> Result<(), SocketError> {
            AsyncHostTcpSocket::set_listen_backlog_size(self, self_, value)
        }

        fn keep_alive_enabled(&mut self, self_: Resource<TcpSocket>) -> Result<bool, SocketError> {
            AsyncHostTcpSocket::keep_alive_enabled(self, self_)
        }

        fn set_keep_alive_enabled(
            &mut self,
            self_: Resource<TcpSocket>,
            value: bool,
        ) -> Result<(), SocketError> {
            AsyncHostTcpSocket::set_keep_alive_enabled(self, self_, value)
        }

        fn keep_alive_idle_time(
            &mut self,
            self_: Resource<TcpSocket>,
        ) -> Result<Duration, SocketError> {
            AsyncHostTcpSocket::keep_alive_idle_time(self, self_)
        }

        fn set_keep_alive_idle_time(
            &mut self,
            self_: Resource<TcpSocket>,
            value: Duration,
        ) -> Result<(), SocketError> {
            AsyncHostTcpSocket::set_keep_alive_idle_time(self, self_, value)
        }

        fn keep_alive_interval(
            &mut self,
            self_: Resource<TcpSocket>,
        ) -> Result<Duration, SocketError> {
            AsyncHostTcpSocket::keep_alive_interval(self, self_)
        }

        fn set_keep_alive_interval(
            &mut self,
            self_: Resource<TcpSocket>,
            value: Duration,
        ) -> Result<(), SocketError> {
            AsyncHostTcpSocket::set_keep_alive_interval(self, self_, value)
        }

        fn keep_alive_count(&mut self, self_: Resource<TcpSocket>) -> Result<u32, SocketError> {
            AsyncHostTcpSocket::keep_alive_count(self, self_)
        }

        fn set_keep_alive_count(
            &mut self,
            self_: Resource<TcpSocket>,
            value: u32,
        ) -> Result<(), SocketError> {
            AsyncHostTcpSocket::set_keep_alive_count(self, self_, value)
        }

        fn hop_limit(&mut self, self_: Resource<TcpSocket>) -> Result<u8, SocketError> {
            AsyncHostTcpSocket::hop_limit(self, self_)
        }

        fn set_hop_limit(
            &mut self,
            self_: Resource<TcpSocket>,
            value: u8,
        ) -> Result<(), SocketError> {
            AsyncHostTcpSocket::set_hop_limit(self, self_, value)
        }

        fn receive_buffer_size(&mut self, self_: Resource<TcpSocket>) -> Result<u64, SocketError> {
            AsyncHostTcpSocket::receive_buffer_size(self, self_)
        }

        fn set_receive_buffer_size(
            &mut self,
            self_: Resource<TcpSocket>,
            value: u64,
        ) -> Result<(), SocketError> {
            AsyncHostTcpSocket::set_receive_buffer_size(self, self_, value)
        }

        fn send_buffer_size(&mut self, self_: Resource<TcpSocket>) -> Result<u64, SocketError> {
            AsyncHostTcpSocket::send_buffer_size(self, self_)
        }

        fn set_send_buffer_size(
            &mut self,
            self_: Resource<TcpSocket>,
            value: u64,
        ) -> Result<(), SocketError> {
            AsyncHostTcpSocket::set_send_buffer_size(self, self_, value)
        }

        fn subscribe(
            &mut self,
            self_: Resource<TcpSocket>,
        ) -> wasmtime::Result<Resource<Pollable>> {
            AsyncHostTcpSocket::subscribe(self, self_)
        }

        fn shutdown(
            &mut self,
            self_: Resource<TcpSocket>,
            shutdown_type: ShutdownType,
        ) -> Result<(), SocketError> {
            AsyncHostTcpSocket::shutdown(self, self_, shutdown_type.into())
        }

        fn drop(&mut self, rep: Resource<TcpSocket>) -> wasmtime::Result<()> {
            AsyncHostTcpSocket::drop(self, rep)
        }
    }

    impl From<ShutdownType> for async_tcp::ShutdownType {
        fn from(other: ShutdownType) -> Self {
            match other {
                ShutdownType::Receive => async_tcp::ShutdownType::Receive,
                ShutdownType::Send => async_tcp::ShutdownType::Send,
                ShutdownType::Both => async_tcp::ShutdownType::Both,
            }
        }
    }
}
