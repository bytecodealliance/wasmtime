//! Tests for the `stream.forward` and `future.forward` built-ins with
//! host-owned producers and/or consumers.  Guest-to-guest forwarding is
//! covered by `tests/component-model/test/async/forward.wast`.

use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};
use wasmtime::component::{
    Component, Destination, FutureConsumer, FutureReader, Linker, Source, StreamConsumer,
    StreamProducer, StreamReader, StreamResult,
};
use wasmtime::{Config, Engine, Result, Store, StoreContextMut};

const BLOCKED: u32 = 0xffff_ffff;
const COMPLETED: u32 = 0;
const DROPPED: u32 = 1;
const COMPLETED_THREE_ITEMS: u32 = 3 << 4;
const DROPPED_THREE_ITEMS: u32 = (3 << 4) | 1;
const CANCELLED_THREE_ITEMS: u32 = (3 << 4) | 2;

/// A `StreamConsumer` which collects everything it receives and flags its own
/// destruction (which is how the host is notified that the stream ended).
struct CollectConsumer {
    data: Arc<Mutex<Vec<u8>>>,
    dropped: Arc<AtomicBool>,
}

impl Drop for CollectConsumer {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::SeqCst);
    }
}

impl StreamConsumer<()> for CollectConsumer {
    type Item = u8;

    fn poll_consume(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        store: StoreContextMut<()>,
        mut source: Source<Self::Item>,
        _finish: bool,
    ) -> Poll<Result<StreamResult>> {
        let mut buffer = Vec::with_capacity(64);
        source.read(store, &mut buffer)?;
        self.data.lock().unwrap().extend(buffer);
        Poll::Ready(Ok(StreamResult::Completed))
    }
}

