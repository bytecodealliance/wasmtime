pub async fn sleep(duration: std::time::Duration) {
    if cfg!(miri) {
        // TODO: We should be able to use `tokio::time::sleep` here, but as of
        // this writing the miri-compatible version of `wasmtime-fiber` uses
        // threads behind the scenes, which means thread-local storage is not
        // preserved when we switch fibers, and that confuses Tokio.  If we ever
        // fix that we can stop using our own, special version of `sleep` and
        // switch back to the Tokio version.

        use std::{
            future,
            sync::{
                Arc, Mutex,
                atomic::{AtomicU32, Ordering::SeqCst},
            },
            task::Poll,
            thread,
        };

        let state = Arc::new(AtomicU32::new(0));
        let waker = Arc::new(Mutex::new(None));
        let mut join_handle = None;
        future::poll_fn(move |cx| match state.load(SeqCst) {
            0 => {
                state.store(1, SeqCst);
                let state = state.clone();
                *waker.lock().unwrap() = Some(cx.waker().clone());
                let waker = waker.clone();
                join_handle = Some(thread::spawn(move || {
                    thread::sleep(duration);
                    state.store(2, SeqCst);
                    let waker = waker.lock().unwrap().clone().unwrap();
                    waker.wake();
                }));
                Poll::Pending
            }
            1 => {
                *waker.lock().unwrap() = Some(cx.waker().clone());
                Poll::Pending
            }
            2 => {
                if let Some(handle) = join_handle.take() {
                    _ = handle.join();
                }
                Poll::Ready(())
            }
            _ => unreachable!(),
        })
        .await;
    } else {
        tokio::time::sleep(duration).await;
    }
}
