use super::*;
use std::future::Future;
use std::pin::pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Waker};
use tokio::sync::oneshot;

#[derive(Default)]
struct Counts {
    admitted: AtomicUsize,
    live: AtomicUsize,
    released: Mutex<Option<oneshot::Sender<()>>>,
}

struct Owner(Arc<Counts>);

impl Drop for Owner {
    fn drop(&mut self) {
        self.0.live.fetch_sub(1, Ordering::SeqCst);
        if let Some(tx) = self.0.released.lock().unwrap().take() {
            let _ = tx.send(());
        }
    }
}

fn tracked_file() -> (File, Arc<Counts>, oneshot::Receiver<()>) {
    let counts = Arc::new(Counts::default());
    let (tx, rx) = oneshot::channel();
    *counts.released.lock().unwrap() = Some(tx);
    let state = counts.clone();
    let mut file = File::new(
        tempfile::tempfile().unwrap(),
        FsPerms::ReadWrite,
        OpenMode::READ | OpenMode::WRITE,
        false,
    );
    file.operation_hook = Some(Arc::new(move || {
        state.admitted.fetch_add(1, Ordering::SeqCst);
        state.live.fetch_add(1, Ordering::SeqCst);
        Ok(FilesystemOperation::new(Owner(state.clone())))
    }));
    (file, counts, rx)
}

#[tokio::test]
async fn owner_survives_future_resource_and_store_destruction() {
    let (file, counts, released) = tracked_file();
    let native = Arc::downgrade(&file.file);
    let mut store = wasmtime::Store::new(&wasmtime::Engine::default(), file);
    let (started_tx, started_rx) = oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    {
        let future = store.data_mut().run_blocking(move |_| {
            started_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            Ok::<_, std::io::Error>(())
        });
        let mut future = pin!(future);
        assert!(
            future
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_pending()
        );
        started_rx.await.unwrap();
    }
    drop(store);
    let live_while_started = counts.live.load(Ordering::SeqCst);
    release_tx.send(()).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), released)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(live_while_started, 1);
    assert_eq!(counts.live.load(Ordering::SeqCst), 0);
    assert_eq!(counts.admitted.load(Ordering::SeqCst), 1);
    assert!(native.upgrade().is_none());
}

#[tokio::test]
async fn native_unwind_disposes_captured_handles_before_owner() {
    let (file, counts, released) = tracked_file();
    let second_file = Arc::new(tempfile::tempfile().unwrap());
    let second_native = Arc::downgrade(&second_file);
    let native = Arc::downgrade(&file.file);
    let (started_tx, started_rx) = oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let task = file
        .spawn_blocking(move |_| {
            let _captured_file = second_file;
            started_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            panic!("intentional native worker panic")
        })
        .unwrap();
    started_rx.await.unwrap();
    drop(task);
    drop(file);
    let live_while_started = counts.live.load(Ordering::SeqCst);
    release_tx.send(()).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), released)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(live_while_started, 1);
    assert!(native.upgrade().is_none());
    assert!(second_native.upgrade().is_none());
    assert_eq!(counts.live.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn abandoned_native_result_disposes_file_before_owner() {
    let (file, counts, released) = tracked_file();
    let returned = Arc::new(tempfile::tempfile().unwrap());
    let native = Arc::downgrade(&returned);
    let operation = file.start_operation().unwrap();
    let result_operation = operation.clone();
    let (started_tx, started_rx) = oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let task = file.spawn_blocking_with(operation, move |_| {
        started_tx.send(()).unwrap();
        release_rx.recv().unwrap();
        NativeOperation::new(returned, result_operation)
    });
    started_rx.await.unwrap();
    drop(file);
    drop(task);
    let live_while_started = counts.live.load(Ordering::SeqCst);
    release_tx.send(()).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), released)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(live_while_started, 1);
    assert!(native.upgrade().is_none());
    assert_eq!(counts.admitted.load(Ordering::SeqCst), 1);
}