/// A guest component which can create streams, forward into them, and
/// read/write on either side of a forward.
///
/// Memory layout: bytes read land at address 0; bytes written come from
/// address 16.
const FORWARDER: &str = r#"
(component
  (core module $libc (memory (export "m") 1))
  (core instance $libc (instantiate $libc))

  (type $s (stream u8))
  (core func $stream.new (canon stream.new $s))
  (core func $stream.read (canon stream.read $s async (memory (core memory $libc "m"))))
  (core func $stream.write (canon stream.write $s async (memory (core memory $libc "m"))))
  (core func $stream.forward (canon stream.forward $s))
  (core func $stream.cancel-read (canon stream.cancel-read $s async))
  (core func $stream.cancel-write (canon stream.cancel-write $s async))
  (core func $stream.drop-writable (canon stream.drop-writable $s))

  (core module $m
    (import "" "m" (memory 1))
    (import "" "stream.new" (func $stream.new (result i64)))
    (import "" "stream.read" (func $stream.read (param i32 i32 i32) (result i32)))
    (import "" "stream.write" (func $stream.write (param i32 i32 i32) (result i32)))
    (import "" "stream.forward" (func $stream.forward (param i32 i32)))
    (import "" "stream.cancel-read" (func $stream.cancel-read (param i32) (result i32)))
    (import "" "stream.cancel-write" (func $stream.cancel-write (param i32) (result i32)))
    (import "" "stream.drop-writable" (func $stream.drop-writable (param i32)))

    ;; The writable end of the stream returned by `mk`.
    (global $dst-w (mut i32) (i32.const 0))
    ;; The ends of the stream created by `mk-src`.
    (global $src-r (mut i32) (i32.const 0))
    (global $src-w (mut i32) (i32.const 0))

    ;; Create a new stream, saving its writable end and returning its
    ;; readable end (which the host will consume).
    (func (export "mk") (result i32)
      (local $tmp i64)
      (local.set $tmp (call $stream.new))
      (global.set $dst-w (i32.wrap_i64 (i64.shr_u (local.get $tmp) (i64.const 32))))
      (i32.wrap_i64 (local.get $tmp))
    )

    ;; Create a new stream, saving both ends (which this component will
    ;; produce into).
    (func (export "mk-src")
      (local $tmp i64)
      (local.set $tmp (call $stream.new))
      (global.set $src-r (i32.wrap_i64 (local.get $tmp)))
      (global.set $src-w (i32.wrap_i64 (i64.shr_u (local.get $tmp) (i64.const 32))))
    )

    ;; Forward the stream created by `mk-src` into the stream created by `mk`.
    (func (export "fwd-src")
      (call $stream.forward (global.get $src-r) (global.get $dst-w))
    )

    ;; Forward the given stream into the stream created by `mk`.
    (func (export "fwd") (param $r i32)
      (call $stream.forward (local.get $r) (global.get $dst-w))
    )

    ;; Write "xyz" to the stream created by `mk-src`, returning the packed
    ;; result code.
    (func (export "write") (result i32)
      (i32.store8 (i32.const 16) (i32.const 120))
      (i32.store8 (i32.const 17) (i32.const 121))
      (i32.store8 (i32.const 18) (i32.const 122))
      (call $stream.write (global.get $src-w) (i32.const 16) (i32.const 3))
    )

    ;; Retrieve the completion of a pending write on the stream created by
    ;; `mk-src` by cancelling it, returning the packed result code.
    (func (export "check-write") (result i32)
      (call $stream.cancel-write (global.get $src-w))
    )

    ;; Drop the writable end of the stream created by `mk-src`.
    (func (export "drop-w")
      (call $stream.drop-writable (global.get $src-w))
    )

    ;; Forward the given stream into a fresh stream, then read three bytes
    ;; from the latter, checking that "abc" arrived.
    (func (export "run") (param $r i32) (result i32)
      (local $tmp i64) (local $r2 i32) (local $w2 i32) (local $code i32)
      (local.set $tmp (call $stream.new))
      (local.set $r2 (i32.wrap_i64 (local.get $tmp)))
      (local.set $w2 (i32.wrap_i64 (i64.shr_u (local.get $tmp) (i64.const 32))))

      (call $stream.forward (local.get $r) (local.get $w2))

      (local.set $code (call $stream.read (local.get $r2) (i32.const 0) (i32.const 3)))
      (call $check-abc)
      (local.get $code)
    )

    ;; Like `run`, except the read is issued (and blocks) before the forward,
    ;; in which case its completion is delivered as an event, retrieved here
    ;; via `stream.cancel-read`.
    (func (export "run-pending") (param $r i32) (result i32)
      (local $tmp i64) (local $r2 i32) (local $w2 i32) (local $code i32)
      (local.set $tmp (call $stream.new))
      (local.set $r2 (i32.wrap_i64 (local.get $tmp)))
      (local.set $w2 (i32.wrap_i64 (i64.shr_u (local.get $tmp) (i64.const 32))))

      (call $stream.read (local.get $r2) (i32.const 0) (i32.const 3))
      i32.const -1 ;; BLOCKED
      i32.ne
      if unreachable end

      (call $stream.forward (local.get $r) (local.get $w2))

      (local.set $code (call $stream.cancel-read (local.get $r2)))
      (call $check-abc)
      (local.get $code)
    )

    ;; Read eight bytes from a fresh stream (which blocks), write "abc" into
    ;; it (leaving an undelivered completion with three bytes copied), then
    ;; forward the given stream into it and retrieve the completion via
    ;; `stream.cancel-read`, returning the packed result code.
    (func (export "run-partial") (param $r i32) (result i32)
      (local $tmp i64) (local $r2 i32) (local $w2 i32) (local $code i32)
      (local.set $tmp (call $stream.new))
      (local.set $r2 (i32.wrap_i64 (local.get $tmp)))
      (local.set $w2 (i32.wrap_i64 (i64.shr_u (local.get $tmp) (i64.const 32))))

      (call $stream.read (local.get $r2) (i32.const 0) (i32.const 8))
      i32.const -1 ;; BLOCKED
      i32.ne
      if unreachable end

      (i32.store8 (i32.const 16) (i32.const 97))
      (i32.store8 (i32.const 17) (i32.const 98))
      (i32.store8 (i32.const 18) (i32.const 99))
      (call $stream.write (local.get $w2) (i32.const 16) (i32.const 3))
      i32.const 48 ;; COMPLETED, three items
      i32.ne
      if unreachable end

      (call $stream.forward (local.get $r) (local.get $w2))

      (local.set $code (call $stream.cancel-read (local.get $r2)))
      (call $check-abc)
      (local.get $code)
    )

    (func $check-abc
      (i32.ne (i32.load8_u (i32.const 0)) (i32.const 97))
      if unreachable end
      (i32.ne (i32.load8_u (i32.const 1)) (i32.const 98))
      if unreachable end
      (i32.ne (i32.load8_u (i32.const 2)) (i32.const 99))
      if unreachable end
    )
  )

  (core instance $i (instantiate $m
    (with "" (instance
      (export "m" (memory $libc "m"))
      (export "stream.new" (func $stream.new))
      (export "stream.read" (func $stream.read))
      (export "stream.write" (func $stream.write))
      (export "stream.forward" (func $stream.forward))
      (export "stream.cancel-read" (func $stream.cancel-read))
      (export "stream.cancel-write" (func $stream.cancel-write))
      (export "stream.drop-writable" (func $stream.drop-writable))
    ))
  ))

  (func (export "mk") (result (stream u8)) (canon lift (core func $i "mk")))
  (func (export "mk-src") (canon lift (core func $i "mk-src")))
  (func (export "fwd-src") (canon lift (core func $i "fwd-src")))
  (func (export "fwd") (param "s" (stream u8)) (canon lift (core func $i "fwd")))
  (func (export "write") (result u32) (canon lift (core func $i "write")))
  (func (export "check-write") (result u32) (canon lift (core func $i "check-write")))
  (func (export "drop-w") (canon lift (core func $i "drop-w")))
  (func (export "run") (param "s" (stream u8)) (result u32) (canon lift (core func $i "run")))
  (func (export "run-pending") (param "s" (stream u8)) (result u32)
    (canon lift (core func $i "run-pending")))
  (func (export "run-partial") (param "s" (stream u8)) (result u32)
    (canon lift (core func $i "run-partial")))
)
"#;

