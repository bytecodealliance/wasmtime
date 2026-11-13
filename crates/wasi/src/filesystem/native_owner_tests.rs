use super::*;
use std::collections::BTreeMap;
use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Condvar, OnceLock, mpsc};
use std::task::{Context, Poll, Wake, Waker};
use std::time::Duration;
use tokio::sync::oneshot;

struct Release {
    count: Arc<AtomicUsize>,
    done: Option<oneshot::Sender<()>>,
}
impl Drop for Release {
    fn drop(&mut self) {
        self.count.fetch_add(1, Ordering::SeqCst);
        if let Some(done) = self.done.take() {
            let _ = done.send(());
        }
    }
}

fn owned(native: std::fs::File) -> (FilesystemHandle, Arc<AtomicUsize>, oneshot::Receiver<()>) {
    let count = Arc::new(AtomicUsize::new(0));
    let (done, released) = oneshot::channel();
    let handle = FilesystemHandle::new(
        native,
        Release {
            count: count.clone(),
            done: Some(done),
        },
    );
    (handle, count, released)
}

#[test]
fn idle_descriptor_and_public_native_clone_retain_content_without_activity() {
    let (handle, count, _) = owned(tempfile::tempfile().unwrap());
    let mut context = WasiFilesystemCtx::default();
    let active = Arc::new(AtomicUsize::new(0));
    let a = active.clone();
    context.set_operation_hook(move || {
        Ok(FilesystemOperation::new(Release {
            count: a.clone(),
            done: None,
        }))
    });
    let mut table = ResourceTable::new();
    let resource = context
        .push_descriptor(
            &mut table,
            Descriptor::File(File::new(handle, FsPerms::ReadOnly, OpenMode::READ, false)),
        )
        .unwrap();
    let file = table
        .get(&resource)
        .unwrap()
        .file()
        .unwrap_or_else(|_| panic!("expected file"));
    drop(file.start_operation().unwrap());
    assert_eq!(active.load(Ordering::SeqCst), 1);
    assert_eq!(count.load(Ordering::SeqCst), 0);
    let native = Arc::clone(&file.file);
    let duplicate = native.try_clone().unwrap();
    drop(table);
    drop(native);
    assert_eq!(count.load(Ordering::SeqCst), 0);
    drop(duplicate);
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[test]
fn idle_directory_clone_retains_content_without_admission() {
    let temp = tempfile::tempdir().unwrap();
    let (handle, count, _) = owned(primitives::open_ambient_dir(temp.path()).unwrap());
    let mut dir = Dir::new(handle, FsPerms::ReadOnly, OpenMode::READ, false);
    dir.operation_hook = Some(Arc::new(|| panic!("idle clone must not admit work")));
    let clone = dir.clone();
    let native = Arc::clone(&dir.dir);
    drop(dir);
    drop(clone);
    assert_eq!(count.load(Ordering::SeqCst), 0);
    assert!(native.metadata().unwrap().is_dir());
    drop(native);
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

// A keyed gate in real multi-descriptor workers lets the initiating future and
// both descriptor resources disappear while the worker still carries B.
struct NativeWorkGate {
    started: mpsc::Sender<()>,
    released: Mutex<bool>,
    wake: Condvar,
}

static NATIVE_WORK: OnceLock<Mutex<BTreeMap<usize, Arc<NativeWorkGate>>>> = OnceLock::new();

pub(super) fn native_work_gate(file: &std::fs::File) {
    let gate = NATIVE_WORK
        .get_or_init(Default::default)
        .lock()
        .unwrap()
        .remove(&(file as *const std::fs::File as usize));
    if let Some(gate) = gate {
        let _ = gate.started.send(());
        let mut released = gate.released.lock().unwrap();
        while !*released {
            released = gate.wake.wait(released).unwrap();
        }
    }
}

struct InstalledNativeWork {
    key: usize,
    gate: Arc<NativeWorkGate>,
    started: mpsc::Receiver<()>,
}

impl InstalledNativeWork {
    fn new(file: &std::fs::File) -> Self {
        let (started, rx) = mpsc::channel();
        let gate = Arc::new(NativeWorkGate {
            started,
            released: Mutex::new(false),
            wake: Condvar::new(),
        });
        let key = file as *const std::fs::File as usize;
        assert!(
            NATIVE_WORK
                .get_or_init(Default::default)
                .lock()
                .unwrap()
                .insert(key, gate.clone())
                .is_none()
        );
        Self {
            key,
            gate,
            started: rx,
        }
    }
}

impl Drop for InstalledNativeWork {
    fn drop(&mut self) {
        *self.gate.released.lock().unwrap() = true;
        self.gate.wake.notify_all();
        NATIVE_WORK.get().unwrap().lock().unwrap().remove(&self.key);
    }
}

#[tokio::test]
async fn second_descriptor_content_survives_actual_comparison_worker() {
    for directory in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let (handle, count, released) = owned(if directory {
            primitives::open_ambient_dir(temp.path()).unwrap()
        } else {
            tempfile::tempfile().unwrap()
        });
        let target = if directory {
            Descriptor::Dir(Dir::new(handle, FsPerms::ReadOnly, OpenMode::READ, false))
        } else {
            Descriptor::File(File::new(handle, FsPerms::ReadOnly, OpenMode::READ, false))
        };
        let mut target_context = WasiFilesystemCtx::default();
        target_context.set_operation_hook(|| panic!("B must not admit an operation"));
        let mut target_table = ResourceTable::new();
        let target = target_context
            .push_descriptor(&mut target_table, target)
            .unwrap();
        let admitted = Arc::new(AtomicUsize::new(0));
        let hook_admitted = admitted.clone();
        let hook: FilesystemOperationHook = Arc::new(move || {
            hook_admitted.fetch_add(1, Ordering::SeqCst);
            Ok(FilesystemOperation::new(()))
        });
        let source = if directory {
            let mut dir = Dir::new(
                primitives::open_ambient_dir(temp.path()).unwrap(),
                FsPerms::ReadOnly,
                OpenMode::READ,
                false,
            );
            dir.operation_hook = Some(hook);
            Descriptor::Dir(dir)
        } else {
            let mut file = File::new(
                tempfile::tempfile().unwrap(),
                FsPerms::ReadOnly,
                OpenMode::READ,
                false,
            );
            file.operation_hook = Some(hook);
            Descriptor::File(file)
        };
        let gate = InstalledNativeWork::new(match &source {
            Descriptor::File(file) => &file.file,
            Descriptor::Dir(dir) => &dir.dir,
        });
        let mut future = Box::pin(source.is_same_object(target_table.get(&target).unwrap()));
        assert!(
            future
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_pending()
        );
        gate.started.recv_timeout(Duration::from_secs(30)).unwrap();
        drop(future);
        drop(target_table);
        drop(target_context);
        drop(source);
        assert_eq!(admitted.load(Ordering::SeqCst), 1);
        assert_eq!(
            count.load(Ordering::SeqCst),
            0,
            "B content released while its native handle is in A's worker"
        );
        drop(gate);
        tokio::time::timeout(Duration::from_secs(30), released)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }
}

#[tokio::test]
async fn second_directory_content_survives_link_and_rename_workers() {
    for rename in [false, true] {
        let source_path = tempfile::tempdir().unwrap();
        let target_path = tempfile::tempdir().unwrap();
        std::fs::write(source_path.path().join("source"), b"contents").unwrap();
        let (handle, count, released) =
            owned(primitives::open_ambient_dir(target_path.path()).unwrap());
        let mut target = Dir::new(handle, FsPerms::ReadWrite, OpenMode::READ, false);
        target.operation_hook = Some(Arc::new(|| panic!("B must not admit an operation")));
        let admitted = Arc::new(AtomicUsize::new(0));
        let hook_admitted = admitted.clone();
        let mut source = Dir::new(
            primitives::open_ambient_dir(source_path.path()).unwrap(),
            FsPerms::ReadWrite,
            OpenMode::READ,
            false,
        );
        source.operation_hook = Some(Arc::new(move || {
            hook_admitted.fetch_add(1, Ordering::SeqCst);
            Ok(FilesystemOperation::new(()))
        }));
        let gate = InstalledNativeWork::new(&source.dir);
        let mut future: std::pin::Pin<Box<dyn Future<Output = Result<(), ErrorCode>> + '_>> =
            if rename {
                Box::pin(source.rename_at("source".into(), &target, "target".into()))
            } else {
                Box::pin(source.link_at(
                    PathFlags::empty(),
                    "source".into(),
                    &target,
                    "target".into(),
                ))
            };
        assert!(
            future
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_pending()
        );
        gate.started.recv_timeout(Duration::from_secs(30)).unwrap();
        drop(future);
        drop(target);
        drop(source);
        assert_eq!(admitted.load(Ordering::SeqCst), 1);
        assert_eq!(
            count.load(Ordering::SeqCst),
            0,
            "B content released while its native handle is in A's worker"
        );
        drop(gate);
        tokio::time::timeout(Duration::from_secs(30), released)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 1);
        assert_eq!(
            std::fs::read(target_path.path().join("target")).unwrap(),
            b"contents"
        );
        assert_eq!(source_path.path().join("source").exists(), !rename);
    }
}

