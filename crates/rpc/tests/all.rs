use core::convert::Infallible;
use core::ops::Deref as _;
use core::pin::{pin, Pin};
use core::task::{Context, Poll};

use std::collections::HashMap;
use std::io;
use std::sync::{Arc, Mutex};

use anyhow::bail;
use test_programs_artifacts::{foreach_rpc, RPC_HELLO_CLIENT_COMPONENT};
use tokio::io::{AsyncRead, AsyncWrite};
use wasmtime::component::types;
use wasmtime::{
    component::{Component, Linker},
    Store,
};
use wasmtime_rpc::{link_function, WrpcView};
use wasmtime_wasi::bindings::Command;
use wasmtime_wasi::pipe::MemoryOutputPipe;
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiView};
use wrpc_transport::{Index, Invoke};

// Assert that we are testing everything through assertion
// of the existence of the test function itself.
macro_rules! assert_test_exists {
    ($name:ident) => {
        #[allow(unused_imports)]
        use self::$name as _;
    };
}

foreach_rpc!(assert_test_exists);

pub struct Ctx<C> {
    pub table: ResourceTable,
    pub wasi: WasiCtx,
    pub wrpc: C,
    pub stderr: MemoryOutputPipe,
}

impl<C: Invoke<Error = wasmtime::Error>> WrpcView<C> for Ctx<C> {
    fn client(&self) -> &C {
        &self.wrpc
    }
}

impl<C: Send> WasiView for Ctx<C> {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

impl<C> Drop for Ctx<C> {
    fn drop(&mut self) {
        let stderr = self.stderr.contents();
        if !stderr.is_empty() {
            println!("[guest] stderr:\n{}\n===", String::from_utf8_lossy(&stderr));
        }
    }
}

#[derive(Clone)]
struct Session {
    outgoing: Arc<Mutex<Option<Result<(), String>>>>,
    incoming: Result<(), String>,
}

impl wrpc_transport::Session for Session {
    type Error = String;
    type TransportError = Infallible;

    async fn finish(
        self,
        res: Result<(), Self::Error>,
    ) -> Result<Result<(), Self::Error>, Self::TransportError> {
        let mut lock = self.outgoing.lock().unwrap();
        assert!(lock.is_none());
        *lock = Some(res);
        Ok(self.incoming)
    }
}

#[derive(Clone)]
struct Incoming {
    indexes: Arc<Mutex<HashMap<Vec<usize>, io::Cursor<&'static [u8]>>>>,
    path: Vec<usize>,
}

impl Index<Self> for Incoming {
    type Error = Infallible;

    fn index(&self, path: &[usize]) -> Result<Self, Self::Error> {
        let mut target = self.path.to_vec();
        target.extend(path);
        Ok(Self {
            indexes: self.indexes.clone(),
            path: target,
        })
    }
}

impl AsyncRead for Incoming {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let mut lock = self.indexes.lock().unwrap();
        let r = lock
            .get_mut(&self.path)
            .expect(&format!("unknown path {:?}", self.path));
        pin!(r).poll_read(cx, buf)
    }
}

#[derive(Clone)]
struct Outgoing {
    indexes: Arc<Mutex<HashMap<Vec<usize>, Vec<u8>>>>,
    path: Vec<usize>,
}

impl Index<Self> for Outgoing {
    type Error = Infallible;

    fn index(&self, path: &[usize]) -> Result<Self, Self::Error> {
        let mut target = self.path.to_vec();
        target.extend(path);
        Ok(Self {
            indexes: self.indexes.clone(),
            path: target,
        })
    }
}