async fn instantiate(
    store: &mut Store<()>,
    engine: &Engine,
    wat: &str,
) -> Result<wasmtime::component::Instance> {
    let component = Component::new(engine, wat)?;
    Linker::new(engine)
        .instantiate_async(store, &component)
        .await
}

fn engine() -> Result<Engine> {
    let mut config = Config::new();
    config.wasm_component_model_async(true);
    config.wasm_component_model_async_stackful(true);
    config.wasm_component_model_more_async_builtins(true);
    config.wasm_component_model_forward(true);
    Engine::new(&config)
}

/// Host producer, forwarded by the guest into a stream the guest then reads.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn forward_host_producer_to_guest() -> Result<()> {
    let engine = engine()?;
    let mut store = Store::new(&engine, ());
    let instance = instantiate(&mut store, &engine, FORWARDER).await?;
    let run = instance.get_typed_func::<(StreamReader<u8>,), (u32,)>(&mut store, "run")?;

    let reader = StreamReader::new(&mut store, bytes::Bytes::from_static(b"abc"))?;
    assert_eq!(
        run.call_async(&mut store, (reader,)).await?,
        (DROPPED_THREE_ITEMS,)
    );

    Ok(())
}

/// Host producer, forwarded by the guest into a stream the guest was already
/// blocked reading from.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn forward_host_producer_to_pending_guest_read() -> Result<()> {
    let engine = engine()?;
    let mut store = Store::new(&engine, ());
    let instance = instantiate(&mut store, &engine, FORWARDER).await?;
    let run = instance.get_typed_func::<(StreamReader<u8>,), (u32,)>(&mut store, "run-pending")?;

    let reader = StreamReader::new(&mut store, bytes::Bytes::from_static(b"abc"))?;
    assert_eq!(
        run.call_async(&mut store, (reader,)).await?,
        (DROPPED_THREE_ITEMS,)
    );

    Ok(())
}

/// A `StreamProducer` which writes via `Destination::as_direct` and has
/// already ended, flagging its own destruction.
struct EndedDirectProducer {
    dropped: Arc<AtomicBool>,
}

