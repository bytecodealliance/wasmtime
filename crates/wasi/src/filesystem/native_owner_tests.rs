use super::*;
use std::collections::BTreeMap;
use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Condvar, OnceLock, mpsc};
use std::task::{Context, Poll, Wake, Waker};
use std::time::Duration;
use tokio::sync::oneshot;

#[cfg(unix)]
#[path = "factory_experiment.rs"]
mod factory_experiment;

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

fn owned_dir(
    native: std::fs::File,
) -> (
    Dir,
    Arc<AtomicUsize>,
    oneshot::Receiver<()>,
    std::sync::Weak<Mutex<Release>>,
) {
    let count = Arc::new(AtomicUsize::new(0));
    let (done, released) = oneshot::channel();
    let owner = Arc::new(Mutex::new(Release {
        count: count.clone(),
        done: Some(done),
    }));
    let weak = Arc::downgrade(&owner);
    let mut dir = Dir::new(native, FsPerms::ReadOnly, OpenMode::READ, false);
    dir.operation_hook = Some(Arc::new(move || {
        Ok(FilesystemOperation::new(owner.clone()))
    }));
    (dir, count, released, weak)
}

#[test]
fn idle_descriptor_clones_retain_content_without_admission() {
    for directory in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let native = if directory {
            primitives::open_ambient_dir(temp.path()).unwrap()
        } else {
            tempfile::tempfile().unwrap()
        };
        let descriptor = if directory {
            Descriptor::Dir(Dir::new(native, FsPerms::ReadOnly, OpenMode::READ, false))
        } else {
            Descriptor::File(File::new(native, FsPerms::ReadOnly, OpenMode::READ, false))
        };
        let count = Arc::new(AtomicUsize::new(0));
        let owner = Release {
            count: count.clone(),
            done: None,
        };
        let mut context = WasiFilesystemCtx::default();
        context.set_operation_hook(move || {
            let _ = &owner;
            panic!("idle descriptor clone must not admit work");
        });
        let mut table = ResourceTable::new();
        let resource = context.push_descriptor(&mut table, descriptor).unwrap();
        drop(context);
        let clone = table.get(&resource).unwrap().clone();
        drop(table);
        assert_eq!(count.load(Ordering::SeqCst), 0);
        let native = match &clone {
            Descriptor::File(file) => &file.file,
            Descriptor::Dir(dir) => &dir.dir,
        };
        assert_eq!(native.metadata().unwrap().is_dir(), directory);
        drop(clone);
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }
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
async fn child_result_retains_content_through_installation_or_disposal() {
    for (directory, install) in [(false, true), (false, false), (true, true), (true, false)] {
        let temp = tempfile::tempdir().unwrap();
        if directory {
            std::fs::create_dir(temp.path().join("child")).unwrap();
        } else {
            std::fs::write(temp.path().join("child"), b"contents").unwrap();
        }
        let (parent, count, _, _) = owned_dir(primitives::open_ambient_dir(temp.path()).unwrap());
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
            let (parent, count, _, owner) =
                owned_dir(primitives::open_ambient_dir(temp.path()).unwrap());
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
            assert_eq!(owner.strong_count(), if success { 2 } else { 1 });
            drop(future);
            assert_eq!(owner.strong_count(), 1);
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
    // Embedder-owned extraction uses the existing Descriptor::clone. Declare
    // independent native state first so it closes before the retained factory.
    struct OwnedNative {
        native: std::fs::File,
        _descriptor: Descriptor,
    }
    for directory in [false, true] {
        for duplicate in [false, true] {
            let temp = tempfile::tempdir().unwrap();
            let native = if directory {
                primitives::open_ambient_dir(temp.path()).unwrap()
            } else {
                tempfile::tempfile().unwrap()
            };
            let fd = Arc::new(AtomicI32::new(native.as_raw_fd()));
            let closed = Arc::new(AtomicBool::new(false));
            let owner = Arc::new(Mutex::new(Check {
                fd: fd.clone(),
                closed: closed.clone(),
            }));
            let descriptor = if directory {
                Descriptor::Dir(Dir::new(native, FsPerms::ReadOnly, OpenMode::READ, false))
            } else {
                Descriptor::File(File::new(native, FsPerms::ReadOnly, OpenMode::READ, false))
            };
            let mut context = WasiFilesystemCtx::default();
            context.set_operation_hook(move || {
                let _ = &owner;
                panic!("owned native extraction must not admit work");
            });
            let mut table = ResourceTable::new();
            let resource = context.push_descriptor(&mut table, descriptor).unwrap();
            drop(context);
            let descriptor = table.get(&resource).unwrap();
            let extracted = duplicate.then(|| OwnedNative {
                native: match descriptor {
                    Descriptor::File(file) => file.file.try_clone().unwrap(),
                    Descriptor::Dir(dir) => dir.dir.try_clone().unwrap(),
                },
                _descriptor: descriptor.clone(),
            });
            if let Some(extracted) = &extracted {
                fd.store(extracted.native.as_raw_fd(), Ordering::SeqCst);
            }
            drop(table);
            assert_eq!(closed.load(Ordering::SeqCst), !duplicate);
            if let Some(extracted) = extracted {
                assert_eq!(extracted.native.metadata().unwrap().is_dir(), directory);
                drop(extracted);
            }
            assert!(closed.load(Ordering::SeqCst));
        }
    }

    // Stored preopens also own native handles, even before a descriptor is
    // issued or after all issued descriptors have been dropped.
    let preopens_closed = [false, true].map(|issue| {
        let temp = tempfile::tempdir().unwrap();
        let mut builder = crate::WasiCtxBuilder::new();
        builder
            .preopened_dir(temp.path(), ".", FsPerms::ReadOnly)
            .unwrap();
        let mut ctx = builder.build();
        let closed = Arc::new(AtomicBool::new(false));
        let owner = Check {
            fd: Arc::new(AtomicI32::new(ctx.filesystem.preopens[0].0.dir.as_raw_fd())),
            closed: closed.clone(),
        };
        ctx.filesystem.set_operation_hook(move || {
            let _ = &owner;
            panic!("preopen issuance must not admit work");
        });
        if issue {
            let mut table = ResourceTable::new();
            WasiFilesystemCtxView {
                ctx: &mut ctx.filesystem,
                table: &mut table,
            }
            .get_directories()
            .unwrap();
            drop(table);
        }
        assert!(!closed.load(Ordering::SeqCst));
        drop(ctx);
        closed.load(Ordering::SeqCst)
    });
    assert_eq!(
        preopens_closed,
        [true, true],
        "stored preopens must close before the context's factory owner releases"
    );

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
            let owner = Arc::new(Mutex::new(Check {
                fd: fd.clone(),
                closed: closed.clone(),
            }));
            let mut parent = Dir::new(native, FsPerms::ReadOnly, OpenMode::READ, false);
            parent.operation_hook = Some(Arc::new(move || {
                Ok(FilesystemOperation::new(owner.clone()))
            }));
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
