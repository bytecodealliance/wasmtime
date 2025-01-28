//! This example demonstrates how wasmtime-wasi-io can be used in a #![no_std]
//! target as the basis for a WASI implementation.
//!
//! This example can execute a wasi:cli/command component on a custom async
//! executor with no dependencies on the environment: execution is
//! deterministic, and no sources of input are provided to the component. The
//! WASI implementation is deliberately limited and incomplete, and many WASI
//! components will not even instantiate, or execute correctly, because this
//! is not a fully fleshed-out example.
//!
//! The wasmtime-wasi implementation of WASI depends on the tokio executor,
//! cap-std family of crates, and others to provide a complete implementation
//! of WASI p2 on top of Unix-based and Windows operating systems. It would be
//! difficult and/or inappropriate to port to other settings. This example
//! might be a good starting point for how to go about rolling your own WASI
//! implementation that is particular to your own execution environment.
//!
//! The wasmtime-wasi-io crate, which is a key part of this example, provides
//! an implementation of the wasi:io package, which is the foundation of
//! WASIp2. wasmtime-wasi-io provides the Pollable, InputStream, and
//! OutputStream traits, and this example shows implementations of those
//! traits for this particular embedding.

use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use anyhow::{bail, Result};
use core::cell::{Cell, RefCell};
use core::fmt::Write as _;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};
use wasmtime::component::{Component, Linker, Resource, ResourceTable};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi_io::{
    bytes::Bytes,
    poll::{subscribe, DynPollable, Pollable},
    streams::{DynInputStream, DynOutputStream, InputStream, OutputStream},
    IoView,
};

/// Unlike super::run, its nice to provide some sort of output showing what the
/// wasi program did while it executed, so this function reports in out_buf
/// what stdout/stderr prints occured on success (returns 0), or the error
/// message on failure (returns != 0).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn run_wasi(
    out_buf: *mut u8,
    out_size: *mut usize,
    wasi_component: *const u8,
    wasi_component_size: usize,
) -> usize {
    let buf = core::slice::from_raw_parts_mut(out_buf, *out_size);
    let wasi_component = core::slice::from_raw_parts(wasi_component, wasi_component_size);
    match run(wasi_component) {
        Ok(output) => {
            let len = buf.len().min(output.len());
            buf[..len].copy_from_slice(&output.as_bytes()[..len]);
            *out_size = len;
            return 0;
        }
        Err(e) => {
            let msg = format!("{e:?}");
            let len = buf.len().min(msg.len());
            buf[..len].copy_from_slice(&msg.as_bytes()[..len]);
            *out_size = len;
            return 1;
        }
    }
}

fn run(wasi_component: &[u8]) -> Result<String> {
    // wasmtime-wasi-io requires an async store, because the wasi:io/poll
    // interface will poll as Pending while execution is suspended and it is
    // waiting for a Pollable to become Ready. This example provides a very
    // small async executor which is entered below with `block_on`.
    let mut config = Config::default();
    config.async_support(true);
    // For future: we could consider turning on fuel in the Config to meter
    // how long a wasm guest could execute for.
    let engine = Engine::new(&config)?;

    // Like with modules, we deserialize components into native code:
    let component = match deserialize(&engine, wasi_component)? {
        Some(c) => c,
        None => return Ok("cannot load native code - requires virtual memory".to_string()),
    };

    // Linker provides wasmtime-wasi-io's implementation of wasi:io package,
    // and a number of other wasi interfaces implemented below as part of this
    // example.
    let mut linker = Linker::new(&engine);
    wasmtime_wasi_io::add_to_linker_async(&mut linker)?;
    add_to_linker_async(&mut linker)?;

    // Ensure all imports of the component are satisfied by the linker:
    let instance_pre = linker.instantiate_pre(&component)?;
    // Ensure the exports of the component provide the Command world:
    let command_pre = CommandPre::new(instance_pre)?;

    // Executor and WasiCtx share the same clock:
    let clock = Clock::new();

    // Use our custom executor to run some async code here:
    block_on(clock.clone(), async move {
        let ctx = ExampleCtx {
            table: ResourceTable::new(),
            clock,
            stdout: WriteLog::new(),
            stderr: WriteLog::new(),
        };
        let mut store = Store::new(&engine, ctx);
        // instantiate runs the wasm `start` section of
        let instance = command_pre.instantiate_async(&mut store).await?;
        instance
            .wasi_cli_run()
            .call_run(&mut store)
            .await?
            .map_err(|()| anyhow::anyhow!("wasi cli run returned error"))?;

        store.into_data().output()
    })
}