impl Drop for EndedDirectProducer {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::SeqCst);
    }
}

impl StreamProducer<()> for EndedDirectProducer {
    type Item = u8;
    type Buffer = bytes::BytesMut;

    fn poll_produce<'a>(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        store: StoreContextMut<'a, ()>,
        dst: Destination<'a, Self::Item, Self::Buffer>,
        _finish: bool,
    ) -> Poll<Result<StreamResult>> {
        let mut dst = dst.as_direct(store, 8);
        assert!(!dst.remaining().is_empty());
        Poll::Ready(Ok(StreamResult::Dropped))
    }
}

/// Host producer which has already ended, forwarded by the guest into a
/// stream with a partially-copied pending read: the read's progress is
/// delivered without polling the producer, which only reports its end of
/// the stream to a later read.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn forward_ended_host_producer_to_partial_guest_read() -> Result<()> {
    let engine = engine()?;
    let mut store = Store::new(&engine, ());
    let instance = instantiate(&mut store, &engine, FORWARDER).await?;
    let run = instance.get_typed_func::<(StreamReader<u8>,), (u32,)>(&mut store, "run-partial")?;

    let dropped = Arc::new(AtomicBool::new(false));
    let reader = StreamReader::new(
        &mut store,
        EndedDirectProducer {
            dropped: dropped.clone(),
        },
    )?;
    assert_eq!(
        run.call_async(&mut store, (reader,)).await?,
        (CANCELLED_THREE_ITEMS,)
    );
    assert!(!dropped.load(Ordering::SeqCst));

    Ok(())
}

/// Guest producer, forwarded by the guest into a stream consumed by the host.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn forward_guest_producer_to_host_consumer() -> Result<()> {
    let engine = engine()?;
    let mut store = Store::new(&engine, ());
    let instance = instantiate(&mut store, &engine, FORWARDER).await?;
    let mk = instance.get_typed_func::<(), (StreamReader<u8>,)>(&mut store, "mk")?;
    let mk_src = instance.get_typed_func::<(), ()>(&mut store, "mk-src")?;
    let fwd_src = instance.get_typed_func::<(), ()>(&mut store, "fwd-src")?;
    let write = instance.get_typed_func::<(), (u32,)>(&mut store, "write")?;
    let drop_w = instance.get_typed_func::<(), ()>(&mut store, "drop-w")?;

    let data = Arc::new(Mutex::new(Vec::new()));
    let dropped = Arc::new(AtomicBool::new(false));

    let (reader,) = mk.call_async(&mut store, ()).await?;
    reader.pipe(
        &mut store,
        CollectConsumer {
            data: data.clone(),
            dropped: dropped.clone(),
        },
    )?;

    mk_src.call_async(&mut store, ()).await?;
    fwd_src.call_async(&mut store, ()).await?;
    assert_eq!(
        write.call_async(&mut store, ()).await?,
        (COMPLETED_THREE_ITEMS,)
    );

    assert_eq!(*data.lock().unwrap(), b"xyz");

    assert!(!dropped.load(Ordering::SeqCst));
    drop_w.call_async(&mut store, ()).await?;
    store
        .run_concurrent(async |_| {
            while !dropped.load(Ordering::SeqCst) {
                tokio::task::yield_now().await;
            }
        })
        .await?;

    Ok(())
}

