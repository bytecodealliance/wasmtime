use crate::prelude::*;
use crate::runtime::vm::VMFuncRef;
use crate::store::StoreOpaque;
use crate::{AsContextMut, Engine, Func, Memory, Store, ValRaw};
use core::ops::Range;
use core::ptr::NonNull;
use wasmtime_environ::RrCallKind;

mod codec;
mod init;
mod objects;

use codec::{Kind, Reader};
use objects::{Objects, RecordedCall, RecordedFunc, RecordedMemory};

/// The call ID with which `rr_enter` reports a call that is not recorded.
const NOT_RECORDED: u32 = u32::MAX;

/// The SHA-256 checksum of a core module's bytecode, by which traces
/// identify the modules they instantiate.
pub type Checksum = [u8; 32];

/// Returns the core modules that `engine` compiles from `wasm`, with their
/// checksums. `wasm` is a core module, in the binary or (with the `wat`
/// feature) text format, which is returned as is.
///
/// A trace refers to the modules it instantiates by these checksums, so this
/// can be used to find or package the bytecode that replaying it requires.
pub fn core_modules(engine: &crate::Engine, wasm: &[u8]) -> Result<Vec<(Checksum, Vec<u8>)>> {
    #[cfg(feature = "wat")]
    let parsed = wat::parse_bytes(wasm).map_err(|e| format_err!("{e}"))?;
    #[cfg(feature = "wat")]
    let wasm = &*parsed;
    let checksum = |wasm: &[u8]| *wasmtime_environ::WasmChecksum::from_binary(wasm, true);
    if !wasmparser::Parser::is_component(wasm) {
        return Ok(vec![(checksum(wasm), wasm.to_vec())]);
    }
    let _ = engine;
    bail!("record/replay does not support components")
}

/// Formats a checksum as hexadecimal, as in file names and messages.
pub fn checksum_hex(checksum: &Checksum) -> String {
    checksum.iter().map(|b| format!("{b:02x}")).collect()
}

/// A complete execution trace, including object construction and startup.
///
/// Traces refer to the modules they instantiate only by [`Checksum`]; see
/// [`Trace::module_checksums`].
pub struct Trace {
    bytes: Vec<u8>,
}

impl Trace {
    /// Returns the serialized trace.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Takes ownership of a serialized trace, checking its version. Its
    /// contents are checked as they are read, such as during replay.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self> {
        let mut reader = Reader::new(&bytes);
        ensure!(
            reader.take(codec::MAGIC.len())? == codec::MAGIC,
            "unsupported record/replay trace version"
        );
        Ok(Self { bytes })
    }

    /// Returns the checksums of the core modules that the trace instantiates,
    /// in the order in which they are first instantiated, without duplicates.
    /// Replaying the trace requires their bytecode.
    ///
    /// This reads the whole trace, and fails if it is truncated or otherwise
    /// malformed.
    pub fn module_checksums(&self) -> Result<Vec<Checksum>> {
        let mut reader = Reader::new(&self.bytes);
        reader.take(codec::MAGIC.len())?;
        let mut checksums = Vec::new();
        loop {
            let (tag, mut body) = reader.record()?;
            match tag {
                codec::END => {
                    body.end()?;
                    reader.end()?;
                    return Ok(checksums);
                }
                codec::MODULE => {
                    body.u32()?;
                    let checksum = body.checksum()?;
                    if !checksums.contains(&checksum) {
                        checksums.push(checksum);
                    }
                }
                _ => {}
            }
        }
    }
}

#[derive(Default)]
pub(crate) struct State {
    session: Option<Box<Session>>,
}

struct Session {
    objects: Objects,
    mode: Mode,
    /// Memory ranges that the host may have written since the last flush.
    dirty_ranges: TryVec<DirtyRange>,
    failure: Option<Error>,
}

