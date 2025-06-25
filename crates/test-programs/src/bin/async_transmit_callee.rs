mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "transmit-callee",
    });

    use super::Component;
    export!(Component);
}

use {
    bindings::{
        exports::local::local::transmit::{Control, Guest},
        wit_future, wit_stream,
    },
    std::future::IntoFuture,
    wit_bindgen_rt::async_support::{self, FutureReader, StreamReader, StreamResult},
};

struct Component;

impl Guest for Component {
    async fn exchange(
        mut control_rx: StreamReader<Control>,
        mut caller_stream_rx: StreamReader<String>,
        caller_future_rx1: FutureReader<String>,
        caller_future_rx2: FutureReader<String>,
    ) -> (
        StreamReader<String>,
        FutureReader<String>,
        FutureReader<String>,
    ) {
        let (mut callee_stream_tx, callee_stream_rx) = wit_stream::new();
        let (callee_future_tx1, callee_future_rx1) = wit_future::new(|| todo!());
        let (callee_future_tx2, callee_future_rx2) = wit_future::new(|| String::new());

        async_support::spawn(async move {
            let mut caller_future_rx1 = Some(caller_future_rx1);
            let mut callee_future_tx1 = Some(callee_future_tx1);

            while let Some(message) = control_rx.next().await {
                match message {
                    Control::ReadStream(value) => {
                        assert_eq!(caller_stream_rx.next().await, Some(value));
                    }
                    Control::ReadStreamZero => {
                        assert_eq!(
                            caller_stream_rx.read(Vec::new()).await.0,
                            StreamResult::Complete(0)
                        );
                    }
                    Control::ReadFuture(value) => {
                        assert_eq!(caller_future_rx1.take().unwrap().into_future().await, value);
                    }
                    Control::WriteStream(value) => {
                        assert!(callee_stream_tx.write_one(value).await.is_none());
                    }
                    Control::WriteStreamZero => {
                        assert_eq!(
                            callee_stream_tx.write(Vec::new()).await.0,
                            StreamResult::Complete(0)
                        );
                    }
                    Control::WriteFuture(value) => {
                        callee_future_tx1
                            .take()
                            .unwrap()
                            .write(value)
                            .await
                            .unwrap();
                    }
                }
            }

            drop((caller_future_rx2, callee_future_tx2));
        });

        (callee_stream_rx, callee_future_rx1, callee_future_rx2)
    }
}

// Unused function; required since this file is built as a `bin`:
fn main() {}