/// Guest producer with a write already pending when the guest forwards into a
/// stream consumed by the host.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn forward_pending_guest_write_to_host_consumer() -> Result<()> {
    let engine = engine()?;
    let mut store = Store::new(&engine, ());
    let instance = instantiate(&mut store, &engine, FORWARDER).await?;
    let mk = instance.get_typed_func::<(), (StreamReader<u8>,)>(&mut store, "mk")?;
    let mk_src = instance.get_typed_func::<(), ()>(&mut store, "mk-src")?;
    let fwd_src = instance.get_typed_func::<(), ()>(&mut store, "fwd-src")?;
    let write = instance.get_typed_func::<(), (u32,)>(&mut store, "write")?;
    let check_write = instance.get_typed_func::<(), (u32,)>(&mut store, "check-write")?;
    let drop_w = instance.get_typed_func::<(), ()>(&mut store, "drop-w")?;

    let data = Arc::new(Mutex::new(Vec::new()));
    let dropped = Arc::new(AtomicBool::new(false));

    let (reader,) = mk.call_async(&mut store, ()).await?;
    reader.pipe(
        &mut store,
        CollectConsumer {
            data: data.clone(),
            dropped: dropped.clone(),
        },
    )?;

    mk_src.call_async(&mut store, ()).await?;
    assert_eq!(write.call_async(&mut store, ()).await?, (BLOCKED,));
    fwd_src.call_async(&mut store, ()).await?;

    assert_eq!(*data.lock().unwrap(), b"xyz");
    assert_eq!(
        check_write.call_async(&mut store, ()).await?,
        (CANCELLED_THREE_ITEMS,)
    );

    assert!(!dropped.load(Ordering::SeqCst));
    drop_w.call_async(&mut store, ()).await?;
    store
        .run_concurrent(async |_| {
            while !dropped.load(Ordering::SeqCst) {
                tokio::task::yield_now().await;
            }
        })
        .await?;

    Ok(())
}

/// Host producer forwarded by the guest into a stream already being consumed
/// by the host.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn forward_host_producer_to_host_consumer() -> Result<()> {
    let engine = engine()?;
    let mut store = Store::new(&engine, ());
    let instance = instantiate(&mut store, &engine, FORWARDER).await?;
    let mk = instance.get_typed_func::<(), (StreamReader<u8>,)>(&mut store, "mk")?;
    let fwd = instance.get_typed_func::<(StreamReader<u8>,), ()>(&mut store, "fwd")?;

    let data = Arc::new(Mutex::new(Vec::new()));
    let dropped = Arc::new(AtomicBool::new(false));

    let (reader,) = mk.call_async(&mut store, ()).await?;
    reader.pipe(
        &mut store,
        CollectConsumer {
            data: data.clone(),
            dropped: dropped.clone(),
        },
    )?;

    let producer = StreamReader::new(&mut store, bytes::Bytes::from_static(b"abc"))?;
    fwd.call_async(&mut store, (producer,)).await?;

    store
        .run_concurrent(async |_| {
            while !dropped.load(Ordering::SeqCst) {
                tokio::task::yield_now().await;
            }
        })
        .await?;

    assert_eq!(*data.lock().unwrap(), b"abc");

    Ok(())
}

/// Guest producer whose writable end was dropped before the guest forwards
/// into a stream consumed by the host: the consumer must observe the end of
/// the stream with no items delivered.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn forward_dropped_guest_writer_to_host_consumer() -> Result<()> {
    let engine = engine()?;
    let mut store = Store::new(&engine, ());
    let instance = instantiate(&mut store, &engine, FORWARDER).await?;
    let mk = instance.get_typed_func::<(), (StreamReader<u8>,)>(&mut store, "mk")?;
    let mk_src = instance.get_typed_func::<(), ()>(&mut store, "mk-src")?;
    let fwd_src = instance.get_typed_func::<(), ()>(&mut store, "fwd-src")?;
    let drop_w = instance.get_typed_func::<(), ()>(&mut store, "drop-w")?;

    let data = Arc::new(Mutex::new(Vec::new()));
    let dropped = Arc::new(AtomicBool::new(false));

    let (reader,) = mk.call_async(&mut store, ()).await?;
    reader.pipe(
        &mut store,
        CollectConsumer {
            data: data.clone(),
            dropped: dropped.clone(),
        },
    )?;

    mk_src.call_async(&mut store, ()).await?;
    drop_w.call_async(&mut store, ()).await?;
    assert!(!dropped.load(Ordering::SeqCst));
    fwd_src.call_async(&mut store, ()).await?;

    assert!(dropped.load(Ordering::SeqCst));
    assert!(data.lock().unwrap().is_empty());

    Ok(())
}

/// A `FutureConsumer` which stores the value it receives and flags its own
/// destruction (which is how the host is notified when the writer end is
/// dropped without producing a value).
struct TakeConsumer {
    value: Arc<Mutex<Option<u8>>>,
    dropped: Arc<AtomicBool>,
}