#[tokio::test]
async fn native_worker_retains_content_after_store_drop() {
    let (handle, count, released) = owned(tempfile::tempfile().unwrap());
    let file = File::new(handle, FsPerms::ReadOnly, OpenMode::READ, false);
    let native = Arc::downgrade(&file.file);
    let mut store = wasmtime::Store::new(&wasmtime::Engine::default(), file);
    let (started_tx, started_rx) = oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    {
        let mut future = Box::pin(store.data_mut().run_blocking(move |file| {
            started_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            file.metadata()
        }));
        assert!(
            future
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_pending()
        );
        started_rx.await.unwrap();
    }
    drop(store);
    assert!(native.upgrade().is_some());
    assert_eq!(count.load(Ordering::SeqCst), 0);
    release_tx.send(()).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(30), released)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert!(native.upgrade().is_none());
}

#[tokio::test]
async fn child_result_retains_content_through_installation_or_disposal() {
    for (directory, install) in [(false, true), (false, false), (true, true), (true, false)] {
        let temp = tempfile::tempdir().unwrap();
        if directory {
            std::fs::create_dir(temp.path().join("child")).unwrap();
        } else {
            std::fs::write(temp.path().join("child"), b"contents").unwrap();
        }
        let (handle, count, _) = owned(primitives::open_ambient_dir(temp.path()).unwrap());
        let parent = Dir::new(handle, FsPerms::ReadOnly, OpenMode::READ, false);
        let result = parent
            .open_at(
                PathFlags::empty(),
                "child".into(),
                OpenFlags::empty(),
                DescriptorFlags::READ,
                false,
            )
            .await
            .unwrap_or_else(|_| panic!("open failed"));
        drop(parent);
        assert_eq!(count.load(Ordering::SeqCst), 0);
        let mut table = ResourceTable::new();
        if install {
            let child = result.push(&mut table).unwrap();
            match table.get(&child).unwrap() {
                Descriptor::Dir(dir) => assert!(directory && dir.dir.metadata().unwrap().is_dir()),
                Descriptor::File(file) => {
                    assert!(!directory);
                    assert_eq!(file.file.metadata().unwrap().len(), 8);
                }
            }
            assert_eq!(count.load(Ordering::SeqCst), 0);
            drop(table);
        } else {
            table.set_max_capacity(0);
            assert!(matches!(
                result.push(&mut table),
                Err(wasmtime::component::ResourceTableError::Full)
            ));
        }
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }
}

