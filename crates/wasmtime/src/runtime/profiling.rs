#[cfg(feature = "component-model")]
use crate::component::Component;
use crate::prelude::*;
use crate::runtime::vm::Backtrace;
use crate::{AsContext, CallHook, Module};
use core::cmp::Ordering;
use fxprof_processed_profile::debugid::DebugId;
use fxprof_processed_profile::{
    CategoryHandle, Frame, FrameFlags, FrameInfo, LibraryInfo, MarkerLocations, MarkerTiming,
    Profile, ReferenceTimestamp, StaticSchemaMarker, StaticSchemaMarkerField, StringHandle, Symbol,
    SymbolTable, Timestamp,
};
use std::ops::Range;
use std::sync::Arc;
use std::time::{Duration, Instant};
use wasmtime_environ::demangle_function_name_or_index;

// TODO: collect more data
// - On non-Windows, measure thread-local CPU usage between events with
//   rustix::time::clock_gettime(ClockId::ThreadCPUTime)
// - Report which wasm module, and maybe instance, each frame came from

/// Collects basic profiling data for a single WebAssembly guest.
///
/// This profiler can't provide measurements that are as accurate or detailed
/// as a platform-specific profiler, such as `perf` on Linux. On the other
/// hand, this profiler works on every platform that Wasmtime supports. Also,
/// as an embedder you can use this profiler selectively on individual guest
/// instances rather than profiling the entire process.
///
/// To use this, you'll need to arrange to call [`GuestProfiler::sample`] at
/// regular intervals while the guest is on the stack. The most straightforward
/// way to do that is to call it from a callback registered with
/// [`Store::epoch_deadline_callback()`](crate::Store::epoch_deadline_callback).
///
/// # Accuracy
///
/// The data collection granularity is limited by the mechanism you use to
/// interrupt guest execution and collect a profiling sample.
///
/// If you use epoch interruption, then samples will only be collected at
/// function entry points and loop headers. This introduces some bias to the
/// results. In addition, samples will only be taken at times when WebAssembly
/// functions are running, not during host-calls.
///
/// It is technically possible to use fuel interruption instead. That
/// introduces worse bias since samples occur after a certain number of
/// WebAssembly instructions, which can take different amounts of time.
///
/// You may instead be able to use platform-specific methods, such as
/// `setitimer(ITIMER_VIRTUAL, ...)` on POSIX-compliant systems, to sample on
/// a more accurate interval. The only current requirement is that the guest
/// you wish to profile must be on the same stack where you call `sample`,
/// and executing within the same thread. However, the `GuestProfiler::sample`
/// method is not currently async-signal-safe, so doing this correctly is not
/// easy.
///
/// # Security
///
/// Profiles produced using this profiler do not include any configuration
/// details from the host, such as virtual memory addresses, or from any
/// WebAssembly modules that you haven't specifically allowed. So for
/// example, these profiles should be safe to share with untrusted users
/// who have provided untrusted code that you are running in a multi-tenancy
/// environment.
///
/// However, the profile does include byte offsets into the text section of
/// the compiled module, revealing some information about the size of the code
/// generated for each module. For user-provided modules, the user could get
/// the same information by compiling the module for themself using a similar
/// version of Wasmtime on the same target architecture, but for any module
/// where they don't already have the WebAssembly module binary available this
/// could theoretically lead to an undesirable information disclosure. So you
/// should only include user-provided modules in profiles.
#[derive(Debug)]
pub struct GuestProfiler {
    profile: Profile,
    modules: Modules,
    process: fxprof_processed_profile::ProcessHandle,
    thread: fxprof_processed_profile::ThreadHandle,
    start: Instant,
    marker: CallMarker,
}

#[derive(Debug)]
struct ProfiledModule {
    module: Module,
    fxprof_libhandle: fxprof_processed_profile::LibraryHandle,
    text_range: Range<usize>,
}

type Modules = Vec<ProfiledModule>;