impl Drop for TakeConsumer {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::SeqCst);
    }
}

impl FutureConsumer<()> for TakeConsumer {
    type Item = u8;

    fn poll_consume(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        store: StoreContextMut<()>,
        mut source: Source<Self::Item>,
        _finish: bool,
    ) -> Poll<Result<()>> {
        let mut buffer = Vec::with_capacity(1);
        source.read(store, &mut buffer)?;
        *self.value.lock().unwrap() = Some(buffer[0]);
        Poll::Ready(Ok(()))
    }
}

/// A guest component which can create futures, forward into them, and write
/// to the source of a forward.
///
/// The host writes 42; the guest writes 0xab from address 16.
const FUTURE_FORWARDER: &str = r#"
(component
  (core module $libc (memory (export "m") 1))
  (core instance $libc (instantiate $libc))

  (type $f (future u8))
  (core func $future.new (canon future.new $f))
  (core func $future.write (canon future.write $f async (memory (core memory $libc "m"))))
  (core func $future.forward (canon future.forward $f))
  (core func $future.cancel-write (canon future.cancel-write $f async))

  (core module $m
    (import "" "m" (memory 1))
    (import "" "future.new" (func $future.new (result i64)))
    (import "" "future.write" (func $future.write (param i32 i32) (result i32)))
    (import "" "future.forward" (func $future.forward (param i32 i32)))
    (import "" "future.cancel-write" (func $future.cancel-write (param i32) (result i32)))

    ;; The writable end of the future returned by `mk`.
    (global $dst-w (mut i32) (i32.const 0))
    ;; The ends of the future created by `mk-src`.
    (global $src-r (mut i32) (i32.const 0))
    (global $src-w (mut i32) (i32.const 0))

    ;; Create a new future, saving its writable end and returning its
    ;; readable end (which the host will consume).
    (func (export "mk") (result i32)
      (local $tmp i64)
      (local.set $tmp (call $future.new))
      (global.set $dst-w (i32.wrap_i64 (i64.shr_u (local.get $tmp) (i64.const 32))))
      (i32.wrap_i64 (local.get $tmp))
    )

    ;; Create a new future, saving both ends (which this component will
    ;; produce into).
    (func (export "mk-src")
      (local $tmp i64)
      (local.set $tmp (call $future.new))
      (global.set $src-r (i32.wrap_i64 (local.get $tmp)))
      (global.set $src-w (i32.wrap_i64 (i64.shr_u (local.get $tmp) (i64.const 32))))
    )

    ;; Forward the future created by `mk-src` into the future created by `mk`.
    (func (export "fwd-src")
      (call $future.forward (global.get $src-r) (global.get $dst-w))
    )

    ;; Forward the given future into the future created by `mk`.
    (func (export "fwd") (param $r i32)
      (call $future.forward (local.get $r) (global.get $dst-w))
    )

    ;; Write 0xab to the future created by `mk-src`, returning the result
    ;; code.
    (func (export "write") (result i32)
      (i32.store8 (i32.const 16) (i32.const 0xab))
      (call $future.write (global.get $src-w) (i32.const 16))
    )

    ;; Retrieve the completion of a pending write on the future created by
    ;; `mk-src` by cancelling it, returning the result code.
    (func (export "check-write") (result i32)
      (call $future.cancel-write (global.get $src-w))
    )
  )

  (core instance $i (instantiate $m
    (with "" (instance
      (export "m" (memory $libc "m"))
      (export "future.new" (func $future.new))
      (export "future.write" (func $future.write))
      (export "future.forward" (func $future.forward))
      (export "future.cancel-write" (func $future.cancel-write))
    ))
  ))

  (func (export "mk") (result (future u8)) (canon lift (core func $i "mk")))
  (func (export "mk-src") (canon lift (core func $i "mk-src")))
  (func (export "fwd-src") (canon lift (core func $i "fwd-src")))
  (func (export "fwd") (param "f" (future u8)) (canon lift (core func $i "fwd")))
  (func (export "write") (result u32) (canon lift (core func $i "write")))
  (func (export "check-write") (result u32) (canon lift (core func $i "check-write")))
)
"#;

