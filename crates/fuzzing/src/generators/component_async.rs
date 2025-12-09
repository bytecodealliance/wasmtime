//! For a high-level overview of this fuzz target see `fuzz_async.rs`

#![expect(missing_docs, reason = "macro-generated code")]

use arbitrary::{Arbitrary, Unstructured};
use indexmap::{IndexMap, IndexSet};

wasmtime::component::bindgen!({
    world: "fuzz-async",
    imports: {
        "wasmtime-fuzz:fuzz/types.get-commands": store,
    },
    exports: { default: async | store },
});

use wasmtime_fuzz::fuzz::types::{
    Command, FuturePayload, StreamReadPayload, StreamReadyPayload, StreamWritePayload,
};

const SOFT_MAX_COMMANDS: usize = 100;
const MAX_STREAM_COUNT: u32 = 10;

/// Structure used for the "component async" fuzzer.
///
/// This encapsulates a list of commands for the fuzzer to run. Note that the
/// commands are not 100% arbitrary but instead they're generated similar to
/// wasm instructions where only some sequences of instructions are valid. The
/// rest of this module is dedicated to the generation of these commands.
#[derive(Debug)]
pub struct ComponentAsync {
    /// A sequence of commands to run, tagged with a scope that they're run
    /// within.
    pub commands: Vec<(Scope, Command)>,
}

/// The possible "scopes" that async commands run within.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Scope {
    /// The outermost layer of the host, which controls invocations of the
    /// guests.
    HostCaller,

    /// The first layer of the guest, or the raw exports from the root of the
    /// component.
    ///
    /// This imports functions from the `GuestCallee`.
    GuestCaller,

    /// The second layer of the guest which imports the host functions directly.
    ///
    /// This is then in turn imported by the `GuestCaller`.
    GuestCallee,

    /// The innermost layer of the host which provides imported functions to the
    /// `GuestCallee`.
    HostCallee,
}

impl Scope {
    const ALL: &[Scope; 4] = &[
        Scope::HostCaller,
        Scope::GuestCaller,
        Scope::GuestCallee,
        Scope::HostCallee,
    ];
    const CALLERS: &[Scope; 3] = &[Scope::HostCaller, Scope::GuestCaller, Scope::GuestCallee];

    fn callee(&self) -> Option<Scope> {
        match self {
            Scope::HostCaller => Some(Scope::GuestCaller),
            Scope::GuestCaller => Some(Scope::GuestCallee),
            Scope::GuestCallee => Some(Scope::HostCallee),
            Scope::HostCallee => None,
        }
    }

    fn caller(&self) -> Option<Scope> {
        match self {
            Scope::HostCaller => None,
            Scope::GuestCaller => Some(Scope::HostCaller),
            Scope::GuestCallee => Some(Scope::GuestCaller),
            Scope::HostCallee => Some(Scope::GuestCallee),
        }
    }

    fn is_host(&self) -> bool {
        match self {
            Scope::HostCaller | Scope::HostCallee => true,
            Scope::GuestCaller | Scope::GuestCallee => false,
        }
    }
}

impl Arbitrary<'_> for ComponentAsync {
    fn arbitrary(u: &mut Unstructured<'_>) -> arbitrary::Result<Self> {
        let mut state = State::default();
        let mut ret = Vec::new();

        // While there's more unstructured data, and our list of commands isn't
        // too long, generate some new commands per-component.
        while !u.is_empty() && ret.len() < SOFT_MAX_COMMANDS {
            state.generate(u, false, &mut ret)?;
        }

        // Optionally, if specified, finish up all async operations.
        if u.arbitrary()? {
            while !state.is_empty() {
                state.generate(u, true, &mut ret)?;
            }
        }

        Ok(ComponentAsync { commands: ret })
    }
}

#[derive(Default)]
struct State {
    next_id: u32,

    /// List of scopes that have an active and pending call to the
    /// `async-pending` function.
    async_pending: Vec<(Scope, u32)>,

    /// Deferred work that can happen at any time, for example asserting the
    /// result of some previous operation.
    deferred: Vec<(Scope, Command)>,

    /// State associated with futures/streams and their handles within.
    futures: HandleStates<(), u32>,
    streams: HandleStates<StreamRead, StreamWrite>,
}

