//! Two-operand ownership checks; the cleanup queue contains no generation Arc.

use super::*;
use std::os::fd::AsRawFd;

struct NativeProbe {
    fd: i32,
    alive: Box<dyn Fn() -> bool + Send + Sync>,
    generation_drops: AtomicUsize,
    hook_calls: AtomicUsize,
}

fn fd_closed(fd: i32) -> bool {
    unsafe extern "C" {
        fn fcntl(fd: std::ffi::c_int, cmd: std::ffi::c_int, ...) -> std::ffi::c_int;
    }
    // F_GETFD is 1 on this Unix target. Probe before cleanup can reuse the fd.
    (unsafe { fcntl(fd, 1) }) == -1
        && std::io::Error::last_os_error().raw_os_error()
            == Some(rustix::io::Errno::BADF.raw_os_error())
}

#[derive(Debug)]
struct GenerationDrop {
    operand: usize,
    native_closed: bool,
    admission_live: usize,
    directory: tempfile::TempDir,
}

struct AdmissionState {
    live: AtomicUsize,
    dropped: AtomicUsize,
    worker_unwound: std::sync::atomic::AtomicBool,
    native_closed_at_drop: Mutex<Option<[bool; 2]>>,
}

struct Generation {
    directory: Option<tempfile::TempDir>,
    operand: usize,
    probe: Arc<NativeProbe>,
    admission: Arc<AdmissionState>,
    cleanup: mpsc::Sender<GenerationDrop>,
}

impl Drop for Generation {
    fn drop(&mut self) {
        let native_closed = fd_closed(self.probe.fd);
        self.probe.generation_drops.fetch_add(1, Ordering::SeqCst);
        // Transfer only the physical cleanup job, never a generation owner.
        // Defer cleanup I/O until both native-close observations are recorded.
        let _ = self.cleanup.send(GenerationDrop {
            operand: self.operand,
            native_closed,
            admission_live: self.admission.live.load(Ordering::SeqCst),
            directory: self.directory.take().unwrap(),
        });
    }
}

struct Admission {
    state: Arc<AdmissionState>,
    native: [Arc<NativeProbe>; 2],
}

impl Drop for Admission {
    fn drop(&mut self) {
        self.state
            .worker_unwound
            .store(std::thread::panicking(), Ordering::SeqCst);
        *self.state.native_closed_at_drop.lock().unwrap() =
            Some(self.native.each_ref().map(|probe| fd_closed(probe.fd)));
        self.state.live.fetch_sub(1, Ordering::SeqCst);
        self.state.dropped.fetch_add(1, Ordering::SeqCst);
    }
}

struct OperationValue {
    // Generation release precedes admission release. Both follow native state
    // because NativeOperation declares its native value before its operation.
    _generation: Arc<Generation>,
    _admission: Admission,
}

#[derive(Clone, Copy, Debug)]
enum Case {
    Files,
    Directories,
    Link,
    Rename,
}

fn isolated_case(name: &str, case: Case) {
    const CHILD: &str = "WASMTIME_FACTORY_EXPERIMENT_CHILD";
    if std::env::var(CHILD).as_deref() != Ok(name) {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", name, "--nocapture", "--test-threads=1"])
            .env(CHILD, name)
            .status()
            .unwrap();
        assert!(status.success(), "isolated ownership check failed");
        return;
    }
    // One blocking worker makes the final queued task a completion fence.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    runtime.block_on(two_operand_case(case));
}