impl AsyncWrite for Outgoing {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        let mut lock = self.indexes.lock().unwrap();
        let w = lock.entry(self.path.clone()).or_default();
        pin!(w).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        let mut lock = self.indexes.lock().unwrap();
        if let Some(w) = lock.get_mut(&self.path) {
            pin!(w).poll_flush(cx)
        } else {
            Poll::Ready(Ok(()))
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        let mut lock = self.indexes.lock().unwrap();
        if let Some(w) = lock.get_mut(&self.path) {
            pin!(w).poll_shutdown(cx)
        } else {
            Poll::Ready(Ok(()))
        }
    }
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn rpc_hello_client() -> anyhow::Result<()> {
    struct Transport {
        outgoing: Outgoing,
        incoming: Incoming,
        session: Session,
    }
    impl Invoke for Transport {
        type Error = wasmtime::Error;
        type Context = &'static str;
        type Session = Session;
        type Outgoing = Outgoing;
        type NestedOutgoing = Outgoing;
        type Incoming = Incoming;

        async fn invoke(
            &self,
            cx: Self::Context,
            instance: &str,
            func: &str,
            params: bytes::Bytes,
            paths: &[&[Option<usize>]],
        ) -> Result<
            wrpc_transport::Invocation<Self::Outgoing, Self::Incoming, Self::Session>,
            Self::Error,
        > {
            assert_eq!(cx, "test");
            assert_eq!(instance, "rpc-examples:hello/handler");
            assert_eq!(func, "hello");
            assert_eq!(
                params,
                [8, b'w', b'a', b's', b'm', b't', b'i', b'm', b'e'].as_slice()
            );
            assert!(paths.is_empty());
            Ok(wrpc_transport::Invocation {
                outgoing: self.outgoing.clone(),
                incoming: self.incoming.clone(),
                session: self.session.clone(),
            })
        }
    }

    let engine = test_programs_artifacts::engine(|config| {
        config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
        config.async_support(true);
    });
    let component = Component::from_file(&engine, RPC_HELLO_CLIENT_COMPONENT)?;

    let stdout = MemoryOutputPipe::new(4096);
    let stderr = MemoryOutputPipe::new(4096);

    let wasi = WasiCtxBuilder::new()
        .stdout(stdout.clone())
        .stderr(stderr.clone())
        .build();
    let outgoing = Arc::default();
    let incoming = Arc::new(Mutex::new(HashMap::from([(
        vec![],
        io::Cursor::new(
            [
                15, b'H', b'e', b'l', b'l', b'o', b',', b' ', b'w', b'a', b's', b'm', b't', b'i',
                b'm', b'e',
            ]
            .as_slice(),
        ),
    )])));
    let error = Arc::default();
    let ctx = Ctx {
        table: ResourceTable::new(),
        wasi,
        wrpc: Transport {
            outgoing: Outgoing {
                indexes: Arc::clone(&outgoing),
                path: vec![],
            },
            incoming: Incoming {
                indexes: incoming,
                path: vec![],
            },
            session: Session {
                outgoing: error,
                incoming: Ok(()),
            },
        },
        stderr,
    };
    let mut store = Store::new(&engine, ctx);
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::add_to_linker_async(&mut linker)?;

    {
        let Some(types::ComponentItem::ComponentInstance(ty)) = component
            .component_type()
            .get_import(&engine, "rpc-examples:hello/handler")
        else {
            bail!("`rpc-examples:hello/handler` instance import not found")
        };
        let mut linker = linker.instance("rpc-examples:hello/handler")?;
        let Some(types::ComponentItem::ComponentFunc(ty)) = ty.get_export(&engine, "hello") else {
            bail!(
                "`hello` function export not found in `rpc-examples:hello/handler` instance import"
            )
        };
        link_function(
            &mut linker,
            ty,
            "rpc-examples:hello/handler",
            "hello",
            "test",
        )?;
    }

    let (command, _instance) = Command::instantiate_async(&mut store, &component, &linker).await?;
    let result = command.wasi_cli_run().call_run(&mut store).await?;
    result.map_err(|()| anyhow::anyhow!("run failed"))?;
    assert_eq!(stdout.contents(), "Hello, wasmtime");
    assert_eq!(outgoing.lock().unwrap().deref(), &HashMap::default());
    Ok(())
}