#[cfg(feature = "p2")]
#[tokio::test]
async fn completed_unpolled_p2_read_and_write_release_ownership() {
    use crate::p2::filesystem::{FileInputStream, FileOutputStream};
    use wasmtime_wasi_io::streams::{InputStream, OutputStream};

    let (file, counts, released) = tracked_file();
    file.file.set_len(4).unwrap();
    let mut input = FileInputStream::new(&file, 0, crate::DEFAULT_MAX_IO_SIZE.get());
    assert!(input.read(4).unwrap().is_empty());
    tokio::time::timeout(std::time::Duration::from_secs(5), released)
        .await
        .unwrap()
        .unwrap();
    // Keep both the stream and its unpolled join result alive.
    assert_eq!(counts.live.load(Ordering::SeqCst), 0);
    assert_eq!(counts.admitted.load(Ordering::SeqCst), 1);
    drop(input);

    let (file, counts, released) = tracked_file();
    let mut output = FileOutputStream::write_at(&file, 0, crate::DEFAULT_MAX_IO_SIZE.get());
    output.write(bytes::Bytes::from_static(b"data")).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), released)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(counts.live.load(Ordering::SeqCst), 0);
    assert_eq!(counts.admitted.load(Ordering::SeqCst), 1);
    assert_eq!(file.file.metadata().unwrap().len(), 4);
    drop(output);
}

#[tokio::test]
async fn custom_descriptor_rejection_is_structured_and_bool_traps() {
    let mut ctx = WasiFilesystemCtx::default();
    ctx.set_operation_hook(|| Err(FilesystemOperationError::Rejected));
    let mut table = ResourceTable::new();
    let fd = ctx
        .push_descriptor(
            &mut table,
            Descriptor::File(File::new(
                tempfile::tempfile().unwrap(),
                FsPerms::ReadWrite,
                OpenMode::READ,
                false,
            )),
        )
        .unwrap();
    let descriptor = table.get(&fd).unwrap();
    assert!(matches!(
        descriptor.stat().await,
        Err(ErrorCode::WouldBlock)
    ));
    let error = descriptor.is_same_object(descriptor).await.unwrap_err();
    assert!(error.downcast_ref::<FilesystemOperationError>().is_some());
    #[cfg(feature = "p2")]
    assert!(matches!(
        crate::p2::bindings::filesystem::types::ErrorCode::from(ErrorCode::WouldBlock),
        crate::p2::bindings::filesystem::types::ErrorCode::WouldBlock
    ));
    #[cfg(feature = "p3")]
    assert!(matches!(
        crate::p3::bindings::filesystem::types::ErrorCode::from(ErrorCode::WouldBlock),
        crate::p3::bindings::filesystem::types::ErrorCode::Busy
    ));
    #[cfg(feature = "p2")]
    {
        use crate::p2::bindings::filesystem::types::{ErrorCode as P2Error, HostDescriptor};
        use wasmtime_wasi_io::{
            poll::Pollable,
            streams::{InputStream, StreamError},
        };
        let mut input = crate::p2::filesystem::FileInputStream::new(
            descriptor
                .file()
                .unwrap_or_else(|_| panic!("expected file")),
            0,
            crate::DEFAULT_MAX_IO_SIZE.get(),
        );
        input.ready().await;
        let StreamError::LastOperationFailed(error) = input.read(1).unwrap_err() else {
            panic!("expected operation error")
        };
        assert!(matches!(
            P2Error::from(error.downcast::<std::io::Error>().unwrap()),
            P2Error::WouldBlock
        ));
        let mut view = WasiFilesystemCtxView {
            ctx: &mut ctx,
            table: &mut table,
        };
        assert!(matches!(
            HostDescriptor::read(&mut view, Resource::new_borrow(fd.rep()), 1, 0)
                .await
                .unwrap_err()
                .downcast()
                .unwrap(),
            P2Error::WouldBlock
        ));
        assert!(matches!(
            HostDescriptor::write(&mut view, Resource::new_borrow(fd.rep()), vec![1], 0)
                .await
                .unwrap_err()
                .downcast()
                .unwrap(),
            P2Error::WouldBlock
        ));
    }
}