impl GuestProfiler {
    /// Begin profiling a new guest. When this function is called, the current
    /// wall-clock time is recorded as the start time for the guest.
    ///
    /// The `module_name` parameter is recorded in the profile to help identify
    /// where the profile came from.
    ///
    /// The `interval` parameter should match the rate at which you intend
    /// to call `sample`. However, this is used as a hint and not required to
    /// exactly match the real sample rate.
    ///
    /// Only modules which are present in the `modules` vector will appear in
    /// stack traces in this profile. Any stack frames which were executing
    /// host code or functions from other modules will be omitted. See the
    /// "Security" section of the [`GuestProfiler`] documentation for guidance
    /// on what modules should not be included in this list.
    pub fn new(
        module_name: &str,
        interval: Duration,
        modules: impl IntoIterator<Item = (String, Module)>,
    ) -> Self {
        let zero = ReferenceTimestamp::from_millis_since_unix_epoch(0.0);
        let mut profile = Profile::new(module_name, zero, interval.into());

        // Past this point, we just need to think about modules as we pull out
        // the disparate module information from components.
        let mut modules: Vec<_> = modules
            .into_iter()
            .filter_map(|(name, module)| {
                let compiled = module.compiled_module();
                let text_range = {
                    // Assumption: within text, the code for a given module is packed linearly and
                    // is non-overlapping; if this is violated, it should be safe but might result
                    // in incorrect profiling results.
                    //
                    // Assumption: there is no code cloning going on
                    // when profiling, so the EngineCode is the same
                    // as the StoreCode. This is a hack and we should
                    // have a better API (e.g.,
                    // `.text_range_for_store_code_if_invariant()`
                    // that returns a Result and errors if config is
                    // wrong).
                    let start = compiled.finished_function_ranges().next()?.1.start;
                    let end = compiled.finished_function_ranges().last()?.1.end;

                    let start = (module.engine_code().text_range().start + start).raw();
                    let end = (module.engine_code().text_range().start + end).raw();
                    start..end
                };

                module_symbols(name, &module).map(|lib| {
                    let libhandle = profile.add_lib(lib);
                    ProfiledModule {
                        module,
                        fxprof_libhandle: libhandle,
                        text_range,
                    }
                })
            })
            .collect();

        modules.sort_unstable_by_key(|m| m.text_range.start);

        profile.set_reference_timestamp(std::time::SystemTime::now().into());
        let process = profile.add_process(module_name, 0, Timestamp::from_nanos_since_reference(0));
        let thread = profile.add_thread(process, 0, Timestamp::from_nanos_since_reference(0), true);
        let start = Instant::now();
        let marker = CallMarker::new(&mut profile);
        Self {
            profile,
            modules,
            process,
            thread,
            start,
            marker,
        }
    }

    /// Create a new profiler for the provided component
    ///
    /// See [`GuestProfiler::new`] for additional information; this function
    /// works identically except that it takes a component and sets up
    /// instrumentation to track calls in each of its constituent modules.
    #[cfg(feature = "component-model")]
    pub fn new_component(
        component_name: &str,
        interval: Duration,
        component: Component,
        extra_modules: impl IntoIterator<Item = (String, Module)>,
    ) -> Self {
        let modules = component
            .static_modules()
            .map(|m| (m.name().unwrap_or("<unknown>").to_string(), m.clone()))
            .chain(extra_modules);
        Self::new(component_name, interval, modules)
    }

    /// Add a sample to the profile. This function collects a backtrace from
    /// any stack frames for allowed modules on the current stack. It should
    /// typically be called from a callback registered using
    /// [`Store::epoch_deadline_callback()`](crate::Store::epoch_deadline_callback).
    ///
    /// The `delta` parameter is the amount of CPU time that was used by this
    /// guest since the previous sample. It is allowed to pass `Duration::ZERO`
    /// here if recording CPU usage information is not needed.
    pub fn sample(&mut self, store: impl AsContext, delta: Duration) {
        let now = Timestamp::from_nanos_since_reference(
            self.start.elapsed().as_nanos().try_into().unwrap(),
        );
        let backtrace = Backtrace::new(store.as_context().0);
        let frames = lookup_frames(&self.modules, &backtrace);
        let stack = self
            .profile
            .intern_stack_frames(self.thread, frames.into_iter());
        self.profile
            .add_sample(self.thread, now, stack, delta.into(), 1);
    }

    /// Add a marker for transitions between guest and host to the profile.
    /// This function should typically be called from a callback registered
    /// using [`Store::call_hook()`](crate::Store::call_hook), and the `kind`
    /// parameter should be the value of the same type passed into that hook.
    pub fn call_hook(&mut self, store: impl AsContext, kind: CallHook) {
        let now = Timestamp::from_nanos_since_reference(
            self.start.elapsed().as_nanos().try_into().unwrap(),
        );
        match kind {
            CallHook::CallingWasm | CallHook::ReturningFromWasm => {}
            CallHook::CallingHost => {
                let backtrace = Backtrace::new(store.as_context().0);
                let frames = lookup_frames(&self.modules, &backtrace);
                let marker = self.profile.add_marker(
                    self.thread,
                    MarkerTiming::IntervalStart(now),
                    self.marker,
                );
                let stack = self
                    .profile
                    .intern_stack_frames(self.thread, frames.into_iter());
                self.profile.set_marker_stack(self.thread, marker, stack);
            }
            CallHook::ReturningFromHost => {
                self.profile
                    .add_marker(self.thread, MarkerTiming::IntervalEnd(now), self.marker);
            }
        }
    }

