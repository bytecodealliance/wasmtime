use super::*;
use crate::filesystem::{FilesystemOperation, FsPerms, OpenMode};
use std::sync::atomic::{AtomicUsize, Ordering};

const TEST_MAX_IO_SIZE: usize = crate::DEFAULT_MAX_IO_SIZE.get();

struct CountDrop(Arc<AtomicUsize>);

impl Drop for CountDrop {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn terminal_read_releases_operation_but_retains_producer_content() {
    for overflow in [false, true] {
        let content = Arc::new(AtomicUsize::new(0));
        let operation = Arc::new(AtomicUsize::new(0));
        let (result, mut status) = oneshot::channel();
        let mut producer = ReadStreamProducer {
            file: File::new(
                FilesystemHandle::new(tempfile::tempfile().unwrap(), CountDrop(content.clone())),
                FsPerms::ReadOnly,
                OpenMode::READ,
                false,
            ),
            offset: u64::MAX,
            result: Some(result),
            task: None,
            max_read_size: TEST_MAX_IO_SIZE,
            operation: Some(FilesystemOperation::new(CountDrop(operation.clone()))),
        };
        if overflow {
            assert!(matches!(producer.complete_read(1), StreamResult::Dropped));
            assert!(matches!(status.try_recv(), Ok(Err(ErrorCode::Overflow))));
        } else {
            producer.close(Ok(()));
            assert!(matches!(status.try_recv(), Ok(Ok(()))));
        }
        assert_eq!(operation.load(Ordering::SeqCst), 1);
        assert_eq!(content.load(Ordering::SeqCst), 0);
        assert!(producer.file.file.metadata().is_ok());
        drop(producer);
        assert_eq!(content.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn terminal_write_and_append_retain_consumer_content() {
    for location in [WriteLocation::Offset(0), WriteLocation::End] {
        let content = Arc::new(AtomicUsize::new(0));
        let operation = Arc::new(AtomicUsize::new(0));
        let (result, mut status) = oneshot::channel();
        let mut consumer = WriteStreamConsumer::new(
            File::new(
                FilesystemHandle::new(tempfile::tempfile().unwrap(), CountDrop(content.clone())),
                FsPerms::ReadWrite,
                OpenMode::WRITE,
                false,
            ),
            location,
            result,
            TEST_MAX_IO_SIZE,
            Some(FilesystemOperation::new(CountDrop(operation.clone()))),
        );
        consumer.close(Ok(()));
        assert!(matches!(status.try_recv(), Ok(Ok(()))));
        assert_eq!(operation.load(Ordering::SeqCst), 1);
        assert_eq!(content.load(Ordering::SeqCst), 0);
        assert!(consumer.file.file.metadata().is_ok());
        drop(consumer);
        assert_eq!(content.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn terminal_read_releases_session_before_producer_drop() {
    let count = Arc::new(AtomicUsize::new(0));
    let (result, mut status) = oneshot::channel();
    let mut producer = ReadStreamProducer {
        file: File::new(
            tempfile::tempfile().unwrap(),
            FsPerms::ReadOnly,
            OpenMode::READ,
            false,
        ),
        offset: u64::MAX,
        result: Some(result),
        task: None,
        max_read_size: TEST_MAX_IO_SIZE,
        operation: Some(FilesystemOperation::new(CountDrop(count.clone()))),
    };
    assert!(matches!(producer.complete_read(1), StreamResult::Dropped));
    assert!(matches!(status.try_recv(), Ok(Err(ErrorCode::Overflow))));
    assert_eq!(count.load(Ordering::SeqCst), 1);
    drop(producer);
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

use futures::FutureExt as _;
use std::collections::BTreeMap;
use std::sync::{Condvar, Mutex, OnceLock, mpsc as sync_mpsc};
use std::time::Duration;
use wasmtime::component::FutureConsumer;
use wasmtime::{Config, Engine, Store};

// Select the exact file under test so parallel filesystem tests cannot enter a
// gate installed by another test. Timeouts only bound a broken test; channels
// and the condition variable establish every ordering asserted below.
struct NativeGate {
    started: sync_mpsc::Sender<()>,
    released: Mutex<bool>,
    wake: Condvar,
}

static GATES: OnceLock<Mutex<BTreeMap<usize, Arc<NativeGate>>>> = OnceLock::new();
static GUEST_GATES: Mutex<Vec<(std::fs::File, Arc<NativeGate>)>> = Mutex::new(Vec::new());

pub(super) fn native_work_gate(file: &Arc<FilesystemHandle>) {
    let gate = GATES
        .get_or_init(Default::default)
        .lock()
        .unwrap()
        .remove(&(Arc::as_ptr(file) as usize));
    // A guest opens its own File allocation. Match its native identity against
    // a separately opened handle in the test's private temporary directory.
    let gate = gate.or_else(|| {
        let mut gates = GUEST_GATES.lock().unwrap();
        let index = gates
            .iter()
            .position(|(native, _)| sys::is_same_file(native, file).unwrap())?;
        Some(gates.swap_remove(index).1)
    });
    if let Some(gate) = gate {
        let _ = gate.started.send(());
        let mut released = gate.released.lock().unwrap();
        while !*released {
            released = gate.wake.wait(released).unwrap();
        }
    }
}

struct InstalledGate {
    key: usize,
    gate: Arc<NativeGate>,
    started: sync_mpsc::Receiver<()>,
}

impl InstalledGate {
    fn new(file: &Arc<FilesystemHandle>) -> Self {
        let (tx, started) = sync_mpsc::channel();
        let gate = Arc::new(NativeGate {
            started: tx,
            released: Mutex::new(false),
            wake: Condvar::new(),
        });
        let key = Arc::as_ptr(file) as usize;
        assert!(
            GATES
                .get_or_init(Default::default)
                .lock()
                .unwrap()
                .insert(key, gate.clone())
                .is_none()
        );
        Self { key, gate, started }
    }

    fn wait_started(&self) {
        self.started.recv_timeout(Duration::from_secs(30)).unwrap();
    }

    fn release(&self) {
        *self.gate.released.lock().unwrap() = true;
        self.gate.wake.notify_all();
    }
}

impl Drop for InstalledGate {
    fn drop(&mut self) {
        self.release();
        GATES.get().unwrap().lock().unwrap().remove(&self.key);
    }
}

#[cfg(feature = "p2")]
struct GuestGate {
    gate: Arc<NativeGate>,
    started: sync_mpsc::Receiver<()>,
}

#[cfg(feature = "p2")]
impl GuestGate {
    fn new(file: std::fs::File) -> Self {
        let (tx, started) = sync_mpsc::channel();
        let gate = Arc::new(NativeGate {
            started: tx,
            released: Mutex::new(false),
            wake: Condvar::new(),
        });
        GUEST_GATES.lock().unwrap().push((file, gate.clone()));
        Self { gate, started }
    }

    fn wait_started(&self) {
        self.started.recv_timeout(Duration::from_secs(30)).unwrap();
    }

    fn release(&self) {
        *self.gate.released.lock().unwrap() = true;
        self.gate.wake.notify_all();
    }
}

#[cfg(feature = "p2")]
impl Drop for GuestGate {
    fn drop(&mut self) {
        self.release();
        GUEST_GATES
            .lock()
            .unwrap()
            .retain(|(_, gate)| !Arc::ptr_eq(gate, &self.gate));
    }
}

#[derive(Default)]
struct Counts {
    admitted: AtomicUsize,
    released: AtomicUsize,
}

struct Release {
    counts: Arc<Counts>,
    done: sync_mpsc::Sender<()>,
}

impl Drop for Release {
    fn drop(&mut self) {
        self.counts.released.fetch_add(1, Ordering::SeqCst);
        let _ = self.done.send(());
    }
}

fn file(blocking: bool, contents: &[u8]) -> (File, Arc<Counts>, sync_mpsc::Receiver<()>) {
    use std::io::Write as _;
    let mut native = tempfile::tempfile().unwrap();
    native.write_all(contents).unwrap();
    let mut file = File::new(
        native,
        FsPerms::ReadWrite,
        OpenMode::READ | OpenMode::WRITE,
        blocking,
    );
    let counts = Arc::new(Counts::default());
    let hook_counts = counts.clone();
    let (done, released) = sync_mpsc::channel();
    file.operation_hook = Some(Arc::new(move || {
        hook_counts.admitted.fetch_add(1, Ordering::SeqCst);
        Ok(FilesystemOperation::new(Release {
            counts: hook_counts.clone(),
            done: done.clone(),
        }))
    }));
    (file, counts, released)
}

fn store() -> Store<()> {
    let mut config = Config::new();
    config.wasm_component_model_async(true);
    Store::new(&Engine::new(&config).unwrap(), ())
}

struct Collect {
    bytes: Vec<u8>,
    chunks: usize,
    counts: Arc<Counts>,
    done: Option<oneshot::Sender<(Vec<u8>, usize)>>,
}

impl StreamConsumer<()> for Collect {
    type Item = u8;

    fn poll_consume(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        mut store: StoreContextMut<()>,
        mut src: Source<'_, u8>,
        _: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        let mut bytes = Vec::with_capacity(src.remaining(&mut store)?);
        src.read(&mut store, &mut bytes)?;
        if !bytes.is_empty() {
            assert_eq!(self.counts.admitted.load(Ordering::SeqCst), 1);
            assert_eq!(self.counts.released.load(Ordering::SeqCst), 0);
            self.chunks += 1;
            self.bytes.extend(bytes);
        }
        Poll::Ready(Ok(StreamResult::Completed))
    }
}

impl Drop for Collect {
    fn drop(&mut self) {
        let _ = self
            .done
            .take()
            .unwrap()
            .send((mem::take(&mut self.bytes), self.chunks));
    }
}

fn collect(
    stream: StreamReader<u8>,
    store: &mut Store<()>,
    counts: Arc<Counts>,
) -> oneshot::Receiver<(Vec<u8>, usize)> {
    let (tx, rx) = oneshot::channel();
    stream
        .pipe(
            store,
            Collect {
                bytes: Vec::new(),
                chunks: 0,
                counts,
                done: Some(tx),
            },
        )
        .unwrap();
    rx
}

struct Status(Option<oneshot::Sender<Result<(), ErrorCode>>>);

impl FutureConsumer<()> for Status {
    type Item = Result<(), ErrorCode>;

    fn poll_consume(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        mut store: StoreContextMut<()>,
        mut source: Source<'_, Self::Item>,
        _: bool,
    ) -> Poll<wasmtime::Result<()>> {
        let mut value = None;
        source.read(&mut store, &mut value)?;
        let _ = self.0.take().unwrap().send(value.unwrap());
        Poll::Ready(Ok(()))
    }
}

#[tokio::test]
async fn read_session_admits_once_across_chunks_and_releases_at_eof() {
    for blocking in [false, true] {
        let contents = vec![42; TEST_MAX_IO_SIZE * 3];
        let (file, counts, _) = file(blocking, &contents);
        let mut store = store();
        let (stream, mut status) =
            read_via_stream(&mut store, Descriptor::File(file), 0, TEST_MAX_IO_SIZE).unwrap();
        // Closing the status resource must not release an active stream.
        status.close(&mut store).unwrap();
        let done = collect(stream, &mut store, counts.clone());
        let (bytes, chunks) = store
            .run_concurrent(async |_| done.await)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(bytes, contents);
        assert!(chunks > 1);
        assert_eq!(counts.admitted.load(Ordering::SeqCst), 1);
        assert_eq!(counts.released.load(Ordering::SeqCst), 1);
    }
}

#[tokio::test]
async fn write_and_append_sessions_admit_once_and_release_on_completion() {
    for blocking in [false, true] {
        for location in [WriteLocation::Offset(0), WriteLocation::End] {
            let (file, counts, _) = file(blocking, &[]);
            let native = file.file.clone();
            let mut store = store();
            let bytes = vec![42; TEST_MAX_IO_SIZE * 3];
            let input = StreamReader::new(&mut store, bytes.clone()).unwrap();
            let status =
                write_via_stream(&mut store, Ok(file), input, location, TEST_MAX_IO_SIZE).unwrap();
            assert_eq!(counts.admitted.load(Ordering::SeqCst), 1);
            assert_eq!(counts.released.load(Ordering::SeqCst), 0);
            let (tx, rx) = oneshot::channel();
            status.pipe(&mut store, Status(Some(tx))).unwrap();
            store
                .run_concurrent(async |_| rx.await)
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            assert_eq!(counts.admitted.load(Ordering::SeqCst), 1);
            assert_eq!(counts.released.load(Ordering::SeqCst), 1);
            let mut actual = vec![0; bytes.len()];
            assert_eq!(
                sys::read_at_cursor_unspecified(&native, &mut actual, 0).unwrap(),
                bytes.len()
            );
            assert_eq!(actual, bytes);
        }
    }
}

#[tokio::test]
async fn started_read_write_and_append_outlive_store() {
    for location in [
        None,
        Some(WriteLocation::Offset(0)),
        Some(WriteLocation::End),
    ] {
        let (file, counts, released) = file(false, &[1, 2, 3]);
        let gate = InstalledGate::new(&file.file);
        let mut store = store();
        match location {
            None => {
                let (stream, mut status) =
                    read_via_stream(&mut store, Descriptor::File(file), 0, TEST_MAX_IO_SIZE)
                        .unwrap();
                status.close(&mut store).unwrap();
                let _ = collect(stream, &mut store, counts.clone());
            }
            Some(location) => {
                let input = StreamReader::new(&mut store, vec![4, 5, 6]).unwrap();
                let mut status =
                    write_via_stream(&mut store, Ok(file), input, location, TEST_MAX_IO_SIZE)
                        .unwrap();
                status.close(&mut store).unwrap();
            }
        }
        assert!(
            store
                .run_concurrent(async |_| std::future::pending::<()>().await)
                .now_or_never()
                .is_none()
        );
        gate.wait_started();
        drop(store);
        assert_eq!(counts.released.load(Ordering::SeqCst), 0);
        gate.release();
        released.recv_timeout(Duration::from_secs(30)).unwrap();
        assert_eq!(counts.released.load(Ordering::SeqCst), 1);
    }
}

#[tokio::test]
async fn eager_directory_worker_outlives_store_and_releases_without_polling() {
    for drop_store in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let (file, counts, released) = file(false, &[]);
        let mut dir = Dir::new(
            crate::filesystem::primitives::open_ambient_dir(directory.path()).unwrap(),
            FsPerms::ReadOnly,
            OpenMode::READ,
            false,
        );
        dir.operation_hook = file.operation_hook;
        let gate = InstalledGate::new(&dir.dir);
        let mut store = store();
        let (_stream, _status) = read_directory(&mut store, Ok(dir)).unwrap();
        gate.wait_started();
        let mut store = Some(store);
        if drop_store {
            drop(store.take());
        }
        assert_eq!(counts.released.load(Ordering::SeqCst), 0);
        gate.release();
        released.recv_timeout(Duration::from_secs(30)).unwrap();
        assert_eq!(counts.released.load(Ordering::SeqCst), 1);
        drop(store);
    }
}

#[test]
fn partial_factory_failure_closes_already_installed_status() {
    let mut store = store();
    let baseline = store.concurrent_state_table_size();
    let (tx, rx) = oneshot::channel::<Result<(), ErrorCode>>();
    let status = FutureReader::new(&mut store, rx).unwrap();
    assert!(store.concurrent_state_table_size() > baseline);
    let failed_stream: wasmtime::Result<StreamReader<u8>> =
        Err(wasmtime::format_err!("stream installation failed"));
    assert!(finish_stream_factory(&mut store, status, failed_stream).is_err());
    assert_eq!(store.concurrent_state_table_size(), baseline);
    assert!(tx.is_closed());

    let (tx, rx) = oneshot::channel::<Result<(), ErrorCode>>();
    let status = FutureReader::new(&mut store, rx).unwrap();
    assert!(
        finish_pipe_factory(
            &mut store,
            status,
            Err(wasmtime::format_err!("pipe failed"))
        )
        .is_err()
    );
    assert_eq!(store.concurrent_state_table_size(), baseline);
    assert!(tx.is_closed());
}

#[tokio::test]
async fn rejected_stream_admission_reports_busy() {
    use crate::filesystem::FilesystemOperationError;
    let (mut file, _, _) = file(false, &[]);
    file.operation_hook = Some(Arc::new(|| Err(FilesystemOperationError::Rejected)));
    let mut store = store();
    let (mut stream, status) =
        read_via_stream(&mut store, Descriptor::File(file), 0, TEST_MAX_IO_SIZE).unwrap();
    stream.close(&mut store).unwrap();
    let (tx, rx) = oneshot::channel();
    status.pipe(&mut store, Status(Some(tx))).unwrap();
    assert!(matches!(
        store
            .run_concurrent(async |_| rx.await)
            .await
            .unwrap()
            .unwrap(),
        Err(ErrorCode::Busy)
    ));
}

struct DrainDirectory(Option<oneshot::Sender<usize>>, usize);

impl StreamConsumer<()> for DrainDirectory {
    type Item = DirectoryEntry;

    fn poll_consume(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        mut store: StoreContextMut<()>,
        mut source: Source<'_, Self::Item>,
        _: bool,
    ) -> Poll<wasmtime::Result<StreamResult>> {
        let mut entries = Vec::with_capacity(source.remaining(&mut store)?);
        source.read(&mut store, &mut entries)?;
        self.1 += entries.len();
        Poll::Ready(Ok(StreamResult::Completed))
    }
}

impl Drop for DrainDirectory {
    fn drop(&mut self) {
        let _ = self.0.take().unwrap().send(self.1);
    }
}

#[tokio::test]
async fn synchronous_directory_keeps_owner_until_iterator_finishes() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("file"), []).unwrap();
    let (file, counts, _) = file(true, &[]);
    let mut dir = Dir::new(
        crate::filesystem::primitives::open_ambient_dir(directory.path()).unwrap(),
        FsPerms::ReadOnly,
        OpenMode::READ,
        true,
    );
    dir.operation_hook = file.operation_hook;
    let mut store = store();
    let (stream, mut status) = read_directory(&mut store, Ok(dir)).unwrap();
    assert_eq!(counts.admitted.load(Ordering::SeqCst), 1);
    assert_eq!(counts.released.load(Ordering::SeqCst), 0);
    status.close(&mut store).unwrap();
    let (tx, rx) = oneshot::channel();
    stream
        .pipe(&mut store, DrainDirectory(Some(tx), 0))
        .unwrap();
    assert_eq!(
        store
            .run_concurrent(async |_| rx.await)
            .await
            .unwrap()
            .unwrap(),
        1
    );
    assert_eq!(counts.released.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn p3_native_read_keeps_content_after_store_drop() {
    let content = Arc::new(AtomicUsize::new(0));
    let (mut file, counts, released) = file(false, &[1, 2, 3]);
    let native = tempfile::tempfile().unwrap();
    file.file = Arc::new(FilesystemHandle::new(native, CountDrop(content.clone())));
    let gate = InstalledGate::new(&file.file);
    let mut store = store();
    let (stream, mut status) =
        read_via_stream(&mut store, Descriptor::File(file), 0, TEST_MAX_IO_SIZE).unwrap();
    status.close(&mut store).unwrap();
    let _ = collect(stream, &mut store, counts.clone());
    assert!(
        store
            .run_concurrent(async |_| std::future::pending::<()>().await)
            .now_or_never()
            .is_none()
    );
    gate.wait_started();
    drop(store);
    assert_eq!(content.load(Ordering::SeqCst), 0);
    gate.release();
    released.recv_timeout(Duration::from_secs(30)).unwrap();
    assert_eq!(content.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn direct_p3_directory_iterator_retains_content_through_stream_lifetime() {
    for consume in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("child"), b"data").unwrap();
        let content = Arc::new(AtomicUsize::new(0));
        let native = crate::filesystem::primitives::open_ambient_dir(temp.path()).unwrap();
        let dir = Dir::new(
            FilesystemHandle::new(native, CountDrop(content.clone())),
            FsPerms::ReadOnly,
            OpenMode::READ,
            true,
        );
        let mut store = store();
        let (mut stream, mut status) = read_directory(&mut store, Ok(dir)).unwrap();
        assert_eq!(content.load(Ordering::SeqCst), 0);
        status.close(&mut store).unwrap();
        if consume {
            let (tx, rx) = oneshot::channel();
            stream
                .pipe(&mut store, DrainDirectory(Some(tx), 0))
                .unwrap();
            assert_eq!(
                store
                    .run_concurrent(async |_| rx.await)
                    .await
                    .unwrap()
                    .unwrap(),
                1
            );
        } else {
            stream.close(&mut store).unwrap();
        }
        assert_eq!(content.load(Ordering::SeqCst), 1);
    }
}

#[cfg(feature = "p2")]
struct GuestCtx {
    wasi: crate::WasiCtx,
    table: ResourceTable,
}

#[cfg(feature = "p2")]
impl crate::WasiView for GuestCtx {
    fn ctx(&mut self) -> crate::WasiCtxView<'_> {
        crate::WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

#[cfg(feature = "p2")]
async fn run_guest(
    path: std::path::PathBuf,
    scenario: &'static str,
    blocking: bool,
    counts: Arc<Counts>,
    released: sync_mpsc::Sender<()>,
) -> wasmtime::Result<()> {
    use crate::p3::bindings::Command;
    use wasmtime::component::{Component, Linker};

    let engine = test_programs_artifacts::engine(|config| {
        config.wasm_component_model_async(true);
    });
    let mut linker = Linker::new(&engine);
    // The test component uses a P1 adapter whose P2 imports require linking.
    crate::p2::add_to_linker_async(&mut linker)?;
    crate::p3::add_to_linker(&mut linker)?;
    let mut builder = crate::WasiCtxBuilder::new();
    builder
        .args(&["filesystem-lifecycle", ".", scenario])
        .allow_blocking_current_thread(blocking)
        .preopened_dir(path, ".", FsPerms::ReadWrite)?;
    builder.filesystem_operation_hook(move || {
        counts.admitted.fetch_add(1, Ordering::SeqCst);
        Ok(FilesystemOperation::new(Release {
            counts: counts.clone(),
            done: released.clone(),
        }))
    });
    let mut store = Store::new(
        &engine,
        GuestCtx {
            wasi: builder.build(),
            table: ResourceTable::new(),
        },
    );
    let component = Component::from_file(
        &engine,
        test_programs_artifacts::P3_API_FILESYSTEM_LIFECYCLE_COMPONENT,
    )?;
    let command = Command::instantiate_async(&mut store, &component, &linker).await?;
    store
        .run_concurrent(async move |store| command.wasi_cli_run().call_run(store).await)
        .await??
        .map_err(|()| wasmtime::format_err!("filesystem lifecycle guest failed"))
}

#[cfg(feature = "p2")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn guest_native_work_retains_admission_after_call_and_store_cancellation() {
    for scenario in ["read", "write", "append", "readdir"] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("lifecycle.txt");
        std::fs::write(&path, vec![b'x'; 16 * 1024]).unwrap();
        let native = if scenario == "readdir" {
            crate::filesystem::primitives::open_ambient_dir(directory.path()).unwrap()
        } else {
            std::fs::File::open(path).unwrap()
        };
        let gate = GuestGate::new(native);
        let counts = Arc::new(Counts::default());
        let (tx, rx) = sync_mpsc::channel();
        let task = tokio::spawn(run_guest(
            directory.path().to_owned(),
            scenario,
            false,
            counts.clone(),
            tx,
        ));
        gate.wait_started();
        let admitted = counts.admitted.load(Ordering::SeqCst);
        assert_eq!(counts.released.load(Ordering::SeqCst) + 1, admitted);
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert_eq!(counts.released.load(Ordering::SeqCst) + 1, admitted);
        gate.release();
        while counts.released.load(Ordering::SeqCst) != admitted {
            rx.recv_timeout(Duration::from_secs(30)).unwrap();
        }
        assert_eq!(counts.admitted.load(Ordering::SeqCst), admitted);
    }
}

#[cfg(feature = "p2")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn guest_terminal_read_releases_admission_with_inert_stream_and_store_alive() {
    for blocking in [false, true] {
        for scenario in ["eof", "error"] {
            let directory = tempfile::tempdir().unwrap();
            std::fs::write(
                directory.path().join("lifecycle.txt"),
                vec![b'x'; 16 * 1024],
            )
            .unwrap();
            let counts = Arc::new(Counts::default());
            let (tx, rx) = sync_mpsc::channel();
            let task = tokio::spawn(run_guest(
                directory.path().to_owned(),
                scenario,
                blocking,
                counts.clone(),
                tx,
            ));
            // open-at, one read session across all chunks, then the guest's
            // terminal marker (stat). No Store teardown can satisfy this count.
            while counts.released.load(Ordering::SeqCst) < 3 {
                rx.recv_timeout(Duration::from_secs(30)).unwrap();
            }
            assert_eq!(counts.admitted.load(Ordering::SeqCst), 3);
            assert_eq!(counts.released.load(Ordering::SeqCst), 3);
            assert!(!task.is_finished(), "guest did not retain the inert stream");
            task.abort();
            assert!(task.await.unwrap_err().is_cancelled());
        }
    }
}