async fn two_operand_case(case: Case) {
    let directories: [tempfile::TempDir; 2] = std::array::from_fn(|_| {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("payload"), b"independent generation").unwrap();
        dir
    });
    let paths = directories.each_ref().map(|dir| dir.path().to_owned());
    let descriptors = directories.each_ref().map(|dir| {
        if !matches!(case, Case::Files) {
            Descriptor::Dir(Dir::new(
                primitives::open_ambient_dir(dir.path()).unwrap(),
                FsPerms::ReadWrite,
                OpenMode::READ,
                false,
            ))
        } else {
            Descriptor::File(File::new(
                std::fs::File::open(dir.path().join("payload")).unwrap(),
                FsPerms::ReadOnly,
                OpenMode::READ,
                false,
            ))
        }
    });
    let probes: [Arc<NativeProbe>; 2] = descriptors.each_ref().map(|descriptor| {
        // Compile-time check of both public native-field types.
        let native: &Arc<std::fs::File> = match descriptor {
            Descriptor::File(file) => &file.file,
            Descriptor::Dir(dir) => &dir.dir,
        };
        let weak = Arc::downgrade(native);
        Arc::new(NativeProbe {
            fd: native.as_raw_fd(),
            alive: Box::new(move || weak.upgrade().is_some_and(|file| file.metadata().is_ok())),
            generation_drops: AtomicUsize::new(0),
            hook_calls: AtomicUsize::new(0),
        })
    });
    let admission = Arc::new(AdmissionState {
        live: AtomicUsize::new(0),
        dropped: AtomicUsize::new(0),
        worker_unwound: std::sync::atomic::AtomicBool::new(false),
        native_closed_at_drop: Mutex::new(None),
    });
    let (cleanup_tx, cleanup_rx) = mpsc::channel();
    let residency: Vec<_> = directories
        .into_iter()
        .enumerate()
        .map(|(operand, directory)| {
            Arc::new(Generation {
                directory: Some(directory),
                operand,
                probe: probes[operand].clone(),
                admission: admission.clone(),
                cleanup: cleanup_tx.clone(),
            })
        })
        .collect();
    drop(cleanup_tx);
    let mut table = ResourceTable::new();
    let resources: Vec<_> = descriptors
        .into_iter()
        .enumerate()
        .map(|(operand, descriptor)| {
            let generation = residency[operand].clone();
            let state = admission.clone();
            let native = probes.clone();
            let mut attachment = WasiFilesystemCtx::default();
            attachment.set_operation_hook(move || {
                native[operand].hook_calls.fetch_add(1, Ordering::SeqCst);
                state.live.fetch_add(1, Ordering::SeqCst);
                Ok(FilesystemOperation::new(OperationValue {
                    _generation: generation.clone(),
                    _admission: Admission {
                        state: state.clone(),
                        native: native.clone(),
                    },
                }))
            });
            // The temporary context is dropped here, not retained in the Store.
            attachment.push_descriptor(&mut table, descriptor).unwrap()
        })
        .collect();
    let store = wasmtime::Store::new(&wasmtime::Engine::default(), table);
    let source = store.data().get(&resources[0]).unwrap();
    let target = store.data().get(&resources[1]).unwrap();
    let gate = InstalledNativeWork::new(match source {
        Descriptor::File(file) => &file.file,
        Descriptor::Dir(dir) => &dir.dir,
    });
    let mut future = Box::pin(async {
        match case {
            Case::Files | Case::Directories => {
                source.is_same_object(target).await.unwrap();
            }
            Case::Link | Case::Rename => {
                let source = source
                    .dir()
                    .unwrap_or_else(|_| panic!("expected directory"));
                let target = target
                    .dir()
                    .unwrap_or_else(|_| panic!("expected directory"));
                if matches!(case, Case::Link) {
                    source
                        .link_at(
                            PathFlags::empty(),
                            "payload".into(),
                            target,
                            "target".into(),
                        )
                        .await
                } else {
                    source
                        .rename_at("payload".into(), target, "target".into())
                        .await
                }
                .unwrap_or_else(|_| panic!("directory operation failed"));
            }
        }
    });
    assert!(
        future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    gate.started.recv_timeout(Duration::from_secs(30)).unwrap();
    // The real worker reached its gate after capturing both operands.
    drop(residency);
    drop(future);
    drop(store); // ResourceTable and both guest-owned descriptors die here.
    drop(resources);

    let generation_drops = probes
        .each_ref()
        .map(|p| p.generation_drops.load(Ordering::SeqCst));
    let native_alive = probes.each_ref().map(|p| (p.alive)());
    let hook_calls = probes
        .each_ref()
        .map(|p| p.hook_calls.load(Ordering::SeqCst));
    let live_during_work = admission.live.load(Ordering::SeqCst);
    let paths_during_work = paths.each_ref().map(|path| path.join("payload").exists());
    eprintln!(
        "GATED case={case:?}: generation_drops={generation_drops:?}, native_valid={native_alive:?}, hook_calls={hook_calls:?}, admission_live={live_during_work}, paths={paths_during_work:?}"
    );

    drop(gate);
    // FIFO execution on the one-worker blocking pool acknowledges that the
    // operation has returned and its cancelled join result was disposed.
    // Its result is inert; it never owns a native handle or generation.
    tokio::task::spawn_blocking(|| ()).await.unwrap();
    let mut drops = Vec::new();
    for _ in 0..2 {
        drops.push(cleanup_rx.recv_timeout(Duration::from_secs(30)).unwrap());
    }
    for observation in &drops {
        eprintln!(
            "GENERATION_DROP operand={}: native_closed={}, admission_live={}",
            observation.operand, observation.native_closed, observation.admission_live
        );
    }
    let closed_at_admission = *admission.native_closed_at_drop.lock().unwrap();
    let worker_unwound = admission.worker_unwound.load(Ordering::SeqCst);
    eprintln!(
        "WORKER_RETURNED: admission_drops={}, admission_live={}, native_closed_at_admission={closed_at_admission:?}, worker_unwound={worker_unwound}",
        admission.dropped.load(Ordering::SeqCst),
        admission.live.load(Ordering::SeqCst)
    );
    let closed_at_generation = drops.iter().all(|drop| drop.native_closed);
    let b_dropped_under_a = drops
        .iter()
        .any(|drop| drop.operand == 1 && drop.admission_live == 1);
    if matches!(case, Case::Link | Case::Rename) {
        assert_eq!(
            std::fs::read(paths[1].join("target")).unwrap(),
            b"independent generation"
        );
        assert_eq!(
            paths[0].join("payload").exists(),
            matches!(case, Case::Link)
        );
    }
    for observation in drops {
        observation.directory.close().unwrap();
    }
    assert!(paths.iter().all(|path| !path.exists()));
    eprintln!("CLEANUP_ACK: both generation paths removed");

    // Keep the same assertions for the raw-secondary negative control. Defer
    // assertion failure until the gate, worker and cleanup have all completed.
    assert_eq!(
        generation_drops,
        [0, 0],
        "generation released while native worker was gated"
    );
    assert_eq!(native_alive, [true, true]);
    assert_eq!(hook_calls, [1, 0], "only A may admit work");
    assert_eq!(live_during_work, 1);
    assert_eq!(paths_during_work, [true, true]);
    assert!(
        closed_at_generation,
        "native close must precede generation release"
    );
    assert!(
        b_dropped_under_a,
        "B releases independently while A is still admitted"
    );
    assert_eq!(closed_at_admission, Some([true, true]));
    assert!(
        !worker_unwound,
        "a detached panic is not normal native completion"
    );
    assert_eq!(admission.live.load(Ordering::SeqCst), 0);
    assert_eq!(admission.dropped.load(Ordering::SeqCst), 1);
    assert_eq!(
        probes
            .each_ref()
            .map(|p| p.generation_drops.load(Ordering::SeqCst)),
        [1, 1]
    );
    assert!(probes.iter().all(|p| !(p.alive)()));
}

#[test]
fn two_operand_files() {
    isolated_case(
        "filesystem::native_owner_tests::factory_experiment::two_operand_files",
        Case::Files,
    );
}

#[test]
fn two_operand_directories() {
    isolated_case(
        "filesystem::native_owner_tests::factory_experiment::two_operand_directories",
        Case::Directories,
    );
}

#[test]
fn link_retains_both_directories() {
    isolated_case(
        "filesystem::native_owner_tests::factory_experiment::link_retains_both_directories",
        Case::Link,
    );
}

#[test]
fn rename_retains_both_directories() {
    isolated_case(
        "filesystem::native_owner_tests::factory_experiment::rename_retains_both_directories",
        Case::Rename,
    );
}
