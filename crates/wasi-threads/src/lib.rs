//! Implement [`wasi-threads`].
//!
//! [`wasi-threads`]: https://github.com/WebAssembly/wasi-threads

use anyhow::{anyhow, Result};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::thread;
use wasmtime::{Caller, ExternType, InstancePre, Linker, Module, SharedMemory, Store};

// This name is a function export designated by the wasi-threads specification:
// https://github.com/WebAssembly/wasi-threads/#detailed-design-discussion
const WASI_ENTRY_POINT: &str = "wasi_thread_start";

pub struct WasiThreadsCtx<T> {
    instance_pre: Arc<InstancePre<T>>,
    tid: AtomicI32,
}

impl<T: Clone + Send + 'static> WasiThreadsCtx<T> {
    pub fn new(module: Module, linker: Arc<Linker<T>>) -> Result<Self> {
        let instance_pre = Arc::new(linker.instantiate_pre(&module)?);
        let tid = AtomicI32::new(0);
        Ok(Self { instance_pre, tid })
    }

    pub fn spawn(&self, host: T, thread_start_arg: i32) -> Result<i32> {
        let instance_pre = self.instance_pre.clone();

        // Check that the thread entry point is present. Why here? If we check
        // for this too early, then we cannot accept modules that do not have an
        // entry point but never spawn a thread. As pointed out in
        // https://github.com/bytecodealliance/wasmtime/issues/6153, checking
        // the entry point here allows wasi-threads to be compatible with more
        // modules.
        //
        // As defined in the wasi-threads specification, returning a negative
        // result here indicates to the guest module that the spawn failed.
        if !has_entry_point(instance_pre.module()) {
            log::error!("failed to find a wasi-threads entry point function; expected an export with name: {WASI_ENTRY_POINT}");
            return Ok(-1);
        }
        if !has_correct_signature(instance_pre.module()) {
            log::error!("the exported entry point function has an incorrect signature: expected `(i32, i32) -> ()`");
            return Ok(-1);
        }

        let wasi_thread_id = self.next_thread_id();
        if wasi_thread_id.is_none() {
            log::error!("ran out of valid thread IDs");
            return Ok(-1);
        }
        let wasi_thread_id = wasi_thread_id.unwrap();

        // Start a Rust thread running a new instance of the current module.
        let builder = thread::Builder::new().name(format!("wasi-thread-{wasi_thread_id}"));
        builder.spawn(move || {
            // Catch any panic failures in host code; e.g., if a WASI module
            // were to crash, we want all threads to exit, not just this one.
            let result = catch_unwind(AssertUnwindSafe(|| {
                // Each new instance is created in its own store.
                let mut store = Store::new(&instance_pre.module().engine(), host);

                let instance = if instance_pre.module().engine().is_async() {
                    wasmtime_wasi::runtime::in_tokio(instance_pre.instantiate_async(&mut store))
                } else {
                    instance_pre.instantiate(&mut store)
                }
                .unwrap();

                let thread_entry_point = instance
                    .get_typed_func::<(i32, i32), ()>(&mut store, WASI_ENTRY_POINT)
                    .unwrap();

                // Start the thread's entry point. Any traps or calls to
                // `proc_exit`, by specification, should end execution for all
                // threads. This code uses `process::exit` to do so, which is
                // what the user expects from the CLI but probably not in a
                // Wasmtime embedding.
                log::trace!(
                    "spawned thread id = {}; calling start function `{}` with: {}",
                    wasi_thread_id,
                    WASI_ENTRY_POINT,
                    thread_start_arg
                );
                let res = if instance_pre.module().engine().is_async() {
                    wasmtime_wasi::runtime::in_tokio(
                        thread_entry_point
                            .call_async(&mut store, (wasi_thread_id, thread_start_arg)),
                    )
                } else {
                    thread_entry_point.call(&mut store, (wasi_thread_id, thread_start_arg))
                };
                match res {
                    Ok(_) => log::trace!("exiting thread id = {} normally", wasi_thread_id),
                    Err(e) => {
                        log::trace!("exiting thread id = {} due to error", wasi_thread_id);
                        let e = wasi_common::maybe_exit_on_error(e);
                        eprintln!("Error: {e:?}");
                        std::process::exit(1);
                    }
                }
            }));

            if let Err(e) = result {
                eprintln!("wasi-thread-{wasi_thread_id} panicked: {e:?}");
                std::process::exit(1);
            }
        })?;

        Ok(wasi_thread_id)
    }

    /// Helper for generating valid WASI thread IDs (TID).
    ///
    /// Callers of `wasi_thread_spawn` expect a TID in range of 0 < TID <= 0x1FFFFFFF
    /// to indicate a successful spawning of the thread whereas a negative
    /// return value indicates an failure to spawn.
    fn next_thread_id(&self) -> Option<i32> {
        match self
            .tid
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| match v {
                ..=0x1ffffffe => Some(v + 1),
                _ => None,
            }) {
            Ok(v) => Some(v + 1),
            Err(_) => None,
        }
    }
}

/// Manually add the WASI `thread_spawn` function to the linker.
///
/// It is unclear what namespace the `wasi-threads` proposal should live under:
/// it is not clear if it should be included in any of the `preview*` releases
/// so for the time being its module namespace is simply `"wasi"` (TODO).
pub fn add_to_linker<T: Clone + Send + 'static>(
    linker: &mut wasmtime::Linker<T>,
    store: &wasmtime::Store<T>,
    module: &Module,
    get_cx: impl Fn(&mut T) -> &WasiThreadsCtx<T> + Send + Sync + Copy + 'static,
) -> anyhow::Result<()> {
    linker.func_wrap(
        "wasi",
        "thread-spawn",
        move |mut caller: Caller<'_, T>, start_arg: i32| -> i32 {
            log::trace!("new thread requested via `wasi::thread_spawn` call");
            let host = caller.data().clone();
            let ctx = get_cx(caller.data_mut());
            match ctx.spawn(host, start_arg) {
                Ok(thread_id) => {
                    assert!(thread_id >= 0, "thread_id = {thread_id}");
                    thread_id
                }
                Err(e) => {
                    log::error!("failed to spawn thread: {}", e);
                    -1
                }
            }
        },
    )?;

    // Find the shared memory import and satisfy it with a newly-created shared
    // memory import.
    for import in module.imports() {
        if let Some(m) = import.ty().memory() {
            if m.is_shared() {
                let mem = SharedMemory::new(module.engine(), m.clone())?;
                linker.define(store, import.module(), import.name(), mem.clone())?;
            } else {
                return Err(anyhow!(
                    "memory was not shared; a `wasi-threads` must import \
                     a shared memory as \"memory\""
                ));
            }
        }
    }
    Ok(())
}

/// Check if wasi-threads' `wasi_thread_start` export is present.
fn has_entry_point(module: &Module) -> bool {
    module.get_export(WASI_ENTRY_POINT).is_some()
}

/// Check if the entry function has the correct signature `(i32, i32) -> ()`.
fn has_correct_signature(module: &Module) -> bool {
    match module.get_export(WASI_ENTRY_POINT) {
        Some(ExternType::Func(ty)) => {
            ty.params().len() == 2
                && ty.params().nth(0).unwrap().is_i32()
                && ty.params().nth(1).unwrap().is_i32()
                && ty.results().len() == 0
        }
        _ => false,
    }
}
