mod bindings {
    wit_bindgen::generate!({
        path: "../misc/component-async-tests/wit",
        world: "transmit-callee",
        async: {
            exports: [
                "local:local/transmit#exchange",
            ],
        }
    });

    use super::Component;
    export!(Component);
}

use {
    bindings::{
        exports::local::local::transmit::{Control, Guest},
        wit_future, wit_stream,
    },
    futures::{SinkExt, StreamExt},
    std::future::IntoFuture,
    wit_bindgen_rt::async_support::{self, FutureReader, StreamReader},
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
        let (callee_future_tx1, callee_future_rx1) = wit_future::new();
        let (callee_future_tx2, callee_future_rx2) = wit_future::new();

        async_support::spawn(async move {
            let mut caller_future_rx1 = Some(caller_future_rx1);
            let mut callee_future_tx1 = Some(callee_future_tx1);

            while let Some(messages) = control_rx.next().await {
                for message in messages {
                    match message {
                        Control::ReadStream(value) => {
                            assert_eq!(caller_stream_rx.next().await, Some(vec![value]));
                        }
                        Control::ReadFuture(value) => {
                            assert_eq!(
                                caller_future_rx1.take().unwrap().into_future().await,
                                Some(value)
                            );
                        }
                        Control::WriteStream(value) => {
                            callee_stream_tx.send(vec![value]).await.unwrap();
                        }
                        Control::WriteFuture(value) => {
                            callee_future_tx1.take().unwrap().write(value).await;
                        }
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