struct Signal(std::sync::mpsc::Sender<()>);
impl Wake for Signal {
    fn wake(self: Arc<Self>) {
        let _ = self.0.send(());
    }
    fn wake_by_ref(self: &Arc<Self>) {
        let _ = self.0.send(());
    }
}

#[test]
fn completed_unpolled_open_owns_only_successful_native_results() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    runtime.block_on(async {
        // Missing path, successful file, successful directory, and a native
        // file opened but rejected because DIRECTORY was requested.
        for (kind, success) in [(0, false), (1, true), (2, true), (3, false)] {
            let (entered_tx, entered_rx) = oneshot::channel();
            let (release_tx, release_rx) = std::sync::mpsc::channel();
            let blocker = tokio::task::spawn_blocking(move || {
                entered_tx.send(()).unwrap();
                release_rx.recv().unwrap();
            });
            entered_rx.await.unwrap();
            let temp = tempfile::tempdir().unwrap();
            if kind == 1 || kind == 3 {
                std::fs::write(temp.path().join("child"), b"contents").unwrap();
            } else if kind == 2 {
                std::fs::create_dir(temp.path().join("child")).unwrap();
            }
            let (handle, count, _) = owned(primitives::open_ambient_dir(temp.path()).unwrap());
            let parent = Dir::new(handle, FsPerms::ReadOnly, OpenMode::READ, false);
            let (woke_tx, woke_rx) = std::sync::mpsc::channel();
            let waker = Waker::from(Arc::new(Signal(woke_tx)));
            let mut future = Box::pin(parent.open_at(
                PathFlags::empty(),
                "child".into(),
                if kind == 3 {
                    OpenFlags::DIRECTORY
                } else {
                    OpenFlags::empty()
                },
                DescriptorFlags::READ,
                false,
            ));
            assert!(matches!(
                future.as_mut().poll(&mut Context::from_waker(&waker)),
                Poll::Pending
            ));
            release_tx.send(()).unwrap();
            blocker.await.unwrap();
            woke_rx
                .recv_timeout(std::time::Duration::from_secs(30))
                .unwrap();
            // Only the parent and a successful native result can own this value.
            assert_eq!(
                Arc::strong_count(parent.dir.owner.as_ref().unwrap()),
                if success { 2 } else { 1 }
            );
            drop(future);
            assert_eq!(Arc::strong_count(parent.dir.owner.as_ref().unwrap()), 1);
            drop(parent);
            assert_eq!(count.load(Ordering::SeqCst), 1);
        }
    });
}