fn deserialize(engine: &Engine, component: &[u8]) -> Result<Option<Component>> {
    match unsafe { Component::deserialize(engine, component) } {
        Ok(component) => Ok(Some(component)),
        Err(e) => {
            // Currently if custom signals/virtual memory are disabled then this
            // example is expected to fail to load since loading native code
            // requires virtual memory. In the future this will go away as when
            // signals-based-traps is disabled then that means that the
            // interpreter should be used which should work here.
            if !cfg!(feature = "custom")
                && e.to_string()
                    .contains("requires virtual memory to be enabled")
            {
                Ok(None)
            } else {
                Err(e)
            }
        }
    }
}

// Generate bindings for the entire wasi:cli command world. We won't impl and
// link with all of these generated bindings for the sake of this example.
wasmtime::component::bindgen!({
    path: "../../../crates/wasi/wit",
    world: "wasi:cli/command",
    async: { only_imports: [] },
    trappable_imports: true,
    // Important: tell bindgen that anywhere it encounters the wasi:io
    // package, refer to the bindings generated in the wasmtime_wasi_io crate.
    // This way, all uses of the streams and pollable in the bindings in this
    // file match with the resource types (DynInputStream, DynOutputStream,
    // DynPollable) we use from the wasmtime_wasi_io crate.
    with: {
        "wasi:io": wasmtime_wasi_io::bindings::wasi::io,
    }
});

/// A Ctx struct particular to this example. In library code designed to be
/// reused and extended, this might be called a WasiCtx and not include a
/// ResourceTable as a member, but for the sake of this example, we put
/// everything that the bind
pub struct ExampleCtx {
    table: ResourceTable,
    clock: Clock,
    stdout: WriteLog,
    stderr: WriteLog,
}

