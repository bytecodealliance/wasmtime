mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "transmit-caller",
        async: {
            imports: [
                "local:local/transmit#exchange",
            ],
            exports: [
                "local:local/run#run",
            ],
        }
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
    futures::{future, FutureExt, SinkExt, StreamExt},
    std::{
        future::{Future, IntoFuture},
        pin::pin,
        task::Poll,
    },
};

struct Component;

impl Guest for Component {
    async fn run() {
        let (mut control_tx, control_rx) = wit_stream::new();
        let (mut caller_stream_tx, caller_stream_rx) = wit_stream::new();
        let (mut caller_future_tx1, caller_future_rx1) = wit_future::new();
        let (caller_future_tx2, caller_future_rx2) = wit_future::new();

        let (mut callee_stream_rx, mut callee_future_rx1, callee_future_rx2) = transmit::exchange(
            control_rx,
            caller_stream_rx,
            caller_future_rx1,
            caller_future_rx2,
        )
        .await;

        // Tell peer to read from its end of the stream and assert that the result matches an expected value.
        control_tx
            .send(vec![Control::ReadStream("a".into())])
            .await
            .unwrap();
        caller_stream_tx.send(vec!["a".into()]).await.unwrap();

        // Start writing another value, but cancel the write before telling the peer to read.
        {
            let send = caller_stream_tx.send(vec!["b".into()]);
            assert!(poll(send).await.is_err());
            caller_stream_tx.cancel();
        }

        // Tell the peer to read an expected value again, which should _not_ match the value provided in the
        // canceled write above.
        control_tx
            .send(vec![Control::ReadStream("c".into())])
            .await
            .unwrap();
        caller_stream_tx.send(vec!["c".into()]).await.unwrap();

        // Start writing a value to the future, but cancel the write before telling the peer to read.
        {
            let send = caller_future_tx1.write("x".into());
            match poll(send).await {
                Ok(_) => panic!(),
                Err(send) => caller_future_tx1 = send.cancel(),
            }
        }

        // Tell the peer to read an expected value again, which should _not_ match the value provided in the
        // canceled write above.
        control_tx
            .send(vec![Control::ReadFuture("y".into())])
            .await
            .unwrap();
        caller_future_tx1.write("y".into()).await;

        // Tell the peer to write a value to its end of the stream, then read from our end and assert the value
        // matches.
        control_tx
            .send(vec![Control::WriteStream("a".into())])
            .await
            .unwrap();
        assert_eq!(callee_stream_rx.next().await, Some(vec!["a".into()]));

        // Start reading a value from the stream, but cancel the read before telling the peer to write.
        {
            let next = callee_stream_rx.next();
            assert!(poll(next).await.is_err());
            callee_stream_rx.cancel();
        }

        // Once again, tell the peer to write a value to its end of the stream, then read from our end and assert
        // the value matches.
        control_tx
            .send(vec![Control::WriteStream("b".into())])
            .await
            .unwrap();
        assert_eq!(callee_stream_rx.next().await, Some(vec!["b".into()]));

        // Start reading a value from the future, but cancel the read before telling the peer to write.
        {
            let next = callee_future_rx1.into_future();
            match poll(next).await {
                Ok(_) => panic!(),
                Err(next) => callee_future_rx1 = next.cancel(),
            }
        }

        // Tell the peer to write a value to its end of the future, then read from our end and assert the value
        // matches.
        control_tx
            .send(vec![Control::WriteFuture("b".into())])
            .await
            .unwrap();
        assert_eq!(callee_future_rx1.into_future().await, Some("b".into()));

        // Start writing a value to the stream, but drop the stream without telling the peer to read.
        let send = caller_stream_tx.send(vec!["d".into()]);
        assert!(poll(send).await.is_err());
        drop(caller_stream_tx);

        // Start reading a value from the stream, but drop the stream without telling the peer to write.
        let next = callee_stream_rx.next();
        assert!(poll(next).await.is_err());
        drop(callee_stream_rx);

        // Start writing a value to the future, but drop the write without telling the peer to read.
        {
            let send = pin!(caller_future_tx2.write("x".into()));
            assert!(poll(send).await.is_err());
        }

        // Start reading a value from the future, but drop the read without telling the peer to write.
        {
            let next = callee_future_rx2.into_future();
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