#[cfg(unix)]
#[test]
fn content_owner_runs_after_native_file_closes() {
    // Isolate the OS descriptor table: another concurrently running test could
    // reuse the closed descriptor number before the owner checks F_GETFD.
    const CHILD: &str = "WASMTIME_NATIVE_OWNER_CLOSE_PROBE";
    if std::env::var_os(CHILD).is_none() {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "filesystem::native_owner_tests::content_owner_runs_after_native_file_closes",
                "--test-threads=1",
            ])
            .env(CHILD, "1")
            .status()
            .unwrap();
        assert!(status.success());
        return;
    }
    use std::os::fd::AsRawFd;
    use std::sync::atomic::{AtomicBool, AtomicI32};
    unsafe extern "C" {
        fn fcntl(fd: std::ffi::c_int, cmd: std::ffi::c_int, ...) -> std::ffi::c_int;
    }
    struct Check {
        fd: Arc<AtomicI32>,
        closed: Arc<AtomicBool>,
    }
    impl Drop for Check {
        fn drop(&mut self) {
            // F_GETFD is 1 on this Unix test target. No fd borrow is constructed.
            self.closed.store(
                unsafe { fcntl(self.fd.load(Ordering::SeqCst), 1) } == -1,
                Ordering::SeqCst,
            );
        }
    }
    let native = tempfile::tempfile().unwrap();
    let closed = Arc::new(AtomicBool::new(false));
    let owner = Check {
        fd: Arc::new(AtomicI32::new(native.as_raw_fd())),
        closed: closed.clone(),
    };
    drop(FilesystemHandle::new(native, owner));
    assert!(closed.load(Ordering::SeqCst));

    // Observe the actual child fd during failed insertion, not just the parent
    // handle or the descriptor allocation's reference count.
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(async {
            let temp = tempfile::tempdir().unwrap();
            std::fs::write(temp.path().join("child"), b"contents").unwrap();
            let native = primitives::open_ambient_dir(temp.path()).unwrap();
            let fd = Arc::new(AtomicI32::new(native.as_raw_fd()));
            let closed = Arc::new(AtomicBool::new(false));
            let parent = Dir::new(
                FilesystemHandle::new(
                    native,
                    Check {
                        fd: fd.clone(),
                        closed: closed.clone(),
                    },
                ),
                FsPerms::ReadOnly,
                OpenMode::READ,
                false,
            );
            let result = parent
                .open_at(
                    PathFlags::empty(),
                    "child".into(),
                    OpenFlags::empty(),
                    DescriptorFlags::READ,
                    false,
                )
                .await
                .unwrap_or_else(|_| panic!("open failed"));
            let Descriptor::File(child) = &result.value else {
                panic!("expected child file")
            };
            fd.store(child.file.as_raw_fd(), Ordering::SeqCst);
            drop(parent);
            assert!(!closed.load(Ordering::SeqCst));
            let mut table = ResourceTable::new();
            table.set_max_capacity(0);
            assert!(matches!(
                result.push(&mut table),
                Err(wasmtime::component::ResourceTableError::Full)
            ));
            assert!(
                closed.load(Ordering::SeqCst),
                "child native fd must close before its content owner releases"
            );
        });
}