// Provide an IoView impl in order to satisfy
// wasmtime_wasi_io::add_to_linker_async.
impl IoView for ExampleCtx {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

impl ExampleCtx {
    // Collect all of the output written to stdout and stderr into a simple
    // human-readable string, to be written to out_buf from run_wasi on
    // success. Lossy utf8 conversion because this is an example.
    fn output(&self) -> Result<String> {
        let mut out = String::new();
        let stdout = self.stdout.log.borrow();
        if !stdout.is_empty() {
            write!(&mut out, "stdout:\n")?;
            for chunk in stdout.iter() {
                write!(&mut out, "{}", String::from_utf8_lossy(chunk))?;
            }
        }
        let stderr = self.stderr.log.borrow();
        if !stderr.is_empty() {
            write!(&mut out, "stderr:\n")?;
            for chunk in stderr.iter() {
                write!(&mut out, "{}", String::from_utf8_lossy(chunk))?;
            }
        }
        Ok(out)
    }
}

// Add the minimum number of wasi interfaces to the Linker to instantiate the
// example application. This does not provide support for the entire
// wasi:cli/command world. Many of these impls are bare-bones and some are
// intentionally broken, see notes below.
pub fn add_to_linker_async(linker: &mut Linker<ExampleCtx>) -> Result<()> {
    wasi::clocks::monotonic_clock::add_to_linker(linker, |t| t)?;
    wasi::clocks::wall_clock::add_to_linker(linker, |t| t)?;
    wasi::cli::environment::add_to_linker(linker, |t| t)?;
    wasi::cli::exit::add_to_linker(linker, &wasi::cli::exit::LinkOptions::default(), |t| t)?;
    wasi::cli::stdin::add_to_linker(linker, |t| t)?;
    wasi::cli::stdout::add_to_linker(linker, |t| t)?;
    wasi::cli::stderr::add_to_linker(linker, |t| t)?;
    wasi::random::random::add_to_linker(linker, |t| t)?;
    wasi::filesystem::preopens::add_to_linker(linker, |t| t)?;
    wasi::filesystem::types::add_to_linker(linker, |t| t)?;
    Ok(())
}

// WasiCtx and the Executor need to share a single clock, so make it reference
// counted.
#[derive(Clone)]
struct Clock(Rc<Cell<u64>>);
impl Clock {
    fn new() -> Self {
        Clock(Rc::new(Cell::new(0)))
    }
    fn get(&self) -> u64 {
        self.0.get()
    }
    fn set(&self, to: u64) {
        self.0.set(to)
    }
    fn timer(&self, due: u64) -> Deadline {
        Deadline {
            clock: self.clone(),
            due,
        }
    }
}
// SAFETY: only will consume this crate in single-threaded environment
unsafe impl Send for Clock {}
unsafe impl Sync for Clock {}

// A Deadline is used to implement the monotonic clock's pollable. It is a
// future which is ready when the clock reaches the due time.
#[derive(Clone)]
struct Deadline {
    clock: Clock,
    due: u64,
}
impl Future for Deadline {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let now = self.clock.get();
        if now < self.due {
            Executor::current().push_deadline(self.due, cx.waker().clone());
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}
#[wasmtime_wasi_io::async_trait]
impl Pollable for Deadline {
    async fn ready(&mut self) {
        self.clone().await
    }
}

// An input-stream which is never ready for reading is used to implement
// stdin.
struct NeverReadable;
#[wasmtime_wasi_io::async_trait]
impl Pollable for NeverReadable {
    async fn ready(&mut self) {
        struct Pending;
        impl Future for Pending {
            type Output = ();
            fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
                Poll::Pending
            }
        }
        Pending.await
    }
}
impl InputStream for NeverReadable {
    fn read(&mut self, _: usize) -> wasmtime_wasi_io::streams::StreamResult<Bytes> {
        unreachable!("never ready for reading")
    }
}

// WriteLog is used implement stdout and stderr. Clonable because wasi:cli
// requires, when calling get_stdout/get_stderr multiple times, to provide
// distinct resources that point to the same underlying stream. RefCell
// provides mutation, and VecDeque provides O(1) push_back operation.
#[derive(Clone)]
struct WriteLog {
    log: Rc<RefCell<VecDeque<Bytes>>>,
}
impl WriteLog {
    fn new() -> Self {
        Self {
            log: Rc::new(RefCell::new(VecDeque::new())),
        }
    }
}
// SAFETY: only will consume this crate in single-threaded environment
unsafe impl Send for WriteLog {}
unsafe impl Sync for WriteLog {}

impl OutputStream for WriteLog {
    fn check_write(&mut self) -> wasmtime_wasi_io::streams::StreamResult<usize> {
        Ok(usize::MAX)
    }
    fn write(&mut self, contents: Bytes) -> wasmtime_wasi_io::streams::StreamResult<()> {
        self.log.borrow_mut().push_back(contents);
        Ok(())
    }
    fn flush(&mut self) -> wasmtime_wasi_io::streams::StreamResult<()> {
        Ok(())
    }
}
#[wasmtime_wasi_io::async_trait]
impl Pollable for WriteLog {
    async fn ready(&mut self) {
        // always ready - return immediately.
    }
}

// Global symbol (no thread local storage on this target) provides ability for
// Future impls to tell the Executor what they are waiting on.
static EXECUTOR: ExecutorGlobal = ExecutorGlobal::new();

// RefCell for mutation, Option so the Executor can be present only for life
// of the block_on call.
struct ExecutorGlobal(RefCell<Option<Executor>>);
impl ExecutorGlobal {
    const fn new() -> Self {
        ExecutorGlobal(RefCell::new(None))
    }
}
// SAFETY: only will consume this crate in single-threaded environment
unsafe impl Send for ExecutorGlobal {}
unsafe impl Sync for ExecutorGlobal {}

// Rc because executor and global both need to hold a reference, and makes it
// convenient to implement current(). RefCell for mutation.
struct Executor(Rc<RefCell<ExecutorInner>>);

impl Executor {
    pub fn new() -> Self {
        Executor(Rc::new(RefCell::new(ExecutorInner {
            schedule: Vec::new(),
        })))
    }
    pub fn current() -> Self {
        Executor(
            EXECUTOR
                .0
                .borrow_mut()
                .as_ref()
                .expect("Executor::current must be called within block_on")
                .0
                .clone(),
        )
    }
    pub fn push_deadline(&mut self, due: u64, waker: Waker) {
        self.0.borrow_mut().schedule.push((due, waker))
    }
}

// Schedule, as provided by the Deadline future impls. Map of due times to
// wakers.
struct ExecutorInner {
    schedule: Vec<(u64, Waker)>,
}

impl ExecutorInner {
    // Get the earliest deadline currently waiting. None if there are no
    // deadlines.
    fn earliest_deadline(&self) -> Option<u64> {
        self.schedule.iter().map(|(due, _)| due).min().copied()
    }
    // Return all wakers associated with deadlines before or equal to the
    // current clock time. Removes the wakers and their deadline from the
    // schedule.
    fn ready_deadlines(&mut self, now: u64) -> Vec<Waker> {
        let mut i = 0;
        let mut wakers = Vec::new();
        // This is basically https://doc.rust-lang.org/std/vec/struct.Vec.html#method.extract_if,
        // which is unstable
        while i < self.schedule.len() {
            if let Some((due, _)) = self.schedule.get(i) {
                if *due <= now {
                    let (_, waker) = self.schedule.remove(i);
                    wakers.push(waker);
                } else {
                    i += 1;
                }
            } else {
                break;
            }
        }
        wakers
    }
}

// Yanked from core::task::wake, which is unfortunately still unstable :/
fn noop_waker() -> Waker {
    use core::task::{RawWaker, RawWakerVTable};
    const VTABLE: RawWakerVTable = RawWakerVTable::new(
        // Cloning just returns a new no-op raw waker
        |_| RAW,
        // `wake` does nothing
        |_| {},
        // `wake_by_ref` does nothing
        |_| {},
        // Dropping does nothing as we don't allocate anything
        |_| {},
    );
    const RAW: RawWaker = RawWaker::new(core::ptr::null(), &VTABLE);

    unsafe { Waker::from_raw(RAW) }
}

fn block_on<R>(clock: Clock, f: impl Future<Output = Result<R>> + Send + 'static) -> Result<R> {
    // Guard against nested invocations
    if EXECUTOR.0.borrow_mut().is_some() {
        panic!("cannot block_on while executor is running!")
    }
    let executor = Executor::new();
    *EXECUTOR.0.borrow_mut() = Some(Executor(executor.0.clone()));

    // No special waker needed for this executor.
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);
    let mut f = core::pin::pin!(f);

