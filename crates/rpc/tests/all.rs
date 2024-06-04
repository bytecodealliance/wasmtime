use core::convert::Infallible;
use core::ops::Deref as _;
use core::pin::{pin, Pin};
use core::task::{Context, Poll};

use std::collections::HashMap;
use std::io;
use std::sync::{Arc, Mutex};

use anyhow::bail;
use test_programs_artifacts::{foreach_rpc, RPC_HELLO_CLIENT_COMPONENT, RPC_SYNC_CLIENT_COMPONENT};
use tokio::io::{AsyncRead, AsyncWrite};
use wasmtime::component::{types, Component, Linker};
use wasmtime::Store;
use wasmtime_rpc::{link_instance, WrpcView};
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

impl Default for Session {
    fn default() -> Self {
        Self {
            outgoing: Arc::default(),
            incoming: Ok(()),
        }
    }
}

impl wrpc_transport::Session for Session {
    type Error = String;
    type TransportError = Infallible;

    async fn finish(
        self,
        res: Result<(), Self::Error>,
    ) -> Result<Result<(), Self::Error>, Self::TransportError> {
        let mut lock = self.outgoing.lock().unwrap();
        assert_eq!(lock.as_ref(), None);
        *lock = Some(res);
        Ok(self.incoming)
    }
}

#[derive(Default, Clone)]
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