#[tokio::test]
async fn preopen_builder_order_and_opened_children_use_hook() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("file"), b"data").unwrap();
    let counts = Arc::new(Counts::default());
    let state = counts.clone();
    let mut builder = crate::WasiCtxBuilder::new();
    builder
        .preopened_dir(temp.path(), ".", FsPerms::ReadWrite)
        .unwrap();
    builder.filesystem_operation_hook(move || {
        state.admitted.fetch_add(1, Ordering::SeqCst);
        state.live.fetch_add(1, Ordering::SeqCst);
        Ok(FilesystemOperation::new(Owner(state.clone())))
    });
    let mut ctx = builder.build();
    let mut table = ResourceTable::new();
    let mut view = WasiFilesystemCtxView {
        ctx: &mut ctx.filesystem,
        table: &mut table,
    };
    let preopen = view.get_directories().unwrap().remove(0).0;
    assert_eq!(counts.live.load(Ordering::SeqCst), 0);
    let dir = view
        .table
        .get(&preopen)
        .unwrap()
        .dir()
        .unwrap_or_else(|_| panic!("expected directory"));
    let child = dir
        .open_at(
            PathFlags::empty(),
            "file".into(),
            OpenFlags::empty(),
            DescriptorFlags::READ,
            false,
        )
        .await
        .unwrap_or_else(|_| panic!("open failed"));
    let child = child.push(view.table).unwrap();
    view.table
        .get(&child)
        .unwrap()
        .stat()
        .await
        .unwrap_or_else(|_| panic!("stat failed"));
    assert_eq!(counts.admitted.load(Ordering::SeqCst), 2);
    assert_eq!(counts.live.load(Ordering::SeqCst), 0);
}

#[test]
fn failed_open_releases_before_future_is_polled_again() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    runtime.block_on(async {
        let (started_tx, started_rx) = oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let blocker = tokio::task::spawn_blocking(move || {
            started_tx.send(()).unwrap();
            release_rx.recv().unwrap();
        });
        started_rx.await.unwrap();
        let temp = tempfile::tempdir().unwrap();
        let (file, counts, released) = tracked_file();
        let mut dir = Dir::new(
            primitives::open_ambient_dir(temp.path()).unwrap(),
            FsPerms::ReadWrite,
            OpenMode::READ,
            false,
        );
        dir.operation_hook = file.operation_hook.clone();
        let mut future = pin!(dir.open_at(
            PathFlags::empty(),
            "missing".into(),
            OpenFlags::empty(),
            DescriptorFlags::READ,
            false
        ));
        assert!(
            future
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_pending()
        );
        release_tx.send(()).unwrap();
        blocker.await.unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(30), released)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(counts.live.load(Ordering::SeqCst), 0);
        assert!(matches!(future.await, Err(ErrorCode::NoEntry)));
    });
}

#[tokio::test]
async fn failed_open_installation_disposes_native_result_before_owner() {
    use std::sync::{Weak, atomic::AtomicBool};
    struct Check {
        native: Arc<Mutex<Option<Weak<FilesystemHandle>>>>,
        released: Arc<AtomicBool>,
        disposed: Arc<AtomicBool>,
    }
    impl Drop for Check {
        fn drop(&mut self) {
            self.disposed.store(
                self.native
                    .lock()
                    .unwrap()
                    .as_ref()
                    .is_some_and(|f| f.upgrade().is_none()),
                Ordering::SeqCst,
            );
            self.released.store(true, Ordering::SeqCst);
        }
    }
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("file"), b"data").unwrap();
    let native = Arc::new(Mutex::new(None));
    let released = Arc::new(AtomicBool::new(false));
    let disposed = Arc::new(AtomicBool::new(false));
    let mut dir = Dir::new(
        primitives::open_ambient_dir(temp.path()).unwrap(),
        FsPerms::ReadWrite,
        OpenMode::READ,
        false,
    );
    let state = (native.clone(), released.clone(), disposed.clone());
    dir.operation_hook = Some(Arc::new(move || {
        Ok(FilesystemOperation::new(Check {
            native: state.0.clone(),
            released: state.1.clone(),
            disposed: state.2.clone(),
        }))
    }));
    let opened = dir
        .open_at(
            PathFlags::empty(),
            "file".into(),
            OpenFlags::empty(),
            DescriptorFlags::READ,
            false,
        )
        .await
        .unwrap_or_else(|_| panic!("open failed"));
    assert!(!released.load(Ordering::SeqCst));
    let Descriptor::File(file) = &opened.value else {
        panic!("expected file")
    };
    *native.lock().unwrap() = Some(Arc::downgrade(&file.file));
    let mut table = ResourceTable::new();
    table.set_max_capacity(0);
    assert!(matches!(
        opened.push(&mut table),
        Err(wasmtime::component::ResourceTableError::Full)
    ));
    assert!(released.load(Ordering::SeqCst));
    assert!(disposed.load(Ordering::SeqCst));
}