    // Drive the Future to completion in the following loop
    let r = 'outer: loop {
        // Arbitrary. Could be as little as 1. There's no fuel-based async
        // yielding in this example so repeated polls is probably not making
        // progress without "providing input" from the outside environment,
        // below.
        const POLLS_PER_CLOCK: usize = 200;
        for _ in 0..POLLS_PER_CLOCK {
            match f.as_mut().poll(&mut cx) {
                Poll::Pending => {}
                Poll::Ready(r) => break 'outer r,
            }
        }

        // This is where a non-example executor would wait for input from the
        // "outside world". This example checks if the schedule indicates the
        // guest is waiting on some future deadline and fast-forwards time
        // until then, because no other input is possible in this example.
        if let Some(sleep_until) = executor.0.borrow().earliest_deadline() {
            clock.set(sleep_until);
        } else {
            clock.set(clock.get() + 1);
        }

        // Any wakers which are ready can be waked now.
        for waker in executor.0.borrow_mut().ready_deadlines(clock.get()) {
            waker.wake()
        }
    };

    // Clean up guard for nested invocations
    let _ = EXECUTOR
        .0
        .borrow_mut()
        .take()
        .expect("executor vacated global while running");
    r
}

// -------------- impls for the bindgen! Host traits ------------------
// These impls are written directly for WasiCtx, which is fine because this
// example isn't trying to create reusable library code.

impl wasi::clocks::monotonic_clock::Host for ExampleCtx {
    fn now(&mut self) -> Result<wasi::clocks::monotonic_clock::Instant> {
        Ok(self.clock.get())
    }
    fn resolution(&mut self) -> Result<wasi::clocks::monotonic_clock::Duration> {
        Ok(1)
    }
    fn subscribe_duration(
        &mut self,
        duration: wasi::clocks::monotonic_clock::Duration,
    ) -> Result<Resource<DynPollable>> {
        self.subscribe_instant(self.clock.get() + duration)
    }
    fn subscribe_instant(
        &mut self,
        deadline: wasi::clocks::monotonic_clock::Instant,
    ) -> Result<Resource<DynPollable>> {
        let timer = self.clock.timer(deadline);
        let deadline = self.table().push(timer)?;
        Ok(subscribe(self.table(), deadline)?)
    }
}

