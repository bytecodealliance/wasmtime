//! Deterministic one-shot worker gate shared by the ownership test modules.
//! Each module has its own address registry; guest identity matching stays in P3.

use std::collections::BTreeMap;
use std::sync::{Arc, Condvar, Mutex, OnceLock, mpsc};
use std::time::Duration;

pub(super) struct NativeGate {
    started: mpsc::Sender<()>,
    released: Mutex<bool>,
    wake: Condvar,
}

impl NativeGate {
    pub(super) fn new() -> (Arc<Self>, mpsc::Receiver<()>) {
        let (started, receiver) = mpsc::channel();
        let gate = Arc::new(Self {
            started,
            released: Mutex::new(false),
            wake: Condvar::new(),
        });
        (gate, receiver)
    }

    pub(super) fn block(&self) {
        let _ = self.started.send(());
        let mut released = self.released.lock().unwrap();
        while !*released {
            released = self.wake.wait(released).unwrap();
        }
    }

    pub(super) fn release(&self) {
        *self.released.lock().unwrap() = true;
        self.wake.notify_all();
    }
}

static GATES: OnceLock<Mutex<BTreeMap<usize, Arc<NativeGate>>>> = OnceLock::new();

pub(super) fn native_work_gate(file: &std::fs::File) -> bool {
    let gate = GATES
        .get_or_init(Default::default)
        .lock()
        .unwrap()
        .remove(&(file as *const std::fs::File as usize));
    if let Some(gate) = gate {
        gate.block();
        true
    } else {
        false
    }
}

pub(super) struct InstalledGate {
    key: usize,
    gate: Arc<NativeGate>,
    started: mpsc::Receiver<()>,
}

impl InstalledGate {
    pub(super) fn new(file: &std::fs::File) -> Self {
        let (gate, started) = NativeGate::new();
        let key = file as *const std::fs::File as usize;
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

    pub(super) fn wait_started(&self) {
        self.started.recv_timeout(Duration::from_secs(30)).unwrap();
    }

    pub(super) fn release(&self) {
        self.gate.release();
    }
}

impl Drop for InstalledGate {
    fn drop(&mut self) {
        self.release();
        GATES.get().unwrap().lock().unwrap().remove(&self.key);
    }
}