#[derive(Default)]
struct HandleStates<R, W> {
    readers: HalfStates<R>,
    writers: HalfStates<W>,
}

impl<R, W> HandleStates<R, W> {
    fn is_empty(&self) -> bool {
        self.readers.is_empty() && self.writers.is_empty()
    }
}

/// State management for "half" of a future/stream read/write pair.
///
/// This tracks all the various states of all handles in the system to be able
/// to select amongst them an arbitrary operation to perform. This structure's
/// sets are primarily manipulated through helper methods to ensure that the set
/// metadata all stays in sync.
#[derive(Default)]
struct HalfStates<T> {
    /// All known handles of this type, where they're located, etc.
    handles: IndexMap<u32, (Scope, HalfState, Transferrable)>,

    /// All handles which can currently be dropped. Handles can't be dropped if
    /// they're in use, for example.
    droppable: IndexSet<u32>,

    /// All handles which can be read/written from (depending on handle type).
    /// Handles where both pairs are in the same component can't be
    /// read/written to for example.
    ready: IndexSet<u32>,

    /// All handles which can be transferred somewhere else.
    ///
    /// Some examples of non-transferrable handles are:
    ///
    /// * writers
    /// * handles with an outstanding read
    /// * host-based handles that have been used at least once (FIXME #12090)
    transferrable: IndexSet<u32>,

    /// Handles currently being read/written to.
    ///
    /// Also includes state about the operation, such as whether it's been
    /// dropped on the other side.
    in_use: IndexMap<u32, (T, OpState)>,

    /// Handles with a pending operation which can be cancelled.
    cancellable: IndexSet<u32>,
}

enum HalfState {
    Idle,
    InUse,
}

#[derive(Copy, Clone, PartialEq, Debug)]
enum Transferrable {
    Yes,
    No,
}

#[derive(Copy, Clone, PartialEq, Debug)]
enum Cancellable {
    Yes,
    No,
}

#[derive(Copy, Clone, PartialEq, Debug)]
enum OpState {
    Pending,
    Dropped,
}

#[derive(Default, Copy, Clone)]
struct StreamRead {
    count: u32,
}

#[derive(Default, Copy, Clone)]
struct StreamWrite {
    item: u32,
    count: u32,
}

impl<T> HalfStates<T> {
    fn is_empty(&self) -> bool {
        self.handles.is_empty()
    }

    /// Adds a new handle `id` to this set.
    fn insert(&mut self, id: u32, scope: Scope, transferrable: Transferrable) {
        let prev = self
            .handles
            .insert(id, (scope, HalfState::Idle, transferrable));
        assert!(prev.is_none());
        assert!(self.droppable.insert(id));
        if transferrable == Transferrable::Yes {
            self.transferrable.insert(id);
        }
    }

    /// Removes the handle `id` for closing.
    fn remove(&mut self, id: u32) -> Scope {
        let (scope, state, transferrable) = self.handles.swap_remove(&id).unwrap();
        assert!(matches!(state, HalfState::Idle));
        self.droppable.swap_remove(&id);
        self.ready.swap_remove(&id);
        if transferrable == Transferrable::Yes {
            assert!(self.transferrable.swap_remove(&id));
        }
        scope
    }

    /// Locks `id` in whatever scope it's currently in for the rest of its
    /// lifetime, preventing its transfer. This is used as a workaround for
    /// #12090.
    fn lock_in_place(&mut self, id: u32) {
        let (_scope, state, transferrable) = self.handles.get_mut(&id).unwrap();
        assert!(matches!(state, HalfState::Idle));
        if matches!(transferrable, Transferrable::Yes) {
            assert!(self.transferrable.swap_remove(&id));
            *transferrable = Transferrable::No;
        }
    }

    /// Starts an operation on the handle `id`.
    fn start(&mut self, id: u32, cancellable: Cancellable, payload: T) {
        let (_scope, state, transferrable) = self.handles.get_mut(&id).unwrap();
        assert!(matches!(state, HalfState::Idle));
        assert!(self.ready.swap_remove(&id));
        self.droppable.swap_remove(&id);
        *state = HalfState::InUse;
        let prev = self.in_use.insert(id, (payload, OpState::Pending));
        assert!(prev.is_none());
        if *transferrable == Transferrable::Yes {
            assert!(self.transferrable.swap_remove(&id));
        }
        if cancellable == Cancellable::Yes {
            assert!(self.cancellable.insert(id));
        }
    }