impl wasi::clocks::wall_clock::Host for ExampleCtx {
    fn now(&mut self) -> Result<wasi::clocks::wall_clock::Datetime> {
        // A bogus time. This datetime is relative to the unix epoch. Just
        // reuse the monotonic time for the sake of the example.
        let now = self.clock.get();
        let seconds = now / 1_000_000_000;
        let nanoseconds = (now - (seconds * 1_000_000_000)) as u32;
        Ok(wasi::clocks::wall_clock::Datetime {
            seconds,
            nanoseconds,
        })
    }
    fn resolution(&mut self) -> Result<wasi::clocks::wall_clock::Datetime> {
        Ok(wasi::clocks::wall_clock::Datetime {
            seconds: 0,
            nanoseconds: 1,
        })
    }
}

// No arguments, environment variables, or cwd are provided.
impl wasi::cli::environment::Host for ExampleCtx {
    fn get_arguments(&mut self) -> Result<Vec<String>> {
        Ok(Vec::new())
    }
    fn get_environment(&mut self) -> Result<Vec<(String, String)>> {
        Ok(Vec::new())
    }
    fn initial_cwd(&mut self) -> Result<Option<String>> {
        Ok(None)
    }
}

// Ideally this would follow the example in wasmtime-wasi: make a struct, impl
// Error on it, and try downcasting to it at the call_run site to see if the
// wasi:cli/exit was used to exit with success without unwinding - valid but
// uncommon behavior that should be treated the same as returning ok from the
// wasi:cli/run.run function. Our example program doesn't exit that way.
impl wasi::cli::exit::Host for ExampleCtx {
    fn exit(&mut self, code: Result<(), ()>) -> Result<()> {
        if code.is_ok() {
            bail!("wasi exit success")
        } else {
            bail!("wasi exit error")
        }
    }
    // This is feature-flagged (unstable) in the wits. Per the LinkOptions
    // passed to the wasi::cli::exit::add_to_linker, it won't be found in
    // any guest code.
    fn exit_with_code(&mut self, _: u8) -> Result<()> {
        unreachable!("this unstable func is not added to the linker");
    }
}

impl wasi::cli::stdin::Host for ExampleCtx {
    fn get_stdin(&mut self) -> Result<Resource<DynInputStream>> {
        let stdin: DynInputStream = Box::new(NeverReadable);
        Ok(self.table().push(stdin)?)
    }
}

impl wasi::cli::stdout::Host for ExampleCtx {
    fn get_stdout(&mut self) -> Result<Resource<DynOutputStream>> {
        let stdout: DynOutputStream = Box::new(self.stdout.clone());
        Ok(self.table().push(stdout)?)
    }
}

impl wasi::cli::stderr::Host for ExampleCtx {
    fn get_stderr(&mut self) -> Result<Resource<DynOutputStream>> {
        let stderr: DynOutputStream = Box::new(self.stderr.clone());
        Ok(self.table().push(stderr)?)
    }
}

// This is obviously bogus and breaks the guarantees given by this interface.
// In a real embedding, provide a high quality source of randomness here.
impl wasi::random::random::Host for ExampleCtx {
    fn get_random_bytes(&mut self, len: u64) -> Result<Vec<u8>> {
        let mut vec = Vec::new();
        vec.resize(len as usize, 0u8);
        Ok(vec)
    }
    fn get_random_u64(&mut self) -> Result<u64> {
        Ok(0)
    }
}

// The preopens are the only place the filesystem is provided a Descriptor,
// from which to try open_at to get more Descriptors. If we don't provide
// anything here, none of the methods on Descriptor will ever be reachable,
// because Resources are unforgable (the runtime will trap bogus indexes).
impl wasi::filesystem::preopens::Host for ExampleCtx {
    fn get_directories(
        &mut self,
    ) -> Result<Vec<(Resource<wasi::filesystem::types::Descriptor>, String)>> {
        // Never construct a Descriptor, so all of the bails in the rest of Filesystem should be
        // unreachable.
        Ok(Vec::new())
    }
}

