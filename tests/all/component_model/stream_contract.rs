use std::mem;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::oneshot;
use wasmtime::component::{
    Component, Destination, Instance, Linker, Source, StreamConsumer, StreamProducer, StreamReader,
    StreamResult, VecBuffer,
};
use wasmtime::{Config, Engine, Result, Store, StoreContextMut};

const COMPONENT: &str = r#"
(component
  (type $s (stream u8))
  (core module $libc (memory (export "mem") 1))
  (core instance $libc (instantiate $libc))
  (core func $stream.new (canon stream.new $s))
  (core func $stream.read
    (canon stream.read $s async (memory (core memory $libc "mem"))))
  (core func $stream.write
    (canon stream.write $s async (memory (core memory $libc "mem"))))
  (core func $stream.cancel-read (canon stream.cancel-read $s))
  (core func $task.return (canon task.return (result u32)))
  (core module $m
    (import "" "mem" (memory 1))
    (import "" "stream.new" (func $stream.new (result i64)))
    (import "" "stream.read"
      (func $stream.read (param i32 i32 i32) (result i32)))
    (import "" "stream.write"
      (func $stream.write (param i32 i32 i32) (result i32)))
    (import "" "stream.cancel-read"
      (func $stream.cancel-read (param i32) (result i32)))
    (import "" "task.return" (func $task.return (param i32)))

    (global $w (mut i32) (i32.const 0))
    (data (i32.const 100) "\01\02\03\04")

    ;; Reads up to `n` items from `s`, returning the result of `stream.read`.
    (func (export "read") (param $s i32) (param $n i32) (result i32)
      (call $stream.read (local.get $s) (i32.const 200) (local.get $n)))

    ;; Starts reading up to `n` items from `s` and then cancels that,
    ;; returning the result of `stream.cancel-read`.
    (func (export "read-then-cancel") (param $s i32) (param $n i32) (result i32)
      (drop (call $stream.read (local.get $s) (i32.const 200) (local.get $n)))
      (call $task.return (call $stream.cancel-read (local.get $s)))
      (i32.const 0 (; EXIT ;)))
    (func (export "cb") (param i32 i32 i32) (result i32) unreachable)

    ;; Creates a stream, returning its readable end and keeping its writable
    ;; end for `write`.
    (func (export "new") (result i32)
      (local $rw i64)
      (local.set $rw (call $stream.new))
      (global.set $w (i32.wrap_i64 (i64.shr_u (local.get $rw) (i64.const 32))))
      (i32.wrap_i64 (local.get $rw)))

    ;; Writes `n` items, returning the result of `stream.write`.
    (func (export "write") (param $n i32) (result i32)
      (call $stream.write (global.get $w) (i32.const 100) (local.get $n))))
  (core instance $i (instantiate $m (with "" (instance
    (export "mem" (memory $libc "mem"))
    (export "stream.new" (func $stream.new))
    (export "stream.read" (func $stream.read))
    (export "stream.write" (func $stream.write))
    (export "stream.cancel-read" (func $stream.cancel-read))
    (export "task.return" (func $task.return))))))
  (func (export "read") (param "s" $s) (param "n" u32) (result u32)
    (canon lift (core func $i "read")))
  (func (export "read-then-cancel") async (param "s" $s) (param "n" u32)
    (result u32)
    (canon lift (core func $i "read-then-cancel") async
      (callback (core func $i "cb"))))
  (func (export "new") (result $s) (canon lift (core func $i "new")))
  (func (export "write") (param "n" u32) (result u32)
    (canon lift (core func $i "write")))
)
"#;

async fn instantiate() -> Result<(Store<()>, Instance)> {
    let mut config = Config::new();
    config.wasm_component_model_async(true);
    let engine = Engine::new(&config)?;
    let component = Component::new(&engine, COMPONENT)?;
    let mut store = Store::new(&engine, ());
    let instance = Linker::new(&engine)
        .instantiate_async(&mut store, &component)
        .await?;
    Ok((store, instance))
}

#[derive(Copy, Clone)]
enum Produce {
    /// Return `Completed` without producing anything (and eventually
    /// `Dropped`, so a host-to-host pipe can't spin forever).
    Nothing,
    /// Buffer `[1, 2, 3]` and return `Completed`, then `Dropped`.
    Items,
    /// Write `[1, 2, 3]` via `Destination::as_direct` and return `Completed`,
    /// then `Dropped`.
    DirectItems,
    /// Return `Pending` until `finish` is set, then `Cancelled`.
    Cancelled,
    /// Return `Pending` until `finish` is set, then buffer `[1, 2, 3]` and
    /// return `Cancelled`.
    CancelledWithItems,
}

struct TestProducer(Produce, usize);

impl StreamProducer<()> for TestProducer {
    type Item = u8;
    type Buffer = VecBuffer<u8>;

