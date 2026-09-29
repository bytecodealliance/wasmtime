use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};
use wasmtime::component::{
    Component, Destination, Linker, Source, StreamConsumer, StreamProducer, StreamReader,
    StreamResult,
};
use wasmtime::{Config, Engine, Result, Store, StoreContextMut};

const DROPPED_ZERO_ITEMS: u32 = 1;

struct SyncDroppedProducer {
    dropped: Arc<AtomicBool>,
}

impl Drop for SyncDroppedProducer {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::SeqCst);
    }
}

impl StreamProducer<()> for SyncDroppedProducer {
    type Item = u8;
    type Buffer = Option<u8>;

    fn poll_produce<'a>(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _store: StoreContextMut<'a, ()>,
        _destination: Destination<'a, Self::Item, Self::Buffer>,
        _finish: bool,
    ) -> Poll<Result<StreamResult>> {
        Poll::Ready(Ok(StreamResult::Dropped))
    }
}

struct SyncDroppedConsumer {
    dropped: Arc<AtomicBool>,
}

impl Drop for SyncDroppedConsumer {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::SeqCst);
    }
}

impl StreamConsumer<()> for SyncDroppedConsumer {
    type Item = u8;

    fn poll_consume(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _store: StoreContextMut<()>,
        _source: Source<'_, Self::Item>,
        _finish: bool,
    ) -> Poll<Result<StreamResult>> {
        Poll::Ready(Ok(StreamResult::Dropped))
    }
}

fn async_stream_engine() -> Result<Engine> {
    let mut config = Config::new();
    config.wasm_component_model_async(true);
    Engine::new(&config)
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn stream_read_sync_dropped_retires_host_producer() -> Result<()> {
    let engine = async_stream_engine()?;

    let component = Component::new(
        &engine,
        r#"
        (component
            (core module $libc (memory (export "mem") 1))
            (core instance $libc (instantiate $libc))
            (core module $m
                (import "" "stream.read" (func $stream.read (param i32 i32 i32) (result i32)))

                ;; Read up to (param 1) items into memory at address 0 and
                ;; return the packed result code. Deliberately does NOT call
                ;; stream.drop-readable: the readable handle stays open.
                (func (export "read") (param i32 i32) (result i32)
                    (call $stream.read (local.get 0) (i32.const 0) (local.get 1))
                )
            )
            (type $s (stream u8))
            (core func $stream.read (canon stream.read $s async (memory (core memory $libc "mem"))))
            (core instance $i (instantiate $m
                (with "" (instance
                    (export "stream.read" (func $stream.read))
                ))
            ))
            (func (export "read") (param "s" (stream u8)) (param "l" u32) (result u32)
                (canon lift (core func $i "read")))
        )
        "#,
    )?;

    let mut store = Store::new(&engine, ());
    let instance = Linker::new(&engine)
        .instantiate_async(&mut store, &component)
        .await?;
    let read = instance.get_typed_func::<(StreamReader<u8>, u32), (u32,)>(&mut store, "read")?;

    let dropped = Arc::new(AtomicBool::new(false));
    let reader = StreamReader::new(
        &mut store,
        SyncDroppedProducer {
            dropped: dropped.clone(),
        },
    )?;

    let (code,) = read.call_async(&mut store, (reader, 4)).await?;

    assert_eq!(
        code, DROPPED_ZERO_ITEMS,
        "guest should observe DROPPED with a count of 0"
    );

    assert!(
        dropped.load(Ordering::SeqCst),
        "producer must be dropped promptly after reporting StreamResult::Dropped \
         from a synchronous poll; it is being stranded in WriteState::HostReady \
         until the guest drops its readable end"
    );

    Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn stream_write_sync_dropped_retires_host_consumer() -> Result<()> {
    let engine = async_stream_engine()?;

    let component = Component::new(
        &engine,
        r#"
        (component
            (core module $libc (memory (export "mem") 1))
            (core instance $libc (instantiate $libc))
            (core module $m
                (import "" "stream.new" (func $stream.new (result i64)))
                (import "" "stream.write" (func $stream.write (param i32 i32 i32) (result i32)))

                ;; The writable end of the stream created by `start`.
                (global $writable (mut i32) (i32.const 0))

                ;; Create a stream; keep the writable end in a global and
                ;; return the readable end to the host. (`stream.new` packs
                ;; the two ends as (writable << 32) | readable.)
                (func (export "start") (result i32)
                    (local $pair i64)
                    (local.set $pair (call $stream.new))
                    (global.set $writable
                        (i32.wrap_i64 (i64.shr_u (local.get $pair) (i64.const 32))))
                    (i32.wrap_i64 (local.get $pair))
                )

                ;; Write 1 item from memory address 0 and return the packed
                ;; result code. Deliberately does NOT call
                ;; stream.drop-writable: the writable handle stays open.
                (func (export "write") (result i32)
                    (call $stream.write (global.get $writable) (i32.const 0) (i32.const 1))
                )
            )
            (type $s (stream u8))
            (core func $stream.new (canon stream.new $s))
            (core func $stream.write (canon stream.write $s async (memory (core memory $libc "mem"))))
            (core instance $i (instantiate $m
                (with "" (instance
                    (export "stream.new" (func $stream.new))
                    (export "stream.write" (func $stream.write))
                ))
            ))
            (func (export "start") (result (stream u8))
                (canon lift (core func $i "start")))
            (func (export "write") (result u32)
                (canon lift (core func $i "write")))
        )
        "#,
    )?;

    let mut store = Store::new(&engine, ());
    let instance = Linker::new(&engine)
        .instantiate_async(&mut store, &component)
        .await?;
    let start = instance.get_typed_func::<(), (StreamReader<u8>,)>(&mut store, "start")?;
    let write = instance.get_typed_func::<(), (u32,)>(&mut store, "write")?;

    let (reader,) = start.call_async(&mut store, ()).await?;

    let dropped = Arc::new(AtomicBool::new(false));
    reader.pipe(
        &mut store,
        SyncDroppedConsumer {
            dropped: dropped.clone(),
        },
    )?;

    let (code,) = write.call_async(&mut store, ()).await?;

    assert_eq!(
        code, DROPPED_ZERO_ITEMS,
        "guest should observe DROPPED with a count of 0"
    );

    assert!(
        dropped.load(Ordering::SeqCst),
        "consumer must be dropped promptly after reporting StreamResult::Dropped \
         from a synchronous poll; it is being stranded in ReadState::HostReady \
         until the guest drops its writable end"
    );

    Ok(())
}