// This impl is completely empty!
impl wasi::filesystem::types::HostDescriptor for ExampleCtx {
    fn read_via_stream(
        &mut self,
        _: Resource<wasi::filesystem::types::Descriptor>,
        _: u64,
    ) -> Result<Result<Resource<DynInputStream>, wasi::filesystem::types::ErrorCode>> {
        unreachable!("no filesystem")
    }
    fn write_via_stream(
        &mut self,
        _: Resource<wasi::filesystem::types::Descriptor>,
        _: u64,
    ) -> Result<Result<Resource<DynOutputStream>, wasi::filesystem::types::ErrorCode>> {
        unreachable!("no filesystem")
    }
    fn append_via_stream(
        &mut self,
        _: Resource<wasi::filesystem::types::Descriptor>,
    ) -> Result<Result<Resource<DynOutputStream>, wasi::filesystem::types::ErrorCode>> {
        unreachable!("no filesystem")
    }
    fn advise(
        &mut self,
        _: Resource<wasi::filesystem::types::Descriptor>,
        _: u64,
        _: u64,
        _: wasi::filesystem::types::Advice,
    ) -> Result<Result<(), wasi::filesystem::types::ErrorCode>> {
        unreachable!("no filesystem")
    }
    fn sync_data(
        &mut self,
        _: Resource<wasi::filesystem::types::Descriptor>,
    ) -> Result<Result<(), wasi::filesystem::types::ErrorCode>> {
        unreachable!("no filesystem")
    }
    fn get_flags(
        &mut self,
        _: Resource<wasi::filesystem::types::Descriptor>,
    ) -> Result<Result<wasi::filesystem::types::DescriptorFlags, wasi::filesystem::types::ErrorCode>>
    {
        unreachable!("no filesystem")
    }
    fn get_type(
        &mut self,
        _: Resource<wasi::filesystem::types::Descriptor>,
    ) -> Result<Result<wasi::filesystem::types::DescriptorType, wasi::filesystem::types::ErrorCode>>
    {
        unreachable!("no filesystem")
    }
    fn set_size(
        &mut self,
        _: Resource<wasi::filesystem::types::Descriptor>,
        _: u64,
    ) -> Result<Result<(), wasi::filesystem::types::ErrorCode>> {
        unreachable!("no filesystem")
    }
    fn set_times(
        &mut self,
        _: Resource<wasi::filesystem::types::Descriptor>,
        _: wasi::filesystem::types::NewTimestamp,
        _: wasi::filesystem::types::NewTimestamp,
    ) -> Result<Result<(), wasi::filesystem::types::ErrorCode>> {
        unreachable!("no filesystem")
    }
    fn read(
        &mut self,
        _: Resource<wasi::filesystem::types::Descriptor>,
        _: u64,
        _: u64,
    ) -> Result<Result<(Vec<u8>, bool), wasi::filesystem::types::ErrorCode>> {
        unreachable!("no filesystem")
    }
    fn write(
        &mut self,
        _: Resource<wasi::filesystem::types::Descriptor>,
        _: Vec<u8>,
        _: u64,
    ) -> Result<Result<u64, wasi::filesystem::types::ErrorCode>> {
        unreachable!("no filesystem")
    }