/// A range of a memory that the host may have written, through the memory
/// APIs, since the last flush.
///
/// The APIs hand out `&mut [u8]` slices, so individual writes are invisible to
/// the recorder: it notes the ranges that were made writable instead, and at
/// the next point at which the guest could observe memory (see
/// `StoreOpaque::rr_flush`) records their contents at that point.
struct DirtyRange {
    memory: RecordedMemory,
    /// The byte range within that memory.
    bytes: Range<usize>,
}

enum Mode {
    Recording {
        bytes: TryVec<u8>,
        // Calls, and the called functions, that have not yet returned.
        outstanding: TryVec<(RecordedCall, RecordedFunc)>,
        next_call: u32,
    },
}

impl State {
    pub(crate) fn active(&self) -> bool {
        self.session.is_some()
    }

    pub(crate) fn recording(&self) -> bool {
        matches!(
            self.session.as_deref().map(|s| &s.mode),
            Some(Mode::Recording { .. })
        )
    }

    /// Records that an active session cannot represent `operation`, which
    /// still takes place. The recording can then only be discarded.
    pub(crate) fn poison(&mut self, operation: &'static str) {
        if let Some(session) = &mut self.session {
            session.fail(format_err!("record/replay does not support {operation}"));
        }
    }

    /// Like `poison`, but also fails the operation.
    pub(crate) fn reject(&mut self, operation: &'static str) -> Result<()> {
        if self.active() {
            self.poison(operation);
            bail!("record/replay does not support {operation}");
        }
        Ok(())
    }
}

impl Session {
    /// Keeps the first failure of a session; later ones are its consequences.
    fn fail(&mut self, error: Error) {
        self.failure.get_or_insert(error);
    }

    /// Appends a complete record when recording.
    fn append(&mut self, tag: u8, body: &[u8]) -> Result<()> {
        let Mode::Recording { bytes, .. } = &mut self.mode;
        codec::record(bytes, tag, body.len())?;
        bytes.extend_from_slice(body)?;
        Ok(())
    }
}

fn validate_recording_engine(engine: &Engine) -> Result<()> {
    ensure!(
        engine.is_recording(),
        "recording requires RrConfig::Recording"
    );
    // A debug handler could run arbitrary host code that the trace does not
    // record.
    ensure!(
        !engine.tunables().debug_guest,
        "record/replay does not support recording with guest debugging"
    );
    ensure!(
        !engine
            .features()
            .contains(wasmparser::WasmFeatures::STACK_SWITCHING),
        "record/replay does not support Wasm stack switching"
    );
    Ok(())
}

impl<T: 'static> Store<T> {
    /// Creates a store that records its execution, including object
    /// construction and startup, until [`Store::finish_recording`].
    ///
    /// See [`crate::rr`] for the restrictions on the recorded execution.
    ///
    /// # Errors
    ///
    /// Returns an error if `engine` is not configured with
    /// [`crate::RrConfig::Recording`], if its configuration is incompatible
    /// with recording, or on allocation failure.
    pub fn new_recording(engine: &Engine, data: T) -> Result<Self> {
        validate_recording_engine(engine)?;
        let mut bytes = TryVec::new();
        bytes.extend_from_slice(codec::MAGIC)?;
        let session = try_new::<Box<_>>(Session {
            objects: Objects::default(),
            mode: Mode::Recording {
                bytes,
                outstanding: TryVec::new(),
                next_call: 0,
            },
            dirty_ranges: TryVec::new(),
            failure: None,
        })?;
        let mut this = Store::try_new(engine, data)?;
        let store = this.as_context_mut().0;
        store.rr.session = Some(session);
        store.vm_store_context_mut().recording = 1;
        Ok(this)
    }

    /// Finishes a recording, flushing pending memory writes. Returns an error
    /// for a poisoned recording; such a trace cannot be replayed. A failed
    /// recording is discarded and recording is disabled on the store.
    ///
    /// A recording may end while guest calls are unfinished, for example
    /// after a host panic.
    pub fn finish_recording(&mut self) -> Result<Trace> {
        let store = self.as_context_mut().0;
        ensure!(store.rr.recording(), "store is not recording");
        let flushed = store.rr_flush();
        let session = store.rr.session.take().unwrap();
        store.vm_store_context_mut().recording = 0;
        if let Some(e) = session.failure {
            return Err(e);
        }
        flushed?;
        let Mode::Recording { mut bytes, .. } = session.mode;
        codec::record(&mut bytes, codec::END, 0)?;
        Ok(Trace {
            bytes: bytes.into(),
        })
    }
}