    fn poll_produce<'a>(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        store: StoreContextMut<'a, ()>,
        mut dst: Destination<'a, Self::Item, Self::Buffer>,
        finish: bool,
    ) -> Poll<Result<StreamResult>> {
        self.1 += 1;
        Poll::Ready(Ok(match self.0 {
            Produce::Nothing if self.1 < 100 => StreamResult::Completed,
            Produce::Items | Produce::DirectItems if self.1 == 1 => {
                if let Produce::Items = self.0 {
                    dst.set_buffer(vec![1, 2, 3].into());
                } else {
                    let mut dst = dst.as_direct(store, 3);
                    dst.remaining().copy_from_slice(&[1, 2, 3]);
                    dst.mark_written(3);
                }
                StreamResult::Completed
            }
            Produce::Nothing | Produce::Items | Produce::DirectItems => StreamResult::Dropped,
            Produce::Cancelled | Produce::CancelledWithItems if !finish => return Poll::Pending,
            Produce::Cancelled => StreamResult::Cancelled,
            Produce::CancelledWithItems => {
                dst.set_buffer(vec![1, 2, 3].into());
                StreamResult::Cancelled
            }
        }))
    }
}

/// A consumer which takes everything it's given if `take` is set, or
/// otherwise nothing, and which sends what it took over `done` when dropped.
struct TestConsumer {
    take: bool,
    taken: Vec<u8>,
    done: Option<oneshot::Sender<Vec<u8>>>,
}

impl TestConsumer {
    fn new(take: bool) -> (Self, oneshot::Receiver<Vec<u8>>) {
        let (tx, rx) = oneshot::channel();
        let consumer = TestConsumer {
            take,
            taken: Vec::new(),
            done: Some(tx),
        };
        (consumer, rx)
    }
}

impl Drop for TestConsumer {
    fn drop(&mut self) {
        let _ = self.done.take().unwrap().send(mem::take(&mut self.taken));
    }
}

impl StreamConsumer<()> for TestConsumer {
    type Item = u8;

    fn poll_consume(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        mut store: StoreContextMut<()>,
        mut src: Source<'_, Self::Item>,
        _finish: bool,
    ) -> Poll<Result<StreamResult>> {
        if self.take {
            let mut buf = Vec::with_capacity(src.remaining(&mut store));
            src.read(&mut store, &mut buf)?;
            self.taken.extend(buf);
        }
        Poll::Ready(Ok(StreamResult::Completed))
    }
}

/// Calls the guest's `export`, either `read` or `read-then-cancel`, with a
/// stream produced by `producer`.
async fn guest_read(export: &str, producer: Produce, n: u32) -> Result<u32> {
    let (mut store, instance) = instantiate().await?;
    let func = instance.get_typed_func::<(StreamReader<u8>, u32), (u32,)>(&mut store, export)?;
    let reader = StreamReader::new(&mut store, TestProducer(producer, 0))?;
    Ok(func.call_async(&mut store, (reader, n)).await?.0)
}

/// Has the guest write `n` items to a stream consumed by a `TestConsumer`.
async fn guest_write(take: bool, n: u32) -> Result<u32> {
    let (mut store, instance) = instantiate().await?;
    let new = instance.get_typed_func::<(), (StreamReader<u8>,)>(&mut store, "new")?;
    let write = instance.get_typed_func::<(u32,), (u32,)>(&mut store, "write")?;
    let (reader,) = new.call_async(&mut store, ()).await?;
    reader.pipe(&mut store, TestConsumer::new(take).0)?;
    Ok(write.call_async(&mut store, (n,)).await?.0)
}

/// Pipes `producer` to a `TestConsumer` on the host, returning what the
/// consumer took once it's dropped.
async fn host_pipe(producer: Produce, take: bool) -> Result<Vec<u8>> {
    let (mut store, _instance) = instantiate().await?;
    let reader = StreamReader::new(&mut store, TestProducer(producer, 0))?;
    let (consumer, done) = TestConsumer::new(take);
    reader.pipe(&mut store, consumer)?;
    Ok(store.run_concurrent(async |_| done.await).await??)
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn consumer_completed_without_consuming() -> Result<()> {
    assert!(guest_write(false, 1).await.is_err());
    assert!(host_pipe(Produce::Items, false).await.is_err());

    // A zero-length write may complete without anything being consumed.
    assert_eq!(guest_write(false, 0).await?, 0);
    assert_eq!(guest_write(true, 4).await?, 4 << 4); // COMPLETED(4)
    Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn producer_completed_without_producing() -> Result<()> {
    assert!(guest_read("read", Produce::Nothing, 1).await.is_err());
    assert!(host_pipe(Produce::Nothing, true).await.is_err());

    // A zero-length read may complete without anything being produced.
    assert_eq!(guest_read("read", Produce::Nothing, 0).await?, 0);
    assert_eq!(guest_read("read", Produce::Items, 1).await?, 1 << 4); // COMPLETED(1)
    assert_eq!(host_pipe(Produce::DirectItems, true).await?, [1, 2, 3]);
    Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn producer_cancelled_after_producing() -> Result<()> {
    let read = "read-then-cancel";
    assert!(
        guest_read(read, Produce::CancelledWithItems, 4)
            .await
            .is_err()
    );

    let result = guest_read(read, Produce::Cancelled, 4).await?;
    assert_eq!(result, 2);
    Ok(())
}