    fn read_directory(
        &mut self,
        _: Resource<wasi::filesystem::types::Descriptor>,
    ) -> Result<
        Result<
            Resource<wasi::filesystem::types::DirectoryEntryStream>,
            wasi::filesystem::types::ErrorCode,
        >,
    > {
        unreachable!("no filesystem")
    }
    fn sync(
        &mut self,
        _: Resource<wasi::filesystem::types::Descriptor>,
    ) -> Result<Result<(), wasi::filesystem::types::ErrorCode>> {
        unreachable!("no filesystem")
    }
    fn create_directory_at(
        &mut self,
        _: Resource<wasi::filesystem::types::Descriptor>,
        _: String,
    ) -> Result<Result<(), wasi::filesystem::types::ErrorCode>> {
        unreachable!("no filesystem")
    }
    fn stat(
        &mut self,
        _: Resource<wasi::filesystem::types::Descriptor>,
    ) -> Result<Result<wasi::filesystem::types::DescriptorStat, wasi::filesystem::types::ErrorCode>>
    {
        unreachable!("no filesystem")
    }
    fn stat_at(
        &mut self,
        _: Resource<wasi::filesystem::types::Descriptor>,
        _: wasi::filesystem::types::PathFlags,
        _: String,
    ) -> Result<Result<wasi::filesystem::types::DescriptorStat, wasi::filesystem::types::ErrorCode>>
    {
        unreachable!("no filesystem")
    }
    fn set_times_at(
        &mut self,
        _: Resource<wasi::filesystem::types::Descriptor>,
        _: wasi::filesystem::types::PathFlags,
        _: String,
        _: wasi::filesystem::types::NewTimestamp,
        _: wasi::filesystem::types::NewTimestamp,
    ) -> Result<Result<(), wasi::filesystem::types::ErrorCode>> {
        unreachable!("no filesystem")
    }
    fn link_at(
        &mut self,
        _: Resource<wasi::filesystem::types::Descriptor>,
        _: wasi::filesystem::types::PathFlags,
        _: String,
        _: Resource<wasi::filesystem::types::Descriptor>,
        _: String,
    ) -> Result<Result<(), wasi::filesystem::types::ErrorCode>> {
        unreachable!("no filesystem")
    }
    fn open_at(
        &mut self,
        _: Resource<wasi::filesystem::types::Descriptor>,
        _: wasi::filesystem::types::PathFlags,
        _: String,
        _: wasi::filesystem::types::OpenFlags,
        _: wasi::filesystem::types::DescriptorFlags,
    ) -> Result<
        Result<Resource<wasi::filesystem::types::Descriptor>, wasi::filesystem::types::ErrorCode>,
    > {
        unreachable!("no filesystem")
    }
    fn readlink_at(
        &mut self,
        _: Resource<wasi::filesystem::types::Descriptor>,
        _: String,
    ) -> Result<Result<String, wasi::filesystem::types::ErrorCode>> {
        unreachable!("no filesystem")
    }
    fn remove_directory_at(
        &mut self,
        _: Resource<wasi::filesystem::types::Descriptor>,
        _: String,
    ) -> Result<Result<(), wasi::filesystem::types::ErrorCode>> {
        unreachable!("no filesystem")
    }
    fn rename_at(
        &mut self,
        _: Resource<wasi::filesystem::types::Descriptor>,
        _: String,
        _: Resource<wasi::filesystem::types::Descriptor>,
        _: String,
    ) -> Result<Result<(), wasi::filesystem::types::ErrorCode>> {
        unreachable!("no filesystem")
    }
    fn symlink_at(
        &mut self,
        _: Resource<wasi::filesystem::types::Descriptor>,
        _: String,
        _: String,
    ) -> Result<Result<(), wasi::filesystem::types::ErrorCode>> {
        unreachable!("no filesystem")
    }
    fn unlink_file_at(
        &mut self,
        _: Resource<wasi::filesystem::types::Descriptor>,
        _: String,
    ) -> Result<Result<(), wasi::filesystem::types::ErrorCode>> {
        unreachable!("no filesystem")
    }
    fn is_same_object(
        &mut self,
        _: Resource<wasi::filesystem::types::Descriptor>,
        _: Resource<wasi::filesystem::types::Descriptor>,
    ) -> Result<bool> {
        unreachable!("no filesystem")
    }
    fn metadata_hash(
        &mut self,
        _: Resource<wasi::filesystem::types::Descriptor>,
    ) -> Result<
        Result<wasi::filesystem::types::MetadataHashValue, wasi::filesystem::types::ErrorCode>,
    > {
        unreachable!("no filesystem")
    }
    fn metadata_hash_at(
        &mut self,
        _: Resource<wasi::filesystem::types::Descriptor>,
        _: wasi::filesystem::types::PathFlags,
        _: String,
    ) -> Result<
        Result<wasi::filesystem::types::MetadataHashValue, wasi::filesystem::types::ErrorCode>,
    > {
        unreachable!("no filesystem")
    }

    fn drop(&mut self, _: Resource<wasi::filesystem::types::Descriptor>) -> Result<()> {
        unreachable!("no filesystem")
    }
}
// Only place this resource can be created is with Descriptor::read_directory,
// so this will never be constructed either.
impl wasi::filesystem::types::HostDirectoryEntryStream for ExampleCtx {
    fn read_directory_entry(
        &mut self,
        _: Resource<wasi::filesystem::types::DirectoryEntryStream>,
    ) -> Result<
        Result<Option<wasi::filesystem::types::DirectoryEntry>, wasi::filesystem::types::ErrorCode>,
    > {
        unreachable!("no filesystem")
    }
    fn drop(&mut self, _: Resource<wasi::filesystem::types::DirectoryEntryStream>) -> Result<()> {
        unreachable!("no filesystem")
    }
}

// No stream is ever constructed from a Descriptor, there will never be a
// valid downcast of a stream error into a filesystem error-code.
impl wasi::filesystem::types::Host for ExampleCtx {
    fn filesystem_error_code(
        &mut self,
        _: Resource<wasmtime_wasi_io::streams::Error>,
    ) -> Result<Option<wasi::filesystem::types::ErrorCode>> {
        Ok(None)
    }
}