impl StoreOpaque {
    fn rr_session(&mut self) -> &mut Session {
        self.rr.session.as_deref_mut().unwrap()
    }

    /// Poisons an active session if recording failed, so that the trace can
    /// never be finalized with a partial record or a missing effect.
    fn rr_poison_on_err<R>(&mut self, result: Result<R>) -> Result<R> {
        if let (Err(e), Some(session)) = (&result, &mut self.rr.session) {
            session.fail(format_err!("failed to record execution: {e}"));
        }
        result
    }

    /// The ID of a memory written by the host, poisoning the recording if
    /// the memory was never registered.
    fn rr_memory_id(&mut self, memory: Memory) -> Option<RecordedMemory> {
        let id = self
            .rr
            .session
            .as_ref()
            .unwrap()
            .objects
            .find_memory(self, memory);
        if id.is_none() {
            self.rr_session()
                .fail(format_err!("host modified an unregistered memory"));
        }
        id
    }

    pub(crate) fn rr_memory_grown(&mut self, memory: Memory) -> Result<()> {
        if !self.rr.recording() {
            return Ok(());
        }
        let Some(id) = self.rr_memory_id(memory) else {
            bail!("host grew an unregistered memory");
        };
        let new_size = memory.internal_data_size(self);
        let result = (|| {
            let mut body = [0; 12];
            body[..4].copy_from_slice(&id.as_u32().to_le_bytes());
            body[4..].copy_from_slice(&u64::try_from(new_size)?.to_le_bytes());
            self.rr_session().append(codec::RESIZE, &body)
        })();
        self.rr_poison_on_err(result)
    }

    fn rr_check(&self) -> Result<()> {
        if let Some(session) = &self.rr.session {
            ensure!(
                session.failure.is_none(),
                "recording has failed; discard it with finish_recording"
            );
        }
        Ok(())
    }

    /// Records the contents of the memory ranges that the host may have
    /// written since the last flush (including through forgotten guards).
    ///
    /// This runs while the store is exclusively borrowed and before the guest
    /// can observe memory again: at every call or return between the host and
    /// Wasm, and when the recording finishes.
    pub(crate) fn rr_flush(&mut self) -> Result<()> {
        self.rr_check()?;
        if !self.rr.recording() {
            return Ok(());
        }
        let result = self.rr_flush_writes();
        self.rr_poison_on_err(result)
    }

    fn rr_flush_writes(&mut self) -> Result<()> {
        // Detach the dirty ranges and the trace while reading memories; this
        // never runs guest or embedder code.
        let session = self.rr_session();
        let mut dirty = core::mem::take(&mut session.dirty_ranges);
        let Mode::Recording { bytes, .. } = &mut session.mode;
        let mut bytes = core::mem::take(bytes);
        // Merge overlapping and adjacent ranges, so that each byte is recorded
        // once. Sorting enables the merge and does not change the trace's
        // meaning: the merged ranges are disjoint, and each record carries the
        // range's current contents.
        dirty.sort_unstable_by_key(|r| (r.memory, r.bytes.start));
        dirty.dedup_by(|next, prev| {
            let overlaps = next.memory == prev.memory && next.bytes.start <= prev.bytes.end;
            if overlaps {
                prev.bytes.end = prev.bytes.end.max(next.bytes.end);
            }
            overlaps
        });
        let result = (|| {
            for DirtyRange {
                memory: id,
                bytes: range,
            } in &dirty
            {
                let memory = self.rr.session.as_ref().unwrap().objects.memories[*id];
                let data = memory
                    .data_in_store(self)
                    .get(range.clone())
                    .ok_or_else(|| format_err!("tracked memory range is no longer valid"))?;
                let len = 12usize
                    .checked_add(data.len())
                    .ok_or_else(|| format_err!("memory record too large"))?;
                codec::record(&mut bytes, codec::WRITE, len)?;
                bytes.extend_from_slice(&id.as_u32().to_le_bytes())?;
                bytes.extend_from_slice(&u64::try_from(range.start)?.to_le_bytes())?;
                bytes.extend_from_slice(data)?;
            }
            Ok(())
        })();
        dirty.clear();
        let session = self.rr_session();
        session.dirty_ranges = dirty;
        let Mode::Recording { bytes: trace, .. } = &mut session.mode;
        *trace = bytes;
        result
    }