#[derive(Default, Clone)]
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
            assert_eq!(instance, "rpc-test:hello/handler");
            assert_eq!(func, "hello");
            assert_eq!(params, "\x08wasmtime".as_bytes());
            assert!(paths.is_empty());
            let indexes =
                HashMap::from([(vec![], io::Cursor::new("\x0fHello, wasmtime".as_bytes()))]);
            Ok(wrpc_transport::Invocation {
                outgoing: self.outgoing.clone(),
                incoming: Incoming {
                    indexes: Arc::new(Mutex::new(indexes)),
                    path: vec![],
                },
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
    let error = Arc::default();
    let ctx = Ctx {
        table: ResourceTable::new(),
        wasi,
        wrpc: Transport {
            outgoing: Outgoing {
                indexes: Arc::clone(&outgoing),
                path: vec![],
            },
            session: Session {
                outgoing: Arc::clone(&error),
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
            .get_import(&engine, "rpc-test:hello/handler")
        else {
            bail!("`rpc-test:hello/handler` instance import not found")
        };
        let mut linker = linker.instance("rpc-test:hello/handler")?;
        link_instance(&engine, &mut linker, ty, "rpc-test:hello/handler", "test")?;
    }

    let (command, _instance) = Command::instantiate_async(&mut store, &component, &linker).await?;
    let result = command.wasi_cli_run().call_run(&mut store).await?;
    result.map_err(|()| anyhow::anyhow!("run failed"))?;
    assert_eq!(stdout.contents(), "Hello, wasmtime");
    assert_eq!(outgoing.lock().unwrap().deref(), &HashMap::default());
    assert_eq!(error.lock().unwrap().deref(), &Some(Ok(())));
    Ok(())
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn rpc_sync_client() -> anyhow::Result<()> {
    struct Transport(
        Arc<Mutex<HashMap<(&'static str, &'static str, &'static [u8]), (Outgoing, Session)>>>,
    );

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
            let mut lock = self.0.lock().unwrap();
            let (outgoing, session, indexes) = match (instance, func) {
                ("foo", "foo") => {
                    assert_eq!(cx, "foo");
                    assert!(paths.is_empty());
                    assert_eq!(params, "\x03foo".as_bytes());
                    let (outgoing, session) = lock.get_mut(&("foo", "foo", b"\x03foo")).unwrap();
                    (outgoing, session, HashMap::default())
                }
                ("foo", "f") => {
                    assert_eq!(cx, "foo");
                    assert!(paths.is_empty());
                    assert_eq!(params, "\x03foo".as_bytes());
                    let (outgoing, session) = lock.get_mut(&("foo", "f", b"\x03foo")).unwrap();
                    (
                        outgoing,
                        session,
                        HashMap::from([(vec![], io::Cursor::new([42].as_slice()))]),
                    )
                }
                ("rpc-test:sync/sync", "fallible") => {
                    assert_eq!(cx, "sync");
                    assert!(paths.is_empty());
                    let (ret, params) = match params.as_ref() {
                        [0x00] => ("\x01\x04test".as_bytes(), "\x00".as_bytes()), // (error "test")
                        [0x01] => ("\x00\x01".as_bytes(), "\x01".as_bytes()),     // (ok true)
                        _ => panic!("invalid `fallible` parameter payload: {params:x?}"),
                    };
                    let (outgoing, session) = lock
                        .get_mut(&("rpc-test:sync/sync", "fallible", params))
                        .unwrap();
                    (
                        outgoing,
                        session,
                        HashMap::from([(vec![], io::Cursor::new(ret))]),
                    )
                }
                ("rpc-test:sync/sync", "numbers") => {
                    assert_eq!(cx, "sync");
                    assert!(paths.is_empty());
                    assert_eq!(params, "".as_bytes());
                    let (outgoing, session) = lock
                        .get_mut(&("rpc-test:sync/sync", "numbers", b""))
                        .unwrap();
                    debug_assert_eq!(9.0f32.to_le_bytes(), b"\x00\x00\x10\x41".as_slice());
                    debug_assert_eq!(
                        10.0f64.to_le_bytes(),
                        b"\x00\x00\x00\x00\x00\x00\x24\x40".as_slice()
                    );
                    (
                        outgoing,
                        session,
                        HashMap::from([(
                            vec![],
                            io::Cursor::new(
                                concat!(
                                    "\x01\x02\x03\x04\x05\x06\x07\x08", // 1 2 3 4 5 6 7 8
                                    "\x00\x00\x10\x41",                 // 9.0
                                    "\x00\x00\x00\x00\x00\x00\x24\x40"  // 10.0
                                )
                                .as_bytes(),
                            ),
                        )]),
                    )
                }
                ("rpc-test:sync/sync", "with-flags") => {
                    assert_eq!(cx, "sync");
                    assert!(paths.is_empty());
                    assert_eq!(params, "\x01\x00\x01".as_bytes());
                    let (outgoing, session) = lock
                        .get_mut(&("rpc-test:sync/sync", "with-flags", b"\x01\x00\x01"))
                        .unwrap();
                    (
                        outgoing,
                        session,
                        HashMap::from([(vec![], io::Cursor::new([0b101].as_slice()))]),
                    )
                }
                ("rpc-test:sync/sync", "with-variant-option") => {
                    assert_eq!(cx, "sync");
                    assert!(paths.is_empty());
                    let (ret, params) = match params.as_ref() {
                        [0x00] => ("\x00".as_bytes(), "\x00".as_bytes()), // none
                        [0x01] => ("\x01\x00\x03bar".as_bytes(), "\x01".as_bytes()), // (some (variant "var" (record (record "bar"))))
                        _ => panic!("invalid `with-variant-option` parameter payload: {params:x?}"),
                    };
                    let (outgoing, session) = lock
                        .get_mut(&("rpc-test:sync/sync", "with-variant-option", params))
                        .unwrap();
                    (
                        outgoing,
                        session,
                        HashMap::from([(vec![], io::Cursor::new(ret))]),
                    )
                }
                ("rpc-test:sync/sync", "with-record") => {
                    assert_eq!(cx, "sync");
                    assert!(paths.is_empty());
                    assert_eq!(params, "".as_bytes());
                    let (outgoing, session) = lock
                        .get_mut(&("rpc-test:sync/sync", "with-record", b""))
                        .unwrap();
                    (
                        outgoing,
                        session,
                        HashMap::from([(vec![], io::Cursor::new("\x03foo".as_bytes()))]),
                    )
                }
                ("rpc-test:sync/sync", "with-record-list") => {
                    assert_eq!(cx, "sync");
                    assert!(paths.is_empty());
                    let (ret, params) = match params.as_ref() {
                        [0x00] => ("\x00".as_bytes(), "\x00".as_bytes()), // (list)
                        [0x03] => (
                            concat!("\x03", "\x010", "\x011", "\x012").as_bytes(), // (list (record (record "0")) (record (record "1")) (record (record "2")))
                            "\x03".as_bytes(),
                        ),
                        _ => panic!("invalid `with-record-list` parameter payload: {params:x?}"),
                    };
                    let (outgoing, session) = lock
                        .get_mut(&("rpc-test:sync/sync", "with-record-list", params))
                        .unwrap();
                    (
                        outgoing,
                        session,
                        HashMap::from([(vec![], io::Cursor::new(ret))]),
                    )
                }
                ("rpc-test:sync/sync", "with-record-tuple") => {
                    assert_eq!(cx, "sync");
                    assert!(paths.is_empty());
                    assert_eq!(params, "".as_bytes());
                    let (outgoing, session) = lock
                        .get_mut(&("rpc-test:sync/sync", "with-record-tuple", b""))
                        .unwrap();
                    (
                        outgoing,
                        session,
                        HashMap::from([(
                            vec![],
                            io::Cursor::new(concat!("\x010", "\x011", "\x012").as_bytes()), // (tuple (record (record "0")) (record (record "1")) (record (record "2")))
                        )]),
                    )
                }
                ("rpc-test:sync/sync", "with-enum") => {
                    assert_eq!(cx, "sync");
                    assert!(paths.is_empty());
                    assert_eq!(params, "".as_bytes());
                    let (outgoing, session) = lock
                        .get_mut(&("rpc-test:sync/sync", "with-enum", b""))
                        .unwrap();
                    (
                        outgoing,
                        session,
                        HashMap::from([(vec![], io::Cursor::new("\x01".as_bytes()))]),
                    )
                }
                _ => panic!("unexpected function call `{func}` from instance `{instance}`"),
            };
            Ok(wrpc_transport::Invocation {
                outgoing: outgoing.clone(),
                incoming: Incoming {
                    indexes: Arc::new(Mutex::new(indexes)),
                    path: vec![],
                },
                session: session.clone(),
            })
        }
    }

    let engine = test_programs_artifacts::engine(|config| {
        config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
        config.async_support(true);
    });
    let component = Component::from_file(&engine, RPC_SYNC_CLIENT_COMPONENT)?;

    let stdout = MemoryOutputPipe::new(4096);
    let stderr = MemoryOutputPipe::new(4096);

    let wasi = WasiCtxBuilder::new()
        .stdout(stdout.clone())
        .stderr(stderr.clone())
        .build();
    let invocations = Arc::new(Mutex::new(HashMap::from([
        (
            ("foo", "foo", "\x03foo".as_bytes()),
            (Outgoing::default(), Session::default()),
        ),
        (
            ("foo", "f", b"\x03foo"),
            (Outgoing::default(), Session::default()),
        ),
        (
            ("rpc-test:sync/sync", "fallible", b"\x00"),
            (Outgoing::default(), Session::default()),
        ),
        (
            ("rpc-test:sync/sync", "fallible", b"\x01"),
            (Outgoing::default(), Session::default()),
        ),
        (
            ("rpc-test:sync/sync", "numbers", &[]),
            (Outgoing::default(), Session::default()),
        ),
        (
            ("rpc-test:sync/sync", "with-flags", b"\x01\x00\x01"),
            (Outgoing::default(), Session::default()),
        ),
        (
            ("rpc-test:sync/sync", "with-variant-option", b"\x00"),
            (Outgoing::default(), Session::default()),
        ),
        (
            ("rpc-test:sync/sync", "with-variant-option", b"\x01"),
            (Outgoing::default(), Session::default()),
        ),
        (
            ("rpc-test:sync/sync", "with-record", b""),
            (Outgoing::default(), Session::default()),
        ),
        (
            ("rpc-test:sync/sync", "with-record-list", b"\x00"),
            (Outgoing::default(), Session::default()),
        ),
        (
            ("rpc-test:sync/sync", "with-record-list", b"\x03"),
            (Outgoing::default(), Session::default()),
        ),
        (
            ("rpc-test:sync/sync", "with-record-tuple", b""),
            (Outgoing::default(), Session::default()),
        ),
        (
            ("rpc-test:sync/sync", "with-enum", b""),
            (Outgoing::default(), Session::default()),
        ),
    ])));
    let ctx = Ctx {
        table: ResourceTable::new(),
        wasi,
        wrpc: Transport(Arc::clone(&invocations)),
        stderr,
    };
    let mut store = Store::new(&engine, ctx);
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::add_to_linker_async(&mut linker)?;

    {
        let Some(types::ComponentItem::ComponentInstance(ty)) = component
            .component_type()
            .get_import(&engine, "rpc-test:sync/sync")
        else {
            bail!("`rpc-test:sync/sync` instance import not found")
        };
        let mut linker = linker.instance("rpc-test:sync/sync")?;
        link_instance(&engine, &mut linker, ty, "rpc-test:sync/sync", "sync")?;
    }
    {
        let Some(types::ComponentItem::ComponentInstance(ty)) =
            component.component_type().get_import(&engine, "foo")
        else {
            bail!("`foo` instance import not found")
        };
        let mut linker = linker.instance("foo")?;
        link_instance(&engine, &mut linker, ty, "foo", "foo")?;
    }

    let (command, _instance) = Command::instantiate_async(&mut store, &component, &linker).await?;
    let result = command.wasi_cli_run().call_run(&mut store).await?;
    result.map_err(|()| anyhow::anyhow!("run failed"))?;
    assert_eq!(stdout.contents(), "");
    for (_, (outgoing, session)) in invocations.lock().unwrap().iter() {
        assert_eq!(
            outgoing.indexes.lock().unwrap().deref(),
            &HashMap::default()
        );
        assert_eq!(session.outgoing.lock().unwrap().deref(), &Some(Ok(())));
    }
    Ok(())
}
