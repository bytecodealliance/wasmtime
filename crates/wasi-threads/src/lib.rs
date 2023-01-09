//! Implement [`wasi-threads`].
//!
//! [`wasi-threads`]: https://github.com/WebAssembly/wasi-threads

use anyhow::{anyhow, Result};
use rand::Rng;
use std::thread;
use wasmtime::{Caller, Linker, Module, SharedMemory, Store};
use wasmtime_wasi::maybe_exit_on_error;

// This name is a function export designated by the wasi-threads specification:
// https://github.com/WebAssembly/wasi-threads/#detailed-design-discussion
const WASI_ENTRY_POINT: &str = "wasi_thread_start";

pub struct WasiThreadsCtx<T> {
    module: Module,
    linker: Linker<T>,
}

impl<T: Clone + Send + 'static> WasiThreadsCtx<T> {
    pub fn new(module: Module, linker: Linker<T>) -> Self {
        Self { module, linker }
    }

    pub fn spawn(&self, host: T, thread_start_arg: i32) -> Result<i32> {
        let module = self.module.clone();
        let linker = self.linker.clone();

        // Start a Rust thread running a new instance of the current module.
        let wasi_thread_id = random_thread_id();
        let builder = thread::Builder::new().name(format!("wasi-thread-{}", wasi_thread_id));
        builder.spawn(move || {
            // Convenience function for printing failures, since the `Thread`
            // has no way to report a failure to the outer context.
            let fail = |msg: String| {
                format!(
                    "wasi-thread-{} exited unsuccessfully: {}",
                    wasi_thread_id, msg
                )
            };

            // Each new instance is created in its own store.
            let mut store = Store::new(&module.engine(), host);
            let instance = linker
                .instantiate(&mut store, &module)
                .expect(&fail("failed to instantiate".into()));
            let thread_entry_point = instance
                .get_typed_func::<(i32, i32), ()>(&mut store, WASI_ENTRY_POINT)
                .expect(&fail(format!(
                    "failed to find wasi-threads entry point function: {}",
                    WASI_ENTRY_POINT
                )));

            // Start the thread's entry point; any failures are simply printed
            // before exiting the thread. It may be necessary to handle failures
            // here somehow, e.g., so that `pthread_join` can be notified if the
            // user function traps for some reason (TODO).
            log::trace!(
                "spawned thread id = {}; calling start function `{}` with: {}",
                wasi_thread_id,
                WASI_ENTRY_POINT,
                thread_start_arg
            );
            match thread_entry_point.call(&mut store, (wasi_thread_id, thread_start_arg)) {
                Ok(_) => {}
                Err(e) => {
                    log::trace!("exiting thread id = {} due to error", wasi_thread_id);
                    let e = maybe_exit_on_error(e);
                    eprintln!("Error: {:?}", e);
                    std::process::exit(1);
                }
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
) -> anyhow::Result<SharedMemory> {
    linker.func_wrap(
        "wasi",
        "thread_spawn",
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
    // memory import. This currently does not handle multiple memories (TODO).
    for import in module.imports() {
        if let Some(m) = import.ty().memory() {
            if m.is_shared() {
                let mem = SharedMemory::new(module.engine(), m.clone())?;
                linker.define(store, import.module(), import.name(), mem.clone())?;
                return Ok(mem);
            }
        }
    }
    Err(anyhow!(
        "unable to link a shared memory import to the module; a `wasi-threads` \
         module should import a single shared memory as \"memory\""
    ))
}