/// Guest producer with a write already pending when the guest forwards into a
/// future consumed by the host.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn forward_pending_guest_future_write_to_host_consumer() -> Result<()> {
    let engine = engine()?;
    let mut store = Store::new(&engine, ());
    let instance = instantiate(&mut store, &engine, FUTURE_FORWARDER).await?;
    let mk = instance.get_typed_func::<(), (FutureReader<u8>,)>(&mut store, "mk")?;
    let mk_src = instance.get_typed_func::<(), ()>(&mut store, "mk-src")?;
    let fwd_src = instance.get_typed_func::<(), ()>(&mut store, "fwd-src")?;
    let write = instance.get_typed_func::<(), (u32,)>(&mut store, "write")?;
    let check_write = instance.get_typed_func::<(), (u32,)>(&mut store, "check-write")?;

    let value = Arc::new(Mutex::new(None));
    let dropped = Arc::new(AtomicBool::new(false));

    let (reader,) = mk.call_async(&mut store, ()).await?;
    reader.pipe(
        &mut store,
        TakeConsumer {
            value: value.clone(),
            dropped: dropped.clone(),
        },
    )?;

    mk_src.call_async(&mut store, ()).await?;
    assert_eq!(write.call_async(&mut store, ()).await?, (BLOCKED,));
    fwd_src.call_async(&mut store, ()).await?;

    assert_eq!(*value.lock().unwrap(), Some(0xab));
    assert_eq!(check_write.call_async(&mut store, ()).await?, (COMPLETED,));

    Ok(())
}

/// Host producer forwarded by the guest into a future already being consumed
/// by the host.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn forward_host_future_producer_to_host_consumer() -> Result<()> {
    let engine = engine()?;
    let mut store = Store::new(&engine, ());
    let instance = instantiate(&mut store, &engine, FUTURE_FORWARDER).await?;
    let mk = instance.get_typed_func::<(), (FutureReader<u8>,)>(&mut store, "mk")?;
    let fwd = instance.get_typed_func::<(FutureReader<u8>,), ()>(&mut store, "fwd")?;

    let value = Arc::new(Mutex::new(None));
    let dropped = Arc::new(AtomicBool::new(false));

    let (reader,) = mk.call_async(&mut store, ()).await?;
    reader.pipe(
        &mut store,
        TakeConsumer {
            value: value.clone(),
            dropped: dropped.clone(),
        },
    )?;

    let producer = FutureReader::new(&mut store, async { Ok::<_, wasmtime::Error>(42u8) })?;
    fwd.call_async(&mut store, (producer,)).await?;

    store
        .run_concurrent(async |_| {
            while !dropped.load(Ordering::SeqCst) {
                tokio::task::yield_now().await;
            }
        })
        .await?;

    assert_eq!(*value.lock().unwrap(), Some(42));

    Ok(())
}

/// Host reader closed without consuming after the guest forwards into it: the
/// drop must propagate back through the forward to the guest's write.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn forward_guest_future_write_to_dropped_host_reader() -> Result<()> {
    let engine = engine()?;
    let mut store = Store::new(&engine, ());
    let instance = instantiate(&mut store, &engine, FUTURE_FORWARDER).await?;
    let mk = instance.get_typed_func::<(), (FutureReader<u8>,)>(&mut store, "mk")?;
    let mk_src = instance.get_typed_func::<(), ()>(&mut store, "mk-src")?;
    let fwd_src = instance.get_typed_func::<(), ()>(&mut store, "fwd-src")?;
    let write = instance.get_typed_func::<(), (u32,)>(&mut store, "write")?;

    let (mut reader,) = mk.call_async(&mut store, ()).await?;
    mk_src.call_async(&mut store, ()).await?;
    fwd_src.call_async(&mut store, ()).await?;
    reader.close(&mut store)?;

    assert_eq!(write.call_async(&mut store, ()).await?, (DROPPED,));

    Ok(())
}