    /// When the guest finishes running, call this function to write the
    /// profile to the given `output`. The output is a JSON-formatted object in
    /// the [Firefox "processed profile format"][fmt]. Files in this format may
    /// be visualized at <https://profiler.firefox.com/>.
    ///
    /// [fmt]: https://github.com/firefox-devtools/profiler/blob/main/docs-developer/processed-profile-format.md
    pub fn finish(mut self, output: impl std::io::Write) -> Result<()> {
        let now = Timestamp::from_nanos_since_reference(
            self.start.elapsed().as_nanos().try_into().unwrap(),
        );
        self.profile.set_thread_end_time(self.thread, now);
        self.profile.set_process_end_time(self.process, now);

        serde_json::to_writer(output, &self.profile)?;
        Ok(())
    }
}

fn module_symbols(name: String, module: &Module) -> Option<LibraryInfo> {
    let compiled = module.compiled_module();
    let symbols = Vec::from_iter(
        module
            .env_module()
            .defined_func_indices()
            .map(|defined_idx| {
                let loc = compiled.func_loc(defined_idx);
                let func_idx = compiled.module().func_index(defined_idx);
                let mut name = String::new();
                demangle_function_name_or_index(
                    &mut name,
                    compiled.func_name(func_idx),
                    defined_idx.as_u32() as usize,
                )
                .unwrap();
                Symbol {
                    address: loc.start,
                    size: Some(loc.length),
                    name,
                }
            }),
    );
    if symbols.is_empty() {
        return None;
    }

    Some(LibraryInfo {
        name,
        debug_name: String::new(),
        path: String::new(),
        debug_path: String::new(),
        debug_id: DebugId::nil(),
        code_id: None,
        arch: None,
        symbol_table: Some(Arc::new(SymbolTable::new(symbols))),
    })
}

fn lookup_frames<'a>(
    modules: &'a Modules,
    backtrace: &'a Backtrace,
) -> impl Iterator<Item = FrameInfo> + 'a {
    backtrace
        .frames()
        // Samply needs to see the oldest frame first, but we list the newest
        // first, so iterate in reverse.
        .rev()
        .filter_map(|frame| {
            let idx = modules
                .binary_search_by(|probe| {
                    if probe.text_range.contains(&frame.pc()) {
                        Ordering::Equal
                    } else {
                        probe.text_range.start.cmp(&frame.pc())
                    }
                })
                .ok()?;
            let module = modules.get(idx)?;

            // We need to point to the modules full text (not just its functions) as
            // the offset for the final phase; these can be different for component
            // model modules.
            let module_text_start = module.module.text().as_ptr_range().start as usize;
            return Some(FrameInfo {
                frame: Frame::RelativeAddressFromReturnAddress(
                    module.fxprof_libhandle,
                    u32::try_from(frame.pc() - module_text_start).unwrap(),
                ),
                category_pair: CategoryHandle::OTHER.into(),
                flags: FrameFlags::empty(),
            });
        })
}

#[derive(Debug, Clone, Copy)]
struct CallMarker {
    name: StringHandle,
}

impl CallMarker {
    fn new(profile: &mut Profile) -> Self {
        let name = profile.intern_string(Self::UNIQUE_MARKER_TYPE_NAME);
        Self { name }
    }
}

impl StaticSchemaMarker for CallMarker {
    const UNIQUE_MARKER_TYPE_NAME: &'static str = "hostcall";
    const FIELDS: &'static [StaticSchemaMarkerField] = &[];
    const LOCATIONS: MarkerLocations = MarkerLocations::MARKER_CHART
        .union(MarkerLocations::MARKER_TABLE.union(MarkerLocations::TIMELINE_OVERVIEW));

    fn name(&self, _profile: &mut Profile) -> StringHandle {
        self.name
    }
    fn category(&self, _profile: &mut Profile) -> CategoryHandle {
        CategoryHandle::OTHER
    }
    fn string_field_value(&self, _field_index: u32) -> StringHandle {
        unreachable!("no fields")
    }
    fn number_field_value(&self, _field_index: u32) -> f64 {
        unreachable!("no fields")
    }
}
