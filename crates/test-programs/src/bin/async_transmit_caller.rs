mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "transmit-caller",
    });

    use super::Component;
    export!(Component);
}

use {
    bindings::{
        exports::local::local::run::Guest,
        local::local::transmit::{self, Control},
        wit_future, wit_stream,
    },
    futures::{FutureExt, StreamExt, future, stream::FuturesUnordered},
    std::{
        future::{Future, IntoFuture},
        pin::{Pin, pin},
        task::Poll,
    },
    wit_bindgen_rt::async_support::{FutureWriteCancel, StreamResult},
};

struct Component;

impl Guest for Component {
    async fn run() {
        let (mut control_tx, control_rx) = wit_stream::new();
        let (mut caller_stream_tx, caller_stream_rx) = wit_stream::new();
        let (mut caller_future_tx1, caller_future_rx1) = wit_future::new(|| todo!());
        let (caller_future_tx2, caller_future_rx2) = wit_future::new(|| String::new());

        let (mut callee_stream_rx, mut callee_future_rx1, callee_future_rx2) = transmit::exchange(
            control_rx,
            caller_stream_rx,
            caller_future_rx1,
            caller_future_rx2,
        )
        .await;

        // Tell peer to read from its end of the stream and assert that the result matches an expected value.
        assert!(
            control_tx
                .write_one(Control::ReadStream("a".into()))
                .await
                .is_none()
        );
        assert!(caller_stream_tx.write_one("a".into()).await.is_none());

        // Start writing another value, but cancel the write before telling the peer to read.
        {
            let send = Box::pin(caller_stream_tx.write_one("b".into()));
            assert!(poll(send).await.is_err());
        }

        // Tell the peer to read an expected value again, which should _not_ match the value provided in the
        // canceled write above.
        assert!(
            control_tx
                .write_one(Control::ReadStream("c".into()))
                .await
                .is_none()
        );
        assert!(caller_stream_tx.write_one("c".into()).await.is_none());

        // Tell the peer to do a zero-length read, do a zero-length write; assert the latter completes, then do a
        // non-zero-length write, assert that it does _not_ complete, then tell the peer to do a non-zero-length
        // read and assert that the write completes.
        assert!(
            control_tx
                .write_one(Control::ReadStreamZero)
                .await
                .is_none()
        );
        {
            assert_eq!(
                caller_stream_tx.write(Vec::new()).await.0,
                StreamResult::Complete(0)
            );

            let send = Box::pin(caller_stream_tx.write_one("d".into()));
            let Err(send) = poll(send).await else {
                panic!()
            };

            let mut futures = FuturesUnordered::new();
            futures.push(Box::pin(send.map(|v| {
                assert!(v.is_none());
            })) as Pin<Box<dyn Future<Output = _>>>);
            futures.push(Box::pin(
                control_tx
                    .write_one(Control::ReadStream("d".into()))
                    .map(|v| {
                        assert!(v.is_none());
                    }),
            ));
            while let Some(()) = futures.next().await {}
        }

        // Start writing a value to the future, but cancel the write before telling the peer to read.
        {
            let send = Box::pin(caller_future_tx1.write("x".into()));
            match poll(send).await {
                Ok(_) => panic!(),
                Err(mut send) => {
                    caller_future_tx1 = match send.as_mut().cancel() {
                        FutureWriteCancel::AlreadySent => unreachable!(),
                        FutureWriteCancel::Dropped(_) => unreachable!(),
                        FutureWriteCancel::Cancelled(_, writer) => writer,
                    }
                }
            }
        }

        // Tell the peer to read an expected value again, which should _not_ match the value provided in the
        // canceled write above.
        assert!(
            control_tx
                .write_one(Control::ReadFuture("y".into()))
                .await
                .is_none()
        );
        caller_future_tx1.write("y".into()).await.unwrap();

        // Tell the peer to write a value to its end of the stream, then read from our end and assert the value
        // matches.
        assert!(
            control_tx
                .write_one(Control::WriteStream("a".into()))
                .await
                .is_none()
        );
        assert_eq!(callee_stream_rx.next().await, Some("a".into()));

        // Start reading a value from the stream, but cancel the read before telling the peer to write.
        {
            let next = Box::pin(callee_stream_rx.read(Vec::with_capacity(1)));
            assert!(poll(next).await.is_err());
        }

        // Once again, tell the peer to write a value to its end of the stream, then read from our end and assert
        // the value matches.
        assert!(
            control_tx
                .write_one(Control::WriteStream("b".into()))
                .await
                .is_none()
        );
        assert_eq!(callee_stream_rx.next().await, Some("b".into()));

        // Tell the peer to do a zero-length write, assert that the read does _not_ complete, then tell the peer to
        // do a non-zero-length write and assert that the read completes.
        assert!(
            control_tx
                .write_one(Control::WriteStreamZero)
                .await
                .is_none()
        );
        {
            let next = Box::pin(callee_stream_rx.next());
            let Err(next) = poll(next).await else {
                panic!()
            };

            let mut futures = FuturesUnordered::new();
            futures.push(Box::pin(next.map(|v| {
                assert_eq!(v, Some("c".into()));
            })) as Pin<Box<dyn Future<Output = _>>>);
            futures.push(Box::pin(
                control_tx
                    .write_one(Control::WriteStream("c".into()))
                    .map(|v| {
                        assert!(v.is_none());
                    }),
            ));
            while let Some(()) = futures.next().await {}
        }

        // Start reading a value from the future, but cancel the read before telling the peer to write.
        {
            let next = Box::pin(callee_future_rx1.into_future());
            match poll(next).await {
                Ok(_) => panic!(),
                Err(mut next) => callee_future_rx1 = next.as_mut().cancel().unwrap_err(),
            }
        }

        // Tell the peer to write a value to its end of the future, then read from our end and assert the value
        // matches.
        assert!(
            control_tx
                .write_one(Control::WriteFuture("b".into()))
                .await
                .is_none()
        );
        assert_eq!(callee_future_rx1.into_future().await, "b");

        // Start writing a value to the stream, but drop the stream without telling the peer to read.
        let send = Box::pin(caller_stream_tx.write_one("d".into()));
        assert!(poll(send).await.is_err());
        drop(caller_stream_tx);

        // Start reading a value from the stream, but drop the stream without telling the peer to write.
        let next = Box::pin(callee_stream_rx.next());
        assert!(poll(next).await.is_err());
        drop(callee_stream_rx);

        // Start writing a value to the future, but drop the write without telling the peer to read.
        {
            let send = pin!(caller_future_tx2.write("x".into()));
            assert!(poll(send).await.is_err());
        }

        // Start reading a value from the future, but drop the read without telling the peer to write.
        {
            let next = Box::pin(callee_future_rx2.into_future());
            assert!(poll(next).await.is_err());
        }
    }
}

async fn poll<T, F: Future<Output = T> + Unpin>(fut: F) -> Result<T, F> {
    let mut fut = Some(fut);
    future::poll_fn(move |cx| {
        let mut fut = fut.take().unwrap();
        Poll::Ready(match fut.poll_unpin(cx) {
            Poll::Ready(v) => Ok(v),
            Poll::Pending => Err(fut),
        })
    })
    .await
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
