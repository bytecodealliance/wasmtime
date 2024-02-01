//! Implement [`wasi-threads`].
//!
//! [`wasi-threads`]: https://github.com/WebAssembly/wasi-threads

use anyhow::{anyhow, Result};
use rand::Rng;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;
use std::thread;
use wasmtime::{Caller, ExternType, InstancePre, Linker, Module, SharedMemory, Store, ValType};
use wasmtime_wasi::maybe_exit_on_error;

// This name is a function export designated by the wasi-threads specification:
// https://github.com/WebAssembly/wasi-threads/#detailed-design-discussion
const WASI_ENTRY_POINT: &str = "wasi_thread_start";

pub struct WasiThreadsCtx<T> {
    instance_pre: Arc<InstancePre<T>>,
}

impl<T: Clone + Send + 'static> WasiThreadsCtx<T> {
    pub fn new(module: Module, linker: Arc<Linker<T>>) -> Result<Self> {
        let instance_pre = Arc::new(linker.instantiate_pre(&module)?);
        Ok(Self { instance_pre })
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

        // Start a Rust thread running a new instance of the current module.
        let wasi_thread_id = random_thread_id();
        let builder = thread::Builder::new().name(format!("wasi-thread-{}", wasi_thread_id));
        builder.spawn(move || {
            // Catch any panic failures in host code; e.g., if a WASI module
            // were to crash, we want all threads to exit, not just this one.
            let result = catch_unwind(AssertUnwindSafe(|| {
                // Each new instance is created in its own store.
                let mut store = Store::new(&instance_pre.module().engine(), host);
                let instance = instance_pre.instantiate(&mut store).unwrap();
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
                match thread_entry_point.call(&mut store, (wasi_thread_id, thread_start_arg)) {
                    Ok(_) => log::trace!("exiting thread id = {} normally", wasi_thread_id),
                    Err(e) => {
                        log::trace!("exiting thread id = {} due to error", wasi_thread_id);
                        let e = maybe_exit_on_error(e);
                        eprintln!("Error: {:?}", e);
                        std::process::exit(1);
                    }
                }
            }));

            if let Err(e) = result {
                eprintln!("wasi-thread-{} panicked: {:?}", wasi_thread_id, e);
                std::process::exit(1);
            }
        })?;

        Ok(wasi_thread_id)
    }
}

/// Helper for generating valid WASI thread IDs (TID).
///
/// Callers of `wasi_thread_spawn` expect a TID >=0 to indicate a successful
/// spawning of the thread whereas a negative return value indicates an
/// failure to spawn.
fn random_thread_id() -> i32 {
    let tid: u32 = rand::thread_rng().gen();
    (tid >> 1) as i32
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
                    assert!(thread_id >= 0, "thread_id = {}", thread_id);
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
    use ValType::*;
    match module.get_export(WASI_ENTRY_POINT) {
        Some(ExternType::Func(ty)) => ty.params().eq([I32, I32]) && ty.results().len() == 0,
        _ => false,
    }
}
