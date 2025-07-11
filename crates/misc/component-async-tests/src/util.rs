use futures::channel::oneshot;
use std::thread;

pub async fn sleep(duration: std::time::Duration) {
    if cfg!(miri) {
        // TODO: We should be able to use `tokio::time::sleep` here, but as of
        // this writing the miri-compatible version of `wasmtime-fiber` uses
        // threads behind the scenes, which means thread-local storage is not
        // preserved when we switch fibers, and that confuses Tokio.  If we ever
        // fix that we can stop using our own, special version of `sleep` and
        // switch back to the Tokio version.

        let (tx, rx) = oneshot::channel();
        let handle = thread::spawn(move || {
            thread::sleep(duration);
            _ = tx.send(());
        });
        _ = rx.await;
        _ = handle.join();
    } else {
        tokio::time::sleep(duration).await;
    }
}