    pub(crate) fn rr_track_memory(&mut self, memory: Memory, range: Range<usize>) {
        if !self.rr.recording() || range.is_empty() {
            return;
        }
        let Some(id) = self.rr_memory_id(memory) else {
            return;
        };
        let session = self.rr_session();
        // Writes are usually sequential, so coalesce with the previous range
        // here, in constant time; the flush merges any other overlaps.
        if let Some(last) = session.dirty_ranges.last_mut()
            && last.memory == id
            && range.start <= last.bytes.end
            && last.bytes.start <= range.end
        {
            last.bytes.start = last.bytes.start.min(range.start);
            last.bytes.end = last.bytes.end.max(range.end);
            return;
        }
        if let Err(oom) = session.dirty_ranges.push(DirtyRange {
            memory: id,
            bytes: range,
        }) {
            session.fail(oom.into());
        }
    }

    /// Registers the function references among a call's parameters or
    /// `results`, so that they can be encoded.
    ///
    /// # Safety
    ///
    /// `raw` must point to initialized slots of the parameter or result types
    /// of `func`, whose function references this store owns.
    unsafe fn rr_boundary_refs(
        &mut self,
        func: RecordedFunc,
        raw: *const ValRaw,
        results: bool,
    ) -> Result<()> {
        let bound = &self.rr_session().objects.funcs[func];
        let count = if results {
            bound.results.len()
        } else {
            bound.params.len()
        };
        for i in 0..count {
            let bound = &self.rr_session().objects.funcs[func];
            let kind = if results {
                bound.results[i]
            } else {
                bound.params[i]
            };
            if let Kind::FuncRef { .. } = kind {
                // SAFETY: the slot is initialized and has funcref type.
                if let Some(ptr) = NonNull::new(unsafe { (*raw.add(i)).get_funcref() }) {
                    // SAFETY: this store owns the reference.
                    unsafe { self.rr_register_func_reference(ptr.cast())? };
                }
            }
        }
        Ok(())
    }

    /// The `rr_enter` builtin, which the trampolines of record/replay engines
    /// call while the store records. Returns the call's ID, or
    /// [`NOT_RECORDED`].
    ///
    /// # Safety
    ///
    /// `func` must be a valid function reference that this store owns, and
    /// `raw` must point to initialized slots of its parameter types, whose
    /// function references this store owns.
    pub(crate) unsafe fn rr_enter(
        &mut self,
        func: NonNull<VMFuncRef>,
        raw: *const ValRaw,
        kind: RrCallKind,
    ) -> Result<u32> {
        if !self.rr.recording() {
            return Ok(NOT_RECORDED);
        }
        // SAFETY: inherited from this function's contract.
        let result = unsafe { self.rr_record_enter(func, raw, kind) };
        self.rr_poison_on_err(result)
    }