    /// Completes an operation on `id`, returning the state it was started with
    /// along with whether it was dropped.
    fn stop(&mut self, id: u32) -> (T, OpState) {
        let (_scope, state, transferrable) = self.handles.get_mut(&id).unwrap();
        assert!(matches!(state, HalfState::InUse));
        *state = HalfState::Idle;
        let dropped = self.in_use.swap_remove(&id).unwrap();
        self.cancellable.swap_remove(&id);
        if *transferrable == Transferrable::Yes {
            assert!(self.transferrable.insert(id));
        }
        assert!(self.droppable.insert(id));
        if dropped.1 != OpState::Dropped {
            assert!(self.ready.insert(id));
        } else {
            self.lock_in_place(id);
        }
        dropped
    }

    /// Updates to `OpState::Dropped` for an operation-in-progress.
    fn set_in_use_state_dropped(&mut self, id: u32) {
        let (_, prev) = self.in_use.get_mut(&id).unwrap();
        assert_eq!(*prev, OpState::Pending);
        *prev = OpState::Dropped;

        // This operation is now "cancellable" meaning that at any point in the
        // future it can be resolved since the other end was dropped.
        self.cancellable.insert(id);
    }
}

impl State {
    fn is_empty(&self) -> bool {
        let State {
            next_id: _,
            async_pending,
            deferred,
            futures,
            streams,
        } = self;
        async_pending.is_empty() && deferred.is_empty() && futures.is_empty() && streams.is_empty()
    }