    /// # Safety
    ///
    /// As for [`StoreOpaque::rr_enter`].
    unsafe fn rr_record_enter(
        &mut self,
        func: NonNull<VMFuncRef>,
        raw: *const ValRaw,
        kind: RrCallKind,
    ) -> Result<u32> {
        self.rr_flush()?;
        // SAFETY: inherited from this function's contract.
        let func = unsafe { func.as_ref() };
        let id = self.rr_session().objects.find_func_ref(func)?;
        // SAFETY: inherited from this function's contract.
        unsafe {
            self.rr_boundary_refs(id, raw, false)?;
        }
        let session = self.rr_session();
        let bound = &session.objects.funcs[id];
        let Mode::Recording {
            bytes,
            outstanding,
            next_call,
        } = &mut session.mode;
        let call = *next_call;
        *next_call = next_call
            .checked_add(1)
            .filter(|c| *c != NOT_RECORDED)
            .ok_or_else(|| format_err!("too many recorded calls"))?;
        let tag = match kind {
            RrCallKind::WasmToHost => codec::ENTER_HOST,
            RrCallKind::HostToWasm => codec::ENTER_WASM,
        };
        codec::record(bytes, tag, 8 + codec::values_len(&bound.params))?;
        bytes.extend_from_slice(&id.as_u32().to_le_bytes())?;
        bytes.extend_from_slice(&call.to_le_bytes())?;
        // SAFETY: inherited from this function's contract.
        unsafe {
            codec::values(bytes, &bound.params, raw, |ptr| {
                session.objects.encode_ref(ptr)
            })?
        };
        outstanding.push((RecordedCall::from_u32(call), id))?;
        Ok(call)
    }

    /// The `rr_leave` builtin, which records the return of `call` from
    /// [`StoreOpaque::rr_enter`]. If not `succeeded` then the call fails with
    /// the error that is being raised.
    ///
    /// # Safety
    ///
    /// `call` must be what `rr_enter` returned for this call in this store. If
    /// `succeeded` then `raw` must point to initialized slots of the result
    /// types of the function that was passed to `rr_enter`, whose function
    /// references this store owns.
    pub(crate) unsafe fn rr_leave(
        &mut self,
        call: u32,
        raw: *const ValRaw,
        kind: RrCallKind,
        succeeded: bool,
    ) -> Result<()> {
        if call == NOT_RECORDED {
            return Ok(());
        }
        let result = if succeeded {
            Ok(())
        } else {
            match crate::runtime::vm::pending_unwind_error() {
                Some(error) => Err(error),
                // A host panic is not recorded; it propagates through the
                // recording.
                None => return Ok(()),
            }
        };
        // SAFETY: inherited from this function's contract.
        let recorded =
            unsafe { self.rr_record_leave(RecordedCall::from_u32(call), raw, &result, kind) };
        let recorded = self.rr_poison_on_err(recorded);
        // A failing call keeps its own error; the poisoned recording reports
        // this one.
        if succeeded { recorded } else { Ok(()) }
    }

    /// # Safety
    ///
    /// As for [`StoreOpaque::rr_leave`], where the call succeeded if `result`
    /// is `Ok`.
    unsafe fn rr_record_leave(
        &mut self,
        call: RecordedCall,
        raw: *const ValRaw,
        result: &Result<()>,
        kind: RrCallKind,
    ) -> Result<()> {
        // The recording may have ended while this call was unfinished.
        if !self.rr.recording() {
            return Ok(());
        }
        self.rr_flush()?;
        let Mode::Recording { outstanding, .. } = &self.rr_session().mode;
        let position = outstanding
            .iter()
            .position(|(c, _)| *c == call)
            .ok_or_else(|| format_err!("return without recorded call"))?;
        let func = outstanding[position].1;
        if result.is_ok() {
            // SAFETY: inherited from this function's contract, as `func` is
            // the function that was passed to `rr_enter`.
            unsafe {
                self.rr_boundary_refs(func, raw, true)?;
            }
        }
        let session = self.rr_session();
        let Mode::Recording {
            bytes, outstanding, ..
        } = &mut session.mode;
        let tag = match kind {
            RrCallKind::WasmToHost => codec::LEAVE_HOST,
            RrCallKind::HostToWasm => codec::LEAVE_WASM,
        };
        let results = &session.objects.funcs[func].results;
        // SAFETY: inherited from this function's contract.
        unsafe {
            codec::outcome(bytes, tag, call, result, results, raw, |ptr| {
                session.objects.encode_ref(ptr)
            })?;
        }
        outstanding.swap_remove(position);
        Ok(())
    }
}