    fn generate(
        &mut self,
        u: &mut Unstructured<'_>,
        finish: bool,
        commands: &mut Vec<(Scope, Command)>,
    ) -> arbitrary::Result<()> {
        let mut choices = Vec::new();

        // If we're not finishing up then have the possibility of
        // immediately-ready sync/async calls and such sort of miscellaneous
        // work.
        if !finish {
            choices.push(Choice::SyncReadyCall);
            choices.push(Choice::AsyncReadyCall);
            choices.push(Choice::FutureNew);
            choices.push(Choice::StreamNew);
        }

        // If we're not finishing up, and if we don't have too much pending
        // work, then possibly make some more pending work.
        if !finish && self.async_pending.len() < 20 {
            choices.push(Choice::AsyncPendingCall);
        }

        // If there's pending work, possibly resolve something.
        if self.async_pending.len() > 0 {
            choices.push(Choice::AsyncPendingResolve);
        }

        // If something has been deferred to later, possibly add that command
        // into the stream.
        if self.deferred.len() > 0 {
            choices.push(Choice::Deferred);
        }

        // Wrap up work with futures by dropping handles, writing, cancelling,
        // etc.
        if self.futures.readers.droppable.len() > 0 {
            choices.push(Choice::FutureDropReadable);
        }
        if self.futures.writers.droppable.len() > 0 {
            choices.push(Choice::FutureDropWritable);
        }
        if self.futures.writers.cancellable.len() > 0 {
            choices.push(Choice::FutureCancelWrite);
        }
        if self.futures.readers.cancellable.len() > 0 {
            choices.push(Choice::FutureCancelRead);
        }
        // If more work is allowed kick of reads/transfers.
        if !finish {
            if self.futures.writers.ready.len() > 0 {
                choices.push(Choice::FutureWrite);
            }
            if self.futures.readers.ready.len() > 0 {
                choices.push(Choice::FutureRead);
            }
            if self.futures.readers.transferrable.len() > 0 {
                choices.push(Choice::FutureReaderTransfer);
            }
        }

        // Streams can be dropped at any time and their pending operations can
        // be ceased at any time.
        if self.streams.readers.droppable.len() > 0 {
            choices.push(Choice::StreamDropReadable);
        }
        if self.streams.writers.droppable.len() > 0 {
            choices.push(Choice::StreamDropWritable);
        }
        if self.streams.readers.cancellable.len() > 0 {
            choices.push(Choice::StreamEndRead);
        }
        if self.streams.writers.cancellable.len() > 0 {
            choices.push(Choice::StreamEndWrite);
        }
        // If more work is allowed then streams can be moved around and new
        // reads/writes may be started.
        if !finish {
            if self.streams.readers.transferrable.len() > 0 {
                choices.push(Choice::StreamReaderTransfer);
            }
            if self.streams.readers.ready.len() > 0 {
                choices.push(Choice::StreamRead);
            }
            if self.streams.writers.ready.len() > 0 {
                choices.push(Choice::StreamWrite);
            }
        }

        #[derive(Debug)]
        enum Choice {
            SyncReadyCall,
            AsyncReadyCall,
            AsyncPendingCall,
            AsyncPendingResolve,
            Deferred,

            FutureNew,
            FutureReaderTransfer,
            FutureRead,
            FutureWrite,
            FutureCancelRead,
            FutureCancelWrite,
            FutureDropReadable,
            FutureDropWritable,

            StreamNew,
            StreamReaderTransfer,
            StreamDropReadable,
            StreamDropWritable,
            StreamRead,
            StreamWrite,
            StreamEndRead,
            StreamEndWrite,
        }

        match u.choose(&choices)? {
            Choice::SyncReadyCall => {
                let caller = *u.choose(Scope::CALLERS)?;
                commands.push((caller, Command::SyncReadyCall));
            }
            Choice::AsyncReadyCall => {
                let caller = *u.choose(Scope::CALLERS)?;
                commands.push((caller, Command::AsyncReadyCall));
            }

            Choice::AsyncPendingCall => {
                let caller = *u.choose(Scope::CALLERS)?;
                let id = self.next_id();
                self.async_pending.push((caller, id));
                commands.push((caller, Command::AsyncPendingImportCall(id)));
            }

            Choice::AsyncPendingResolve => {
                let index = u.int_in_range(0..=self.async_pending.len() - 1)?;
                let (caller, id) = self.async_pending.swap_remove(index);
                let callee = caller.callee().unwrap();

                // FIXME(#11833) the host can't cancel calls at this time, so
                // they can only be completed. Everything else though is
                // guest-initiated which means that the call can be either
                // completed or cancelled.
                let complete = caller == Scope::HostCaller || u.arbitrary()?;

                if complete {
                    commands.push((callee, Command::AsyncPendingExportComplete(id)));
                    self.deferred
                        .push((caller, Command::AsyncPendingImportAssertReady(id)));
                } else {
                    commands.push((caller, Command::AsyncPendingImportCancel(id)));
                    self.deferred
                        .push((callee, Command::AsyncPendingExportAssertCancelled(id)));
                }
            }

            Choice::Deferred => {
                let index = u.int_in_range(0..=self.deferred.len() - 1)?;
                let (scope, cmd) = self.deferred.swap_remove(index);
                commands.push((scope, cmd));
            }

            Choice::FutureNew => {
                let scope = *u.choose(Scope::ALL)?;
                let id = self.next_id();
                commands.push((scope, Command::FutureNew(id)));
                self.futures.readers.insert(id, scope, Transferrable::Yes);
                self.futures.writers.insert(id, scope, Transferrable::No);

                // Future writers cannot be dropped without writing.
                assert!(self.futures.writers.droppable.swap_remove(&id));
            }
            Choice::FutureReaderTransfer => {
                let set = &mut self.futures.readers.transferrable;
                let i = u.int_in_range(0..=set.len() - 1)?;
                let id = *set.get_index(i).unwrap();
                let scope = &mut self.futures.readers.handles[&id].0;

                enum Action {
                    CallerTake(Scope),
                    GiveCallee(Scope),
                }

                let action = match (scope.caller(), scope.callee()) {
                    (Some(caller), None) => Action::CallerTake(caller),
                    (None, Some(callee)) => Action::GiveCallee(callee),
                    (Some(caller), Some(callee)) => {
                        if u.arbitrary()? {
                            Action::CallerTake(caller)
                        } else {
                            Action::GiveCallee(callee)
                        }
                    }
                    (None, None) => unreachable!(),
                };
                match action {
                    Action::CallerTake(caller) => {
                        commands.push((caller, Command::FutureTake(id)));
                        *scope = caller;
                    }
                    Action::GiveCallee(callee) => {
                        commands.push((*scope, Command::FutureGive(id)));
                        *scope = callee;
                    }
                }

                // See what scope the reader/writer half are in. Allow
                // operations if they're in different scopes, but disallow
                // operations if they're in the same scope.
                let reader_scope = Some(*scope);
                let writer_scope = self.futures.writers.handles.get(&id).map(|p| p.0);
                if reader_scope == writer_scope {
                    self.futures.readers.ready.swap_remove(&id);
                    self.futures.writers.ready.swap_remove(&id);
                } else {
                    self.futures.readers.ready.insert(id);
                    if writer_scope.is_some() && !self.futures.writers.in_use.contains_key(&id) {
                        self.futures.writers.ready.insert(id);
                    }
                }
            }
            Choice::FutureRead => {
                let set = &self.futures.readers.ready;
                let i = u.int_in_range(0..=set.len() - 1)?;
                let id = *set.get_index(i).unwrap();
                let scope = self.futures.readers.handles[&id].0;

                if let Some((item, _)) = self.futures.writers.in_use.get(&id) {
                    // If the future has an active write, then this should
                    // complete with that write. The write is then resolved and
                    // the future reader/writer are both gone.
                    let item = *item;
                    commands.push((
                        scope,
                        Command::FutureReadReady(FuturePayload { future: id, item }),
                    ));
                    let write_scope = self.futures.writers.handles[&id].0;
                    commands.push((write_scope, Command::FutureWriteAssertComplete(id)));

                    self.futures.writers.stop(id);
                    self.futures.readers.remove(id);
                    self.futures.writers.remove(id);
                } else {
                    // If the write-end is idle, then this should be a pending
                    // future read.
                    //
                    // FIXME(#12090) host reads cannot be cancelled
                    let cancellable = if scope.is_host() {
                        Cancellable::No
                    } else {
                        Cancellable::Yes
                    };
                    self.futures.readers.start(id, cancellable, ());
                    commands.push((scope, Command::FutureReadPending(id)));
                }
            }
            Choice::FutureWrite => {
                let set = &self.futures.writers.ready;
                let i = u.int_in_range(0..=set.len() - 1)?;
                let id = *set.get_index(i).unwrap();
                let scope = self.futures.writers.handles[&id].0;
                let item = self.next_id();
                let payload = FuturePayload { future: id, item };

                if !self.futures.readers.handles.contains_key(&id) {
                    // If the reader is gone then this write should complete
                    // immediately with "dropped" and furthermore the writer
                    // should now be removed.
                    commands.push((scope, Command::FutureWriteDropped(id)));
                    self.futures.writers.remove(id);
                } else if self.futures.readers.in_use.contains_key(&id) {
                    // If the reader is in-progress then this should complete
                    // the read/write pair. The reader/writer are both removed
                    // as a result.
                    commands.push((scope, Command::FutureWriteReady(payload)));
                    let read_scope = self.futures.readers.handles[&id].0;
                    commands.push((read_scope, Command::FutureReadAssertComplete(payload)));
                    self.futures.readers.stop(id);
                    self.futures.readers.remove(id);
                    self.futures.writers.remove(id);
                } else {
                    // If the read-end is idle, then this should be a pending
                    // future read.
                    self.futures.writers.start(id, Cancellable::Yes, item);
                    commands.push((scope, Command::FutureWritePending(payload)));
                }
            }
            Choice::FutureCancelWrite => {
                let set = &self.futures.writers.cancellable;
                let i = u.int_in_range(0..=set.len() - 1)?;
                let id = *set.get_index(i).unwrap();
                let scope = self.futures.writers.handles[&id].0;

                let (_write, state) = self.futures.writers.stop(id);
                match state {
                    OpState::Pending => {
                        commands.push((scope, Command::FutureCancelWrite(id)));
                        assert!(self.futures.writers.droppable.swap_remove(&id));
                    }
                    OpState::Dropped => {
                        commands.push((scope, Command::FutureWriteAssertDropped(id)));
                        self.futures.writers.remove(id);
                    }
                }
            }
            Choice::FutureCancelRead => {
                let set = &self.futures.readers.cancellable;
                let i = u.int_in_range(0..=set.len() - 1)?;
                let id = *set.get_index(i).unwrap();
                let scope = self.futures.readers.handles[&id].0;

                let (_read, state) = self.futures.readers.stop(id);
                match state {
                    OpState::Pending => {
                        commands.push((scope, Command::FutureCancelRead(id)));
                    }
                    // Writers cannot be dropped with futures, so this is not
                    // reachable.
                    OpState::Dropped => unreachable!(),
                }
            }
            Choice::FutureDropReadable => {
                let set = &self.futures.readers.droppable;
                let i = u.int_in_range(0..=set.len() - 1)?;
                let id = *set.get_index(i).unwrap();
                let scope = self.futures.readers.remove(id);
                commands.push((scope, Command::FutureDropReadable(id)));

                // If the writer is active then its write is now destined to
                // finish with "dropped", and otherwise the writer is also now
                // droppable since the reader handle is gone.
                if self.futures.writers.in_use.contains_key(&id) {
                    self.futures.writers.set_in_use_state_dropped(id);
                } else {
                    assert!(self.futures.writers.droppable.insert(id));
                }
            }
            Choice::FutureDropWritable => {
                let set = &self.futures.writers.droppable;
                let i = u.int_in_range(0..=set.len() - 1)?;
                let id = *set.get_index(i).unwrap();
                let scope = self.futures.writers.remove(id);

                // Writers can't actually be dropped prior to writing so fake
                // a write by writing a value and asserting that the result is
                // "dropped".
                commands.push((scope, Command::FutureWriteDropped(id)));

                assert!(!self.futures.readers.handles.contains_key(&id));
            }

            Choice::StreamNew => {
                let scope = *u.choose(Scope::ALL)?;
                let id = self.next_id();
                commands.push((scope, Command::StreamNew(id)));
                self.streams.readers.insert(id, scope, Transferrable::Yes);
                self.streams.writers.insert(id, scope, Transferrable::No);
            }
            Choice::StreamReaderTransfer => {
                let set = &mut self.streams.readers.transferrable;
                let i = u.int_in_range(0..=set.len() - 1)?;
                let id = *set.get_index(i).unwrap();
                let scope = &mut self.streams.readers.handles[&id].0;

                enum Action {
                    CallerTake(Scope),
                    GiveCallee(Scope),
                }

                let action = match (scope.caller(), scope.callee()) {
                    (Some(caller), None) => Action::CallerTake(caller),
                    (None, Some(callee)) => Action::GiveCallee(callee),
                    (Some(caller), Some(callee)) => {
                        if u.arbitrary()? {
                            Action::CallerTake(caller)
                        } else {
                            Action::GiveCallee(callee)
                        }
                    }
                    (None, None) => unreachable!(),
                };
                match action {
                    Action::CallerTake(caller) => {
                        commands.push((caller, Command::StreamTake(id)));
                        *scope = caller;
                    }
                    Action::GiveCallee(callee) => {
                        commands.push((*scope, Command::StreamGive(id)));
                        *scope = callee;
                    }
                }

                // See what scope the reader/writer half are in. Allow
                // operations if they're in different scopes, but disallow
                // operations if they're in the same scope.
                //
                // Note that host<->host reads/writes for streams aren't fuzzed
                // at this time so that's also explicitly disallowed.
                let reader_scope = Some(*scope);
                let writer_scope = self.streams.writers.handles.get(&id).map(|p| p.0);
                if reader_scope == writer_scope
                    || reader_scope.is_some_and(|s| s.is_host())
                        == writer_scope.is_some_and(|s| s.is_host())
                {
                    self.streams.readers.ready.swap_remove(&id);
                    self.streams.writers.ready.swap_remove(&id);
                } else {
                    self.streams.readers.ready.insert(id);
                    if writer_scope.is_some() && !self.streams.writers.in_use.contains_key(&id) {
                        self.streams.writers.ready.insert(id);
                    }
                }
            }
            Choice::StreamDropReadable => {
                let set = &self.streams.readers.droppable;
                let i = u.int_in_range(0..=set.len() - 1)?;
                let id = *set.get_index(i).unwrap();
                let scope = self.streams.readers.remove(id);
                commands.push((scope, Command::StreamDropReadable(id)));

                if self.streams.writers.in_use.contains_key(&id) {
                    self.streams.writers.set_in_use_state_dropped(id);
                }
            }
            Choice::StreamDropWritable => {
                let set = &self.streams.writers.droppable;
                let i = u.int_in_range(0..=set.len() - 1)?;
                let id = *set.get_index(i).unwrap();
                let scope = self.streams.writers.remove(id);
                commands.push((scope, Command::StreamDropWritable(id)));

                if self.streams.readers.in_use.contains_key(&id) {
                    self.streams.readers.set_in_use_state_dropped(id);
                }
            }
            Choice::StreamRead => {
                let set = &self.streams.readers.ready;
                let i = u.int_in_range(0..=set.len() - 1)?;
                let id = *set.get_index(i).unwrap();
                let scope = self.streams.readers.handles[&id].0;
                let count = u.int_in_range(0..=MAX_STREAM_COUNT)?;

                // FIXME(#12090)
                if scope.is_host() {
                    self.streams.readers.lock_in_place(id);
                }

                if !self.streams.writers.handles.contains_key(&id) {
                    // If the write handle is dropped, then this should
                    // immediately report as such.
                    commands.push((
                        scope,
                        Command::StreamReadDropped(StreamReadPayload { stream: id, count }),
                    ));
                    // Can't read from this stream again, so it's not ready,
                    // and then we also can't lift/lower it any more so it's
                    // locked in place.
                    assert!(self.streams.readers.ready.swap_remove(&id));
                    self.streams.readers.lock_in_place(id);
                } else if self.streams.writers.in_use.contains_key(&id) {
                    // If the write handle is active then this read should
                    // complete immediately.
                    let write_count = self.streams.writers.in_use[&id].0.count;
                    let write_scope = self.streams.writers.handles[&id].0;
                    let min = count.min(write_count);

                    match (count, write_count) {
                        // Two zero-length operations rendezvousing will leave
                        // the reader blocked but the writer should wake up. A
                        // nonzero-length read and a 0-length write performs
                        // the same way too.
                        (0, 0) | (1.., 0) => {
                            self.streams
                                .readers
                                .start(id, Cancellable::Yes, StreamRead { count });
                            commands.push((
                                scope,
                                Command::StreamReadPending(StreamReadPayload { stream: id, count }),
                            ));
                            self.streams.writers.stop(id);
                            commands.push((
                                write_scope,
                                Command::StreamWriteAssertComplete(StreamReadPayload {
                                    stream: id,
                                    count: min,
                                }),
                            ));
                        }

                        // A zero-length read with a nonzero-length-write
                        // should wake up just the reader and do nothing to the
                        // writer.
                        (0, 1..) => {
                            commands.push((
                                scope,
                                Command::StreamReadReady(StreamReadyPayload {
                                    stream: id,
                                    item: 0,
                                    ready_count: min,
                                    op_count: count,
                                }),
                            ));
                        }

                        // With two nonzero lengths both operations should complete.
                        (1.., 1..) => {
                            let (write, _) = self.streams.writers.stop(id);
                            commands.push((
                                scope,
                                Command::StreamReadReady(StreamReadyPayload {
                                    stream: id,
                                    item: write.item,
                                    ready_count: min,
                                    op_count: count,
                                }),
                            ));
                            commands.push((
                                write_scope,
                                Command::StreamWriteAssertComplete(StreamReadPayload {
                                    stream: id,
                                    count: min,
                                }),
                            ));
                        }
                    }
                } else {
                    // If the write handle is not active then this should be in
                    // a pending state now.
                    self.streams
                        .readers
                        .start(id, Cancellable::Yes, StreamRead { count });
                    commands.push((
                        scope,
                        Command::StreamReadPending(StreamReadPayload { stream: id, count }),
                    ));
                }
            }
            Choice::StreamWrite => {
                let set = &self.streams.writers.ready;
                let i = u.int_in_range(0..=set.len() - 1)?;
                let id = *set.get_index(i).unwrap();
                let scope = self.streams.writers.handles[&id].0;
                let item = self.next_id();
                let count = u.int_in_range(0..=MAX_STREAM_COUNT)?;

                // FIXME(#12090)
                if scope.is_host() {
                    self.streams.writers.lock_in_place(id);
                }

                if !self.streams.readers.handles.contains_key(&id) {
                    // If the read handle is dropped, then this should
                    // immediately report as such.
                    commands.push((
                        scope,
                        Command::StreamWriteDropped(StreamWritePayload {
                            stream: id,
                            item,
                            count,
                        }),
                    ));
                    // Cannot write ever again to this handle so remove it from
                    // the writable set.
                    assert!(self.streams.writers.ready.swap_remove(&id));
                } else if self.streams.readers.in_use.contains_key(&id) {
                    // If the read handle is active then this write should
                    // complete immediately.
                    let read_count = self.streams.readers.in_use[&id].0.count;
                    let read_scope = self.streams.readers.handles[&id].0;
                    let min = count.min(read_count);

                    match (read_count, count) {
                        // A zero-length write, no matter what the read half is
                        // pending as, is always ready and doesn't affect the
                        // reader.
                        (_, 0) => {
                            commands.push((
                                scope,
                                Command::StreamWriteReady(StreamReadyPayload {
                                    stream: id,
                                    item,
                                    op_count: count,
                                    ready_count: min,
                                }),
                            ));
                        }

                        // With a zero-length read and a nonzero-length write
                        // the writer is blocked but the reader is unblocked.
                        (0, 1..) => {
                            self.streams.writers.start(
                                id,
                                Cancellable::Yes,
                                StreamWrite { item, count },
                            );
                            commands.push((
                                scope,
                                Command::StreamWritePending(StreamWritePayload {
                                    stream: id,
                                    item,
                                    count,
                                }),
                            ));
                            self.streams.readers.stop(id);
                            commands.push((
                                read_scope,
                                Command::StreamReadAssertComplete(StreamWritePayload {
                                    stream: id,
                                    item,
                                    count: min,
                                }),
                            ));
                        }

                        // Nonzero sizes means that the write immediately
                        // finishes and the read is also now ready to complete.
                        (1.., 1..) => {
                            commands.push((
                                scope,
                                Command::StreamWriteReady(StreamReadyPayload {
                                    stream: id,
                                    item,
                                    op_count: count,
                                    ready_count: min,
                                }),
                            ));
                            self.streams.readers.stop(id);
                            commands.push((
                                read_scope,
                                Command::StreamReadAssertComplete(StreamWritePayload {
                                    stream: id,
                                    item,
                                    count: min,
                                }),
                            ));
                        }
                    }
                } else {
                    // If the read handle is not active then this should be in
                    // a pending state now.
                    self.streams
                        .writers
                        .start(id, Cancellable::Yes, StreamWrite { item, count });
                    commands.push((
                        scope,
                        Command::StreamWritePending(StreamWritePayload {
                            stream: id,
                            item,
                            count,
                        }),
                    ));
                }
            }
            Choice::StreamEndRead => {
                let set = &self.streams.readers.cancellable;
                let i = u.int_in_range(0..=set.len() - 1)?;
                let id = *set.get_index(i).unwrap();
                let scope = self.streams.readers.handles[&id].0;

                let (_read, state) = self.streams.readers.stop(id);
                match state {
                    OpState::Pending => {
                        commands.push((scope, Command::StreamCancelRead(id)));
                    }
                    OpState::Dropped => {
                        commands.push((scope, Command::StreamReadAssertDropped(id)));
                    }
                }
            }
            Choice::StreamEndWrite => {
                let set = &self.streams.writers.cancellable;
                let i = u.int_in_range(0..=set.len() - 1)?;
                let id = *set.get_index(i).unwrap();
                let scope = self.streams.writers.handles[&id].0;

                let (_write, state) = self.streams.writers.stop(id);
                match state {
                    OpState::Pending => {
                        commands.push((scope, Command::StreamCancelWrite(id)));
                    }
                    OpState::Dropped => {
                        commands.push((
                            scope,
                            Command::StreamWriteAssertDropped(StreamReadPayload {
                                stream: id,
                                count: 0,
                            }),
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn next_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}
